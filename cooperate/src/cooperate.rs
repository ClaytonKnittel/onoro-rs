use std::{
  collections::{hash_map::RandomState, HashSet},
  fmt::Display,
  hash::{BuildHasher, Hash},
  sync::Arc,
  thread,
};

use abstract_game::{Game, GameResult, Score, ScoreValue};
use rand::prelude::*;

use crate::{
  global_data::GlobalData,
  null_lock::NullLock,
  search_worker::{start_worker, WorkerData},
  stack::Stack,
  table::Table,
};

#[derive(Clone)]
pub struct Options {
  /// The number of worker threads to use in the thread pool.
  pub num_threads: u32,
  /// The depth to search the game to.
  pub search_depth: u32,
  /// The depth to expand to for generating work units.
  pub unit_depth: u32,
}

fn generate_frontier<G>(initial_state: G, options: &Options) -> Vec<*mut Stack<G>>
where
  G: Game + Hash + PartialEq + Eq + Display + 'static,
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
      Box::into_raw(Box::new(Stack::make_root(
        state,
        options.search_depth - options.unit_depth,
      )))
    })
    .collect()
}

fn construct_globals<G, H>(game: &G, options: Options, hasher: H) -> Arc<GlobalData<G, H>>
where
  G: Game + Display + Hash + PartialEq + Eq + 'static,
  G::Move: Display,
  H: BuildHasher + Clone,
{
  let globals = Arc::new(GlobalData::with_hasher(
    options.search_depth,
    options.num_threads,
    hasher,
  ));

  let mut rng = thread_rng();
  for stack in generate_frontier(game.clone(), &options).into_iter() {
    let rand_idx = rng.gen_range(0..options.num_threads);
    globals
      .queue(rand_idx)
      .push(unsafe { NullLock::new(stack) });
  }

  globals
}

pub fn solve<G>(game: &G, options: Options) -> Score
where
  G: Game + Display + Send + Sync + Hash + PartialEq + Eq + 'static,
  G::Move: Display,
{
  solve_with_hasher(game, options, RandomState::new())
}

fn playout<G, H>(game: &G, tbl: &Table<G, H>, depth: u32)
where
  G: Game + Display + Send + Sync + Hash + PartialEq + Eq + 'static,
  G::Move: Display,
  H: BuildHasher + Clone + Send + Sync + 'static,
{
  println!("{}", game);
  if depth == 0 || game.finished() != GameResult::NotFinished {
    return;
  }

  for mv in game.each_move() {
    let next_state = game.with_move(mv);
    if depth == 1 && next_state.finished() != GameResult::NotFinished {
      playout(&next_state, tbl, depth - 1);
    }
    if depth != 1 {
      if let Some(score) = tbl.get(&next_state) {
        if score.determined(depth) && score.score_at_depth(depth) != ScoreValue::Tie {
          println!("{score}");
          playout(&next_state, tbl, depth - 1);
        }
      }
    }
  }
}

pub fn solve_with_hasher<G, H>(game: &G, options: Options, hasher: H) -> Score
where
  G: Game + Display + Send + Sync + Hash + PartialEq + Eq + 'static,
  G::Move: Display,
  H: BuildHasher + Clone + Send + Sync + 'static,
{
  let globals = construct_globals(game, options.clone(), hasher);
  let thread_handles: Vec<_> = (0..options.num_threads)
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

  // for mv in game.each_move() {
  //   let next_state = game.with_move(mv);
  // print!("{mv}: ");
  // if let Some(score) = globals.resolved_states_table().get(&next_state) {
  //   println!("{score}");
  //   if score.score_at_depth(score.determined_depth()) != ScoreValue::Tie {
  //     playout(&next_state, globals.resolved_states_table(), score.determined_depth());
  //   }
  // } else {
  //   println!();
  // }
  // }

  globals.resolved_states_table().get(game).unwrap()
}

#[cfg(test)]
mod tests {
  use std::{collections::hash_map::RandomState, thread, time::SystemTime};

  use abstract_game::{Game, GameResult};

