use std::{fmt::Display, hash::Hash};

use abstract_game::{Game, GameIter, Score};

use crate::table::TableEntry;

#[derive(PartialEq, Eq)]
pub enum NimPlayer {
  First,
  Second,
}

#[derive(Clone, Copy)]
pub struct NimMove {
  sticks: u32,
}

impl Display for NimMove {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.sticks)
  }
}

pub struct NimMoveIter {
  sticks: u32,
  max_sticks: u32,
}

impl GameIter for NimMoveIter {
  type Item = NimMove;
  type Game = Nim;

  fn next(&mut self, nim: &Nim) -> Option<Self::Item> {
    if self.sticks > self.max_sticks {
      None
    } else {
      self.sticks += 1;
      Some(NimMove {
        sticks: self.sticks - 1,
      })
    }
  }
}

#[derive(Clone)]
pub struct Nim {
  sticks: u32,
  turn: u32,
  score: Score,
}

impl Nim {
  pub fn new(sticks: u32) -> Self {
    Self {
      sticks,
      turn: 0,
      score: Score::no_info(),
    }
  }

  pub fn expected_score(&self) -> Score {
    if self.sticks % 3 == 0 {
      let turn_count_win = self.sticks * 2 / 3;
      Score::new(false, turn_count_win - 1, turn_count_win)
    } else {
      let turn_count_win = self.sticks / 3 * 2;
      Score::new(true, turn_count_win, turn_count_win + 1)
    }
  }
}

impl Game for Nim {
  type Move = NimMove;
  type MoveIterator = NimMoveIter;
  type PlayerIdentifier = NimPlayer;

  fn move_generator(&self) -> Self::MoveIterator {
    NimMoveIter {
      sticks: 1,
      max_sticks: self.sticks.min(2),
    }
  }

  fn make_move(&mut self, m: Self::Move) {
    self.sticks -= m.sticks;
    self.turn += 1;
  }

  fn current_player(&self) -> Self::PlayerIdentifier {
    if self.turn % 2 == 0 {
      NimPlayer::First
    } else {
      NimPlayer::Second
    }
  }

  fn finished(&self) -> Option<Self::PlayerIdentifier> {
    if self.sticks == 0 {
      // The winner is the player to take the last stick.
      if self.turn % 2 == 0 {
        Some(NimPlayer::Second)
      } else {
        Some(NimPlayer::First)
      }
    } else {
      None
    }
  }
}

impl TableEntry for Nim {
  fn score(&self) -> abstract_game::Score {
    self.score.clone()
  }

  fn set_score(&mut self, score: Score) {
    self.score = score;
  }

  fn merge(&mut self, other: &Self) {
    self.score.merge(&other.score);
  }
}

impl Hash for Nim {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.sticks.hash(state);
  }
}

impl PartialEq for Nim {
  fn eq(&self, other: &Self) -> bool {
    self.sticks == other.sticks
  }
}

impl Eq for Nim {}

impl Display for Nim {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{} (turn {})", self.sticks, self.turn)
  }
}
