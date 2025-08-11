use std::{hint::black_box, time::Duration};

use criterion::{
  criterion_group, criterion_main, measurement::Measurement, BenchmarkGroup, Criterion, Throughput,
};
use itertools::Itertools;
use onoro::Onoro;
use onoro_impl::{benchmark_util::CheckWinBenchmark, Onoro16};
use rand::{rngs::StdRng, Rng, SeedableRng};

fn random_playout<R: Rng>(mut onoro: Onoro16, num_moves: usize, rng: &mut R) -> Onoro16 {
  for _ in 0..num_moves {
    let moves = onoro.each_move().collect_vec();
    let m = moves[rng.gen_range(0..moves.len())];
    onoro.make_move(m);
  }
  onoro
}

#[inline(never)]
fn generate_random_states<R: Rng>(count: usize, num_moves: usize, rng: &mut R) -> Vec<Onoro16> {
  (0..count)
    .map(|_| random_playout(Onoro16::default_start(), num_moves, rng))
    .collect()
}

fn benchmark_each_move<M: Measurement, R: Rng>(
  group: &mut BenchmarkGroup<M>,
  id: &str,
  num_games: usize,
  num_moves: usize,
  rng: &mut R,
) {
  let states = generate_random_states(num_games, num_moves, rng);
  group.bench_function(id, |b| {
    b.iter(|| {
      for onoro in &states {
        for m in onoro.each_move() {
          black_box(m);
        }
      }
    })
  });
}

fn find_moves_p1(c: &mut Criterion) {
  const N_GAMES: usize = 10_000;

  let mut group = c.benchmark_group("find moves phase 1");
  group.throughput(Throughput::Elements(N_GAMES as u64));
  group.measurement_time(Duration::from_secs(20));

  let mut rng = StdRng::seed_from_u64(392420);

  benchmark_each_move(
    &mut group,
    "find moves phase 1 after 4 moves",
    N_GAMES,
    4,
    &mut rng,
  );

  benchmark_each_move(
    &mut group,
    "find moves phase 1 after 8 moves",
    N_GAMES,
    8,
    &mut rng,
  );

  benchmark_each_move(
    &mut group,
    "find moves phase 1 after 12 moves",
    N_GAMES,
    12,
    &mut rng,
  );

  group.finish();
}

fn find_moves_p2(c: &mut Criterion) {
  const N_GAMES: usize = 5_000;

  let mut group = c.benchmark_group("find moves phase 2");
  group.throughput(Throughput::Elements(N_GAMES as u64));
  group.measurement_time(Duration::from_secs(20));

  let mut rng = StdRng::seed_from_u64(392421);

  benchmark_each_move(
    &mut group,
    "find moves phase 2 after 13 moves",
    N_GAMES,
    13,
    &mut rng,
  );

  benchmark_each_move(
    &mut group,
    "find moves phase 2 after 15 moves",
    N_GAMES,
    15,
    &mut rng,
  );

  benchmark_each_move(
    &mut group,
    "find moves phase 2 after 17 moves",
    N_GAMES,
    17,
    &mut rng,
  );

  group.finish();
}

fn check_win(c: &mut Criterion) {
  const N_GAMES: usize = 10_000;

  let mut group = c.benchmark_group("check win");
  group.throughput(Throughput::Elements(N_GAMES as u64));
  group.measurement_time(Duration::from_secs(20));

  let mut rng = StdRng::seed_from_u64(4324908);

  let mut states = generate_random_states(N_GAMES, 18, &mut rng);
  // Make an extra move for half the games. Otherwise, it would be the same
  // color's turn in every game.
  for state in &mut states {
    if rng.gen_bool(0.5) {
      *state = random_playout(state.clone(), 1, &mut rng);
    }
  }

  group.bench_function("check win", |b| {
    b.iter(|| {
      for onoro in &states {
        for pawn in onoro.pawns() {
          black_box(onoro.bench_check_win(pawn.pos.into()));
        }
      }
    })
  });
  group.finish();
}

criterion_group!(onoro_benches, find_moves_p1, find_moves_p2, check_win);
criterion_main!(onoro_benches);
