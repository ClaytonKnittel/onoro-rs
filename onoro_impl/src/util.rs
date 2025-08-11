use core::arch::x86_64::{_mm_set_epi8, _mm_shuffle_epi8};
use std::arch::x86_64::{
  _mm_cmpeq_epi8, _mm_cvtsi128_si64x, _mm_set_epi64x, _mm_unpackhi_epi64, _mm_unpacklo_epi8,
};

use itertools::Itertools;

#[inline]
pub const fn unreachable() -> ! {
  #[cfg(debug_assertions)]
  unreachable!();
  #[cfg(not(debug_assertions))]
  unsafe {
    std::hint::unreachable_unchecked()
  }
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

#[inline]
#[target_feature(enable = "ssse3")]
unsafe fn broadcast_u8_to_u64_sse3(v: u8) -> u64 {
  let v = v as i8;
  let mask = _mm_set_epi8(0, 0, 0, 0, 0, 0, 0, 0, v, v, v, v, v, v, v, v);
  _mm_cvtsi128_si64x(mask) as u64
}

#[cfg(any(test, not(target_feature = "ssse3")))]
#[inline]
pub const fn broadcast_u8_to_u64_slow(v: u8) -> u64 {
  const BYTE_ANCHOR: u64 = 0x0101_0101_0101_0101;
  (v as u64) * BYTE_ANCHOR
}

/// Given a `u8`, returns a `u64` with each byte of the `u64` equal to the
/// passed `u8`.
#[inline(always)]
pub fn broadcast_u8_to_u64(v: u8) -> u64 {
  #[cfg(target_feature = "ssse3")]
  unsafe {
    broadcast_u8_to_u64_sse3(v)
  }
  #[cfg(not(target_feature = "ssse3"))]
  broadcast_u8_to_u64_slow(v)
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

  let shuffle_mask = _mm_set_epi64x(0, packed_positions as i64);
  let lo_masks = _mm_shuffle_epi8(lo_data, shuffle_mask);
  let hi_masks = _mm_shuffle_epi8(hi_data, shuffle_mask);

  let masks = _mm_unpacklo_epi8(lo_masks, hi_masks);

  let lo_masks = _mm_cvtsi128_si64x(masks) as u64;
  let masks = _mm_unpackhi_epi64(masks, masks);
  let hi_masks = _mm_cvtsi128_si64x(masks) as u64;
  let masks = lo_masks + hi_masks;

  let masks = masks + (masks >> 16);
  let masks = masks + (masks >> 32);
  masks & 0x0000_0000_0000_ffff
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

/// Given 8 byte values packed into a u64, returns a u64 with each
/// corresponding bit index set for each byte (1-indexed). Zero byte values are
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
  let needle_mask = _mm_set_epi8(0, 0, 0, 0, 0, 0, 0, 0, b, b, b, b, b, b, b, b);
  let byte_vec = _mm_set_epi64x(byte_vec as i64, byte_vec as i64);

  let equal_mask = _mm_cmpeq_epi8(needle_mask, byte_vec);
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

/// bitmask of 0xff in byte if that byte = needle in byte_vec, 0 otherwise
#[inline(always)]
pub fn equal_mask_epi8(byte_vec: u64, needle: u8) -> u64 {
  #[cfg(target_feature = "ssse3")]
  unsafe {
    equal_mask_epi8_sse3(byte_vec, needle)
  }
  #[cfg(not(target_feature = "ssse3"))]
  equal_mask_epi8_slow(byte_vec, needle)
}

#[cfg(test)]
mod tests {
  use rstest::rstest;
  use rstest_reuse::{apply, template};

  use crate::util::{
    broadcast_u8_to_u64, broadcast_u8_to_u64_slow, equal_mask_epi8, equal_mask_epi8_slow,
    packed_positions_to_mask, packed_positions_to_mask_slow,
  };

  #[template]
  #[rstest]
  fn broadcast_u8_to_u64(
    #[values(broadcast_u8_to_u64, broadcast_u8_to_u64_slow)] broadcast: impl FnOnce(u8) -> u64,
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
  fn test_broadcast_u8_to_u64(broadcast: impl FnOnce(u8) -> u64, args: (u8, u64)) {
    let (input, expected) = args;
    assert_eq!(broadcast(input), expected);
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
}
