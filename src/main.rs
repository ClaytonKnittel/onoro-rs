use onoro::Onoro;

fn main() {
  let game = Onoro::<16>::default_start();

  println!("size of game state: {}", std::mem::size_of::<Onoro::<16>>());

  println!("{}", game);
}
