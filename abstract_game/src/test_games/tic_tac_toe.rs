use std::fmt::Debug;

use crate::{Game, GameMoveIterator, GamePlayer, GameResult};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TTTMove(u32);

impl TTTMove {
  pub fn new(coord: (u32, u32)) -> Self {
    Self(0x0001_0001 << (coord.0 + coord.1 * 3))
  }

  pub fn board_index(&self) -> u32 {
    self.0.trailing_zeros() % 16
  }

  pub fn x(&self) -> u32 {
    self.board_index() % 3
  }

  pub fn y(&self) -> u32 {
    self.board_index() / 3
  }
}

impl Debug for TTTMove {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "({}, {})", self.x(), self.y(),)
  }
}

pub struct TTTMoveGen {
  move_mask: u32,
}

impl GameMoveIterator for TTTMoveGen {
  type Item = TTTMove;
  type Game = TicTacToe;

  fn next(&mut self, game: &TicTacToe) -> Option<TTTMove> {
    let mut move_mask = self.move_mask;
    while move_mask != 0x0200_0200 {
      let next_mask = move_mask << 1;

      if game.board & move_mask == 0 {
        self.move_mask = next_mask;
        return Some(TTTMove(move_mask));
      }

      move_mask = next_mask;
    }
    None
  }
}

#[derive(Clone)]
pub struct TicTacToe {
  board: u32,
  current_player: GamePlayer,
}

impl TicTacToe {
  pub fn new() -> Self {
    Self {
      board: 0,
      current_player: GamePlayer::Player1,
    }
  }

  fn turn_mask(&self) -> u32 {
    if self.current_player.is_p1() {
      0x0000_ffff
    } else {
      0xffff_0000
    }
  }
}

impl Default for TicTacToe {
  fn default() -> Self {
    Self::new()
  }
}

impl Game for TicTacToe {
  type Move = TTTMove;
  type MoveGenerator = TTTMoveGen;

  fn move_generator(&self) -> TTTMoveGen {
    TTTMoveGen {
      move_mask: 0x0001_0001,
    }
  }

  fn make_move(&mut self, m: TTTMove) {
    debug_assert_eq!(self.board & m.0, 0);
    self.board += m.0 & self.turn_mask();
    self.current_player = self.current_player.opposite();
  }

  fn current_player(&self) -> GamePlayer {
    self.current_player
  }

  fn finished(&self) -> GameResult {
    // Check for 3 in a row, column, or diagonal.
    let board = self.board;

    let three_in_a_row = (board & (board >> 1) & (board >> 2)) != 0;
    let three_in_a_col = (board & (board >> 3) & (board >> 6)) != 0;

    let contains_bits = |board: u32, bits: u32| -> bool { board & bits == bits };
    let diag_tl_to_br = contains_bits(board, 0x0000_0111) || contains_bits(board, 0x0111_0000);
    let diag_tr_to_bl = contains_bits(board, 0x0000_0054) || contains_bits(board, 0x0054_0000);

    if three_in_a_row || three_in_a_col || diag_tl_to_br || diag_tr_to_bl {
      GameResult::Win(self.current_player.opposite())
    } else if board.count_ones() == 9 {
      GameResult::Tie
    } else {
      GameResult::NotFinished
    }
  }
}

impl Debug for TicTacToe {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let tile_at = |coord: (u32, u32)| {
      let m = TTTMove::new(coord);
      let masked = self.board & m.0;
      if masked == 0 {
        '.'
      } else if masked < 0x0001_0000 {
        'X'
      } else {
        'O'
      }
    };
    write!(
      f,
      "{}{}{}\n{}{}{}\n{}{}{}",
      tile_at((0, 2)),
      tile_at((1, 2)),
      tile_at((2, 2)),
      tile_at((0, 1)),
      tile_at((1, 1)),
      tile_at((2, 1)),
      tile_at((0, 0)),
      tile_at((1, 0)),
      tile_at((2, 0)),
    )
  }
}

#[cfg(test)]
mod tests {
  use std::fmt::Debug;

  use googletest::{
    description::Description,
    gtest,
    matcher::{Matcher, MatcherResult},
    prelude::*,
  };
  use itertools::Itertools;

