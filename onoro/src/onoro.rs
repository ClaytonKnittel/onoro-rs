use itertools::interleave;

use crate::{
  error::OnoroError,
  onoro_util::{pawns_from_board_string, BoardLayoutPawns},
  Move, Pawn, PawnColor,
};

pub trait Onoro: Sized {
  /// Initializes an empty game. This should not be called outside the `Onoro`
  /// trait.
  ///
  /// # Safety
  ///
  /// Any constructor returning an owned instance of `Onoro` _must_ make at
  /// least one move after initializing an `Onoro` with this function.
  unsafe fn new() -> Self;

  fn default_start() -> Self;

  /// Returns the number of pawns each player has.
  fn pawns_per_player() -> usize;

  fn from_board_string(board_layout: &str) -> Result<Self, OnoroError> {
    let BoardLayoutPawns {
      black_pawns,
      white_pawns,
    } = pawns_from_board_string(board_layout, Self::pawns_per_player())?;

    let mut game = unsafe { Self::new() };
    unsafe {
      game.make_move_unchecked(Move::Phase1Move { to: black_pawns[0] });
    }
    for pos in interleave(white_pawns, black_pawns.into_iter().skip(1)) {
      game.make_move(Move::Phase1Move { to: pos });
    }

    Ok(game)
  }

  /// Returns Some(..) if the game is over, containing the color of the player who won.
  fn finished(&self) -> Option<PawnColor>;

  /// Returns an iterator over all pawns in the game. The order does not matter.
  fn pawns(&self) -> impl Iterator<Item = Pawn> + '_;

  /// Returns true if the game is in phase 1, meaning the move made by the next
  /// player is to place a new pawn on the board, not to move an existing pawn.
  fn in_phase1(&self) -> bool;

  /// Returns an iterator over all legal moves that can be made from this state.
  fn each_move(&self) -> impl Iterator<Item = Move>;

  /// Makes a move, mutating the game state.
  fn make_move(&mut self, m: Move);

  /// Make move without checking that we are in the right phase. This is used by
  /// the game constructors to place the first pawn on an empty board.
  ///
  /// # Safety
  ///
  /// This function should not be called outside the Onoro trait.
  unsafe fn make_move_unchecked(&mut self, m: Move) {
    self.make_move(m);
  }
}
