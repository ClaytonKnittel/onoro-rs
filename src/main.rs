use std::time::Instant;

use onoro::{Move, Onoro, Onoro16, OnoroView};
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
      o2.make_move(m.clone());
      if o2.finished().is_none() {
        onoro.make_move(m);
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
    onoro2.make_move(m);
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
    onoro2.make_move(m);
    total_states += explore_p2(&onoro2, depth - 1);
  }

  total_states
}

fn main() {
  for game in vec![
    Onoro16::default_start(),
    Onoro16::default_start2(),
    Onoro16::default_start3(),
  ]
  .into_iter()
  {
    let view = OnoroView::new(game);
    println!("{}", view);
  }

  // let mut game = Onoro16::default_start();

  // println!("size of game state: {}", std::mem::size_of::<Onoro16>());

  // println!("{}", game);

  // let guard = pprof::ProfilerGuardBuilder::default()
  //   .frequency(1000)
  //   .blocklist(&["libc", "libgcc", "pthread", "vdso"])
  //   .build()
  //   .unwrap();

  // to_phase2(&mut game);

  // let start = Instant::now();
  // let num_states = explore_p2(&game, 5);
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
