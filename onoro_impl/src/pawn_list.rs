#[cfg(target_feature = "sse4.1")]
use std::arch::x86_64::*;

#[cfg(not(target_feature = "sse4.1"))]
use algebra::{group::Trivial, ordinal::Ordinal};
#[cfg(not(target_feature = "sse4.1"))]
use itertools::Itertools;
use onoro::{groups::SymmetryClass, hex_pos::HexPos};
#[cfg(not(target_feature = "sse4.1"))]
use onoro::{
  groups::{C2, D3, D6, K4},
  hex_pos::HexPosOffset,
};

#[cfg(target_feature = "sse4.1")]
use crate::util::MM128Iter;
use crate::{PackedIdx, util::unreachable};

const N: usize = 16;

#[cfg(target_feature = "sse4.1")]
#[derive(Clone, Copy)]
#[repr(align(16))]
struct MM128Contents([i8; 16]);

#[cfg(target_feature = "sse4.1")]
impl MM128Contents {
  #[target_feature(enable = "sse4.1")]
  fn load(&self) -> __m128i {
    unsafe { _mm_load_si128(self.0.as_ptr() as *const _) }
  }

  /// A vector register with all zero lanes.
  const fn zero() -> MM128Contents {
    MM128Contents([0; 16])
  }

  /// A vector register with 1's in the x-coordinate lanes.
  const fn x_ones() -> MM128Contents {
    MM128Contents([1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0])
  }

  /// A vector register with 1's in the x- and y-coordinate lanes.
  const fn xy_ones() -> MM128Contents {
    MM128Contents([1; 16])
  }

  // The following *_shuffle methods produce shuffle control masks for
  // `_mm_shuffle_epi8`.

  /// This simply copies the input operand.
  const fn noop_shuffle() -> MM128Contents {
    MM128Contents([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15])
  }

  /// This forces the result of the shuffle to be zero, since all highest-order
  /// bits of the control mask lanes are 1.
  const fn zero_shuffle() -> MM128Contents {
    MM128Contents([-1; 16])
  }

  /// This swaps adjacent pairs of epi8 lanes, which swaps the x and y
  /// coordinates of each coordinate pair.
  const fn swap_xy_shuffle() -> MM128Contents {
    MM128Contents([1, 0, 3, 2, 5, 4, 7, 6, 9, 8, 11, 10, 13, 12, 15, 14])
  }

  /// This preserves the x-coordinate lanes and sets the y-coordinate lanes to
  /// 0.
  const fn isolate_x_shuffle() -> MM128Contents {
    MM128Contents([0, -1, 2, -1, 4, -1, 6, -1, 8, -1, 10, -1, 12, -1, 14, -1])
  }

  /// This preserves the y-coordinate lanes and sets the x-coordinate lanes to
  /// 0.
  const fn isolate_y_shuffle() -> MM128Contents {
    MM128Contents([-1, 1, -1, 3, -1, 5, -1, 7, -1, 9, -1, 11, -1, 13, -1, 15])
  }

  /// This copies all x-coordinate lanes to their corresponding y-coordinates.
  const fn duplicate_x_shuffle() -> MM128Contents {
    MM128Contents([0, 0, 2, 2, 4, 4, 6, 6, 8, 8, 10, 10, 12, 12, 14, 14])
  }

  /// This copies all y-coordinate lanes to their corresponding x-coordinates.
  const fn duplicate_y_shuffle() -> MM128Contents {
    MM128Contents([1, 1, 3, 3, 5, 5, 7, 7, 9, 9, 11, 11, 13, 13, 15, 15])
  }

  /// This copies all x-coordinate lanes to their corresponding y-coordinates
  /// and zeros out all x-coordinate lanes.
  const fn move_x_to_y_shuffle() -> MM128Contents {
    MM128Contents([-1, 0, -1, 2, -1, 4, -1, 6, -1, 8, -1, 10, -1, 12, -1, 14])
  }

  /// This copies all y-coordinate lanes to their corresponding x-coordinates
  /// and zeros out all y-coordinate lanes.
  const fn move_y_to_x_shuffle() -> MM128Contents {
    MM128Contents([1, -1, 3, -1, 5, -1, 7, -1, 9, -1, 11, -1, 13, -1, 15, -1])
  }
}

#[cfg(target_feature = "sse4.1")]
mod rotate_impl {
  use std::arch::x86_64::*;

  use onoro::groups::SymmetryClass;

  use crate::pawn_list::MM128Contents;

  // All D6 rotation/reflection operations on (x, y) coordinate pairs can be
  // expressed as a sum/difference/assignment of x and/or y for each
  // coordinate.
  //
  // Here are the mappings of transformations for each element of D6:
  // ```text
  // r0 => (x, y)
  // r1 => (x - y, y)
  // r2 => (-y, x - y)
  // r3 => (-x, -y)
  // r4 => (y - x, -x)
  // r5 => (y, y - x)
  // s0 => (x - y, -y)
  // s1 => (x, x - y)
  // s2 => (y, x)
  // s3 => (y - x, y)
  // s4 => (-x, y - x)
  // s5 => (-y, -x)
  // ```
  //
  // We can rewrite each of these expressions as `a - b`, where `a` and `b`
  // are one of `x`, `y`, or 0.
  //
  // If we have a 128-bit vector register with 16 epi8 channels containing 8
  // coordinate pairs, then we can apply a particular group operation to every
  // element of the vector register simultaneously using shuffles. The trick
  // is that we can select the positive and negative components of the
  // difference expression for the operation for each coordinate with two
  // _mm_shuffle_epi8 calls, then compute the difference with _mm_sub_epi8.
  //
  // Since the instructions we will use for every group operation are the
  // same, with the only difference being the shuffle mask, we can construct a
  // lookup table mapping group operation ordinals to shuffle masks for the
  // positive and negative components, then all operations can share the same
  // code path.

