use std::ops::Index;

use algebra::group::Group;
use const_random::const_random;

use crate::{
  groups::{D3, D6},
  hex_pos::{HexPos, HexPosOffset},
  tile_hash::{TileHash, C_MASK, V_MASK},
};

#[derive(Debug)]
pub struct HashTable<const N: usize, const N2: usize, G: Group> {
  table: [TileHash<G>; N2],
}

impl<const N: usize, const N2: usize, G: Group> HashTable<N, N2, G> {
  const fn center() -> HexPos {
    HexPos::new((N / 2) as u32, (N / 2) as u32)
  }

  /// Converts a `HexPos` to an ordinal, which is a unique mapping from valid
  /// `HexPos`s on the board to the range 0..N2.
  const fn hex_pos_ord(pos: &HexPos) -> usize {
    pos.x() as usize + (pos.y() as usize) * N
  }

  /// The inverse of `self.hex_pos_ord`.
  const fn ord_to_hex_pos(ord: usize) -> HexPos {
    HexPos::new((ord % N) as u32, (ord / N) as u32)
  }

  const fn in_bounds(pos: &HexPos) -> bool {
    (pos.x() as usize) < N && (pos.y() as usize) < N
  }

  /// Returns true if the hash for `pos` will be computed before `cur_idx`.
  const fn has_been_computed(pos: &HexPosOffset, cur_idx: usize) -> bool {
    let pos = pos.add_hex(&Self::center());
    let ord = Self::hex_pos_ord(&pos);
    ord < cur_idx && Self::in_bounds(&pos)
  }
}

impl<const N: usize, const N2: usize> HashTable<N, N2, D6> {
  /// Generates a hash table for boards with symmetry class C.
  pub const fn new_c() -> Self {
    let mut table = [TileHash::<D6>::uninitialized(); N2];

    let mut i = 0usize;
    'tile_loop: while i < N2 {
      let pos = Self::ord_to_hex_pos(i);

      // Normalize coordinates to the center.
      let pos = pos.sub_hex(&Self::center());

      let d6b = TileHash::<D6>::new(const_random!(u64) & C_MASK);

      if pos.eq_cnst(&HexPosOffset::origin()) {
        table[i] = d6b.make_invariant(&D6::Rot(1)).make_invariant(&D6::Rfl(0));
        i += 1;
        continue;
      }

      // Try the 5 rotational symmetries
      let mut s = pos;
      let mut op = D6::Rot(0);
      let mut r = 0;
      while r < 5 {
        s = s.apply_d6_c(&D6::Rot(1));
        // Accumulate the inverses of the operations we have been doing.
        op = op.const_op(&D6::Rot(5));

        if Self::has_been_computed(&s, i) {
          let ord = Self::hex_pos_ord(&s.add_hex(&Self::center()));
          let hash = table[ord];
          // Apply the accumulated group op to transform `s` back to `pos`.
          table[i] = hash.apply(&op);
          i += 1;
          continue 'tile_loop;
        }

        r += 1;
      }

      // Try the 6 reflective symmetries.
      op = D6::Rfl(0);
      s = pos.apply_d6_c(&op);
      let mut r = 0;
      while r < 6 {
        if s.eq_cnst(&pos) {
          // This tile is symmetric to itself under some reflection
          table[i] = d6b.make_invariant(&op);
          i += 1;
          continue 'tile_loop;
        }

        if Self::has_been_computed(&s, i) {
          let ord = Self::hex_pos_ord(&s.add_hex(&Self::center()));
          let hash = table[ord];
          // Apply the accumulated group op to transform `s` back to `pos`.
          table[i] = hash.apply(&op);
          i += 1;
          continue 'tile_loop;
        }

        s = s.apply_d6_c(&D6::Rot(1));
        // Accumulate the inverses of the operations we have been doing.
        op = op.const_op(&D6::Rot(5));
        r += 1;
      }

      // Otherwise, if the tile is not self-symmetric and has no symmetries that
      // have already been computed, use the randomly generated hash.
      table[i] = d6b;
      i += 1;
    }

    Self { table }
  }
}

