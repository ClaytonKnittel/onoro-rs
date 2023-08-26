use abstract_game::{Game, Score};
use seize::{Collector, Linked};

use crate::{stack::Stack, table::Table};

pub struct GlobalData<State, H, const N: usize> {
  /// Global memory reclamation construct.
  collector: Collector,
  /// There is a hash table of all pending states for each search depth.
  pending_states: [Table<State, H>; N],
  /// There is a hash table for all states which have been resolved to some
  /// degree. They may need to be recomputed to a greater depth, but the
  /// information in this table will only ever accumulate over time.
  resolved_states: Table<State, H>,
}

pub enum LookupResult {
  Found { score: Score },
  NotFound {},
  Queued {},
}

impl<State, H, const N: usize> GlobalData<State, H, N> {
  pub fn collector(&self) -> &Collector {
    &self.collector
  }

  /// Will try to find the bottom frame of the stack in the state tables. If it
  /// isn't found, it will reserve a spot in `pending_states` by placing the
  /// bottom game state of the stack.
  pub fn get_or_queue<G>(&self, stack: *mut Linked<Stack<G, N>>) -> LookupResult
  where
    G: Game,
  {
    todo!()
  }
}
