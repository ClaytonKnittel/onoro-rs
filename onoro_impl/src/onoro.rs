use std::{
  cmp,
  fmt::{Debug, Display},
};

use abstract_game::{GameIterator, GameMoveIterator};
use algebra::group::Group;
use itertools::interleave;
use onoro::{
  Color, Colored, Onoro, OnoroMoveWrapper, OnoroPawn, PawnColor, TileState,
  groups::{C2, D3, D6, K4},
  hex_pos::{HexPos, HexPosOffset},
};
#[cfg(test)]
use onoro::{error::OnoroResult, make_onoro_error};
#[cfg(test)]
use union_find::UnionFind;

use crate::{
  FilterNullPackedIdx,
  canonicalize::{BoardSymmetryState, board_symm_state},
  r#move::Move,
  onoro_state::OnoroState,
  p1_move_gen::P1MoveGenerator,
  p2_move_gen::P2MoveGenerator,
  packed_hex_pos::PackedHexPos,
  packed_idx::{IdxOffset, PackedIdx},
  util::{broadcast_u8_to_u64, equal_mask_epi8, packed_positions_to_mask, unlikely},
};

/// An Onoro game state with `N / 2` pawns per player.
#[derive(Clone)]
#[repr(align(8))]
pub struct OnoroImpl<const N: usize> {
  /// Array of indexes of pawn positions. Odd entries (even index) are black
  /// pawns, the others are white. Filled from lowest to highest index as the
  /// first phase proceeds.
  pawn_poses: [PackedIdx; N],
  state: OnoroState,
  // Sum of all HexPos's of pieces on the board
  sum_of_mass: PackedHexPos,
}

impl<const N: usize> OnoroImpl<N> {
  /// Constructs an identical Onoro game rotated by `op`.
  fn rotated<G: Group, OpFn: FnMut(&HexPosOffset, &G) -> HexPosOffset>(
    &self,
    op: G,
    mut op_fn: OpFn,
  ) -> Self {
    let mut game = unsafe { Self::new() };

    let mut black_pawns = Vec::new();
    let mut white_pawns = Vec::new();
    let symm_state = board_symm_state(self);
    let origin = self.origin(&symm_state);
    let center = HexPos::new(N as u32 / 2, N as u32 / 2);
    for pawn in self.pawns() {
      let pos = HexPos::from(pawn.pos) - origin;
      let pos = op_fn(&pos, &op);

      match pawn.color {
        PawnColor::Black => {
          black_pawns.push(pos + center);
        }
        PawnColor::White => {
          white_pawns.push(pos + center);
        }
      }
    }

    unsafe {
      game.make_move_unchecked(Move::Phase1Move {
        to: black_pawns[0].into(),
      });
    }
    for pos in interleave(white_pawns, black_pawns.into_iter().skip(1)) {
      game.make_move(Move::Phase1Move { to: pos.into() })
    }

    if !self.in_phase1() && !self.onoro_state().black_turn() {
      game.mut_onoro_state().swap_player_turn();
    }

    game
  }

  pub fn rotated_d6_c(&self, op: D6) -> Self {
    self.rotated(op, HexPosOffset::apply_d6_c)
  }

  pub fn rotated_d3_v(&self, op: D3) -> Self {
    self.rotated(op, HexPosOffset::apply_d3_v)
  }

  pub fn rotated_k4_e(&self, op: K4) -> Self {
    self.rotated(op, HexPosOffset::apply_k4_e)
  }

  pub fn rotated_c2_cv(&self, op: C2) -> Self {
    self.rotated(op, HexPosOffset::apply_c2_cv)
  }

  pub fn rotated_c2_ce(&self, op: C2) -> Self {
    self.rotated(op, HexPosOffset::apply_c2_ce)
  }

  pub fn rotated_c2_ev(&self, op: C2) -> Self {
    self.rotated(op, HexPosOffset::apply_c2_ev)
  }

