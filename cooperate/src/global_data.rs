use std::hash::{BuildHasher, Hash};

use abstract_game::{Game, Score};
use seize::{Collector, Linked};

use crate::{
  stack::Stack,
  table::{Table, TableEntry},
};

pub struct GlobalData<G, H, const N: usize>
where
  G: Game,
{
  /// Global memory reclamation construct.
  collector: Collector,
  /// There is a hash table of all pending states for each search depth.
  pending_states: [Table<G, H>; N],
  /// There is a hash table for all states which have been resolved to some
  /// degree. They may need to be recomputed to a greater depth, but the
  /// information in this table will only ever accumulate over time.
  resolved_states: Table<G, H>,
}

pub enum LookupResult {
  Found { score: Score },
  NotFound,
  Queued,
}

impl<G, H, const N: usize> GlobalData<G, H, N>
where
  G: Game + Hash + Eq + TableEntry,
  H: BuildHasher + Clone,
{
  pub fn collector(&self) -> &Collector {
    &self.collector
  }

  /// Will try to find the bottom frame of the stack in the state tables. If it
  /// isn't found, it will reserve a spot in `pending_states` by placing the
  /// bottom game state of the stack.
  pub fn get_or_queue(&self, stack: *mut Linked<Stack<G, N>>) -> LookupResult {
    let bottom_state = unsafe { (*stack).bottom_frame() };
    let game = bottom_state.game();
    if let Some(resolved_state) = self.resolved_states.get(game) {
      return LookupResult::Found {
        score: resolved_state.score(),
      };
    }

    // If the state wasn't found in the resolved table, then try to insert it
    // into its respective pending table.
    let depth_idx = unsafe { (*stack).bottom_depth() } as usize - 1;
    debug_assert!(depth_idx < N);
    while !self.pending_states[depth_idx].insert(game) {
      // If there is already a pending computation, then queue ourselves on it.
      todo!();
    }

    // We claimed the pending slot.
    LookupResult::NotFound
  }
}
