use std::{
  borrow::Borrow,
  fmt::{Debug, Display},
  num::Wrapping,
};

use onoro::{
  OnoroIndex,
  hex_pos::{HexPos, HexPosOffset},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PackedIdx {
  bytes: Wrapping<u8>,
}

impl PackedIdx {
  /// An offset to apply to (y - x) so it is never negative.
  pub const fn xy_offset<const N: usize>() -> u32 {
    N as u32
  }

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

  /// Returns the coordinate along the xy-axis, the angular bisector between
  /// the x- and y-axes. This is normalized such that any PackedIdx will return
  /// a positive value.
  ///
  ///```text
  /// (0,3)   (1,3)   (2,3)   (3,3)
  ///   3       2       1       0
  ///     (0,2)   (1,2)   (2,2)   (3,2)
  ///       2       1       0      -1
  ///         (0,1)   (1,1)   (2,1)   (3,1)
  ///           1       0      -1      -2
  ///             (0,0)   (1,0)   (2,0)   (3,0)
  ///               0      -1      -2      -3
  ///```
  pub const fn xy<const N: usize>(&self) -> u32 {
    self.y() + Self::xy_offset::<N>() - self.x()
  }

  /// Returns the underlying representation of the `PackedIdx` as a `u8`.
  ///
  /// # Safety
  ///
  /// This function is unsafe because this representation should normally be
  /// opaque to anything external to this class, but it can be used for more
  /// efficient tile occupancy checking in the game state.
  pub const unsafe fn bytes(&self) -> u8 {
    self.bytes.0
  }

  /// # Safety
  ///
  /// Assumes no overflow in x or y
  pub const unsafe fn unsafe_add(&self, other: &PackedIdx) -> PackedIdx {
    // Assume no overflow in x or y
    PackedIdx {
      bytes: Wrapping(self.bytes.0.wrapping_add(other.bytes.0)),
    }
  }

  /// # Safety
  ///
  /// This breaks the type safety of relative/absolute coordinates.
  pub const unsafe fn from_idx_offset(offset: IdxOffset) -> Self {
    PackedIdx {
      bytes: Wrapping(offset.bytes.0),
    }
  }
}

impl OnoroIndex for PackedIdx {
  fn from_coords(x: u32, y: u32) -> Self {
    Self::new(x, y)
  }

  fn x(&self) -> i32 {
    self.x() as i32
  }

  fn y(&self) -> i32 {
    self.y() as i32
  }
}

impl From<HexPos> for PackedIdx {
  fn from(value: HexPos) -> Self {
    Self::new(value.x(), value.y())
  }
}

impl From<PackedIdx> for HexPos {
  fn from(value: PackedIdx) -> Self {
    Self::new(value.x(), value.y())
  }
}

impl From<PackedIdx> for HexPosOffset {
  fn from(value: PackedIdx) -> Self {
    Self::new(value.x() as i32, value.y() as i32)
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

impl std::ops::Sub for PackedIdx {
  type Output = IdxOffset;

  fn sub(self, rhs: Self) -> Self::Output {
    debug_assert!(
      self.x() >= rhs.x() && self.y() >= rhs.y(),
      "Cannot subtract larger PackedIdx from smaller one: {self} - {rhs}"
    );
    IdxOffset::from_bytes(Wrapping(self.bytes.0.wrapping_sub(rhs.bytes.0)))
  }
}

impl Display for PackedIdx {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "({}, {})", self.x(), self.y())
  }
}

impl Debug for PackedIdx {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{self}")
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

  const fn from_bytes(bytes: Wrapping<u8>) -> Self {
    Self { bytes }
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

impl From<HexPosOffset> for IdxOffset {
  fn from(value: HexPosOffset) -> Self {
    Self::new(value.x(), value.y())
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

pub trait FilterNullPackedIdx<B: Borrow<PackedIdx>>: Iterator<Item = B> {
  /// Filters null `PackedIdx`'s from the iterator.
  fn filter_null(self) -> impl Iterator<Item = B>;
}

impl<B, I> FilterNullPackedIdx<B> for I
where
  B: Borrow<PackedIdx>,
  I: Iterator<Item = B>,
{
  fn filter_null(self) -> impl Iterator<Item = B> {
    self.filter(|packed_idx| *packed_idx.borrow() != PackedIdx::null())
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
