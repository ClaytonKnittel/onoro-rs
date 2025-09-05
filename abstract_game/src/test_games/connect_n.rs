use std::{fmt::Debug, hint::unreachable_unchecked};

use crate::{Game, GameMoveIterator, GamePlayer, GameResult};

trait InARow<U> {
  fn in_a_row(self, n: u32) -> Option<U>;
}

impl<T, U> InARow<U> for T
where
  T: IntoIterator<Item = Option<U>>,
  U: PartialEq + Clone,
{
  fn in_a_row(self, n: u32) -> Option<U> {
    self
      .into_iter()
      .fold(None, |acc, item| {
        let Some((u, count)) = acc else {
          return Some((item?, 1));
        };
        if count == n {
          return Some((u, count));
        }

        let item = item?;
        if u == item {
          Some((u, count + 1))
        } else {
          Some((item, 1))
        }
      })
      .and_then(|(item, count)| (count == n).then_some(item))
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConnectMove {
  pub col: u32,
}

pub struct ConnectMoveGen {
  col: u32,
}

impl GameMoveIterator for ConnectMoveGen {
  type Item = ConnectMove;
  type Game = ConnectN;

  fn next(&mut self, game: &ConnectN) -> Option<ConnectMove> {
    while self.col < game.width && game.at((self.col, game.height - 1)) != TileState::Empty {
      self.col += 1;
    }
    if self.col < game.width {
      self.col += 1;
      Some(ConnectMove { col: self.col - 1 })
    } else {
      None
    }
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TileState {
  Empty,
  P1,
  P2,
}

impl From<TileState> for Option<GamePlayer> {
  fn from(tile_state: TileState) -> Self {
    match tile_state {
      TileState::Empty => None,
      TileState::P1 => Some(GamePlayer::Player1),
      TileState::P2 => Some(GamePlayer::Player2),
    }
  }
}

#[derive(Clone)]
pub struct ConnectN {
  board: Vec<u32>,
  width: u32,
  height: u32,
  in_a_row: u32,
}

impl ConnectN {
  pub fn new(width: u32, height: u32, in_a_row: u32) -> Self {
    debug_assert!(in_a_row <= width);
    debug_assert!(in_a_row <= height);
    Self {
      board: vec![0; (2 * width * height).div_ceil(u32::BITS) as usize],
      width,
      height,
      in_a_row,
    }
  }

  fn pos_to_idx(&self, pos: (u32, u32)) -> (u32, usize) {
    debug_assert!((0..self.width).contains(&pos.0));
    debug_assert!((0..self.height).contains(&pos.1));
    let idx = pos.0 + pos.1 * self.width;
    (2 * (idx % 16), idx as usize / 16)
  }

  fn at(&self, pos: (u32, u32)) -> TileState {
    let (bit_idx, v_idx) = self.pos_to_idx(pos);
    match (self.board[v_idx] >> bit_idx) & 0x3 {
      0x0 => TileState::Empty,
      0x1 => TileState::P1,
      0x2 => TileState::P2,
      _ => unsafe { unreachable_unchecked() },
    }
  }

  fn set(&mut self, pos: (u32, u32), player: GamePlayer) {
    debug_assert_eq!(self.at(pos), TileState::Empty);
    let (bit_idx, v_idx) = self.pos_to_idx(pos);
    self.board[v_idx] += match player {
      GamePlayer::Player1 => 0x1,
      GamePlayer::Player2 => 0x2,
    } << bit_idx;
  }

  fn n_moves_made(&self) -> u32 {
    self.board.iter().map(|b| b.count_ones()).sum()
  }
}

impl Game for ConnectN {
  type Move = ConnectMove;
  type MoveGenerator = ConnectMoveGen;

  fn move_generator(&self) -> ConnectMoveGen {
    ConnectMoveGen { col: 0 }
  }

  fn make_move(&mut self, m: ConnectMove) {
    let y = (0..)
      .find(|&y| self.at((m.col, y)) == TileState::Empty)
      .unwrap();
    self.set((m.col, y), self.current_player());
  }

  fn current_player(&self) -> GamePlayer {
    if self.board.iter().map(|v| v.count_ones()).sum::<u32>() % 2 == 0 {
      GamePlayer::Player1
    } else {
      GamePlayer::Player2
    }
  }

  fn finished(&self) -> GameResult {
    for y in 0..self.height {
      if let Some(winner) = (0..self.width)
        .map(|x| self.at((x, y)).into())
        .in_a_row(self.in_a_row)
      {
        return GameResult::Win(winner);
      }
    }

    for x in 0..self.width {
      if let Some(winner) = (0..self.height)
        .map(|y| match self.at((x, y)) {
          TileState::P1 => Some(GamePlayer::Player1),
          TileState::P2 => Some(GamePlayer::Player2),
          TileState::Empty => None,
        })
        .in_a_row(self.in_a_row)
      {
        return GameResult::Win(winner);
      }
    }

    for dxy in 1..(self.width + self.height) {
      if let Some(winner) = (dxy.saturating_sub(self.width)..dxy.min(self.height))
        .map(|d| (dxy - d - 1, d))
        .map(|coord| match self.at(coord) {
          TileState::P1 => Some(GamePlayer::Player1),
          TileState::P2 => Some(GamePlayer::Player2),
          TileState::Empty => None,
        })
        .in_a_row(self.in_a_row)
      {
        return GameResult::Win(winner);
      }
    }

    for dxy in (-(self.height as i32) + 1)..self.width as i32 {
      if let Some(winner) = ((-dxy).max(0) as u32
        ..((self.width as i32 - dxy) as u32).min(self.height))
        .map(|d| ((dxy + d as i32) as u32, d))
        .map(|coord| match self.at(coord) {
          TileState::P1 => Some(GamePlayer::Player1),
          TileState::P2 => Some(GamePlayer::Player2),
          TileState::Empty => None,
        })
        .in_a_row(self.in_a_row)
      {
        return GameResult::Win(winner);
      }
    }

    if self.n_moves_made() == self.width * self.height {
      return GameResult::Tie;
    }

    GameResult::NotFinished
  }
}

impl Debug for ConnectN {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    for y in (0..self.height).rev() {
      for x in 0..self.width {
        write!(
          f,
          "{}",
          match self.at((x, y)) {
            TileState::Empty => ".",
            TileState::P1 => "X",
            TileState::P2 => "O",
          }
        )?;
      }
      writeln!(f)?;
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use crate::{
    test_games::{ConnectMove, ConnectN},
    Game, GamePlayer, GameResult,
  };

  use googletest::{gtest, prelude::*};
  use itertools::Itertools;

  #[gtest]
  fn test_first_moves() {
    let connect_four = ConnectN::new(7, 6, 4);

    expect_that!(
      connect_four.each_move().collect_vec(),
      unordered_elements_are![
        &ConnectMove { col: 0 },
        &ConnectMove { col: 1 },
        &ConnectMove { col: 2 },
        &ConnectMove { col: 3 },
        &ConnectMove { col: 4 },
        &ConnectMove { col: 5 },
        &ConnectMove { col: 6 },
      ]
    );
  }

  #[gtest]
  fn test_second_moves() {
    let mut connect_four = ConnectN::new(7, 6, 4);
    connect_four.make_move(ConnectMove { col: 4 });

    expect_that!(
      connect_four.each_move().collect_vec(),
      unordered_elements_are![
        &ConnectMove { col: 0 },
        &ConnectMove { col: 1 },
        &ConnectMove { col: 2 },
        &ConnectMove { col: 3 },
        &ConnectMove { col: 4 },
        &ConnectMove { col: 5 },
        &ConnectMove { col: 6 },
      ]
    );
  }

  #[gtest]
  fn test_col_full_moves() {
    let mut connect_four = ConnectN::new(7, 6, 4);
    for _ in 0..6 {
      connect_four.make_move(ConnectMove { col: 4 });
    }

    expect_that!(
      connect_four.each_move().collect_vec(),
      unordered_elements_are![
        &ConnectMove { col: 0 },
        &ConnectMove { col: 1 },
        &ConnectMove { col: 2 },
        &ConnectMove { col: 3 },
        &ConnectMove { col: 5 },
        &ConnectMove { col: 6 },
      ]
    );
  }

  #[gtest]
  fn test_not_finished_empty() {
    let connect_four = ConnectN::new(7, 6, 4);
    expect_eq!(connect_four.finished(), GameResult::NotFinished);
  }

  #[gtest]
  fn test_not_finished_one_move() {
    let mut connect_four = ConnectN::new(7, 6, 4);
    connect_four.make_move(ConnectMove { col: 3 });
    expect_eq!(connect_four.finished(), GameResult::NotFinished);
  }

  #[gtest]
  fn test_not_finished_one_move_edge() {
    let mut connect_four = ConnectN::new(5, 4, 3);
    connect_four.make_move(ConnectMove { col: 4 });
    expect_eq!(connect_four.finished(), GameResult::NotFinished);
  }

  #[gtest]
  fn test_win_row() {
    let mut connect_four = ConnectN::new(7, 6, 4);
    connect_four.make_move(ConnectMove { col: 3 });
    connect_four.make_move(ConnectMove { col: 4 });
    connect_four.make_move(ConnectMove { col: 2 });
    connect_four.make_move(ConnectMove { col: 5 });
    connect_four.make_move(ConnectMove { col: 1 });
    connect_four.make_move(ConnectMove { col: 6 });
    connect_four.make_move(ConnectMove { col: 0 });

    expect_eq!(
      connect_four.finished(),
      GameResult::Win(GamePlayer::Player1)
    );
  }

  #[gtest]
  fn test_win_col() {
    let mut connect_four = ConnectN::new(7, 6, 4);
    connect_four.make_move(ConnectMove { col: 3 });
    connect_four.make_move(ConnectMove { col: 4 });
    connect_four.make_move(ConnectMove { col: 3 });
    connect_four.make_move(ConnectMove { col: 4 });
    connect_four.make_move(ConnectMove { col: 3 });
    connect_four.make_move(ConnectMove { col: 4 });
    connect_four.make_move(ConnectMove { col: 3 });

    expect_eq!(
      connect_four.finished(),
      GameResult::Win(GamePlayer::Player1)
    );
  }

  #[gtest]
  fn test_win_diag1() {
    let mut connect_four = ConnectN::new(7, 6, 4);
    connect_four.make_move(ConnectMove { col: 3 });
    connect_four.make_move(ConnectMove { col: 4 });
    connect_four.make_move(ConnectMove { col: 4 });
    connect_four.make_move(ConnectMove { col: 5 });
    connect_four.make_move(ConnectMove { col: 5 });
    connect_four.make_move(ConnectMove { col: 5 });
    connect_four.make_move(ConnectMove { col: 6 });
    connect_four.make_move(ConnectMove { col: 6 });
    connect_four.make_move(ConnectMove { col: 6 });
    connect_four.make_move(ConnectMove { col: 6 });

    expect_eq!(
      connect_four.finished(),
      GameResult::Win(GamePlayer::Player1)
    );
  }

  #[gtest]
  fn test_win_diag2() {
    let mut connect_four = ConnectN::new(7, 6, 4);
    connect_four.make_move(ConnectMove { col: 3 });
    connect_four.make_move(ConnectMove { col: 2 });
    connect_four.make_move(ConnectMove { col: 2 });
    connect_four.make_move(ConnectMove { col: 1 });
    connect_four.make_move(ConnectMove { col: 1 });
    connect_four.make_move(ConnectMove { col: 1 });
    connect_four.make_move(ConnectMove { col: 0 });
    connect_four.make_move(ConnectMove { col: 0 });
    connect_four.make_move(ConnectMove { col: 0 });
    connect_four.make_move(ConnectMove { col: 0 });

    expect_eq!(
      connect_four.finished(),
      GameResult::Win(GamePlayer::Player1)
    );
  }
}
