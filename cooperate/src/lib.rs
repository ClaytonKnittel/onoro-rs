use std::hash::{BuildHasher, Hash};

use arrayvec::ArrayVec;
use dashmap::{setref::one::Ref, DashSet};

/// Algorithm:
/// ```rs
/// fn do_alg() {
///   while let Some(unit) = queue.pop() {
///     'seq: loop {
///       let frame = unit.bottom_frame();
///
///       if let Some(move) = frame.moves_iter.next() {
///         let next_state = onoro.with_move(move);
///         match table.get_or_queue(next_state) {
///           FOUND(score) => {
///             // Check if score is usable for this depth or not. If not, will
///             // need to search again with deeper depth.
///             todo!()
///             // Update best score in frame
///             frame.maybe_update_score(score, move);
///           }
///           NOT_FOUND(set_ref) => {
///             // Compute the score of the move. The set_ref is a reference to
///             // the placeholder state in the set indicating that this state
///             // is currently being computed.
///             // TODO: need to figure out how to handle deadlocking - if a
///             // state currently being explored is encountered again, need
///             // to recognize and mark as tie
///             unit.insert_frame(Frame::new(next_state, set_ref));
///           }
///           // If the state is found pending, then it will be added to the list
///           // of states waiting on the result of some game state. After this
///           // result is found (it is being processed by another worker), all
///           // states which are pending are re-added to some worker's queue
///           // (randomly distributed).
///           PENDING => { break 'seq; }
///         }
///       } else {
///         // All moves have been explored. Update the table with the game's
///         // now-known score, and re-queue all pending units.
///         todo!()
///       }
///     }
///   }
/// }
/// ```
/// Each task has a stack frame exactly large enough to hold enough frames for a
/// depth-first search of depth `N`.
struct Stack<Frame, const N: usize>
where
  Frame: Sized,
{
  frames: ArrayVec<Frame, N>,
}

struct Unit<Frame, const N: usize>
where
  Frame: Sized,
{
  stack: Stack<Frame, N>,
}

pub struct Table<State, H>
where
  H: BuildHasher + Clone,
{
  table: DashSet<State, H>,
}

impl<State, H> Table<State, H>
where
  State: Hash + Eq,
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
  pub fn update(&self, view: &mut State) {
    // while !self.table.insert(view.clone()) {
    //   if let Some(prev_view) = self.table.remove(view) {
    //     let merged_score = view.onoro().score().merge(&prev_view.onoro().score());
    //     view.mut_onoro().set_score(merged_score);
    //   }
    // }
    todo!()
  }
}
