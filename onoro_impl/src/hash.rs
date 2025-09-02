use std::ops::Index;

use algebra::{
  finite::Finite,
  group::{Cyclic, Group, Trivial},
  ordinal::Ordinal,
};
use const_random::const_random;

#[cfg(target_feature = "sse4.1")]
use onoro::PawnColor;
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
#[cfg(target_feature = "sse4.1")]
use crate::{PackedIdx, pawn_list::PawnList8};

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
    debug_assert!(index < N * N);
    let result = unsafe { &*(self.table.as_ptr() as *const T).add(index) };
    debug_assert_eq!(self.get(index) as *const _, result as *const _);
    result
  }
}

#[derive(Debug)]
pub struct HashTable<const N: usize, G: Group> {
  cur_table: ConstTable<TileHash<G>, N>,
  other_table: ConstTable<TileHash<G>, N>,
}

impl<const N: usize, G: Group> HashTable<N, G> {
  /// Computes the hash of a game state on a given hash table.
  pub fn hash<const ONORO_N: usize>(
    &self,
    onoro: &OnoroImpl<ONORO_N>,
    symm_state: &BoardSymmetryState,
  ) -> u64 {
    debug_assert!(ONORO_N <= N);

    #[cfg(target_feature = "sse4.1")]
    if const { ONORO_N == 16 } {
      return self.hash_fast(onoro, symm_state);
    }

    let origin = onoro.origin(symm_state);
    onoro.pawns().fold(0u64, |hash, pawn| {
      // The position of the pawn relative to the rotation-invariant origin of
      // the board.
      let pos = HexPos::from(pawn.pos) - origin;
      // The position of the pawn normalized to align board states on all
      // symmetry axes which the board isn't possibly symmetric about itself.
      let normalized_pos = pos.apply_d6_c(&D6::from_ord(symm_state.op_ord()));
      // The position of the pawn in table space, relative to the center of the
      // hash table.
      let table_pos = normalized_pos + Self::center();
      // The index of the tile this pawn is on.
      let table_idx = Self::hex_pos_ord(&table_pos);

      let pawn_hash = if pawn.color == onoro.player_color() {
        self.cur_table[table_idx].hash()
      } else {
        self.other_table[table_idx].hash()
      };

      // Zobrist hashing accumulates all hashes with xor.
      hash ^ pawn_hash
    })
  }

  /// Looks up the hash of each pawn in the list and returns their xor sum.
  #[cfg(target_feature = "sse4.1")]
  fn fold_hashes(pawns: PawnList8, table: &ConstTable<TileHash<G>, N>) -> u64 {
    pawns
      .pawn_indices::<N>(Self::center())
      .map(|cur_pawn_idx| table[cur_pawn_idx].hash())
      .fold(0, |acc, hash| acc ^ hash)
  }

