use std::{
  fmt::Display,
  hash::{BuildHasher, Hash},
  sync::Arc,
};

use abstract_game::{Game, GameResult, Score};

use crate::{
  global_data::{GlobalData, LookupResult},
  queue::Queue,
  stack::{Stack, StackType},
  table::TableEntry,
};

struct WorkerData<G, H, const N: usize>
where
  G: Game,
{
  /// The queue of frames local to this worker thread. This can be "stolen" from
  /// by other workers when they run out of work to do.
  queue: Queue<Stack<G, N>>,
  globals: Arc<GlobalData<G, H, N>>,
}

fn start_worker<G, H, const N: usize>(data: WorkerData<G, H, N>)
where
  G: Display + Game + Hash + Eq + TableEntry + 'static,
  G::Move: Display,
  H: BuildHasher + Clone,
{
  loop {
    let guard = data.globals.collector().enter();
    let unit = data.queue.pop(&guard);

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
          StackType::Root => {
            break;
          }
          StackType::Child { parent } => {
            Stack::resolve_outstanding_child(parent);
          }
        }
        break;
      }

      println!(
        "\nExploring\n{}\n(depth {})",
        stack.bottom_frame().unwrap().game(),
        unsafe { &mut *stack_ptr }.bottom_depth()
      );

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
          Score::tie(1)
        };
        println!("    parent score is {score_for_parent}");
        stack.pop_with_backstepped_score(score_for_parent);
      } else {
        match data.globals.get_or_queue(stack_ptr) {
          LookupResult::Found { score } => {
            // Update best score in frame
            println!("    Found",);
            stack.pop_with_score(score);
          }
          // If the state was not found, then we can continue on exploring it.
          LookupResult::NotFound => {
            println!("    Inserted placeholder in table");
          }
          // If the state was queued, then it was added to the list of states
          // waiting on the result of some game state. After this result is
          // found, all states which are pending are re-added to some worker's
          // queue (randomly distributed).
          LookupResult::Queued => {
            println!("    Queued on other state");
            break;
          }
        }
      }

      data.globals.explore_next_state(stack_ptr, &data.queue);
    }
  }
}

#[cfg(test)]
mod tests {
  use std::sync::{atomic::Ordering, Arc};

  use seize::AtomicPtr;

  use crate::{
    global_data::GlobalData,
    queue::Queue,
    stack::Stack,
    table::TableEntry,
    test::{nim::Nim, tic_tac_toe::Ttt},
  };

  use super::{start_worker, WorkerData};

  #[test]
  fn test_nim_serial() {
    const STICKS: usize = 100;
    const STICKS_P_1: usize = STICKS + 1;
    let globals = Arc::new(GlobalData::<_, _, STICKS_P_1>::new());
    let queue = Queue::new();

    let stack = AtomicPtr::new(
      globals
        .collector()
        .link_boxed(Stack::make_root(Nim::new(STICKS as u32), STICKS as u32 + 1)),
    );
    queue.push(stack.load(Ordering::Relaxed));
    let d = WorkerData {
      queue,
      globals: globals.clone(),
    };

    start_worker(d);

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
    let globals = Arc::new(GlobalData::<_, _, DEPTH>::new());
    let queue = Queue::new();

    let stack = AtomicPtr::new(
      globals
        .collector()
        .link_boxed(Stack::make_root(Ttt::new(), DEPTH as u32)),
    );
    queue.push(stack.load(Ordering::Relaxed));
    let d = WorkerData {
      queue,
      globals: globals.clone(),
    };

    start_worker(d);

    for state in globals.resolved_states_table().table().iter() {
      println!("{}\n{}", state.score(), *state);
    }
  }
}
