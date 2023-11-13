use std::time::SystemTime;

use cooperate::solve_with_hasher;
use onoro::{Onoro16, OnoroView};

use crate::passthrough_hasher::BuildPassThroughHasher;

mod passthrough_hasher;

fn main() {
  let game = Onoro16::default_start();

  println!("size of game state: {}", std::mem::size_of::<Onoro16>());
  println!(
    "size of game view: {}",
    std::mem::size_of_val(&OnoroView::new(Onoro16::default_start()))
  );

  println!("{}", game);

  let guard = pprof::ProfilerGuardBuilder::default()
    .frequency(1000)
    .blocklist(&["libc", "libgcc", "pthread", "vdso"])
    .build()
    .unwrap();

  let start = SystemTime::now();
  let options = cooperate::Options {
    num_threads: 16,
    search_depth: 15,
    unit_depth: 8,
  };
  let score = solve_with_hasher(
    &OnoroView::new(Onoro16::default_start()),
    options,
    BuildPassThroughHasher,
  );
  let end = SystemTime::now();

  if let Ok(report) = guard.report().build() {
    let file = std::fs::File::create("onoro.svg").unwrap();
    report.flamegraph(file).unwrap();
  };

  println!("Done: {:?}", end.duration_since(start).unwrap());
  println!("Score: {score}");
}
