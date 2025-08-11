use core::arch::x86_64::{__m128i, _mm_set_epi8, _mm_shuffle_epi8};
use std::arch::x86_64::{_mm_cvtsi128_si64x, _mm_set_epi64x};

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

/// Given a `u8`, returns a `u64` with each byte of the `u64` equal to the
/// passed `u8`.
#[inline]
pub const fn broadcast_u8_to_u64(v: u8) -> u64 {
  const BYTE_ANCHOR: u64 = 0x0101_0101_0101_0101;
  (v as u64) * BYTE_ANCHOR
}

/// Given 8 byte values packed into a u64, returns a u64 with each
/// corresponding bit index set for each byte (1-indexed). Zero byte values are
/// ignored. All non-zero bytes must be unique.
#[target_feature(enable = "ssse3")]
unsafe fn packed_positions_to_mask_sse3(packed_positions: u64) -> u64 {
  debug_assert!(
    packed_positions
      .to_ne_bytes()
      .into_iter()
      .all(|byte| byte < 0x10)
  );
  debug_assert!(
    packed_positions
      .to_ne_bytes()
      .into_iter()
      .filter(|&byte| byte != 0)
      .all_unique()
  );

  let lo_data = _mm_set_epi8(
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0x80u8 as i8,
    0x40,
    0x20,
    0x10,
    0x08,
    0x04,
    0x02,
    0x01,
    0,
  );
  let hi_data = _mm_set_epi8(
    0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  );

  let shuffle_mask = _mm_set_epi64x(0, packed_positions as i64);
  let lo_masks = _mm_shuffle_epi8(lo_data, shuffle_mask);
  let hi_masks = _mm_shuffle_epi8(hi_data, shuffle_mask);

  let lo_masks = _mm_cvtsi128_si64x(lo_masks) as u64;
  let hi_masks = _mm_cvtsi128_si64x(hi_masks) as u64;

  let lo_masks = (lo_masks | (lo_masks >> 8)) & 0x00ff_00ff_00ff_00ff;
  let hi_masks = (hi_masks | (hi_masks << 8)) & 0xff00_ff00_ff00_ff00;
  let masks = lo_masks | hi_masks;

  let masks = masks | (masks >> 16);
  let masks = masks | (masks >> 32);
  masks & 0x0000_0000_0000_ffff
}

#[cfg(any(test, not(target_feature = "ssse3")))]
fn packed_positions_to_mask_slow(packed_positions: u64) -> u64 {
  packed_positions
    .to_ne_bytes()
    .into_iter()
    .filter(|&byte| byte != 0)
    .fold(0, |mask, byte| mask | (1u64 << (byte - 1)))
}

pub fn packed_positions_to_mask(packed_positions: u64) -> u64 {
  #[cfg(target_feature = "ssse3")]
  unsafe {
    packed_positions_to_mask_sse3(packed_positions)
  }
  #[cfg(not(target_feature = "ssse3"))]
  packed_positions_to_mask_slow(packed_positions)
}

#[cfg(test)]
mod tests {
  use crate::util::{packed_positions_to_mask, packed_positions_to_mask_slow};

  #[test]
  fn test_packed_positions_to_mask() {
    assert_eq!(packed_positions_to_mask(0x0102030405060708), 0xff);
    assert_eq!(packed_positions_to_mask(0x01040506070a0c0e), 0x2a79);
  }

  #[test]
  fn test_packed_positions_to_mask_slow() {
    assert_eq!(packed_positions_to_mask_slow(0x0102030405060708), 0xff);
    assert_eq!(packed_positions_to_mask_slow(0x01040506070a0c0e), 0x2a79);
  }
}
