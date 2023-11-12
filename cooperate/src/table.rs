use std::{
  collections::hash_map::RandomState,
  hash::{BuildHasher, Hash},
};

use abstract_game::{Game, Score};
use dashmap::{mapref::entry::Entry, DashMap};

pub struct Table<G, H> {
  table: DashMap<G, Score, H>,
}

impl<G> Table<G, RandomState>
where
  G: Game + Hash + Eq,
{
  #[cfg(test)]
  pub fn new() -> Self {
    Self {
      table: DashMap::new(),
    }
  }
}

impl<G, H> Table<G, H>
where
  G: Game + Hash + Eq,
  H: BuildHasher + Clone,
{
  pub fn with_hasher(hasher: H) -> Self {
    Self {
      table: DashMap::with_hasher(hasher),
    }
  }

  #[cfg(test)]
  pub fn table(&self) -> &DashMap<G, Score, H> {
    &self.table
  }

  pub fn get(&self, key: &G) -> Option<Score> {
    self.table.get(key).map(|entry| entry.value().clone())
  }

  /// Updates an Onoro view in the table, potentially modifying the passed view
  /// to match the merged view that is in the table upon returning.
  pub fn update(&self, state: G, score: Score) {
    match self.table.entry(state) {
      Entry::Occupied(mut entry) => {
        entry.insert(entry.get().merge(&score));
      }
      Entry::Vacant(entry) => {
        entry.insert(score);
      }
    }
  }
}