impl<const N: usize, const N2: usize> HashTable<N, N2, D3> {
  /// Generates a hash table for boards with symmetry class V.
  pub const fn new_v() -> Self {
    let mut table = [TileHash::<D3>::uninitialized(); N2];

    let mut i = 0usize;
    'tile_loop: while i < N2 {
      let pos = Self::ord_to_hex_pos(i);

      // Normalize coordinates to the center.
      let pos = pos.sub_hex(&Self::center());

      let d3b = TileHash::<D3>::new(const_random!(u64) & V_MASK);

      // Try the 2 rotational symmetries
      let mut s = pos;
      let mut op = D3::Rot(0);
      let mut r = 0;
      while r < 2 {
        s = s.apply_d3_v(&D3::Rot(1));
        // Accumulate the inverses of the operations we have been doing.
        op = op.const_op(&D3::Rot(2));

        if Self::has_been_computed(&s, i) {
          let ord = Self::hex_pos_ord(&s.add_hex(&Self::center()));
          let hash = table[ord];
          // Apply the accumulated group op to transform `s` back to `pos`.
          table[i] = hash.apply(&op);
          i += 1;
          continue 'tile_loop;
        }

        r += 1;
      }

      // Try the 3 reflective symmetries.
      op = D3::Rfl(0);
      s = pos.apply_d3_v(&op);
      let mut r = 0;
      while r < 3 {
        if s.eq_cnst(&pos) {
          // This tile is symmetric to itself under some reflection
          table[i] = d3b.make_invariant(&op);
          i += 1;
          continue 'tile_loop;
        }

        if Self::has_been_computed(&s, i) {
          let ord = Self::hex_pos_ord(&s.add_hex(&Self::center()));
          let hash = table[ord];
          // Apply the accumulated group op to transform `s` back to `pos`.
          table[i] = hash.apply(&op);
          i += 1;
          continue 'tile_loop;
        }

        s = s.apply_d3_v(&D3::Rot(1));
        // Accumulate the inverses of the operations we have been doing.
        op = op.const_op(&D3::Rot(2));
        r += 1;
      }

      // Otherwise, if the tile is not self-symmetric and has no symmetries that
      // have already been computed, use the randomly generated hash.
      table[i] = d3b;
      i += 1;
    }

    Self { table }
  }
}

impl<const N: usize, const N2: usize, G: Group> Index<usize> for HashTable<N, N2, G> {
  type Output = TileHash<G>;

  fn index(&self, index: usize) -> &Self::Output {
    &self.table[index]
  }
}

#[cfg(test)]
mod test {
  use algebra::monoid::Monoid;

  use crate::{
    groups::{D3, D6},
    hash::HashTable,
  };

  type HD6 = HashTable<16, 256, D6>;
  type HD3 = HashTable<16, 256, D3>;

  #[test]
  fn test_d6_table() {
    const D6T: HD6 = HashTable::new_c();

    for i in 0..256 {
      let pos = HD6::ord_to_hex_pos(i) - HD6::center();

      let mut s = pos;
      let mut op = D6::identity();
      let hash = D6T[i];

      for _ in 0..5 {
        s = s.apply_d6_c(&D6::Rot(1));
        op = D6::Rot(1) * op;

        let symm_pos = s + HD6::center();
        if HD6::in_bounds(&symm_pos) {
          let ord = HD6::hex_pos_ord(&symm_pos);
          let symm_hash = D6T[ord];

          assert_eq!(
            symm_hash,
            hash.apply(&op),
            "Expected equality of:\n  left: {}\n right: {}",
            symm_hash,
            hash.apply(&op)
          );
        }
      }

      op = D6::Rfl(0);
      s = pos.apply_d6_c(&op);
      for _ in 0..6 {
        let symm_pos = s + HD6::center();
        if HD6::in_bounds(&symm_pos) {
          let ord = HD6::hex_pos_ord(&symm_pos);
          let symm_hash = D6T[ord];

          assert_eq!(
            symm_hash,
            hash.apply(&op),
            "Expected equality of:\n  left: {}\n right: {}",
            symm_hash,
            hash.apply(&op)
          );
        }

        s = s.apply_d6_c(&D6::Rot(1));
        op = D6::Rot(1) * op;
      }
    }
  }

  #[test]
  fn test_d3_table() {
    const D3T: HD3 = HashTable::new_v();

    for i in 0..256 {
      let pos = HD3::ord_to_hex_pos(i) - HD3::center();

      let mut s = pos;
      let mut op = D3::identity();
      let hash = D3T[i];

      for _ in 0..2 {
        s = s.apply_d3_v(&D3::Rot(1));
        op = D3::Rot(1) * op;

        let symm_pos = s + HD3::center();
        if HD3::in_bounds(&symm_pos) {
          let ord = HD3::hex_pos_ord(&symm_pos);
          let symm_hash = D3T[ord];

          assert_eq!(
            symm_hash,
            hash.apply(&op),
            "Expected equality of:\n  left: {}\n right: {}",
            symm_hash,
            hash.apply(&op)
          );
        }
      }

      op = D3::Rfl(0);
      s = pos.apply_d3_v(&op);
      for _ in 0..3 {
        let symm_pos = s + HD3::center();
        if HD3::in_bounds(&symm_pos) {
          println!("Checking {} for {}", op, pos);
          let ord = HD3::hex_pos_ord(&symm_pos);
          let symm_hash = D3T[ord];

          assert_eq!(
            symm_hash,
            hash.apply(&op),
            "Expected equality of:\n  left: {}\n right: {} under {}",
            symm_hash,
            hash.apply(&op),
            op
          );
        }

        s = s.apply_d3_v(&D3::Rot(1));
        op = D3::Rot(1) * op;
      }
    }
  }
}
