use itertools::interleave;

use crate::{
  error::OnoroError,
  hex_pos::HexPosOffset,
  onoro::{OnoroIndex, OnoroMove},
  onoro_util::{pawns_from_board_string, BoardLayoutPawns},
  Onoro, PawnColor,
};

pub trait OnoroInitialize: Onoro + Sized {
  /// Initializes an empty game. This should not be called outside the `Onoro`
  /// trait.
  ///
  /// # Safety
  ///
  /// Any constructor returning an owned instance of `Onoro` _must_ make at
  /// least one move after initializing an `Onoro` with this function.
  unsafe fn new() -> Self;

  fn default_start() -> Self {
    let mid_idx = ((Self::board_width() - 1) / 2) as u32;
    let mut game = unsafe { Self::new() };
    unsafe {
      game.make_move_unchecked(Self::Move::make_phase1(Self::Index::from_coords(
        mid_idx, mid_idx,
      )));
    }
    game.make_move(Self::Move::make_phase1(Self::Index::from_coords(
      mid_idx + 1,
      mid_idx + 1,
    )));
    game.make_move(Self::Move::make_phase1(Self::Index::from_coords(
      mid_idx + 1,
      mid_idx,
    )));
    game
  }

  fn from_board_string(board_layout: &str) -> Result<Self, OnoroError> {
    let BoardLayoutPawns {
      black_pawns,
      white_pawns,
    } = pawns_from_board_string(board_layout, Self::pawns_per_player())?;

    let mut game = unsafe { Self::new() };
    unsafe {
      game.make_move_unchecked(Self::Move::make_phase1(black_pawns[0]));
    }
    for pos in interleave(white_pawns, black_pawns.into_iter().skip(1)) {
      game.make_move(Self::Move::make_phase1(pos));
    }

    Ok(game)
  }

  fn from_pawns(mut pawns: Vec<(HexPosOffset, PawnColor)>) -> Result<Self, String> {
    let n_pawns = pawns.len();
    debug_assert!(n_pawns <= 2 * Self::pawns_per_player());
    let (min_x, min_y) = pawns
      .iter()
      .fold((i32::MAX, i32::MAX), |(min_x, min_y), (pos, _)| {
        (min_x.min(pos.x()), min_y.min(pos.y()))
      });

    if pawns.iter().any(|(pos, _)| {
      pos.x() - min_x >= Self::board_width() as i32 - 1
        || pos.y() - min_y >= Self::board_width() as i32 - 1
    }) {
      return Err("Pawns stretch beyond the maximum allowed size of the board, meaning this state is invalid.".to_owned());
    }

    let black_count = pawns
      .iter()
      .filter(|(_, color)| matches!(color, PawnColor::Black))
      .count();
    let white_count = n_pawns - black_count;
    if !((black_count - 1)..=black_count).contains(&white_count) {
      return Err(format!(
        "There must be either one fewer or equally many white pawns as there are black. Found {black_count} black and {white_count} white.",
      ));
    }

    // Move all black pawns to the front.
    pawns.sort_by_key(|(_, color)| matches!(color, PawnColor::White));
    for i in 0..(n_pawns - 1) / 2 {
      pawns.swap(2 * i + 1, n_pawns.div_ceil(2) + i);
    }
    debug_assert!(pawns
      .iter()
      .enumerate()
      .all(|(idx, (_, color))| { (idx % 2 == 0) == matches!(color, PawnColor::Black) }));

    Ok(Self::from_indexes(pawns.into_iter().map(|(pos, _)| {
      Self::Index::from_coords((pos.x() - min_x + 1) as u32, (pos.y() - min_y + 1) as u32)
    })))
  }

  fn from_indexes(pawns: impl IntoIterator<Item = Self::Index>) -> Self {
    let mut game = unsafe { Self::new() };
    for idx in pawns {
      unsafe {
        game.make_move_unchecked(Self::Move::make_phase1(idx));
      }
    }
    game
  }

  fn hex_start() -> Self {
    Self::from_board_string(
      ". B W
        W . B
         B W .",
    )
    .unwrap()
  }
}
