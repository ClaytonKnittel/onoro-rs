use abstract_game::Game;

mod global_data;
mod metrics;
mod queue;
mod search;
mod search_worker;
mod stack;
mod table;
mod test;
mod util;

pub use metrics::*;

pub struct Options {
  /// The number of worker threads to use in the thread pool.
  num_threads: u32,
  /// The depth to expand to for generating work units.
  unit_depth: u32,
}

pub fn solve<G: Game>(game: &G, options: Options) {}
