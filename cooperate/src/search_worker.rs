use std::{
  fmt::Display,
  hash::{BuildHasher, Hash},
  sync::Arc,
};

use abstract_game::{Game, Score};

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

    'seq: loop {
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
        "Exploring {} (depth {})",
        stack.bottom_frame().unwrap().game(),
        unsafe { &mut *stack_ptr }.bottom_depth()
      );

      let game = stack.bottom_frame().unwrap().game();
      if let Some(winner) = game.finished() {
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
            break 'seq;
          }
        }
      }

      data.globals.explore_next_state(stack_ptr, &data.queue);
    }
  }
}

#[cfg(test)]
mod tests {
  use std::{
    collections::hash_map::RandomState,
    fmt::Display,
    hash::Hash,
    sync::{atomic::Ordering, Arc},
  };

  use abstract_game::{Game, Score};
  use seize::AtomicPtr;

  use crate::{global_data::GlobalData, queue::Queue, stack::Stack, table::TableEntry};

  use super::{start_worker, WorkerData};

  #[derive(PartialEq, Eq)]
  enum NimPlayer {
    First,
    Second,
  }

  #[derive(Clone, Copy)]
  struct NimMove {
    sticks: u32,
  }

  impl Display for NimMove {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.sticks)
    }
  }

  struct NimMoveIter {
    sticks: u32,
    max_sticks: u32,
  }

  impl Iterator for NimMoveIter {
    type Item = NimMove;

    fn next(&mut self) -> Option<Self::Item> {
      if self.sticks > self.max_sticks {
        None
      } else {
        self.sticks += 1;
        Some(NimMove {
          sticks: self.sticks - 1,
        })
      }
    }
  }

  #[derive(Clone)]
  struct Nim {
    sticks: u32,
    turn: u32,
    score: Score,
  }

  impl Nim {
    fn new(sticks: u32) -> Self {
      Self {
        sticks,
        turn: 0,
        score: Score::no_info(),
      }
    }
  }

  impl Game for Nim {
    type Move = NimMove;
    type MoveIterator = NimMoveIter;
    type PlayerIdentifier = NimPlayer;

    fn each_move(&self) -> Self::MoveIterator {
      NimMoveIter {
        sticks: 1,
        max_sticks: self.sticks.min(2),
      }
    }

    fn make_move(&mut self, m: Self::Move) {
      self.sticks -= m.sticks;
      self.turn += 1;
    }

    fn current_player(&self) -> Self::PlayerIdentifier {
      if self.turn % 2 == 0 {
        NimPlayer::First
      } else {
        NimPlayer::Second
      }
    }

    fn finished(&self) -> Option<Self::PlayerIdentifier> {
      if self.sticks == 0 {
        // The winner is the player to take the last stick.
        if self.turn % 2 == 0 {
          Some(NimPlayer::Second)
        } else {
          Some(NimPlayer::First)
        }
      } else {
        None
      }
    }
  }

  impl TableEntry for Nim {
    fn score(&self) -> abstract_game::Score {
      self.score.clone()
    }

    fn set_score(&mut self, score: Score) {
      self.score = score;
    }

    fn merge(&mut self, other: &Self) {
      self.score.merge(&other.score);
    }
  }

  impl Hash for Nim {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
      self.sticks.hash(state);
    }
  }

  impl PartialEq for Nim {
    fn eq(&self, other: &Self) -> bool {
      self.sticks == other.sticks
    }
  }

  impl Eq for Nim {}

  impl Display for Nim {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{} (turn {})", self.sticks, self.turn)
    }
  }

  #[test]
  fn test_basic() {
    const STICKS: usize = 100;
    const STICKS_P_1: usize = STICKS + 1;
    let globals = Arc::new(GlobalData::<_, _, STICKS_P_1>::new(RandomState::new()));
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
      let game = game.unwrap().clone();
      if sticks % 3 == 0 {
        let turn_count_win = sticks * 2 / 3;
        assert_eq!(
          game.score(),
          Score::new(false, turn_count_win - 1, turn_count_win),
          "game: {}",
          game
        );
      } else {
        let turn_count_win = sticks / 3 * 2;
        assert_eq!(
          game.score(),
          Score::new(true, turn_count_win, turn_count_win + 1),
          "game: {}",
          game
        );
      }
    }
  }
}
