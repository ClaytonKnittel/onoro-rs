use std::time::Instant;

use onoro::{Move, Onoro16, Score, ScoreValue};
use rand::Rng;

fn validate_moves(onoro: &Onoro16) {
  let mut move_iter = onoro.each_p1_move();
  for m in onoro.each_p1_move() {
    assert_eq!(move_iter.next().unwrap(), m);
  }
  assert!(move_iter.next().is_none());
}

fn first_move(onoro: &Onoro16) -> Move {
  onoro.each_p1_move().next().unwrap()
}

fn nth_move(onoro: &Onoro16, idx: usize) -> Move {
  onoro.each_p2_move().nth(idx).unwrap()
}

fn random_move(onoro: &Onoro16) -> Move {
  let moves = onoro.each_p1_move().collect::<Vec<_>>();

  let mut rng = rand::thread_rng();
  let n = rng.gen_range(0..moves.len());
  moves[n].clone()
}

fn to_phase2(onoro: &mut Onoro16) {
  while onoro.in_phase1() {
    for m in onoro.each_p1_move() {
      let mut o2 = onoro.clone();
      o2.make_move(&m);
      if o2.finished().is_none() {
        onoro.make_move(&m);
        break;
      }
    }
  }
}

fn explore(onoro: &Onoro16, depth: u32) -> u64 {
  let mut total_states = 1;

  if onoro.finished().is_some() || !onoro.in_phase1() || depth == 0 {
    return total_states;
  }

  for m in onoro.each_p1_move() {
    let mut onoro2 = onoro.clone();
    onoro2.make_move(&m);
    total_states += explore(&onoro2, depth - 1);
  }

  total_states
}

fn explore_p2(onoro: &Onoro16, depth: u32) -> u64 {
  let mut total_states = 1;

  if onoro.finished().is_some() || depth == 0 {
    return total_states;
  }

  for m in onoro.each_p2_move() {
    let mut onoro2 = onoro.clone();
    onoro2.make_move(&m);
    total_states += explore_p2(&onoro2, depth - 1);
  }

  total_states
}

fn find_best_move(onoro: &Onoro16, depth: u32) -> (Option<Score>, Option<Move>) {
  // Can't score games that are already over.
  debug_assert!(onoro.finished().is_none());
  if depth == 0 {
    return (Some(Score::tie(0)), None);
  }

  let mut best_score = None;
  let mut best_move = None;

  // First, check if any move ends the game.
  for m in onoro.each_move() {
    let mut g = onoro.clone();
    g.make_move(&m);
    if g.finished().is_some() {
      return (Some(Score::win(1)), Some(m));
    }
  }

  for m in onoro.each_move() {
    let mut g = onoro.clone();
    g.make_move(&m);

    let (score, _) = find_best_move(&g, depth - 1);
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

fn main() {
  let mut game = Onoro16::default_start();

  println!("size of game state: {}", std::mem::size_of::<Onoro16>());

  println!("{}", game);

  // let guard = pprof::ProfilerGuardBuilder::default()
  //   .frequency(1000)
  //   .blocklist(&["libc", "libgcc", "pthread", "vdso"])
  //   .build()
  //   .unwrap();

  let (score, m) = find_best_move(&game, 1);
  println!("{}, {}", m.unwrap(), score.unwrap());

  // to_phase2(&mut game);
  // println!("{game}");

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
