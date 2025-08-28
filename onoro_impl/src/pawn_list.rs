use std::arch::x86_64::*;

use onoro::hex_pos::HexPos;
#[cfg(not(target_feature = "ssse3"))]
use onoro::hex_pos::HexPosOffset;

use crate::{PackedIdx, util::unreachable};

const N: usize = 16;

pub struct PawnList8 {
  /// Stores 8 pawns, with x- and y- coordinates in back-to-back epi8 channels.
  #[cfg(target_feature = "ssse3")]
  pawns: __m128i,
  #[cfg(not(target_feature = "ssse3"))]
  pawns: [HexPosOffset; 8],
}

impl PawnList8 {
  #[cfg(target_feature = "ssse3")]
  #[target_feature(enable = "ssse3")]
  fn extract_black_pawns_sse(pawn_poses: &[PackedIdx; N], origin: HexPos) -> Self {
    let pawns = unsafe { _mm_loadu_si128(pawn_poses.as_ptr() as *const _) };

    let black_x_coords_mask = _mm_set1_epi16(0x00_0f);
    let x_coords = _mm_and_si128(pawns, black_x_coords_mask);

    let black_y_coords_mask = _mm_set1_epi16(0x00_f0);
    let y_coords = _mm_and_si128(pawns, black_y_coords_mask);
    let y_coords = _mm_slli_epi16::<4>(y_coords);

    let pawns = _mm_or_si128(x_coords, y_coords);

    let centered_pawns = Self::centered_by(pawns, origin);

    Self {
      pawns: centered_pawns,
    }
  }

  #[cfg(target_feature = "ssse3")]
  #[target_feature(enable = "ssse3")]
  fn centered_by(pawns: __m128i, origin: HexPos) -> __m128i {
    let x = origin.x();
    let y = origin.y();
    if x > u8::MAX as u32 || y > u8::MAX as u32 {
      unreachable();
    }
    let origin_array = _mm_set1_epi16((x | (y << 8)) as i16);

    _mm_sub_epi8(pawns, origin_array)
  }

  #[cfg(not(target_feature = "ssse3"))]
  fn extract_black_pawns_slow(pawn_poses: &[PackedIdx; N], origin: HexPos) -> Self {
    let pawns = [
      HexPos::from(pawn_poses[0]) - origin,
      HexPos::from(pawn_poses[2]) - origin,
      HexPos::from(pawn_poses[4]) - origin,
      HexPos::from(pawn_poses[6]) - origin,
      HexPos::from(pawn_poses[8]) - origin,
      HexPos::from(pawn_poses[10]) - origin,
      HexPos::from(pawn_poses[12]) - origin,
      HexPos::from(pawn_poses[14]) - origin,
    ];
    Self { pawns }
  }

  pub fn extract_black_pawns(pawn_poses: &[PackedIdx; N], origin: HexPos) -> Self {
    #[cfg(target_feature = "ssse3")]
    unsafe {
      Self::extract_black_pawns_sse(pawn_poses, origin)
    }
    #[cfg(not(target_feature = "ssse3"))]
    Self::extract_black_pawns_slow(pawn_poses, origin)
  }
}

#[cfg(test)]
mod tests {
  use std::arch::x86_64::{_mm_bsrli_si128, _mm_cvtsi128_si64x};

  use googletest::{gtest, prelude::*};
  use onoro::hex_pos::{HexPos, HexPosOffset};

  use crate::{PackedIdx, pawn_list::PawnList8};

  #[cfg(target_feature = "ssse3")]
  #[target_feature(enable = "ssse3")]
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

  #[cfg(not(target_feature = "ssse3"))]
  fn pos_at_slow(pawn_list: &PawnList8, idx: usize) -> HexPosOffset {
    pawn_list.pawns[idx]
  }

  fn pos_at(pawn_list: &PawnList8, idx: usize) -> HexPosOffset {
    #[cfg(target_feature = "ssse3")]
    unsafe {
      pos_at_sse(pawn_list, idx)
    }
    #[cfg(not(target_feature = "ssse3"))]
    pos_at_slow(pawn_list, idx)
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

    let pawn_list = PawnList8::extract_black_pawns(&pawns, HexPos::new(4, 10));

    expect_that!(
      positions(&pawn_list),
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
  }
}
