use crate::{Game, GameResult, Score, Solver};

pub struct SimpleSolver;

impl SimpleSolver {
  fn score_for_game<G: Game>(game: &G, depth: u32) -> Score {
    match game.finished() {
      GameResult::Win(player) => {
        if player == game.current_player() {
          Score::lose(1)
        } else {
          Score::win(1)
        }
      }
      GameResult::Tie => Score::guaranteed_tie(),
      GameResult::NotFinished => Self::solve_impl(game, depth - 1).backstep(),
    }
  }

  fn solve_impl<G: Game>(game: &G, depth: u32) -> Score {
    debug_assert!(matches!(game.finished(), GameResult::NotFinished));
    if depth == 0 {
      return Score::no_info();
    }

    game
      .each_move()
      .map(|m| game.with_move(m))
      .map(|next_game| Self::score_for_game(&next_game, depth))
      .max()
      // If you can't make a move, you lose.
      .unwrap_or(Score::lose(1))
  }
}

impl Solver for SimpleSolver {
  fn best_move<G: Game>(&mut self, game: &G, depth: u32) -> (Score, Option<G::Move>) {
    debug_assert!(matches!(game.finished(), GameResult::NotFinished));
    if depth == 0 {
      return (Score::no_info(), None);
    }

    game
      .each_move()
      .map(|m| {
        let next_game = game.with_move(m);
        let score = Self::score_for_game(&next_game, depth);
        (score, Some(m))
      })
      .max_by_key(|(score, _)| score.clone())
      // If you can't make a move, you lose.
      .unwrap_or((Score::lose(1), None))
  }
}

#[cfg(test)]
mod tests {
  use crate::{test_games::Nim, ScoreValue, SimpleSolver, Solver};

  use googletest::{gtest, prelude::*};

  #[gtest]
  fn test_solve_nim() {
    for sticks in 1..=20 {
      let depth = sticks + 1;
      let expected_winner = sticks % 3 != 0;

      let mut solver = SimpleSolver;
      let (score, best_move) = solver.best_move(&Nim::new(sticks), sticks + 1);

      expect_eq!(
        score.score_at_depth(depth),
        if expected_winner {
          ScoreValue::CurrentPlayerWins
        } else {
          ScoreValue::OtherPlayerWins
        },
        "Game with {sticks} sticks"
      );
      if expected_winner {
        expect_that!(best_move, some(eq(sticks % 3)));
      } else {
        expect_that!(best_move, some(anything()));
      }
    }
  }
}