  #[cfg(target_feature = "sse4.1")]
  fn hash_fast<const ONORO_N: usize>(
    &self,
    onoro: &OnoroImpl<ONORO_N>,
    symm_state: &BoardSymmetryState,
  ) -> u64 {
    let origin = onoro.origin(symm_state);

    let pawn_poses: &[PackedIdx; 16] =
      unsafe { (onoro.pawn_poses() as &[_]).try_into().unwrap_unchecked() };
    let black_pawns = PawnList8::extract_black_pawns(pawn_poses, origin);
    let white_pawns = PawnList8::extract_white_pawns(pawn_poses, origin);

    let black_pawns = black_pawns.apply_d6_c(symm_state.op_ord());
    let white_pawns = white_pawns.apply_d6_c(symm_state.op_ord());

    let (cur_pawns, other_pawns) = match onoro.player_color() {
      PawnColor::Black => (black_pawns, white_pawns),
      PawnColor::White => (white_pawns, black_pawns),
    };

    Self::fold_hashes(cur_pawns, &self.cur_table)
      ^ Self::fold_hashes(other_pawns, &self.other_table)
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
  const fn make_c() -> ConstTable<TileHash<D6>, N> {
    let mut table = ConstTable::filled(TileHash::<D6>::uninitialized());
    let mut rng = Xoroshiro128::from_seed([const_random!(u64), const_random!(u64)]);

    let mut i = 0usize;
    'tile_loop: while i < N * N {
      if i == 0 {
        *table.get_mut(i) = TileHash::<D6>::new(0);
        i += 1;
        continue;
      }

      let pos = Self::ord_to_hex_pos(i);

      // Normalize coordinates to the center.
      let pos = pos.sub_hex(&Self::center());

      let (new_rng, h) = rng.next_u64();
      rng = new_rng;
      let d6b = TileHash::<D6>::new(h & C_MASK);

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

    table
  }

  /// Generates a hash table for boards with symmetry class C.
  pub const fn new_c() -> Self {
    Self {
      cur_table: Self::make_c(),
      other_table: Self::make_c(),
    }
  }
}

impl<const N: usize> HashTable<N, D3> {
  const fn make_v() -> ConstTable<TileHash<D3>, N> {
    let mut table = ConstTable::filled(TileHash::<D3>::uninitialized());
    let mut rng = Xoroshiro128::from_seed([const_random!(u64), const_random!(u64)]);

    let mut i = 0usize;
    'tile_loop: while i < N * N {
      if i == 0 {
        *table.get_mut(i) = TileHash::<D3>::new(0);
        i += 1;
        continue;
      }

      let pos = Self::ord_to_hex_pos(i);

      // Normalize coordinates to the center.
      let pos = pos.sub_hex(&Self::center());

      let (new_rng, h) = rng.next_u64();
      rng = new_rng;
      let d3b = TileHash::<D3>::new(h & V_MASK);

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

    table
  }

  /// Generates a hash table for boards with symmetry class V.
  pub const fn new_v() -> Self {
    Self {
      cur_table: Self::make_v(),
      other_table: Self::make_v(),
    }
  }
}

impl<const N: usize> HashTable<N, K4> {
  const fn make_e() -> ConstTable<TileHash<K4>, N> {
    let mut table = ConstTable::filled(TileHash::<K4>::uninitialized());
    let mut rng = Xoroshiro128::from_seed([const_random!(u64), const_random!(u64)]);

    let mut i = 0usize;
    'tile_loop: while i < N * N {
      if i == 0 {
        *table.get_mut(i) = TileHash::<K4>::new(0);
        i += 1;
        continue;
      }

      let pos = Self::ord_to_hex_pos(i);

      // Normalize coordinates to the center.
      let pos = pos.sub_hex(&Self::center());

      let (new_rng, h) = rng.next_u64();
      rng = new_rng;
      let k4b = TileHash::<K4>::new(h & E_MASK);

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

    table
  }

  /// Generates a hash table for boards with symmetry class E.
  pub const fn new_e() -> Self {
    Self {
      cur_table: Self::make_e(),
      other_table: Self::make_e(),
    }
  }
}

impl<const N: usize> HashTable<N, C2> {
  const fn make_c2(
    symm_class: SymmetryClass,
    mut rng: Xoroshiro128,
  ) -> ConstTable<TileHash<Cyclic<2>>, N> {
    let mut table = ConstTable::filled(TileHash::<C2>::uninitialized());

    let mut i = 0usize;
    while i < N * N {
      if i == 0 {
        *table.get_mut(i) = TileHash::<C2>::new(0);
        i += 1;
        continue;
      }

      let pos = Self::ord_to_hex_pos(i);

      // Normalize coordinates to the center.
      let pos = pos.sub_hex(&Self::center());

      let (new_rng, h) = rng.next_u64();
      rng = new_rng;
      let c2b = TileHash::<C2>::new(h);

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

    table
  }

  pub const fn new_cv() -> Self {
    let rng = Xoroshiro128::from_seed([const_random!(u64), const_random!(u64)]);
    Self {
      cur_table: Self::make_c2(SymmetryClass::CV, rng),
      other_table: Self::make_c2(SymmetryClass::CV, rng),
    }
  }

  pub const fn new_ce() -> Self {
    let rng = Xoroshiro128::from_seed([const_random!(u64), const_random!(u64)]);
    Self {
      cur_table: Self::make_c2(SymmetryClass::CE, rng),
      other_table: Self::make_c2(SymmetryClass::CE, rng),
    }
  }

  pub const fn new_ev() -> Self {
    let rng = Xoroshiro128::from_seed([const_random!(u64), const_random!(u64)]);
    Self {
      cur_table: Self::make_c2(SymmetryClass::EV, rng),
      other_table: Self::make_c2(SymmetryClass::EV, rng),
    }
  }
}

impl<const N: usize> HashTable<N, Trivial> {
  const fn make_trivial() -> ConstTable<TileHash<Trivial>, N> {
    let mut table = ConstTable::filled(TileHash::<Trivial>::uninitialized());
    let mut rng = Xoroshiro128::from_seed([const_random!(u64), const_random!(u64)]);

    let mut i = 0usize;
    while i < N * N {
      if i == 0 {
        *table.get_mut(i) = TileHash::<Trivial>::new(0);
        i += 1;
        continue;
      }

      let (new_rng, h) = rng.next_u64();
      rng = new_rng;
      *table.get_mut(i) = TileHash::<Trivial>::new(h);
      i += 1;
    }

    table
  }

