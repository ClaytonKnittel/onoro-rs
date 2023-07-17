use crate::finite::Finite;

/// A trait for sets with labeled elements.
pub trait Ordinal: Finite {
  /// Returns a unique integer for each element of a set which exactly covers
  /// the range (0..size). There must exist an element associated with each
  /// number in the range.
  fn ord(&self) -> usize;

  /// The inverse of `ord`, returns the element associated with the ordinal.
  fn from_ord(ord: usize) -> Self;
}
