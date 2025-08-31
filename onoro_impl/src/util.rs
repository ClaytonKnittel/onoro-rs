use std::arch::x86_64::*;

use itertools::Itertools;
use num_traits::PrimInt;

use crate::{FilterNullPackedIdx, PackedIdx};

#[inline]
pub const fn unreachable() -> ! {
  #[cfg(debug_assertions)]
  unreachable!();
  #[cfg(not(debug_assertions))]
  unsafe {
    std::hint::unreachable_unchecked()
  }
}

#[inline(always)]
#[cold]
fn cold() {}

#[inline(always)]
pub fn likely(b: bool) -> bool {
  if !b {
    cold()
  }
  b
}

#[inline(always)]
pub fn unlikely(b: bool) -> bool {
  if b {
    cold()
  }
  b
}

macro_rules! define_cmp {
  ($max_name:ident, $min_name:ident, $t:ty) => {
    #[allow(dead_code)]
    #[inline]
    pub const fn $max_name(a: $t, b: $t) -> $t {
      [a, b][(a < b) as usize]
    }

    #[allow(dead_code)]
    #[inline]
    pub const fn $min_name(a: $t, b: $t) -> $t {
      [a, b][(a >= b) as usize]
    }
  };
}

// const-context-compatible integer comparison methods.
define_cmp!(max_u8, min_u8, u8);
define_cmp!(max_u16, min_u16, u16);
define_cmp!(max_u32, min_u32, u32);
define_cmp!(max_u64, min_u64, u64);
define_cmp!(max_i8, min_i8, i8);
define_cmp!(max_i16, min_i16, i16);
define_cmp!(max_i32, min_i32, i32);
define_cmp!(max_i64, min_i64, i64);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MinAndMax<I: PrimInt> {
  min: I,
  max: I,
}

impl<I: PrimInt> MinAndMax<I> {
  pub fn new(min: I, max: I) -> Self {
    Self { min, max }
  }

  pub fn min(&self) -> I {
    self.min
  }

  pub fn max(&self) -> I {
    self.max
  }

  pub fn delta(&self) -> I {
    self.max - self.min
  }

  pub fn acc(self, value: I) -> Self {
    Self {
      min: self.min.min(value),
      max: self.max.max(value),
    }
  }
}

impl<I: PrimInt> Default for MinAndMax<I> {
  fn default() -> Self {
    Self {
      min: I::max_value(),
      max: I::min_value(),
    }
  }
}

/// Sorts the 16-bit lanes of `vec`, returning the sorted result.
#[allow(dead_code)]
#[inline]
#[target_feature(enable = "sse4.1")]
pub fn sort_epi16(vec: __m128i) -> __m128i {
  #[target_feature(enable = "sse4.1")]
  fn sort_epi32_pairs<const SHUFFLE_MASK: i32>(vec: __m128i, lower_positions: __m128i) -> __m128i {
    let shuffled = _mm_shuffle_epi32::<SHUFFLE_MASK>(vec);
    let cmp = _mm_cmplt_epi16(vec, shuffled);
    let select = _mm_add_epi8(cmp, lower_positions);
    _mm_blendv_epi8(vec, shuffled, select)
  }

  #[target_feature(enable = "sse4.1")]
  fn sort_epi8_pairs(vec: __m128i, shuffle_mask: __m128i, lower_positions: __m128i) -> __m128i {
    let shuffled = _mm_shuffle_epi8(vec, shuffle_mask);
    let cmp = _mm_cmplt_epi16(vec, shuffled);
    let select = _mm_add_epi8(cmp, lower_positions);
    _mm_blendv_epi8(vec, shuffled, select)
  }

  // Implemented using the optimal sorting network for size = 8:
  // [(0,2),(1,3),(4,6),(5,7)]
  // shuffled: [2, 3, 0, 1, 6, 7, 4, 5]
  let vec = sort_epi32_pairs::<0b10_11_00_01>(vec, _mm_set1_epi64x(0x0000_0000_8080_8080));

  // [(0,4),(1,5),(2,6),(3,7)]
  // shuffled: [4, 5, 6, 7, 0, 1, 2, 3]
  let vec =
    sort_epi32_pairs::<0b01_00_11_10>(vec, _mm_set_epi64x(0, 0x8080_8080_8080_8080u64 as i64));

  // [(0,1),(2,3),(4,5),(6,7)]
  let vec = sort_epi8_pairs(
    vec,
    _mm_set_epi64x(0x0d0c_0f0e_0908_0b0a, 0x0504_0706_0100_0302),
    _mm_set1_epi32(0x0000_8080),
  );

  // [(2,4),(3,5)]
  let vec =
    sort_epi32_pairs::<0b11_01_10_00>(vec, _mm_set1_epi64x(0x8080_8080_0000_0000u64 as i64));

  // [(1,4),(3,6)]
  let vec = sort_epi8_pairs(
    vec,
    _mm_set_epi64x(0x0f0e_0706_0b0a_0302, 0x0d0c_0504_0908_0100),
    _mm_set1_epi32(0x8080_0000u32 as i32),
  );

  // [(1,2),(3,4),(5,6)]
  sort_epi8_pairs(
    vec,
    _mm_set_epi64x(0x0f0e_0b0a_0d0c_0706, 0x0908_0302_0504_0100),
    _mm_set1_epi32(0x8080_0000u32 as i32),
  )
}

