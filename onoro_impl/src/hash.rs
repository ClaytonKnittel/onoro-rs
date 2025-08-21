use std::ops::{Index, IndexMut};

use algebra::{
  finite::Finite,
  group::{Cyclic, Group, Trivial},
};
use const_random::const_random;

use onoro::{
  Onoro,
  groups::{C2, D3, D6, K4, SymmetryClass},
  hex_pos::{HexPos, HexPosOffset},
};

use crate::{
  OnoroImpl,
  canonicalize::BoardSymmetryState,
  const_rand::Xoroshiro128,
  tile_hash::{C_MASK, E_MASK, TileHash, V_MASK},
};

#[derive(Debug)]
struct ConstTable<T, const N: usize> {
  table: [[T; N]; N],
}

impl<T, const N: usize> ConstTable<T, N> {
  const fn filled(default: T) -> Self
  where
    T: Copy,
  {
    Self {
      table: [[default; N]; N],
    }
  }

  const fn get(&self, index: usize) -> &T {
    &self.table[index / N][index % N]
  }

  const fn get_mut(&mut self, index: usize) -> &mut T {
    &mut self.table[index / N][index % N]
  }
}

impl<T, const N: usize> Index<usize> for ConstTable<T, N> {
  type Output = T;

  fn index(&self, index: usize) -> &Self::Output {
    self.get(index)
  }
}

impl<T, const N: usize> IndexMut<usize> for ConstTable<T, N> {
  fn index_mut(&mut self, index: usize) -> &mut Self::Output {
    self.get_mut(index)
  }
}

#[derive(Debug)]
pub struct HashTable<const N: usize, G: Group> {
  table: ConstTable<TileHash<G>, N>,
}

impl<const N: usize, G: Group> HashTable<N, G> {
  /// Computes the hash of a game state on a given hash table.
  pub fn hash<const ONORO_N: usize>(
    &self,
    onoro: &OnoroImpl<ONORO_N>,
    symm_state: &BoardSymmetryState,
  ) -> u64 {
    let origin = onoro.origin(symm_state);
    onoro.pawns().fold(0u64, |hash, pawn| {
      // The position of the pawn relative to the rotation-invariant origin of
      // the board.
      let pos = HexPos::from(pawn.pos) - origin;
      // The position of the pawn normalized to align board states on all
      // symmetry axes which the board isn't possibly symmetric about itself.
      let normalized_pos = pos.apply_d6_c(&symm_state.op);
      // The position of the pawn in table space, relative to the center of the
      // hash table.
      let table_pos = normalized_pos + Self::center();
      // The index of the tile this pawn is on.
      let table_idx = Self::hex_pos_ord(&table_pos);
      let tile_hash = &self[table_idx];

      let pawn_hash = if pawn.color == onoro.player_color() {
        tile_hash.cur_player_hash()
      } else {
        tile_hash.other_player_hash()
      };

      // Zobrist hashing accumulates all hashes with xor.
      hash ^ pawn_hash
    })
  }

  const fn center() -> HexPos {
    HexPos::new((N / 2) as u32, (N / 2) as u32)
  }

  /// Converts a `HexPos` to an ordinal, which is a unique mapping from valid
  /// `HexPos`s on the board to the range 0..N*N.
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
  const fn has_been_computed(pos: &HexPos, cur_idx: usize) -> bool {
    let ord = Self::hex_pos_ord(pos);
    ord < cur_idx && Self::in_bounds(pos)
  }
}

