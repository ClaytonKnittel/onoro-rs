use std::{hint::black_box, time::Duration};

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use onoro_impl::{
  benchmark_util::{generate_random_unfinished_states, OnoroViewBenchmark},
  OnoroView,
};
use rand::{rngs::StdRng, SeedableRng};

fn construct_views(c: &mut Criterion) {
  const N_GAMES: usize = 10_000;

  let mut group = c.benchmark_group("construct");
  group.throughput(Throughput::Elements(N_GAMES as u64));
  group.measurement_time(Duration::from_secs(20));

  let mut rng = StdRng::seed_from_u64(90383240);
  let states = generate_random_unfinished_states(N_GAMES, 18, &mut rng).unwrap();

  #[cfg(feature = "profiled")]
  let guard = pprof::ProfilerGuardBuilder::default()
    .frequency(1000)
    .blocklist(&["libc", "libgcc", "pthread", "vdso"])
    .build()
    .unwrap();

  group.bench_function("construct after 18 moves", |b| {
    b.iter(|| {
      for onoro in &states {
        let view = OnoroView::bench_find_canonical_view(onoro);
        black_box(view);
      }
    })
  });

  #[cfg(feature = "profiled")]
  if let Ok(report) = guard.report().build() {
    let file = std::fs::File::create("onoro_view_construct.svg").unwrap();
    report.flamegraph(file).unwrap();
  };

  group.finish();
}

criterion_group!(onoro_view_benches, construct_views);
criterion_main!(onoro_view_benches);
