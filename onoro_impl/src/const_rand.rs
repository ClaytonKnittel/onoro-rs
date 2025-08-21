#[derive(Copy, Clone)]
pub struct Xoroshiro128([u64; 2]);

impl Xoroshiro128 {
  pub const fn from_seed(seed: [u64; 2]) -> Self {
    Self(seed)
  }

  #[inline]
  pub const fn next_u64(&self) -> (Self, u64) {
    let s0 = self.0[0];
    let s1 = self.0[1];
    let result = s0.wrapping_add(s1);
    let s1 = s0 ^ s1;

    (
      Self([s0.rotate_left(55) ^ s1 ^ (s1 << 14), s1.rotate_left(36)]),
      result,
    )
  }
}