  /// The shuffle mask to select the positive component of the difference
  /// operation, indexed by group operation ordinal.
  const D6_POSITIVE_MASKS: [MM128Contents; 12] = [
    MM128Contents::noop_shuffle(),
    MM128Contents::duplicate_x_shuffle(),
    MM128Contents::move_x_to_y_shuffle(),
    MM128Contents::zero_shuffle(),
    MM128Contents::move_y_to_x_shuffle(),
    MM128Contents::duplicate_y_shuffle(),
    MM128Contents::isolate_x_shuffle(),
    MM128Contents::duplicate_x_shuffle(),
    MM128Contents::swap_xy_shuffle(),
    MM128Contents::duplicate_y_shuffle(),
    MM128Contents::isolate_y_shuffle(),
    MM128Contents::zero_shuffle(),
  ];

  /// The shuffle mask to select the negative component of the difference
  /// operation, indexed by group operation ordinal.
  const D6_NEGATIVE_MASKS: [MM128Contents; 12] = [
    MM128Contents::zero_shuffle(),
    MM128Contents::move_y_to_x_shuffle(),
    MM128Contents::duplicate_y_shuffle(),
    MM128Contents::noop_shuffle(),
    MM128Contents::duplicate_x_shuffle(),
    MM128Contents::move_x_to_y_shuffle(),
    MM128Contents::duplicate_y_shuffle(),
    MM128Contents::isolate_y_shuffle(),
    MM128Contents::zero_shuffle(),
    MM128Contents::isolate_x_shuffle(),
    MM128Contents::duplicate_x_shuffle(),
    MM128Contents::swap_xy_shuffle(),
  ];

  /// Applies the `D6` symmetry operation to every pair of epi8 lanes, treated as
  /// coordinate pairs centered at (0, 0).
  #[target_feature(enable = "sse4.1")]
  #[inline]
  pub fn apply_d6_c_sse(pawns: __m128i, op_ord: usize) -> __m128i {
    let positive_mask = unsafe { D6_POSITIVE_MASKS.get_unchecked(op_ord) }.load();
    let negative_mask = unsafe { D6_NEGATIVE_MASKS.get_unchecked(op_ord) }.load();
    let positive = _mm_shuffle_epi8(pawns, positive_mask);
    let negative = _mm_shuffle_epi8(pawns, negative_mask);
    _mm_sub_epi8(positive, negative)
  }

  // All rotation/reflection operations for the 7 possible symmetry classes of
  // Onoro boards (see `SymmetryClass`) on (x, y) coordinate pairs can be
  // expressed as a sum/difference/assignment of x, y, and/or the constant 1
  // for each coordinate.
  //
  // Here are the mappings of transformations for each element of each symmetry
  // class:
  // C (D6): same as above
  // V (D3):
  //   r0 => (x, y)
  //   r1 => (1 - y, x - y)
  //   r2 => (1 + y - x, 1 - x)
  //   s0 => (x, x - y)
  //   s1 => (1 + y - x, y)
  //   s2 => (1 - y, 1 - x)
  // E (K4):
  //   (r0, r0) => (x, y)
  //   (r1, r0) => (x - y, -y)
  //   (r0, r1) => (1 + y - x, y)
  //   (r1, r1) => (1 - x, -y)
  // CV (C2):
  //   r0 => (x, y)
  //   r1 => (x, x - y)
  // CE (C2):
  //   r0 => (x, y)
  //   r1 => (x - y, -y)
  // EV (C2):
  //   r0 => (x, y)
  //   r1 => (1 + y - x, y)
  // Trivial:
  //   r0 => (x, y)
  //
  // As you can see, all expressions have the form
  // [0 or 1] + [x or y or 0] - [x or y or 0].
  //
  // We will use a trick similar to the one above, where we will shuffle the
  // input 128-bit vector register according to a shuffle mask which is
  // dependent on the operation being applied. However we will need to add a
  // constant value to the result, which is also dependent on the operation
  // being applied, since some expressions contain a 1.
  //
  // We will again use a lookup table mapping group operation ordinals to a
  // constant value and shuffle masks for the positive and negative components,
  // then all operations can share the same code path.

  /// A map from (SymmetryClass, rotation/reflection ordinal) to a constant
  /// value.
  const ONES_MASKS: [MM128Contents; 29] = [
    // Trivial
    MM128Contents::zero(),
    // EV
    MM128Contents::zero(),
    MM128Contents::x_ones(),
    // CE
    MM128Contents::zero(),
    MM128Contents::zero(),
    // CV
    MM128Contents::zero(),
    MM128Contents::zero(),
    // E
    MM128Contents::zero(),
    MM128Contents::zero(),
    MM128Contents::x_ones(),
    MM128Contents::x_ones(),
    // V
    MM128Contents::zero(),
    MM128Contents::x_ones(),
    MM128Contents::xy_ones(),
    MM128Contents::zero(),
    MM128Contents::x_ones(),
    MM128Contents::xy_ones(),
    // C
    MM128Contents::zero(),
    MM128Contents::zero(),
    MM128Contents::zero(),
    MM128Contents::zero(),
    MM128Contents::zero(),
    MM128Contents::zero(),
    MM128Contents::zero(),
    MM128Contents::zero(),
    MM128Contents::zero(),
    MM128Contents::zero(),
    MM128Contents::zero(),
    MM128Contents::zero(),
  ];

