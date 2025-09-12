use std::time::SystemTime;

use cooperate::{cooperate::solve_with_hasher, passthrough_hasher::BuildPassThroughHasher};
use onoro::Onoro;
use onoro_impl::{Onoro16, OnoroView};

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

  let start = SystemTime::now();
  let options = cooperate::cooperate::Options {
    num_threads: 16,
    search_depth: 9,
    unit_depth: 4,
  };
  let score = solve_with_hasher(&OnoroView::new(onoro), options, BuildPassThroughHasher);
  let end = SystemTime::now();

  println!("Done: {:?}", end.duration_since(start).unwrap());
  println!("Score: {score}");
}
