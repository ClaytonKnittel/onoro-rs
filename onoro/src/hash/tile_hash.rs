use std::marker::PhantomData;

use algebra::group::{Cyclic, Group};

use super::groups::{C2, D3, D6, K4};

const C_MASK: u64 = 0x0fff_ffff_ffff_ffff;
const V_MASK: u64 = 0x0fff_ffff_ffff_ffff;

pub struct TileHash<G: Group> {
  hash: u64,
  _p: PhantomData<G>,
}

impl<G> TileHash<G>
where
  G: Group,
{
  pub fn new<R>(rng: &mut R) -> Self
  where
    R: rand::Rng,
  {
    Self {
      hash: rng.next_u64(),
      _p: PhantomData::default(),
    }
  }

  pub fn color_swap(&mut self) {
    let hl = self.hash & 0x5555_5555_5555_5555;
    let hr = self.hash & 0xaaaa_aaaa_aaaa_aaaa;

    self.hash = (hl << 1) | (hr >> 1);
  }
}

impl TileHash<D6> {
  fn r1(h: u64) -> u64 {
    ((h << 10) | (h >> 50)) & C_MASK
  }

  fn r2(h: u64) -> u64 {
    Self::r1(Self::r1(h))
  }

  fn r3(h: u64) -> u64 {
    Self::r1(Self::r2(h))
  }

  fn r4(h: u64) -> u64 {
    Self::r1(Self::r3(h))
  }

  fn r5(h: u64) -> u64 {
    Self::r1(Self::r4(h))
  }

  fn s0(h: u64) -> u64 {
    let b14 = h & 0x000000ffc00003ff;
    let b26 = h & 0x0ffc0000000ffc00;
    let b35 = h & 0x0003ff003ff00000;

    let b26 = (b26 << 40) | (b26 >> 40);
    let b35 = ((b35 << 20) | (b35 >> 20)) & 0x0003ff003ff00000;
    b14 | b26 | b35
  }

  fn s1(h: u64) -> u64 {
    let b12 = h & 0x00000000000fffff;
    let b36 = h & 0x0ffc00003ff00000;
    let b45 = h & 0x0003ffffc0000000;

    let b12 = ((b12 << 10) | (b12 >> 10)) & 0x00000000000fffff;
    let b36 = (b36 << 30) | (b36 >> 30);
    let b45 = ((b45 << 10) | (b45 >> 10)) & 0x0003ffffc0000000;
    b12 | b36 | b45
  }

  fn s2(h: u64) -> u64 {
    let b13 = h & 0x000000003ff003ff;
    let b25 = h & 0x0003ff00000ffc00;
    let b46 = h & 0x0ffc00ffc0000000;

    let b13 = ((b13 << 20) | (b13 >> 20)) & 0x000000003ff003ff;
    let b46 = ((b46 << 20) | (b46 >> 20)) & 0x0ffc00ffc0000000;
    b13 | b25 | b46
  }

  fn s3(h: u64) -> u64 {
    let b14 = h & 0x000000ffc00003ff;
    let b23 = h & 0x000000003ffffc00;
    let b56 = h & 0x0fffff0000000000;

    let b14 = ((b14 << 30) | (b14 >> 30)) & 0x000000ffc00003ff;
    let b23 = ((b23 << 10) | (b23 >> 10)) & 0x000000003ffffc00;
    let b56 = ((b56 << 10) | (b56 >> 10)) & 0x0fffff0000000000;
    b14 | b23 | b56
  }

  fn s4(h: u64) -> u64 {
    let b15 = h & 0x0003ff00000003ff;
    let b24 = h & 0x000000ffc00ffc00;
    let b36 = h & 0x0ffc00003ff00000;

    let b15 = (b15 << 40) | (b15 >> 40);
    let b24 = ((b24 << 20) | (b24 >> 20)) & 0x000000ffc00ffc00;
    b15 | b24 | b36
  }

  fn s5(h: u64) -> u64 {
    let b16 = h & 0x0ffc0000000003ff;
    let b25 = h & 0x0003ff00000ffc00;
    let b34 = h & 0x000000fffff00000;

    let b16 = (b16 << 50) | (b16 >> 50);
    let b25 = (b25 << 30) | (b25 >> 30);
    let b34 = ((b34 << 10) | (b34 >> 10)) & 0x000000fffff00000;
    b16 | b25 | b34
  }

  pub fn apply(&mut self, op: &D6) {
    self.hash = match op {
      D6::Rot(0) => self.hash,
      D6::Rot(1) => Self::r1(self.hash),
      D6::Rot(2) => Self::r2(self.hash),
      D6::Rot(3) => Self::r3(self.hash),
      D6::Rot(4) => Self::r4(self.hash),
      D6::Rot(5) => Self::r5(self.hash),
      D6::Rfl(0) => Self::s0(self.hash),
      D6::Rfl(1) => Self::s1(self.hash),
      D6::Rfl(2) => Self::s2(self.hash),
      D6::Rfl(3) => Self::s3(self.hash),
      D6::Rfl(4) => Self::s4(self.hash),
      D6::Rfl(5) => Self::s5(self.hash),
      _ => unreachable!(),
    };
  }

