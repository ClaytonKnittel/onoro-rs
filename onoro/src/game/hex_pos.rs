use std::fmt::Display;

use algebra::group::Cyclic;

use crate::groups::{C2, D3, D6, K4};

use super::{packed_hex_pos::PackedHexPos, packed_idx::PackedIdx};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HexPos {
  x: u32,
  y: u32,
}

impl HexPos {
  pub const fn new(x: u32, y: u32) -> Self {
    Self { x, y }
  }

  pub fn zero() -> Self {
    Self { x: 0, y: 0 }
  }

  pub const fn x(&self) -> u32 {
    self.x
  }

  pub const fn y(&self) -> u32 {
    self.y
  }

  /// Returns an iterator over all neighbors of this `HexPos`.
  pub fn each_neighbor(&self) -> impl Iterator<Item = Self> {
    [
      self + &HexPosOffset::new(-1, -1),
      self + &HexPosOffset::new(0, -1),
      self + &HexPosOffset::new(-1, 0),
      self + &HexPosOffset::new(1, 0),
      self + &HexPosOffset::new(0, 1),
      self + &HexPosOffset::new(1, 1),
    ]
    .into_iter()
  }

  /// Returns an iterator over the top-left neighbors of this `HexPos`. This has
  /// the property that for any two hex poses `p1` and `p2`, if `p1` is a top
  /// left neighbor of `p2`, `p2` is not a top left neighbor of `p1`, and for
  /// any two neighboring hex poses `p1` and `p2`, one of `p1` and `p2` are the
  /// top left neighbor of the other.
  pub fn each_top_left_neighbor(&self) -> impl Iterator<Item = Self> {
    [
      self + &HexPosOffset::new(-1, -1),
      self + &HexPosOffset::new(0, -1),
      self + &HexPosOffset::new(-1, 0),
    ]
    .into_iter()
  }
}

impl From<PackedHexPos> for HexPos {
  fn from(value: PackedHexPos) -> Self {
    Self {
      x: value.x() as u32,
      y: value.y() as u32,
    }
  }
}

impl From<PackedIdx> for HexPos {
  fn from(value: PackedIdx) -> Self {
    Self {
      x: value.x() as u32,
      y: value.y() as u32,
    }
  }
}

impl std::ops::Add<HexPosOffset> for HexPos {
  type Output = Self;

  fn add(self, rhs: HexPosOffset) -> Self::Output {
    Self {
      x: (self.x as i32 + rhs.x) as u32,
      y: (self.y as i32 + rhs.y) as u32,
    }
  }
}

impl std::ops::Add<&HexPosOffset> for &HexPos {
  type Output = HexPos;

  fn add(self, rhs: &HexPosOffset) -> Self::Output {
    HexPos {
      x: (self.x as i32 + rhs.x) as u32,
      y: (self.y as i32 + rhs.y) as u32,
    }
  }
}

impl std::ops::AddAssign<HexPosOffset> for HexPos {
  fn add_assign(&mut self, rhs: HexPosOffset) {
    self.x = (self.x as i32 + rhs.x) as u32;
    self.y = (self.y as i32 + rhs.y) as u32;
  }
}

impl std::ops::Sub for HexPos {
  type Output = HexPosOffset;

  fn sub(self, rhs: Self) -> Self::Output {
    HexPosOffset {
      x: self.x as i32 - rhs.x as i32,
      y: self.y as i32 - rhs.y as i32,
    }
  }
}

impl std::ops::Sub for &HexPos {
  type Output = HexPosOffset;

  fn sub(self, rhs: Self) -> Self::Output {
    HexPosOffset {
      x: self.x as i32 - rhs.x as i32,
      y: self.y as i32 - rhs.y as i32,
    }
  }
}

impl std::ops::Sub<HexPosOffset> for HexPos {
  type Output = HexPos;

  fn sub(self, rhs: HexPosOffset) -> Self::Output {
    Self {
      x: (self.x as i32 - rhs.x) as u32,
      y: (self.y as i32 - rhs.y) as u32,
    }
  }
}

impl std::ops::Sub<&HexPosOffset> for &HexPos {
  type Output = HexPos;

  fn sub(self, rhs: &HexPosOffset) -> Self::Output {
    HexPos {
      x: (self.x as i32 - rhs.x) as u32,
      y: (self.y as i32 - rhs.y) as u32,
    }
  }
}

