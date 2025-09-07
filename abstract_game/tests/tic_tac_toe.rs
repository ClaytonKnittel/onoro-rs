use abstract_game::{
  test_games::{TTTMove, TicTacToe},
  Game, ScoreValue, SimpleSolver, Solver,
};

use googletest::{gtest, prelude::*};
use rstest::rstest;
use rstest_reuse::{apply, template};

#[template]
#[rstest]
fn solvers(#[values(SimpleSolver)] solver: (impl Solver)) {}

#[apply(solvers)]
#[gtest]
fn test_solve(mut solver: impl Solver) {
  let mut ttt = TicTacToe::new();
  {
    let (score, m) = solver.best_move(&ttt, 9);
    expect_eq!(score.score_at_depth(9), ScoreValue::Tie);
    expect_that!(m, some(anything()));
  }

  // . . .
  // . . .
  // X . .
  ttt.make_move(TTTMove::new((0, 0)));
  {
    let (score, m) = solver.best_move(&ttt, 8);
    expect_eq!(score.score_at_depth(8), ScoreValue::Tie);
    expect_that!(
      m,
      some(any![
        eq(TTTMove::new((0, 1))),
        eq(TTTMove::new((1, 1))),
        eq(TTTMove::new((1, 0))),
      ])
    );
  }

  // . . .
  // . . .
  // X . O
  ttt.make_move(TTTMove::new((2, 0)));
  {
    let (score, m) = solver.best_move(&ttt, 7);
    expect_eq!(score.score_at_depth(7), ScoreValue::CurrentPlayerWins);
    expect_that!(m, some(eq(TTTMove::new((2, 2)))));
  }

  // . . X
  // . . .
  // X . O
  ttt.make_move(TTTMove::new((2, 2)));
  {
    let (score, m) = solver.best_move(&ttt, 6);
    expect_eq!(score.score_at_depth(6), ScoreValue::OtherPlayerWins);
    expect_that!(m, some(eq(TTTMove::new((1, 1)))));
  }

  // . . X
  // . O .
  // X . O
  ttt.make_move(TTTMove::new((1, 1)));
  {
    let (score, m) = solver.best_move(&ttt, 5);
    expect_eq!(score.score_at_depth(5), ScoreValue::CurrentPlayerWins);
    expect_that!(m, some(eq(TTTMove::new((0, 2)))));
  }

  // X . X
  // . O .
  // X . O
  ttt.make_move(TTTMove::new((0, 2)));
  {
    let (score, m) = solver.best_move(&ttt, 5);
    expect_eq!(score.score_at_depth(5), ScoreValue::OtherPlayerWins);
    expect_that!(m, some(anything()));
  }

  // X . X
  // O O .
  // X . O
  ttt.make_move(TTTMove::new((0, 1)));
  {
    let (score, m) = solver.best_move(&ttt, 4);
    expect_eq!(score.score_at_depth(4), ScoreValue::CurrentPlayerWins);
    expect_that!(m, some(eq(TTTMove::new((1, 2)))));
  }
}
