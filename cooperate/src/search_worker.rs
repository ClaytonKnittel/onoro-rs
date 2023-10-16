use std::{
  fmt::Display,
  hash::{BuildHasher, Hash},
  sync::Arc,
};

use abstract_game::Game;

use crate::{
  global_data::{GlobalData, LookupResult},
  queue::Queue,
  stack::Stack,
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
      let frame = stack.bottom_frame_mut();
      println!("Exploring {}", frame.game());

      if let Some(m) = frame.next_move() {
        println!("  move {}", m);
        let next_state = frame.game().with_move(m);
        // This is unsafe because we are modifying the stack and using `frame`
        // later, whose lifetime depends on stack. However, we know that no
        // references will be invalidated, so it is safe.
        unsafe { &mut *stack_ptr }.push(next_state);

        match data.globals.get_or_queue(stack_ptr) {
          LookupResult::Found { score } => {
            // Update best score in frame
            print!(
              "    Found, updating score from {}, {} to ",
              frame.best_score().0,
              match frame.best_score().1 {
                Some(m) => m.to_string(),
                None => "[None]".to_string(),
              }
            );
            frame.maybe_update_score(score, m);
            println!(
              "{}, {}",
              frame.best_score().0,
              match frame.best_score().1 {
                Some(m) => m.to_string(),
                None => "[None]".to_string(),
              }
            );
            stack.pop();
          }
          // If the state was not found, then we can continue on exploring it.
          LookupResult::NotFound => {
            println!("    Not found in table");
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
      } else {
        let stack = unsafe { &mut *stack_ptr };
        let bottom_state = stack.bottom_frame();
        let game = bottom_state.game();
        println!("  Out of moves, committing score {}", game.score());
        // All moves have been explored. Update the table with the game's
        // now-known score, and re-queue all pending units.
        data.globals.commit_score(stack_ptr, &data.queue);
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use std::{
    fmt::Display,
    hash::Hash,
    sync::{atomic::Ordering, Arc},
  };

  use abstract_game::{Game, Score};
  use seize::AtomicPtr;

  use crate::{global_data::GlobalData, queue::Queue, stack::Stack, table::TableEntry};

  use super::{start_worker, WorkerData};

  struct PassThroughHasher {
    state: u64,
  }

  impl std::hash::Hasher for PassThroughHasher {
    fn write(&mut self, bytes: &[u8]) {
      debug_assert!(bytes.len() == 8 && self.state == 0);
      self.state = unsafe { *(bytes.as_ptr() as *const u64) };
    }

    fn finish(&self) -> u64 {
      self.state
    }
  }

  #[derive(Clone)]
  struct BuildPassThroughHasher;

  impl std::hash::BuildHasher for BuildPassThroughHasher {
    type Hasher = PassThroughHasher;
    fn build_hasher(&self) -> PassThroughHasher {
      PassThroughHasher { state: 0 }
    }
  }

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
      if self.sticks == self.max_sticks {
        None
      } else {
        self.sticks += 1;
        Some(NimMove {
          sticks: self.sticks - 1,
        })
      }
    }
  }

  #[derive(Clone, PartialEq, Eq)]
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

    fn merge(&mut self, other: &Self) {
      self.score.merge(&other.score);
    }
  }

  impl Hash for Nim {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
      (self.sticks as u64 + self.turn as u64 * 0x1_0000_0000).hash(state);
    }
  }

  impl Display for Nim {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{} (turn {})", self.sticks, self.turn)
    }
  }

  #[test]
  fn test_basic() {
    let globals = Arc::new(GlobalData::<_, _, 11>::new(BuildPassThroughHasher));
    let queue = Queue::new();

    let stack = AtomicPtr::new(
      globals
        .collector()
        .link_boxed(Stack::make_root(Nim::new(10), 11)),
    );
    queue.push(stack.load(Ordering::Relaxed));
    let d = WorkerData { queue, globals };

    start_worker(d);
  }
}