  /// The shuffle mask to select the positive component of the expression,
  /// indexed by SymmetryClass and group operation ordinal.
  const POSITIVE_MASKS: [MM128Contents; 29] = [
    // Trivial
    MM128Contents::noop_shuffle(),
    // EV
    MM128Contents::noop_shuffle(),
    MM128Contents::duplicate_y_shuffle(),
    // CE
    MM128Contents::noop_shuffle(),
    MM128Contents::isolate_x_shuffle(),
    // CV
    MM128Contents::noop_shuffle(),
    MM128Contents::duplicate_x_shuffle(),
    // E
    MM128Contents::noop_shuffle(),
    MM128Contents::isolate_x_shuffle(),
    MM128Contents::duplicate_y_shuffle(),
    MM128Contents::zero_shuffle(),
    // V
    MM128Contents::noop_shuffle(),
    MM128Contents::move_x_to_y_shuffle(),
    MM128Contents::move_y_to_x_shuffle(),
    MM128Contents::duplicate_x_shuffle(),
    MM128Contents::duplicate_y_shuffle(),
    MM128Contents::zero_shuffle(),
    // C
    D6_POSITIVE_MASKS[0],
    D6_POSITIVE_MASKS[1],
    D6_POSITIVE_MASKS[2],
    D6_POSITIVE_MASKS[3],
    D6_POSITIVE_MASKS[4],
    D6_POSITIVE_MASKS[5],
    D6_POSITIVE_MASKS[6],
    D6_POSITIVE_MASKS[7],
    D6_POSITIVE_MASKS[8],
    D6_POSITIVE_MASKS[9],
    D6_POSITIVE_MASKS[10],
    D6_POSITIVE_MASKS[11],
  ];

  /// The shuffle mask to select the negative component of the expression,
  /// indexed by SymmetryClass and group operation ordinal.
  const NEGATIVE_MASKS: [MM128Contents; 29] = [
    // Trivial
    MM128Contents::zero_shuffle(),
    // EV
    MM128Contents::zero_shuffle(),
    MM128Contents::isolate_x_shuffle(),
    // CE
    MM128Contents::zero_shuffle(),
    MM128Contents::duplicate_y_shuffle(),
    // CV
    MM128Contents::zero_shuffle(),
    MM128Contents::isolate_y_shuffle(),
    // E
    MM128Contents::zero_shuffle(),
    MM128Contents::duplicate_y_shuffle(),
    MM128Contents::isolate_x_shuffle(),
    MM128Contents::noop_shuffle(),
    // V
    MM128Contents::zero_shuffle(),
    MM128Contents::duplicate_y_shuffle(),
    MM128Contents::duplicate_x_shuffle(),
    MM128Contents::isolate_y_shuffle(),
    MM128Contents::isolate_x_shuffle(),
    MM128Contents::swap_xy_shuffle(),
    // C
    D6_NEGATIVE_MASKS[0],
    D6_NEGATIVE_MASKS[1],
    D6_NEGATIVE_MASKS[2],
    D6_NEGATIVE_MASKS[3],
    D6_NEGATIVE_MASKS[4],
    D6_NEGATIVE_MASKS[5],
    D6_NEGATIVE_MASKS[6],
    D6_NEGATIVE_MASKS[7],
    D6_NEGATIVE_MASKS[8],
    D6_NEGATIVE_MASKS[9],
    D6_NEGATIVE_MASKS[10],
    D6_NEGATIVE_MASKS[11],
  ];

  /// Given a SymmetryClass, returns the offset in the above tables we should
  /// index from for group operations in that symmetry class.
  const fn symmetry_class_offset(symm_class: SymmetryClass) -> usize {
    match symm_class {
      SymmetryClass::Trivial => 0,
      SymmetryClass::EV => 1,
      SymmetryClass::CE => 3,
      SymmetryClass::CV => 5,
      SymmetryClass::E => 7,
      SymmetryClass::V => 11,
      SymmetryClass::C => 17,
    }
  }

  /// Applies a particular rotation/reflection operation to every pair of epi8
  /// lanes, treated as coordinate pairs centered at (0, 0), per the provided
  /// symmetry class and operation ordinal (whose interpretation is determined
  /// by the symmetry class).
  #[target_feature(enable = "sse4.1")]
  #[inline]
  pub fn apply_sse(pawns: __m128i, symm_class: SymmetryClass, op_ord: usize) -> __m128i {
    let idx = symmetry_class_offset(symm_class) + op_ord;
    let ones = unsafe { ONES_MASKS.get_unchecked(idx) }.load();
    let positive_mask = unsafe { POSITIVE_MASKS.get_unchecked(idx) }.load();
    let negative_mask = unsafe { NEGATIVE_MASKS.get_unchecked(idx) }.load();
    let positive = _mm_shuffle_epi8(pawns, positive_mask);
    let negative = _mm_shuffle_epi8(pawns, negative_mask);

    _mm_sub_epi8(_mm_add_epi8(positive, ones), negative)
  }
}

