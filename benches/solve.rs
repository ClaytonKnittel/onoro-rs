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

  group.bench_function("solve default start", |b| {
    b.iter(|| {
      let onoro = Onoro16::default_start();
      let mut solver = TTSolver::with_hasher(BuildPassThroughHasher);
      for state in solver.playout(&OnoroView::new(onoro), 4) {
        black_box(state);
      }
    })
  });

  group.finish();
}

criterion_group!(solve_benches, solve_default);
criterion_main!(solve_benches);
