use abstract_game::GameMoveIterator;
use std::{
  fmt::Display,
  ptr::null_mut,
  sync::atomic::{AtomicU32, Ordering},
};

use abstract_game::{Game, Score};

use crate::transparent_iterator::TransparentIterator;

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
///
/// The type of a stack is either the root, which contains the initial game
/// state as it's first frame, or a child, which has a pointer to the parent
/// that it is solving a branch for.
pub enum StackType<G>
where
  G: Game,
{
  Root,
  Child { parent: *mut Stack<G> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

pub struct StackFrame<G>
where
  G: Game,
{
  /// The current game state at this frame.
  game: G,
  /// An iterator over the moves at this game state. If `None`, then no moves
  /// have been iterated over yet.
  move_gen: Option<G::MoveGenerator>,
  /// The current move being explored by the child of this frame.
  current_move: Option<G::Move>,
  /// The best score found for this game so far.
  best_score: Score,
  /// The corresponding best move found for `best_score`.
  best_move: Option<G::Move>,
  /// All stack frames have an unordered list of all of their suspended direct
  /// dependents. This can only be appended to under the bin mutex lock from the
  /// pending states hashmap, and reclaimed for revival after removing this
  /// frame from the pending states hashmap.
  dependents: *mut Stack<G>,
}

impl<G> StackFrame<G>
where
  G: Game + Display,
  G::Move: Display,
{
  pub fn new(game: G) -> Self {
    let mut s = Self {
      game,
      move_gen: None,
      current_move: None,
      best_score: Score::no_info(),
      best_move: None,
      dependents: null_mut(),
    };
    s.advance();
    s
  }

  pub fn game(&self) -> &G {
    &self.game
  }

  /// The current move to explore for this stack frame.
  pub fn current_move(&self) -> Option<G::Move> {
    self.current_move
  }

  pub fn best_score(&self) -> (Score, Option<G::Move>) {
    // The state should have been fully explored.
    debug_assert!(self.current_move.is_none());
    (
      if self.best_move.is_none() {
        // If there were no possible moves, then the game is considered lost for
        // the current player.
        Score::lose(1)
      } else {
        self.best_score.clone()
      },
      self.best_move,
    )
  }

  /// Updates the best score/move pair of this frame if `score` is better than
  /// the current best score, and advances the current move to the next move.
  fn update_score_and_advance(&mut self, score: Score) {
    if self.best_move.is_none() || score.better(&self.best_score) {
      // println!(
      //   "    Updating {} ({}) to {} ({}) for\n{}\n",
      //   if self.best_move.is_none() {
      //     "[None]".to_string()
      //   } else {
      //     self.best_move.unwrap().to_string()
      //   },
      //   self.best_score,
      //   self.current_move.unwrap(),
      //   score,
      //   self.game()
      // );
      self.best_score = score;
      self.best_move = self.current_move;
    } else {
      // println!(
      //   "    Not updating {} ({}) vs {} ({}) for\n{}\n",
      //   self.best_move.unwrap(),
      //   self.best_score,
      //   self.current_move.unwrap(),
      //   score,
      //   self.game()
      // );
    }
    self.advance();
  }

  pub unsafe fn queue_dependant_unlocked(&mut self, dependant: *mut Stack<G>) {
    unsafe {
      (*dependant).next = self.dependents;
    }
    self.dependents = dependant;
  }

  pub unsafe fn pop_dependant_unlocked(&mut self) -> Option<*mut Stack<G>> {
    let head = self.dependents;
    if head.is_null() {
      return None;
    }

    self.dependents = unsafe { (*head).next };
    Some(head)
  }

  /// Advances the current move to the next possible move.
  fn advance(&mut self) {
    let move_gen = match &mut self.move_gen {
      Some(move_gen) => move_gen,
      None => {
        self.move_gen = Some(self.game.move_generator());
        self.move_gen.as_mut().unwrap()
      }
    };
    self.current_move = move_gen.next(&self.game);
  }
}

/// Each task has a stack frame exactly large enough to hold enough frames for a
/// depth-first search of depth `N`.
pub struct Stack<G>
where
  G: Game,
{
  /// The search depth this stack frame will go out to starting from the root
  /// frame.
  root_depth: u32,
  /// The frames of this stack.
  frames: Vec<StackFrame<G>>,
  ty: StackType<G>,
  /// TODO: Can remove state? Implicit from where the stack lies in the data
  /// structure.
  state: StackState,
  /// For suspended states, this is a pointer to the next dependant suspended
  /// state of "dependant", forming a singly-linked list of the dependent
  /// states. This `next` pointer is modified under the bin-lock of the state
  /// that this stack is suspended on in `DashMap`. When the stack is
  /// unsuspended, there will not be any other threads that have references to
  /// this frame since queueing is done under a lock.
  next: *mut Stack<G>,
  /// A split state is a state with split children, upon whose completion will
  /// resolve the state at the bottom of the stack. It only tracks the number of
  /// outstanding children. The child to decrease this number to 0 is the one to
  /// revive the state.
  outstanding_children: AtomicU32,
}

impl<G> Stack<G>
where
  G: Game + Display + 'static,
  G::Move: Display,
{
  pub fn make_root(initial_game: G, depth: u32) -> Self {
    let mut root = Self {
      root_depth: depth,
      ty: StackType::Root,
      frames: Vec::with_capacity(depth as usize),
      state: StackState::Live {},
      next: null_mut(),
      outstanding_children: AtomicU32::new(0),
    };
    root.frames.push(StackFrame::new(initial_game));
    root
  }

  fn make_child(game: G, depth: u32, parent: *mut Self) -> Self {
    let mut root = Self {
      root_depth: depth,
      frames: Vec::with_capacity(depth as usize),
      ty: StackType::Child { parent },
      state: StackState::Live {},
      next: null_mut(),
      outstanding_children: AtomicU32::new(0),
    };
    root.frames.push(StackFrame::new(game));
    root
  }

  pub fn stack_type(&self) -> &StackType<G> {
    &self.ty
  }

  pub fn push(&mut self, game: G) {
    debug_assert!(!self.is_full());
    self.frames.push(StackFrame::new(game));
  }

  pub fn update_parent_score_and_advance(&mut self, score: Score) {
    if let Some(parent_frame) = self.frames.last_mut() {
      parent_frame.update_score_and_advance(score);
    }
  }

  /// To be called to resolve the bottom frame to the given score which is
  /// already relative to the parent frame. This will remove the bottom stack
  /// frame and update the score/current move of the parent stack frame.
  pub fn pop_with_backstepped_score(&mut self, score: Score) -> StackFrame<G> {
    let completed_frame = self.frames.pop().unwrap();
    self.update_parent_score_and_advance(score);
    completed_frame
  }

  /// To be called to resolve the bottom frame to the given score. This will
  /// remove the bottom stack frame and update the score/current move of the
  /// parent stack frame.
  pub fn pop_with_score(&mut self, score: Score) -> StackFrame<G> {
    self.pop_with_backstepped_score(score.backstep())
  }

  /// To be called when the bottom stack frame has resolved its score. This will
  /// remove the bottom stack frame and update the score/current move of the
  /// parent stack frame.
  pub fn pop(&mut self) -> StackFrame<G> {
    let completed_frame = self.frames.last().unwrap();
    self.pop_with_score(completed_frame.best_score().0.clone())
  }

  pub fn stack_state(&self) -> StackState {
    self.state
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
  pub fn split(self_ptr: *mut Self) -> impl Iterator<Item = Self> {
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
    let game = stack.bottom_frame().unwrap().game();
    game
      .each_move()
      .map(move |m| {
        let stack = unsafe { &mut *self_ptr };
        stack.outstanding_children.fetch_add(1, Ordering::Relaxed);
        let mut game = stack.bottom_frame().unwrap().game().clone();
        game.make_move(m);
        Self::make_child(game, stack.bottom_depth() - 1, self_ptr)
      })
      .chain(TransparentIterator::new(move || {
        let stack = unsafe { &mut *self_ptr };
        if stack.outstanding_children.fetch_sub(1, Ordering::Relaxed) == 0 {
          // TODO: All children have finished, revive this frame.
        }
      }))
  }

  /// TODO: try tracking the best score/move in the parent stack frame, protect
  /// those and outstanding_children with a lock, instead of re-iterating over
  /// the parent and relying on the children states to be in the resolved table.
  pub fn resolve_outstanding_child(self_ptr: *mut Self) {
    let stack = unsafe { &mut *self_ptr };
    if stack.outstanding_children.fetch_sub(1, Ordering::Relaxed) == 0 {
      // TODO: All children have finished, revive this frame.
    }
  }

  pub fn is_full(&self) -> bool {
    self.frames.len() == self.root_depth as usize
  }

  pub fn frame(&self, idx: u32) -> &StackFrame<G> {
    self.frames.get(idx as usize).unwrap()
  }

  pub fn frame_mut(&mut self, idx: u32) -> &mut StackFrame<G> {
    self.frames.get_mut(idx as usize).unwrap()
  }

  pub fn bottom_frame(&self) -> Option<&StackFrame<G>> {
    self.frames.last()
  }

  pub fn bottom_frame_mut(&mut self) -> Option<&mut StackFrame<G>> {
    self.frames.last_mut()
  }

  pub fn bottom_frame_idx(&self) -> usize {
    self.frames.len() - 1
  }

  /// The search depth of the bottom frame of this stack.
  pub fn bottom_depth(&self) -> u32 {
    self.root_depth - self.frames.len() as u32 + 1
  }
}