impl std::ops::SubAssign<HexPosOffset> for HexPos {
  fn sub_assign(&mut self, rhs: HexPosOffset) {
    self.x = (self.x as i32 - rhs.x) as u32;
    self.y = (self.y as i32 - rhs.y) as u32;
  }
}

impl Display for HexPos {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "({}, {})", self.x, self.y)
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HexPosOffset {
  x: i32,
  y: i32,
}

impl HexPosOffset {
  pub const fn new(x: i32, y: i32) -> Self {
    Self { x, y }
  }

  pub const fn origin() -> Self {
    Self { x: 0, y: 0 }
  }

  pub const fn x(&self) -> i32 {
    self.x
  }

  pub const fn y(&self) -> i32 {
    self.y
  }

  /// Returns the sectant this point lies in, treating (0, 0) as the origin. The
  /// first sectant (0) is only the origin tile. The second (1) is every hex
  /// with (x >= 0, y >= 0, y < x). The third sectant (2) is the second sectant
  /// with c_r1 applied, etc. (up to sectant 6)
  pub fn sectant(&self) -> u32 {
    if self.x == 0 && self.y == 0 {
      0
    } else if self.y < 0 || (self.x < 0 && self.y == 0) {
      if self.x < self.y {
        4
      } else if self.x < 0 {
        5
      } else {
        6
      }
    } else {
      if self.y < self.x {
        1
      } else if self.x > 0 {
        2
      } else {
        3
      }
    }
  }

  /// The group of symmetries about the midpoint of a hex tile (c)
  pub fn apply_d6_c(&self, op: &D6) -> Self {
    match op {
      D6::Rot(0) => self.clone(),
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
      _ => unreachable!(),
    }
  }

  /// The group of symmetries about the vertex of a hex tile (v)
  pub fn apply_d3_v(&self, op: &D3) -> Self {
    match op {
      D3::Rot(0) => self.clone(),
      D3::Rot(1) => self.v_r2(),
      D3::Rot(2) => self.v_r4(),
      D3::Rfl(0) => self.v_s1(),
      D3::Rfl(1) => self.v_s3(),
      D3::Rfl(2) => self.v_s5(),
      _ => unreachable!(),
    }
  }

  /// The group of symmetries about the center of an edge (e) (C2 x C2 = { c_r0,
  /// c_s0 } x { c_r0, e_s3 })
  pub fn apply_k4_e(&self, op: &K4) -> Self {
    match (op.left(), op.right()) {
      (Cyclic::<2>(0), Cyclic::<2>(0)) => self.clone(),
      (Cyclic::<2>(1), Cyclic::<2>(0)) => self.e_s0(),
      (Cyclic::<2>(0), Cyclic::<2>(1)) => self.e_s3(),
      (Cyclic::<2>(1), Cyclic::<2>(1)) => self.e_r3(),
      _ => unreachable!(),
    }
  }

  /// The group of symmetries about the line from the center of a hex tile to a
  /// vertex.
  pub fn apply_c2_cv(&self, op: &C2) -> Self {
    match op {
      Cyclic::<2>(0) => self.clone(),
      Cyclic::<2>(1) => self.c_s1(),
      _ => unreachable!(),
    }
  }

  /// The group of symmetries about the line from the center of a hex tile to the
  /// midpoint of an edge.
  pub fn apply_c2_ce(&self, op: &C2) -> Self {
    match op {
      Cyclic::<2>(0) => self.clone(),
      Cyclic::<2>(1) => self.c_s0(),
      _ => unreachable!(),
    }
  }

  /// The group of symmetries about an edge.
  pub fn apply_c2_ev(&self, op: &C2) -> Self {
    match op {
      Cyclic::<2>(0) => self.clone(),
      Cyclic::<2>(1) => self.c_s3(),
      _ => unreachable!(),
    }
  }

  /// Applies the corresponding group operation for the given symmetry class (C,
  /// V, E, CV, ...) given the ordinal of the group operation.
  /// TODO remove if decide not to use
  ///  fn apply<G: Group>(uint32_t op_ordinal, SymmetryClass symm_class) const;

  /// The following all rotate the point 60, 120, and 180 degrees (R1, R2, R3).
  ///
  /// c_r1 rotates 60 degrees about the center of the origin tile.
  ///
  /// v_r2 rotates 120 degrees about the top right vertex of the origin tile.
  ///
  /// e_r3 rotates 180 degrees about the center of the right edge of the origin
  /// tile.
  ///
  /// Note: these algorithms are incompatible with each other, i.e.
  /// p.c_r1().c_r1() != p.v_r2().