  /// Generates a hash table for boards with trivial symmetry class.
  pub const fn new_trivial() -> Self {
    Self {
      cur_table: Self::make_trivial(),
      other_table: Self::make_trivial(),
    }
  }
}

#[cfg(test)]
mod test {
  use algebra::{
    finite::Finite,
    group::{Cyclic, Group, Trivial},
    monoid::Monoid,
  };
  use onoro::groups::{C2, D3, D6, K4};

  use crate::{hash::HashTable, tile_hash::TileHash};

  type HD6 = HashTable<16, D6>;
  type HD3 = HashTable<16, D3>;
  type HK4 = HashTable<16, K4>;
  type HC2 = HashTable<16, C2>;
  type HT = HashTable<16, Trivial>;

  #[derive(Clone, Copy)]
  enum Turn {
    CurPlayer,
    OtherPlayer,
  }

  impl Turn {
    fn iter() -> impl Iterator<Item = Self> {
      [Self::CurPlayer, Self::OtherPlayer].into_iter()
    }
  }

  fn get_hash<const N: usize, G: Group>(
    table: &HashTable<N, G>,
    i: usize,
    turn: Turn,
  ) -> &TileHash<G> {
    match turn {
      Turn::CurPlayer => &table.cur_table[i],
      Turn::OtherPlayer => &table.other_table[i],
    }
  }

  #[test]
  fn test_d6_table() {
    const D6T: HD6 = HashTable::new_c();

    for i in 0..256 {
      let pos = HD6::ord_to_hex_pos(i) - HD6::center();

      for turn in Turn::iter() {
        let mut s = pos;
        let mut op = D6::identity();
        let hash = get_hash(&D6T, i, turn);

        if i == 0 {
          assert_eq!(hash, &TileHash::new(0));
        }

        for _ in 0..5 {
          s = s.apply_d6_c(&D6::Rot(1));
          op = D6::Rot(1) * op;

          let symm_pos = s + HD6::center();
          if HD6::in_bounds(&symm_pos) {
            let ord = HD6::hex_pos_ord(&symm_pos);
            let symm_hash = get_hash(&D6T, ord, turn);

            assert_eq!(symm_hash, &hash.apply(&op));
          }
        }

        op = D6::Rfl(0);
        s = pos.apply_d6_c(&op);
        for _ in 0..6 {
          let symm_pos = s + HD6::center();
          if HD6::in_bounds(&symm_pos) {
            let ord = HD6::hex_pos_ord(&symm_pos);
            let symm_hash = get_hash(&D6T, ord, turn);

            assert_eq!(symm_hash, &hash.apply(&op));
          }

          s = s.apply_d6_c(&D6::Rot(1));
          op = D6::Rot(1) * op;
        }
      }
    }
  }

