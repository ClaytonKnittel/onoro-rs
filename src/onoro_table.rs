use abstract_game::Score;
use dashmap::DashMap;
use onoro::Onoro16View;

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
  table: DashMap<Onoro16View, Score, BuildPassThroughHasher>,
}

impl OnoroTable {
  pub fn new() -> Self {
    Self {
      table: DashMap::with_hasher(BuildPassThroughHasher),
    }
  }

  pub fn table(&self) -> &DashMap<Onoro16View, Score, BuildPassThroughHasher> {
    &self.table
  }

  pub fn len(&self) -> usize {
    self.table.len()
  }

  pub fn get(&self, key: &Onoro16View) -> Option<Score> {
    self.table.get(key).map(|entry| entry.clone())
  }

  /// Updates the score for an OnoroView in the table, returning the updated
  /// score for the view.
  pub fn update(&self, view: Onoro16View, score: Score) -> Score {
    match self.table.entry(view) {
      dashmap::mapref::entry::Entry::Occupied(mut entry) => {
        *entry.get_mut() = entry.get().merge(&score);
        entry.get().clone()
      }
      dashmap::mapref::entry::Entry::Vacant(entry) => {
        entry.insert(score.clone());
        score
      }
    }
  }
}
