use std::{
  fmt::Display,
  hash::{BuildHasher, Hash},
  sync::Arc,
};

use abstract_game::{Game, GameResult, Score};
use seize::reclaim;

use crate::{
  global_data::{GlobalData, LookupResult},
  stack::{Stack, StackType},
  table::TableEntry,
};

pub struct WorkerData<G, H>
where
  G: Game,
{
  /// Index of this worker thread, which corresponds to the position of the
  /// thread's queue in the globals struct.
  thread_idx: u32,

  globals: Arc<GlobalData<G, H>>,
}

impl<G, H> WorkerData<G, H>
where
  G: Game,
{
  pub fn new(thread_idx: u32, globals: Arc<GlobalData<G, H>>) -> Self {
    Self {
      thread_idx,
      globals,
    }
  }
}

pub fn start_worker<G, H>(data: WorkerData<G, H>)
where
  G: Display + Game + Hash + Eq + TableEntry + 'static,
  G::Move: Display,
  H: BuildHasher + Clone,
{
  let queue = data.globals.queue(data.thread_idx);

  loop {
    let guard = data.globals.collector().enter();
    let unit = queue.pop(&guard);

    let stack_ptr = match unit {
      Some(stack_ptr) => stack_ptr,
      None => break,
    };
    // We own stack here, so we can access it without atomics.
    let stack = unsafe { &mut *stack_ptr };

    loop {
      if stack.bottom_frame().is_none() {
        // We've finished exploring this stack frame.
        match stack.stack_type() {
          StackType::Root => {}
          StackType::Child { parent } => {
            Stack::resolve_outstanding_child(parent);
          }
        }

        unsafe {
          data
            .globals
            .collector()
            .retire(stack_ptr, reclaim::boxed::<Stack<G>>);
        }
        break;
      }

      // println!(
      //   "\nExploring\n{}\n(depth {})",
      //   stack.bottom_frame().unwrap().game(),
      //   unsafe { &mut *stack_ptr }.bottom_depth()
      // );

      let game = stack.bottom_frame().unwrap().game();
      let game_result = game.finished();
      if game_result != GameResult::NotFinished {
        // Since scores indicating a player is currently winning are not
        // representable, we construct scores for the parent of this frame that
        // indicate the opposite player will can in one turn.
        let score_for_parent = if let GameResult::Win(winner) = game_result {
          if winner == game.current_player() {
            // If the current player is winning, then in the parent frame, the
            // current player (the other player in this frame) is losing next
            // turn.
            Score::lose(1)
          } else {
            // If the current player is losing, then in the parent frame, the
            // current player (the other player in this frame) is winning next
            // turn.
            Score::win(1)
          }
        } else {
          Score::guaranteed_tie()
        };
        // println!("    parent score is {score_for_parent}");
        stack.pop_with_backstepped_score(score_for_parent);
      } else {
        match data.globals.get_or_queue(stack_ptr) {
          LookupResult::Found { score } => {
            // Update best score in frame
            // println!("    Found",);
            stack.pop_with_score(score);
          }
          // If the state was not found, then we can continue on exploring it.
          LookupResult::NotFound => {
            // println!("    Inserted placeholder in table");
          }
          // If the state was queued, then it was added to the list of states
          // waiting on the result of some game state. After this result is
          // found, all states which are pending are re-added to some worker's
          // queue (randomly distributed).
          LookupResult::Queued => {
            // println!("    Queued on other state");
            break;
          }
        }
      }

      data.globals.explore_next_state(stack_ptr, queue);
    }
  }
}

#[cfg(test)]
mod tests {
  use std::sync::{atomic::Ordering, Arc};

  use abstract_game::{Game, GameResult};
  use seize::AtomicPtr;

  use crate::{
    global_data::GlobalData,
    stack::Stack,
    table::TableEntry,
    test::{nim::Nim, search::find_best_move_serial, tic_tac_toe::Ttt},
    Metrics,
  };

  use super::{start_worker, WorkerData};

  #[test]
  fn test_nim_serial() {
    const STICKS: usize = 100;
    let globals = Arc::new(GlobalData::new(STICKS as u32 + 1, 1));

    let stack = AtomicPtr::new(
      globals
        .collector()
        .link_boxed(Stack::make_root(Nim::new(STICKS as u32), STICKS as u32 + 1)),
    );
    globals.queue(0).push(stack.load(Ordering::Relaxed));

    start_worker(WorkerData {
      thread_idx: 0,
      globals: globals.clone(),
    });

    for sticks in 1..=STICKS as u32 {
      let game = globals.resolved_states_table().get(&Nim::new(sticks));
      assert!(game.is_some());
      let game = game.unwrap();
      assert_eq!(game.score(), game.expected_score());
    }
  }

  #[test]
  fn test_ttt_serial() {
    const DEPTH: usize = 10;
    let globals = Arc::new(GlobalData::new(10, 1));

    let stack = AtomicPtr::new(
      globals
        .collector()
        .link_boxed(Stack::make_root(Ttt::new(), DEPTH as u32)),
    );
    globals.queue(0).push(stack.load(Ordering::Relaxed));

    start_worker(WorkerData {
      thread_idx: 0,
      globals: globals.clone(),
    });

    // The table should contain the completed initial state.
    assert!(globals
      .resolved_states_table()
      .table()
      .contains(&Ttt::new()));

    for state in globals.resolved_states_table().table().iter() {
      // Terminal states should not be stored in the table.
      assert_eq!(state.key().finished(), GameResult::NotFinished);

      // Compute the score using a simple min-max search.
      let expected_score = find_best_move_serial(state.key(), DEPTH as u32, &mut Metrics::new())
        .0
        .unwrap();

      // We can't expect the scores to be equal, since the score from the
      // algorithm may not be complete (i.e. there's a win in X turns, but we're
      // unsure if there's a way to win in fewer turns). We expect them to be
      // compatible.
      assert!(
        state.score().compatible(&expected_score),
        "Expect computed score {} to be compatible with true score {}",
        state.score(),
        expected_score
      );
    }
  }
}
