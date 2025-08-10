use googletest::{expect_eq, expect_that, expect_true, gtest, prelude::none};
use itertools::Itertools;
use onoro::{Onoro, OnoroPawn, PawnColor};
use rstest::rstest;
use rstest_reuse::{apply, template};

#[template]
#[rstest]
fn default_start(#[values(onoro_impl::Onoro16::default_start())] onoro: impl Onoro) {}

#[apply(default_start)]
#[gtest]
fn test_pawns_in_play(onoro: impl Onoro) {
  expect_eq!(onoro.pawns_in_play(), 3);
}

#[apply(default_start)]
#[gtest]
fn test_finished(onoro: impl Onoro) {
  expect_that!(onoro.finished(), none());
}

#[apply(default_start)]
#[gtest]
fn test_turn(onoro: impl Onoro) {
  expect_eq!(onoro.turn(), PawnColor::White);
}

#[apply(default_start)]
#[gtest]
fn test_pawns(onoro: impl Onoro) {
  let pawns = onoro.pawns().collect_vec();
  expect_eq!(pawns.len(), 3);

  for pawn in pawns {
    expect_eq!(onoro.get_tile(pawn.pos()), pawn.color().into());
  }
}

#[apply(default_start)]
#[gtest]
fn test_in_phase1(onoro: impl Onoro) {
  expect_true!(onoro.in_phase1());
}
