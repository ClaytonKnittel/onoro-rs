use algebra::{group::Trivial, monoid::Monoid, ordinal::Ordinal, semigroup::Semigroup};
use onoro::groups::SymmetryClass;

use crate::{
  OnoroImpl,
  canonicalize::{BoardSymmetryState, board_symm_state},
  hash::HashTable,
  tile_hash::HashGroup,
};
use onoro::groups::{C2, D3, D6, K4};

/// Always generate hash tables for the full game. Only a part of the tables
/// will be used for smaller games.
type ViewHashTable<G> = HashTable<16, G>;

macro_rules! define_find_orientation {
  ($name:ident, $group:ty, $table_constructor:ident) => {
    fn $name<const N: usize>(onoro: &OnoroImpl<N>, symm_state: &BoardSymmetryState) -> (u64, u8) {
      static TABLE: ViewHashTable<$group> = HashTable::$table_constructor();
      let hash = HashGroup::<$group>::new(TABLE.hash(onoro, symm_state));

      // Try all symmetries of the board state with invariant center of mass,
      // choose the symmetry with the numerically smallest hash code.
      <$group>::for_each()
        .map(|op| (hash.apply(&op).hash(), op.ord() as u8))
        .min_by(|(hash1, _), (hash2, _)| hash1.cmp(hash2))
        .unwrap()
    }
  };
}

#[derive(Clone, Debug)]
pub struct CanonicalView {
  symm_class: SymmetryClass,
  op_ord: u8,
  hash: u64,
}

impl CanonicalView {
  pub fn symm_class(&self) -> SymmetryClass {
    self.symm_class
  }

  pub fn op_ord(&self) -> u8 {
    self.op_ord
  }

  pub fn hash(&self) -> u64 {
    self.hash
  }

  pub fn find_canonical_view<const N: usize>(onoro: &OnoroImpl<N>) -> Self {
    let symm_state = board_symm_state(onoro);
    let (hash, op_ord) = match symm_state.symm_class {
      SymmetryClass::C => Self::find_canonical_orientation_d6(onoro, &symm_state),
      SymmetryClass::V => Self::find_canonical_orientation_d3(onoro, &symm_state),
      SymmetryClass::E => Self::find_canonical_orientation_k4(onoro, &symm_state),
      SymmetryClass::CV => Self::find_canonical_orientation_c2_cv(onoro, &symm_state),
      SymmetryClass::CE => Self::find_canonical_orientation_c2_ce(onoro, &symm_state),
      SymmetryClass::EV => Self::find_canonical_orientation_c2_ev(onoro, &symm_state),
      SymmetryClass::Trivial => Self::find_canonical_orientation_trivial(onoro, &symm_state),
    };

    CanonicalView {
      symm_class: symm_state.symm_class,
      op_ord,
      hash,
    }
  }

  define_find_orientation!(find_canonical_orientation_d6, D6, new_c);
  define_find_orientation!(find_canonical_orientation_d3, D3, new_v);
  define_find_orientation!(find_canonical_orientation_k4, K4, new_e);
  define_find_orientation!(find_canonical_orientation_c2_cv, C2, new_cv);
  define_find_orientation!(find_canonical_orientation_c2_ce, C2, new_ce);
  define_find_orientation!(find_canonical_orientation_c2_ev, C2, new_ev);

  fn find_canonical_orientation_trivial<const N: usize>(
    onoro: &OnoroImpl<N>,
    symm_state: &BoardSymmetryState,
  ) -> (u64, u8) {
    static TT: ViewHashTable<Trivial> = HashTable::new_trivial();
    (TT.hash(onoro, symm_state), Trivial::identity().ord() as u8)
  }
}
