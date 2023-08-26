use std::{
  hash::{BuildHasher, Hash},
  sync::atomic::{AtomicU32, Ordering},
};

use abstract_game::{Game, Score, ScoreValue};
use arrayvec::ArrayVec;
use dashmap::{setref::one::Ref, DashSet};
use seize::{AtomicPtr, Collector};

mod metrics;
mod search;

pub use metrics::*;

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

/// The type of a stack is either the root, which contains the initial game
/// state as it's first frame, or a child, which has a pointer to the parent
/// that it is solving a branch for.
enum StackType<Frame, const N: usize> {
  Root,
  Child { parent: AtomicPtr<Stack<Frame, N>> },
}

enum StackState<Frame, const N: usize> {
  /// Live states are states that can currently be worked on.
  Live {
    /// Live states are in an unordered list when queued for execution, which is
    /// operated on purely atomically without the use of locks.
    next: AtomicPtr<Stack<Frame, N>>,
  },
  /// A split state is a state with split children, upon whose completion will
  /// resolve the state at the bottom of the stack. It only tracks the number of
  /// outstanding children. The child to decrease this number to 0 is the one to
  /// revive the state.
  Split { outstanding_children: AtomicU32 },
  /// Suspended states are states that are waiting on the result of some other
  /// pending computation. States may only suspend themselves on the computation
  /// of a frame going exactly as deep as they intend to. Any less deep, and a
  /// definitive answer may not be found (TODO: maybe wait anyway? definitive
  /// answer could be found). Any more deep, and topoligical deadlock is
  /// possible - if a state is dependent on another state, which is itself
  /// dependent on this state (to arbitrary degrees of separation), then the
  /// whole cycle of dependent states would be suspended and never resumed.
  Suspended {
    /// Suspended states have a pointer to the next dependant suspended state of
    /// "dependant", forming a singly-linked list of the dependent states.
    next: AtomicPtr<Stack<Frame, N>>,
  },
}

struct StackFrame<Frame, const N: usize> {
  /// Application-specific frame struct.
  frame: Frame,
  /// All stack frames have an unordered list of all of their suspended direct
  /// dependants.
  dependants: AtomicPtr<Stack<Frame, N>>,
}

/// Each task has a stack frame exactly large enough to hold enough frames for a
/// depth-first search of depth `N`.
struct Stack<Frame, const N: usize>
where
  Frame: Sized,
{
  frames: ArrayVec<StackFrame<Frame, N>, N>,
  ty: StackType<Frame, N>,
  state: StackState<Frame, N>,
}

struct WorkerData<Frame, State, H, const N: usize> {
  /// The queue of frames local to this worker thread. This can be "stolen" from
  /// by other workers when they run out of work to do.
  queue: AtomicPtr<Stack<Frame, N>>,
  globals: GlobalData<State, H, N>,
}

struct GlobalData<State, H, const N: usize> {
  /// Global memory reclamation construct.
  collector: Collector,
  /// There is a hash table of all pending states for each search depth.
  pending_states: [Table<State, H>; N],
  /// There is a hash table for all states which have been resolved to some
  /// degree. They may need to be recomputed to a greater depth, but the
  /// information in this table will only ever accumulate over time.
  resolved_states: Table<State, H>,
}

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

pub struct Options {
  /// The number of worker threads to use in the thread pool.
  num_threads: u32,
  /// The depth to expand to for generating work units.
  unit_depth: u32,
}

pub fn solve<G: Game>(game: &G, options: Options) {}
