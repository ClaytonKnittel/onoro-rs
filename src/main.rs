use onoro::Onoro;

fn main() {
  let game = Onoro::<16, 256>::default_start();

  println!(
    "size of game state: {}",
    std::mem::size_of::<Onoro::<16, 256>>()
  );

  println!("{}", game);

  game.validate().unwrap();
}
