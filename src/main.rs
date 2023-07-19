use onoro::Onoro;

fn main() {
  let game = Onoro::<16>::new();

  println!("size of game state: {}", std::mem::size_of::<Onoro::<16>>());

  println!("{}", game);
}
