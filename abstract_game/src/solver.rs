use std::iter::successors;

use crate::{Game, GameResult, Score};

pub trait Solver {
  fn solve<G: Game>(&mut self, game: &G, depth: u32) -> (Score, Option<G::Move>);

  fn playout<G: Game>(&mut self, game: &G, depth: u32) -> impl Iterator<Item = (G, G::Move)> {
    let (_, m) = self.solve(game, depth);
    successors(m.map(|m| (game.with_move(m), m)), move |(game, _)| {
      if matches!(game.finished(), GameResult::Win(_) | GameResult::Tie) {
        return None;
      }

      let (_, m) = self.solve(game, depth);
      m.map(|m| (game.with_move(m), m))
    })
  }
}
