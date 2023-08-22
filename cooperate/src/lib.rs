use std::{
  hash::{BuildHasher, Hash},
  sync::{atomic::AtomicPtr, Arc},
};

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

/// The type of a stack is either the root, which contains the initial game state as it's first
/// frame, or a child, which has a pointer to the parent that it is solving a branch for.
enum StackType<Frame, const N: usize> {
  Root,
  Child { parent: Arc<Stack<Frame, N>> },
}

enum StackState<Frame, const N: usize> {
  Live {
    /// Live states are in an unordered list when queued for execution, which is operated on purely
    /// atomically without the use of locks.
    next: AtomicPtr<Stack<Frame, N>>,
    /// Live states have an unordered list of all of their suspended direct dependants.
    dependants: AtomicPtr<Stack<Frame, N>>,
  },
  Suspended {
    /// Suspended states have a pointer to the next dependant suspended state of "dependant".
    next: AtomicPtr<Stack<Frame, N>>,
    /// Suspended states have a pointer to their direct dependant, which can be recursively traced to
    /// find the root live node that this state is dependent on. This forms a linked union find data
    /// structure, where the representatives of each union is the live StackState that each state
    /// transitively depends on. Every other state in the union must be suspended, and a live state
    /// must first check that it isn't the root live state a suspended state is dependant on before
    /// it can suspend itself. This will prevent topological deadlock via circular dependency of
    /// state discovery.
    ///
    /// Since a currently executing live state is owned by a single thread, the check that the root
    /// live state is not itself, done before suspending a live state and making it dependant on
    /// another, can be done without locking. If a state finds itself to be the root dependant live
    /// state, then no transitive dependants of this state could have changed state during the
    /// search for the root. TODO: figure out how to handle the case where it's dependant on
    /// another live state, which can be concurrently modified. No obvious way to lock on the root
    /// state safely.
    dependant: AtomicPtr<Stack<Frame, N>>,
  },
}

/// Each task has a stack frame exactly large enough to hold enough frames for a
/// depth-first search of depth `N`.
struct Stack<Frame, const N: usize>
where
  Frame: Sized,
{
  frames: ArrayVec<Frame, N>,
  ty: StackType<Frame, N>,
  state: StackState<Frame, N>,
}

struct WorkerData<Frame, const N: usize> {
  /// The queue of frames local to this worker thread. This can be "stolen" from by other workers
  /// when they run out of work to do.
  queue: AtomicPtr<Stack<Frame, N>>,
}

/// Trait for entries in the concurrent hash table, which holds all previously computed and
/// in-progress states.
pub trait TableEntry {
  /// Returns true when an entry in the table is in progress, otherwise it's considered resolved.
  fn in_progress(&self) -> bool;

  /// Merges two states into one. For processes which slowly discover information about entries,
  /// this method should merge the information obtained by both entries into one entry. This is
  /// used to resolve table insertion conflicts.
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