  pub fn print_with_move(&self, m: Move) -> String {
    let mut g = self.clone();
    g.make_move(m);

    let pawn_idx = match m {
      Move::Phase1Move { to: _ } => self.pawns_in_play(),
      Move::Phase2Move { to: _, from_idx } => from_idx,
    };

    let mut res = String::new();

    let ((min_x, min_y), (max_x, max_y)) = g.pawns().fold(
      ((N, N), (0, 0)),
      |((min_x, min_y), (max_x, max_y)), pawn| {
        (
          (
            min_x.min(pawn.pos.x() as usize),
            min_y.min(pawn.pos.y() as usize),
          ),
          (
            max_x.max(pawn.pos.x() as usize),
            max_y.max(pawn.pos.y() as usize),
          ),
        )
      },
    );

    let min_x = min_x.saturating_sub(1);
    let min_y = min_y.saturating_sub(1);
    let max_x = (max_x + 1).min(N - 1);
    let max_y = (max_y + 1).min(N - 1);

    for y in (min_y..=max_y).rev() {
      res = format!("{res}{: <width$}", "", width = max_y - y);
      for x in min_x..=max_x {
        let pos = PackedIdx::new(x as u32, y as u32);
        let former_pawn_idx = self.get_pawn_idx_slow(pos);
        let new_pawn_idx = g.get_pawn_idx_slow(pos);

        res = format!(
          "{res}{}",
          match g.get_tile(pos) {
            TileState::Black =>
              if new_pawn_idx == Some(pawn_idx) {
                Colored::new("B", Color::Magenta)
              } else {
                "B".into()
              },
            TileState::White =>
              if new_pawn_idx == Some(pawn_idx) {
                Colored::new("W", Color::Magenta)
              } else {
                "W".into()
              },
            TileState::Empty =>
              if former_pawn_idx == Some(pawn_idx) {
                Colored::new(".", Color::Red)
              } else {
                ".".into()
              },
          }
        );

        if x < Self::board_width() - 1 {
          res = format!("{res} ");
        }
      }

      if y > min_y {
        res = format!("{res}\n");
      }
    }

    res
  }

  /// Converts a `HexPos` to an ordinal, which is a unique mapping from valid
  /// `HexPos`s on the board to the range 0..N*N.
  pub const fn hex_pos_ord(pos: &HexPos) -> usize {
    pos.x() as usize + (pos.y() as usize) * N
  }

  /// The inverse of `self.hex_pos_ord`.
  pub const fn ord_to_hex_pos(ord: usize) -> HexPos {
    HexPos::new((ord % N) as u32, (ord / N) as u32)
  }

  pub fn pawns_gen(&self) -> PawnGenerator<N> {
    PawnGenerator { pawn_idx: 0 }
  }

