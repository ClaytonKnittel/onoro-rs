#[cfg(target_feature = "sse4.1")]
use std::arch::x86_64::*;

#[cfg(target_feature = "sse4.1")]
use algebra::group::Cyclic;
use algebra::group::Trivial;
#[cfg(not(target_feature = "sse4.1"))]
use itertools::Itertools;
#[cfg(not(target_feature = "sse4.1"))]
use onoro::hex_pos::HexPosOffset;
use onoro::{
  groups::{C2, D3, D6, K4},
  hex_pos::HexPos,
};

use crate::{PackedIdx, util::unreachable};

const N: usize = 16;

#[cfg(target_feature = "sse4.1")]
#[repr(align(16))]
struct MM128Contents([i8; 16]);

#[cfg(target_feature = "sse4.1")]
impl MM128Contents {
  #[target_feature(enable = "sse4.1")]
  fn load(&self) -> __m128i {
    unsafe { _mm_load_si128(self.0.as_ptr() as *const _) }
  }

  const fn noop_mask() -> MM128Contents {
    MM128Contents([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15])
  }

  const fn zero_mask() -> MM128Contents {
    MM128Contents([-1; 16])
  }

  const fn swap_xy_mask() -> MM128Contents {
    MM128Contents([1, 0, 3, 2, 5, 4, 7, 6, 9, 8, 11, 10, 13, 12, 15, 14])
  }

  const fn isolate_x_mask() -> MM128Contents {
    MM128Contents([0, -1, 2, -1, 4, -1, 6, -1, 8, -1, 10, -1, 12, -1, 14, -1])
  }

  const fn isolate_y_mask() -> MM128Contents {
    MM128Contents([-1, 1, -1, 3, -1, 5, -1, 7, -1, 9, -1, 11, -1, 13, -1, 15])
  }

  const fn duplicate_x_mask() -> MM128Contents {
    MM128Contents([0, 0, 2, 2, 4, 4, 6, 6, 8, 8, 10, 10, 12, 12, 14, 14])
  }

  const fn duplicate_y_mask() -> MM128Contents {
    MM128Contents([1, 1, 3, 3, 5, 5, 7, 7, 9, 9, 11, 11, 13, 13, 15, 15])
  }

  const fn move_x_to_y_mask() -> MM128Contents {
    MM128Contents([-1, 0, -1, 2, -1, 4, -1, 6, -1, 8, -1, 10, -1, 12, -1, 14])
  }

  const fn move_y_to_x_mask() -> MM128Contents {
    MM128Contents([1, -1, 3, -1, 5, -1, 7, -1, 9, -1, 11, -1, 13, -1, 15, -1])
  }
}

#[cfg(target_feature = "sse4.1")]
#[derive(Clone, Copy)]
pub struct PawnList8 {
  /// Stores 8 pawns, with x- and y- coordinates in back-to-back epi8 channels.
  pawns: __m128i,
  zero_poses: __m128i,
}

#[cfg(target_feature = "sse4.1")]
impl PawnList8 {
  #[target_feature(enable = "sse4.1")]
  fn extract_black_pawns_sse(pawn_poses: &[PackedIdx; N], origin: HexPos) -> Self {
    let pawns = unsafe { _mm_loadu_si128(pawn_poses.as_ptr() as *const _) };

    let black_x_coords_mask = _mm_set1_epi16(0x00_0f);
    let x_coords = _mm_and_si128(pawns, black_x_coords_mask);

    let black_y_coords_mask = _mm_set1_epi16(0x00_f0);
    let y_coords = _mm_and_si128(pawns, black_y_coords_mask);
    let y_coords = _mm_slli_epi16::<4>(y_coords);

    let pawns = _mm_or_si128(x_coords, y_coords);
    let zero_poses = _mm_cmpeq_epi16(pawns, _mm_setzero_si128());

    let centered_pawns = Self::centered_by(pawns, origin);

    Self {
      pawns: centered_pawns,
      zero_poses,
    }
  }

