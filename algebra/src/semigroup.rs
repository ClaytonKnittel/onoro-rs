use std::ops::Mul;

/// An algebraic semigroup.
pub trait Semigroup: PartialEq + Sized + Mul<Output = Self> {}
