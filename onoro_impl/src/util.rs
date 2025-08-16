use core::arch::x86_64::{_mm_set_epi8, _mm_shuffle_epi8};
use std::arch::x86_64::{
  __m128i, _mm_add_epi16, _mm_and_si128, _mm_bsrli_si128, _mm_cmpeq_epi8, _mm_cvtsi128_si32,
  _mm_cvtsi128_si64x, _mm_loadu_si128, _mm_max_epu8, _mm_min_epu8, _mm_mullo_epi16, _mm_or_si128,
  _mm_set_epi16, _mm_set_epi64x, _mm_setzero_si128, _mm_srli_epi16, _mm_sub_epi8,
  _mm_unpackhi_epi64, _mm_unpacklo_epi8,
};
#[cfg(all(target_feature = "avx512bw", target_feature = "avx512vl"))]
use std::arch::x86_64::{_mm_reduce_max_epu8, _mm_reduce_min_epu8};

use itertools::Itertools;
use onoro::hex_pos::HexPos;

use crate::PackedIdx;

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

/// Given a `u8`, returns a `u64` with each byte of the `u64` equal to the
/// passed `u8`.
#[inline(always)]
pub const fn broadcast_u8_to_u64(v: u8) -> u64 {
  const BYTE_ANCHOR: u64 = 0x0101_0101_0101_0101;
  (v as u64) * BYTE_ANCHOR
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
unsafe fn packed_positions_to_board_bitvec_sse3(
  pawn_poses: &[PackedIdx],
  lower_left: PackedIdx,
  width: u32,
) -> u64 {
  const SELECT_X_MASK: i64 = 0x0f0f_0f0f_0f0f_0f0f;

  let select_x = _mm_set_epi64x(SELECT_X_MASK, SELECT_X_MASK);
  let select_y = _mm_set_epi64x(!SELECT_X_MASK, !SELECT_X_MASK);

  let pawns = unsafe { _mm_loadu_si128(pawn_poses.as_ptr() as *const _) };

  let ll = unsafe { lower_left.bytes() } as i8;
  let lower_left_offsets = _mm_set_epi8(
    ll, ll, ll, ll, ll, ll, ll, ll, ll, ll, ll, ll, ll, ll, ll, ll,
  );

  let pawns = _mm_sub_epi8(pawns, lower_left_offsets);

  let x_coords = _mm_and_si128(pawns, select_x);
  let pawns = _mm_srli_epi16::<4>(pawns);
  let y_coords = _mm_and_si128(pawns, select_y);

  let width = width as i16;
  let width_mult = _mm_set_epi16(width, width, width, width, width, width, width, width);

  let scaled_y = _mm_mullo_epi16(y_coords, width_mult);

  let indices = _mm_add_epi16(x_coords, scaled_y);

  //
  let lo_indices = _mm_cvtsi128_si64x(indices) as u64;
  let indices = _mm_unpackhi_epi64(indices, indices);
  let hi_indices = _mm_cvtsi128_si64x(indices) as u64;

  const fn build_table() -> [u64; 64] {
    let mut arr: [u64; 64] = [0u64; 64];
    let mut i = 1;
    while i < 64 {
      arr[i] = 1u64 << i;
      i += 1;
    }
    arr
  }
  const TABLE: [u64; 64] = build_table();

  lo_indices
    .to_ne_bytes()
    .iter()
    .chain(hi_indices.to_ne_bytes().iter())
    .map(|&b| TABLE[b as usize])
    .sum()

  // #[rustfmt::skip]
  // let lo_data = _mm_set_epi8(
  //   0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
  //   0x80u8 as i8, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01,
  // );
  // #[rustfmt::skip]
  // let hi_data = _mm_set_epi8(
  //   0x80u8 as i8, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01,
  //   0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
  // );

  // let lo_masks = _mm_shuffle_epi8(lo_data, indices);
  // let hi_masks = _mm_shuffle_epi8(hi_data, indices);

  // // 16-byte complete masks, which now need to be shifted to the correct 16-bit
  // // quadrant of the final u64.
  // let masks1 = _mm_unpacklo_epi8(lo_masks, hi_masks);
  // let masks2 = _mm_unpackhi_epi8(lo_masks, hi_masks);
}

#[inline(always)]
fn packed_positions_to_board_bitvec_slow<const N: usize>(
  pawn_poses: &[PackedIdx; N],
  lower_left: PackedIdx,
  width: u32,
) -> u64 {
  let width = width as usize;
  pawn_poses
    .iter()
    .filter(|&&pos| pos != PackedIdx::null())
    .fold(0, |board_vec, &pos| {
      let d = unsafe { PackedIdx::from_idx_offset(pos - lower_left) };
      let index = d.y() as usize * width + d.x() as usize;
      debug_assert!(index > width);
      board_vec | (1 << index)
    })
}

/// Given the pawn positions, the lower-left packed index, and the width of the
/// region of pawns (with a 1-tile border on all 4 sides), returns a board
/// bitvec, which has a bit set in index x + y * width relative to the lower
/// left packed index provided.
#[inline(always)]
fn packed_positions_to_board_bitvec<const N: usize>(
  pawn_poses: &[PackedIdx; N],
  lower_left: PackedIdx,
  width: u32,
) -> u64 {
  debug_assert!(pawn_poses.iter().all(|idx| { idx.x() > 0 }));
  debug_assert!(pawn_poses.iter().all(|idx| { idx.x() <= width }));
  debug_assert!(pawn_poses.iter().all(|idx| { idx.y() > 0 }));
  debug_assert!(
    pawn_poses
      .iter()
      .all(|idx| { idx.x() + 1 + (idx.y() + 1) * width < u64::BITS })
  );

  #[cfg(target_feature = "ssse3")]
  if N == 16 {
    return unsafe { packed_positions_to_board_bitvec_sse3(pawn_poses, lower_left, width) };
  }
  packed_positions_to_board_bitvec_slow(pawn_poses, lower_left, width)
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

#[cfg(all(target_feature = "avx512bw", target_feature = "avx512vl"))]
#[target_feature(enable = "avx512bw,avx512vl")]
fn mm_min_max_epu8_ignore_zero_avx512(vec: __m128i) -> (u8, u8) {
  let zeros = _mm_cmpeq_epi8(vec, _mm_setzero_si128());
  let min_vec = _mm_or_si128(vec, zeros);
  let max_vec = vec;
  (_mm_reduce_min_epu8(min_vec), _mm_reduce_max_epu8(max_vec))
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

#[target_feature(enable = "ssse3")]
pub fn mm_min_max_ignore_zero_epu8(vec: __m128i) -> (u8, u8) {
  #[cfg(all(target_feature = "avx512bw", target_feature = "avx512vl"))]
  return unsafe { mm_min_max_epu8_ignore_zero_avx512(vec) };

  let zeros = _mm_cmpeq_epi8(vec, _mm_setzero_si128());
  let min_vec = _mm_or_si128(vec, zeros);
  let max_vec = vec;

  unsafe {
    (
      horizontal_compress(min_vec, |v1, v2| _mm_min_epu8(v1, v2)),
      horizontal_compress(max_vec, |v1, v2| _mm_max_epu8(v1, v2)),
    )
  }
}

#[inline]
#[target_feature(enable = "ssse3")]
fn packed_positions_boinding_box_sse3(pawn_poses: &[PackedIdx]) -> (HexPos, HexPos) {
  debug_assert_eq!(pawn_poses.len(), 16);

  /// Selects the x-coordinates of every PackedIdx position.
  const SELECT_X_MASK: i64 = 0x0f0f_0f0f_0f0f_0f0f;

  let select_x = _mm_set_epi64x(SELECT_X_MASK, SELECT_X_MASK);
  let select_y = _mm_set_epi64x(!SELECT_X_MASK, !SELECT_X_MASK);

  let pawns = unsafe { _mm_loadu_si128(pawn_poses.as_ptr() as *const _) };

  let x_coords = _mm_and_si128(pawns, select_x);
  let y_coords = _mm_and_si128(pawns, select_y);

  let (min_x, max_x) = mm_min_max_ignore_zero_epu8(x_coords);
  let (min_y, max_y) = mm_min_max_ignore_zero_epu8(y_coords);

  (
    HexPos::new(min_x as u32, (min_y >> 4) as u32),
    HexPos::new(max_x as u32, (max_y >> 4) as u32),
  )
}

#[inline]
fn packed_positions_bounding_box_slow<const N: usize>(
  pawn_poses: &[PackedIdx; N],
) -> (HexPos, HexPos) {
  debug_assert!(!pawn_poses.is_empty(), "Pawn positions cannot be empty");

  let (min_pos, max_pos) = pawn_poses
    .iter()
    .filter(|&&pos| pos != PackedIdx::null())
    .fold(
      ((u32::MAX, u32::MAX), (0, 0)),
      |(min_pos, max_pos), &pos| {
        (
          (min_pos.0.min(pos.x()), min_pos.1.min(pos.y())),
          (max_pos.0.max(pos.x()), max_pos.1.max(pos.y())),
        )
      },
    );

  (
    HexPos::new(min_pos.0, min_pos.1),
    HexPos::new(max_pos.0, max_pos.1),
  )
}

#[inline(always)]
pub fn packed_positions_bounding_box<const N: usize>(
  pawn_poses: &[PackedIdx; N],
) -> (HexPos, HexPos) {
  #[cfg(target_feature = "ssse3")]
  if N == 16 {
    return unsafe { packed_positions_boinding_box_sse3(pawn_poses) };
  }
  packed_positions_bounding_box_slow(pawn_poses)
}

#[cfg(test)]
mod tests {
  use onoro::hex_pos::HexPos;
  use rstest::rstest;
  use rstest_reuse::{apply, template};

  use crate::{
    PackedIdx,
    util::{
      broadcast_u8_to_u64, equal_mask_epi8, equal_mask_epi8_slow, packed_positions_bounding_box,
      packed_positions_bounding_box_slow, packed_positions_to_board_bitvec,
      packed_positions_to_board_bitvec_slow, packed_positions_to_mask,
      packed_positions_to_mask_slow,
    },
  };

  #[repr(align(8))]
  struct PawnPoses([PackedIdx; 16]);

  #[template]
  #[rstest]
  fn broadcast_u8_to_u64(
    #[values(
      (0x12, 0x12_12_12_12_12_12_12_12),
      (0x00, 0x00_00_00_00_00_00_00_00),
      (0xff, 0xff_ff_ff_ff_ff_ff_ff_ff),
    )]
    args: (u8, u64),
  ) {
  }

  #[apply(broadcast_u8_to_u64)]
  #[test]
  fn test_broadcast_u8_to_u64(args: (u8, u64)) {
    let (input, expected) = args;
    assert_eq!(broadcast_u8_to_u64(input), expected);
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

  const POSES_BOARD_BITVEC_EXAMPLE: PawnPoses = PawnPoses([
    PackedIdx::new(4, 5),
    PackedIdx::new(6, 4),
    PackedIdx::new(5, 3),
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
  fn packed_positions_to_board_bitvec<const N: usize>(
    #[values(
      packed_positions_to_board_bitvec,
      packed_positions_to_board_bitvec_slow
    )]
    packed_positions_to_board_bitvec: impl FnOnce(&[PackedIdx; N], PackedIdx, u32) -> u64,
    #[values(
      (&POSES_BOARD_BITVEC_EXAMPLE.0, PackedIdx::new(3, 2), 5, 0x01_20_80),
    )]
    args: (&[PackedIdx; N], PackedIdx, u32, u64),
  ) {
  }

  #[apply(packed_positions_to_board_bitvec)]
  #[test]
  fn test_packed_positions_to_board_bitvec<const N: usize>(
    packed_positions_to_board_bitvec: impl FnOnce(&[PackedIdx; N], PackedIdx, u32) -> u64,
    args: (&[PackedIdx; N], PackedIdx, u32, u64),
  ) {
    let (pawn_poses, lower_left, width, expected) = args;
    assert_eq!(
      packed_positions_to_board_bitvec(pawn_poses, lower_left, width),
      expected
    );
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
    #[values(packed_positions_bounding_box, packed_positions_bounding_box_slow)]
    packed_positions: impl FnOnce(&[PackedIdx; N]) -> (HexPos, HexPos),
    #[values(
      (&[PackedIdx::new(1, 1)], (HexPos::new(1, 1), HexPos::new(1, 1))),
      (&POSES_BB_EXAMPLE.0, (HexPos::new(1, 1), HexPos::new(3, 5))),
    )]
    args: (&[PackedIdx; N], (HexPos, HexPos)),
  ) {
  }

  #[apply(packed_positions_bounding_box)]
  #[test]
  fn test_packed_positions_bounding_box<const N: usize>(
    packed_positions: impl FnOnce(&[PackedIdx; N]) -> (HexPos, HexPos),
    args: (&[PackedIdx; N], (HexPos, HexPos)),
  ) {
    let (input, expected) = args;
    assert_eq!(packed_positions(input), expected);
  }
}