impl<const N: usize> HashTable<N, D6> {
  /// Generates a hash table for boards with symmetry class C.
  pub const fn new_c() -> Self {
    let mut table = ConstTable::filled(TileHash::<D6>::uninitialized());
    let mut rng = Xoroshiro128::from_seed([const_random!(u64), const_random!(u64)]);

    let mut i = 0usize;
    'tile_loop: while i < N * N {
      let pos = Self::ord_to_hex_pos(i);

      // Normalize coordinates to the center.
      let pos = pos.sub_hex(&Self::center());

      let (new_rng, h_cur) = rng.next_u64();
      let (new_rng, h_oth) = new_rng.next_u64();
      rng = new_rng;
      let d6b = TileHash::<D6>::new(h_cur & C_MASK, h_oth & C_MASK);

      if pos.eq_cnst(&HexPosOffset::origin()) {
        *table.get_mut(i) = d6b.make_invariant(&D6::Rot(1)).make_invariant(&D6::Rfl(0));
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

        let symm_pos = s.add_hex(&Self::center());
        if Self::has_been_computed(&symm_pos, i) {
          let ord = Self::hex_pos_ord(&symm_pos);
          let hash = table.get(ord);
          // Apply the accumulated group op to transform `s` back to `pos`.
          *table.get_mut(i) = hash.apply(&op);
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
          *table.get_mut(i) = d6b.make_invariant(&op);
          i += 1;
          continue 'tile_loop;
        }

        let symm_pos = s.add_hex(&Self::center());
        if Self::has_been_computed(&symm_pos, i) {
          let ord = Self::hex_pos_ord(&symm_pos);
          let hash = table.get(ord);
          // Apply the accumulated group op to transform `s` back to `pos`.
          *table.get_mut(i) = hash.apply(&op);
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
      *table.get_mut(i) = d6b;
      i += 1;
    }

    Self { table }
  }
}

impl<const N: usize> HashTable<N, D3> {
  /// Generates a hash table for boards with symmetry class V.
  pub const fn new_v() -> Self {
    let mut table = ConstTable::filled(TileHash::<D3>::uninitialized());
    let mut rng = Xoroshiro128::from_seed([const_random!(u64), const_random!(u64)]);

    let mut i = 0usize;
    'tile_loop: while i < N * N {
      let pos = Self::ord_to_hex_pos(i);

      // Normalize coordinates to the center.
      let pos = pos.sub_hex(&Self::center());

      let (new_rng, h_cur) = rng.next_u64();
      let (new_rng, h_oth) = new_rng.next_u64();
      rng = new_rng;
      let d3b = TileHash::<D3>::new(h_cur & V_MASK, h_oth & V_MASK);

      // Try the 2 rotational symmetries
      let mut s = pos;
      let mut op = D3::Rot(0);
      let mut r = 0;
      while r < 2 {
        s = s.apply_d3_v(&D3::Rot(1));
        // Accumulate the inverses of the operations we have been doing.
        op = op.const_op(&D3::Rot(2));

        let symm_pos = s.add_hex(&Self::center());
        if Self::has_been_computed(&symm_pos, i) {
          let ord = Self::hex_pos_ord(&symm_pos);
          let hash = table.get(ord);
          // Apply the accumulated group op to transform `s` back to `pos`.
          *table.get_mut(i) = hash.apply(&op);
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
          *table.get_mut(i) = d3b.make_invariant(&op);
          i += 1;
          continue 'tile_loop;
        }

        let symm_pos = s.add_hex(&Self::center());
        if Self::has_been_computed(&symm_pos, i) {
          let ord = Self::hex_pos_ord(&symm_pos);
          let hash = table.get(ord);
          // Apply the accumulated group op to transform `s` back to `pos`.
          *table.get_mut(i) = hash.apply(&op);
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
      *table.get_mut(i) = d3b;
      i += 1;
    }

    Self { table }
  }
}

impl<const N: usize> HashTable<N, K4> {
  /// Generates a hash table for boards with symmetry class E.
  pub const fn new_e() -> Self {
    let mut table = ConstTable::filled(TileHash::<K4>::uninitialized());
    let mut rng = Xoroshiro128::from_seed([const_random!(u64), const_random!(u64)]);

    let mut i = 0usize;
    'tile_loop: while i < N * N {
      let pos = Self::ord_to_hex_pos(i);

      // Normalize coordinates to the center.
      let pos = pos.sub_hex(&Self::center());

      let (new_rng, h_cur) = rng.next_u64();
      let (new_rng, h_oth) = new_rng.next_u64();
      rng = new_rng;
      let k4b = TileHash::<K4>::new(h_cur & E_MASK, h_oth & E_MASK);

      // Check the 3 symmetries for existing hashes.
      let mut op_ord = 1;
      while op_ord < K4::SIZE {
        let op = K4::const_from_ord(op_ord);
        op_ord += 1;

        let symm_pos = pos.apply_k4_e(&op).add_hex(&Self::center());
        if Self::has_been_computed(&symm_pos, i) {
          let ord = Self::hex_pos_ord(&symm_pos);
          let hash = table.get(ord);
          // Apply the accumulated group op to transform `s` back to `pos`.
          *table.get_mut(i) = hash.apply(&op);
          i += 1;
          continue 'tile_loop;
        }
      }

      // Check the 3 symmetries for self-mapping.
      let mut op_ord = 1;
      while op_ord < K4::SIZE {
        let op = K4::const_from_ord(op_ord);
        op_ord += 1;

        let s = pos.apply_k4_e(&op);
        if s.eq_cnst(&pos) {
          // This tile is symmetric to itself under some reflection
          *table.get_mut(i) = k4b.make_invariant(&op);
          i += 1;
          continue 'tile_loop;
        }
      }

      // Otherwise, if the tile is not self-symmetric and has no symmetries that
      // have already been computed, use the randomly generated hash.
      *table.get_mut(i) = k4b;
      i += 1;
    }