  pub fn pawns_typed(&self) -> GameIterator<'_, PawnGenerator<N>, Self> {
    self.pawns_gen().to_iter(self)
  }

  pub fn color_pawns_gen(&self, color: PawnColor) -> SingleColorPawnGenerator<N> {
    SingleColorPawnGenerator {
      pawn_idx: match color {
        PawnColor::Black => 0,
        PawnColor::White => 1,
      },
    }
  }

  pub fn color_pawns_typed(
    &self,
    color: PawnColor,
  ) -> GameIterator<'_, SingleColorPawnGenerator<N>, Self> {
    self.color_pawns_gen(color).to_iter(self)
  }

  pub fn color_pawns(&self, color: PawnColor) -> impl Iterator<Item = Pawn> + '_ {
    self.color_pawns_typed(color)
  }

  fn onoro_state(&self) -> &OnoroState {
    &self.state
  }

  fn mut_onoro_state(&mut self) -> &mut OnoroState {
    &mut self.state
  }

  /// The color of the current player as a `PawnColor`.
  pub fn player_color(&self) -> PawnColor {
    if self.onoro_state().black_turn() {
      PawnColor::Black
    } else {
      PawnColor::White
    }
  }

  pub(crate) fn pawn_poses(&self) -> &[PackedIdx; N] {
    &self.pawn_poses
  }

  pub fn sum_of_mass(&self) -> PackedHexPos {
    self.sum_of_mass
  }

  /// Returns the origin tile, which all group operations operate with respect
  /// to. This is orientation-invariant, meaning for any symmetry of this board
  /// state, the same origin tile will be chosen.
  pub fn origin(&self, symm_state: &BoardSymmetryState) -> HexPos {
    let x = self.sum_of_mass.x() as u32;
    let y = self.sum_of_mass.y() as u32;
    let truncated_com = HexPos::new(x / self.pawns_in_play(), y / self.pawns_in_play());
    truncated_com + symm_state.center_offset
  }

  pub const fn symm_state_table_width() -> usize {
    N
  }

  /// Returns the size of the symm state table, in terms of number of elements.
  pub const fn symm_state_table_size() -> usize {
    Self::symm_state_table_width() * Self::symm_state_table_width()
  }

  pub fn each_move_gen(&self) -> MoveGenerator<N> {
    if self.in_phase1() {
      MoveGenerator::P1Moves(self.p1_move_gen())
    } else {
      MoveGenerator::P2Moves(self.p2_move_gen())
    }
  }

  fn p1_move_gen(&self) -> P1MoveGenerator<N> {
    debug_assert!(self.in_phase1());
    P1MoveGenerator::new(self)
  }

  fn p2_move_gen(&self) -> P2MoveGenerator<N> {
    debug_assert!(!self.in_phase1());
    P2MoveGenerator::new(self)
  }

  /// Adds a new pawn to the game board at index `i`, without checking what was
  /// there before or verifying that `i` was the correct place to put the pawn.
  /// This will mutate the game state to accomodate the change.
  ///
  ///  Important: this will not update `self.onoro_state().turn()` or
  /// `self.onoro_state().black_turn()`, the caller is responsible for doing so.
  fn place_pawn(&mut self, i: usize, pos: PackedIdx) {
    unsafe {
      *self.pawn_poses.get_unchecked_mut(i) = pos;
    }

    self.sum_of_mass = (HexPos::from(self.sum_of_mass) + pos.into()).into();
    self.adjust_to_new_pawn_and_check_win(pos);
  }

  /// Moves the pawn at index `i` to pos `pos`, mutating the game state to
  /// accomodate the change.
  ///
  ///  Important: this will not update `self.onoro_state().turn()` or
  /// `self.onoro_state().black_turn()`, the caller is responsible for doing so.
  fn move_pawn(&mut self, i: usize, pos: PackedIdx) {
    let mut com_offset: HexPosOffset = pos.into();

    let prev_idx = unsafe { *self.pawn_poses.get_unchecked(i) };
    debug_assert_ne!(prev_idx, PackedIdx::null());
    com_offset -= prev_idx.into();

    unsafe {
      *self.pawn_poses.get_unchecked_mut(i) = pos;
    }

    self.sum_of_mass = (HexPos::from(self.sum_of_mass) + com_offset).into();
    self.adjust_to_new_pawn_and_check_win(pos);
  }

  /// This is very rare, and only called when a pawn is placed on the outer
  /// perimeter of the bounding parallelogram.
  #[inline(never)]
  fn shift_pawns(&mut self, shift: HexPosOffset) {
    let idx_offset = IdxOffset::from(shift);
    self.pawn_poses.iter_mut().filter_null().for_each(|pos| {
      *pos += idx_offset;
    });
    self.sum_of_mass =
      (HexPos::from(self.sum_of_mass) + shift * (self.pawns_in_play() as i32)).into();
  }

  /// Adjust the game state to accomodate a new pawn at position `pos`. This may
  /// shift all pawns on the board. This will also check if the new pawn has
  /// caused the current player to win, and set onoro_state().finished if they
  /// have.
  fn adjust_to_new_pawn_and_check_win(&mut self, pos: PackedIdx) {
    // The amount to shift the whole board by. This will keep pawns off the
    // outer perimeter.
    let shift = Self::calc_move_shift(pos);
    if shift != HexPosOffset::origin() {
      self.shift_pawns(shift);
    }

    let finished = self.check_win(HexPos::from(pos) + shift);
    self.mut_onoro_state().set_finished(finished);
  }

  /// Given the position of a newly placed/moved pawn, returns the offset to
  /// apply to all positions on the board.
  fn calc_move_shift(m: PackedIdx) -> HexPosOffset {
    let mut offset = HexPosOffset::new(0, 0);

    if m.y() == 0 {
      offset = HexPosOffset::new(0, 1);
    } else if m.y() == Self::board_width() as u32 - 1 {
      offset = HexPosOffset::new(0, -1);
    }
    if m.x() == 0 {
      offset += HexPosOffset::new(1, 0);
    } else if m.x() == Self::board_width() as u32 - 1 {
      offset += HexPosOffset::new(-1, 0);
    }

    offset
  }

  fn check_win_fast(pawn_poses: &[PackedIdx; N], last_move: HexPos, black_turn: bool) -> bool {
    debug_assert_eq!(N, 16);

    /// Masks off the pawns in even-indexed bytes.
    const SINGLE_COLOR_MASK: u64 = 0x00ff00ff_00ff00ff;

    /// Selects the x-coordinates of every PackedIdx position.
    const SELECT_X_MASK: u64 = 0x0f0f_0f0f_0f0f_0f0f;

    // For big-endian architectures, we will read the pawn positions in the
    // reverse order from little endian. We can compensate for this by
    // "swapping the colors" of the pieces.
    #[cfg(target_endian = "big")]
    let black_turn = !black_turn;

    // Given one half of the packed positions, returns the positions of the
    // pawns for the last player to move in the even-indexed bytes of a u64.
    let extract_last_player_pawns = |array: &[PackedIdx]| -> u64 {
      let positions = unsafe { *(array.as_ptr() as *const u64) };
      let positions = positions >> (black_turn as u32 * 8);
      positions & SINGLE_COLOR_MASK
    };

    // Extract the positions of the pawns of the last player to move, then
    // combine them into a single u64.
    let low_positions = extract_last_player_pawns(&pawn_poses[0..8]);
    let hi_positions = extract_last_player_pawns(&pawn_poses[8..16]);
    let all_pawns = low_positions | (hi_positions << 8);

    // Extract the x and y coordinates of the pawns.
    let pawns_x = all_pawns & SELECT_X_MASK;
    let pawns_y = (all_pawns >> 4) & SELECT_X_MASK;
    // Extract the difference between the x and y coordinates of the pawns,
    // `+ 0xff` to prevent underflow.
    let pawns_delta = pawns_x + SELECT_X_MASK - pawns_y;

    // Create byte masks for the positions which equal `last_move` in either
    // the x-coordinate, y-coordinate, or (x - y)-coordinate.
    let x_equal_mask = equal_mask_epi8(pawns_x, last_move.x() as u8);
    let y_equal_mask = equal_mask_epi8(pawns_y, last_move.y() as u8);
    let delta_equal_mask =
      equal_mask_epi8(pawns_delta, (last_move.x() + 0xf - last_move.y()) as u8);

    // Mask off any positions which were not equal to `last_move` in the other
    // dimension. Note that for (x - y), we can use either x or y as indices
    // for the positions, since both the x- and y-coordinates are sequential
    // along these diagonal lines.
    let x_equal_y_coords = x_equal_mask & pawns_y;
    let y_equal_x_coords = y_equal_mask & pawns_x;
    let xy_equal_x_coords = delta_equal_mask & pawns_x;

    // We want to determine if any of the above three masks have 4
    // sequential-valued bytes (in any order). To do this, we can map each byte
    // vector to a bitmask, where a bit will be set in the mask if the index of
    // the bit appeared in the byte vector.
    let y_in_a_row = packed_positions_to_mask(x_equal_y_coords);
    let x_in_a_row = packed_positions_to_mask(y_equal_x_coords);
    let xy_in_a_row = packed_positions_to_mask(xy_equal_x_coords);

    // These masks will only set the first 15 bits of the result, since each
    // coordinate is in the range 1..15.
    debug_assert!(x_in_a_row < 0x8000);
    debug_assert!(y_in_a_row < 0x8000);
    debug_assert!(xy_in_a_row < 0x8000);

    // Now that we have bitmasks, each of which are < 0x8000, and we would like
    // to check if any 4 bits are in a row in any of them, we can merge all 3
    // into a single u64 and check for 4 bits in a row in that.
    //
    // We have to be careful that there is at least one guaranteed zero bit
    // between each of the 3 masks in the result, so they don't interfere with
    // each other.
    let all_in_a_row = x_in_a_row | (y_in_a_row << 17) | (xy_in_a_row << 34);

    // Check if any 4 bits in a row are set.
    let all_in_a_row = all_in_a_row & (all_in_a_row >> 1);
    let all_in_a_row = all_in_a_row & (all_in_a_row >> 2);
    all_in_a_row != 0
  }

  fn check_win_slow(&self, last_move: HexPos) -> bool {
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
      let pos: HexPos = unsafe { *self.pawn_poses.get_unchecked(i) }.into();
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

  pub(crate) fn check_win(&self, last_move: HexPos) -> bool {
    if N != 16 {
      return self.check_win_slow(last_move);
    }

    Self::check_win_fast(&self.pawn_poses, last_move, self.onoro_state().black_turn())
  }

  /// Returns a mask with a single bit set in the index corresponding to the
  /// pawn at tile `idx`.
  #[target_feature(enable = "ssse3")]
  unsafe fn pawn_search_mask(pawn_poses: &[PackedIdx; N], idx: PackedIdx) -> u32 {
    use std::arch::x86_64::*;

    let pawns = unsafe { _mm_loadu_si128(pawn_poses.as_ptr() as *const _) };

    // Construct a mask to search for `idx` in the positions lists.
    let idx_search = _mm_set1_epi8(unsafe { idx.bytes() } as i8);

    // Search for `idx` in the positions list. This will either return 0, or
    // a mask with a single byte set to 0xff.
    let masked_pawns = _mm_cmpeq_epi8(pawns, idx_search);

    // Compress the mask to the first 16 bits of a u32.
    _mm_movemask_epi8(masked_pawns) as u32
  }

  #[target_feature(enable = "ssse3")]
  unsafe fn get_tile_fast(pawn_poses: &[PackedIdx; N], idx: PackedIdx) -> TileState {
    debug_assert_eq!(N, 16);
    if unlikely(idx == PackedIdx::null()) {
      return TileState::Empty;
    }

    let mask = unsafe { Self::pawn_search_mask(pawn_poses, idx) };

    // If an even-indexed bit it set, the tile is black. Otherwise, if any
    // other bit is set, the tile is white, else the tile is empty.
    if (mask & 0x55_55) != 0 {
      TileState::Black
    } else if mask != 0 {
      TileState::White
    } else {
      TileState::Empty
    }
  }

  #[target_feature(enable = "ssse3")]
  unsafe fn get_pawn_idx_fast(pawn_poses: &[PackedIdx; N], idx: PackedIdx) -> u32 {
    debug_assert_eq!(N, 16);
    let mask = unsafe { Self::pawn_search_mask(pawn_poses, idx) };
    debug_assert_ne!(mask, 0);
    mask.trailing_zeros()
  }

  /// Given a position on the board, returns the index of the pawn with that
  /// position, or `None` if no such pawn exists.
  fn get_pawn_idx_slow(&self, idx: PackedIdx) -> Option<u32> {
    if idx == PackedIdx::null() {
      return None;
    }

    let pawn_poses_ptr = self.pawn_poses.as_ptr() as *const u64;

    // Read the internal representation of `idx` as a `u8`, and spread it across
    // all 8 bytes of a `u64` mask.
    let mask = broadcast_u8_to_u64(unsafe { idx.bytes() });

    for i in 0..N / 8 {
      let xor_search = mask ^ unsafe { *pawn_poses_ptr.add(i) };

      let zero_mask =
        (xor_search.wrapping_sub(0x0101010101010101u64)) & !xor_search & 0x8080808080808080u64;
      if zero_mask != 0 {
        let set_bit_idx = zero_mask.trailing_zeros();
        return Some(8 * i as u32 + (set_bit_idx / 8));
      }
    }

    // Only necessary if N not a multiple of eight.
    for i in 8 * (N / 8)..N {
      if unsafe { *self.pawn_poses.get_unchecked(i) } == idx {
        return Some(i as u32);
      }
    }

    None
  }

  fn get_tile_slow(&self, idx: PackedIdx) -> TileState {
    match self.get_pawn_idx_slow(idx) {
      Some(i) => {
        if i % 2 == 0 {
          TileState::Black
        } else {
          TileState::White
        }
      }
      None => TileState::Empty,
    }
  }

  pub fn get_pawn_idx(&self, idx: PackedIdx) -> u32 {
    debug_assert_ne!(idx, PackedIdx::null());

    #[cfg(target_feature = "ssse3")]
    if N == 16 {
      return unsafe { Self::get_pawn_idx_fast(&self.pawn_poses, idx) };
    }
    self.get_pawn_idx_slow(idx).unwrap()
  }

  /// Bounds checks a hex pos before turning it into a PackedIdx for lookup.
  #[cfg(test)]
  fn get_tile_hex_pos(&self, idx: HexPos) -> TileState {
    if idx.x() >= N as u32 || idx.y() >= N as u32 {
      TileState::Empty
    } else {
      self.get_tile(idx.into())
    }
  }
}

