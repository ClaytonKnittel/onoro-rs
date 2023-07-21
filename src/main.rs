use std::time::Instant;

use onoro::{Move, Onoro16};
use rand::Rng;

fn validate_moves(onoro: &Onoro16) {
  let mut move_iter = onoro.each_p1_move();
  for m in onoro.each_p1_move() {
    assert_eq!(move_iter.next().unwrap(), m);
  }
  assert!(move_iter.next().is_none());
}

fn random_move(onoro: &Onoro16) -> Move {
  let moves = onoro.each_p1_move().collect::<Vec<_>>();

  let mut rng = rand::thread_rng();
  let n = rng.gen_range(0..moves.len());
  moves[n].clone()
}

fn to_phase2(onoro: &mut Onoro16) {
  while onoro.in_phase1() {
    println!("{onoro}");
    let m = random_move(onoro);
    onoro.make_move(m);
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

fn main() {
  let mut game = Onoro16::default_start();

  // println!("size of game state: {}", std::mem::size_of::<Onoro16>());

  // println!("{}", game);

  // let guard = pprof::ProfilerGuardBuilder::default()
  //   .frequency(1000)
  //   .blocklist(&["libc", "libgcc", "pthread", "vdso"])
  //   .build()
  //   .unwrap();

  to_phase2(&mut game);

  for m in game.each_p2_move() {
    println!("{m}");
  }

  // let start = Instant::now();
  // let num_states = explore(&game, 10);
  // let end = Instant::now();

  // println!("Explored {} game states in {:?}", num_states, end - start);
  // println!(
  //   "{} states/sec",
  //   num_states as f64 / (end - start).as_secs_f64()
  // );

  // for _ in 0..1000000 {
  //   let mut g = game.clone();
  //   for _ in 0..13 {
  //     let m = random_move(&g);
  //     g.make_move(m);
  //     // if g.in_phase1() {
  //     //   validate_moves(&g);
  //     // }
  //     // g.validate().unwrap();
  //   }
  //   // println!("{g}");
  // }

  // if let Ok(report) = guard.report().build() {
  //   let file = std::fs::File::create("flamegraph.svg").unwrap();
  //   report.flamegraph(file).unwrap();
  // };
}
