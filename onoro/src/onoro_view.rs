use std::fmt::Display;

use algebra::{group::Trivial, monoid::Monoid, ordinal::Ordinal};

use crate::{
  canonicalize::{board_symm_state, BoardSymmetryState},
  groups::{SymmetryClass, C2, D3, D6, K4},
  hash::HashTable,
  tile_hash::HashGroup,
  Onoro,
};

#[derive(Debug)]
pub struct OnoroView<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> {
  onoro: Onoro<N, N2, ADJ_CNT_SIZE>,
  symm_class: SymmetryClass,
  op_ord: u8,
  hash: u64,
}

impl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> OnoroView<N, N2, ADJ_CNT_SIZE> {
  pub fn new(onoro: Onoro<N, N2, ADJ_CNT_SIZE>) -> Self {
    let symm_state = board_symm_state(&onoro);

    let (hash, op_ord) = match symm_state.symm_class {
      SymmetryClass::C => Self::find_canonical_orientation_d6(&onoro, &symm_state),
      SymmetryClass::V => Self::find_canonical_orientation_d3(&onoro, &symm_state),
      SymmetryClass::E => Self::find_canonical_orientation_k4(&onoro, &symm_state),
      SymmetryClass::CV => Self::find_canonical_orientation_c2_cv(&onoro, &symm_state),
      SymmetryClass::CE => Self::find_canonical_orientation_c2_ce(&onoro, &symm_state),
      SymmetryClass::EV => Self::find_canonical_orientation_c2_ev(&onoro, &symm_state),
      SymmetryClass::Trivial => Self::find_canonical_orientation_trivial(&onoro, &symm_state),
    };

    Self {
      onoro,
      symm_class: symm_state.symm_class,
      op_ord,
      hash,
    }
  }

  fn find_canonical_orientation_d6(
    onoro: &Onoro<N, N2, ADJ_CNT_SIZE>,
    symm_state: &BoardSymmetryState,
  ) -> (u64, u8) {
    static D6T: HashTable<16, 256, D6> = HashTable::new_c();
    let hash = HashGroup::<D6>::new(D6T.hash(onoro, symm_state));

    // Try all symmetries of the board state with invariant center of mass,
    // choose the symmetry with the numerically smallest hash code.
    D6::for_each()
      .map(|op| (hash.apply(&op).hash(), op.ord() as u8))
      .min_by(|(_op1, hash1), (_op2, hash2)| hash1.cmp(hash2))
      .unwrap()
  }

  fn find_canonical_orientation_d3(
    onoro: &Onoro<N, N2, ADJ_CNT_SIZE>,
    symm_state: &BoardSymmetryState,
  ) -> (u64, u8) {
    static D3T: HashTable<16, 256, D3> = HashTable::new_v();
    let hash = HashGroup::<D3>::new(D3T.hash(onoro, symm_state));

    // Try all symmetries of the board state with invariant center of mass,
    // choose the symmetry with the numerically smallest hash code.
    D3::for_each()
      .map(|op| (hash.apply(&op).hash(), op.ord() as u8))
      .min_by(|(_op1, hash1), (_op2, hash2)| hash1.cmp(hash2))
      .unwrap()
  }

  fn find_canonical_orientation_k4(
    onoro: &Onoro<N, N2, ADJ_CNT_SIZE>,
    symm_state: &BoardSymmetryState,
  ) -> (u64, u8) {
    static K4T: HashTable<16, 256, K4> = HashTable::new_e();
    let hash = HashGroup::<K4>::new(K4T.hash(onoro, symm_state));

    // Try all symmetries of the board state with invariant center of mass,
    // choose the symmetry with the numerically smallest hash code.
    K4::for_each()
      .map(|op| (hash.apply(&op).hash(), op.ord() as u8))
      .min_by(|(_op1, hash1), (_op2, hash2)| hash1.cmp(hash2))
      .unwrap()
  }

  fn find_canonical_orientation_c2_cv(
    onoro: &Onoro<N, N2, ADJ_CNT_SIZE>,
    symm_state: &BoardSymmetryState,
  ) -> (u64, u8) {
    static C2CVT: HashTable<16, 256, C2> = HashTable::new_cv();
    let hash = HashGroup::<C2>::new(C2CVT.hash(onoro, symm_state));

    // Try all symmetries of the board state with invariant center of mass,
    // choose the symmetry with the numerically smallest hash code.
    C2::for_each()
      .map(|op| (hash.apply(&op).hash(), op.ord() as u8))
      .min_by(|(_op1, hash1), (_op2, hash2)| hash1.cmp(hash2))
      .unwrap()
  }

  fn find_canonical_orientation_c2_ce(
    onoro: &Onoro<N, N2, ADJ_CNT_SIZE>,
    symm_state: &BoardSymmetryState,
  ) -> (u64, u8) {
    static C2CET: HashTable<16, 256, C2> = HashTable::new_ce();
    let hash = HashGroup::<C2>::new(C2CET.hash(onoro, symm_state));

    // Try all symmetries of the board state with invariant center of mass,
    // choose the symmetry with the numerically smallest hash code.
    C2::for_each()
      .map(|op| (hash.apply(&op).hash(), op.ord() as u8))
      .min_by(|(_op1, hash1), (_op2, hash2)| hash1.cmp(hash2))
      .unwrap()
  }

  fn find_canonical_orientation_c2_ev(
    onoro: &Onoro<N, N2, ADJ_CNT_SIZE>,
    symm_state: &BoardSymmetryState,
  ) -> (u64, u8) {
    static C2EVT: HashTable<16, 256, C2> = HashTable::new_ev();
    let hash = HashGroup::<C2>::new(C2EVT.hash(onoro, symm_state));

    // Try all symmetries of the board state with invariant center of mass,
    // choose the symmetry with the numerically smallest hash code.
    C2::for_each()
      .map(|op| (hash.apply(&op).hash(), op.ord() as u8))
      .min_by(|(_op1, hash1), (_op2, hash2)| hash1.cmp(hash2))
      .unwrap()
  }

  fn find_canonical_orientation_trivial(
    onoro: &Onoro<N, N2, ADJ_CNT_SIZE>,
    symm_state: &BoardSymmetryState,
  ) -> (u64, u8) {
    static TT: HashTable<16, 256, Trivial> = HashTable::new_trivial();
    (TT.hash(onoro, symm_state), Trivial::identity().ord() as u8)
  }
}

impl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> Display
  for OnoroView<N, N2, ADJ_CNT_SIZE>
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "{}\n{:?}: {} ({:#018x?})",
      self.onoro,
      self.symm_class,
      match self.symm_class {
        SymmetryClass::C => D6::from_ord(self.op_ord as usize).to_string(),
        SymmetryClass::V => D3::from_ord(self.op_ord as usize).to_string(),
        SymmetryClass::E => K4::from_ord(self.op_ord as usize).to_string(),
        SymmetryClass::CV | SymmetryClass::CE | SymmetryClass::EV =>
          C2::from_ord(self.op_ord as usize).to_string(),
        SymmetryClass::Trivial => Trivial::from_ord(self.op_ord as usize).to_string(),
      },
      self.hash
    )
  }
}