impl<const N: usize> Onoro for OnoroImpl<N> {
  type Index = PackedIdx;
  type Move = Move;
  type Pawn = Pawn;

  unsafe fn new() -> Self {
    Self {
      pawn_poses: [PackedIdx::null(); N],
      state: OnoroState::new(),
      sum_of_mass: HexPos::zero().into(),
    }
  }

  fn pawns_per_player() -> usize {
    N / 2
  }

  fn turn(&self) -> PawnColor {
    if self.onoro_state().black_turn() {
      PawnColor::Black
    } else {
      PawnColor::White
    }
  }

  fn pawns_in_play(&self) -> u32 {
    self.onoro_state().turn() + 1
  }

  fn finished(&self) -> Option<PawnColor> {
    self.onoro_state().finished().then(|| {
      if self.onoro_state().black_turn() {
        PawnColor::White
      } else {
        PawnColor::Black
      }
    })
  }

  fn get_tile(&self, idx: PackedIdx) -> TileState {
    #[cfg(target_feature = "ssse3")]
    if N == 16 {
      return unsafe { Self::get_tile_fast(&self.pawn_poses, idx) };
    }
    self.get_tile_slow(idx)
  }

  fn pawns(&self) -> impl Iterator<Item = Pawn> + '_ {
    self.pawns_typed()
  }

  fn in_phase1(&self) -> bool {
    self.onoro_state().turn() < N as u32 - 1
  }

  fn each_move(&self) -> impl Iterator<Item = Move> {
    self.each_move_gen().to_iter(self)
  }

  fn make_move(&mut self, m: Move) {
    debug_assert!(self.finished().is_none());

    match m {
      Move::Phase1Move { to: _ } => {
        debug_assert!(self.in_phase1());
      }
      Move::Phase2Move { to: _, from_idx: _ } => {
        debug_assert!(!self.in_phase1());
      }
    }
    unsafe { self.make_move_unchecked(m) }
  }

  unsafe fn make_move_unchecked(&mut self, m: Move) {
    match m {
      Move::Phase1Move { to } => {
        // Increment the turn first, so self.onoro_state().turn() is 0 for turn
        // 1.
        self.mut_onoro_state().inc_turn();
        let pawn_idx = self.onoro_state().turn() as usize;
        self.place_pawn(pawn_idx, to);
      }
      Move::Phase2Move { to, from_idx } => {
        self.mut_onoro_state().swap_player_turn();
        self.move_pawn(from_idx as usize, to);
      }
    }
  }

  fn to_move_wrapper(&self, m: &Move) -> OnoroMoveWrapper<PackedIdx> {
    match *m {
      Move::Phase1Move { to } => OnoroMoveWrapper::Phase1 { to },
      Move::Phase2Move { to, from_idx } => OnoroMoveWrapper::Phase2 {
        from: *self.pawn_poses.get(from_idx as usize).unwrap(),
        to,
      },
    }
  }

  #[cfg(test)]
  fn validate(&self) -> OnoroResult {
    let mut n_b_pawns = 0u32;
    let mut n_w_pawns = 0u32;
    let mut sum_of_mass = HexPos::zero();

    let mut uf = UnionFind::new(N * N);

    for pawn in self.pawns() {
      sum_of_mass += pawn.pos.into();

      if pawn.pos.x() == 0
        || pawn.pos.x() >= N as u32 - 1
        || pawn.pos.y() == 0
        || pawn.pos.y() >= N as u32 - 1
      {
        return Err(make_onoro_error!(
          "Pawn with coordinates on border of board: {}",
          pawn
        ));
      }

      match pawn.color {
        PawnColor::Black => {
          n_b_pawns += 1;
        }
        PawnColor::White => {
          n_w_pawns += 1;
        }
      };

      match (self.get_tile(pawn.pos), &pawn.color) {
        (TileState::Black, PawnColor::Black) => {}
        (TileState::White, PawnColor::White) => {}
        (TileState::Empty, _) => {
          return Err(make_onoro_error!(
            "Unexpected empty tile found with `get_tile` at `pawn.pos` ({}) from pawn returned by iterator.",
            pawn
          ));
        }
        (get_tile_color, _) => {
          return Err(make_onoro_error!(
            "Mismatched tile colors for iterator pawn ({}), `get_tile` returns color {:?} at this position",
            pawn,
            get_tile_color
          ));
        }
      }

      HexPos::from(pawn.pos)
        .each_top_left_neighbor()
        .for_each(|neighbor_pos| {
          if self.get_tile_hex_pos(neighbor_pos) != TileState::Empty {
            uf.union(
              Self::hex_pos_ord(&HexPos::from(pawn.pos)),
              Self::hex_pos_ord(&neighbor_pos),
            );
          }
        });
    }

    if n_b_pawns + n_w_pawns != self.pawns_in_play() {
      return Err(make_onoro_error!(
        "Expected {} pawns in play, but found {}",
        self.pawns_in_play(),
        n_b_pawns + n_w_pawns
      ));
    }

    if self.in_phase1() && self.onoro_state().black_turn() as u32 != (self.onoro_state().turn() & 1)
    {
      return Err(make_onoro_error!(
        "Expected black turn to be {}, but was {}",
        self.onoro_state().turn() & 1,
        self.onoro_state().black_turn()
      ));
    }

    if n_b_pawns
      != n_w_pawns
        + if !self.in_phase1() || self.onoro_state().black_turn() {
          0
        } else {
          1
        }
    {
      return Err(make_onoro_error!(
        "Expected {} black pawns and {} white pawns, but found {} and {}",
        self.pawns_in_play().div_ceil(2),
        self.pawns_in_play() / 2,
        n_b_pawns,
        n_w_pawns
      ));
    }

    if sum_of_mass != self.sum_of_mass.into() {
      return Err(make_onoro_error!(
        "Sum of mass not correct: expect {}, but have {}",
        sum_of_mass,
        self.sum_of_mass
      ));
    }

    let empty_tiles = Self::board_size() - self.pawns_in_play() as usize;
    let pawn_groups = uf.unique_sets() - empty_tiles;

    if pawn_groups != 1 {
      return Err(make_onoro_error!(
        "Expected 1 contiguous pawn group, but found {}",
        pawn_groups
      ));
    }

    Ok(())
  }
}

