use std::{hint::black_box, time::Duration};

use cooperate::{passthrough_hasher::BuildPassThroughHasher, solvers::ttable_solver::TTSolver};
use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use onoro::{abstract_game::Solver, Onoro};
use onoro_impl::{Onoro16, OnoroView};

fn solve_default(c: &mut Criterion) {
  const N_GAMES: usize = 1;

  let mut group = c.benchmark_group("solve");
  group.throughput(Throughput::Elements(N_GAMES as u64));
  group.measurement_time(Duration::from_secs(20));

  let onoro = Onoro16::default_start();
  group.bench_function("solve default start to depth 7", |b| {
    b.iter(|| {
      let mut solver = TTSolver::with_hasher(BuildPassThroughHasher);
      black_box(solver.best_move(&OnoroView::new(onoro.clone()), 7));
    })
  });

  let onoro = Onoro16::hex_start();
  group.bench_function("solve hex start to depth 7", |b| {
    b.iter(|| {
      let mut solver = TTSolver::with_hasher(BuildPassThroughHasher);
      black_box(solver.best_move(&OnoroView::new(onoro.clone()), 7));
    })
  });

  group.finish();
}

criterion_group!(solve_benches, solve_default);
criterion_main!(solve_benches);
