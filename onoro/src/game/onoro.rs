use super::{
  hex_pos::{HexPos16, HexPos32},
  onoro_state::OnoroState,
  packed_idx::{IdxOffset, PackedIdx},
  packed_score::PackedScore,
  score::Score,
};

pub enum TileState {
  Empty,
  Black,
  White,
}

/// An Onoro game state with `N / 2` pawns per player.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Onoro<const N: usize> {
  /// Array of indexes of pawn positions. Odd entries (even index) are black
  /// pawns, the others are white. Filled from lowest to highest index as the
  /// first phase proceeds.
  pawn_poses: [PackedIdx; N],
  score: PackedScore<OnoroState>,
  // Sum of all HexPos's of pieces on the board
  sum_of_mass: HexPos16,
  hash: u64,
}

impl<const N: usize> Onoro<N> {
  pub fn new() -> Self {
    Self {
      pawn_poses: [PackedIdx::null(); N],
      score: PackedScore::new(Score::tie(0), OnoroState::new()),
      sum_of_mass: HexPos16::origin(),
      hash: 0,
    }
  }

  fn onoro_state(&self) -> &OnoroState {
    self.score.packed_data()
  }

  fn mut_onoro_state(&mut self) -> &mut OnoroState {
    self.score.mut_packed_data()
  }

  /// Returns the width of the game board. This is also the upper bound on the
  /// x and y coordinate values in PackedIdx.
  pub const fn board_width() -> usize {
    N
  }

  /// Returns the total number of tiles in the game board.
  pub const fn board_size() -> usize {
    Self::board_width() * Self::board_width()
  }

  pub const fn symm_state_table_width() -> usize {
    N
  }

  /// Returns the size of the symm state table, in terms of number of elements.
  pub const fn symm_state_table_size() -> usize {
    Self::symm_state_table_width() * Self::symm_state_table_width()
  }

  /// Sets the pawn at index `i` to `pos`. This will mutate the state of the
  /// game.
  pub fn make_move(&mut self, i: usize, pos: PackedIdx) {
    let mut com_offset: HexPos32 = pos.into();

    let prev_idx = self.pawn_poses[i];
    if prev_idx != PackedIdx::null() {
      com_offset -= prev_idx.into();
    }

    self.pawn_poses[i] = pos;
    // The amount to shift the whole board by. This will keep pawns off the
    // outer perimeter.
    let shift = Self::calc_move_shift(&pos);
    // Only shift the pawns if we have to, to avoid extra memory
    // reading/writing.
    if shift != IdxOffset::identity() {
      self.pawn_poses.iter_mut().for_each(|pos| {
        if *pos != PackedIdx::null() {
          *pos += shift;
        }
      });
    }

    self.sum_of_mass += com_offset.into();
    self.mut_onoro_state().set_hashed(false);

    // Check for a win
  }

  /// Given the position of a newly placed/moved pawn, returns the offset to
  /// apply to all positions on the board.
  fn calc_move_shift(m: &PackedIdx) -> IdxOffset {
    let mut offset = IdxOffset::new(0, 0);

    if m.y() == 0 {
      offset = IdxOffset::new(0, 1);
    } else if m.y() == Self::board_width() as u32 - 1 {
      offset = IdxOffset::new(0, -1);
    }
    if m.x() == 0 {
      offset += IdxOffset::new(1, 0);
    } else if m.x() == Self::board_width() as u32 - 1 {
      offset += IdxOffset::new(-1, 0);
    }

    offset
  }
}
