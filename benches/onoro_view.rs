use std::{hint::black_box, time::Duration};

use algebra::{finite::Finite, ordinal::Ordinal};
use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use onoro::groups::D6;
use onoro_impl::{
  benchmark_util::{generate_random_unfinished_states, BenchCanonicalView},
  OnoroImpl, OnoroView,
};
use rand::{rngs::StdRng, Rng, SeedableRng};

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
        let view = BenchCanonicalView::find_canonical_view(onoro);
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

fn generate_game_pairs<const N: usize, R: Rng>(
  games: Vec<OnoroImpl<N>>,
  rng: &mut R,
) -> Vec<(OnoroView<N>, OnoroView<N>)> {
  games
    .into_iter()
    .map(|game| {
      let ord = rng.gen_range(0..D6::SIZE);
      let op = D6::from_ord(ord);
      let other_game = game.rotated_d6_c(op);
      (OnoroView::new(game), OnoroView::new(other_game))
    })
    .collect()
}

fn cmp_views(c: &mut Criterion) {
  const N_GAMES: usize = 10_000;

  let mut group = c.benchmark_group("cmp");
  group.throughput(Throughput::Elements(N_GAMES as u64));
  group.measurement_time(Duration::from_secs(20));

  let mut rng = StdRng::seed_from_u64(4238903259);
  let states = generate_random_unfinished_states(N_GAMES, 18, &mut rng).unwrap();
  let states = generate_game_pairs(states, &mut rng);

  #[cfg(feature = "profiled")]
  let guard = pprof::ProfilerGuardBuilder::default()
    .frequency(1000)
    .blocklist(&["libc", "libgcc", "pthread", "vdso"])
    .build()
    .unwrap();

  group.bench_function("compare views after 18 moves", |b| {
    b.iter(|| {
      for (view1, view2) in &states {
        black_box(view1 == view2);
      }
    })
  });

  #[cfg(feature = "profiled")]
  if let Ok(report) = guard.report().build() {
    let file = std::fs::File::create("onoro_view_cmp.svg").unwrap();
    report.flamegraph(file).unwrap();
  };

  group.finish();
}

criterion_group!(onoro_view_benches, construct_views, cmp_views);
criterion_main!(onoro_view_benches);
