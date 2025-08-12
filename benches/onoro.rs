use std::{hint::black_box, time::Duration};

use criterion::{
  criterion_group, criterion_main, measurement::Measurement, BenchmarkGroup, Criterion, Throughput,
};
use itertools::Itertools;
use onoro::{
  error::{OnoroError, OnoroResult},
  Onoro, OnoroPawn,
};
use onoro_impl::{benchmark_util::CheckWinBenchmark, Move, Onoro16};
use rand::{rngs::StdRng, Rng, SeedableRng};

fn make_random_move<R: Rng>(onoro: &mut Onoro16, rng: &mut R) -> Move {
  let mut moves = onoro.each_move().collect_vec();
  moves.sort_by(|&m1, &m2| match (m1, m2) {
    (Move::Phase1Move { to: to1 }, Move::Phase1Move { to: to2 }) => {
      (to1.x(), to1.y()).cmp(&(to2.x(), to2.y()))
    }

    (
      Move::Phase2Move {
        to: to1,
        from_idx: from1,
      },
      Move::Phase2Move {
        to: to2,
        from_idx: from2,
      },
    ) => (to1.x(), to1.y())
      .cmp(&(to2.x(), to2.y()))
      .then(from1.cmp(&from2)),

    // All moves should be in the same phase.
    _ => unreachable!(),
  });
  let m = moves[rng.gen_range(0..moves.len())];
  onoro.make_move(m);
  m
}

/// Plays a random number of moves in the game, returning the number of moves
/// played until the game finished. If the game did not finish, returns
/// `num_moves + 1`.
fn random_playout<R: Rng>(onoro: &mut Onoro16, num_moves: usize, rng: &mut R) -> usize {
  for i in 1..=num_moves {
    make_random_move(onoro, rng);
    if onoro.finished().is_some() {
      return i;
    }
  }

  num_moves + 1
}

#[inline(never)]
fn generate_random_unfinished_states<R: Rng>(
  count: usize,
  num_moves: usize,
  rng: &mut R,
) -> OnoroResult<Vec<Onoro16>> {
  let mut states = Vec::with_capacity(count);

  let attempts = 100 * count;
  for _ in 0..attempts {
    let mut onoro = Onoro16::default_start();
    if random_playout(&mut onoro, num_moves, rng) > num_moves {
      states.push(onoro);
    }
    if states.len() == count {
      return Ok(states);
    }
  }

  Err(
    OnoroError::new(format!(
      "Failed to generate {count} random states with {num_moves} moves after {attempts} attempts"
    ))
    .into(),
  )
}

#[inline(never)]
fn generate_random_walks<R: Rng>(
  initial_state: &Onoro16,
  count: usize,
  rng: &mut R,
) -> OnoroResult<Vec<Vec<Move>>> {
  const MAX_MOVES: usize = 1000;
  debug_assert!(initial_state.finished().is_none());

  (0..count)
    .map(|_| {
      let mut onoro = initial_state.clone();
      let mut moves = Vec::new();
      for _ in 0..MAX_MOVES {
        if onoro.finished().is_some() {
          return Ok(moves);
        }
        moves.push(make_random_move(&mut onoro, rng));
      }

      Err(
        OnoroError::new(format!(
          "Exceeded maximum number of moves {MAX_MOVES} without finishing the game."
        ))
        .into(),
      )
    })
    .collect()
}

fn benchmark_each_move<M: Measurement, R: Rng>(
  group: &mut BenchmarkGroup<M>,
  id: &str,
  num_games: usize,
  num_moves: usize,
  rng: &mut R,
) -> OnoroResult {
  let states = generate_random_unfinished_states(num_games, num_moves, rng)?;
  group.bench_function(id, |b| {
    b.iter(|| {
      for onoro in &states {
        for m in onoro.each_move() {
          black_box(m);
        }
      }
    })
  });

  Ok(())
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
  )
  .unwrap();

  benchmark_each_move(
    &mut group,
    "find moves phase 1 after 8 moves",
    N_GAMES,
    8,
    &mut rng,
  )
  .unwrap();

  benchmark_each_move(
    &mut group,
    "find moves phase 1 after 12 moves",
    N_GAMES,
    12,
    &mut rng,
  )
  .unwrap();

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
  )
  .unwrap();

  benchmark_each_move(
    &mut group,
    "find moves phase 2 after 15 moves",
    N_GAMES,
    15,
    &mut rng,
  )
  .unwrap();

  benchmark_each_move(
    &mut group,
    "find moves phase 2 after 17 moves",
    N_GAMES,
    17,
    &mut rng,
  )
  .unwrap();

  group.finish();
}

fn make_move(c: &mut Criterion) {
  const N_GAMES: usize = 10_000;

  let mut rng = StdRng::seed_from_u64(4328975198);

  let initial_state = Onoro16::default_start();
  let states = generate_random_walks(&initial_state, N_GAMES, &mut rng).unwrap();

  let num_elements = states.iter().map(|moves| moves.len()).sum::<usize>();

  let mut group = c.benchmark_group("make move");
  group.throughput(Throughput::Elements(num_elements as u64));
  group.measurement_time(Duration::from_secs(20));

  group.bench_function("make move", |b| {
    b.iter(|| {
      for moves in &states {
        let mut onoro = initial_state.clone();
        for &m in moves {
          onoro.make_move(m);
        }
        black_box(onoro);
      }
    })
  });
  group.finish();
}

fn check_win(c: &mut Criterion) {
  const N_GAMES: usize = 10_000;

  let mut group = c.benchmark_group("check win");
  group.throughput(Throughput::Elements(
    (2 * Onoro16::pawns_per_player() * N_GAMES) as u64,
  ));
  group.measurement_time(Duration::from_secs(20));

  let mut rng = StdRng::seed_from_u64(4324908);

  let mut states = generate_random_unfinished_states(N_GAMES, 18, &mut rng).unwrap();
  // Make an extra move for half the games. Otherwise, it would be the same
  // color's turn in every game.
  for state in &mut states {
    if rng.gen_bool(0.5) {
      random_playout(state, 1, &mut rng);
    }
  }

  group.bench_function("check win", |b| {
    b.iter(|| {
      for onoro in &states {
        for pawn in onoro.pawns() {
          black_box(onoro.bench_check_win(pawn.pos().into()));
        }
      }
    })
  });
  group.finish();
}

fn get_tile(c: &mut Criterion) {
  const N_GAMES: usize = 10_000;

  let mut group = c.benchmark_group("get tile");
  group.throughput(Throughput::Elements(
    (2 * Onoro16::pawns_per_player() * N_GAMES) as u64,
  ));
  group.measurement_time(Duration::from_secs(20));

  let mut rng = StdRng::seed_from_u64(901482019);

  let states = generate_random_unfinished_states(N_GAMES, 18, &mut rng).unwrap();

  group.bench_function("get tile", |b| {
    b.iter(|| {
      for onoro in &states {
        for pawn in onoro.pawns() {
          black_box(onoro.get_tile(pawn.pos()));
        }
      }
    })
  });
  group.finish();
}

criterion_group!(
  onoro_benches,
  find_moves_p1,
  find_moves_p2,
  make_move,
  check_win,
  get_tile
);
criterion_main!(onoro_benches);
