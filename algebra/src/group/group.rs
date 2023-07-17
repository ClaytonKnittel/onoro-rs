use crate::monoid::Monoid;

/// An algebraic group.
pub trait Group: Monoid {
  /// The unique inverse of a group element.
  fn inverse(&self) -> Self;
}
