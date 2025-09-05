use std::fmt::Debug;

use crate::{Game, Score};

pub trait Solver {
  fn solve<G: Game + Debug>(&mut self, game: &G, depth: u32) -> (Score, Option<G::Move>);
}