  use crate::{
    test_games::{TTTMove, TicTacToe},
    Game, GameResult,
  };

  #[gtest]
  fn test_first_moves() {
    expect_that!(
      TicTacToe::new().each_move().collect_vec(),
      unordered_elements_are![
        &TTTMove::new((0, 0)),
        &TTTMove::new((0, 1)),
        &TTTMove::new((0, 2)),
        &TTTMove::new((1, 0)),
        &TTTMove::new((1, 1)),
        &TTTMove::new((1, 2)),
        &TTTMove::new((2, 0)),
        &TTTMove::new((2, 1)),
        &TTTMove::new((2, 2)),
      ]
    );
  }

  #[gtest]
  fn test_second_moves() {
    let mut ttt = TicTacToe::new();
    ttt.make_move(TTTMove::new((1, 1)));
    expect_that!(
      ttt.each_move().collect_vec(),
      unordered_elements_are![
        &TTTMove::new((0, 0)),
        &TTTMove::new((0, 1)),
        &TTTMove::new((0, 2)),
        &TTTMove::new((1, 0)),
        &TTTMove::new((1, 2)),
        &TTTMove::new((2, 0)),
        &TTTMove::new((2, 1)),
        &TTTMove::new((2, 2)),
      ]
    );
  }

  #[derive(MatcherBase)]
  struct EndsInWinMatcher;

  impl<T> Matcher<T> for EndsInWinMatcher
  where
    T: Copy + Debug + IntoIterator<Item = TTTMove>,
  {
    fn matches(&self, actual: T) -> MatcherResult {
      let moves = actual.into_iter().collect_vec();
      let n_moves = moves.len();

      let mut ttt = TicTacToe::new();
      for (i, m) in moves.into_iter().enumerate() {
        ttt = ttt.with_move(m);
        if i == n_moves - 1 {
          if ttt.finished() != GameResult::Win(ttt.current_player.opposite()) {
            return MatcherResult::NoMatch;
          } else {
            return MatcherResult::Match;
          }
        } else if ttt.finished() != GameResult::NotFinished {
          return MatcherResult::NoMatch;
        }
      }

      unreachable!();
    }

    fn describe(&self, matcher_result: MatcherResult) -> Description {
      match matcher_result {
        MatcherResult::Match => Description::new().text("Expected all ties until the last move."),
        MatcherResult::NoMatch => {
          Description::new().text("Not all states were ties until the last move.")
        }
      }
    }

    fn explain_match(&self, actual: T) -> Description {
      Description::new().text(format!(
        "{:?}",
        actual
          .into_iter()
          .scan(TicTacToe::new(), |ttt, m| {
            *ttt = ttt.with_move(m);
            Some(ttt.finished())
          })
          .collect_vec()
      ))
    }
  }

  fn ends_in_win() -> EndsInWinMatcher {
    EndsInWinMatcher
  }

  #[gtest]
  fn test_win_row() {
    expect_that!(
      [
        TTTMove::new((0, 0)),
        TTTMove::new((2, 0)),
        TTTMove::new((0, 1)),
        TTTMove::new((1, 1)),
        TTTMove::new((0, 2)),
      ],
      ends_in_win()
    );
  }

  #[gtest]
  fn test_win_col() {
    expect_that!(
      [
        TTTMove::new((0, 1)),
        TTTMove::new((2, 0)),
        TTTMove::new((2, 1)),
        TTTMove::new((1, 2)),
        TTTMove::new((1, 1)),
      ],
      ends_in_win()
    );
  }

  #[gtest]
  fn test_win_diag1() {
    expect_that!(
      [
        TTTMove::new((0, 0)),
        TTTMove::new((2, 0)),
        TTTMove::new((1, 1)),
        TTTMove::new((1, 2)),
        TTTMove::new((2, 2)),
      ],
      ends_in_win()
    );
  }

  #[gtest]
  fn test_win_diag2() {
    expect_that!(
      [
        TTTMove::new((0, 2)),
        TTTMove::new((2, 1)),
        TTTMove::new((1, 1)),
        TTTMove::new((1, 2)),
        TTTMove::new((2, 0)),
      ],
      ends_in_win()
    );
  }
}
