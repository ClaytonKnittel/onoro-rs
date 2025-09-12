use std::{
  collections::{HashMap, HashSet},
  fmt::Debug,
};

use itertools::Itertools;
use onoro::{
  Onoro, OnoroIndex, OnoroMoveWrapper, OnoroPawn, PawnColor, TileState,
  abstract_game::{Game, GameMoveIterator, GamePlayer, GameResult},
};

type Move = OnoroMoveWrapper<PackedIdx>;

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
      PackedIdx(q + 1, r),
      PackedIdx(q - 1, r),
      PackedIdx(q, r + 1),
      PackedIdx(q, r - 1),
      PackedIdx(q + 1, r + 1),
      PackedIdx(q - 1, r - 1),
    ]
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pawn {
  pub color: PawnColor,
  pub pos: PackedIdx,
}

impl OnoroPawn for Pawn {
  type Index = PackedIdx;

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
#[derive(Clone)]
pub struct OnoroGame {
  /// Map from board coordinates to the color of the occupying pawn.
  board: HashMap<PackedIdx, PawnColor>,

  /// Whose turn it is to play.
  to_move: PawnColor,

  /// Cached result of the game, if known. (None = ongoing)
  winner: Option<PawnColor>,
}

impl OnoroGame {
  fn pawns_remaining(&self, color: PawnColor) -> usize {
    Self::pawns_per_player()
      - self
        .board
        .values()
        .filter(|&&pawn_color| pawn_color == color)
        .count()
  }

  fn check_win(&self, pos: PackedIdx) -> Option<PawnColor> {
    debug_assert!(self.board.contains_key(&pos));
    let color = self.board.get(&pos).unwrap();
    let directions = [(1, 0), (0, 1), (1, 1)];

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
    temp_board.keys().all(|&pos| {
      pos
        .neighbors()
        .iter()
        .filter(|n| temp_board.contains_key(n))
        .count()
        >= 2
    })
  }
}

pub struct OnoroMoveGen<I>(I);

impl<I> GameMoveIterator for OnoroMoveGen<I>
where
  I: Iterator<Item = Move>,
{
  type Game = OnoroGame;

  fn next(&mut self, _game: &OnoroGame) -> Option<Move> {
    self.0.next()
  }
}

impl Game for OnoroGame {
  type Move = Move;
  type MoveGenerator = OnoroMoveGen<Box<dyn Iterator<Item = Move>>>;

  fn move_generator(&self) -> Self::MoveGenerator {
    OnoroMoveGen(if self.in_phase1() {
      Box::new(
        self
          .board
          .keys()
          .flat_map(PackedIdx::neighbors)
          // Remove duplicates
          .collect::<HashSet<_>>()
          .into_iter()
          // Remove occupied locations
          .filter(|neighbor| !self.board.contains_key(neighbor))
          // Remove locations which aren't adjacent to at least 2 pawns.
          .filter(|neighbor| {
            neighbor
              .neighbors()
              .iter()
              .filter(|n| self.board.contains_key(n))
              .count()
              >= 2
          })
          .map(|to| Move::Phase1 { to })
          .collect_vec()
          .into_iter(),
      )
    } else {
      Box::new(
        self
          .board
          .iter()
          // Filter out other player's pawns.
          .filter_map(|(&pos, &c)| (c == self.to_move).then_some(pos))
          // Expand each position to all neighbors of all pawns
          .cartesian_product(
            self
              .board
              .keys()
              .flat_map(PackedIdx::neighbors)
              // Remove duplicates
              .collect::<HashSet<_>>()
              .into_iter()
              .filter(|to| !self.board.contains_key(to))
              .collect_vec(),
          )
          .filter(|&(from, to)| self.is_legal_move(from, to))
          .map(|(from, to)| Move::Phase2 { from, to })
          .collect_vec()
          .into_iter(),
      )
    })
  }

  fn make_move(&mut self, m: Self::Move) {
    debug_assert!(self.winner.is_none());

    match m {
      Move::Phase1 { to } => {
        debug_assert!(self.in_phase1(), "Cannot place during phase 2");
        debug_assert!(self.pawns_remaining(self.to_move) > 0);
        debug_assert!(!self.board.contains_key(&to));

        debug_assert!(
          to.neighbors()
            .iter()
            .filter(|n| self.board.contains_key(n))
            .count()
            >= 2
        );
      }

      Move::Phase2 { from, to } => {
        debug_assert!(!self.in_phase1(), "Cannot move during phase 1");
        debug_assert_eq!(self.board.get(&from), Some(&self.to_move));
        debug_assert!(!self.board.contains_key(&to));
        debug_assert!(self.is_legal_move(from, to));
      }
    }

    unsafe { self.make_move_unchecked(m) };
  }

  fn current_player(&self) -> GamePlayer {
    match self.to_move {
      PawnColor::Black => GamePlayer::Player1,
      PawnColor::White => GamePlayer::Player2,
    }
  }

  fn finished(&self) -> GameResult {
    match self.winner {
      Some(PawnColor::Black) => GameResult::Win(GamePlayer::Player1),
      Some(PawnColor::White) => GameResult::Win(GamePlayer::Player2),
      None => GameResult::NotFinished,
    }
  }
}

impl Onoro for OnoroGame {
  type Index = PackedIdx;
  type Pawn = Pawn;

  unsafe fn new() -> Self {
    OnoroGame {
      board: HashMap::new(),
      to_move: PawnColor::Black,
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

  fn get_tile(&self, idx: PackedIdx) -> TileState {
    match self.board.get(&idx) {
      Some(&color) => color.into(),
      None => TileState::Empty,
    }
  }

  fn pawns(&self) -> impl Iterator<Item = Pawn> + '_ {
    self.board.iter().map(|(&pos, &color)| Pawn { color, pos })
  }

  fn in_phase1(&self) -> bool {
    self.board.len() < 2 * Self::pawns_per_player()
  }

  unsafe fn make_move_unchecked(&mut self, m: Move) {
    let to = match m {
      Move::Phase1 { to } => to,
      Move::Phase2 { from, to } => {
        self.board.remove(&from);
        to
      }
    };
    self.board.insert(to, self.to_move);

    self.winner = self.check_win(to);
    self.to_move = opponent(self.to_move);

    // If no legal moves exist, the current player loses
    if self.each_move().next().is_none() {
      self.winner = Some(opponent(self.to_move));
    }
  }

  fn to_move_wrapper(&self, m: &Move) -> OnoroMoveWrapper<PackedIdx> {
    *m
  }
}

impl Debug for OnoroGame {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    self.display(f)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn pawn_count(game: &OnoroGame, color: PawnColor) -> usize {
    game.pawns().filter(|p| p.color == color).count()
  }

  fn make_legal_move(game: &mut OnoroGame) {
    let m = game.each_move().next().expect("no legal moves");
    game.make_move(m);
  }

  #[test]
  fn test_initial_setup() {
    let game = OnoroGame::default_start();

    assert_eq!(game.turn(), PawnColor::White);
    assert_eq!(game.pawns_in_play(), 3);
    assert!(game.in_phase1());

    assert_eq!(pawn_count(&game, PawnColor::Black), 2);
    assert_eq!(pawn_count(&game, PawnColor::White), 1);
  }

  #[test]
  fn test_phase1_moves_exist() {
    unsafe {
      let game = OnoroGame::new();
      let moves: Vec<_> = game.each_move().collect();
      assert!(!moves.is_empty());
      for m in &moves {
        if let Move::Phase1 { to } = m {
          // Placement must be onto empty tile
          assert_eq!(game.get_tile(*to), TileState::Empty);
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
