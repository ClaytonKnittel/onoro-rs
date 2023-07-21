use onoro::Onoro16;

fn main() {
  let game = Onoro16::default_start();

  println!("size of game state: {}", std::mem::size_of::<Onoro16>());

  println!("{}", game);

  game.for_each_move(|m| {
    println!("{m}");
    let mut new_game = game.clone();
    new_game.make_move(m);
    println!("{new_game}");
    true
  });

  for m in game.each_p1_move() {
    println!("{m}");
    let mut new_game = game.clone();
    new_game.make_move(m);
    println!("{new_game}");
  }

  game.validate().unwrap();
}
