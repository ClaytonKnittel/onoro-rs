use std::{fmt::Display, hash::Hash};

use abstract_game::{Game, GameMoveGenerator, GameResult, Score};

use crate::{array::Array, table::TableEntry};

#[derive(Debug, PartialEq, Eq)]
pub enum GomokuPlayer {
  First,
  Second,
}

impl From<GomokuTile> for GomokuPlayer {
  fn from(value: GomokuTile) -> Self {
    match value {
      GomokuTile::X => GomokuPlayer::First,
      GomokuTile::O => GomokuPlayer::Second,
      GomokuTile::Empty => panic!("Calling into::<GomokuPlayer>() on Empty tile."),
    }
  }
}

#[derive(Clone, Copy)]
pub struct GomokuMove {
  x: u32,
  y: u32,
}

impl Display for GomokuMove {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "({}, {})", self.x, self.y)
  }
}

pub struct GomokuMoveIter {
  x: u32,
  y: u32,
}

impl GomokuMoveIter {
  fn inc(&mut self, gomoku: &Gomoku) {
    self.x = (self.x + 1) % gomoku.width;
    self.y += if self.x == 0 { 1 } else { 0 };
  }
}

impl GameMoveGenerator for GomokuMoveIter {
  type Item = GomokuMove;
  type Game = Gomoku;

  fn next(&mut self, gomoku: &Gomoku) -> Option<Self::Item> {
    while self.y < gomoku.height && gomoku.tile_at(self.x, self.y) != GomokuTile::Empty {
      self.inc(gomoku);
    }
    if self.y != gomoku.height {
      let res = Some(GomokuMove {
        x: self.x,
        y: self.y,
      });
      self.inc(gomoku);
      res
    } else {
      None
    }
  }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum GomokuTile {
  Empty,
  X,
  O,
}

impl Display for GomokuTile {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "{}",
      match self {
        GomokuTile::Empty => ".",
        GomokuTile::X => "X",
        GomokuTile::O => "O",
      }
    )
  }
}

#[derive(Clone)]
pub struct Gomoku {
  tiles: Array<GomokuTile>,
  width: u32,
  height: u32,
  /// The number of pieces needed in a straight/diagonal line to win.
  to_win: u32,
  turn: u32,
  score: Score,
}

impl Gomoku {
  pub fn new(width: u32, height: u32, to_win: u32) -> Self {
    let mut tiles = Array::new(width * height);
    for _ in 0..tiles.capacity() {
      tiles.push(GomokuTile::Empty);
    }

    Self {
      tiles,
      width,
      height,
      to_win,
      turn: 0,
      score: Score::no_info(),
    }
  }

  fn idx(&self, x: u32, y: u32) -> u32 {
    x + self.width * y
  }

  pub fn tile_at(&self, x: u32, y: u32) -> GomokuTile {
    debug_assert!(x < self.width);
    debug_assert!(y < self.height);
    *self.tiles.get(self.idx(x, y))
  }
}

impl Game for Gomoku {
  type Move = GomokuMove;
  type MoveGenerator = GomokuMoveIter;
  type PlayerIdentifier = GomokuPlayer;

  fn move_generator(&self) -> Self::MoveGenerator {
    Self::MoveGenerator { x: 0, y: 0 }
  }

  fn make_move(&mut self, m: Self::Move) {
    debug_assert_eq!(self.tile_at(m.x, m.y), GomokuTile::Empty);
    *self.tiles.get_mut(self.idx(m.x, m.y)) = if self.turn % 2 == 0 {
      GomokuTile::X
    } else {
      GomokuTile::O
    };
    self.turn += 1;
  }

  fn current_player(&self) -> Self::PlayerIdentifier {
    if self.turn % 2 == 0 {
      GomokuPlayer::First
    } else {
      GomokuPlayer::Second
    }
  }

  fn finished(&self) -> GameResult<Self::PlayerIdentifier> {
    // Check rows
    for y in 0..self.height {
      for x in 0..(self.width - (self.to_win - 1)) {
        if self.tile_at(x, y) != GomokuTile::Empty
          && (1..self.to_win).all(|offset| self.tile_at(x, y) == self.tile_at(x + offset, y))
        {
          return GameResult::Win(self.tile_at(x, y).into());
        }
      }
    }
    // Check columns
    for y in 0..(self.height - (self.to_win - 1)) {
      for x in 0..self.width {
        if self.tile_at(x, y) != GomokuTile::Empty
          && (1..self.to_win).all(|offset| self.tile_at(x, y) == self.tile_at(x, y + offset))
        {
          return GameResult::Win(self.tile_at(x, y).into());
        }
      }
    }
    // Check top left to bottom right diagonals.
    for y in 0..(self.height - (self.to_win - 1)) {
      for x in 0..(self.width - (self.to_win - 1)) {
        if self.tile_at(x, y) != GomokuTile::Empty
          && (1..self.to_win)
            .all(|offset| self.tile_at(x, y) == self.tile_at(x + offset, y + offset))
        {
          return GameResult::Win(self.tile_at(x, y).into());
        }
      }
    }
    // Check top right to bottom left diagonals.
    for y in 0..(self.height - (self.to_win - 1)) {
      for x in 0..(self.width - (self.to_win - 1)) {
        if self.tile_at(x + self.to_win - 1, y) != GomokuTile::Empty
          && (1..self.to_win).all(|offset| {
            self.tile_at(x + self.to_win - 1, y)
              == self.tile_at(x + self.to_win - 1 - offset, y + offset)
          })
        {
          return GameResult::Win(self.tile_at(x + self.to_win - 1, y).into());
        }
      }
    }

    if self.turn == self.width * self.height {
      GameResult::Tie
    } else {
      GameResult::NotFinished
    }
  }
}

impl TableEntry for Gomoku {
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

impl Hash for Gomoku {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.tiles.hash(state);
  }
}

impl PartialEq for Gomoku {
  fn eq(&self, other: &Self) -> bool {
    self.tiles == other.tiles
  }
}

impl Eq for Gomoku {}

impl Display for Gomoku {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    for y in 0..self.height {
      let y = self.height - y - 1;
      for x in 0..self.width {
        write!(f, "{} ", self.tile_at(x, y))?;
      }
      if y != 0 {
        writeln!(f)?;
      }
    }
    Ok(())
  }
}
