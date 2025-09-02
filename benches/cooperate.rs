use std::{hint::black_box, time::Duration};

use cooperate::solve_with_hasher;
use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use onoro::Onoro;
use onoro_impl::{Onoro16, OnoroView};

pub struct PassThroughHasher {
  state: u64,
}

impl std::hash::Hasher for PassThroughHasher {
  fn write(&mut self, bytes: &[u8]) {
    debug_assert!(bytes.len() == 8 && self.state == 0);
    self.state = unsafe { *(bytes.as_ptr() as *const u64) };
  }

  fn finish(&self) -> u64 {
    self.state
  }
}

#[derive(Clone)]
pub struct BuildPassThroughHasher;

impl std::hash::BuildHasher for BuildPassThroughHasher {
  type Hasher = PassThroughHasher;
  fn build_hasher(&self) -> PassThroughHasher {
    PassThroughHasher { state: 0 }
  }
}

fn solve_default_start(c: &mut Criterion) {
  let mut group = c.benchmark_group("solve p1");
  group.throughput(Throughput::Elements(1));
  group.measurement_time(Duration::from_secs(20));

  let onoro = Onoro16::default_start();

  group.bench_function("solve 1 thread default start to depth 5", |b| {
    b.iter(|| {
      let options = cooperate::Options {
        num_threads: 1,
        search_depth: 7,
        unit_depth: 0,
      };
      let score = solve_with_hasher(
        &OnoroView::new(onoro.clone()),
        options,
        BuildPassThroughHasher,
      );
      black_box(score);
    })
  });

  group.finish();
}

fn solve_phase2(c: &mut Criterion) {
  let mut group = c.benchmark_group("solve p2");
  group.throughput(Throughput::Elements(1));
  group.measurement_time(Duration::from_secs(20));

  let onoro = Onoro16::from_board_string(
    ". . . W . .
      . B B B W .
       . W B B B W
        B W W W B .
         . W . . . .",
  )
  .unwrap();

  group.bench_function("solve 1 thread phase 2 to depth 4", |b| {
    b.iter(|| {
      let options = cooperate::Options {
        num_threads: 1,
        search_depth: 4,
        unit_depth: 0,
      };
      let score = solve_with_hasher(
        &OnoroView::new(onoro.clone()),
        options,
        BuildPassThroughHasher,
      );
      black_box(score);
    })
  });

  group.finish();
}

criterion_group!(cooperate_benches, solve_default_start, solve_phase2);
criterion_main!(cooperate_benches);
