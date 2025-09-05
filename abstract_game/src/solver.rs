use crate::{Game, Score};

pub trait Solver {
  fn solve<G: Game>(&mut self, game: &G, depth: u32) -> Score;
}
