use std::{
  collections::hash_map::RandomState,
  fmt::Display,
  hash::{BuildHasher, Hash},
};

use abstract_game::{Game, GameResult, Score};
use crossbeam_queue::SegQueue;
use dashmap::{mapref::entry::Entry, DashMap};

use crate::{null_lock::NullLock, stack::Stack, table::Table, Metrics};

struct PendingFrame<G>
where
  G: Game,
{
  /// A pointer to the stack. NullLock allows us to share this value between
  /// threads, and it's our responsibility to make sure it's only accessed
  /// behind locks/when a thread has exclusive access to it.
  stack: NullLock<*mut Stack<G>>,
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

pub struct GlobalData<G, H>
where
  G: Game,
{
  /// All of the queues for the worker threads. Each thread's queue is at index
  /// `thread_idx` in this vector. The entries in these queues can be "stolen"
  /// from by other workers when they run out of work to do.
  queues: Vec<SegQueue<NullLock<*mut Stack<G>>>>,
  /// There is a hash table of all pending states for each search depth.
  pending_states: Vec<DashMap<G, PendingFrame<G>, H>>,
  /// There is a hash table for all states which have been resolved to some
  /// degree. They may need to be recomputed to a greater depth, but the
  /// information in this table will only ever accumulate over time.
  resolved_states: Table<G, H>,
}

impl<G> GlobalData<G, RandomState>
where
  G: Display + Game + Clone + Hash + Eq + 'static,
  G::Move: Display,
{
  #[cfg(test)]
  pub fn new(search_depth: u32, num_threads: u32) -> Self {
    Self {
      queues: (0..num_threads).map(|_| SegQueue::new()).collect(),
      pending_states: (0..search_depth)
        .map(|_| DashMap::<G, PendingFrame<G>, RandomState>::new())
        .collect(),
      resolved_states: Table::new(),
    }
  }
}

impl<G, H> GlobalData<G, H>
where
  G: Display + Game + Clone + Hash + Eq + 'static,
  G::Move: Display,
  H: BuildHasher + Clone,
{
  pub fn with_hasher(search_depth: u32, num_threads: u32, hasher: H) -> Self {
    Self {
      queues: (0..num_threads).map(|_| SegQueue::new()).collect(),
      pending_states: (0..search_depth)
        .map(|_| DashMap::<G, PendingFrame<G>, H>::with_hasher(hasher.clone()))
        .collect(),
      resolved_states: Table::with_hasher(hasher),
    }
  }

  pub fn queue(&self, thread_idx: u32) -> &SegQueue<NullLock<*mut Stack<G>>> {
    self.queues.get(thread_idx as usize).unwrap()
  }

  pub fn resolved_states_table(&self) -> &Table<G, H> {
    &self.resolved_states
  }

  /// Will try to find the bottom frame of the stack in the state tables. If it
  /// isn't found, or it is found but wasn't searched deep enough, it will
  /// reserve a spot in `pending_states` by placing the bottom game state of the
  /// stack.
  pub fn get_or_queue(&self, stack_ptr: *mut Stack<G>, metrics: &mut Metrics) -> LookupResult {
    let stack = unsafe { &mut *stack_ptr };
    let bottom_state = stack.bottom_frame().unwrap();
    let game = bottom_state.game();
    if let Some(score) = self.resolved_states.get(game) {
      if score.determined(stack.bottom_depth()) {
        metrics.hits += 1;
        return LookupResult::Found { score };
      }
    }

    // If the state wasn't found in the resolved table, then try to insert it
    // into its respective pending table.
    let depth_idx = stack.bottom_depth() as usize - 1;
    match self.pending_states[depth_idx].entry(game.clone()) {
      Entry::Occupied(entry) => {
        // If there is already a pending computation, then queue ourselves on it.
        let pending_frame = entry.get();
        // Do not need to protect this load since this is under the bin mutex
        // lock in DashMap.
        let pending_stack = unsafe { &mut **pending_frame.stack.lock() };
        let frame = pending_stack.frame_mut(pending_frame.frame_idx);
        unsafe {
          (*stack_ptr).suspend();
          frame.queue_dependant_unlocked(stack_ptr);
        }

        metrics.queues += 1;
        LookupResult::Queued
      }
      Entry::Vacant(entry) => {
        entry.insert(PendingFrame {
          stack: unsafe { NullLock::new(stack_ptr) },
          frame_idx: stack.bottom_frame_idx() as u32,
        });

        // We claimed the pending slot.
        metrics.claims += 1;
        LookupResult::NotFound
      }
    }

    // TODO: check that resolved_states still doesn't have game? Can maybe
    // guarantee to avoid repeat work by checking again. Will have to revive all
    // queued frames on the frame just inserted, but should be rare so not a big
    // deal.
  }

  /// Commits the scores of every complete stack frame, if there are any and
  /// starting from the bottom, and finds the next move that needs to be
  /// explored.
  ///
  /// TODO: take stack: &Stack<...> as a parameter, not stack_ptr.
  pub fn explore_next_state(
    &self,
    stack_ptr: *mut Stack<G>,
    queue: &SegQueue<NullLock<*mut Stack<G>>>,
  ) {
    let stack = unsafe { &mut *stack_ptr };

    let mut bottom_depth = stack.bottom_depth();
    // TODO: don't generate moves for bottom stack frames, we will never use them.
    while let Some(bottom_state) = stack.bottom_frame_mut() {
      match bottom_state.current_move() {
        Some(m) => {
          let game = bottom_state.game().with_move(m);
          // println!("  move {} for\n{}", m, bottom_state.game());

          if bottom_depth == 1 {
            let score = match game.finished() {
              GameResult::Win(winner) => {
                if winner == bottom_state.game().current_player() {
                  Score::win(1)
                } else {
                  Score::lose(1)
                }
              }
              GameResult::Tie => Score::guaranteed_tie(),
              GameResult::NotFinished => {
                Score::tie(1)
                // TODO: not immediately clear if search imm win is faster.
                // if game.search_immediate_win().is_some() {
                //   self.commit_game_with_score(bottom_state.game().clone(), Score::win(1));
                //   // If this game is a win for the current player, it's a lose for the
                //   // player of the previous turn.
                //   Score::lose(2)
                // } else {
                //   // Don't commit game, since we have no information on it (tie to
                //   // depth 1 is not worth committing).
                //   Score::tie(1)
                // }
              }
            };

            stack.update_parent_score_and_advance(score);
          } else {
            // println!("  move {} for\n{}", m, bottom_state.game());
            let next_state = bottom_state.game().with_move(m);
            stack.push(next_state);
            break;
          }
        }
        None => {
          self.commit_score(stack, stack_ptr, queue);
          bottom_depth += 1
        }
      }
    }
  }

  /// Commits the bottom stack frame to `resolved_states`. Re-queues all states
  /// that are pending on the resolution of this game state to this worker's own
  /// queue.
  fn commit_score(
    &self,
    stack: &mut Stack<G>,
    stack_ptr: *mut Stack<G>,
    queue: &SegQueue<NullLock<*mut Stack<G>>>,
  ) {
    let depth_idx = stack.bottom_depth() as usize - 1;
    let bottom_frame_idx = stack.bottom_frame_idx();

    let bottom_state = stack.bottom_frame_mut().unwrap();
    let score = bottom_state.best_score().0.clone();
    let game = bottom_state.game().clone();
    // println!("  Out of moves, committing score {} for\n{}", score, game);
    self.commit_game_with_score(game.clone(), score);

    // Remove the state from the pending states.
    // println!("    removing at {depth_idx}");
    match self.pending_states[depth_idx].entry(game) {
      Entry::Occupied(entry) => {
        let pending_frame = entry.remove();
        debug_assert_eq!(*pending_frame.stack, stack_ptr);
        debug_assert_eq!(pending_frame.frame_idx as usize, bottom_frame_idx);
      }
      Entry::Vacant(_) => {
        debug_assert!(false, "Unexpected vacant entry in pending table.");
      }
    }
    // for x in self.pending_states[depth_idx].iter() {
    //   println!("      found guy at {depth_idx}:\n{}\n", x.key());
    // }

    // Re-queue all pending states.
    while let Some(dependant) = unsafe { bottom_state.pop_dependant_unlocked() } {
      unsafe { &mut *dependant }.revive();
      queue.push(unsafe { NullLock::new(dependant) });
    }

    // Pop this state from the stack.
    stack.pop();
  }

  fn commit_game_with_score(&self, game: G, score: Score) {
    self.resolved_states.update(game, score);
  }
}
