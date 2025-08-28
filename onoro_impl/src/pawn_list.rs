use std::arch::x86_64::*;

#[cfg(not(target_feature = "ssse3"))]
use onoro::hex_pos::HexPosOffset;
use onoro::{groups::D6, hex_pos::HexPos};

use crate::{PackedIdx, util::unreachable};

const N: usize = 16;

pub struct PawnList8 {
  /// Stores 8 pawns, with x- and y- coordinates in back-to-back epi8 channels.
  #[cfg(target_feature = "ssse3")]
  pawns: __m128i,
  #[cfg(not(target_feature = "ssse3"))]
  pawns: [HexPosOffset; 8],
}

#[cfg(target_feature = "ssse3")]
impl PawnList8 {
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

  #[target_feature(enable = "ssse3")]
  fn extract_white_pawns_sse(pawn_poses: &[PackedIdx; N], origin: HexPos) -> Self {
    let pawns = unsafe { _mm_loadu_si128(pawn_poses.as_ptr() as *const _) };

    let white_x_coords_mask = _mm_set1_epi16(0x0f_00);
    let x_coords = _mm_and_si128(pawns, white_x_coords_mask);
    let x_coords = _mm_srli_epi16::<8>(x_coords);

    let white_y_coords_mask = _mm_set1_epi16(0xf0_00u16 as i16);
    let y_coords = _mm_and_si128(pawns, white_y_coords_mask);
    let y_coords = _mm_srli_epi16::<4>(y_coords);

    let pawns = _mm_or_si128(x_coords, y_coords);

    let centered_pawns = Self::centered_by(pawns, origin);

    Self {
      pawns: centered_pawns,
    }
  }

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

  pub fn extract_black_pawns(pawn_poses: &[PackedIdx; N], origin: HexPos) -> Self {
    unsafe { Self::extract_black_pawns_sse(pawn_poses, origin) }
  }

  pub fn extract_white_pawns(pawn_poses: &[PackedIdx; N], origin: HexPos) -> Self {
    unsafe { Self::extract_white_pawns_sse(pawn_poses, origin) }
  }

  #[target_feature(enable = "ssse3")]
  fn only_x(pawns: __m128i) -> __m128i {
    let x_mask = _mm_set1_epi16(0x00ff);
    _mm_and_si128(pawns, x_mask)
  }

  #[target_feature(enable = "ssse3")]
  fn only_y(pawns: __m128i) -> __m128i {
    let y_mask = _mm_set1_epi16(0xff00);
    _mm_and_si128(pawns, y_mask)
  }

  #[target_feature(enable = "ssse3")]
  fn negate_x(pawns: __m128i) -> __m128i {
    let x_mask = _mm_set1_epi16(0x00ff);
    let add_x_mask = _mm_set1_epi16(0x0001);
    _mm_add_epi8(_mm_xor_si128(pawns, x_mask), add_x_mask)
  }

  #[target_feature(enable = "ssse3")]
  fn negate_y(pawns: __m128i) -> __m128i {
    let y_mask = _mm_set1_epi16(0xff00);
    let add_y_mask = _mm_set1_epi16(0x0100);
    _mm_add_epi8(_mm_xor_si128(pawns, y_mask), add_y_mask)
  }

  #[target_feature(enable = "ssse3")]
  fn negate_xy(pawns: __m128i) -> __m128i {
    _mm_sub_epi8(_mm_setzero_si128(), pawns)
  }

  #[target_feature(enable = "ssse3")]
  fn swap_xy(pawns: __m128i) -> __m128i {
    let shuffle_indexes = _mm_set_epi8(14, 15, 12, 13, 10, 11, 8, 9, 6, 7, 4, 5, 2, 3, 0, 1);
    _mm_shuffle_epi8(pawns, shuffle_indexes)
  }

  #[target_feature(enable = "ssse3")]
  fn c_r1(&self) -> Self {
    let pawns = self.pawns;
    // (y, x)
    let swapped = Self::swap_xy(pawns);
    // (-y, x)
    let negated = Self::negate_x(swapped);
    // (x - y, x)
    let rotated = _mm_add_epi8(Self::only_x(pawns), negated);
    Self { pawns: rotated }
  }

  #[target_feature(enable = "ssse3")]
  fn c_r2(&self) -> Self {
    let pawns = self.pawns;
    // (y, x)
    let swapped = Self::swap_xy(pawns);
    // (-y, x)
    let negated = Self::negate_x(swapped);
    // (-y, x - y)
    let rotated = _mm_sub_epi8(negated, Self::only_y(pawns));
    Self { pawns: rotated }
  }

  #[target_feature(enable = "ssse3")]
  fn c_r3(&self) -> Self {
    Self {
      pawns: Self::negate_xy(self.pawns),
    }
  }

  #[target_feature(enable = "ssse3")]
  fn c_r4(&self) -> Self {
    let pawns = self.pawns;
    // (y, x)
    let swapped = Self::swap_xy(pawns);
    // (y, -x)
    let negated = Self::negate_y(swapped);
    // (y - x, -x)
    let rotated = _mm_sub_epi8(negated, Self::only_x(pawns));
    Self { pawns: rotated }
  }

  #[target_feature(enable = "ssse3")]
  fn c_r5(&self) -> Self {
    let pawns = self.pawns;
    // (y, x)
    let swapped = Self::swap_xy(pawns);
    // (y, -x)
    let negated = Self::negate_y(swapped);
    // (y, y - x)
    let rotated = _mm_add_epi8(Self::only_y(pawns), negated);
    Self { pawns: rotated }
  }

  #[target_feature(enable = "ssse3")]
  pub fn apply_d6_c(&self, op: &D6) -> Self {
    match op {
      D6::Rot(0) => *self,
      D6::Rot(1) => self.c_r1(),
      D6::Rot(2) => self.c_r2(),
      D6::Rot(3) => self.c_r3(),
      D6::Rot(4) => self.c_r4(),
      D6::Rot(5) => self.c_r5(),
      D6::Rfl(0) => self.c_s0(),
      D6::Rfl(1) => self.c_s1(),
      D6::Rfl(2) => self.c_s2(),
      D6::Rfl(3) => self.c_s3(),
      D6::Rfl(4) => self.c_s4(),
      D6::Rfl(5) => self.c_s5(),
      _ => unreachable(),
    }
  }
}

#[cfg(not(target_feature = "ssse3"))]
impl PawnList8 {
  pub fn extract_black_pawns(pawn_poses: &[PackedIdx; N], origin: HexPos) -> Self {
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

  pub fn extract_white_pawns(pawn_poses: &[PackedIdx; N], origin: HexPos) -> Self {
    let pawns = [
      HexPos::from(pawn_poses[1]) - origin,
      HexPos::from(pawn_poses[3]) - origin,
      HexPos::from(pawn_poses[5]) - origin,
      HexPos::from(pawn_poses[7]) - origin,
      HexPos::from(pawn_poses[9]) - origin,
      HexPos::from(pawn_poses[11]) - origin,
      HexPos::from(pawn_poses[13]) - origin,
      HexPos::from(pawn_poses[15]) - origin,
    ];
    Self { pawns }
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
}
