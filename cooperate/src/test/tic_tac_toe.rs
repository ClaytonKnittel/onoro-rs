use std::{fmt::Display, hash::Hash};

use abstract_game::{Game, GameMoveIterator, GamePlayer, GameResult, Score};

use crate::test::serial_search::find_best_move_serial;

#[derive(Clone, Copy)]
pub struct TttMove {
  x: u32,
  y: u32,
}

impl Display for TttMove {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "({}, {})", self.x, self.y)
  }
}

pub struct TttMoveIter {
  x: u32,
  y: u32,
}

impl TttMoveIter {
  fn inc(&mut self) {
    self.x = (self.x + 1) % 3;
    self.y += if self.x == 0 { 1 } else { 0 };
  }
}

impl GameMoveIterator for TttMoveIter {
  type Item = TttMove;
  type Game = Ttt;

  fn next(&mut self, ttt: &Ttt) -> Option<Self::Item> {
    while self.y < 3 && ttt.tile_at(self.x, self.y) != TttTile::Empty {
      self.inc();
    }
    if self.y != 3 {
      let res = Some(TttMove {
        x: self.x,
        y: self.y,
      });
      self.inc();
      res
    } else {
      None
    }
  }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TttTile {
  Empty,
  X,
  O,
}

impl Display for TttTile {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "{}",
      match self {
        TttTile::Empty => ".",
        TttTile::X => "X",
        TttTile::O => "O",
      }
    )
  }
}

#[derive(Clone)]
pub struct Ttt {
  /// Bits 0 - 10 are the positions of the Xs, bits 16 - 26 are the positions of
  /// the Os.
  tile_mask: u32,
  turn: u32,
}

impl Ttt {
  pub fn new() -> Self {
    Self {
      tile_mask: 0,
      turn: 0,
    }
  }

  fn idx(x: u32, y: u32) -> usize {
    (x + 4 * y) as usize
  }

  /// Returns the player whose piece is in the most significant set bit, or
  /// Empty if the mask is empty.
  fn player_for_mask(mask: u32) -> TttTile {
    let leading_zeros = mask.leading_zeros();
    if leading_zeros == 32 {
      TttTile::Empty
    } else if leading_zeros >= 16 {
      TttTile::X
    } else {
      TttTile::O
    }
  }

  pub fn tile_at(&self, x: u32, y: u32) -> TttTile {
    let mask = 0x0001_0001u32 << Self::idx(x, y);
    Self::player_for_mask(self.tile_mask & mask)
  }

  pub fn compute_expected_score(&self, depth: u32) -> Score {
    find_best_move_serial(self, depth).0.unwrap()
  }
}

impl Game for Ttt {
  type Move = TttMove;
  type MoveGenerator = TttMoveIter;

  fn move_generator(&self) -> Self::MoveGenerator {
    Self::MoveGenerator { x: 0, y: 0 }
  }

  fn make_move(&mut self, m: Self::Move) {
    let mut mask = 1u32 << Self::idx(m.x, m.y);
    if self.turn % 2 != 0 {
      mask <<= 16;
    }
    self.tile_mask |= mask;
    self.turn += 1;
  }

  fn current_player(&self) -> GamePlayer {
    if self.turn % 2 == 0 {
      GamePlayer::Player1
    } else {
      GamePlayer::Player2
    }
  }

  fn finished(&self) -> GameResult {
    let board = self.tile_mask;
    // Finished horizotally
    let horiz = board & (board >> 1) & (board >> 2);
    let vert = board & (board >> 4) & (board >> 8);
    let rdiag = board & (board >> 5) & (board >> 10);
    let ldiag = board & (board >> 3) & (board >> 6);

    let finished = horiz | vert | rdiag | ldiag;
    match Self::player_for_mask(finished) {
      TttTile::X => return GameResult::Win(GamePlayer::Player1),
      TttTile::O => return GameResult::Win(GamePlayer::Player2),
      TttTile::Empty => {}
    }

    if self.turn == 9 {
      GameResult::Tie
    } else {
      GameResult::NotFinished
    }
  }
}

impl Hash for Ttt {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.tile_mask.hash(state);
  }
}

impl PartialEq for Ttt {
  fn eq(&self, other: &Self) -> bool {
    self.tile_mask == other.tile_mask
  }
}

impl Eq for Ttt {}

impl Display for Ttt {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    for y in 0..3 {
      let y = 2 - y;
      for x in 0..3 {
        write!(f, "{} ", self.tile_at(x, y))?;
      }
      if y != 0 {
        writeln!(f)?;
      }
    }
    Ok(())
  }
}
