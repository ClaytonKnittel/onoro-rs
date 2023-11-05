use std::{
  collections::{hash_map::RandomState, HashSet},
  fmt::{Debug, Display},
  hash::Hash,
  sync::{atomic::Ordering, Arc},
};

use abstract_game::Game;
use rand::prelude::*;
use seize::{AtomicPtr, Collector};

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

fn generate_frontier<G>(
  initial_state: G,
  options: &Options,
  collector: &Collector,
) -> Vec<AtomicPtr<Stack<G>>>
where
  G: Game + TableEntry + Hash + PartialEq + Eq + Display + 'static,
  G::Move: Display,
{
  let mut visited_states = HashSet::new();
  let mut frontier = vec![initial_state];

  for _ in 0..options.unit_depth {
    let mut next_frontier = Vec::new();

    for state in frontier.into_iter() {
      for m in state.each_move() {
        let child = state.with_move(m);
        if visited_states.insert(child.clone()) {
          next_frontier.push(child);
        }
      }
    }

    frontier = next_frontier;
  }

  frontier
    .into_iter()
    .map(|state| {
      AtomicPtr::new(collector.link_boxed(Stack::make_root(
        state,
        options.search_depth - options.unit_depth,
      )))
    })
    .collect()
}

fn construct_globals<G>(game: &G, options: Options) -> Arc<GlobalData<G, RandomState>>
where
  G: Game + TableEntry + Display + Hash + PartialEq + Eq + 'static,
  G::Move: Display,
  G::PlayerIdentifier: Debug,
{
  let globals = Arc::new(GlobalData::new(options.search_depth, options.num_threads));

  let mut rng = thread_rng();
  for stack in generate_frontier(game.clone(), &options, globals.collector()).into_iter() {
    let rand_idx = rng.gen_range(0..options.num_threads);
    globals.queue(rand_idx).push(stack.load(Ordering::Relaxed));
  }

  globals
}

pub fn solve<G>(game: &G, options: Options)
where
  G: Game + TableEntry + Display + Hash + PartialEq + Eq + 'static,
  G::Move: Display,
  G::PlayerIdentifier: Debug,
{
  let globals = construct_globals(game, options);
  start_worker(WorkerData::new(0, globals.clone()));
}

#[cfg(test)]
mod tests {
  use std::{thread, time::SystemTime};

  use abstract_game::{Game, GameResult};

  use crate::{
    cooperate::construct_globals,
    search_worker::{start_worker, WorkerData},
    table::TableEntry,
    test::{
      gomoku::Gomoku,
      nim::Nim,
      search::{do_find_best_move_serial, find_best_move_serial},
      tic_tac_toe::Ttt,
    },
  };

  #[test]
  fn test_nim_serial() {
    const STICKS: u32 = 100;

    let globals = construct_globals(
      &Nim::new(STICKS),
      crate::Options {
        search_depth: STICKS + 1,
        num_threads: 1,
        unit_depth: 0,
      },
    );

    start_worker(WorkerData::new(0, globals.clone()));

    for sticks in 1..=STICKS {
      let game = globals.resolved_states_table().get(&Nim::new(sticks));
      assert!(game.is_some());
      let game = game.unwrap();
      assert_eq!(game.score(), game.expected_score());
    }
  }

  #[test]
  fn test_nim_p2() {
    const STICKS: u32 = 100;

    let globals = construct_globals(
      &Nim::new(STICKS),
      crate::Options {
        search_depth: STICKS + 1,
        num_threads: 2,
        unit_depth: 1,
      },
    );

    let thread_handles: Vec<_> = (0..2)
      .map(|thread_idx| {
        let globals = globals.clone();
        thread::spawn(move || {
          start_worker(WorkerData::new(thread_idx, globals));
        })
      })
      .collect();

    for thread in thread_handles.into_iter() {
      assert!(thread.join().is_ok());
    }

    for sticks in 1..=(STICKS - 1) {
      let game = globals.resolved_states_table().get(&Nim::new(sticks));
      assert!(game.is_some());
      let game = game.unwrap();
      assert_eq!(game.score(), game.expected_score());
    }
  }

