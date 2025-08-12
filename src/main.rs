use std::time::SystemTime;

use cooperate::solve_with_hasher;
use onoro::Onoro;
use onoro_impl::{Onoro16, OnoroView};

use crate::passthrough_hasher::BuildPassThroughHasher;

mod passthrough_hasher;

fn main() {
  // let onoro = Onoro16::from_board_string(
  //   ". B . . . . . . . . . . . .
  //     W B B B W B W B W B W W W B
  //      . . . . . . . . . . . . W .").unwrap();
  // let onoro = Onoro16::from_board_string(
  //   ". B . . . . . B W B W W W B
  //     W B B B W B W . . . . . W .").unwrap();
  let onoro = Onoro16::from_board_string(
    ". . . W . .
      . B B B W .
       . W B B B W
        B W W W B .
         . W . . . .",
  )
  .unwrap();

  println!("size of game state: {}", std::mem::size_of::<Onoro16>());
  println!(
    "size of game view: {}",
    std::mem::size_of_val(&OnoroView::new(Onoro16::default_start()))
  );

  println!("{}", onoro);

  let guard = pprof::ProfilerGuardBuilder::default()
    .frequency(1000)
    .blocklist(&["libc", "libgcc", "pthread", "vdso"])
    .build()
    .unwrap();

  let start = SystemTime::now();
  let options = cooperate::Options {
    num_threads: 16,
    search_depth: 9,
    unit_depth: 4,
  };
  let score = solve_with_hasher(&OnoroView::new(onoro), options, BuildPassThroughHasher);
  let end = SystemTime::now();

  if let Ok(report) = guard.report().build() {
    let file = std::fs::File::create("onoro.svg").unwrap();
    report.flamegraph(file).unwrap();
  };

  println!("Done: {:?}", end.duration_since(start).unwrap());
  println!("Score: {score}");
}
