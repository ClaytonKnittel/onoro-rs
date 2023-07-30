use std::{fmt::Display, ops::Mul};

use crate::{finite::Finite, group::Group, monoid::Monoid, ordinal::Ordinal, semigroup::Semigroup};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trivial;

impl Mul for Trivial {
  type Output = Self;

  fn mul(self, _rhs: Self) -> Self::Output {
    Self
  }
}

impl Finite for Trivial {
  const SIZE: usize = 1;
}

impl Ordinal for Trivial {
  fn ord(&self) -> usize {
    0
  }

  fn from_ord(_ord: usize) -> Self {
    Self
  }
}

impl Semigroup for Trivial {}

impl Monoid for Trivial {
  fn identity() -> Self {
    Self
  }
}

impl Group for Trivial {
  fn inverse(&self) -> Self {
    Self
  }
}

impl Display for Trivial {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "e")
  }
}
