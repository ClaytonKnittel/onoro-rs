#[derive(Clone, Copy)]
struct Node {
  /// The index of parent of this node (self if root).
  parent: usize,
  /// Size of the tree under this element if this is a root, otherwise this
  /// value is 0.
  size: usize,
}

pub struct ConstUnionFind<const N: usize> {
  unique_sets: usize,
  elements: [Node; N],
}

impl<const N: usize> ConstUnionFind<N> {
  pub fn new() -> Self {
    let mut elements = [Node { parent: 0, size: 0 }; N];
    elements.iter_mut().enumerate().for_each(|(idx, node)| {
      *node = Node {
        parent: idx,
        size: 1,
      }
    });

    Self {
      unique_sets: N,
      elements,
    }
  }

  pub fn unique_sets(&self) -> usize {
    self.unique_sets
  }

  /// Gives id of the root of tree that node is in.
  pub fn find(&mut self, mut node_id: usize) -> usize {
    let mut node = self.elements[node_id];

    while node.parent != node_id {
      let parent = self.elements[node.parent];
      // Slowly compress tree by assigning node's parent to its grandparent.
      self.elements[node_id].parent = parent.parent;

      // Next look at the former parent of node, rather than skipping to it's
      // new parent. This will cause a long chain of nodes to be compressed into
      // two equally-sized trees.
      node_id = node.parent;
      node = parent;
    }

    node_id
  }

  /// Unions the two sets that a and b are in (noop if are already in the same
  /// set), returning the new set index of the two nodes.
  pub fn union(&mut self, a_id: usize, b_id: usize) -> usize {
    let mut a_root_id = self.find(a_id);
    let b_root_id = self.find(b_id);

    if a_root_id != b_root_id {
      let a_root = self.elements[a_root_id];
      let b_root = self.elements[b_root_id];

      // Attach smaller tree to larger tree.
      if a_root.size < b_root.size {
        self.elements[b_root_id].size += a_root.size;
        self.elements[a_root_id].parent = b_root_id;
        a_root_id = b_root_id;
      } else {
        self.elements[a_root_id].size += b_root.size;
        self.elements[b_root_id].parent = a_root_id;
      }

      // Two sets have joined, reducing the number of unique sets by one.
      self.unique_sets -= 1;
    }

    a_root_id
  }
}

#[cfg(test)]
mod tests {
  use crate::ConstUnionFind;

  #[test]
  fn test_basic() {
    let mut uf = ConstUnionFind::<10>::new();

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
    let mut uf = ConstUnionFind::<256>::new();

    for i in 0..255 {
      uf.union(i, i + 1);
    }

    let root_id = uf.find(0);
    for i in 1..256 {
      assert_eq!(uf.find(i), root_id);
    }
  }
}