  #[target_feature(enable = "sse4.1")]
  fn extract_white_pawns_sse(pawn_poses: &[PackedIdx; N], origin: HexPos) -> Self {
    let pawns = unsafe { _mm_loadu_si128(pawn_poses.as_ptr() as *const _) };

    let white_x_coords_mask = _mm_set1_epi16(0x0f_00);
    let x_coords = _mm_and_si128(pawns, white_x_coords_mask);
    let x_coords = _mm_srli_epi16::<8>(x_coords);

    let white_y_coords_mask = _mm_set1_epi16(0xf0_00u16 as i16);
    let y_coords = _mm_and_si128(pawns, white_y_coords_mask);
    let y_coords = _mm_srli_epi16::<4>(y_coords);

    let pawns = _mm_or_si128(x_coords, y_coords);
    let zero_poses = _mm_cmpeq_epi16(pawns, _mm_setzero_si128());

    let centered_pawns = Self::centered_by(pawns, origin);

    Self {
      pawns: centered_pawns,
      zero_poses,
    }
  }

  #[target_feature(enable = "sse4.1")]
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

  #[target_feature(enable = "sse4.1")]
  fn xy_ones() -> __m128i {
    _mm_set1_epi8(0x01)
  }

  #[target_feature(enable = "sse4.1")]
  fn x_ones() -> __m128i {
    _mm_set1_epi16(0x0001)
  }

  #[target_feature(enable = "sse4.1")]
  fn negate_xy(pawns: __m128i) -> __m128i {
    _mm_sub_epi8(_mm_setzero_si128(), pawns)
  }

  #[target_feature(enable = "sse4.1")]
  fn swap_xy(pawns: __m128i) -> __m128i {
    let shuffle_indexes = _mm_set_epi8(14, 15, 12, 13, 10, 11, 8, 9, 6, 7, 4, 5, 2, 3, 0, 1);
    _mm_shuffle_epi8(pawns, shuffle_indexes)
  }

  #[target_feature(enable = "sse4.1")]
  fn isolate_x(pawns: __m128i) -> __m128i {
    let mask = _mm_set1_epi16(0x00ff);
    _mm_and_si128(pawns, mask)
  }

  #[target_feature(enable = "sse4.1")]
  fn isolate_y(pawns: __m128i) -> __m128i {
    let mask = _mm_set1_epi16(0xff00u16 as i16);
    _mm_and_si128(pawns, mask)
  }

  #[target_feature(enable = "sse4.1")]
  fn duplicate_x(pawns: __m128i) -> __m128i {
    let shuffle_indexes = _mm_set_epi8(14, 14, 12, 12, 10, 10, 8, 8, 6, 6, 4, 4, 2, 2, 0, 0);
    _mm_shuffle_epi8(pawns, shuffle_indexes)
  }

  #[target_feature(enable = "sse4.1")]
  fn duplicate_y(pawns: __m128i) -> __m128i {
    let shuffle_indexes = _mm_set_epi8(15, 15, 13, 13, 11, 11, 9, 9, 7, 7, 5, 5, 3, 3, 1, 1);
    _mm_shuffle_epi8(pawns, shuffle_indexes)
  }

  #[target_feature(enable = "sse4.1")]
  fn move_x_to_y(pawns: __m128i) -> __m128i {
    let shuffle_indexes = _mm_set_epi8(14, -1, 12, -1, 10, -1, 8, -1, 6, -1, 4, -1, 2, -1, 0, -1);
    _mm_shuffle_epi8(pawns, shuffle_indexes)
  }

  #[target_feature(enable = "sse4.1")]
  fn move_y_to_x(pawns: __m128i) -> __m128i {
    let shuffle_indexes = _mm_set_epi8(-1, 15, -1, 13, -1, 11, -1, 9, -1, 7, -1, 5, -1, 3, -1, 1);
    _mm_shuffle_epi8(pawns, shuffle_indexes)
  }

  #[target_feature(enable = "sse4.1")]
  fn c_r1(&self) -> Self {
    let pawns = self.pawns;
    // (x, x)
    let xx = Self::duplicate_x(pawns);
    // (y, 0)
    let yz = Self::move_y_to_x(pawns);
    // (x - y, x)
    let rotated = _mm_sub_epi8(xx, yz);
    Self {
      pawns: rotated,
      ..*self
    }
  }

