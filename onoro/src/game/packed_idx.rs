#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackedIdx {
  bytes: u8,
}

impl PackedIdx {
  /// Returns a PackedIdx which cannot be an index of a pawn on the game board,
  /// no matter how the pawns are arranged. This relies on the board
  /// self-adjusting to keep pawns off of the border.
  pub fn null() -> Self {
    Self { bytes: 0 }
  }

  pub fn x(&self) -> u32 {
    (self.bytes as u32) & 0x0fu32
  }

  pub fn y(&self) -> u32 {
    ((self.bytes as u32) >> 4) & 0x0fu32
  }
}

impl std::ops::Add for PackedIdx {
  type Output = Self;

  fn add(self, rhs: Self) -> Self::Output {
    // Assume no overflow in x or y
    Self {
      bytes: self.bytes + rhs.bytes,
    }
  }
}
