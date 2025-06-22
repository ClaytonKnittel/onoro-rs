use std::ops::Mul;

/// An algebraic semigroup.
pub trait Semigroup: PartialEq + Sized + Mul<Output = Self> {
  fn for_each() -> impl Iterator<Item = Self>;
}
