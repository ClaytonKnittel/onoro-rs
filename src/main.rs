use std::{collections::HashSet, time::Instant};

use onoro::{Move, Onoro16, Onoro16View, OnoroView, Score, ScoreValue};
use rand::Rng;

type OnoroTable = HashSet<Onoro16View>;

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

#[derive(Debug, Default)]
struct Metrics {
  n_states: u64,
  n_misses: u64,
  n_hits: u64,
  n_leaves: u64,
}

fn find_best_move(
  onoro: &Onoro16,
  depth: u32,
  metrics: &mut Metrics,
) -> (Option<Score>, Option<Move>) {
  // Can't score games that are already over.
  debug_assert!(onoro.finished().is_none());

  metrics.n_states += 1;

  if depth == 0 {
    metrics.n_leaves += 1;
    return (Some(Score::tie(0)), None);
  }

  let mut best_score = None;
  let mut best_move = None;

  // First, check if any move ends the game.
  for m in onoro.each_move() {
    let mut g = onoro.clone();
    g.make_move(m);
    if g.finished().is_some() {
      metrics.n_leaves += 1;
      return (Some(Score::win(1)), Some(m));
    }
  }

  metrics.n_misses += 1;

  for m in onoro.each_move() {
    let mut g = onoro.clone();
    g.make_move(m);

    let (score, _) = find_best_move(&g, depth - 1, metrics);
    let score = match score {
      Some(score) => score.backstep(),
      // Consider winning by no legal moves as not winning until after the
      // other player's attempt at making a move, since all game states that
      // don't have 4 in a row of a pawn are considered a tie.
      None => Score::win(2),
    };

    match best_score.clone() {
      Some(best_score_val) => {
        if score.better(&best_score_val) {
          best_score = Some(score.clone());
          best_move = Some(m);
        }
      }
      None => {
        best_score = Some(score.clone());
        best_move = Some(m);
      }
    }

    // Stop the search early if there's already a winning move.
    if score.score_at_depth(depth) == ScoreValue::CurrentPlayerWins {
      break;
    }
  }

  (best_score, best_move)
}

fn find_best_move_table(
  onoro: &Onoro16,
  table: &mut OnoroTable,
  depth: u32,
  metrics: &mut Metrics,
) -> (Option<Score>, Option<Move>) {
  // Can't score games that are already over.
  debug_assert!(onoro.finished().is_none());
  debug_assert!(onoro.validate().map_err(|res| panic!("{}", res)).is_ok());

  if depth < 2 {
    return find_best_move(onoro, depth, metrics);
  }

  metrics.n_states += 1;

  if depth == 0 {
    metrics.n_leaves += 1;
    return (Some(Score::tie(0)), None);
  }

  let mut best_score = None;
  let mut best_move = None;

  // First, check if any move ends the game.
  for m in onoro.each_move() {
    let mut g = onoro.clone();
    g.make_move(m);
    if g.finished().is_some() {
      metrics.n_leaves += 1;
      return (Some(Score::win(1)), Some(m));
    }
  }

  metrics.n_misses += 1;

  for m in onoro.each_move() {
    let mut g = onoro.clone();
    g.make_move(m);

    let mut view = OnoroView::new(g);
    let score = table
      .get(&view)
      .map(|view| view.onoro().score())
      .and_then(|score| {
        if score.determined(depth - 1) {
          metrics.n_states += 1;
          metrics.n_hits += 1;
          Some(score)
        } else {
          None
        }
      })
      .unwrap_or_else(|| {
        let (score, _) = find_best_move_table(view.onoro(), table, depth - 1, metrics);
        let score = match score {
          Some(score) => score,
          // Consider winning by no legal moves as not winning until after the
          // other player's attempt at making a move, since all game states that
          // don't have 4 in a row of a pawn are considered a tie.
          None => Score::win(1),
        };

        // Update the cached score in case it changed.
        let score = if let Some(cached_view) = table.take(&view) {
          score.merge(&cached_view.onoro().score())
        } else {
          score
        };

        score
      });

    view.mut_onoro().set_score(score.clone());
    table.replace(view);

    let score = score.backstep();
    match best_score.clone() {
      Some(best_score_val) => {
        if score.better(&best_score_val) {
          best_score = Some(score.clone());
          best_move = Some(m);
        }
      }
      None => {
        best_score = Some(score.clone());
        best_move = Some(m);
      }
    }

    // Stop the search early if there's already a winning move.
    if score.score_at_depth(depth) == ScoreValue::CurrentPlayerWins {
      break;
    }
  }

  (best_score, best_move)
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

  let depth = 13;

  for _ in 0..1 {
    let mut metrics = Metrics::default();
    let mut table = OnoroTable::new();

    let start = Instant::now();
    let (score, m) = find_best_move_table(&game, &mut table, depth, &mut metrics);
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

    // println!("Checking table: {} entries:", table.len());
    // for view in table.iter() {
    //   let view_score = view.onoro().score();
    //   let (score, _) = find_best_move(
    //     view.onoro(),
    //     view_score.determined_depth(),
    //     &mut Metrics::default(),
    //   );
    //   let score = score.unwrap();

    //   assert_eq!(
    //     view_score,
    //     score,
    //     "Expected equal scores at {}, found {} in table, computed {} to depth {}",
    //     view.onoro(),
    //     view_score,
    //     score,
    //     view_score.determined_depth()
    //   );
    // }

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