  #[target_feature(enable = "sse4.1")]
  fn c_r2(&self) -> Self {
    let pawns = self.pawns;
    // (y, y)
    let yy = Self::duplicate_y(pawns);
    // (0, x)
    let zx = Self::move_x_to_y(pawns);
    // (-y, x - y)
    let rotated = _mm_sub_epi8(zx, yy);
    Self {
      pawns: rotated,
      ..*self
    }
  }

  #[target_feature(enable = "sse4.1")]
  fn c_r3(&self) -> Self {
    Self {
      pawns: Self::negate_xy(self.pawns),
      ..*self
    }
  }

  #[target_feature(enable = "sse4.1")]
  fn c_r4(&self) -> Self {
    let pawns = self.pawns;
    // (x, x)
    let xx = Self::duplicate_x(pawns);
    // (y, 0)
    let yz = Self::move_y_to_x(pawns);
    // (y - x, -x)
    let rotated = _mm_sub_epi8(yz, xx);
    Self {
      pawns: rotated,
      ..*self
    }
  }

  #[target_feature(enable = "sse4.1")]
  fn c_r5(&self) -> Self {
    let pawns = self.pawns;
    // (y, y)
    let yy = Self::duplicate_y(pawns);
    // (0, x)
    let zx = Self::move_x_to_y(pawns);
    // (y, y - x)
    let rotated = _mm_sub_epi8(yy, zx);
    Self {
      pawns: rotated,
      ..*self
    }
  }

  #[target_feature(enable = "sse4.1")]
  fn c_s0(&self) -> Self {
    let pawns = self.pawns;
    // (y, y)
    let yy = Self::duplicate_y(pawns);
    // (x, 0)
    let xz = Self::isolate_x(pawns);
    // (x - y, -y)
    let rotated = _mm_sub_epi8(xz, yy);
    Self {
      pawns: rotated,
      ..*self
    }
  }

  #[target_feature(enable = "sse4.1")]
  fn c_s1(&self) -> Self {
    let pawns = self.pawns;
    // (x, x)
    let xx = Self::duplicate_x(pawns);
    // (0, y)
    let zy = Self::isolate_y(pawns);
    // (x, x - y)
    let rotated = _mm_sub_epi8(xx, zy);
    Self {
      pawns: rotated,
      ..*self
    }
  }

  #[target_feature(enable = "sse4.1")]
  fn c_s2(&self) -> Self {
    Self {
      pawns: Self::swap_xy(self.pawns),
      ..*self
    }
  }

  #[target_feature(enable = "sse4.1")]
  fn c_s3(&self) -> Self {
    let pawns = self.pawns;
    // (y, y)
    let yy = Self::duplicate_y(pawns);
    // (x, 0)
    let xz = Self::isolate_x(pawns);
    // (y - x, y)
    let rotated = _mm_sub_epi8(yy, xz);
    Self {
      pawns: rotated,
      ..*self
    }
  }

  #[target_feature(enable = "sse4.1")]
  fn c_s4(&self) -> Self {
    let pawns = self.pawns;
    // (x, x)
    let xx = Self::duplicate_x(pawns);
    // (0, y)
    let zy = Self::isolate_y(pawns);
    // (-x, y - x)
    let rotated = _mm_sub_epi8(zy, xx);
    Self {
      pawns: rotated,
      ..*self
    }
  }

  #[target_feature(enable = "sse4.1")]
  fn c_s5(&self) -> Self {
    let pawns = self.pawns;
    // (y, x)
    let yx = Self::swap_xy(pawns);
    // (-y, -x)
    let rotated = Self::negate_xy(yx);
    Self {
      pawns: rotated,
      ..*self
    }
  }

