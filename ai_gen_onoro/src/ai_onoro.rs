use std::collections::HashMap;

use onoro::{Onoro, PawnColor, TileState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PackedIdx(i32, i32); // Axial hex coordinates

impl PackedIdx {
  pub fn add(&self, dq: i32, dr: i32) -> PackedIdx {
    PackedIdx(self.0 + dq, self.1 + dr)
  }

  pub fn sub(&self, dq: i32, dr: i32) -> PackedIdx {
    PackedIdx(self.0 - dq, self.1 - dr)
  }
}

impl From<onoro::PackedIdx> for PackedIdx {
  fn from(value: onoro::PackedIdx) -> Self {
    Self(value.x() as i32, value.y() as i32)
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Move {
  Place(PackedIdx),
  Move(PackedIdx, PackedIdx), // from, to
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pawn {
  pub color: PawnColor,
  pub pos: PackedIdx,
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
}

impl Onoro for OnoroGame {
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

  fn get_tile(&self, idx: onoro::PackedIdx) -> TileState {
    match self.board.get(&(idx.into())) {
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

  fn each_move(&self) -> impl Iterator<Item = Move> + '_ {
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
        .filter(move |(_, &c)| c == self.to_move)
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
        let remaining = self.pawns_remaining_mut(self.to_move);
        assert!(*remaining > 0);
        assert!(!self.board.contains_key(&pos));

        let touching = pos
          .neighbors()
          .iter()
          .filter(|n| self.board.contains_key(n))
          .count();
        assert!(touching >= 2);

        self.board.insert(pos, self.to_move);
        *remaining -= 1;

        if self.white_pawns_remaining == 0 && self.black_pawns_remaining == 0 {
          self.phase1 = false;
        }

        if let Some(winner) = self.check_win(pos) {
          self.winner = Some(winner);
        }

        self.to_move = match self.to_move {
          PawnColor::Black => PawnColor::White,
          PawnColor::White => PawnColor::Black,
        };
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

        self.to_move = self.to_move.opponent();
      }
    }

    // If no legal moves exist, the current player loses
    if self.each_move().next().is_none() {
      self.winner = Some(self.to_move.opponent());
    }
  }
}
