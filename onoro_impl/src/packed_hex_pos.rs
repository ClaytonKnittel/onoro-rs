use std::fmt::Display;

use onoro::hex_pos::HexPos;

/// A compact version of `HexPos`, used purely for saving memory. This is a
/// dummy class that can't do much, and can be converted to a normal `HexPos` to
/// use in computation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackedHexPos {
  x: u16,
  y: u16,
}

impl PackedHexPos {
  pub fn x(&self) -> u16 {
    self.x
  }

  pub fn y(&self) -> u16 {
    self.y
  }
}

impl From<HexPos> for PackedHexPos {
  fn from(value: HexPos) -> Self {
    Self {
      x: value.x() as u16,
      y: value.y() as u16,
    }
  }
}

impl From<PackedHexPos> for HexPos {
  fn from(value: PackedHexPos) -> Self {
    Self::new(value.x() as u32, value.y() as u32)
  }
}

impl Display for PackedHexPos {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", HexPos::from(*self))
  }
}
