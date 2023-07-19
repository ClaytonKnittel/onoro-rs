use std::cmp;

use crate::util::broadcast_u8_to_u64;

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
  pub fn set_tile(&mut self, i: usize, pos: PackedIdx) {
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
    let finished = self.check_win((pos + shift).into());
    self.mut_onoro_state().set_finished(finished);
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

  fn check_win(&self, last_move: HexPos32) -> bool {
    // Bitvector of positions occupied by pawns of this color along the 3 lines
    // extending out from last_move. Intentionally leave a zero bit between each
    // of the 3 sets so they can't form a continuous string of 1's across
    // borders.
    // - s[0-15]: line running along the x-axis, with bit i corresponding to
    //     (x, i)
    // - s[17-32]: line running along the line x = y, with bit i corresponding to
    //     (x - min(x, y) + i, y - min(x, y) + i).
    // - s[34-49]: line running along the y-axis, with bit i corresponding to
    //     (i, y)
    // let mut s = (0x1u64 << last_move.x())
    //   | (0x20000u64 << cmp::min(last_move.x(), last_move.y()))
    //   | (0x400000000u64 << last_move.y());
    let mut s = 0;

    // Unsafe pawn iteration: rely on the fact that idx_t::null_idx() will not
    // complete a line in the first phase of the game (can't reach the border
    // without being able to move pawns), and for phase two, all pawns are
    // placed, so this is safe.
    for i in (0..N)
      // If it is currently the black player's turn, then white placed the last
      // piece at `last_move`, so check if white is winning. Otherwise, check if
      // black is winning.
      .skip(self.onoro_state().black_turn() as usize)
      .step_by(2)
    {
      let pos: HexPos32 = self.pawn_poses[i].into();
      let delta = pos - last_move;
      let dx = delta.x();
      let dy = delta.y();

      s |= if dy == 0 { 0x1u64 } else { 0 } << pos.x();
      s |= if dx == dy { 0x20000u64 } else { 0 } << cmp::min(pos.x(), pos.y());
      s |= if dx == 0 { 0x400000000u64 } else { 0 } << pos.y();
    }

    // Check if any 4 bits in a row are set:
    s = s & (s << 2);
    s = s & (s << 1);
    s != 0
  }

  /// Given a position on the board, returns the tile state of that position,
  /// i.e. the color of the piece on that tile, or `Empty` if no piece is there.
  ///
  /// TODO: perf benchmark this against `get_tile`.
  fn get_tile_slow(&self, idx: PackedIdx) -> TileState {
    match self
      .pawn_poses
      .iter()
      .enumerate()
      .find(|(_, &pos)| pos == idx)
    {
      Some((idx, _)) => {
        if idx % 2 == 0 {
          TileState::Black
        } else {
          TileState::White
        }
      }
      None => TileState::Empty,
    }
  }

  /// Given a position on the board, returns the tile state of that position,
  /// i.e. the color of the piece on that tile, or `Empty` if no piece is there.
  fn get_tile(&self, idx: PackedIdx) -> TileState {
    if idx == PackedIdx::null() {
      return TileState::Empty;
    }

    let pawn_poses_ptr = self.pawn_poses.as_ptr() as *const u64;

    // Read the internal representation of `idx` as a `u8`, and spread it across
    // all 8 bytes of a `u64` mask.
    let mask = broadcast_u8_to_u64(unsafe { idx.bytes() });

    for i in 0..N / 8 {
      let xor_search = mask ^ unsafe { *pawn_poses_ptr.offset(i as isize) };

      let zero_mask = (xor_search - 0x0101010101010101u64) & !xor_search & 0x8080808080808080u64;
      if zero_mask != 0 {
        let set_bit_idx = zero_mask.trailing_zeros();
        // Find the parity of `set_bit_idx` / 8. Black has the even indices,
        // white has the odd.
        if (set_bit_idx & 0x8) == 0 {
          return TileState::Black;
        } else {
          return TileState::White;
        }
      }
    }

    // only necessary if NPawns not a multiple of eight
    for i in 8 * (N / 8)..N {
      if self.pawn_poses[i] == idx {
        if i % 2 == 0 {
          return TileState::Black;
        } else {
          return TileState::White;
        }
      }
    }

    return TileState::Empty;
  }
}
