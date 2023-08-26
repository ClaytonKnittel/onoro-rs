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
  G: Game + Hash + Eq + TableEntry,
  H: BuildHasher + Clone,
{
  loop {
    let guard = data.globals.collector().enter();
    let unit = data.queue.pop(&guard);

    let stack = match unit {
      Some(stack) => stack,
      None => break,
    };

    'seq: loop {
      let frame = unsafe { (*stack).bottom_frame() };

      if let Some(m) = frame.next_move() {
        let next_state = frame.game().with_move(m);
        unsafe { (*stack).push(next_state) };

        match data.globals.get_or_queue(stack) {
          LookupResult::Found { score } => {
            // Check if score is usable for this depth or not. If not, will
            // need to search again with deeper depth.
            todo!();
            // Update best score in frame
            // frame.maybe_update_score(score, m);
          }
          LookupResult::NotFound {} => {
            // Compute the score of the move. The set_ref is a reference to
            // the placeholder state in the set indicating that this state
            // is currently being computed.
            // unit.insert_frame(Frame::new(next_state, set_ref));
          }
          // If the state is found pending, then it will be added to the list
          // of states waiting on the result of some game state. After this
          // result is found (it is being processed by another worker), all
          // states which are pending are re-added to some worker's queue
          // (randomly distributed).
          PENDING => {
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