  #[target_feature(enable = "sse4.1")]
  fn apply_d6_c_sse(&self, op: &D6) -> Self {
    use algebra::ordinal::Ordinal;

    const POSITIVE_MASKS: [MM128Contents; 12] = [
      MM128Contents::noop_mask(),
      MM128Contents::duplicate_x_mask(),
      MM128Contents::move_x_to_y_mask(),
      MM128Contents::zero_mask(),
      MM128Contents::move_y_to_x_mask(),
      MM128Contents::duplicate_y_mask(),
      MM128Contents::isolate_x_mask(),
      MM128Contents::duplicate_x_mask(),
      MM128Contents::swap_xy_mask(),
      MM128Contents::duplicate_y_mask(),
      MM128Contents::isolate_y_mask(),
      MM128Contents::zero_mask(),
    ];
    const NEGATIVE_MASKS: [MM128Contents; 12] = [
      MM128Contents::zero_mask(),
      MM128Contents::move_y_to_x_mask(),
      MM128Contents::duplicate_y_mask(),
      MM128Contents::noop_mask(),
      MM128Contents::duplicate_x_mask(),
      MM128Contents::move_x_to_y_mask(),
      MM128Contents::duplicate_y_mask(),
      MM128Contents::isolate_y_mask(),
      MM128Contents::zero_mask(),
      MM128Contents::isolate_x_mask(),
      MM128Contents::duplicate_x_mask(),
      MM128Contents::swap_xy_mask(),
    ];

    let positive_mask = POSITIVE_MASKS[op.ord()].load();
    let negative_mask = NEGATIVE_MASKS[op.ord()].load();
    let positive = _mm_shuffle_epi8(self.pawns, positive_mask);
    let negative = _mm_shuffle_epi8(self.pawns, negative_mask);
    Self {
      pawns: _mm_sub_epi8(positive, negative),
      ..*self
    }
  }

  pub fn apply_d6_c(&self, op: &D6) -> Self {
    unsafe { self.apply_d6_c_sse(op) }
  }

  #[target_feature(enable = "sse4.1")]
  fn v_r2(&self) -> Self {
    let pawns = self.pawns;
    // (y, y)
    let yy = Self::duplicate_y(pawns);
    // (0, x)
    let zx = Self::move_x_to_y(pawns);
    // (1 - y, x - y)
    let rotated = _mm_sub_epi8(_mm_add_epi8(zx, Self::x_ones()), yy);
    Self {
      pawns: rotated,
      ..*self
    }
  }

  #[target_feature(enable = "sse4.1")]
  fn v_r4(&self) -> Self {
    let pawns = self.pawns;
    // (x, x)
    let xx = Self::duplicate_x(pawns);
    // (y, 0)
    let yz = Self::move_y_to_x(pawns);
    // (y + 1 - x, 1 - x)
    let rotated = _mm_sub_epi8(_mm_add_epi8(yz, Self::xy_ones()), xx);
    Self {
      pawns: rotated,
      ..*self
    }
  }

  #[target_feature(enable = "sse4.1")]
  fn v_s1(&self) -> Self {
    let pawns = self.pawns;
    // (x, x)
    let xx = Self::duplicate_x(pawns);
    // (0, y)
    let zy = Self::isolate_y(pawns);
    // (x, x - y)
    let rotated = _mm_sub_epi8(xx, zy);
    Self {
      pawns: rotated,
      ..*self
    }
  }

  #[target_feature(enable = "sse4.1")]
  fn v_s3(&self) -> Self {
    let pawns = self.pawns;
    // (y, y)
    let yy = Self::duplicate_y(pawns);
    // (x, 0)
    let xz = Self::isolate_x(pawns);
    // (y + 1 - x, y)
    let rotated = _mm_sub_epi8(_mm_add_epi8(yy, Self::x_ones()), xz);
    Self {
      pawns: rotated,
      ..*self
    }
  }

  #[target_feature(enable = "sse4.1")]
  fn v_s5(&self) -> Self {
    let pawns = self.pawns;
    // (y, x)
    let yx = Self::swap_xy(pawns);
    // (1 - y, 1 - x)
    let rotated = _mm_sub_epi8(Self::xy_ones(), yx);
    Self {
      pawns: rotated,
      ..*self
    }
  }

  #[target_feature(enable = "sse4.1")]
  fn apply_d3_v_sse(&self, op: &D3) -> Self {
    match op {
      D3::Rot(0) => *self,
      D3::Rot(1) => self.v_r2(),
      D3::Rot(2) => self.v_r4(),
      D3::Rfl(0) => self.v_s1(),
      D3::Rfl(1) => self.v_s3(),
      D3::Rfl(2) => self.v_s5(),
      _ => unreachable(),
    }
  }

  pub fn apply_d3_v(&self, op: &D3) -> Self {
    unsafe { self.apply_d3_v_sse(op) }
  }

