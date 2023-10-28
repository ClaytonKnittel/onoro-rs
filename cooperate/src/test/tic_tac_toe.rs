use std::fmt::Display;

use abstract_game::{Game, GameMoveGenerator};

#[derive(PartialEq, Eq)]
pub enum TttPlayer {
  First,
  Second,
}

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

impl GameMoveGenerator for TttMoveIter {
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

#[derive(Clone)]
pub struct Ttt {
  board: [TttTile; 9],
  turn: u32,
}

impl Ttt {
  pub fn new() -> Self {
    Self {
      board: [TttTile::Empty; 9],
      turn: 0,
    }
  }

  pub fn tile_at(&self, x: u32, y: u32) -> TttTile {
    self.board[(x + 3 * y) as usize]
  }
}

impl Game for Ttt {
  type Move = TttMove;
  type MoveGenerator = TttMoveIter;
  type PlayerIdentifier = TttPlayer;

  fn move_generator(&self) -> Self::MoveGenerator {
    Self::MoveGenerator { x: 0, y: 0 }
  }

  fn make_move(&mut self, m: Self::Move) {
    self.board[(m.x + 3 * m.y) as usize] = if self.turn % 2 == 0 {
      TttTile::X
    } else {
      TttTile::O
    };
  }

  fn current_player(&self) -> Self::PlayerIdentifier {
    todo!()
  }

  fn finished(&self) -> Option<Self::PlayerIdentifier> {
    todo!()
  }
}