/// Stores a list of 8 pawns in an __m128i register, with each adjacent pair of
/// epi8 lanes containing the origin-relative x- and y- coordinates of a pawn.
#[cfg(target_feature = "sse4.1")]
#[derive(Clone, Copy)]
pub struct PawnList8 {
  /// Stores 8 pawns, with x- and y- coordinates in back-to-back epi8 channels.
  pawns: __m128i,
  /// A mask of the pawns in `pawns` which were originally `PackedIdx::null()`.
  null_mask: __m128i,
}

#[cfg(target_feature = "sse4.1")]
impl PawnList8 {
  /// Extracts the 8 black pawns (at even incides) from `pawn_poses` into a
  /// `PawnList8`, ignoring any `null` poses.
  #[target_feature(enable = "sse4.1")]
  fn extract_black_pawns_sse(pawn_poses: &[PackedIdx; N], origin: HexPos) -> Self {
    let pawns = unsafe { _mm_loadu_si128(pawn_poses.as_ptr() as *const _) };

    // Mask just the x coordinates from each `PackedIdx`. These are
    // conveniently already in the correct lanes, so no need to shift.
    let black_x_coords_mask = _mm_set1_epi16(0x00_0f);
    let x_coords = _mm_and_si128(pawns, black_x_coords_mask);

    // Mask just the y coordinates from each `PackedIdx`, and shift them left
    // by 4 bits into the epi8 lane to the left of their corresponding x
    // coordinate.
    let black_y_coords_mask = _mm_set1_epi16(0x00_f0);
    let y_coords = _mm_and_si128(pawns, black_y_coords_mask);
    let y_coords = _mm_slli_epi16::<4>(y_coords);

    // Combine the x and y coordinate vectors.
    let pawns = _mm_or_si128(x_coords, y_coords);
    // Record which `PackedIdx`s were originally `null`.
    let null_mask = Self::null_mask(pawns);

    let centered_pawns = Self::centered_by(pawns, origin);

    Self {
      pawns: centered_pawns,
      null_mask,
    }
  }

  /// Extracts the 8 white pawns (at odd incides) from `pawn_poses` into a
  /// `PawnList8`, ignoring any `null` poses.
  #[target_feature(enable = "sse4.1")]
  fn extract_white_pawns_sse(pawn_poses: &[PackedIdx; N], origin: HexPos) -> Self {
    let pawns = unsafe { _mm_loadu_si128(pawn_poses.as_ptr() as *const _) };

    // Mask just the x coordinates from each `PackedIdx`, and shift them right
    // by one epi8 lane.
    let white_x_coords_mask = _mm_set1_epi16(0x0f_00);
    let x_coords = _mm_and_si128(pawns, white_x_coords_mask);
    let x_coords = _mm_srli_epi16::<8>(x_coords);

    // Mask just the y coordinates from each `PackedIdx`, and shift them right
    // by 4 bits into the lower 4 bits of their epi8 lane.
    let white_y_coords_mask = _mm_set1_epi16(0xf0_00u16 as i16);
    let y_coords = _mm_and_si128(pawns, white_y_coords_mask);
    let y_coords = _mm_srli_epi16::<4>(y_coords);

    // Combine the x and y coordinate vectors.
    let pawns = _mm_or_si128(x_coords, y_coords);
    // Record which `PackedIdx`s were originally `null`.
    let null_mask = Self::null_mask(pawns);

    let centered_pawns = Self::centered_by(pawns, origin);

    Self {
      pawns: centered_pawns,
      null_mask,
    }
  }

  #[target_feature(enable = "sse4.1")]
  fn broadcast_hex_pos(pos: HexPos) -> __m128i {
    let x = pos.x();
    let y = pos.y();
    if x > u8::MAX as u32 || y > u8::MAX as u32 {
      unreachable();
    }
    _mm_set1_epi16((x | (y << 8)) as i16)
  }

  /// Subtracts `origin` from each coordinate pair in adjacent epi8 lanes of
  /// `pawns`, making each coordinate relative to `origin`.
  #[target_feature(enable = "sse4.1")]
  fn centered_by(pawns: __m128i, origin: HexPos) -> __m128i {
    let origin_array = Self::broadcast_hex_pos(origin);
    _mm_sub_epi8(pawns, origin_array)
  }

  #[target_feature(enable = "sse4.1")]
  fn null_mask(pawns: __m128i) -> __m128i {
    _mm_cmpeq_epi16(pawns, _mm_setzero_si128())
  }

  /// Given the pawn_poses from the Onoro state, extracts just the 8 black
  /// pawns and packs them into a __m128i register. `null` indexes are ignored.
  pub fn extract_black_pawns(pawn_poses: &[PackedIdx; N], origin: HexPos) -> Self {
    unsafe { Self::extract_black_pawns_sse(pawn_poses, origin) }
  }

  /// Given the pawn_poses from the Onoro state, extracts just the 8 white
  /// pawns and packs them into a __m128i register. `null` indexes are ignored.
  pub fn extract_white_pawns(pawn_poses: &[PackedIdx; N], origin: HexPos) -> Self {
    unsafe { Self::extract_white_pawns_sse(pawn_poses, origin) }
  }

  /// Applies a rotation/reflection from the D6 group about the origin of the
  /// board for each pawn in the list, where `op_ord` is the ordinal of the D6
  /// operation.
  pub fn apply_d6_c(&self, op_ord: usize) -> Self {
    Self {
      pawns: unsafe { rotate_impl::apply_d6_c_sse(self.pawns, op_ord) },
      ..*self
    }
  }

