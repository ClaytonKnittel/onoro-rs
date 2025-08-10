use googletest::{expect_true, gtest};
use itertools::Itertools;
use onoro::{Onoro, OnoroIndex, OnoroPawn};
use rstest::rstest;
use rstest_reuse::{apply, template};
use std::collections::HashMap;

const BOARD_POSITIONS: [&str; 10] = [
  ". . . . . . . 
    . . W . . . . 
     . W W B B B . 
      . . . . W . . 
       . . . . W . . 
        . . . B B . . 
         . . . B B . . 
          . . . . W . . 
           . . . . W . . 
            . . . B W . . 
             . . . . . . .",
  ". . . . . . . . . . . 
    . . . . . . . . B W . 
     . . B B . . B W W W . 
      . . W . . W B . . . . 
       . . B . B . . . . . . 
        . B W W . . . . . . . 
         . . . . . . . . . . .",
  ". . . . . . . . . . . . 
    . . . . . . . . . W W . 
     . . . . . . . . . W . . 
      . . . . . B W . B . . . 
       . . . . . W B . B . . . 
        . . . . W . B W . . . . 
         . . . W . . . . . . . . 
          . . B . . . . . . . . . 
           . B B . . . . . . . . . 
            . . . . . . . . . . . .",
  ". . . . . . . . . . . . 
    . . . . . . . . . . B . 
     . . . . . . . W B W W . 
      . . . . . B B . . . . . 
       . . . W W . . . . . . . 
        . . B . W . . . . . . . 
         . B W . B B . . . . . . 
          . . . . W . . . . . . . 
           . . . . . . . . . . . .",
  ". . . . . . . . 
    . . . . . W . . 
     . . . . B B B . 
      . . B W . B . . 
       . . W . W . B . 
        . B W . B W W . 
         . . . . . W . . 
          . . . . . . . .",
  ". . . . . . . . . . . 
    . . . . . . . . . W . 
     . . . B W . . . W B . 
      . . W . W . B W . . . 
       . B B B . . W . . . . 
        . . . B B W . . . . . 
         . . . . . . . . . . .",
  ". . . . . . . . . . . 
    . . . . . . . . W B . 
     . . . . . . . B W . . 
      . . . . . . . B . . . 
       . . W . . . . W . . . 
        . B W . . . B . . . . 
         . . W B B W B . . . . 
          . . . . W . . . . . . 
           . . . . . . . . . . .",
  ". . . . . . . 
    . . . . . W . 
     . . . . W W . 
      . B W B . . . 
       . B B W W . . 
        . . . B . . . 
         . . . B . . . 
          . . . W B B . 
           . . . . W . . 
            . . . . . . .",
  ". . . . . . . . . . . . 
    . . . . . . . . . W W . 
     . . . . . . . . B B . . 
      . . . . . . W B W . . . 
       . . . . . B . B . . . . 
        . . . . . B . . . . . . 
         . . . W W B . . . . . . 
          . B W . . . . . . . . . 
           . W . . . . . . . . . . 
            . . . . . . . . . . . .",
  ". . . . . . 
    . . . B . . 
     . . W W W . 
      . . . B . . 
       . . B B . . 
        . . B . . . 
         . . W B . . 
          . . . W . . 
           . . . W . . 
            . . B . . . 
             . . B . . . 
              . W W . . . 
               . . . . . .",
];

#[template]
#[rstest]
fn many_positions(
  #[values(
    onoro_impl::Onoro16::from_board_string(BOARD_POSITIONS[0]).unwrap(),
    onoro_impl::Onoro16::from_board_string(BOARD_POSITIONS[1]).unwrap(),
    onoro_impl::Onoro16::from_board_string(BOARD_POSITIONS[2]).unwrap(),
    onoro_impl::Onoro16::from_board_string(BOARD_POSITIONS[3]).unwrap(),
    onoro_impl::Onoro16::from_board_string(BOARD_POSITIONS[4]).unwrap(),
    onoro_impl::Onoro16::from_board_string(BOARD_POSITIONS[5]).unwrap(),
    onoro_impl::Onoro16::from_board_string(BOARD_POSITIONS[6]).unwrap(),
    onoro_impl::Onoro16::from_board_string(BOARD_POSITIONS[7]).unwrap(),
    onoro_impl::Onoro16::from_board_string(BOARD_POSITIONS[8]).unwrap(),
    onoro_impl::Onoro16::from_board_string(BOARD_POSITIONS[9]).unwrap(),
  )]
  onoro: impl Onoro,
) {
}

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
