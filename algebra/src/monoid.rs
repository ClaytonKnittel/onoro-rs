use crate::semigroup::Semigroup;

/// An algebraic monoid.
pub trait Monoid: Semigroup {
  /// The identity element of the monoid.
  fn identity() -> Self;
}
