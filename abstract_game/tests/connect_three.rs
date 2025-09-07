use abstract_game::{
  test_games::{ConnectMove, ConnectN},
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
  let conn = ConnectN::new(4, 3, 3);

  let (score, m) = solver.best_move(&conn, 12);
  expect_eq!(score.score_at_depth(12), ScoreValue::CurrentPlayerWins);
  expect_that!(
    m,
    some(any![eq(ConnectMove { col: 1 }), eq(ConnectMove { col: 2 })])
  );
}

#[apply(solvers)]
#[gtest]
fn test_lose_in_corner(mut solver: impl Solver) {
  let mut conn = ConnectN::new(4, 3, 3);
  conn.make_move(ConnectMove { col: 0 });

  let (score, m) = solver.best_move(&conn, 12);
  expect_eq!(score.score_at_depth(12), ScoreValue::OtherPlayerWins);
  expect_that!(
    m,
    some(any![eq(ConnectMove { col: 1 }), eq(ConnectMove { col: 2 })])
  );
}
