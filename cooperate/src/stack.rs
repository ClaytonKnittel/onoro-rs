use core::num;
use std::{
  ptr::null_mut,
  sync::atomic::{AtomicU32, Ordering},
};

use abstract_game::{Game, Score};
use arrayvec::ArrayVec;
use seize::{AtomicPtr, Linked};

use crate::{queue::QueueItem, util::TransparentIterator};

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
pub enum StackType<G, const N: usize>
where
  G: Game,
{
  Root,
  Child { parent: AtomicPtr<Stack<G, N>> },
}

#[derive(Debug, PartialEq, Eq)]
pub enum StackState {
  /// Live states are states that can currently be worked on.
  Live,
  /// A split state is a state with split children, upon whose completion will
  /// resolve the state at the bottom of the stack. It only tracks the number of
  /// outstanding children. The child to decrease this number to 0 is the one to
  /// revive the state.
  Split,
  /// Suspended states are states that are waiting on the result of some other
  /// pending computation. States may only suspend themselves on the computation
  /// of a frame going exactly as deep as they intend to. Any less deep, and a
  /// definitive answer may not be found (TODO: maybe wait anyway? definitive
  /// answer could be found). Any more deep, and topoligical deadlock is
  /// possible - if a state is dependent on another state, which is itself
  /// dependent on this state (to arbitrary degrees of separation), then the
  /// whole cycle of dependent states would be suspended and never resumed.
  Suspended,
}

pub struct StackFrame<G, const N: usize>
where
  G: Game,
{
  /// The current game state at this frame.
  game: G,
  /// An iterator over the moves at this game state. If `None`, then no moves
  /// have been iterated over yet.
  move_iter: Option<G::MoveIterator>,
  // The best score found for this game so far.
  best_score: Score,
  // The corresponding best move found for `best_score`.
  best_move: Option<G::Move>,
  /// All stack frames have an unordered list of all of their suspended direct
  /// dependants. This can only be appended to under the bin mutex lock from the
  /// pending states hashmap, and reclaimed for revival after removing this
  /// frame from the pending states hashmap.
  dependants: AtomicPtr<Stack<G, N>>,
}

impl<G, const N: usize> StackFrame<G, N>
where
  G: Game,
{
  pub fn new(game: G) -> Self {
    Self {
      game,
      move_iter: None,
      best_score: Score::no_info(),
      best_move: None,
      dependants: AtomicPtr::new(null_mut()),
    }
  }

  pub fn game(&self) -> &G {
    &self.game
  }

  pub fn next_move(&mut self) -> Option<G::Move> {
    match &mut self.move_iter {
      Some(move_iter) => move_iter.next(),
      None => {
        self.move_iter = Some(self.game.each_move());
        self.move_iter.as_mut().unwrap().next()
      }
    }
  }

  /// Updates the best score/move pair of this frame if `score` is better than
  /// the current best score.
  pub fn maybe_update_score(&mut self, score: Score, m: G::Move) {
    if score.better(&self.best_score) {
      self.best_score = score;
      self.best_move = Some(m);
    }
  }

  pub unsafe fn queue_dependant_unlocked(&self, dependant: *mut Linked<Stack<G, N>>) {
    unsafe {
      (*dependant).next = self.dependants.load(Ordering::Relaxed);
    }
    self.dependants.store(dependant, Ordering::Relaxed);
  }

  pub unsafe fn pop_dependant_unlocked(&self) -> Option<*mut Linked<Stack<G, N>>> {
    let head = self.dependants.load(Ordering::Relaxed);
    if head.is_null() {
      return None;
    }

    self
      .dependants
      .store(unsafe { (*head).next }, Ordering::Relaxed);
    Some(head)
  }
}

/// Each task has a stack frame exactly large enough to hold enough frames for a
/// depth-first search of depth `N`.
pub struct Stack<G, const N: usize>
where
  G: Game,
{
  /// The search depth this stack frame will go out to starting from the root
  /// frame.
  root_depth: u32,
  frames: ArrayVec<StackFrame<G, N>, N>,
  ty: StackType<G, N>,
  /// TODO: Can remove state? Implicit from where the stack lies in the data
  /// structure.
  state: StackState,
  /// For live states that are queued for execution, this is a pointer to the
  /// next queued item.
  ///
  /// For suspended states, this is a pointer to the next dependant suspended
  /// state of "dependant", forming a singly-linked list of the dependent
  /// states. This `next` pointer is modified under the bin-lock of the state
  /// that this stack is suspended on in `DashMap`. When the stack is
  /// unsuspended, there will not be any other threads that have references to
  /// this frame since queueing is done under a lock.
  next: *mut Linked<Stack<G, N>>,
  /// A split state is a state with split children, upon whose completion will
  /// resolve the state at the bottom of the stack. It only tracks the number of
  /// outstanding children. The child to decrease this number to 0 is the one to
  /// revive the state.
  outstanding_children: AtomicU32,
}

