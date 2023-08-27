use std::{
  ptr::null_mut,
  sync::atomic::{AtomicU32, Ordering},
};

use abstract_game::Game;
use arrayvec::ArrayVec;
use seize::{AtomicPtr, Linked};

use crate::queue::QueueItem;

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
  /// Application-specific frame struct.
  game: G,
  move_iter: Option<G::MoveIterator>,
  /// All stack frames have an unordered list of all of their suspended direct
  /// dependants.
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

  pub unsafe fn queue_dependant_unlocked(&self, dependant: *mut Linked<Stack<G, N>>) {
    unsafe {
      (*dependant).next = self.dependants.load(Ordering::Relaxed);
    }
    self.dependants.store(dependant, Ordering::Relaxed);
  }
}

/// Each task has a stack frame exactly large enough to hold enough frames for a
/// depth-first search of depth `N`.
pub struct Stack<G, const N: usize>
where
  G: Game,
{
  root_depth: u32,
  frames: ArrayVec<StackFrame<G, N>, N>,
  ty: StackType<G, N>,
  /// TODO: Can remove state? Implicit from where the stack lies in the data
  /// structure.
  state: StackState,
  /// Live states are queued for execution, which requires a pointer to the next
  /// queued item.
  ///
  /// Suspended states have a pointer to the next dependant suspended state of
  /// "dependant", forming a singly-linked list of the dependent states.
  next: *mut Linked<Stack<G, N>>,
  /// A split state is a state with split children, upon whose completion will
  /// resolve the state at the bottom of the stack. It only tracks the number of
  /// outstanding children. The child to decrease this number to 0 is the one to
  /// revive the state.
  outstanding_children: AtomicU32,
}

impl<G, const N: usize> Stack<G, N>
where
  G: Game,
{
  pub fn root(initial_game: G, depth: u32) -> Self {
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

  pub fn push(&mut self, game: G) {
    self.frames.push(StackFrame::new(game));
  }

  pub fn revive(&mut self) {
    debug_assert_ne!(self.state, StackState::Live);
    self.state = StackState::Live;
  }

  pub fn suspend(&mut self) {
    debug_assert_eq!(self.state, StackState::Live);
    self.state = StackState::Suspended;
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

  pub fn bottom_frame_idx(&mut self) -> usize {
    self.frames.len() - 1
  }

  /// The search depth of the bottom frame of this stack.
  pub fn bottom_depth(&self) -> u32 {
    self.root_depth + self.frames.len() as u32 - 1
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
