use crate::{
  MoveGenerator, OnoroImpl, OnoroView,
  canonicalize::{BoardSymmetryState, gen_symm_state_table},
};

pub type Onoro8 = OnoroImpl<8>;
pub type Onoro16 = OnoroImpl<16>;

pub type Onoro8View = OnoroView<8>;
pub type Onoro16View = OnoroView<16>;

pub type Onoro8MoveIterator = MoveGenerator<8>;
pub type Onoro16MoveIterator = MoveGenerator<16>;

// TODO: use these
#[allow(unused)]
pub(crate) const SYMM_TABLE_8: [[BoardSymmetryState; 8]; 8] = gen_symm_state_table::<8>();
#[allow(unused)]
pub(crate) const SYMM_TABLE_16: [[BoardSymmetryState; 16]; 16] = gen_symm_state_table::<16>();
