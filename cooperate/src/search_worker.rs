use std::{
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

fn start_worker<G, H, const N: usize>(mut data: WorkerData<G, H, N>)
where
  G: Game + Hash + Eq + TableEntry + 'static,
  H: BuildHasher + Clone,
{
  loop {
    let guard = data.globals.collector().enter();
    let unit = data.queue.pop(&guard);

    let stack_ptr = match unit {
      Some(stack_ptr) => stack_ptr,
      None => break,
    };

    'seq: loop {
      // We own stack here, so we can access it without atomics.
      let stack = unsafe { &mut *stack_ptr };
      let frame = stack.bottom_frame_mut();

      if let Some(m) = frame.next_move() {
        let next_state = frame.game().with_move(m);
        stack.push(next_state);

        match data.globals.get_or_queue(stack_ptr) {
          LookupResult::Found { score } => {
            todo!();
            // Update best score in frame
            // frame.maybe_update_score(score, m);
          }
          LookupResult::NotFound => {
            // Compute the score of the move. The set_ref is a reference to
            // the placeholder state in the set indicating that this state
            // is currently being computed.
            // unit.insert_frame(Frame::new(next_state, set_ref));
          }
          // If the state was queued, then it was added to the list of states
          // waiting on the result of some game state. After this result is
          // found, all states which are pending are re-added to some worker's
          // queue (randomly distributed).
          LookupResult::Queued => {
            break 'seq;
          }
        }
      } else {
        // All moves have been explored. Update the table with the game's
        // now-known score, and re-queue all pending units.
        todo!()
      }
    }
  }
}
