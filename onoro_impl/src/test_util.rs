use std::ops::{Index, IndexMut};

use crate::PackedIdx;

#[repr(align(8))]
pub struct PawnPoses(pub [PackedIdx; 16]);

impl Index<usize> for PawnPoses {
  type Output = PackedIdx;

  fn index(&self, index: usize) -> &Self::Output {
    &self.0[index]
  }
}

impl IndexMut<usize> for PawnPoses {
  fn index_mut(&mut self, index: usize) -> &mut Self::Output {
    &mut self.0[index]
  }
}
