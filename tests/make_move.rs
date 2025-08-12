use googletest::{expect_that, gtest, prelude::unordered_elements_are};
use itertools::Itertools;
use onoro::{
  error::OnoroResult,
  test_util::{OnoroCmp, OnoroFactory},
  Onoro,
};
use rstest::rstest;
use rstest_reuse::{apply, template};

struct Onoro16Factory;
impl OnoroFactory for Onoro16Factory {
  type T = onoro_impl::Onoro16;
}

struct AiOnoroFactory;
impl OnoroFactory for AiOnoroFactory {
  type T = ai_gen_onoro::OnoroGame;
}

#[template]
#[rstest]
fn onoro_factory(#[values(Onoro16Factory, AiOnoroFactory)] factory: impl OnoroFactory) {}

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

#[apply(onoro_factory)]
#[gtest]
fn test_make_move_phase2_few_options<T: OnoroFactory>(_factory: T) -> OnoroResult {
  let onoro = T::from_board_string(
    ". . B W B W
      . W . . . B
       B . . . . W
        W . . . . B
         B . . . W .
          W B W B  .",
  )?;
  let moves = all_moves(&onoro);

  expect_that!(
    moves.iter().map(OnoroCmp).collect_vec(),
    unordered_elements_are![
      &OnoroCmp(&T::from_board_string(
        ". . B W B W
          . W . . . B
           . B . . . W
            W . . . . B
             B . . . W .
              W B W B  .",
      )?),
      &OnoroCmp(&T::from_board_string(
        ". . . W B W
          . W B . . B
           B . . . . W
            W . . . . B
             B . . . W .
              W B W B  .",
      )?),
      &OnoroCmp(&T::from_board_string(
        ". . B W B W
          . W . . . B
           B . . . . W
            W . . . B .
             B . . . W .
              W B W B  .",
      )?),
      &OnoroCmp(&T::from_board_string(
        ". . B W B W
          . W . . . B
           B . . . . W
            W . . . . B
             B . . B W .
              W B W .  .",
      )?),
    ]
  );

  Ok(())
}