#[inline]
#[target_feature(enable = "ssse3")]
unsafe fn packed_positions_to_mask_sse3(packed_positions: u64) -> u64 {
  debug_assert!(
    packed_positions
      .to_ne_bytes()
      .into_iter()
      .all(|byte| byte < 0x10),
    "Packed positions must all be < 0x10: {packed_positions:#016x}"
  );
  debug_assert!(
    packed_positions
      .to_ne_bytes()
      .into_iter()
      .filter(|&byte| byte != 0)
      .all_unique(),
    "Packed positions must be unique and non-zero: {packed_positions:#016x}"
  );

  // We construct a map from index to (1 << (index - 1)) (0 if index == 0).
  // We will be invoking _mm_shuffle_epi8, which uses these vectors as a lookup
  // table.
  //
  // Since the range of bytes we support is 0 - 15, we need 2 bytes to
  // represent (1 << (index - 1)) for all possible indices. The size of entries
  // in the lookup table is 1 byte, so we have two lookup tables, one holding
  // the lower half of the masks and one holding the upper half.
  #[rustfmt::skip]
  let lo_data = _mm_set_epi8(
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80u8 as i8,
    0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01, 0x00,
  );
  #[rustfmt::skip]
  let hi_data = _mm_set_epi8(
    0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
  );

  // Fill the lower half of a __m128i register with the packed positions.
  let shuffle_mask = _mm_set_epi64x(0, packed_positions as i64);

  // Construct masks containing the lower and upper halves of the corresponding
  // masks for each packed position. Note that only the first 8 bytes of each
  // will be non-zero, since `shuffle_mask` has a zero upper half.
  let lo_masks = _mm_shuffle_epi8(lo_data, shuffle_mask);
  let hi_masks = _mm_shuffle_epi8(hi_data, shuffle_mask);

  // Interleave the byte of these two masks. Now, each 16-byte entry of `masks`
  // contains a complete masks for some packed position.
  let masks = _mm_unpacklo_epi8(lo_masks, hi_masks);

  // Pull out the lower and upper halves of the masks array into two u64's.
  let lo_masks = _mm_cvtsi128_si64x(masks) as u64;
  let masks = _mm_unpackhi_epi64(masks, masks);
  let hi_masks = _mm_cvtsi128_si64x(masks) as u64;

  // Merge all masks into a single 16-bit mask. Note that all masks are
  // guaranteed to be unique, so we can add them instead of bitwise-or'ing
  // them.
  let masks = lo_masks + hi_masks;
  masks.overflowing_mul(0x0001_0001_0001_0001).0 >> 48
}

#[cfg(any(test, not(target_feature = "ssse3")))]
#[inline]
fn packed_positions_to_mask_slow(packed_positions: u64) -> u64 {
  packed_positions
    .to_ne_bytes()
    .into_iter()
    .filter(|&byte| byte != 0)
    .fold(0, |mask, byte| mask | (1u64 << (byte - 1)))
}

/// Returns a u64 with bits set in the positions indicated by the lowest 4 bits
/// of the byte values in `packed_positions`, minus 1. Zero byte values are
/// ignored. All non-zero bytes must be unique.
#[inline(always)]
pub fn packed_positions_to_mask(packed_positions: u64) -> u64 {
  #[cfg(target_feature = "ssse3")]
  unsafe {
    packed_positions_to_mask_sse3(packed_positions)
  }
  #[cfg(not(target_feature = "ssse3"))]
  packed_positions_to_mask_slow(packed_positions)
}

