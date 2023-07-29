use algebra::group::Group;

use crate::{groups::D6, tile_hash::TileHash};

struct HashTable<const N: usize, const N2: usize, G: Group> {
  table: [TileHash<G>; N2],
}

impl<const N: usize, const N2: usize> HashTable<N, N2, D6> {
  /// Generates a hash table for boards with symmetry class C.
  const fn new_c() -> Self {
    todo!()
  }
}
