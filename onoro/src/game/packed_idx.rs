#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackedIdx {
  bytes: u8,
}

impl PackedIdx {
  pub const fn new(x: u32, y: u32) -> Self {
    debug_assert!(x < 0x10);
    debug_assert!(y < 0x10);

    Self {
      bytes: (x | (y << 4)) as u8,
    }
  }

  /// Returns a PackedIdx which cannot be an index of a pawn on the game board,
  /// no matter how the pawns are arranged. This relies on the board
  /// self-adjusting to keep pawns off of the border.
  pub const fn null() -> Self {
    Self { bytes: 0 }
  }

  pub const fn x(&self) -> u32 {
    (self.bytes as u32) & 0x0fu32
  }

  pub const fn y(&self) -> u32 {
    ((self.bytes as u32) >> 4) & 0x0fu32
  }

  pub const unsafe fn unsafe_add(&self, other: &PackedIdx) -> PackedIdx {
    // Assume no overflow in x or y
    PackedIdx {
      bytes: self.bytes + other.bytes,
    }
  }
}

impl std::ops::Add<IdxOffset> for PackedIdx {
  type Output = Self;

  fn add(self, rhs: IdxOffset) -> Self::Output {
    Self::new(
      (self.x() as i32 + rhs.x()) as u32,
      (self.y() as i32 + rhs.y()) as u32,
    )
  }
}

pub struct IdxOffset {
  x: i32,
  y: i32,
}

impl IdxOffset {
  pub const fn new(x: i32, y: i32) -> Self {
    Self { x, y }
  }

  pub const fn x(&self) -> i32 {
    self.x
  }

  pub const fn y(&self) -> i32 {
    self.y
  }
}

impl std::ops::Add<PackedIdx> for IdxOffset {
  type Output = Self;

  fn add(self, rhs: PackedIdx) -> Self::Output {
    Self::new(self.x() + rhs.x() as i32, self.y() + rhs.y() as i32)
  }
}

impl std::ops::Add for IdxOffset {
  type Output = Self;

  fn add(self, rhs: IdxOffset) -> Self::Output {
    Self::new(self.x() + rhs.x(), self.y() + rhs.y())
  }
}

impl std::ops::AddAssign for IdxOffset {
  fn add_assign(&mut self, rhs: IdxOffset) {
    self.x += rhs.x();
    self.y += rhs.y();
  }
}
