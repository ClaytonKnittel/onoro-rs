use std::collections::HashMap;

use googletest::{expect_that, gtest, prelude::unordered_elements_are};
use itertools::Itertools;
use onoro::{
  error::{OnoroError, OnoroResult},
  Onoro, OnoroIndex, OnoroMoveWrapper, OnoroPawn, PawnColor,
};
use rstest::rstest;
use rstest_reuse::{apply, template};

const BLACK: &PawnColor = &PawnColor::Black;
const WHITE: &PawnColor = &PawnColor::White;

trait OnoroFactory {
  type T: Onoro;

  fn from_board_string(board_string: &str) -> OnoroResult<Self::T> {
    Ok(Self::T::from_board_string(board_string)?)
  }
}

struct Onoro16Factory;
impl OnoroFactory for Onoro16Factory {
  type T = onoro_impl::Onoro16;
}

struct AiOnoroFactory;
impl OnoroFactory for AiOnoroFactory {
  type T = ai_gen_onoro::OnoroGame;
}

struct Phase2Move<Index: OnoroIndex> {
  from: Index,
  to: Index,
}

#[template]
#[rstest]
fn onoro_factory(#[values(Onoro16Factory, AiOnoroFactory)] factory: impl OnoroFactory) {}

type NeighborColors = Vec<PawnColor>;

fn neighbor_colors<T: Onoro>(onoro: &T, pos: T::Index) -> NeighborColors {
  onoro
    .pawns()
    .filter_map(|pawn| (pawn.pos().adjacent(pos) && pos != pawn.pos()).then_some(pawn.color()))
    .collect()
}

fn neighbor_colors_excluding<T: Onoro>(
  onoro: &T,
  pos: T::Index,
  exclude: T::Index,
) -> NeighborColors {
  onoro
    .pawns()
    .filter_map(|pawn| {
      (pawn.pos().adjacent(pos) && pos != pawn.pos() && exclude != pawn.pos())
        .then_some(pawn.color())
    })
    .collect()
}

fn collect_phase1_moves<T: Onoro>(onoro: &T) -> OnoroResult<Vec<T::Index>> {
  onoro
    .each_move()
    .map(|m| onoro.to_move_wrapper(m))
    .map(|m| {
      if let OnoroMoveWrapper::Phase1 { to } = m {
        Ok(to)
      } else {
        Err(OnoroError::new(format!("Expected phase 1 moves, found {m:?}")).into())
      }
    })
    .collect()
}

fn phase1_move_adjacencies<T: Onoro>(onoro: &T) -> OnoroResult<Vec<NeighborColors>> {
  Ok(
    collect_phase1_moves(onoro)?
      .into_iter()
      .map(|m| neighbor_colors(onoro, m))
      .collect_vec(),
  )
}

fn collect_phase2_moves<T: Onoro>(onoro: &T) -> OnoroResult<Vec<Phase2Move<T::Index>>> {
  onoro
    .each_move()
    .map(|m| onoro.to_move_wrapper(m))
    .map(|m| {
      if let OnoroMoveWrapper::Phase2 { from, to } = m {
        Ok(Phase2Move { from, to })
      } else {
        Err(OnoroError::new(format!("Expected phase 2 moves, found {m:?}")).into())
      }
    })
    .collect()
}

fn phase2_move_adjacencies<T: Onoro>(
  onoro: &T,
) -> OnoroResult<Vec<(NeighborColors, Vec<NeighborColors>)>> {
  Ok(
    collect_phase2_moves(onoro)?
      .into_iter()
      .fold(HashMap::<_, Vec<_>>::new(), |mut map, m| {
        map
          .entry((m.from.x(), m.from.y()))
          .or_default()
          .push(neighbor_colors_excluding(onoro, m.to, m.from));
        map
      })
      .into_iter()
      .map(|(from, to_adj)| {
        (
          neighbor_colors(onoro, T::Index::from_coords(from.0 as u32, from.1 as u32)),
          to_adj,
        )
      })
      .collect_vec(),
  )
}

#[apply(onoro_factory)]
#[gtest]
fn test_each_move_default_start<T: OnoroFactory>(_factory: T) -> OnoroResult {
  let onoro = T::from_board_string(
    ". W
      B B",
  )?;
  let adj = phase1_move_adjacencies(&onoro)?;

  expect_that!(
    adj,
    unordered_elements_are![
      unordered_elements_are![BLACK, WHITE],
      unordered_elements_are![BLACK, WHITE],
      unordered_elements_are![BLACK, BLACK],
    ]
  );

  Ok(())
}

#[apply(onoro_factory)]
#[gtest]
fn test_each_move_hex_start<T: OnoroFactory>(_factory: T) -> OnoroResult {
  let onoro = T::from_board_string(
    ". W B
      B . W
       W B .",
  )?;
  let adj = phase1_move_adjacencies(&onoro)?;

  expect_that!(
    adj,
    unordered_elements_are![
      unordered_elements_are![BLACK, WHITE],
      unordered_elements_are![BLACK, WHITE],
      unordered_elements_are![BLACK, WHITE],
      unordered_elements_are![BLACK, WHITE],
      unordered_elements_are![BLACK, WHITE],
      unordered_elements_are![BLACK, WHITE],
      unordered_elements_are![BLACK, BLACK, BLACK, WHITE, WHITE, WHITE],
    ]
  );

  Ok(())
}

#[apply(onoro_factory)]
#[gtest]
fn test_each_move_line<T: OnoroFactory>(_factory: T) -> OnoroResult {
  let onoro = T::from_board_string(
    ". W . . . .
      B B W B W W
       . . . . B .",
  )?;
  let adj = phase1_move_adjacencies(&onoro)?;

  expect_that!(
    adj,
    unordered_elements_are![
      unordered_elements_are![BLACK, BLACK],
      unordered_elements_are![WHITE, WHITE],
      unordered_elements_are![BLACK, WHITE],
      unordered_elements_are![BLACK, WHITE],
      unordered_elements_are![BLACK, WHITE],
      unordered_elements_are![BLACK, WHITE],
      unordered_elements_are![BLACK, WHITE],
      unordered_elements_are![BLACK, WHITE],
      unordered_elements_are![BLACK, WHITE, WHITE],
      unordered_elements_are![BLACK, BLACK, WHITE],
    ]
  );

  Ok(())
}

#[apply(onoro_factory)]
#[gtest]
fn test_each_move_phase2_few_options<T: OnoroFactory>(_factory: T) -> OnoroResult {
  let onoro = T::from_board_string(
    ". . B W B W
      . W . . . B
       B . . . . W
        W . . . . B
         B . . . W .
          W B W B  .",
  )?;
  let adj = phase2_move_adjacencies(&onoro)?;

  let unit = || {
    (
      unordered_elements_are![WHITE, WHITE],
      unordered_elements_are![unordered_elements_are![WHITE, WHITE]],
    )
  };
  expect_that!(adj, unordered_elements_are![unit(), unit(), unit(), unit()]);

  Ok(())
}
