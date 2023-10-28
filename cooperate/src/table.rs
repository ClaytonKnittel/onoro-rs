use std::{
  collections::hash_map::RandomState,
  hash::{BuildHasher, Hash},
};

use abstract_game::{Game, Score};
use dashmap::{setref::one::Ref, DashSet};

/// Trait for entries in the concurrent hash table, which holds all previously
/// computed and in-progress states. The purpose of this is to allow `Score`s to
/// be packed in game states to reduce the memory footprint of the
/// completed-states table.
pub trait TableEntry {
  /// The score of this explored game state. This score must be transparent to
  /// PartialEq/Eq/Hash for hashing to work properly. I.e., otherwise equal
  /// `TableEntry`s with different scores should be equal.
  fn score(&self) -> Score;

  /// Sets the score of this explored game state.
  fn set_score(&mut self, score: Score);

  /// Merges two states into one. For processes which slowly discover
  /// information about entries, this method should merge the information
  /// obtained by both entries into one entry. This is used to resolve table
  /// insertion conflicts.
  fn merge(&mut self, other: &Self);
}

pub struct Table<G, H> {
  table: DashSet<G, H>,
}

impl<G> Table<G, RandomState>
where
  G: Game + Hash + Eq + TableEntry,
{
  pub fn new() -> Self {
    Self {
      table: DashSet::new(),
    }
  }
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

  /// Updates an Onoro view in the table, potentially modifying the passed view
  /// to match the merged view that is in the table upon returning.
  pub fn update(&self, state: &mut G) {
    while !self.table.insert(state.clone()) {
      if let Some(other_state) = self.table.remove(&state) {
        state.merge(&other_state);
      }
    }
  }
}
