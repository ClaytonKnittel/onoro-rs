use std::hash::{BuildHasher, Hash};

use dashmap::{setref::one::Ref, DashSet};

/// Trait for entries in the concurrent hash table, which holds all previously
/// computed and in-progress states.
pub trait TableEntry {
  /// Merges two states into one. For processes which slowly discover
  /// information about entries, this method should merge the information
  /// obtained by both entries into one entry. This is used to resolve table
  /// insertion conflicts.
  fn merge(&mut self, other: &Self);
}

pub struct Table<State, H> {
  table: DashSet<State, H>,
}

impl<State, H> Table<State, H>
where
  State: Hash + Eq + TableEntry + Clone,
  H: BuildHasher + Clone,
{
  pub fn with_hasher(hasher: H) -> Self {
    Self {
      table: DashSet::with_hasher(hasher),
    }
  }

  pub fn table(&self) -> &DashSet<State, H> {
    &self.table
  }

  pub fn len(&self) -> usize {
    self.table.len()
  }

  pub fn get<'a>(&'a self, key: &State) -> Option<Ref<'a, State, H>> {
    self.table.get(key)
  }

  /// Updates an Onoro view in the table, potentially modifying the passed view
  /// to match the merged view that is in the table upon returning.
  pub fn update(&self, state: &mut State) {
    // while !self.table.insert(view.clone()) {
    //   if let Some(prev_view) = self.table.remove(view) {
    //     let merged_score = view.onoro().score().merge(&prev_view.onoro().score());
    //     view.mut_onoro().set_score(merged_score);
    //   }
    // }
    while !self.table.insert(state.clone()) {
      if let Some(other_state) = self.table.remove(state) {
        state.merge(&other_state);
      }
    }
  }
}