  /// Applies a rotation/reflection from the respective group for the given
  /// `symm_class` for each pawn in the list, where `op_ord` is the ordinal of
  /// the operation for the derived group.
  pub fn apply(&self, symm_class: SymmetryClass, op_ord: usize) -> Self {
    Self {
      pawns: unsafe { rotate_impl::apply_sse(self.pawns, symm_class, op_ord) },
      ..*self
    }
  }

  /// Sets all coordinates to (127, 127) which were `null` in the original
  /// pawns array when the coordinates were extracted. This is an impossible
  /// relative coordinate given we are playing with 16 pawns)
  #[target_feature(enable = "sse4.1")]
  fn remove_null_pawns(&self) -> __m128i {
    let impossible_coords = _mm_set1_epi8(i8::MAX);
    _mm_blendv_epi8(self.pawns, impossible_coords, self.null_mask)
  }

  /// Returns true if the two pawn lists are equal, ignoring the order of the
  /// elements.
  #[target_feature(enable = "sse4.1")]
  fn equal_ignoring_order_sse(&self, other: PawnList8) -> bool {
    debug_assert_eq!(
      _mm_movemask_epi8(self.null_mask).count_ones(),
      _mm_movemask_epi8(other.null_mask).count_ones()
    );

    let pawns1 = self.remove_null_pawns();
    let pawns2 = other.remove_null_pawns();

    // Replicate each of the lower/higher four epi16 channels into adjacent
    // pairs of channels.
    let lo_pawns1 = _mm_unpacklo_epi16(pawns1, pawns1);
    let hi_pawns1 = _mm_unpackhi_epi16(pawns1, pawns1);

    // Broadcast all eight different coordinates into a full __m128i register.
    let total = [
      _mm_shuffle_epi32::<0b00_00_00_00>(lo_pawns1),
      _mm_shuffle_epi32::<0b01_01_01_01>(lo_pawns1),
      _mm_shuffle_epi32::<0b10_10_10_10>(lo_pawns1),
      _mm_shuffle_epi32::<0b11_11_11_11>(lo_pawns1),
      _mm_shuffle_epi32::<0b00_00_00_00>(hi_pawns1),
      _mm_shuffle_epi32::<0b01_01_01_01>(hi_pawns1),
      _mm_shuffle_epi32::<0b10_10_10_10>(hi_pawns1),
      _mm_shuffle_epi32::<0b11_11_11_11>(hi_pawns1),
    ]
    .into_iter()
    // Compare each mask to the other pawns list.
    .map(|search_mask| _mm_cmpeq_epi16(pawns2, search_mask))
    .reduce(|l, r| _mm_add_epi16(l, r));

    // The pawn lists were equal if all coordinates from `pawns1` were found in
    // `pawns2`.
    _mm_movemask_epi8(unsafe { total.unwrap_unchecked() }) == 0xffff
  }

  /// Returns true if `self` and `other` contain the same coordinates in some
  /// order.
  pub fn equal_ignoring_order(&self, other: PawnList8) -> bool {
    unsafe { self.equal_ignoring_order_sse(other) }
  }

  #[target_feature(enable = "sse4.1")]
  fn zero_null_pawns(&self, pawns: __m128i) -> __m128i {
    _mm_andnot_si128(self.null_mask, pawns)
  }

  #[target_feature(enable = "sse4.1")]
  fn pawn_indices_sse<const N: usize>(&self, origin: HexPos) -> impl Iterator<Item = usize> {
    debug_assert!(N.is_power_of_two());

    let origin_array = Self::broadcast_hex_pos(origin);
    let absolute_pawns = _mm_add_epi8(self.pawns, origin_array);
    let x_coords_mask = _mm_set1_epi16(0x00_ff);
    let x_coords = _mm_and_si128(absolute_pawns, x_coords_mask);

    let y_coords_mask = _mm_set1_epi16(0xff_00u16 as i16);
    let y_coords = _mm_and_si128(absolute_pawns, y_coords_mask);

    let y_shifted = match const { 8 - N.trailing_zeros() } {
      0 => _mm_srli_epi16::<0>(y_coords),
      1 => _mm_srli_epi16::<1>(y_coords),
      2 => _mm_srli_epi16::<2>(y_coords),
      3 => _mm_srli_epi16::<3>(y_coords),
      4 => _mm_srli_epi16::<4>(y_coords),
      5 => _mm_srli_epi16::<5>(y_coords),
      6 => _mm_srli_epi16::<6>(y_coords),
      7 => _mm_srli_epi16::<7>(y_coords),
      _ => unreachable(),
    };

    let indices = self.zero_null_pawns(_mm_add_epi16(x_coords, y_shifted));
    indices.iter_epi16().map(|i| i as usize)
  }

  pub fn pawn_indices<const N: usize>(&self, origin: HexPos) -> impl Iterator<Item = usize> {
    unsafe { self.pawn_indices_sse::<N>(origin) }
  }
}

#[cfg(not(target_feature = "sse4.1"))]
#[derive(Clone, Copy)]
pub struct PawnList8 {
  pawns: [Option<HexPosOffset>; 8],
}