  use crate::{
    cooperate::construct_globals,
    search_worker::{start_worker, WorkerData},
    test::{
      gomoku::Gomoku,
      nim::Nim,
      serial_search::{find_best_move_serial, find_best_move_serial_table},
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
      RandomState::new(),
    );

    start_worker(WorkerData::new(0, globals.clone()));

    for sticks in 1..=STICKS {
      let cached_score = globals.resolved_states_table().get(&Nim::new(sticks));
      assert!(cached_score.is_some());
      assert_eq!(cached_score.unwrap(), Nim::new(sticks).expected_score());
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
      RandomState::new(),
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
      let cached_score = globals.resolved_states_table().get(&Nim::new(sticks));
      assert!(cached_score.is_some());
      assert_eq!(cached_score.unwrap(), Nim::new(sticks).expected_score());
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
      RandomState::new(),
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
      RandomState::new(),
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
  fn test_gomoku_4x4_p2() {
    const DEPTH: u32 = 16;
    const THREADS: u32 = 2;

    let globals = construct_globals(
      &Gomoku::new(4, 4, 4),
      crate::Options {
        search_depth: DEPTH,
        num_threads: THREADS,
        unit_depth: 3,
      },
      RandomState::new(),
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
    let table = find_best_move_serial(&Gomoku::new(4, 4, 4), DEPTH).2;

    for state in globals.resolved_states_table().table().iter() {
      // Terminal states should not be stored in the table.
      assert_eq!(state.key().finished(), GameResult::NotFinished);

      let expected_score = table.get(state.key()).unwrap_or_else(|| {
        find_best_move_serial_table(state.key(), DEPTH, &table);
        table.get(state.key()).unwrap()
      });

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
  fn test_gomoku_4x4_p8() {
    const DEPTH: u32 = 16;
    const THREADS: u32 = 8;

    let globals = construct_globals(
      &Gomoku::new(4, 4, 4),
      crate::Options {
        search_depth: DEPTH,
        num_threads: THREADS,
        unit_depth: 3,
      },
      RandomState::new(),
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
    let table = find_best_move_serial(&Gomoku::new(4, 4, 4), DEPTH).2;

    for state in globals.resolved_states_table().table().iter() {
      // Terminal states should not be stored in the table.
      assert_eq!(state.key().finished(), GameResult::NotFinished);

      let expected_score = table.get(state.key()).unwrap_or_else(|| {
        find_best_move_serial_table(state.key(), DEPTH, &table);
        table.get(state.key()).unwrap()
      });

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
  fn test_gomoku_4x4_p32() {
    const DEPTH: u32 = 16;
    const THREADS: u32 = 32;

    let globals = construct_globals(
      &Gomoku::new(4, 4, 4),
      crate::Options {
        search_depth: DEPTH,
        num_threads: THREADS,
        unit_depth: 5,
      },
      RandomState::new(),
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
    let table = find_best_move_serial(&Gomoku::new(4, 4, 4), DEPTH).2;

    for state in globals.resolved_states_table().table().iter() {
      // Terminal states should not be stored in the table.
      assert_eq!(state.key().finished(), GameResult::NotFinished);

      let expected_score = table.get(state.key()).unwrap_or_else(|| {
        find_best_move_serial_table(state.key(), DEPTH, &table);
        table.get(state.key()).unwrap()
      });

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
  fn test_gomoku_5x5_p32() {
    const DEPTH: u32 = 9;
    const THREADS: u32 = 32;

    let globals = construct_globals(
      &Gomoku::new(5, 5, 4),
      crate::Options {
        search_depth: DEPTH,
        num_threads: THREADS,
        unit_depth: 5,
      },
      RandomState::new(),
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
    let table = find_best_move_serial(&Gomoku::new(5, 5, 4), DEPTH).2;

    for state in globals.resolved_states_table().table().iter() {
      // Terminal states should not be stored in the table.
      assert_eq!(state.key().finished(), GameResult::NotFinished);

      let expected_score = table.get(state.key()).unwrap_or_else(|| {
        find_best_move_serial_table(state.key(), DEPTH, &table);
        table.get(state.key()).unwrap()
      });

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
}