impl<G, const N: usize> Stack<G, N>
where
  G: Game + 'static,
{
  pub fn make_root(initial_game: G, depth: u32) -> Self {
    let mut root = Self {
      root_depth: depth,
      frames: ArrayVec::new(),
      ty: StackType::Root,
      state: StackState::Live {},
      next: null_mut(),
      outstanding_children: AtomicU32::new(0),
    };
    root.frames.push(StackFrame::new(initial_game));
    root
  }

  fn make_child(game: G, depth: u32, parent: AtomicPtr<Self>) -> Self {
    let mut root = Self {
      root_depth: depth,
      frames: ArrayVec::new(),
      ty: StackType::Child { parent },
      state: StackState::Live {},
      next: null_mut(),
      outstanding_children: AtomicU32::new(0),
    };
    root.frames.push(StackFrame::new(game));
    root
  }

  pub fn push(&mut self, game: G) {
    debug_assert!(!self.frames.is_full());
    self.frames.push(StackFrame::new(game));
  }

  pub fn pop(&mut self) -> StackFrame<G, N> {
    self.frames.pop().unwrap()
  }

  pub fn revive(&mut self) {
    debug_assert_ne!(self.state, StackState::Live);
    self.state = StackState::Live;
  }

  pub fn suspend(&mut self) {
    debug_assert_eq!(self.state, StackState::Live);
    self.state = StackState::Suspended;
  }

  /// Splits a stack frame into a separate stack frame for each possible move of
  /// the bottom game state, returning an iterator over the stack frames for
  /// each child. The iterator must be consumed completely.
  ///
  /// TODO: may want to split at the first frame, not the last.
  pub fn split(self_ptr: AtomicPtr<Self>) -> impl Iterator<Item = Self> {
    // Load the pointer directly without lifetime-protecting, since at this
    // point no other thread can be referencing this stack.
    let self_ptr = self_ptr.load(Ordering::Relaxed);

    let stack = unsafe { &mut *self_ptr };
    debug_assert_eq!(stack.state, StackState::Live);
    debug_assert_eq!(stack.outstanding_children.load(Ordering::Relaxed), 0);

    stack.state = StackState::Split;
    // Keep 1 extra outstanding children counter so it will be impossible for
    // any of the child frames to complete and decrement this to 0 before we
    // have finished producing all of the children. This should be exceedingly
    // rare, but it's due diligence.
    stack.outstanding_children.store(1, Ordering::Relaxed);

    // Generate the child states of this stack frame.
    let game = stack.bottom_frame().game();
    game
      .each_move()
      .map(move |m| {
        let stack = unsafe { &mut *self_ptr };
        stack.outstanding_children.fetch_add(1, Ordering::Relaxed);
        let mut game = stack.bottom_frame().game().clone();
        game.make_move(m);
        Self::make_child(game, stack.bottom_depth() - 1, AtomicPtr::new(self_ptr))
      })
      .chain(TransparentIterator::new(move || {
        let stack = unsafe { &mut *self_ptr };
        if stack.outstanding_children.fetch_sub(1, Ordering::Relaxed) == 0 {
          // TODO: All children have finished, revive this frame.
        }
      }))
  }

  pub fn frame(&self, idx: usize) -> &StackFrame<G, N> {
    debug_assert!(idx < self.frames.len());
    unsafe { self.frames.get_unchecked(idx) }
  }

  pub fn mut_frame(&mut self, idx: usize) -> &mut StackFrame<G, N> {
    debug_assert!(idx < self.frames.len());
    unsafe { self.frames.get_unchecked_mut(idx) }
  }

  pub fn bottom_frame(&self) -> &StackFrame<G, N> {
    self.frames.last().unwrap()
  }

  pub fn bottom_frame_mut(&mut self) -> &mut StackFrame<G, N> {
    self.frames.last_mut().unwrap()
  }

  pub fn bottom_frame_idx(&self) -> usize {
    self.frames.len() - 1
  }

  /// The search depth of the bottom frame of this stack.
  pub fn bottom_depth(&self) -> u32 {
    self.root_depth - (self.frames.len() as u32 - 1)
  }
}

impl<G, const N: usize> QueueItem for Stack<G, N>
where
  G: Game,
{
  fn next(&self) -> *mut Linked<Self> {
    self.next
  }

  fn set_next(&mut self, next: *mut Linked<Self>) {
    self.next = next;
  }
}
