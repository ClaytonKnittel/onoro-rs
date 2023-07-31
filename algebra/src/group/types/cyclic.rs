use std::{fmt::Display, ops::Mul};

use crate::{finite::Finite, group::Group, monoid::Monoid, ordinal::Ordinal, semigroup::Semigroup};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cyclic<const N: u16>(pub u16);

impl<const N: u16> Cyclic<N> {
  pub const fn const_op(&self, rhs: &Self) -> Self {
    Self((self.0 + rhs.0) % N)
  }

  pub const fn const_from_ord(ord: usize) -> Self {
    Self(ord as u16)
  }

  pub fn for_each() -> impl Iterator<Item = Self> {
    (0..Self::SIZE).map(Self::from_ord)
  }
}

impl<const N: u16> Mul for Cyclic<N> {
  type Output = Self;

  fn mul(self, rhs: Self) -> Self::Output {
    self.const_op(&rhs)
  }
}

impl<const N: u16> Finite for Cyclic<N> {
  const SIZE: usize = N as usize;
}

impl<const N: u16> Ordinal for Cyclic<N> {
  fn ord(&self) -> usize {
    self.0 as usize
  }

  fn from_ord(ord: usize) -> Self {
    Self(ord as u16)
  }
}

impl<const N: u16> Semigroup for Cyclic<N> {}

impl<const N: u16> Monoid for Cyclic<N> {
  fn identity() -> Self {
    Self(0)
  }
}

impl<const N: u16> Group for Cyclic<N> {
  fn inverse(&self) -> Self {
    Self((N - self.0) % N)
  }
}

impl<const N: u16> Display for Cyclic<N> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "r{}", self.0)
  }
}

#[cfg(test)]
mod test {
  use super::*;

  fn for_each<const N: u16>() -> impl Iterator<Item = Cyclic<N>> {
    (0..(Cyclic::<N>::size())).map(Cyclic::<N>::from_ord)
  }

  fn permute_all<const N: u16>() {
    for i in 0..N {
      let a: Cyclic<N> = Cyclic(i);
      assert_eq!(a.ord(), i as usize);
      assert_eq!(Cyclic::from_ord(i as usize), a);

      for j in 0..N {
        let b: Cyclic<N> = Cyclic(j);

        assert_eq!(a * b, Cyclic((i + j) % N));
      }
    }
  }

  #[test]
  fn test_ops() {
    permute_all::<1>();
    permute_all::<2>();
    permute_all::<3>();
    permute_all::<4>();
    permute_all::<5>();
    permute_all::<6>();
  }

  #[test]
  fn test_identity() {
    let id = Cyclic::<6>::identity();

    for el in for_each::<6>() {
      assert_eq!(id * el, el);
      assert_eq!(el * id, el);
    }
  }

  fn test_ords<const N: u16>() {
    let mut seen = vec![false; N as usize];
    for el in for_each::<N>() {
      assert!(!seen[el.ord()]);
      seen[el.ord()] = true;
    }
    assert!(seen.iter().all(|seen| *seen))
  }

  #[test]
  fn test_ord() {
    test_ords::<1>();
    test_ords::<2>();
    test_ords::<3>();
    test_ords::<4>();
    test_ords::<5>();
    test_ords::<6>();
  }

  fn test_eqs<const N: u16>() {
    for el1 in for_each::<N>() {
      for el2 in for_each::<N>() {
        if el1.ord() == el2.ord() {
          assert!(el1 == el2);
        } else {
          assert!(el1 != el2);
        }
      }
    }
  }

  #[test]
  fn test_eq() {
    test_eqs::<1>();
    test_eqs::<2>();
    test_eqs::<3>();
    test_eqs::<4>();
    test_eqs::<5>();
    test_eqs::<6>();
  }

  fn test_invs<const N: u16>() {
    for el in for_each::<N>() {
      let inv = el.inverse();
      assert_eq!(el * inv, Cyclic::<N>::identity());
      assert_eq!(inv * el, Cyclic::<N>::identity());
    }
  }

  #[test]
  fn test_inv() {
    test_invs::<1>();
    test_invs::<2>();
    test_invs::<3>();
    test_invs::<4>();
    test_invs::<5>();
    test_invs::<6>();
  }
}