impl<const N: usize> Debug for OnoroImpl<N> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{self}")
  }
}

impl<const N: usize> Display for OnoroImpl<N> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    self.display(f)
  }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Pawn {
  pub pos: PackedIdx,
  pub color: PawnColor,
  board_idx: u8,
}

impl OnoroPawn for Pawn {
  type Index = PackedIdx;

  fn pos(&self) -> PackedIdx {
    self.pos
  }

  fn color(&self) -> PawnColor {
    self.color
  }
}

impl Display for Pawn {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "{} pawn at {}",
      match self.color {
        PawnColor::Black => "black",
        PawnColor::White => "white",
      },
      HexPos::from(self.pos)
    )
  }
}

pub struct PawnGeneratorImpl<
  // If true, only iterates over pawns of one color, otherwise iterating over all pawns.
  const ONE_COLOR: bool,
  const N: usize,
> {
  // TODO: Should this be a u8?
  pawn_idx: usize,
}

impl<const ONE_COLOR: bool, const N: usize> GameMoveIterator for PawnGeneratorImpl<ONE_COLOR, N> {
  type Item = Pawn;
  type Game = OnoroImpl<N>;

  fn next(&mut self, onoro: &Self::Game) -> Option<Self::Item> {
    if self.pawn_idx >= onoro.pawns_in_play() as usize {
      return None;
    }

    let pawn = Pawn {
      pos: unsafe { *onoro.pawn_poses.get_unchecked(self.pawn_idx) },
      color: if self.pawn_idx % 2 == 0 {
        PawnColor::Black
      } else {
        PawnColor::White
      },
      board_idx: self.pawn_idx as u8,
    };
    self.pawn_idx += if ONE_COLOR { 2 } else { 1 };

    Some(pawn)
  }
}

