use std::{sync::Arc, time::Instant};

use cooperate::Metrics;
use onoro::{Move, Onoro16, OnoroView};
use rand::Rng;

use crate::{
  onoro_table::OnoroTable,
  par_search_opts::ParSearchOptions,
  search::{find_best_move, find_best_move_par_old, find_best_move_table},
};

mod onoro_table;
mod par_search;
mod par_search_opts;
mod search;

#[allow(dead_code)]
fn validate_moves(onoro: &Onoro16) {
  let mut move_iter = onoro.each_p1_move();
  for m in onoro.each_p1_move() {
    assert_eq!(move_iter.next().unwrap(), m);
  }
  assert!(move_iter.next().is_none());
}

#[allow(dead_code)]
fn first_move(onoro: &Onoro16) -> Move {
  onoro.each_p1_move().next().unwrap()
}

#[allow(dead_code)]
fn nth_move(onoro: &Onoro16, idx: usize) -> Move {
  onoro.each_p2_move().nth(idx).unwrap()
}

#[allow(dead_code)]
fn random_move(onoro: &Onoro16) -> Move {
  let moves = onoro.each_p1_move().collect::<Vec<_>>();

  let mut rng = rand::thread_rng();
  let n = rng.gen_range(0..moves.len());
  moves[n]
}

#[allow(dead_code)]
fn to_phase2(onoro: &mut Onoro16) {
  while onoro.in_phase1() {
    for m in onoro.each_p1_move() {
      let mut o2 = onoro.clone();
      o2.make_move(m);
      if o2.finished().is_none() {
        onoro.make_move(m);
        break;
      }
    }
  }
}

#[allow(dead_code)]
fn explore(onoro: &Onoro16, depth: u32) -> u64 {
  let mut total_states = 1;

  if onoro.finished().is_some() || !onoro.in_phase1() || depth == 0 {
    return total_states;
  }

  for m in onoro.each_p1_move() {
    let mut onoro2 = onoro.clone();
    onoro2.make_move(m);
    total_states += explore(&onoro2, depth - 1);
  }

  total_states
}

#[allow(dead_code)]
fn explore_p2(onoro: &Onoro16, depth: u32) -> u64 {
  let mut total_states = 1;

  if onoro.finished().is_some() || depth == 0 {
    return total_states;
  }

  for m in onoro.each_p2_move() {
    let mut onoro2 = onoro.clone();
    onoro2.make_move(m);
    total_states += explore_p2(&onoro2, depth - 1);
  }

  total_states
}

fn main() {
  let mut game = Onoro16::default_start();

  println!("size of game state: {}", std::mem::size_of::<Onoro16>());
  println!(
    "size of game view: {}",
    std::mem::size_of_val(&OnoroView::new(Onoro16::default_start()))
  );

  println!("{}", game);

  // let guard = pprof::ProfilerGuardBuilder::default()
  //   .frequency(1000)
  //   .blocklist(&["libc", "libgcc", "pthread", "vdso"])
  //   .build()
  //   .unwrap();

  let depth = 10;

  for _ in 0..1 {
    let mut metrics = Metrics::default();
    let table = Arc::new(OnoroTable::new());

    let start = Instant::now();
    let (score, m) = find_best_move_table(&game, table.clone(), depth, &mut metrics);
    let end = Instant::now();
    let m = m.unwrap();
    println!("{}, {}", m, score.unwrap());
    println!("{}", game.print_with_move(m));
    println!(
      "{} states explored, {} hits, {} misses, {} leaves",
      metrics.n_states, metrics.n_hits, metrics.n_misses, metrics.n_leaves
    );
    println!(
      "{:?}, {} states/sec",
      end - start,
      metrics.n_states as f64 / (end - start).as_secs_f64()
    );

    let mut metrics = Metrics::default();
    let table = Arc::new(OnoroTable::new());
    let start = Instant::now();
    let (score, m) = find_best_move_par_old(
      &game,
      table.clone(),
      depth,
      ParSearchOptions::default().with_unit_depth(5),
      &mut metrics,
    );
    let end = Instant::now();
    let m = m.unwrap();
    println!("{}, {}", m, score.unwrap());
    println!("{}", game.print_with_move(m));
    println!(
      "{} states explored, {} hits, {} misses, {} leaves",
      metrics.n_states, metrics.n_hits, metrics.n_misses, metrics.n_leaves
    );
    println!(
      "{:?}, {} states/sec",
      end - start,
      metrics.n_states as f64 / (end - start).as_secs_f64()
    );

    println!("Checking table: {} entries:", table.len());
    for view in table.table().iter().step_by(17) {
      let view_score = view.onoro().score();
      let (score, _) = find_best_move(
        view.onoro(),
        view_score.determined_depth(),
        &mut Metrics::default(),
      );
      let score = score.unwrap();

      if !view_score.compatible(&score) {
        println!(
          "Expected compatible scores at {}, found {} in table, computed {} to depth {}",
          view.onoro(),
          view_score,
          score,
          view_score.determined_depth()
        );
      }
      // assert_eq!(
      //   view_score,
      //   score,
      //   "Expected equal scores at {}, found {} in table, computed {} to depth {}",
      //   view.onoro(),
      //   view_score,
      //   score,
      //   view_score.determined_depth()
      // );
    }

    game.make_move(m);
  }

  // to_phase2(&mut game);
  // println!("{game}");

  // println!("{}", game.pawns_mathematica_list());

  // let start = Instant::now();
  // let num_states = explore_p2(&game, 2);
  // let end = Instant::now();

  // println!("Explored {} game states in {:?}", num_states, end - start);
  // println!(
  //   "{} states/sec",
  //   num_states as f64 / (end - start).as_secs_f64()
  // );

  // if let Ok(report) = guard.report().build() {
  //   let file = std::fs::File::create("flamegraph.svg").unwrap();
  //   report.flamegraph(file).unwrap();
  // };
}