    Self { table }
  }
}

impl<const N: usize> HashTable<N, C2> {
  const fn new_c2(symm_class: SymmetryClass, mut rng: Xoroshiro128) -> Self {
    let mut table = ConstTable::filled(TileHash::<C2>::uninitialized());

    let mut i = 0usize;
    while i < N * N {
      let pos = Self::ord_to_hex_pos(i);

      // Normalize coordinates to the center.
      let pos = pos.sub_hex(&Self::center());

      let (new_rng, h_cur) = rng.next_u64();
      let (new_rng, h_oth) = new_rng.next_u64();
      rng = new_rng;
      let c2b = TileHash::<C2>::new(h_cur, h_oth);

      // check the symmetry for existing hashes/self-mapping
      let s = match symm_class {
        SymmetryClass::CV => pos.apply_c2_cv(&Cyclic(1)),
        SymmetryClass::CE => pos.apply_c2_ce(&Cyclic(1)),
        SymmetryClass::EV => pos.apply_c2_ev(&Cyclic(1)),
        _ => panic!("Can only generate C2 hash table from symm_class CV, CE, or EV."),
      };
      let symm_pos = s.add_hex(&Self::center());
      if s.eq_cnst(&pos) {
        *table.get_mut(i) = c2b.make_invariant(&Cyclic(1));
      } else if Self::has_been_computed(&symm_pos, i) {
        let ord = Self::hex_pos_ord(&symm_pos);
        let hash = table.get(ord);
        // Apply the accumulated group op to transform `s` back to `pos`.
        *table.get_mut(i) = hash.apply(&Cyclic(1));
      } else {
        *table.get_mut(i) = c2b;
      }

      i += 1;
    }

    Self { table }
  }

  pub const fn new_cv() -> Self {
    let rng = Xoroshiro128::from_seed([const_random!(u64), const_random!(u64)]);
    Self::new_c2(SymmetryClass::CV, rng)
  }

  pub const fn new_ce() -> Self {
    let rng = Xoroshiro128::from_seed([const_random!(u64), const_random!(u64)]);
    Self::new_c2(SymmetryClass::CE, rng)
  }

  pub const fn new_ev() -> Self {
    let rng = Xoroshiro128::from_seed([const_random!(u64), const_random!(u64)]);
    Self::new_c2(SymmetryClass::EV, rng)
  }
}

impl<const N: usize> HashTable<N, Trivial> {
  /// Generates a hash table for boards with symmetry class E.
  pub const fn new_trivial() -> Self {
    let mut table = ConstTable::filled(TileHash::<Trivial>::uninitialized());
    let mut rng = Xoroshiro128::from_seed([const_random!(u64), const_random!(u64)]);

    let mut i = 0usize;
    while i < N * N {
      let (new_rng, h_cur) = rng.next_u64();
      let (new_rng, h_oth) = new_rng.next_u64();
      rng = new_rng;
      *table.get_mut(i) = TileHash::<Trivial>::new(h_cur, h_oth);
      i += 1;
    }

    Self { table }
  }
}

impl<const N: usize, G: Group> Index<usize> for HashTable<N, G> {
  type Output = TileHash<G>;