pub type PawnGenerator<const N: usize> = PawnGeneratorImpl<false, N>;
pub type SingleColorPawnGenerator<const N: usize> = PawnGeneratorImpl<true, N>;

pub enum MoveGenerator<const N: usize> {
  P1Moves(P1MoveGenerator<N>),
  P2Moves(P2MoveGenerator<N>),
}

impl<const N: usize> GameMoveIterator for MoveGenerator<N> {
  type Item = Move;
  type Game = OnoroImpl<N>;

  fn next(&mut self, onoro: &Self::Game) -> Option<Self::Item> {
    match self {
      Self::P1Moves(p1_iter) => p1_iter.next(onoro),
      Self::P2Moves(p2_iter) => p2_iter.next(onoro),
    }
  }
}

#[cfg(test)]
mod tests {
  use std::ops::{Index, IndexMut};

  use googletest::{expect_false, expect_true, gtest};
  use onoro::{Onoro, TileState, hex_pos::HexPos};

  use crate::{Onoro16, OnoroImpl, onoro_defs::Onoro8, packed_idx::PackedIdx};

  #[repr(align(8))]
  struct PawnPoses([PackedIdx; 16]);
  impl Index<usize> for PawnPoses {
    type Output = PackedIdx;

    fn index(&self, index: usize) -> &Self::Output {
      &self.0[index]
    }
  }
  impl IndexMut<usize> for PawnPoses {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
      &mut self.0[index]
    }
  }

  /// Given a position on the board, returns the tile state of that position,
  /// i.e. the color of the piece on that tile, or `Empty` if no piece is there.
  fn get_tile_test<const N: usize>(onoro: &OnoroImpl<N>, idx: PackedIdx) -> TileState {
    if idx == PackedIdx::null() {
      return TileState::Empty;
    }

    match onoro
      .pawn_poses
      .iter()
      .enumerate()
      .find(|&(_, &pos)| pos == idx)
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

  #[test]
  fn test_get_tile_simple() {
    let onoro = Onoro8::default_start();

    for y in 0..Onoro8::board_width() {
      for x in 0..Onoro8::board_width() {
        let idx = PackedIdx::new(x as u32, y as u32);
        assert_eq!(onoro.get_tile(idx), get_tile_test(&onoro, idx));
      }
    }
  }

  #[test]
  fn test_get_tile_simple_16() {
    let onoro = Onoro16::default_start();

    for y in 0..Onoro16::board_width() {
      for x in 0..Onoro16::board_width() {
        let idx = PackedIdx::new(x as u32, y as u32);
        assert_eq!(onoro.get_tile(idx), get_tile_test(&onoro, idx));
      }
    }
  }

  /// Given a position on the board, returns the index of the pawn on that
  /// tile, or `None` if no piece is there.
  fn get_pawn_idx_test<const N: usize>(onoro: &OnoroImpl<N>, idx: PackedIdx) -> Option<u32> {
    if idx == PackedIdx::null() {
      return None;
    }

    onoro
      .pawn_poses
      .iter()
      .enumerate()
      .find_map(|(i, &pos)| (pos == idx).then_some(i as u32))
  }

  #[test]
  fn test_get_pawn_idx_simple() {
    let onoro = Onoro8::default_start();

    for y in 0..Onoro8::board_width() {
      for x in 0..Onoro8::board_width() {
        let idx = PackedIdx::new(x as u32, y as u32);
        if let Some(pawn_idx) = get_pawn_idx_test(&onoro, idx) {
          assert_eq!(onoro.get_pawn_idx(idx), pawn_idx);
        }
      }
    }
  }

  #[test]
  fn test_get_pawn_idx_simple_16() {
    let onoro = Onoro16::default_start();

    for y in 0..Onoro16::board_width() {
      for x in 0..Onoro16::board_width() {
        let idx = PackedIdx::new(x as u32, y as u32);
        if let Some(pawn_idx) = get_pawn_idx_test(&onoro, idx) {
          assert_eq!(onoro.get_pawn_idx(idx), pawn_idx);
        }
      }
    }
  }

  #[gtest]
  fn test_check_win_simple() {
    let mut pawn_poses = PawnPoses([PackedIdx::null(); 16]);
    pawn_poses[0] = PackedIdx::new(5, 5);
    pawn_poses[2] = PackedIdx::new(6, 5);
    pawn_poses[4] = PackedIdx::new(7, 5);
    pawn_poses[6] = PackedIdx::new(8, 5);

    expect_true!(Onoro16::check_win_fast(
      &pawn_poses.0,
      HexPos::new(6, 5),
      false
    ));
  }

  #[gtest]
  fn test_check_win_hole() {
    let mut pawn_poses = PawnPoses([PackedIdx::null(); 16]);
    pawn_poses[0] = PackedIdx::new(5, 5);
    pawn_poses[2] = PackedIdx::new(6, 5);
    pawn_poses[4] = PackedIdx::new(8, 5);
    pawn_poses[6] = PackedIdx::new(9, 5);

    expect_false!(Onoro16::check_win_fast(
      &pawn_poses.0,
      HexPos::new(6, 5),
      false
    ));
  }

  #[gtest]
  fn test_check_win_wrong_row() {
    let mut pawn_poses = PawnPoses([PackedIdx::null(); 16]);
    pawn_poses[0] = PackedIdx::new(5, 5);
    pawn_poses[2] = PackedIdx::new(6, 5);
    pawn_poses[4] = PackedIdx::new(7, 5);
    pawn_poses[6] = PackedIdx::new(8, 5);

    expect_false!(Onoro16::check_win_fast(
      &pawn_poses.0,
      HexPos::new(6, 6),
      false
    ));
  }

  #[gtest]
  fn test_check_win_spread_out() {
    let mut pawn_poses = PawnPoses([PackedIdx::null(); 16]);
    pawn_poses[0] = PackedIdx::new(5, 5);
    pawn_poses[4] = PackedIdx::new(6, 5);
    pawn_poses[14] = PackedIdx::new(7, 5);
    pawn_poses[8] = PackedIdx::new(8, 5);

    expect_true!(Onoro16::check_win_fast(
      &pawn_poses.0,
      HexPos::new(6, 5),
      false
    ));
  }

  #[gtest]
  fn test_check_win_wrong_color() {
    let mut pawn_poses = PawnPoses([PackedIdx::null(); 16]);
    pawn_poses[0] = PackedIdx::new(5, 5);
    pawn_poses[2] = PackedIdx::new(6, 5);
    pawn_poses[4] = PackedIdx::new(7, 5);
    pawn_poses[6] = PackedIdx::new(8, 5);

    expect_false!(Onoro16::check_win_fast(
      &pawn_poses.0,
      HexPos::new(6, 5),
      true
    ));
  }

  #[gtest]
  fn test_check_win_in_y() {
    let mut pawn_poses = PawnPoses([PackedIdx::null(); 16]);
    pawn_poses[0] = PackedIdx::new(5, 2);
    pawn_poses[2] = PackedIdx::new(5, 3);
    pawn_poses[4] = PackedIdx::new(5, 4);
    pawn_poses[6] = PackedIdx::new(5, 5);

    expect_true!(Onoro16::check_win_fast(
      &pawn_poses.0,
      HexPos::new(5, 5),
      false
    ));
  }

  #[gtest]
  fn test_check_win_in_xy() {
    let mut pawn_poses = PawnPoses([PackedIdx::null(); 16]);
    pawn_poses[0] = PackedIdx::new(4, 2);
    pawn_poses[2] = PackedIdx::new(5, 3);
    pawn_poses[4] = PackedIdx::new(6, 4);
    pawn_poses[6] = PackedIdx::new(7, 5);

    expect_true!(Onoro16::check_win_fast(
      &pawn_poses.0,
      HexPos::new(5, 3),
      false
    ));
  }

  #[gtest]
  fn test_check_win_near_zero() {
    let mut pawn_poses = PawnPoses([PackedIdx::null(); 16]);
    pawn_poses[1] = PackedIdx::new(1, 3);
    pawn_poses[3] = PackedIdx::new(2, 3);
    pawn_poses[5] = PackedIdx::new(3, 3);

    expect_false!(Onoro16::check_win_fast(
      &pawn_poses.0,
      HexPos::new(1, 3),
      true
    ));
  }
}
