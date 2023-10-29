use std::{
  collections::hash_map::RandomState,
  fmt::Display,
  hash::Hash,
  sync::{atomic::Ordering, Arc},
};

use abstract_game::Game;
use seize::{reclaim, AtomicPtr};

use crate::{
  global_data::GlobalData,
  search_worker::{start_worker, WorkerData},
  stack::Stack,
  table::TableEntry,
};

pub struct Options {
  /// The number of worker threads to use in the thread pool.
  num_threads: u32,
  /// The depth to search the game to.
  search_depth: u32,
  /// The depth to expand to for generating work units.
  unit_depth: u32,
}

fn construct_globals<G>(game: &G, options: Options) -> Arc<GlobalData<G, RandomState>>
where
  G: Game + TableEntry + Display + Hash + PartialEq + Eq + 'static,
  G::Move: Display,
{
  let globals = Arc::new(GlobalData::new(options.search_depth, options.num_threads));

  let stack = AtomicPtr::new(
    globals
      .collector()
      .link_boxed(Stack::make_root(game.clone(), options.search_depth)),
  );
  globals.queue(0).push(stack.load(Ordering::Relaxed));

  globals
}

pub fn solve<G>(game: &G, options: Options)
where
  G: Game + TableEntry + Display + Hash + PartialEq + Eq + 'static,
  G::Move: Display,
{
  let globals = construct_globals(game, options);
  start_worker(WorkerData::new(0, globals.clone()));
}

#[cfg(test)]
mod tests {
  use crate::{
    cooperate::construct_globals,
    search_worker::{start_worker, WorkerData},
    table::TableEntry,
    test::nim::Nim,
  };

  #[test]
  fn test_nim_serial() {
    const STICKS: usize = 100;

    let globals = construct_globals(
      &Nim::new(STICKS as u32),
      crate::Options {
        search_depth: STICKS as u32 + 1,
        num_threads: 1,
        unit_depth: 0,
      },
    );

    start_worker(WorkerData::new(0, globals.clone()));

    for sticks in 1..=STICKS as u32 {
      let game = globals.resolved_states_table().get(&Nim::new(sticks));
      assert!(game.is_some());
      let game = game.unwrap();
      assert_eq!(game.score(), game.expected_score());
    }
  }
}
