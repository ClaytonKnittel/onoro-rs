#[derive(Clone, Copy)]
struct Node {
  /// The index of parent of this node (self if root).
  parent: u8,
}

pub struct UnionFind {
  unique_sets: usize,
  elements: Vec<Node>,
}

impl UnionFind {
  pub fn new(capacity: usize) -> Self {
    let elements = (0..capacity)
      .enumerate()
      .map(|(idx, _)| Node { parent: idx as u8 })
      .collect();

    Self {
      unique_sets: capacity,
      elements,
    }
  }

  pub fn capacity(&self) -> usize {
    self.elements.len()
  }

  pub fn unique_sets(&self) -> usize {
    self.unique_sets
  }

  fn get_node(&self, node_id: usize) -> Node {
    debug_assert!(node_id < self.capacity());
    unsafe { *self.elements.get_unchecked(node_id) }
  }

  /// Gives id of the root of tree that node is in.
  pub fn find(&mut self, mut node_id: usize) -> usize {
    debug_assert!(node_id < self.capacity());
    let mut node = self.get_node(node_id);

    while node.parent as usize != node_id {
      let parent = self.get_node(node.parent as usize);
      // Slowly compress tree by assigning node's parent to its grandparent.
      unsafe {
        self.elements.get_unchecked_mut(node_id).parent = parent.parent;
      }

      // Next look at the former parent of node, rather than skipping to it's
      // new parent. This will cause a long chain of nodes to be compressed into
      // two equally-sized trees.
      node_id = node.parent as usize;
      node = parent;
    }

    node_id
  }

  /// Unions the two sets that a and b are in (noop if are already in the same
  /// set), returning the new set index of the two nodes.
  pub fn union(&mut self, a_id: usize, b_id: usize) -> usize {
    let a_root_id = self.find(a_id);
    let b_root_id = self.find(b_id);

    if a_root_id != b_root_id {
      unsafe {
        self.elements.get_unchecked_mut(b_root_id).parent = a_root_id as u8;
      }

      // Two sets have joined, reducing the number of unique sets by one.
      self.unique_sets -= 1;
    }

    a_root_id
  }
}

#[cfg(test)]
mod tests {
  use crate::UnionFind;

  #[test]
  fn test_basic() {
    let mut uf = UnionFind::new(10);

    for i in 0..10 {
      assert_eq!(uf.find(i), i);
    }

    uf.union(1, 3);
    uf.union(4, 5);
    uf.union(1, 5);

    assert!(uf.find(1) == uf.find(3));
    assert!(uf.find(1) == uf.find(4));
    assert!(uf.find(1) == uf.find(5));
    assert!(uf.find(0) == 0);
    assert!(uf.find(2) == 2);
    assert!(uf.find(6) == 6);
    assert!(uf.find(7) == 7);
    assert!(uf.find(8) == 8);
    assert!(uf.find(9) == 9);
  }

  #[test]
  fn test_long_chain() {
    let mut uf = UnionFind::new(256);

    for i in 0..255 {
      uf.union(i, i + 1);
    }

    let root_id = uf.find(0);
    for i in 1..256 {
      assert_eq!(uf.find(i), root_id);
    }
  }
}