  fn index(&self, index: usize) -> &Self::Output {
    &self.table[index]
  }
}

#[cfg(test)]
mod test {
  use algebra::{finite::Finite, group::Cyclic, monoid::Monoid};
  use onoro::groups::{C2, D3, D6, K4};

  use crate::hash::HashTable;

  type HD6 = HashTable<16, D6>;
  type HD3 = HashTable<16, D3>;
  type HK4 = HashTable<16, K4>;
  type HC2 = HashTable<16, C2>;

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

          assert_eq!(symm_hash, hash.apply(&op));
        }
      }

      op = D3::Rfl(0);
      s = pos.apply_d3_v(&op);
      for _ in 0..3 {
        let symm_pos = s + HD3::center();
        if HD3::in_bounds(&symm_pos) {
          let ord = HD3::hex_pos_ord(&symm_pos);
          let symm_hash = D3T[ord];

          assert_eq!(symm_hash, hash.apply(&op));
        }

        s = s.apply_d3_v(&D3::Rot(1));
        op = D3::Rot(1) * op;
      }
    }
  }

  #[test]
  fn test_k4_table() {
    const K4T: HK4 = HashTable::new_e();

    for i in 0..256 {
      let pos = HK4::ord_to_hex_pos(i) - HK4::center();
      let hash = K4T[i];

      let mut op_ord = 1;
      while op_ord < K4::SIZE {
        let op = K4::const_from_ord(op_ord);
        op_ord += 1;

        let symm_pos = pos.apply_k4_e(&op) + HK4::center();
        if HK4::in_bounds(&symm_pos) {
          let ord = HD3::hex_pos_ord(&symm_pos);
          let symm_hash = K4T[ord];

          assert_eq!(symm_hash, hash.apply(&op));
        }
      }
    }
  }

  #[test]
  fn test_c2_cv_table() {
    const C2T: HC2 = HashTable::new_cv();

    for i in 0..256 {
      let pos = HC2::ord_to_hex_pos(i) - HC2::center();
      let hash = C2T[i];

      let symm_pos = pos.apply_c2_cv(&Cyclic(1)) + HC2::center();
      if HC2::in_bounds(&symm_pos) {
        let ord = HC2::hex_pos_ord(&symm_pos);
        let symm_hash = C2T[ord];

        assert_eq!(symm_hash, hash.apply(&Cyclic(1)));
      }
    }
  }

  #[test]
  fn test_c2_ce_table() {
    const C2T: HC2 = HashTable::new_ce();

    for i in 0..256 {
      let pos = HC2::ord_to_hex_pos(i) - HC2::center();
      let hash = C2T[i];

      let symm_pos = pos.apply_c2_ce(&Cyclic(1)) + HC2::center();
      if HC2::in_bounds(&symm_pos) {
        let ord = HC2::hex_pos_ord(&symm_pos);
        let symm_hash = C2T[ord];

        assert_eq!(symm_hash, hash.apply(&Cyclic(1)));
      }
    }
  }

  #[test]
  fn test_c2_ev_table() {
    const C2T: HC2 = HashTable::new_ev();

    for i in 0..256 {
      let pos = HC2::ord_to_hex_pos(i) - HC2::center();
      let hash = C2T[i];

      let symm_pos = pos.apply_c2_ev(&Cyclic(1)) + HC2::center();
      if HC2::in_bounds(&symm_pos) {
        let ord = HC2::hex_pos_ord(&symm_pos);
        let symm_hash = C2T[ord];

        assert_eq!(symm_hash, hash.apply(&Cyclic(1)));
      }
    }
  }
}
