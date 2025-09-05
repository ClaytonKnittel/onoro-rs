use crate::{Game, GameMoveIterator, GamePlayer, GameResult};

pub struct NimMoveIter {
  sticks: u32,
}

impl GameMoveIterator for NimMoveIter {
  type Item = u32;
  type Game = Nim;

  fn next(&mut self, nim: &Nim) -> Option<Self::Item> {
    if self.sticks >= 2.min(nim.sticks) {
      None
    } else {
      self.sticks += 1;
      Some(self.sticks)
    }
  }
}

#[derive(Clone, Debug)]
pub struct Nim {
  sticks: u32,
  player1: bool,
}

impl Nim {
  pub fn new(sticks: u32) -> Self {
    Self {
      sticks,
      player1: true,
    }
  }
}

impl Game for Nim {
  type Move = u32;
  type MoveGenerator = NimMoveIter;

  fn move_generator(&self) -> NimMoveIter {
    NimMoveIter { sticks: 0 }
  }

  fn make_move(&mut self, sticks: u32) {
    debug_assert!(sticks <= self.sticks);
    self.sticks -= sticks;
    self.player1 = !self.player1;
  }

  fn current_player(&self) -> GamePlayer {
    if self.player1 {
      GamePlayer::Player1
    } else {
      GamePlayer::Player2
    }
  }

  fn finished(&self) -> GameResult {
    if self.sticks == 0 {
      GameResult::Win(if self.player1 {
        GamePlayer::Player2
      } else {
        GamePlayer::Player1
      })
    } else {
      GameResult::NotFinished
    }
  }
}
