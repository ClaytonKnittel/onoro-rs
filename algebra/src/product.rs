use std::fmt::Display;

use crate::{
  finite::Finite,
  group::{Cyclic, Group},
  monoid::Monoid,
  ordinal::Ordinal,
  semigroup::Semigroup,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DirectProduct<L, R> {
  left: L,
  right: R,
}

impl<L, R> DirectProduct<L, R> {
  pub const fn new(left: L, right: R) -> Self {
    Self { left, right }
  }

  pub const fn left(&self) -> &L {
    &self.left
  }

  pub const fn right(&self) -> &R {
    &self.right
  }
}

impl DirectProduct<Cyclic<2>, Cyclic<2>> {
  pub const fn const_op(&self, rhs: &Self) -> Self {
    Self {
      left: self.left.const_op(&rhs.left),
      right: self.right.const_op(&rhs.right),
    }
  }
}

impl<L, R> Finite for DirectProduct<L, R>
where
  L: Finite,
  R: Finite,
{
  const SIZE: usize = L::SIZE * R::SIZE;
}

impl<L, R> Ordinal for DirectProduct<L, R>
where
  L: Ordinal,
  R: Ordinal,
{
  fn ord(&self) -> usize {
    self.left.ord() + self.right.ord() * L::SIZE
  }

  fn from_ord(ord: usize) -> Self {
    let l = ord % L::SIZE;
    let r = ord / L::SIZE;
    Self {
      left: L::from_ord(l),
      right: R::from_ord(r),
    }
  }
}

impl<L, R> Semigroup for DirectProduct<L, R>
where
  L: Semigroup,
  R: Semigroup,
{
  fn op(&self, other: &Self) -> Self {
    Self {
      left: self.left.op(&other.left),
      right: self.right.op(&other.right),
    }
  }
}

impl<L, R> Monoid for DirectProduct<L, R>
where
  L: Monoid,
  R: Monoid,
{
  fn identity() -> Self {
    Self {
      left: L::identity(),
      right: R::identity(),
    }
  }
}

impl<L, R> Group for DirectProduct<L, R>
where
  L: Group,
  R: Group,
{
  fn inverse(&self) -> Self {
    Self {
      left: self.left.inverse(),
      right: self.right.inverse(),
    }
  }
}

impl<L, R> Display for DirectProduct<L, R>
where
  L: Display,
  R: Display,
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    // TODO: make only outermost DirectProduct use parens
    write!(f, "({}, {})", self.left, self.right)
  }
}

#[macro_export]
macro_rules! direct_product_type {
  ($g:ty) => {
    $g
  };
  ($l:ty, $($rs:ty),+) => {
    $crate::product::DirectProduct<$l, direct_product_type!($($rs),+)>
  }
}

#[macro_export]
macro_rules! direct_product {
  ($g:expr) => {
    $g
  };
  ($l:expr, $($rs:expr),+) => {
    $crate::product::DirectProduct::new($l, direct_product!($($rs),+))
  }
}

#[cfg(test)]
mod tests {
  use crate::{
    group::{Cyclic, Dihedral},
    monoid::Monoid,
    ordinal::Ordinal,
    semigroup::Semigroup,
  };

  #[test]
  fn test_macro() {
    type G = direct_product_type!(Dihedral<7>, Cyclic<2>);
    let mut e1: G = direct_product!(Dihedral::Rot(1), Cyclic::from_ord(1));
    let op = e1.clone();

    // Should require 13 rotations to get to the identity.
    for _ in 0..13 {
      assert_ne!(e1, G::identity());
      e1 = e1.op(&op);
    }
    assert_eq!(e1, G::identity());
  }

  #[test]
  fn test_macro_large() {
    type G = direct_product_type!(Dihedral<7>, Cyclic<11>, Cyclic<3>, Dihedral<5>);
    let mut e1: G = direct_product!(
      Dihedral::Rot(3),
      Cyclic::from_ord(4),
      Cyclic::from_ord(2),
      Dihedral::Rfl(0)
    );
    let op = e1.clone();

    // Should require 7 * 11 * 3 * 2 - 1 = 461 rotations to get to the identity.
    for _ in 0..461 {
      assert_ne!(e1, G::identity());
      e1 = e1.op(&op);
    }
    assert_eq!(e1, G::identity());
  }
}
