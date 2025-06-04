use crate::{canonicalize::BoardSymmetryState, TILE_BITS};

const fn adjacency_count_size(n: usize) -> usize {
  (n * n * TILE_BITS).div_ceil(64)
}

#[macro_export]
macro_rules! onoro_type {
  ($n:literal) => {
    $crate::Onoro<$n, { $n * $n }, { adjacency_count_size($n) }>
  };
}

#[macro_export]
macro_rules! onoro_view_type {
  ($n:literal) => {
    $crate::OnoroView<$n, { $n * $n }, { adjacency_count_size($n) }>
  };
}

#[macro_export]
macro_rules! onoro_iter_type {
  ($n:literal) => {
    $crate::MoveGenerator<$n, { $n * $n }, { adjacency_count_size($n) }>
  };
}

#[macro_export]
macro_rules! gen_onoro_symm_state_table {
  ($n:literal) => {
    $crate::canonicalize::gen_symm_state_table::<$n, { $n * $n }>()
  };
}

pub type Onoro8 = onoro_type!(8);
pub type Onoro16 = onoro_type!(16);

pub type Onoro8View = onoro_view_type!(8);
pub type Onoro16View = onoro_view_type!(16);

pub type Onoro8MoveIterator = onoro_iter_type!(8);
pub type Onoro16MoveIterator = onoro_iter_type!(16);

pub(crate) const SYMM_TABLE_8: [BoardSymmetryState; 64] = gen_onoro_symm_state_table!(8);
pub(crate) const SYMM_TABLE_16: [BoardSymmetryState; 256] = gen_onoro_symm_state_table!(16);
