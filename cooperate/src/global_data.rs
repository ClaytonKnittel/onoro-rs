use std::{
  fmt::Display,
  hash::{BuildHasher, Hash},
  sync::atomic::Ordering,
};

use abstract_game::{Game, Score};
use dashmap::{mapref::entry::Entry, DashMap};
use seize::{AtomicPtr, Collector, Linked};

use crate::{
  queue::Queue,
  stack::{Stack, StackFrame},
  table::{Table, TableEntry},
};

struct PendingFrame<G, const N: usize>
where
  G: Game,
{
  /// A pointer to the lifetime-protected stack.
  ///
  /// TODO: Do we need to lifetime-protect stacks? Or can we allocate all of
  /// them up front and free them at the end?
  stack: AtomicPtr<Stack<G, N>>,
  /// The index of the frame that is being awaited on by this entry in the
  /// pending states map.
  ///
  /// TODO: can infer from context, based on root depth and position in the
  /// pending states table.
  frame_idx: u32,
}

pub enum LookupResult {
  Found { score: Score },
  NotFound,
  Queued,
}

pub struct GlobalData<G, H, const N: usize>
where
  G: Game,
{
  /// Global memory reclamation construct.
  collector: Collector,
  /// There is a hash table of all pending states for each search depth.
  pending_states: [DashMap<G, PendingFrame<G, N>, H>; N],
  /// There is a hash table for all states which have been resolved to some
  /// degree. They may need to be recomputed to a greater depth, but the
  /// information in this table will only ever accumulate over time.
  resolved_states: Table<G, H>,
}

impl<G, H, const N: usize> GlobalData<G, H, N>
where
  G: Display + Game + Clone + Hash + Eq + TableEntry + 'static,
  G::Move: Display,
  H: BuildHasher + Clone,
{
  pub fn new(hasher: H) -> Self {
    Self {
      collector: Collector::new(),
      pending_states: [0; N]
        .map(|_| DashMap::<G, PendingFrame<G, N>, H>::with_hasher(hasher.clone())),
      resolved_states: Table::with_hasher(hasher),
    }
  }

  pub fn collector(&self) -> &Collector {
    &self.collector
  }

  /// Will try to find the bottom frame of the stack in the state tables. If it
  /// isn't found, or it is found but wasn't searched deep enough, it will
  /// reserve a spot in `pending_states` by placing the bottom game state of the
  /// stack.
  ///
  /// Stack must be under a seize::Guard for this to be safe.
  pub fn get_or_queue(&self, stack_ptr: *mut Linked<Stack<G, N>>) -> LookupResult {
    let stack = unsafe { &mut *stack_ptr };
    let bottom_state = stack.bottom_frame();
    let game = bottom_state.game();
    if let Some(resolved_state) = self.resolved_states.get(game) {
      if resolved_state.score().determined(stack.bottom_depth()) {
        return LookupResult::Found {
          score: resolved_state.score(),
        };
      }
    }

    // If the state wasn't found in the resolved table, then try to insert it
    // into its respective pending table.
    let depth_idx = stack.bottom_depth() as usize - 1;
    debug_assert!(depth_idx < N);
    match self.pending_states[depth_idx].entry(game.clone()) {
      Entry::Occupied(entry) => {
        // If there is already a pending computation, then queue ourselves on it.
        let pending_frame = entry.get();
        // Do not need to protect this load since this is under the bin mutex
        // lock in DashMap.
        let pending_stack = unsafe { &mut *pending_frame.stack.load(Ordering::Relaxed) };
        let frame = pending_stack.frame(pending_frame.frame_idx as usize);
        unsafe {
          (*stack_ptr).suspend();
          frame.queue_dependant_unlocked(stack_ptr);
        }

        LookupResult::Queued
      }
      Entry::Vacant(entry) => {
        entry.insert(PendingFrame {
          stack: std::sync::atomic::AtomicPtr::new(stack_ptr),
          frame_idx: stack.bottom_frame_idx() as u32,
        });

        println!("    inserting at {depth_idx}");

        // We claimed the pending slot.
        LookupResult::NotFound
      }
    }

    // TODO: check that resolved_states still doesn't have game? Can maybe
    // guarantee to avoid repeat work by checking again. Will have to revive all
    // queued frames on the frame just inserted, but should be rare so not a big
    // deal.
  }

  /// Commits the scores of every complete stack frame, starting from the bottom.
  ///
  /// TODO: take stack: &Stack<...> as a parameter, not stack_ptr.
  pub fn commit_scores(&self, stack_ptr: *mut Linked<Stack<G, N>>, queue: &Queue<Stack<G, N>>) {
    let stack = unsafe { &mut *stack_ptr };
    let mut bottom_state = stack.bottom_frame();
    debug_assert!(bottom_state.current_move().is_none());

    while bottom_state.current_move().is_none() {
      self.commit_score(stack, stack_ptr, queue);
      bottom_state = stack.bottom_frame();
    }
  }

  /// Commits the bottom stack frame to `resolved_states`. Re-queues all states
  /// that are pending on the resolution of this game state to this worker's own
  /// queue.
  fn commit_score(
    &self,
    stack: &mut Stack<G, N>,
    stack_ptr: *mut Linked<Stack<G, N>>,
    queue: &Queue<Stack<G, N>>,
  ) {
    let depth_idx = stack.bottom_depth() as usize - 1;
    let bottom_frame_idx = stack.bottom_frame_idx();

    let bottom_state = stack.bottom_frame_mut();
    let game = bottom_state.game();
    // TODO: this may update the score of game, but right now that is ignored.
    // Updated scores will contain more information, so we should be using that
    // and update game's score here.
    self.resolved_states.update(game);

    // Remove the state from the pending states.
    println!("    removing at {depth_idx}");
    debug_assert!(depth_idx < N);
    match self.pending_states[depth_idx].entry(game.clone()) {
      Entry::Occupied(entry) => {
        let pending_frame = entry.remove();
        debug_assert_eq!(pending_frame.stack.load(Ordering::Relaxed), stack_ptr);
        debug_assert_eq!(pending_frame.frame_idx as usize, bottom_frame_idx);
      }
      Entry::Vacant(_) => {
        debug_assert!(false, "Unexpected vacant entry in pending table.");
      }
    }

    // Re-queue all pending states.
    while let Some(dependant) = unsafe { bottom_state.pop_dependant_unlocked() } {
      queue.push(dependant);
    }

    // Pop this state from the stack.
    stack.pop();
  }
}