  #[test]
  fn test_ttt_p2() {
    const DEPTH: u32 = 10;
    const THREADS: u32 = 2;

    let globals = construct_globals(
      &Ttt::new(),
      crate::Options {
        search_depth: DEPTH,
        num_threads: THREADS,
        unit_depth: 1,
      },
    );

    let thread_handles: Vec<_> = (0..THREADS)
      .map(|thread_idx| {
        let globals = globals.clone();
        thread::Builder::new()
          .name(format!("worker_{thread_idx}"))
          .spawn(move || {
            start_worker(WorkerData::new(thread_idx, globals));
          })
          .unwrap()
      })
      .collect();

    let mut any_bad = false;
    for thread in thread_handles.into_iter() {
      any_bad = thread.join().is_err() || any_bad;
    }
    assert!(!any_bad);

    for state in globals.resolved_states_table().table().iter() {
      // Terminal states should not be stored in the table.
      assert_eq!(state.key().finished(), GameResult::NotFinished);

      // Compute the score using a simple min-max search.
      let expected_score = state.compute_expected_score(DEPTH);

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

  #[test]
  fn test_ttt_p8() {
    const DEPTH: u32 = 10;
    const THREADS: u32 = 8;

    let globals = construct_globals(
      &Ttt::new(),
      crate::Options {
        search_depth: DEPTH,
        num_threads: THREADS,
        unit_depth: 2,
      },
    );

    let thread_handles: Vec<_> = (0..THREADS)
      .map(|thread_idx| {
        let globals = globals.clone();
        thread::Builder::new()
          .name(format!("worker_{thread_idx}"))
          .spawn(move || {
            start_worker(WorkerData::new(thread_idx, globals));
          })
          .unwrap()
      })
      .collect();

    let mut any_bad = false;
    for thread in thread_handles.into_iter() {
      any_bad = thread.join().is_err() || any_bad;
    }
    assert!(!any_bad);

    for state in globals.resolved_states_table().table().iter() {
      // Terminal states should not be stored in the table.
      assert_eq!(state.key().finished(), GameResult::NotFinished);

      // Compute the score using a simple min-max search.
      let expected_score = state.compute_expected_score(DEPTH);

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

  #[test]
  #[ignore]
  fn test_gomoku_p2() {
    const DEPTH: u32 = 16;
    const THREADS: u32 = 2;

    let globals = construct_globals(
      &Gomoku::new(4, 4, 4),
      crate::Options {
        search_depth: DEPTH,
        num_threads: THREADS,
        unit_depth: 3,
      },
    );

    println!("Solving...");
    let start = SystemTime::now();
    let thread_handles: Vec<_> = (0..THREADS)
      .map(|thread_idx| {
        let globals = globals.clone();
        thread::Builder::new()
          .name(format!("worker_{thread_idx}"))
          .spawn(move || {
            start_worker(WorkerData::new(thread_idx, globals));
          })
          .unwrap()
      })
      .collect();

    let mut any_bad = false;
    for thread in thread_handles.into_iter() {
      any_bad = thread.join().is_err() || any_bad;
    }
    let end = SystemTime::now();
    println!("Done: {:?}", end.duration_since(start).unwrap());

    assert!(!any_bad);

    // Compute the ground truth table.
    let mut table = find_best_move_serial(&Gomoku::new(4, 4, 4), DEPTH).2;

    for state in globals.resolved_states_table().table().iter() {
      // Terminal states should not be stored in the table.
      assert_eq!(state.key().finished(), GameResult::NotFinished);

      let expected_score = table
        .get(state.key())
        .map(|game_ref| game_ref.score())
        .unwrap_or_else(|| {
          do_find_best_move_serial(state.key(), DEPTH, &mut table);
          table.get(state.key()).unwrap().score()
        });

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

  #[test]
  #[ignore]
  fn test_gomoku_p8() {
    const DEPTH: u32 = 16;
    const THREADS: u32 = 8;

    let globals = construct_globals(
      &Gomoku::new(4, 4, 4),
      crate::Options {
        search_depth: DEPTH,
        num_threads: THREADS,
        unit_depth: 3,
      },
    );

    println!("Solving...");
    let start = SystemTime::now();
    let thread_handles: Vec<_> = (0..THREADS)
      .map(|thread_idx| {
        let globals = globals.clone();
        thread::Builder::new()
          .name(format!("worker_{thread_idx}"))
          .spawn(move || {
            start_worker(WorkerData::new(thread_idx, globals));
          })
          .unwrap()
      })
      .collect();

    let mut any_bad = false;
    for thread in thread_handles.into_iter() {
      any_bad = thread.join().is_err() || any_bad;
    }
    let end = SystemTime::now();
    println!("Done: {:?}", end.duration_since(start).unwrap());

    assert!(!any_bad);

    // Compute the ground truth table.
    let mut table = find_best_move_serial(&Gomoku::new(4, 4, 4), DEPTH).2;

    for state in globals.resolved_states_table().table().iter() {
      // Terminal states should not be stored in the table.
      assert_eq!(state.key().finished(), GameResult::NotFinished);

      let expected_score = table
        .get(state.key())
        .map(|game_ref| game_ref.score())
        .unwrap_or_else(|| {
          do_find_best_move_serial(state.key(), DEPTH, &mut table);
          table.get(state.key()).unwrap().score()
        });

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

  #[test]
  #[ignore]
  fn test_gomoku_p32() {
    const DEPTH: u32 = 16;
    const THREADS: u32 = 32;

    let globals = construct_globals(
      &Gomoku::new(4, 4, 4),
      crate::Options {
        search_depth: DEPTH,
        num_threads: THREADS,
        unit_depth: 5,
      },
    );

    println!("Solving...");
    let start = SystemTime::now();
    let thread_handles: Vec<_> = (0..THREADS)
      .map(|thread_idx| {
        let globals = globals.clone();
        thread::Builder::new()
          .name(format!("worker_{thread_idx}"))
          .spawn(move || {
            start_worker(WorkerData::new(thread_idx, globals));
          })
          .unwrap()
      })
      .collect();

    let mut any_bad = false;
    for thread in thread_handles.into_iter() {
      any_bad = thread.join().is_err() || any_bad;
    }
    let end = SystemTime::now();
    println!("Done: {:?}", end.duration_since(start).unwrap());

    assert!(!any_bad);

    // Compute the ground truth table.
    let mut table = find_best_move_serial(&Gomoku::new(4, 4, 4), DEPTH).2;

    for state in globals.resolved_states_table().table().iter() {
      // Terminal states should not be stored in the table.
      assert_eq!(state.key().finished(), GameResult::NotFinished);

      let expected_score = table
        .get(state.key())
        .map(|game_ref| game_ref.score())
        .unwrap_or_else(|| {
          do_find_best_move_serial(state.key(), DEPTH, &mut table);
          table.get(state.key()).unwrap().score()
        });

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