  #[target_feature(enable = "sse4.1")]
  fn e_s0(&self) -> Self {
    let pawns = self.pawns;
    // (y, y)
    let yy = Self::duplicate_y(pawns);
    // (x, 0)
    let xz = Self::isolate_x(pawns);
    // (x - y, -y)
    let rotated = _mm_sub_epi8(xz, yy);
    Self {
      pawns: rotated,
      ..*self
    }
  }

  #[target_feature(enable = "sse4.1")]
  fn e_s3(&self) -> Self {
    let pawns = self.pawns;
    // (y, y)
    let yy = Self::duplicate_y(pawns);
    // (x, 0)
    let xz = Self::isolate_x(pawns);
    // (y + 1 - x, y)
    let rotated = _mm_sub_epi8(_mm_add_epi8(yy, Self::x_ones()), xz);
    Self {
      pawns: rotated,
      ..*self
    }
  }

  #[target_feature(enable = "sse4.1")]
  fn e_r3(&self) -> Self {
    let pawns = self.pawns;
    // (1 - x, -y)
    let rotated = _mm_sub_epi8(Self::x_ones(), pawns);
    Self {
      pawns: rotated,
      ..*self
    }
  }

  #[target_feature(enable = "sse4.1")]
  fn apply_k4_e_sse(&self, op: &K4) -> Self {
    match (op.left(), op.right()) {
      (Cyclic::<2>(0), Cyclic::<2>(0)) => *self,
      (Cyclic::<2>(1), Cyclic::<2>(0)) => self.e_s0(),
      (Cyclic::<2>(0), Cyclic::<2>(1)) => self.e_s3(),
      (Cyclic::<2>(1), Cyclic::<2>(1)) => self.e_r3(),
      _ => unreachable(),
    }
  }

  pub fn apply_k4_e(&self, op: &K4) -> Self {
    unsafe { self.apply_k4_e_sse(op) }
  }

  #[target_feature(enable = "sse4.1")]
  fn apply_c2_cv_sse(&self, op: &C2) -> Self {
    match op {
      Cyclic::<2>(0) => *self,
      Cyclic::<2>(1) => self.c_s1(),
      _ => unreachable(),
    }
  }

  pub fn apply_c2_cv(&self, op: &C2) -> Self {
    unsafe { self.apply_c2_cv_sse(op) }
  }

  #[target_feature(enable = "sse4.1")]
  fn apply_c2_ce_sse(&self, op: &C2) -> Self {
    match op {
      Cyclic::<2>(0) => *self,
      Cyclic::<2>(1) => self.c_s0(),
      _ => unreachable(),
    }
  }

  pub fn apply_c2_ce(&self, op: &C2) -> Self {
    unsafe { self.apply_c2_ce_sse(op) }
  }

  #[target_feature(enable = "sse4.1")]
  fn apply_c2_ev_sse(&self, op: &C2) -> Self {
    match op {
      Cyclic::<2>(0) => *self,
      Cyclic::<2>(1) => self.e_s3(),
      _ => unreachable(),
    }
  }

  pub fn apply_c2_ev(&self, op: &C2) -> Self {
    unsafe { self.apply_c2_ev_sse(op) }
  }

  pub fn apply_trivial(&self, _op: &Trivial) -> Self {
    *self
  }

  #[target_feature(enable = "sse4.1")]
  fn masked_pawns(&self) -> __m128i {
    _mm_andnot_si128(self.zero_poses, self.pawns)
  }

  /// Returns true if the two pawn lists are equal, ignoring the order of the
  /// elements.
  #[target_feature(enable = "sse4.1")]
  fn equal_ignoring_order_sse(&self, other: PawnList8) -> bool {
    let pawns1 = self.masked_pawns();
    let pawns2 = other.masked_pawns();

    let lo_pawns1 = _mm_cvtsi128_si64x(pawns1) as u64;
    let pawns1 = _mm_unpackhi_epi64(pawns1, pawns1);
    let hi_pawns1 = _mm_cvtsi128_si64x(pawns1) as u64;

    let eq_poses = |needle: i16| {
      let search_mask = _mm_set1_epi16(needle);
      _mm_cmpeq_epi16(pawns2, search_mask)
    };

    let total = [
      lo_pawns1 as i16,
      (lo_pawns1 >> 16) as i16,
      (lo_pawns1 >> 32) as i16,
      (lo_pawns1 >> 48) as i16,
      hi_pawns1 as i16,
      (hi_pawns1 >> 16) as i16,
      (hi_pawns1 >> 32) as i16,
      (hi_pawns1 >> 48) as i16,
    ]
    .into_iter()
    .map(eq_poses)
    .reduce(|l, r| _mm_add_epi16(l, r));

    _mm_movemask_epi8(unsafe { total.unwrap_unchecked() }) == 0xffff
  }