#[inline]
#[target_feature(enable = "ssse3")]
unsafe fn equal_mask_epi8_sse3(byte_vec: u64, needle: u8) -> u64 {
  let b = needle as i8;
  // Broadcast needle to the first 8 bytes of a __m128i register.
  let needle_mask = _mm_set_epi8(0, 0, 0, 0, 0, 0, 0, 0, b, b, b, b, b, b, b, b);
  // Move `byte_vec` into the lower half of a __m128i register.
  let byte_vec = _mm_set_epi64x(0, byte_vec as i64);

  // cmpeq_epi8 sets each byte of the result to 0xff if the two corresponding
  // bytes from `needle_mask` and `byte_vec` are equal, or 0x00 if they are
  // not.
  let equal_mask = _mm_cmpeq_epi8(needle_mask, byte_vec);
  // Since we were only operating in the lower half, we can simply extract the
  // lower 64 bits of the result.
  _mm_cvtsi128_si64x(equal_mask) as u64
}

#[cfg(any(test, not(target_feature = "ssse3")))]
#[inline]
fn equal_mask_epi8_slow(byte_vec: u64, needle: u8) -> u64 {
  byte_vec
    .to_ne_bytes()
    .into_iter()
    .map(|byte| if byte == needle { 0xff } else { 0 })
    .enumerate()
    .fold(0, |mask, (i, byte)| mask | (byte << (i * 8)))
}

/// Compares packed 8-bit integers in `byte_vec` with `needle` for equality.
/// Returns a u64 mask where each corresponding byte from `byte_vec` is set to
/// `0xff` if it matches `needle`, or `0x00` if it does not.
#[inline(always)]
pub fn equal_mask_epi8(byte_vec: u64, needle: u8) -> u64 {
  #[cfg(target_feature = "ssse3")]
  unsafe {
    equal_mask_epi8_sse3(byte_vec, needle)
  }
  #[cfg(not(target_feature = "ssse3"))]
  equal_mask_epi8_slow(byte_vec, needle)
}

#[target_feature(enable = "ssse3")]
unsafe fn horizontal_compress<F>(v: __m128i, mut compressor: F) -> u8
where
  F: FnMut(__m128i, __m128i) -> __m128i,
{
  let v = compressor(v, _mm_unpackhi_epi64(v, v));
  let v = compressor(v, _mm_bsrli_si128(v, 4));
  let v = compressor(v, _mm_bsrli_si128(v, 2));
  let v = compressor(v, _mm_bsrli_si128(v, 1));

  let result = _mm_cvtsi128_si32(v) as u32;
  result as u8
}

#[derive(Default, PartialEq, Eq, Debug)]
pub struct CoordLimits {
  x: MinAndMax<u32>,
  y: MinAndMax<u32>,
  xy: MinAndMax<u32>,
}

impl CoordLimits {
  pub fn new(x: MinAndMax<u32>, y: MinAndMax<u32>, xy: MinAndMax<u32>) -> Self {
    Self { x, y, xy }
  }

  pub fn x(&self) -> MinAndMax<u32> {
    self.x
  }

  pub fn y(&self) -> MinAndMax<u32> {
    self.y
  }

  pub fn xy(&self) -> MinAndMax<u32> {
    self.xy
  }
}

