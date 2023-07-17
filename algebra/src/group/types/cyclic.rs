use std::fmt::Display;

use crate::{finite::Finite, group::Group, monoid::Monoid, ordinal::Ordinal, semigroup::Semigroup};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cyclic<const N: u16> {
  ord: u16,
}

impl<const N: u16> Finite for Cyclic<N> {
  const SIZE: usize = N as usize;
}

impl<const N: u16> Ordinal for Cyclic<N> {
  fn ord(&self) -> usize {
    self.ord as usize
  }

  fn from_ord(ord: usize) -> Self {
    Self { ord: ord as u16 }
  }
}

impl<const N: u16> Semigroup for Cyclic<N> {
  fn op(&self, other: &Self) -> Self {
    Self {
      ord: (self.ord + other.ord) % N,
    }
  }
}

impl<const N: u16> Monoid for Cyclic<N> {
  fn identity() -> Self {
    Self { ord: 0 }
  }
}

impl<const N: u16> Group for Cyclic<N> {
  fn inverse(&self) -> Self {
    Self {
      ord: (N - self.ord) % N,
    }
  }
}

impl<const N: u16> Display for Cyclic<N> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "r{}", self.ord)
  }
}

#[cfg(test)]
mod test {
  use super::*;

  fn for_each<const N: u16>() -> impl Iterator<Item = Cyclic<N>> {
    (0..(Cyclic::<N>::size())).map(|i| Cyclic::<N>::from_ord(i))
  }

  fn permute_all<const N: u16>() {
    for i in 0..N {
      let a: Cyclic<N> = Cyclic { ord: i };
      assert_eq!(a.ord(), i as usize);
      assert_eq!(Cyclic::from_ord(i as usize), a);

      for j in 0..N {
        let b: Cyclic<N> = Cyclic { ord: j };

        assert_eq!(a.op(&b), Cyclic { ord: (i + j) % N });
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
      assert_eq!(id.op(&el), el);
      assert_eq!(el.op(&id), el);
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
      assert_eq!(el.op(&inv), Cyclic::<N>::identity());
      assert_eq!(inv.op(&el), Cyclic::<N>::identity());
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
