use onoro::{Move, Onoro16};
use rand::Rng;

fn validate_moves(onoro: &Onoro16) {
  let mut move_iter = onoro.each_p1_move();
  onoro.for_each_move(|m| {
    assert_eq!(move_iter.next().unwrap(), m);
    true
  });
  assert!(move_iter.next().is_none());
}

fn random_move(onoro: &Onoro16) -> Move {
  onoro.each_p1_move().next().unwrap()
  // let moves = onoro.each_p1_move().collect::<Vec<_>>();

  // let mut rng = rand::thread_rng();
  // let n = rng.gen_range(0..moves.len());
  // moves[n].clone()
}

fn main() {
  let game = Onoro16::default_start();

  println!("size of game state: {}", std::mem::size_of::<Onoro16>());

  println!("{}", game);

  for _ in 0..1000000 {
    let mut g = game.clone();
    for _ in 0..13 {
      let m = random_move(&g);
      g.make_move(m);
      // if g.in_phase1() {
      //   validate_moves(&g);
      // }
      // g.validate().unwrap();
    }
    // println!("{g}");
  }

  game.validate().unwrap();
}