#[inline]
#[target_feature(enable = "ssse3")]
fn packed_positions_coord_limits_sse3(pawn_poses: &[PackedIdx]) -> CoordLimits {
  const N: usize = 16;
  debug_assert_eq!(pawn_poses.len(), N);

  let min_max_ignore_zero = |vec: __m128i, zeros: __m128i| -> MinAndMax<u32> {
    let min_vec = _mm_or_si128(vec, zeros);
    let max_vec = vec;

    let min = unsafe { horizontal_compress(min_vec, |v1, v2| _mm_min_epu8(v1, v2)) };
    let max = unsafe { horizontal_compress(max_vec, |v1, v2| _mm_max_epu8(v1, v2)) };

    MinAndMax::new(min as u32, max as u32)
  };

  /// Selects the x-coordinates of every PackedIdx position.
  const SELECT_X_MASK: i64 = 0x0f0f_0f0f_0f0f_0f0f;
  let select_x = _mm_set1_epi64x(SELECT_X_MASK);

  let pawns = unsafe { _mm_loadu_si128(pawn_poses.as_ptr() as *const _) };

  // Mark the zero bytes of pawns.
  let zeros = _mm_cmpeq_epi8(pawns, _mm_setzero_si128());

  let x_coords = _mm_and_si128(pawns, select_x);
  let y_coords = _mm_and_si128(_mm_srli_epi64::<4>(pawns), select_x);

  // Derive (y - x) from x_ and y_coords. To prevent underflow, we add
  // xy_offset from PackedIdx (which will match PackedIdx::xy()).
  let diff_offset = _mm_set1_epi8(PackedIdx::xy_offset::<N>() as i8);
  // Mask off the offset for the zero bytes.
  let diff_offset = _mm_andnot_si128(zeros, diff_offset);
  // Calculate `y + xy_offset - x`.
  let xy_coords = _mm_sub_epi8(_mm_add_epi8(y_coords, diff_offset), x_coords);

  CoordLimits::new(
    min_max_ignore_zero(x_coords, zeros),
    min_max_ignore_zero(y_coords, zeros),
    min_max_ignore_zero(xy_coords, zeros),
  )
}

#[inline]
fn packed_positions_coord_limits_slow<const N: usize>(pawn_poses: &[PackedIdx; N]) -> CoordLimits {
  debug_assert!(!pawn_poses.is_empty(), "Pawn positions cannot be empty");

  pawn_poses
    .iter()
    .filter_null()
    .fold(CoordLimits::default(), |CoordLimits { x, y, xy }, &pos| {
      CoordLimits {
        x: x.acc(pos.x()),
        y: y.acc(pos.y()),
        xy: xy.acc(pos.xy::<N>()),
      }
    })
}

/// Returns the ranges of x-, y-, and (y - x)-coordinates of all the pawns in
/// `pawn_poses`, ignoring null indices.
#[inline(always)]
pub fn packed_positions_coord_limits<const N: usize>(pawn_poses: &[PackedIdx; N]) -> CoordLimits {
  #[cfg(target_feature = "ssse3")]
  if N == 16 {
    return unsafe { packed_positions_coord_limits_sse3(pawn_poses) };
  }
  packed_positions_coord_limits_slow(pawn_poses)
}

#[cfg(test)]
mod tests {
  #[cfg(target_feature = "sse4.1")]
  use std::arch::x86_64::*;

  #[cfg(target_feature = "sse4.1")]
  use googletest::{gtest, prelude::*};
  #[cfg(target_feature = "sse4.1")]
  use itertools::Itertools;
  #[cfg(target_feature = "sse4.1")]
  use rand::{Rng, SeedableRng, rngs::StdRng};
  use rstest::rstest;
  use rstest_reuse::{apply, template};

  #[cfg(target_feature = "sse4.1")]
  use crate::util::sort_epi16;
  use crate::{
    PackedIdx,
    test_util::PawnPoses,
    util::{
      CoordLimits, MinAndMax, equal_mask_epi8, equal_mask_epi8_slow, packed_positions_coord_limits,
      packed_positions_coord_limits_slow, packed_positions_to_mask, packed_positions_to_mask_slow,
    },
  };

  #[cfg(target_feature = "sse4.1")]
  #[target_feature(enable = "sse4.1")]
  fn pos_at_epi16(vec: __m128i, idx: usize) -> i16 {
    debug_assert!(idx < 8);
    let pawns = match idx {
      0 => _mm_bsrli_si128::<0>(vec),
      1 => _mm_bsrli_si128::<2>(vec),
      2 => _mm_bsrli_si128::<4>(vec),
      3 => _mm_bsrli_si128::<6>(vec),
      4 => _mm_bsrli_si128::<8>(vec),
      5 => _mm_bsrli_si128::<10>(vec),
      6 => _mm_bsrli_si128::<12>(vec),
      7 => _mm_bsrli_si128::<14>(vec),
      _ => unreachable!(),
    };
    _mm_cvtsi128_si64x(pawns) as i16
  }

