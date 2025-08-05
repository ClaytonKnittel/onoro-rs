use std::collections::{HashMap, HashSet};

use onoro::{Onoro, OnoroIndex, OnoroMove, OnoroPawn, PawnColor, TileState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PackedIdx(i32, i32); // Axial hex coordinates

impl OnoroIndex for PackedIdx {
  fn from_coords(x: u32, y: u32) -> Self {
    Self(x as i32, y as i32)
  }

  fn x(&self) -> i32 {
    self.0
  }

  fn y(&self) -> i32 {
    self.1
  }
}

impl PackedIdx {
  pub fn add(&self, dq: i32, dr: i32) -> PackedIdx {
    PackedIdx(self.0 + dq, self.1 + dr)
  }

  pub fn sub(&self, dq: i32, dr: i32) -> PackedIdx {
    PackedIdx(self.0 - dq, self.1 - dr)
  }
}

impl PackedIdx {
  pub fn neighbors(&self) -> [PackedIdx; 6] {
    let &PackedIdx(q, r) = self;
    [
      PackedIdx(q + 1, r),     // east
      PackedIdx(q - 1, r),     // west
      PackedIdx(q, r + 1),     // southeast
      PackedIdx(q, r - 1),     // northwest
      PackedIdx(q + 1, r - 1), // northeast
      PackedIdx(q - 1, r + 1), // southwest
    ]
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Move {
  Place(PackedIdx),
  Move(PackedIdx, PackedIdx), // from, to
}

impl OnoroMove<PackedIdx> for Move {
  fn make_phase1(pos: PackedIdx) -> Self {
    Move::Place(pos)
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pawn {
  pub color: PawnColor,
  pub pos: PackedIdx,
}

impl OnoroPawn<PackedIdx> for Pawn {
  fn pos(&self) -> PackedIdx {
    self.pos
  }

  fn color(&self) -> PawnColor {
    self.color
  }
}

fn opponent(pawn_color: PawnColor) -> PawnColor {
  match pawn_color {
    PawnColor::Black => PawnColor::White,
    PawnColor::White => PawnColor::Black,
  }
}

fn is_fully_connected(board: &HashMap<PackedIdx, PawnColor>) -> bool {
  if board.is_empty() {
    return true;
  }

  let mut visited = HashSet::new();
  let mut stack = Vec::new();

  // Start from any pawn
  let &start = board.keys().next().unwrap();
  stack.push(start);
  visited.insert(start);

  while let Some(current) = stack.pop() {
    for neighbor in current.neighbors() {
      if board.contains_key(&neighbor) && visited.insert(neighbor) {
        stack.push(neighbor);
      }
    }
  }

  visited.len() == board.len()
}

/// The main struct implementing the game state.
pub struct OnoroGame {
  /// Map from board coordinates to the color of the occupying pawn.
  board: HashMap<PackedIdx, PawnColor>,

  /// Whose turn it is to play.
  to_move: PawnColor,

  /// How many pawns each player has remaining to place.
  white_pawns_remaining: usize,
  black_pawns_remaining: usize,

  /// Whether the game is in the placement phase (Phase 1).
  phase1: bool,

  /// Cached result of the game, if known. (None = ongoing)
  winner: Option<PawnColor>,
}

impl OnoroGame {
  fn pawns_remaining(&self, color: PawnColor) -> usize {
    match color {
      PawnColor::White => self.white_pawns_remaining,
      PawnColor::Black => self.black_pawns_remaining,
    }
  }

  fn pawns_remaining_mut(&mut self, color: PawnColor) -> &mut usize {
    match color {
      PawnColor::White => &mut self.white_pawns_remaining,
      PawnColor::Black => &mut self.black_pawns_remaining,
    }
  }

  fn check_win(&self, pos: PackedIdx) -> Option<PawnColor> {
    let color = self.board.get(&pos)?;
    let directions = [
      (1, 0),  // east-west
      (0, 1),  // southeast-northwest
      (1, -1), // northeast-southwest
    ];

    for &(dq, dr) in &directions {
      let mut count = 1;

      // forward direction
      let mut cur = pos.add(dq, dr);
      while self.board.get(&cur) == Some(color) {
        count += 1;
        cur = cur.add(dq, dr);
      }

      // backward direction
      let mut cur = pos.sub(dq, dr);
      while self.board.get(&cur) == Some(color) {
        count += 1;
        cur = cur.sub(dq, dr);
      }

      if count >= 4 {
        return Some(*color);
      }
    }

    None
  }

  pub fn is_legal_move(&self, from: PackedIdx, to: PackedIdx) -> bool {
    // 1. from must contain current player's pawn, to must be empty
    if self.board.get(&from) != Some(&self.to_move) || self.board.contains_key(&to) {
      return false;
    }

    // 2. Simulate move
    let mut temp_board = self.board.clone();
    temp_board.remove(&from);
    temp_board.insert(to, self.to_move);

    // 3. Check connectivity
    if !is_fully_connected(&temp_board) {
      return false;
    }

    // 4. Check no lonely pawns
    for &pos in temp_board.keys() {
      let neighbors = pos.neighbors();
      let neighbor_count = neighbors
        .iter()
        .filter(|n| temp_board.contains_key(n))
        .count();
      if neighbor_count <= 1 {
        return false;
      }
    }

    true
  }
}

impl Onoro for OnoroGame {
  type Index = PackedIdx;
  type Move = Move;
  type Pawn = Pawn;

  unsafe fn new() -> Self {
    let mut board = HashMap::new();

    // Initial triangle: 2 black, 1 white
    board.insert(PackedIdx(0, 0), PawnColor::Black);
    board.insert(PackedIdx(1, 0), PawnColor::Black);
    board.insert(PackedIdx(0, 1), PawnColor::White);

    OnoroGame {
      board,
      to_move: PawnColor::White,
      white_pawns_remaining: 7,
      black_pawns_remaining: 6,
      phase1: true,
      winner: None,
    }
  }

  fn pawns_per_player() -> usize {
    8
  }

  fn turn(&self) -> PawnColor {
    self.to_move
  }

  fn pawns_in_play(&self) -> u32 {
    self.board.len() as u32
  }

  fn finished(&self) -> Option<PawnColor> {
    self.winner
  }

  fn get_tile(&self, idx: PackedIdx) -> TileState {
    match self.board.get(&idx) {
      Some(PawnColor::Black) => TileState::Black,
      Some(PawnColor::White) => TileState::White,
      None => TileState::Empty,
    }
  }

  fn pawns(&self) -> impl Iterator<Item = Pawn> + '_ {
    self.board.iter().map(|(&pos, &color)| Pawn { color, pos })
  }

  fn in_phase1(&self) -> bool {
    self.phase1
  }

  fn each_move(&self) -> impl Iterator<Item = Move> {
    if self.phase1 {
      let mut candidates = HashSet::new();

      for pos in self.board.keys() {
        for neighbor in pos.neighbors() {
          if !self.board.contains_key(&neighbor) {
            let touching = neighbor
              .neighbors()
              .iter()
              .filter(|n| self.board.contains_key(n))
              .count();
            if touching >= 2 {
              candidates.insert(neighbor);
            }
          }
        }
      }

      candidates
        .into_iter()
        .map(Move::Place)
        .collect::<Vec<_>>()
        .into_iter()
    } else {
      // Phase 2: generate all valid (from, to) moves
      self
        .board
        .iter()
        .filter(move |&(_, &c)| c == self.to_move)
        .flat_map(move |(&from, _)| {
          from
            .neighbors()
            .into_iter()
            .filter(move |to| !self.board.contains_key(to) && self.is_legal_move(from, *to))
            .map(move |to| Move::Move(from, to))
        })
        .collect::<Vec<_>>()
        .into_iter()
    }
  }

  fn make_move(&mut self, m: Move) {
    match m {
      Move::Place(pos) => {
        assert!(self.phase1, "Cannot place during phase 2");
        assert!(self.pawns_remaining(self.to_move) > 0);
        assert!(!self.board.contains_key(&pos));

        let touching = pos
          .neighbors()
          .iter()
          .filter(|n| self.board.contains_key(n))
          .count();
        assert!(touching >= 2);

        self.board.insert(pos, self.to_move);
        *self.pawns_remaining_mut(self.to_move) -= 1;

        if self.white_pawns_remaining == 0 && self.black_pawns_remaining == 0 {
          self.phase1 = false;
        }

        if let Some(winner) = self.check_win(pos) {
          self.winner = Some(winner);
        }

        self.to_move = opponent(self.to_move);
      }

      Move::Move(from, to) => {
        assert!(!self.phase1, "Cannot move during phase 1");
        assert_eq!(self.board.get(&from), Some(&self.to_move));
        assert!(!self.board.contains_key(&to));
        assert!(self.is_legal_move(from, to));

        self.board.remove(&from);
        self.board.insert(to, self.to_move);

        if let Some(winner) = self.check_win(to) {
          self.winner = Some(winner);
        }

        self.to_move = opponent(self.to_move);
      }
    }

    // If no legal moves exist, the current player loses
    if self.each_move().next().is_none() {
      self.winner = Some(opponent(self.to_move));
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn count_moves(game: &OnoroGame) -> usize {
    game.each_move().count()
  }

  fn pawn_count(game: &OnoroGame, color: PawnColor) -> usize {
    game.pawns().filter(|p| p.color == color).count()
  }

  fn get_tile_color(game: &OnoroGame, pos: PackedIdx) -> TileState {
    game.get_tile(pos.into())
  }

  fn make_legal_move(game: &mut OnoroGame) {
    let m = game.each_move().next().expect("no legal moves");
    game.make_move(m);
  }

  #[test]
  fn test_initial_setup() {
    unsafe {
      let game = OnoroGame::new();

      assert_eq!(game.turn(), PawnColor::White);
      assert_eq!(game.pawns_in_play(), 3);
      assert!(game.in_phase1());

      assert_eq!(pawn_count(&game, PawnColor::Black), 2);
      assert_eq!(pawn_count(&game, PawnColor::White), 1);
    }
  }

  #[test]
  fn test_phase1_moves_exist() {
    unsafe {
      let game = OnoroGame::new();
      let moves: Vec<_> = game.each_move().collect();
      assert!(!moves.is_empty());
      for m in &moves {
        if let Move::Place(pos) = m {
          // Placement must be onto empty tile
          assert_eq!(game.get_tile((*pos).into()), TileState::Empty);
        } else {
          panic!("expected only placement moves in phase 1");
        }
      }
    }
  }

  #[test]
  fn test_win_detection_four_in_line() {
    // Manually construct a win situation for black
    let mut game = unsafe { OnoroGame::new() };

    // Place until phase 2
    while game.in_phase1() {
      make_legal_move(&mut game);
    }

    // Clear board and simulate 4 in a line
    game.board.clear();
    game.phase1 = false;
    game.white_pawns_remaining = 0;
    game.black_pawns_remaining = 0;

    let line = [
      PackedIdx(0, 0),
      PackedIdx(1, 0),
      PackedIdx(2, 0),
      PackedIdx(3, 0),
    ];
    for pos in &line {
      game.board.insert(*pos, PawnColor::Black);
    }

    assert_eq!(game.check_win(PackedIdx(1, 0)), Some(PawnColor::Black));
    assert_eq!(game.check_win(PackedIdx(2, 0)), Some(PawnColor::Black));
  }

  #[test]
  fn test_illegal_move_due_to_disconnection() {
    // Make a connected ring
    let mut game = unsafe { OnoroGame::new() };
    game.board.clear();
    game.phase1 = false;
    game.white_pawns_remaining = 0;
    game.black_pawns_remaining = 0;

    let ring = [
      PackedIdx(0, 0),
      PackedIdx(1, 0),
      PackedIdx(1, -1),
      PackedIdx(0, -1),
      PackedIdx(-1, 0),
      PackedIdx(-1, 1),
    ];

    for &pos in &ring {
      game.board.insert(pos, PawnColor::Black);
    }

    game.to_move = PawnColor::Black;

    // Attempt to move a pawn that disconnects the ring
    let from = PackedIdx(0, 0);
    let to = PackedIdx(2, 2);

    assert!(!game.is_legal_move(from, to));
  }

  #[test]
  fn test_legal_move_preserves_connection_and_no_lonely_pawns() {
    let mut game = unsafe { OnoroGame::new() };
    game.board.clear();
    game.phase1 = false;
    game.white_pawns_remaining = 0;
    game.black_pawns_remaining = 0;

    let group = [
      PackedIdx(0, 0),
      PackedIdx(1, 0),
      PackedIdx(0, 1),
      PackedIdx(1, 1),
    ];

    for &pos in &group {
      game.board.insert(pos, PawnColor::White);
    }

    game.to_move = PawnColor::White;

    let from = PackedIdx(1, 1);
    let to = PackedIdx(2, 0);

    assert!(game.is_legal_move(from, to));
  }
}
