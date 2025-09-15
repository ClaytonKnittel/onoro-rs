use onoro::abstract_game::{Score, Solver};

use crate::{Move, OnoroImpl, OnoroView};

pub struct OnoroSolver<S> {
  solver: S,
}

impl<S: Solver> OnoroSolver<S> {
  pub fn new(solver: S) -> Self {
    Self { solver }
  }
}

impl<const N: usize, S> Solver for OnoroSolver<S>
where
  S: Solver<Game = OnoroView<N>>,
{
  type Game = OnoroImpl<N>;

  fn best_move(&mut self, game: &Self::Game, depth: u32) -> (Score, Option<Move>) {
    self.solver.best_move(&OnoroView::new(game.clone()), depth)
  }
}