  pub fn equal_ignoring_order(&self, other: PawnList8) -> bool {
    unsafe { self.equal_ignoring_order_sse(other) }
  }
}

#[cfg(not(target_feature = "sse4.1"))]
#[derive(Clone, Copy)]
pub struct PawnList8 {
  pawns: [HexPosOffset; 8],
}

#[cfg(not(target_feature = "sse4.1"))]
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

  pub fn apply_d6_c(&self, op: &D6) -> Self {
    Self {
      pawns: self.pawns.map(|pos| pos.apply_d6_c(op)),
    }
  }

  pub fn apply_d3_v(&self, op: &D3) -> Self {
    Self {
      pawns: self.pawns.map(|pos| pos.apply_d3_v(op)),
    }
  }

  pub fn apply_k4_e(&self, op: &K4) -> Self {
    Self {
      pawns: self.pawns.map(|pos| pos.apply_k4_e(op)),
    }
  }

  pub fn apply_c2_cv(&self, op: &C2) -> Self {
    Self {
      pawns: self.pawns.map(|pos| pos.apply_c2_cv(op)),
    }
  }

  pub fn apply_c2_ce(&self, op: &C2) -> Self {
    Self {
      pawns: self.pawns.map(|pos| pos.apply_c2_ce(op)),
    }
  }

  pub fn apply_c2_ev(&self, op: &C2) -> Self {
    Self {
      pawns: self.pawns.map(|pos| pos.apply_c2_ev(op)),
    }
  }

  pub fn apply_trivial(&self, op: &Trivial) -> Self {
    *self
  }

  /// Returns true if the two pawn lists are equal ignoring the order of the
  /// elements.
  pub fn equal_ignoring_order(&self, other: Self) -> bool {
    self.pawns.iter().all(|pos| other.pawns.contains(pos))
  }
}

#[cfg(test)]
mod tests {
  use std::arch::x86_64::{_mm_bsrli_si128, _mm_cvtsi128_si64x};

  use algebra::{group::Trivial, semigroup::Semigroup};
  use googletest::{gtest, prelude::*};
  use itertools::Itertools;
  use onoro::{
    groups::{C2, D3, D6, K4},
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
    pawn_list.pawns[idx]
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
    ($name:ident, $apply_op:ident, $op_t:ty) => {
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
            let rotated_black = black_pawns.$apply_op(&op);
            let rotated_white = white_pawns.$apply_op(&op);

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

  test_rotate!(test_rotate_d6_c, apply_d6_c, D6);
  test_rotate!(test_rotate_d3_v, apply_d3_v, D3);
  test_rotate!(test_rotate_k4_e, apply_k4_e, K4);
  test_rotate!(test_rotate_c2_cv, apply_c2_cv, C2);
  test_rotate!(test_rotate_c2_ce, apply_c2_ce, C2);
  test_rotate!(test_rotate_c2_ev, apply_c2_ev, C2);
  test_rotate!(test_rotate_trivial, apply_trivial, Trivial);

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

      let mut poses1 = [PackedIdx::null(); N];
      let mut poses2 = [PackedIdx::null(); N];
      for ((pos1, pos2), random_pos) in poses1
        .iter_mut()
        .zip(poses2.iter_mut())
        .zip(gen_unique_poses(N, &mut rng))
      {
        *pos1 = random_pos;
        *pos2 = random_pos;
      }

      let (black_equal, white_equal) = if rng.gen_bool(0.5) {
        (true, true)
      } else {
        // Generate different positions.
        randomly_mutate(&mut poses2, &mut rng);

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
}