#[cfg(not(target_feature = "sse4.1"))]
impl PawnList8 {
  pub fn extract_black_pawns(pawn_poses: &[PackedIdx; N], origin: HexPos) -> Self {
    let pawns = [
      pawn_poses[0]
        .is_nonnull()
        .then_some(HexPos::from(pawn_poses[0]) - origin),
      pawn_poses[2]
        .is_nonnull()
        .then_some(HexPos::from(pawn_poses[2]) - origin),
      pawn_poses[4]
        .is_nonnull()
        .then_some(HexPos::from(pawn_poses[4]) - origin),
      pawn_poses[6]
        .is_nonnull()
        .then_some(HexPos::from(pawn_poses[6]) - origin),
      pawn_poses[8]
        .is_nonnull()
        .then_some(HexPos::from(pawn_poses[8]) - origin),
      pawn_poses[10]
        .is_nonnull()
        .then_some(HexPos::from(pawn_poses[10]) - origin),
      pawn_poses[12]
        .is_nonnull()
        .then_some(HexPos::from(pawn_poses[12]) - origin),
      pawn_poses[14]
        .is_nonnull()
        .then_some(HexPos::from(pawn_poses[14]) - origin),
    ];
    Self { pawns }
  }

  pub fn extract_white_pawns(pawn_poses: &[PackedIdx; N], origin: HexPos) -> Self {
    let pawns = [
      pawn_poses[1]
        .is_nonnull()
        .then_some(HexPos::from(pawn_poses[1]) - origin),
      pawn_poses[3]
        .is_nonnull()
        .then_some(HexPos::from(pawn_poses[3]) - origin),
      pawn_poses[5]
        .is_nonnull()
        .then_some(HexPos::from(pawn_poses[5]) - origin),
      pawn_poses[7]
        .is_nonnull()
        .then_some(HexPos::from(pawn_poses[7]) - origin),
      pawn_poses[9]
        .is_nonnull()
        .then_some(HexPos::from(pawn_poses[9]) - origin),
      pawn_poses[11]
        .is_nonnull()
        .then_some(HexPos::from(pawn_poses[11]) - origin),
      pawn_poses[13]
        .is_nonnull()
        .then_some(HexPos::from(pawn_poses[13]) - origin),
      pawn_poses[15]
        .is_nonnull()
        .then_some(HexPos::from(pawn_poses[15]) - origin),
    ];
    Self { pawns }
  }

  pub fn apply_d6_c(&self, op_ord: usize) -> Self {
    Self {
      pawns: self
        .pawns
        .map(|pos| pos.map(|pos| pos.apply_d6_c(&D6::from_ord(op_ord)))),
    }
  }

  fn apply_d3_v(&self, op: &D3) -> Self {
    Self {
      pawns: self.pawns.map(|pos| pos.map(|pos| pos.apply_d3_v(op))),
    }
  }

  fn apply_k4_e(&self, op: &K4) -> Self {
    Self {
      pawns: self.pawns.map(|pos| pos.map(|pos| pos.apply_k4_e(op))),
    }
  }

  fn apply_c2_cv(&self, op: &C2) -> Self {
    Self {
      pawns: self.pawns.map(|pos| pos.map(|pos| pos.apply_c2_cv(op))),
    }
  }

  fn apply_c2_ce(&self, op: &C2) -> Self {
    Self {
      pawns: self.pawns.map(|pos| pos.map(|pos| pos.apply_c2_ce(op))),
    }
  }

  fn apply_c2_ev(&self, op: &C2) -> Self {
    Self {
      pawns: self.pawns.map(|pos| pos.map(|pos| pos.apply_c2_ev(op))),
    }
  }

  fn apply_trivial(&self, op: &Trivial) -> Self {
    *self
  }

  pub fn apply(&self, symm_class: SymmetryClass, op_ord: usize) -> Self {
    match symm_class {
      SymmetryClass::C => self.apply_d6_c(op_ord),
      SymmetryClass::V => self.apply_d3_v(&D3::from_ord(op_ord)),
      SymmetryClass::E => self.apply_k4_e(&K4::from_ord(op_ord)),
      SymmetryClass::CV => self.apply_c2_cv(&C2::from_ord(op_ord)),
      SymmetryClass::CE => self.apply_c2_ce(&C2::from_ord(op_ord)),
      SymmetryClass::EV => self.apply_c2_ev(&C2::from_ord(op_ord)),
      SymmetryClass::Trivial => self.apply_trivial(&Trivial::from_ord(op_ord)),
    }
  }

  /// Returns true if `self` and `other` contain the same coordinates in some
  /// order.
  pub fn equal_ignoring_order(&self, other: Self) -> bool {
    self
      .pawns
      .iter()
      .all(|pos| pos.is_none_or(|pos| other.pawns.contains(&Some(pos))))
  }

  pub fn pawn_indices<const N: usize>(&self, origin: HexPos) -> impl Iterator<Item = usize> {
    self.pawns.iter().map(move |&pos| {
      if let Some(pos) = pos {
        let pos = origin + pos;
        debug_assert!(pos.x() < N as u32 && pos.y() < N as u32);
        pos.x() as usize + pos.y() as usize * N
      } else {
        0
      }
    })
  }
}

#[cfg(test)]
mod tests {
  #[cfg(target_feature = "sse4.1")]
  use std::arch::x86_64::{_mm_bsrli_si128, _mm_cvtsi128_si64x};

  use algebra::{group::Trivial, ordinal::Ordinal, semigroup::Semigroup};
  use googletest::{gtest, prelude::*};
  use itertools::Itertools;
  use onoro::{
    groups::{C2, D3, D6, K4, SymmetryClass},
    hex_pos::{HexPos, HexPosOffset},
  };
  use rand::{Rng, SeedableRng, rngs::StdRng};

  use crate::{
    PackedIdx,
    pawn_list::{N, PawnList8},
  };