  fn make_r1(h: u64) -> u64 {
    // Repeat the first 10 bits across the remaining 50 bits, leaving the end
    // zeroed out.
    let b = h & 0x3ff;
    let b = b | (b << 10);
    b | (b << 20) | (b << 40)
  }

  fn make_s0(h: u64) -> u64 {
    let b14 = h & 0x000000ffc00003ff;
    let b26 = h & 0x00000000000ffc00;
    let b35 = h & 0x000000003ff00000;

    let b26 = b26 | (b26 << 40);
    let b35 = b35 | (b35 << 20);
    b14 | b26 | b35
  }

  fn make_s1(h: u64) -> u64 {
    let b12 = h & 0x00000000000003ff;
    let b36 = h & 0x000000003ff00000;
    let b45 = h & 0x000000ffc0000000;

    let b12 = b12 | (b12 << 10);
    let b36 = b36 | (b36 << 30);
    let b45 = b45 | (b45 << 10);
    b12 | b36 | b45
  }

  fn make_s2(h: u64) -> u64 {
    let b13 = h & 0x00000000000003ff;
    let b25 = h & 0x0003ff00000ffc00;
    let b46 = h & 0x000000ffc0000000;

    let b13 = b13 | (b13 << 20);
    let b46 = b46 | (b46 << 20);
    b13 | b25 | b46
  }

  fn make_s3(h: u64) -> u64 {
    let b14 = h & 0x00000000000003ff;
    let b23 = h & 0x00000000000ffc00;
    let b56 = h & 0x0003ff0000000000;

    let b14 = b14 | (b14 << 30);
    let b23 = b23 | (b23 << 10);
    let b56 = b56 | (b56 << 10);
    b14 | b23 | b56
  }

  fn make_s4(h: u64) -> u64 {
    let b15 = h & 0x00000000000003ff;
    let b24 = h & 0x00000000000ffc00;
    let b36 = h & 0x0ffc00003ff00000;

    let b15 = b15 | (b15 << 40);
    let b24 = b24 | (b24 << 20);
    b15 | b24 | b36
  }

  fn make_s5(h: u64) -> u64 {
    let b16 = h & 0x00000000000003ff;
    let b25 = h & 0x00000000000ffc00;
    let b34 = h & 0x000000003ff00000;

    let b16 = b16 | (b16 << 50);
    let b25 = b25 | (b25 << 30);
    let b34 = b34 | (b34 << 10);
    b16 | b25 | b34
  }

  pub fn make_invariant(&mut self, op: &D6) {
    self.hash = match op {
      D6::Rot(1) => Self::make_r1(self.hash),
      D6::Rfl(0) => Self::make_s0(self.hash),
      D6::Rfl(1) => Self::make_s1(self.hash),
      D6::Rfl(2) => Self::make_s2(self.hash),
      D6::Rfl(3) => Self::make_s3(self.hash),
      D6::Rfl(4) => Self::make_s4(self.hash),
      D6::Rfl(5) => Self::make_s5(self.hash),
      D6::Rot(0) | D6::Rot(2) | D6::Rot(3) | D6::Rot(4) | D6::Rot(5) => {
        panic!("Cannot make D6 hash invariant under {op}")
      }
      _ => unreachable!(),
    }
  }
}

impl TileHash<D3> {
  fn r1(h: u64) -> u64 {
    return ((h << 20) | (h >> 40)) & V_MASK;
  }

  fn r2(h: u64) -> u64 {
    return Self::r1(Self::r1(h));
  }

  fn s0(h: u64) -> u64 {
    let b1 = h & 0x00000000000fffff;
    let b2 = h & 0x000000fffff00000;
    let b3 = h & 0x0fffff0000000000;

    let b2 = b2 << 20;
    let b3 = b3 >> 20;
    return b1 | b2 | b3;
  }

  fn s1(h: u64) -> u64 {
    let b1 = h & 0x00000000000fffff;
    let b2 = h & 0x000000fffff00000;
    let b3 = h & 0x0fffff0000000000;

    let b1 = b1 << 20;
    let b2 = b2 >> 20;
    return b1 | b2 | b3;
  }

  fn s2(h: u64) -> u64 {
    let b13 = h & 0x0fffff00000fffff;
    let b2 = h & 0x000000fffff0000;

    let b13 = (b13 << 40) | (b13 >> 40);
    return b13 | b2;
  }

