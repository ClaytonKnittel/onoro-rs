pub enum TileState {
  Empty,
  Black,
  White,
}

/// An Onoro game state with `N / 2` pawns per player.
pub struct Onoro<const N: u32> {}

impl<const N: u32> Onoro<N> {
  /// Returns the width of the game board. This is also the upper bound on the
  /// x and y coordinate values in PackedIdx.
  pub const fn board_width() -> u32 {
    N
  }

  /// Returns the total number of tiles in the game board.
  pub const fn board_size() -> u32 {
    Self::board_width() * Self::board_width()
  }

  pub const fn symm_state_table_width() -> u32 {
    N
  }

  /// Returns the size of the symm state table, in terms of number of elements.
  pub const fn symm_state_table_size() -> u32 {
    N
  }
}
