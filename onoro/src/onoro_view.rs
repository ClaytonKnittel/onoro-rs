use crate::{canonicalize::board_symm_state, groups::SymmetryClass, Onoro};

pub struct OnoroView<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> {
  onoro: Onoro<N, N2, ADJ_CNT_SIZE>,
  symm_class: SymmetryClass,
  op_ord: u8,
}

impl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> OnoroView<N, N2, ADJ_CNT_SIZE> {
  pub fn new(onoro: Onoro<N, N2, ADJ_CNT_SIZE>) -> Self {
    let symm_state = board_symm_state(&onoro);

    // TODO: find the canonical orientation.
    Self {
      onoro,
      symm_class: symm_state.symm_class,
      op_ord: 0,
    }
  }
}
