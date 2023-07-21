use std::num::Wrapping;

use super::hex_pos::HexPos;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackedIdx {
  bytes: Wrapping<u8>,
}

impl PackedIdx {
  pub const fn new(x: u32, y: u32) -> Self {
    debug_assert!(x < 0x10);
    debug_assert!(y < 0x10);

    Self {
      bytes: Wrapping((x | (y << 4)) as u8),
    }
  }

  /// Returns a PackedIdx which cannot be an index of a pawn on the game board,
  /// no matter how the pawns are arranged. This relies on the board
  /// self-adjusting to keep pawns off of the border.
  pub const fn null() -> Self {
    Self { bytes: Wrapping(0) }
  }

  pub const fn x(&self) -> u32 {
    (self.bytes.0 as u32) & 0x0fu32
  }

  pub const fn y(&self) -> u32 {
    ((self.bytes.0 as u32) >> 4) & 0x0fu32
  }

  /// Returns the underlying representation of the `PackedIdx` as a `u8`.
  ///
  /// This function is unsafe because this representation should normally be
  /// opaque to anything external to this class, but it can be used for more
  /// efficient tile occupancy checking in the game state.
  pub const unsafe fn bytes(&self) -> u8 {
    self.bytes.0
  }

  pub const unsafe fn unsafe_add(&self, other: &PackedIdx) -> PackedIdx {
    // Assume no overflow in x or y
    PackedIdx {
      bytes: Wrapping(self.bytes.0.wrapping_add(other.bytes.0)),
    }
  }
}

impl std::ops::Add<IdxOffset> for PackedIdx {
  type Output = Self;

  fn add(self, rhs: IdxOffset) -> Self::Output {
    Self {
      bytes: self.bytes + rhs.bytes,
    }
  }
}

impl std::ops::AddAssign<IdxOffset> for PackedIdx {
  fn add_assign(&mut self, rhs: IdxOffset) {
    self.bytes += rhs.bytes
  }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct IdxOffset {
  bytes: Wrapping<u8>,
}

impl IdxOffset {
  pub const fn new(x: i32, y: i32) -> Self {
    Self {
      bytes: Wrapping(Self::by_x(x).bytes.0.wrapping_add(Self::by_y(y).bytes.0)),
    }
  }

  /// Constructs an `IdxOffset` that shifts a `PackedIdx` by `dx` along the
  /// x-axis.
  pub const fn by_x(dx: i32) -> Self {
    // For negative dx, let the bits above 0-3 overflow into the y "slot", so
    // that upon subtraction the y slot will remain unchanged (so long as x is
    // not smaller than abs(dx)).
    Self {
      bytes: Wrapping(dx as u8),
    }
  }

  /// Constructs an `IdxOffset` that shifts a `PackedIdx` by `dy` along the
  /// y-axis.
  pub const fn by_y(dy: i32) -> Self {
    Self {
      bytes: Wrapping((dy << 4) as u8),
    }
  }

  /// Constructs the additive identity of `IdxOffset`.
  pub const fn identity() -> Self {
    Self::new(0, 0)
  }
}

impl std::ops::Add<PackedIdx> for IdxOffset {
  type Output = PackedIdx;

  fn add(self, rhs: PackedIdx) -> Self::Output {
    PackedIdx {
      bytes: self.bytes + rhs.bytes,
    }
  }
}

impl std::ops::Add for IdxOffset {
  type Output = Self;

  fn add(self, rhs: IdxOffset) -> Self::Output {
    Self {
      bytes: self.bytes + rhs.bytes,
    }
  }
}

impl std::ops::AddAssign for IdxOffset {
  fn add_assign(&mut self, rhs: IdxOffset) {
    self.bytes += rhs.bytes
  }
}

impl From<HexPos> for PackedIdx {
  fn from(value: HexPos) -> Self {
    Self::new(value.x(), value.y())
  }
}

#[cfg(test)]
mod tests {
  use super::{IdxOffset, PackedIdx};

  #[test]
  fn test_add_x() {
    let pos = PackedIdx::new(3, 7);
    let offset = IdxOffset::by_x(1);
    assert_eq!(pos + offset, PackedIdx::new(4, 7));
  }

  #[test]
  fn test_add_negative_x() {
    let pos = PackedIdx::new(3, 7);
    let offset = IdxOffset::by_x(-1);
    assert_eq!(pos + offset, PackedIdx::new(2, 7));
  }

  #[test]
  fn test_add_y() {
    let pos = PackedIdx::new(3, 7);
    let offset = IdxOffset::by_y(1);
    assert_eq!(pos + offset, PackedIdx::new(3, 8));
  }

  #[test]
  fn test_add_negative_y() {
    let pos = PackedIdx::new(3, 7);
    let offset = IdxOffset::by_y(-1);
    assert_eq!(pos + offset, PackedIdx::new(3, 6));
  }

  #[test]
  fn test_add_two_dim() {
    let pos = PackedIdx::new(3, 7);
    let offset = IdxOffset::new(2, 1);
    assert_eq!(pos + offset, PackedIdx::new(5, 8));

    let pos = PackedIdx::new(3, 7);
    let offset = IdxOffset::new(2, -1);
    assert_eq!(pos + offset, PackedIdx::new(5, 6));

    let pos = PackedIdx::new(3, 7);
    let offset = IdxOffset::new(-2, 1);
    assert_eq!(pos + offset, PackedIdx::new(1, 8));

    let pos = PackedIdx::new(3, 7);
    let offset = IdxOffset::new(-2, -1);
    assert_eq!(pos + offset, PackedIdx::new(1, 6));
  }
}
