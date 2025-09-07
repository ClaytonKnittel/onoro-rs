use std::{
  fmt::Display,
  hash::{BuildHasher, Hash},
  sync::Arc,
};

use abstract_game::{Game, GameResult, Score};

use crate::{
  global_data::{GlobalData, LookupResult},
  stack::{Stack, StackType},
  Metrics,
};

pub struct WorkerData<G, H>
where
  G: Game,
{
  /// Index of this worker thread, which corresponds to the position of the
  /// thread's queue in the globals struct.
  thread_idx: u32,

  globals: Arc<GlobalData<G, H>>,
  metrics: Metrics,
}

impl<G, H> WorkerData<G, H>
where
  G: Game,
{
  pub fn new(thread_idx: u32, globals: Arc<GlobalData<G, H>>) -> Self {
    Self {
      thread_idx,
      globals,
      metrics: Metrics::new(),
    }
  }
}

pub fn start_worker<G, H>(mut data: WorkerData<G, H>)
where
  G: Display + Game + Hash + Eq + 'static,
  G::Move: Display,
  H: BuildHasher + Clone,
{
  let queue = data.globals.queue(data.thread_idx);

  loop {
    let unit = queue.pop();

    let stack_ptr = match unit {
      Some(stack_ptr) => *stack_ptr,
      // TODO: steal
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
            Stack::resolve_outstanding_child(*parent);
          }
        }

        // Delete the stack pointer.
        unsafe { drop(Box::from_raw(stack_ptr)) };
        break;
      }

      // println!(
      //   "\n[{}] Exploring\n{}\n(depth {})",
      //   data.thread_idx,
      //   stack.bottom_frame().unwrap().game(),
      //   unsafe { &mut *stack_ptr }.bottom_depth()
      // );

      let bottom_frame = stack.bottom_frame().unwrap();
      let game = bottom_frame.game();
      let game_result = game.finished();
      match game_result {
        GameResult::Win(winner) => {
          // Since scores indicating a player is currently winning are not
          // representable, we construct scores for the parent of this frame that
          // indicate the opposite player will can in one turn.
          let score_for_parent = if winner == game.current_player() {
            // If the current player is winning, then in the parent frame, the
            // current player (the other player in this frame) is losing next
            // turn.
            Score::lose(1)
          } else {
            // If the current player is losing, then in the parent frame, the
            // current player (the other player in this frame) is winning next
            // turn.
            Score::win(1)
          };

          // println!(
          //   "    [{}] parent score is {score_for_parent}",
          //   data.thread_idx
          // );
          stack.pop_with_backstepped_score(score_for_parent);
        }
        GameResult::Tie => {
          // println!(
          //   "    [{}] parent score is {}",
          //   data.thread_idx,
          //   Score::guaranteed_tie()
          // );
          stack.pop_with_backstepped_score(Score::guaranteed_tie());
        }
        GameResult::NotFinished => {
          // First, check if there is an immediate winning move.

          match data.globals.get_or_queue(stack_ptr, &mut data.metrics) {
            LookupResult::Found { score } => {
              // Update best score in frame
              // println!("    [{}] Found", data.thread_idx);
              stack.pop_with_score(score);
            }
            // If the state was not found, then we can continue on exploring it.
            LookupResult::NotFound => {
              // println!("    [{}] Inserted placeholder in table", data.thread_idx);
            }
            // If the state was queued, then it was added to the list of states
            // waiting on the result of some game state. After this result is
            // found, all states which are pending are re-added to some worker's
            // queue (randomly distributed).
            LookupResult::Queued => {
              // println!("    [{}] Queued on other state", data.thread_idx);
              break;
            }
          }
        }
      }

      data.globals.explore_next_state(stack_ptr, queue);
    }
  }

  // println!("Worker {} done: {:?}", data.thread_idx, data.metrics);
}

#[cfg(test)]
mod tests {
  use std::{sync::Arc, time::SystemTime};

  use abstract_game::{Game, GameResult};

  use crate::{
    global_data::GlobalData,
    null_lock::NullLock,
    stack::Stack,
    test::{
      gomoku::Gomoku,
      nim::Nim,
      serial_search::{find_best_move_serial, find_best_move_serial_table},
      tic_tac_toe::Ttt,
    },
  };

  use super::{start_worker, WorkerData};

  #[test]
  fn test_nim_serial() {
    const STICKS: u32 = 100;
    let globals = Arc::new(GlobalData::new(STICKS + 1, 1));
    globals.queue(0).push(unsafe {
      NullLock::new(Box::into_raw(Box::new(Stack::make_root(
        Nim::new(STICKS),
        STICKS + 1,
      ))))
    });

    start_worker(WorkerData::new(0, globals.clone()));

    for sticks in 1..=STICKS {
      let cached_score = globals.resolved_states_table().get(&Nim::new(sticks));
      assert!(cached_score.is_some());
      assert_eq!(cached_score.unwrap(), Nim::new(sticks).expected_score());
    }
  }

  #[test]
  fn test_ttt_serial() {
    const DEPTH: u32 = 10;
    let globals = Arc::new(GlobalData::new(DEPTH, 1));
    globals
      .queue(0)
      .push(unsafe { NullLock::new(Box::into_raw(Box::new(Stack::make_root(Ttt::new(), DEPTH)))) });

    start_worker(WorkerData::new(0, globals.clone()));

    // The table should contain the completed initial state.
    assert!(globals
      .resolved_states_table()
      .table()
      .contains_key(&Ttt::new()));

    for state in globals.resolved_states_table().table().iter() {
      // Terminal states should not be stored in the table.
      assert_eq!(state.key().finished(), GameResult::NotFinished);

      // Compute the score using a simple min-max search.
      let expected_score = state.key().compute_expected_score(DEPTH);

      // We can't expect the scores to be equal, since the score from the
      // algorithm may not be complete (i.e. there's a win in X turns, but we're
      // unsure if there's a way to win in fewer turns). We expect them to be
      // compatible.
      assert!(
        state.value().compatible(&expected_score),
        "Expect computed score {} to be compatible with true score {}",
        state.value(),
        expected_score
      );
    }
  }

  #[test]
  #[ignore]
  fn test_gomoku_4x4_serial() {
    const DEPTH: u32 = 16;
    let globals = Arc::new(GlobalData::new(DEPTH, 1));
    globals.queue(0).push(unsafe {
      NullLock::new(Box::into_raw(Box::new(Stack::make_root(
        Gomoku::new(4, 4, 4),
        DEPTH,
      ))))
    });

    println!("Solving...");
    let start = SystemTime::now();
    start_worker(WorkerData::new(0, globals.clone()));
    let end = SystemTime::now();
    println!("Done: {:?}", end.duration_since(start).unwrap());

    // The table should contain the completed initial state.
    assert!(globals
      .resolved_states_table()
      .table()
      .contains_key(&Gomoku::new(4, 4, 4)));

    let table = find_best_move_serial(&Gomoku::new(4, 4, 4), DEPTH).2;

    for state in globals.resolved_states_table().table().iter() {
      // Terminal states should not be stored in the table.
      assert_eq!(state.key().finished(), GameResult::NotFinished);

      let expected_score = table.get(state.key()).unwrap_or_else(|| {
        find_best_move_serial_table(state.key(), DEPTH, &table);
        table.get(state.key()).unwrap()
      });

      assert!(
        state.value().compatible(&expected_score),
        "Expect computed score {} to be compatible with true score {} for state\n{}",
        state.value(),
        expected_score,
        state.key()
      );
    }
  }
}