  #[test]
  fn test_d3_table() {
    const D3T: HD3 = HashTable::new_v();

    for i in 0..256 {
      let pos = HD3::ord_to_hex_pos(i) - HD3::center();

      for turn in Turn::iter() {
        let mut s = pos;
        let mut op = D3::identity();
        let hash = get_hash(&D3T, i, turn);

        if i == 0 {
          assert_eq!(hash, &TileHash::new(0));
        }

        for _ in 0..2 {
          s = s.apply_d3_v(&D3::Rot(1));
          op = D3::Rot(1) * op;

          let symm_pos = s + HD3::center();
          if HD3::in_bounds(&symm_pos) {
            let ord = HD3::hex_pos_ord(&symm_pos);
            let symm_hash = get_hash(&D3T, ord, turn);

            assert_eq!(symm_hash, &hash.apply(&op));
          }
        }

        op = D3::Rfl(0);
        s = pos.apply_d3_v(&op);
        for _ in 0..3 {
          let symm_pos = s + HD3::center();
          if HD3::in_bounds(&symm_pos) {
            let ord = HD3::hex_pos_ord(&symm_pos);
            let symm_hash = get_hash(&D3T, ord, turn);

            assert_eq!(symm_hash, &hash.apply(&op));
          }

          s = s.apply_d3_v(&D3::Rot(1));
          op = D3::Rot(1) * op;
        }
      }
    }
  }

  #[test]
  fn test_k4_table() {
    const K4T: HK4 = HashTable::new_e();

    for i in 0..256 {
      let pos = HK4::ord_to_hex_pos(i) - HK4::center();

      for turn in Turn::iter() {
        let hash = get_hash(&K4T, i, turn);

        if i == 0 {
          assert_eq!(hash, &TileHash::new(0));
        }

        let mut op_ord = 1;
        while op_ord < K4::SIZE {
          let op = K4::const_from_ord(op_ord);
          op_ord += 1;

          let symm_pos = pos.apply_k4_e(&op) + HK4::center();
          if HK4::in_bounds(&symm_pos) {
            let ord = HD3::hex_pos_ord(&symm_pos);
            let symm_hash = get_hash(&K4T, ord, turn);

            assert_eq!(symm_hash, &hash.apply(&op));
          }
        }
      }
    }
  }

  #[test]
  fn test_c2_cv_table() {
    const C2T: HC2 = HashTable::new_cv();

    for i in 0..256 {
      let pos = HC2::ord_to_hex_pos(i) - HC2::center();

      for turn in Turn::iter() {
        let hash = get_hash(&C2T, i, turn);

        if i == 0 {
          assert_eq!(hash, &TileHash::new(0));
        }

        let symm_pos = pos.apply_c2_cv(&Cyclic(1)) + HC2::center();
        if HC2::in_bounds(&symm_pos) {
          let ord = HC2::hex_pos_ord(&symm_pos);
          let symm_hash = get_hash(&C2T, ord, turn);

          assert_eq!(symm_hash, &hash.apply(&Cyclic(1)));
        }
      }
    }
  }

  #[test]
  fn test_c2_ce_table() {
    const C2T: HC2 = HashTable::new_ce();

    for i in 0..256 {
      let pos = HC2::ord_to_hex_pos(i) - HC2::center();

      for turn in Turn::iter() {
        let hash = get_hash(&C2T, i, turn);

        if i == 0 {
          assert_eq!(hash, &TileHash::new(0));
        }

        let symm_pos = pos.apply_c2_ce(&Cyclic(1)) + HC2::center();
        if HC2::in_bounds(&symm_pos) {
          let ord = HC2::hex_pos_ord(&symm_pos);
          let symm_hash = get_hash(&C2T, ord, turn);

          assert_eq!(symm_hash, &hash.apply(&Cyclic(1)));
        }
      }
    }
  }

  #[test]
  fn test_c2_ev_table() {
    const C2T: HC2 = HashTable::new_ev();

    for i in 0..256 {
      let pos = HC2::ord_to_hex_pos(i) - HC2::center();

      for turn in Turn::iter() {
        let hash = get_hash(&C2T, i, turn);

        if i == 0 {
          assert_eq!(hash, &TileHash::new(0));
        }

        let symm_pos = pos.apply_c2_ev(&Cyclic(1)) + HC2::center();
        if HC2::in_bounds(&symm_pos) {
          let ord = HC2::hex_pos_ord(&symm_pos);
          let symm_hash = get_hash(&C2T, ord, turn);

          assert_eq!(symm_hash, &hash.apply(&Cyclic(1)));
        }
      }
    }
  }

  #[test]
  fn test_trivial_table() {
    const TT: HT = HashTable::new_trivial();
    assert_eq!(TT.cur_table[0], TileHash::new(0));
    assert_eq!(TT.other_table[0], TileHash::new(0));
  }
}
