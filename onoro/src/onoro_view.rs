use algebra::{
  group::{Group, Trivial},
  ordinal::Ordinal,
};

use crate::{
  canonicalize::{board_symm_state, BoardSymmetryState},
  groups::{SymmetryClass, C2, D3, D6, K4},
  Onoro,
};

pub struct OnoroView<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> {
  onoro: Onoro<N, N2, ADJ_CNT_SIZE>,
  symm_class: SymmetryClass,
  op_ord: u8,
}

impl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> OnoroView<N, N2, ADJ_CNT_SIZE> {
  pub fn new(onoro: Onoro<N, N2, ADJ_CNT_SIZE>) -> Self {
    let symm_state = board_symm_state(&onoro);

    let (hash, op_ord) = match symm_state.symm_class {
      SymmetryClass::C => Self::find_canonical_orientation::<D6>(&onoro, &symm_state),
      SymmetryClass::V => Self::find_canonical_orientation::<D3>(&onoro, &symm_state),
      SymmetryClass::E => Self::find_canonical_orientation::<K4>(&onoro, &symm_state),
      SymmetryClass::CV => Self::find_canonical_orientation::<C2>(&onoro, &symm_state),
      SymmetryClass::CE => Self::find_canonical_orientation::<C2>(&onoro, &symm_state),
      SymmetryClass::EV => Self::find_canonical_orientation::<C2>(&onoro, &symm_state),
      SymmetryClass::TRIVIAL => Self::find_canonical_orientation::<Trivial>(&onoro, &symm_state),
    };

    Self {
      onoro,
      symm_class: symm_state.symm_class,
      op_ord,
    }
  }

  fn find_canonical_orientation<G: Group>(
    onoro: &Onoro<N, N2, ADJ_CNT_SIZE>,
    symm_state: &BoardSymmetryState,
  ) -> (u64, u8) {
    todo!()
  }
}
