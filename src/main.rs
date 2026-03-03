use std::time::SystemTime;

use cooperate::{passthrough_hasher::BuildPassThroughHasher, solvers::ttable_solver::TTSolver};
use onoro::{
  abstract_game::{Game, Solver},
  Onoro,
};
use onoro_impl::{Onoro16, OnoroView};

fn main() {
  // let onoro = Onoro16::from_board_string(
  //   ". B . . . . . . . . . . . .
  //     W B B B W B W B W B W W W B
  //      . . . . . . . . . . . . W .").unwrap();
  // let onoro = Onoro16::from_board_string(
  //   ". B . . . . . B W B W W W B
  //     W B B B W B W . . . . . W .").unwrap();
  // let onoro = Onoro16::from_board_string(
  //   ". . . W . .
  //     . B B B W .
  //      . W B B B W
  //       B W W W B .
  //        . W . . . .",
  // )
  // .unwrap();
  let onoro = Onoro16::from_board_string(
    ". . . . .
      . . B W .
       . B B B .
        . W W . .
         . . . . .",
  )
  .unwrap();

  println!("size of game state: {}", std::mem::size_of::<Onoro16>());
  println!(
    "size of game view: {}",
    std::mem::size_of_val(&OnoroView::new(Onoro16::default_start()))
  );

  println!("{}", onoro);

  let mut solver = TTSolver::with_hasher(BuildPassThroughHasher);

  let start = SystemTime::now();
  let mut game = OnoroView::new(onoro);
  for depth in (0..=8).rev() {
    let (score, m) = solver.best_move(&game, depth);
    println!("Making move {m:?} with score {score}");
    game.make_move(m.unwrap());
    println!("{game}");

    if game.finished().is_finished() {
      break;
    }
  }
  let end = SystemTime::now();

  println!("Done: {:?}", end.duration_since(start).unwrap());
}
