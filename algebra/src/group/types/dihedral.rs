use std::{fmt::Display, ops::Mul};

use crate::{finite::Finite, group::Group, monoid::Monoid, ordinal::Ordinal, semigroup::Semigroup};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dihedral<const N: u16> {
  Rot(u16),
  Rfl(u16),
}

impl<const N: u16> Dihedral<N> {
  /// While const traits are nightly-only, define const versions of the trait
  /// impls manually.
  pub const fn const_identity() -> Self {
    Self::Rot(0)
  }

  pub const fn const_ord(self) -> usize {
    match self {
      Self::Rot(i) => i as usize,
      Self::Rfl(i) => (N + i) as usize,
    }
  }

  pub const fn const_op(&self, rhs: &Self) -> Self {
    match (*self, *rhs) {
      (Self::Rot(i), Self::Rot(j)) => Self::Rot((i + j) % N),
      (Self::Rot(i), Self::Rfl(j)) => Self::Rfl((i + j) % N),
      (Self::Rfl(i), Self::Rot(j)) => Self::Rfl((N + i - j) % N),
      (Self::Rfl(i), Self::Rfl(j)) => Self::Rot((N + i - j) % N),
    }
  }
}

impl<const N: u16> Mul for Dihedral<N> {
  type Output = Self;

  fn mul(self, rhs: Self) -> Self::Output {
    self.const_op(&rhs)
  }
}

impl<const N: u16> Finite for Dihedral<N> {
  const SIZE: usize = 2 * (N as usize);
}

impl<const N: u16> Ordinal for Dihedral<N> {
  fn ord(&self) -> usize {
    Self::const_ord(*self)
  }

  fn from_ord(ord: usize) -> Self {
    if ord < N as usize {
      Self::Rot(ord as u16)
    } else {
      debug_assert!(ord < 2 * N as usize);
      Self::Rfl((ord - N as usize) as u16)
    }
  }
}

impl<const N: u16> Semigroup for Dihedral<N> {
  fn for_each() -> impl Iterator<Item = Self> {
    (0..Self::SIZE).map(Self::from_ord)
  }
}

impl<const N: u16> Monoid for Dihedral<N> {
  fn identity() -> Self {
    Self::Rot(0)
  }
}

impl<const N: u16> Group for Dihedral<N> {
  fn inverse(&self) -> Self {
    match self {
      Self::Rot(i) => Self::Rot((N - i) % N),
      Self::Rfl(i) => Self::Rfl(*i),
    }
  }
}

impl<const N: u16> Display for Dihedral<N> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Rot(i) => write!(f, "r{i}"),
      Self::Rfl(i) => write!(f, "s{i}"),
    }
  }
}

#[cfg(test)]
mod test {
  use super::*;

  fn for_each<const N: u16>() -> impl Iterator<Item = Dihedral<N>> {
    (0..(Dihedral::<N>::size())).map(Dihedral::<N>::from_ord)
  }

  fn permute_all<const N: u16>() {
    for i in 0..(2 * N) {
      let a: Dihedral<N> = if i < N {
        Dihedral::Rot(i)
      } else {
        Dihedral::Rfl(i - N)
      };
      assert_eq!(a.ord(), i as usize);
      assert_eq!(Dihedral::from_ord(i as usize), a);

      for j in 0..(2 * N) {
        let b = if j < N {
          Dihedral::Rot(j)
        } else {
          Dihedral::Rfl(j - N)
        };

        if i < N {
          if j < N {
            // r_i * r_j = r_i+j
            assert_eq!(a * b, Dihedral::Rot((i + j) % N));
          } else {
            // r_i * s_j = s_i+j
            assert_eq!(a * b, Dihedral::Rfl((i + j) % N));
          }
        } else {
          #[warn(clippy::collapsible_else_if)]
          if j < N {
            // s_i * r_j = s_i-j
            assert_eq!(a * b, Dihedral::Rfl((N + i - j) % N));
          } else {
            // s_i * s_j = r_i-j
            assert_eq!(a * b, Dihedral::Rot((N + i - j) % N));
          }
        }
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
    let id = Dihedral::<6>::identity();

    for el in for_each::<6>() {
      assert_eq!(id * el, el);
      assert_eq!(el * id, el);
    }
  }

  fn test_ords<const N: u16>() {
    let mut seen = vec![false; (2 * N) as usize];
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
      assert_eq!(el * inv, Dihedral::<N>::identity());
      assert_eq!(inv * el, Dihedral::<N>::identity());
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
