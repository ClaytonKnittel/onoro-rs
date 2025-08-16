use abstract_game::GameMoveIterator;
use onoro::Onoro;

use crate::{Move, OnoroImpl};

pub struct P2MoveGenerator<const N: usize> {}

impl<const N: usize> P2MoveGenerator<N> {
  pub fn new(onoro: &OnoroImpl<N>) -> Self {
    debug_assert!(!onoro.in_phase1());

    Self {}
  }
}

impl<const N: usize> GameMoveIterator for P2MoveGenerator<N> {
  type Item = Move;
  type Game = OnoroImpl<N>;

  fn next(&mut self, _game: &Self::Game) -> Option<Self::Item> {
    None
  }
}