  fn c_r1(&self) -> Self {
    Self {
      x: self.x - self.y,
      y: self.x,
    }
  }

  fn c_r2(&self) -> Self {
    self.c_r1().c_r1()
  }

  fn c_r3(&self) -> Self {
    self.c_r2().c_r1()
  }

  fn c_r4(&self) -> Self {
    self.c_r3().c_r1()
  }

  fn c_r5(&self) -> Self {
    self.c_r4().c_r1()
  }

  fn v_r2(&self) -> Self {
    Self {
      x: 1 - self.y,
      y: self.x - self.y,
    }
  }

  fn v_r4(&self) -> Self {
    self.v_r2().v_r2()
  }

  fn e_r3(&self) -> Self {
    Self {
      x: 1 - self.x,
      y: -self.y,
    }
  }

  /// [cve]_r<n>: Reflects the point across a line at angle n*30 degrees,
  /// passing through:
  ///  - c: the center of the origin hex
  ///  - v: the top right vertex of the origin hex
  ///  - e: the center of the right edge of the origin hex

  fn c_s0(&self) -> Self {
    Self {
      x: self.x - self.y,
      y: -self.y,
    }
  }

  fn c_s1(&self) -> Self {
    self.c_s0().c_r1()
  }

  fn c_s2(&self) -> Self {
    self.c_s0().c_r2()
  }

  fn c_s3(&self) -> Self {
    self.c_s0().c_r3()
  }

  fn c_s4(&self) -> Self {
    self.c_s0().c_r4()
  }

  fn c_s5(&self) -> Self {
    self.c_s0().c_r5()
  }

  fn v_s1(&self) -> Self {
    self.c_s1()
  }

  fn v_s3(&self) -> Self {
    self.v_s1().v_r2()
  }

  fn v_s5(&self) -> Self {
    self.v_s1().v_r4()
  }

  fn e_s0(&self) -> Self {
    self.c_s0()
  }

  fn e_s3(&self) -> Self {
    self.e_s0().e_r3()
  }
}

impl From<PackedIdx> for HexPosOffset {
  fn from(value: PackedIdx) -> Self {
    Self {
      x: value.x() as i32,
      y: value.y() as i32,
    }
  }
}

impl std::ops::Add for HexPosOffset {
  type Output = Self;

  fn add(self, rhs: HexPosOffset) -> Self::Output {
    Self {
      x: self.x + rhs.x,
      y: self.y + rhs.y,
    }
  }
}

impl std::ops::Add for &HexPosOffset {
  type Output = HexPosOffset;

  fn add(self, rhs: &HexPosOffset) -> Self::Output {
    HexPosOffset {
      x: self.x + rhs.x,
      y: self.y + rhs.y,
    }
  }
}

impl std::ops::Add<HexPos> for HexPosOffset {
  type Output = HexPos;

  fn add(self, rhs: HexPos) -> Self::Output {
    HexPos {
      x: (self.x + rhs.x as i32) as u32,
      y: (self.y + rhs.y as i32) as u32,
    }
  }
}

impl std::ops::Add<&HexPos> for &HexPosOffset {
  type Output = HexPos;

  fn add(self, rhs: &HexPos) -> Self::Output {
    HexPos {
      x: (self.x + rhs.x as i32) as u32,
      y: (self.y + rhs.y as i32) as u32,
    }
  }
}

impl std::ops::AddAssign for HexPosOffset {
  fn add_assign(&mut self, rhs: HexPosOffset) {
    self.x += rhs.x;
    self.y += rhs.y;
  }
}

impl std::ops::Sub for HexPosOffset {
  type Output = HexPosOffset;

  fn sub(self, rhs: Self) -> Self::Output {
    Self {
      x: self.x - rhs.x,
      y: self.y - rhs.y,
    }
  }
}

impl std::ops::Sub for &HexPosOffset {
  type Output = HexPosOffset;

  fn sub(self, rhs: Self) -> Self::Output {
    HexPosOffset {
      x: self.x - rhs.x,
      y: self.y - rhs.y,
    }
  }
}

impl std::ops::SubAssign for HexPosOffset {
  fn sub_assign(&mut self, rhs: HexPosOffset) {
    self.x -= rhs.x;
    self.y -= rhs.y;
  }
}
