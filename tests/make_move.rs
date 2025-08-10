use std::{collections::HashMap, fmt::Debug};

use googletest::{expect_that, gtest, prelude::unordered_elements_are};
use itertools::Itertools;
use onoro::{error::OnoroResult, Onoro, OnoroIndex, OnoroPawn, PawnColor};
use onoro_impl::Onoro16;
use rstest::rstest;
use rstest_reuse::{apply, template};

trait OnoroFactory {
  type T: Onoro + Clone + Debug;

  fn from_board_string(board_string: &str) -> OnoroResult<Self::T> {
    Ok(Self::T::from_board_string(board_string)?)
  }
}

struct Onoro16Factory;
impl OnoroFactory for Onoro16Factory {
  type T = Onoro16;
}

fn pawn_map<T: Onoro>(onoro: &T) -> HashMap<(i32, i32), PawnColor> {
  let min_x = onoro.pawns().map(|pawn| pawn.pos().x()).min().unwrap();
  let min_y = onoro.pawns().map(|pawn| pawn.pos().y()).min().unwrap();
  onoro
    .pawns()
    .map(|pawn| {
      (
        (pawn.pos().x() - min_x, pawn.pos().y() - min_y),
        pawn.color(),
      )
    })
    .collect()
}

#[derive(Debug, Clone, Copy)]
struct OnoroCmp<'a, T: Onoro + Debug>(&'a T);
impl<'a, T: Onoro + Debug> PartialEq for OnoroCmp<'a, T> {
  fn eq(&self, other: &Self) -> bool {
    pawn_map(self.0) == pawn_map(other.0)
  }
}
impl<'a, T: Onoro + Debug> Eq for OnoroCmp<'a, T> {}

#[template]
#[rstest]
fn onoro_factory(#[values(Onoro16Factory)] factory: impl OnoroFactory) {}

fn all_moves<T: Onoro + Clone>(onoro: &T) -> Vec<T> {
  onoro
    .each_move()
    .map(|m| {
      let mut copy = onoro.clone();
      copy.make_move(m);
      copy
    })
    .collect()
}

#[apply(onoro_factory)]
#[gtest]
fn test_make_move_default_start<T: OnoroFactory>(_factory: T) -> OnoroResult {
  let onoro = T::from_board_string(
    ". W
      B B",
  )?;

  let moves = all_moves(&onoro);

  expect_that!(
    moves.iter().map(OnoroCmp).collect_vec(),
    unordered_elements_are![
      &OnoroCmp(&T::from_board_string(
        "W W
          B B"
      )?),
      &OnoroCmp(&T::from_board_string(
        ". W W
          B B ."
      )?),
      &OnoroCmp(&T::from_board_string(
        ". W
          B B
           W ."
      )?),
    ]
  );

  Ok(())
}

#[apply(onoro_factory)]
#[gtest]
fn test_make_move_hex_start<T: OnoroFactory>(_factory: T) -> OnoroResult {
  let onoro = T::from_board_string(
    ". W B
      B . W
       W B .",
  )?;

  let moves = all_moves(&onoro);

  expect_that!(
    moves.iter().map(OnoroCmp).collect_vec(),
    unordered_elements_are![
      &OnoroCmp(&T::from_board_string(
        "B W B
          B . W
           W B ."
      )?),
      &OnoroCmp(&T::from_board_string(
        ". . B
          . W B
           B . W
            W B ."
      )?),
      &OnoroCmp(&T::from_board_string(
        ". W B B
          B . W .
           W B . ."
      )?),
      &OnoroCmp(&T::from_board_string(
        ". W B
          B . W
           W B B"
      )?),
      &OnoroCmp(&T::from_board_string(
        ". W B
          B . W
           W B .
            B . ."
      )?),
      &OnoroCmp(&T::from_board_string(
        ". . W B
          . B . W
           B W B ."
      )?),
      &OnoroCmp(&T::from_board_string(
        ". W B
          B B W
           W B ."
      )?),
    ]
  );

  Ok(())
}
