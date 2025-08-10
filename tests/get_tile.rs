use googletest::{expect_true, gtest};
use itertools::Itertools;
use onoro::{Onoro, OnoroIndex, OnoroPawn};
use rstest::rstest;
use rstest_reuse::{apply, template};
use std::collections::HashMap;

#[template]
#[rstest]
fn many_positions(#[values(onoro::Onoro16::default_start())] onoro: impl Onoro) {}

fn expect_pawns_in_bounds<T: Onoro>(onoro: &T) {
  let n_pawns = 2 * T::pawns_per_player() as i32;
  for pawn in onoro.pawns() {
    expect_true!((0..n_pawns).contains(&pawn.pos().x()));
    expect_true!((0..n_pawns).contains(&pawn.pos().y()));
  }
}

#[apply(many_positions)]
#[gtest]
fn test_get_tile<T: Onoro>(onoro: T) {
  expect_pawns_in_bounds(&onoro);

  let pawns = onoro.pawns().collect_vec();
  let pawn_positions: HashMap<_, _> = pawns
    .iter()
    .map(|pawn| ((pawn.pos().x(), pawn.pos().y()), pawn.color()))
    .collect();

  let n_pawns = 2 * T::pawns_per_player() as u32;
  for y in 0..n_pawns {
    for x in 0..n_pawns {
      use googletest::expect_eq;
      use onoro::TileState;

      let expected_tile = match pawn_positions.get(&(x as i32, y as i32)) {
        Some(&color) => color.into(),
        None => TileState::Empty,
      };

      expect_eq!(onoro.get_tile(T::Index::from_coords(x, y)), expected_tile);
    }
  }
}
