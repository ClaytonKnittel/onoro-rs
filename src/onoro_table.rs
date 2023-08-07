use dashmap::{setref::one::Ref, DashSet};
use onoro::{Onoro16, Onoro16View, OnoroView};

pub struct PassThroughHasher {
  state: u64,
}

impl std::hash::Hasher for PassThroughHasher {
  fn write(&mut self, bytes: &[u8]) {
    debug_assert!(bytes.len() == 8 && self.state == 0);
    self.state = unsafe { *(bytes.as_ptr() as *const u64) };
  }

  fn finish(&self) -> u64 {
    self.state
  }
}

#[derive(Clone)]
pub struct BuildPassThroughHasher;

impl std::hash::BuildHasher for BuildPassThroughHasher {
  type Hasher = PassThroughHasher;
  fn build_hasher(&self) -> PassThroughHasher {
    PassThroughHasher { state: 0 }
  }
}

pub struct OnoroTable {
  table: DashSet<Onoro16View, BuildPassThroughHasher>,
}

impl OnoroTable {
  pub fn new() -> Self {
    Self {
      table: DashSet::with_hasher(BuildPassThroughHasher),
    }
  }

  pub fn table(&self) -> &DashSet<Onoro16View, BuildPassThroughHasher> {
    &self.table
  }

  pub fn len(&self) -> usize {
    self.table.len()
  }

  pub fn get<'a>(
    &'a self,
    key: &Onoro16View,
  ) -> Option<Ref<'a, Onoro16View, BuildPassThroughHasher>> {
    self.table.get(key)
  }

  /// Updates an Onoro view in the table, potentially modifying the passed view
  /// to match the merged view that is in the table upon returning.
  pub fn update(&self, view: &mut Onoro16View) {
    while !self.table.insert(view.clone()) {
      if let Some(prev_view) = self.table.remove(view) {
        let merged_score = view.onoro().score().merge(&prev_view.onoro().score());
        view.mut_onoro().set_score(merged_score);
      }
    }
  }
}
