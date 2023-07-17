/// An algebraic semigroup.
pub trait Semigroup: PartialEq + Sized {
  fn op(&self, other: &Self) -> Self;
}
