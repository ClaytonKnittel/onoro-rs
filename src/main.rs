use onoro::Onoro;

fn main() {
  let game = Onoro::<16>::default_start();

  println!("size of game state: {}", std::mem::size_of::<Onoro::<16>>());

  println!("{}", game);

  for pawn in game.pawns() {
    println!("{}", pawn);
  }

  println!("Black pawns");
  for pawn in game.color_pawns(onoro::PawnColor::Black) {
    println!("{}", pawn);
  }
  println!("White pawns");
  for pawn in game.color_pawns(onoro::PawnColor::White) {
    println!("{}", pawn);
  }
}