  #[cfg(target_feature = "sse4.1")]
  #[target_feature(enable = "sse4.1")]
  fn pos_at_sse(pawn_list: &PawnList8, idx: usize) -> HexPosOffset {
    debug_assert!(idx < 8);
    let pawns = match idx {
      0 => _mm_bsrli_si128::<0>(pawn_list.pawns),
      1 => _mm_bsrli_si128::<2>(pawn_list.pawns),
      2 => _mm_bsrli_si128::<4>(pawn_list.pawns),
      3 => _mm_bsrli_si128::<6>(pawn_list.pawns),
      4 => _mm_bsrli_si128::<8>(pawn_list.pawns),
      5 => _mm_bsrli_si128::<10>(pawn_list.pawns),
      6 => _mm_bsrli_si128::<12>(pawn_list.pawns),
      7 => _mm_bsrli_si128::<14>(pawn_list.pawns),
      _ => unreachable!(),
    };
    let pos = _mm_cvtsi128_si64x(pawns);
    HexPosOffset::new((pos & 0xff) as i8 as i32, ((pos >> 8) & 0xff) as i8 as i32)
  }

  fn pos_at(pawn_list: &PawnList8, idx: usize) -> HexPosOffset {
    #[cfg(target_feature = "sse4.1")]
    unsafe {
      pos_at_sse(pawn_list, idx)
    }
    #[cfg(not(target_feature = "sse4.1"))]
    pawn_list.pawns[idx].unwrap()
  }

  fn positions(pawn_list: &PawnList8) -> Vec<HexPosOffset> {
    (0..8).map(|idx| pos_at(pawn_list, idx)).collect()
  }

  #[gtest]
  fn test_extract() {
    let pawns = [
      PackedIdx::new(1, 8),
      PackedIdx::new(3, 9),
      PackedIdx::new(2, 10),
      PackedIdx::new(4, 11),
      PackedIdx::new(3, 12),
      PackedIdx::new(5, 11),
      PackedIdx::new(4, 10),
      PackedIdx::new(6, 9),
      PackedIdx::new(5, 12),
      PackedIdx::new(7, 11),
      PackedIdx::new(6, 10),
      PackedIdx::new(2, 9),
      PackedIdx::new(7, 8),
      PackedIdx::new(1, 9),
      PackedIdx::new(8, 10),
      PackedIdx::new(2, 11),
    ];

    let black_pawn_list = PawnList8::extract_black_pawns(&pawns, HexPos::new(4, 10));
    let white_pawn_list = PawnList8::extract_white_pawns(&pawns, HexPos::new(4, 10));

    expect_that!(
      positions(&black_pawn_list),
      elements_are![
        &HexPosOffset::new(-3, -2),
        &HexPosOffset::new(-2, 0),
        &HexPosOffset::new(-1, 2),
        &HexPosOffset::new(0, 0),
        &HexPosOffset::new(1, 2),
        &HexPosOffset::new(2, 0),
        &HexPosOffset::new(3, -2),
        &HexPosOffset::new(4, 0),
      ]
    );

    expect_that!(
      positions(&white_pawn_list),
      elements_are![
        &HexPosOffset::new(-1, -1),
        &HexPosOffset::new(0, 1),
        &HexPosOffset::new(1, 1),
        &HexPosOffset::new(2, -1),
        &HexPosOffset::new(3, 1),
        &HexPosOffset::new(-2, -1),
        &HexPosOffset::new(-3, -1),
        &HexPosOffset::new(-2, 1),
      ]
    );
  }

  macro_rules! test_rotate {
    ($name:ident, $apply_op:ident, $symm_class:expr, $op_t:ty) => {
      #[gtest]
      fn $name() {
        for y in 1..=15 {
          let mut poses = [PackedIdx::null(); N];
          for (x, pos) in poses.iter_mut().enumerate() {
            *pos = PackedIdx::new(x.max(1) as u32, y);
          }

          let center = HexPos::new(6, 10);
          let black_pawns = PawnList8::extract_black_pawns(&poses, center);
          let white_pawns = PawnList8::extract_white_pawns(&poses, center);

          for op in <$op_t>::for_each() {
            let op_ord = op.ord();
            let rotated_black = black_pawns.apply($symm_class, op_ord);
            let rotated_white = white_pawns.apply($symm_class, op_ord);

            let expected_black = poses
              .iter()
              .step_by(2)
              .map(|&idx| HexPos::from(idx) - center)
              .map(|pos| pos.$apply_op(&op))
              .collect_vec();
            let expected_white = poses
              .iter()
              .skip(1)
              .step_by(2)
              .map(|&idx| HexPos::from(idx) - center)
              .map(|pos| pos.$apply_op(&op))
              .collect_vec();

            assert_that!(positions(&rotated_black), container_eq(expected_black));
            assert_that!(positions(&rotated_white), container_eq(expected_white));
          }
        }
      }
    };
  }

  test_rotate!(test_rotate_d6_c, apply_d6_c, SymmetryClass::C, D6);
  test_rotate!(test_rotate_d3_v, apply_d3_v, SymmetryClass::V, D3);
  test_rotate!(test_rotate_k4_e, apply_k4_e, SymmetryClass::E, K4);
  test_rotate!(test_rotate_c2_cv, apply_c2_cv, SymmetryClass::CV, C2);
  test_rotate!(test_rotate_c2_ce, apply_c2_ce, SymmetryClass::CE, C2);
  test_rotate!(test_rotate_c2_ev, apply_c2_ev, SymmetryClass::EV, C2);
  test_rotate!(
    test_rotate_trivial,
    apply_trivial,
    SymmetryClass::Trivial,
    Trivial
  );

