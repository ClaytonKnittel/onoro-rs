use crate::{finite::Finite, group::Group, monoid::Monoid, ordinal::Ordinal, semigroup::Semigroup};

#[derive(Clone, Debug, PartialEq, Eq)]
struct DirectProduct<L, R> {
  left: L,
  right: R,
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
