use std::hash::{BuildHasher, Hash};

use abstract_game::{Game, Score};
use dashmap::{setref::one::Ref, DashSet};

/// Trait for entries in the concurrent hash table, which holds all previously
/// computed and in-progress states.
pub trait TableEntry {
  fn score(&self) -> Score;

  /// Merges two states into one. For processes which slowly discover
  /// information about entries, this method should merge the information
  /// obtained by both entries into one entry. This is used to resolve table
  /// insertion conflicts.
  fn merge(&mut self, other: &Self);
}

pub struct Table<G, H> {
  table: DashSet<G, H>,
}

impl<G, H> Table<G, H>
where
  G: Game + Hash + Eq + TableEntry,
  H: BuildHasher + Clone,
{
  pub fn with_hasher(hasher: H) -> Self {
    Self {
      table: DashSet::with_hasher(hasher),
    }
  }

  pub fn table(&self) -> &DashSet<G, H> {
    &self.table
  }

  pub fn len(&self) -> usize {
    self.table.len()
  }

  pub fn get<'a>(&'a self, key: &G) -> Option<Ref<'a, G, H>> {
    self.table.get(key)
  }

  pub fn insert(&self, state: &G) -> bool {
    self.table.insert(state.clone())
  }

  /// Updates an Onoro view in the table, potentially modifying the passed view
  /// to match the merged view that is in the table upon returning.
  pub fn update(&self, state: &mut G) {
    while !self.table.insert(state.clone()) {
      if let Some(other_state) = self.table.remove(state) {
        state.merge(&other_state);
      }
    }
  }
}