  fn equal_ignoring_order<'a>(
    lhs: impl IntoIterator<Item = &'a PackedIdx>,
    rhs: impl IntoIterator<Item = &'a PackedIdx>,
  ) -> bool {
    lhs
      .into_iter()
      .map(|pos| (pos.x(), pos.y()))
      .sorted()
      .collect_vec()
      == rhs
        .into_iter()
        .map(|pos| (pos.x(), pos.y()))
        .sorted()
        .collect_vec()
  }

  fn gen_unique_poses<R: Rng>(count: usize, rng: &mut R) -> impl Iterator<Item = PackedIdx> {
    let mut poses = Vec::with_capacity(count);
    for _ in 0..count {
      let pos = loop {
        let pos = PackedIdx::new(rng.gen_range(1..15), rng.gen_range(1..15));
        if poses.contains(&pos) {
          continue;
        }

        break pos;
      };

      poses.push(pos);
    }

    poses.into_iter()
  }

  fn randomly_mutate<R: Rng>(poses: &mut [PackedIdx], rng: &mut R) {
    let to_change = (0..poses.len()).map(|_| rng.gen_bool(0.4)).collect_vec();
    for (i, _) in to_change
      .iter()
      .cloned()
      .enumerate()
      .filter(|&(_, to_change)| to_change)
    {
      let pos = loop {
        let pos = PackedIdx::new(rng.gen_range(1..15), rng.gen_range(1..15));
        if poses
          .iter()
          .enumerate()
          .find(|&(_, &p)| pos == p)
          .is_some_and(|(idx, _)| idx < i || !to_change[idx])
        {
          continue;
        }

        break pos;
      };

      poses[i] = pos;
    }
  }

  #[test]
  fn fuzz_equals_ignoring_order() {
    const ITERATIONS: u32 = 10_000;

    let mut rng = StdRng::seed_from_u64(19304910);

    for t in 0..ITERATIONS {
      let origin = HexPos::new(rng.gen_range(1..15), rng.gen_range(1..15));
      let count = if rng.gen_bool(0.75) {
        16
      } else {
        rng.gen_range(1..=16)
      };

      let mut poses1 = [PackedIdx::null(); N];
      let mut poses2 = [PackedIdx::null(); N];
      for ((pos1, pos2), random_pos) in poses1
        .iter_mut()
        .zip(poses2.iter_mut())
        .zip(gen_unique_poses(N, &mut rng))
        .take(count)
      {
        *pos1 = random_pos;
        *pos2 = random_pos;
      }

      let (black_equal, white_equal) = if rng.gen_bool(0.5) {
        (true, true)
      } else {
        // Generate different positions.
        randomly_mutate(&mut poses2[0..count], &mut rng);

        (
          equal_ignoring_order(poses1.iter().step_by(2), poses2.iter().step_by(2)),
          equal_ignoring_order(
            poses1.iter().skip(1).step_by(2),
            poses2.iter().skip(1).step_by(2),
          ),
        )
      };

      // Shuffle the even indices of poses2.
      for i in 2..N {
        let j = 2 * rng.gen_range(0..=(i / 2)) + (i % 2);
        poses2.swap(i, j);
      }

      let black_pawns1 = PawnList8::extract_black_pawns(&poses1, origin);
      let black_pawns2 = PawnList8::extract_black_pawns(&poses2, origin);
      assert_eq!(
        black_pawns1.equal_ignoring_order(black_pawns2),
        black_equal,
        "Iteration {t}"
      );

      let white_pawns1 = PawnList8::extract_white_pawns(&poses1, origin);
      let white_pawns2 = PawnList8::extract_white_pawns(&poses2, origin);
      assert_eq!(
        white_pawns1.equal_ignoring_order(white_pawns2),
        white_equal,
        "Iteration {t}"
      );
    }
  }

  #[test]
  fn test_equals_with_zero() {
    let origin = HexPos::new(8, 8);

    let mut poses1 = [PackedIdx::null(); N];
    let mut poses2 = [PackedIdx::null(); N];
    poses1[0] = PackedIdx::new(8, 9);
    poses1[2] = PackedIdx::new(9, 9);
    poses2[0] = PackedIdx::new(8, 9);
    poses2[2] = PackedIdx::new(8, 8);

    let black_pawns1 = PawnList8::extract_black_pawns(&poses1, origin);
    let black_pawns2 = PawnList8::extract_black_pawns(&poses2, origin);
    assert!(!black_pawns1.equal_ignoring_order(black_pawns2));
  }

  #[gtest]
  fn test_pawn_indices() {
    for y in 1..=15 {
      for l in 1..=8 {
        let mut poses = [PackedIdx::null(); N];
        for (x, pos) in poses.iter_mut().step_by(2).take(l).enumerate() {
          *pos = PackedIdx::new(x as u32 + 1, y);
        }

        let center = HexPos::new(N as u32 / 2 + 1, y);
        let pawns = PawnList8::extract_black_pawns(&poses, center);

        let other_center = HexPos::new(9, 8);
        let expected_indices = poses
          .iter()
          .step_by(2)
          .take(l)
          .map(|&idx| {
            let pos = HexPos::from(idx) - center + other_center;
            pos.x() as usize + pos.y() as usize * N
          })
          .chain(std::iter::once(0).cycle().take(8 - l))
          .collect_vec();

        let indices = pawns.pawn_indices::<N>(other_center).collect_vec();
        assert_that!(indices, container_eq(expected_indices), "y = {y}, l = {l}");
      }
    }
  }
}
