use std::ops::Index;

use algebra::group::Group;
use const_random::const_random;

use crate::{
  groups::D6,
  hex_pos::{HexPos, HexPosOffset},
  tile_hash::{TileHash, C_MASK},
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

      let mut s = pos.clone_const();
      let mut op = D6::Rot(0);

      // Try the other 5 rotational symmetries
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

impl<const N: usize, const N2: usize, G: Group> Index<usize> for HashTable<N, N2, G> {
  type Output = TileHash<G>;

  fn index(&self, index: usize) -> &Self::Output {
    &self.table[index]
  }
}

#[cfg(test)]
mod test {
  use algebra::monoid::Monoid;

  use crate::{groups::D6, hash::HashTable};

  type HD6 = HashTable<16, 256, D6>;

  #[test]
  fn test_d6_table() {
    const d6t: HD6 = HashTable::new_c();

    for i in 0..256 {
      let pos = HD6::ord_to_hex_pos(i) - HD6::center();

      let mut s = pos;
      let mut op = D6::identity();
      let hash = d6t[i];

      for _ in 0..5 {
        s = s.apply_d6_c(&D6::Rot(1));
        op = D6::Rot(1) * op;

        let symm_pos = s + HD6::center();
        if HD6::in_bounds(&symm_pos) {
          let ord = HD6::hex_pos_ord(&symm_pos);
          let symm_hash = d6t[ord];

          assert_eq!(
            symm_hash,
            hash.apply(&op),
            "Expected equality of:\n  left: {:x?}\n right: {:x?}",
            symm_hash,
            hash.apply(&op)
          );
        }
      }
    }
  }
}
