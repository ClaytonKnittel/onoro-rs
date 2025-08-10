use googletest::{expect_that, gtest, prelude::unordered_elements_are};
use itertools::Itertools;
use onoro::{
  error::{OnoroError, OnoroResult},
  Onoro, OnoroIndex, OnoroMoveWrapper, OnoroPawn, PawnColor,
};
use onoro_impl::Onoro16;
use rstest::rstest;
use rstest_reuse::{apply, template};

const BLACK: PawnColor = PawnColor::Black;
const WHITE: PawnColor = PawnColor::White;

trait OnoroFactory {
  type T: Onoro;

  fn from_board_string(board_string: &str) -> OnoroResult<Self::T> {
    Ok(Self::T::from_board_string(board_string)?)
  }
}

struct Onoro16Factory;
impl OnoroFactory for Onoro16Factory {
  type T = Onoro16;
}

#[template]
#[rstest]
fn many_positions(#[values(Onoro16Factory)] factory: impl OnoroFactory) {}

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

fn phase1_move_adjacencies<T: Onoro>(onoro: &T) -> OnoroResult<Vec<Vec<PawnColor>>> {
  let pawn_positions = onoro.pawns().collect_vec();
  Ok(
    collect_phase1_moves(onoro)?
      .into_iter()
      .map(|m| {
        pawn_positions
          .iter()
          .filter_map(|pawn| pawn.pos().adjacent(m).then_some(pawn.color()))
          .collect_vec()
      })
      .collect_vec(),
  )
}

#[apply(many_positions)]
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
      unordered_elements_are![&BLACK, &WHITE],
      unordered_elements_are![&BLACK, &WHITE],
      unordered_elements_are![&BLACK, &BLACK],
    ]
  );

  Ok(())
}

#[apply(many_positions)]
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
      unordered_elements_are![&BLACK, &WHITE],
      unordered_elements_are![&BLACK, &WHITE],
      unordered_elements_are![&BLACK, &WHITE],
      unordered_elements_are![&BLACK, &WHITE],
      unordered_elements_are![&BLACK, &WHITE],
      unordered_elements_are![&BLACK, &WHITE],
      unordered_elements_are![&BLACK, &BLACK, &BLACK, &WHITE, &WHITE, &WHITE],
    ]
  );

  Ok(())
}

#[apply(many_positions)]
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
      unordered_elements_are![&BLACK, &BLACK],
      unordered_elements_are![&WHITE, &WHITE],
      unordered_elements_are![&BLACK, &WHITE],
      unordered_elements_are![&BLACK, &WHITE],
      unordered_elements_are![&BLACK, &WHITE],
      unordered_elements_are![&BLACK, &WHITE],
      unordered_elements_are![&BLACK, &WHITE],
      unordered_elements_are![&BLACK, &WHITE],
      unordered_elements_are![&BLACK, &WHITE, &WHITE],
      unordered_elements_are![&BLACK, &BLACK, &WHITE],
    ]
  );

  Ok(())
}