  #[cfg(target_feature = "sse4.1")]
  #[gtest]
  fn fuzz_sort_epi16() {
    const ITERATIONS: u32 = 10_000;

    let mut rng = StdRng::seed_from_u64(393990259);

    for t in 0..ITERATIONS {
      let list = (0..8).map(|_| rng.r#gen::<i16>()).collect_vec();
      let expected_sorted_list = list.iter().cloned().sorted().collect_vec();

      let sorted = unsafe {
        sort_epi16(_mm_set_epi16(
          list[7], list[6], list[5], list[4], list[3], list[2], list[1], list[0],
        ))
      };
      let sorted_list = (0..8)
        .map(|i| unsafe { pos_at_epi16(sorted, i) })
        .collect_vec();

      assert_that!(
        sorted_list,
        container_eq(expected_sorted_list),
        "Failed on iteration {t}"
      );
    }
  }

  #[template]
  #[rstest]
  fn packed_positions(
    #[values(packed_positions_to_mask, packed_positions_to_mask_slow)]
    packed_positions: impl FnOnce(u64) -> u64,
    #[values(
      (0x01_02_03_04_05_06_07_08, 0xff),
      (0x01_04_05_06_07_0a_0c_0e, 0x2a79),
      (0x03_02_07_00_00_05_00_00, 0x56),
    )]
    args: (u64, u64),
  ) {
  }

  #[apply(packed_positions)]
  #[test]
  fn test_packed_positions_to_mask(packed_positions: impl FnOnce(u64) -> u64, args: (u64, u64)) {
    let (input, expected) = args;
    assert_eq!(packed_positions(input), expected);
  }

  #[template]
  #[rstest]
  fn equal_mask_epi8(
    #[values(equal_mask_epi8, equal_mask_epi8_slow)] equal_mask: impl FnOnce(u64, u8) -> u64,
    #[values(
      (0x01_02_03_04_05_06_07_08, 0x03, 0x00_00_ff_00_00_00_00_00),
      (0xaa_bb_aa_bb_bb_aa_bb_aa, 0xaa, 0xff_00_ff_00_00_ff_00_ff),
    )]
    args: (u64, u8, u64),
  ) {
  }

  #[apply(equal_mask_epi8)]
  #[test]
  fn test_equal_mask_epi8(equal_mask: impl FnOnce(u64, u8) -> u64, args: (u64, u8, u64)) {
    let (byte_vec, needle, expected) = args;
    assert_eq!(equal_mask(byte_vec, needle), expected);
  }

  const POSES_BB_EXAMPLE: PawnPoses = PawnPoses([
    PackedIdx::new(2, 1),
    PackedIdx::new(3, 2),
    PackedIdx::new(1, 5),
    PackedIdx::null(),
    PackedIdx::null(),
    PackedIdx::null(),
    PackedIdx::null(),
    PackedIdx::null(),
    PackedIdx::null(),
    PackedIdx::null(),
    PackedIdx::null(),
    PackedIdx::null(),
    PackedIdx::null(),
    PackedIdx::null(),
    PackedIdx::null(),
    PackedIdx::null(),
  ]);

  #[template]
  #[rstest]
  fn packed_positions_bounding_box<const N: usize>(
    #[values(packed_positions_coord_limits, packed_positions_coord_limits_slow)]
    packed_positions: impl FnOnce(&[PackedIdx; N]) -> CoordLimits,
    #[values(
      (
        &[PackedIdx::new(1, 1)],
        CoordLimits::new(
          MinAndMax::new(1, 1),
          MinAndMax::new(1, 1),
          MinAndMax::new(PackedIdx::xy_offset::<1>(), PackedIdx::xy_offset::<1>()),
        ),
      ),
      (
        &POSES_BB_EXAMPLE.0,
        CoordLimits::new(
          MinAndMax::new(1, 3),
          MinAndMax::new(1, 5),
          MinAndMax::new(PackedIdx::xy_offset::<16>() - 1, PackedIdx::xy_offset::<16>() + 4),
        ),
      ),
    )]
    args: (&[PackedIdx; N], CoordLimits),
  ) {
  }

  #[apply(packed_positions_bounding_box)]
  #[test]
  fn test_packed_positions_bounding_box<const N: usize>(
    packed_positions: impl FnOnce(&[PackedIdx; N]) -> CoordLimits,
    args: (&[PackedIdx; N], CoordLimits),
  ) {
    let (input, expected) = args;
    assert_eq!(packed_positions(input), expected);
  }
}