  pub fn apply(&mut self, op: &D3) {
    self.hash = match op {
      D3::Rot(0) => self.hash,
      D3::Rot(1) => Self::r1(self.hash),
      D3::Rot(2) => Self::r2(self.hash),
      D3::Rfl(0) => Self::s0(self.hash),
      D3::Rfl(1) => Self::s1(self.hash),
      D3::Rfl(2) => Self::s2(self.hash),
      _ => unreachable!(),
    };
  }

  fn make_r1(h: u64) -> u64 {
    // Repeat the first 21 bits across the remaining 42 bits, leaving the end
    // zeroed out.
    let b = h & 0xfffff;
    b | (b << 20) | (b << 40)
  }

  fn make_s0(h: u64) -> u64 {
    let b1 = h & 0x00000000000fffff;
    let b23 = h & 0x000000fffff00000;

    let b23 = b23 | (b23 << 20);
    b1 | b23
  }

  fn make_s1(h: u64) -> u64 {
    let b12 = h & 0x00000000000fffff;
    let b3 = h & 0x0fffff0000000000;

    let b12 = b12 | (b12 << 20);
    b12 | b3
  }

  fn make_s2(h: u64) -> u64 {
    let b13 = h & 0x00000000000fffff;
    let b2 = h & 0x000000fffff00000;

    let b13 = b13 | (b13 << 40);
    b13 | b2
  }

  pub fn make_invariant(&mut self, op: &D3) {
    self.hash = match op {
      D3::Rot(1) => Self::make_r1(self.hash),
      D3::Rfl(0) => Self::make_s0(self.hash),
      D3::Rfl(1) => Self::make_s1(self.hash),
      D3::Rfl(2) => Self::make_s2(self.hash),
      D3::Rot(0) | D3::Rot(2) => {
        panic!("Cannot make D3 hash invariant under {op}")
      }
      _ => unreachable!(),
    };
  }
}

impl TileHash<K4> {
  fn a(h: u64) -> u64 {
    (h << 32) | (h >> 32)
  }

  fn b(h: u64) -> u64 {
    let b13 = h & 0x0000ffff0000ffff;
    let b24 = h & 0xffff0000ffff0000;

    (b13 << 16) | (b24 >> 16)
  }

  fn c(h: u64) -> u64 {
    let b = h.swap_bytes();
    let b1357 = b & 0x00ff00ff00ff00ff;
    let b2468 = b & 0xff00ff00ff00ff00;

    return (b1357 << 8) | (b2468 >> 8);
  }

  pub fn apply(&mut self, op: &K4) {
    self.hash = match (op.left(), op.right()) {
      (Cyclic::<2>(0), Cyclic::<2>(0)) => self.hash,
      (Cyclic::<2>(1), Cyclic::<2>(0)) => Self::a(self.hash),
      (Cyclic::<2>(0), Cyclic::<2>(1)) => Self::b(self.hash),
      (Cyclic::<2>(1), Cyclic::<2>(1)) => Self::c(self.hash),
      _ => unreachable!(),
    };
  }

  fn make_a(h: u64) -> u64 {
    let b12 = h & 0x0000_0000_ffff_ffff;

    b12 | (b12 << 32)
  }

  fn make_b(h: u64) -> u64 {
    let b13 = h & 0x0000_ffff_0000_ffff;

    b13 | (b13 << 16)
  }

  fn make_c(h: u64) -> u64 {
    let b1 = h & 0xffff;
    let b2 = h & 0xffff_0000;
    b1 | b2 | (b2 << 16) | (b1 << 48)
  }

  pub fn make_invariant(&mut self, op: &K4) {
    self.hash = match (op.left(), op.right()) {
      (Cyclic::<2>(1), Cyclic::<2>(0)) => Self::make_a(self.hash),
      (Cyclic::<2>(0), Cyclic::<2>(1)) => Self::make_b(self.hash),
      (Cyclic::<2>(1), Cyclic::<2>(1)) => Self::make_c(self.hash),
      (Cyclic::<2>(0), Cyclic::<2>(0)) => {
        panic!("Cannot make K4 hash invariant under {op}")
      }
      _ => unreachable!(),
    };
  }
}

impl TileHash<C2> {
  fn a(h: u64) -> u64 {
    (h << 32) | (h >> 32)
  }

  pub fn apply(&mut self, op: &C2) {
    self.hash = match op {
      Cyclic::<2>(0) => self.hash,
      Cyclic::<2>(1) => Self::a(self.hash),
      _ => unreachable!(),
    };
  }

  fn make_a(h: u64) -> u64 {
    let b12 = h & 0x0000_0000_ffff_ffff;

    b12 | (b12 << 32)
  }

  pub fn make_invariant(&mut self, op: &C2) {
    self.hash = match op {
      Cyclic::<2>(1) => Self::make_a(self.hash),
      Cyclic::<2>(0) => {
        panic!("Cannot make C2 hash invariant under {op}")
      }
      _ => unreachable!(),
    };
  }
}
