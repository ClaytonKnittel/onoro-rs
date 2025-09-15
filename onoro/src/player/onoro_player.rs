use std::marker::PhantomData;

use abstract_game::{error::GameInterfaceResult, interactive::human_player::HumanPlayer, Game};

use crate::Onoro;

pub struct OnoroPlayer<G> {
  _phantom: PhantomData<G>,
}

impl<G: Onoro> OnoroPlayer<G> {
  pub fn new() -> Self {
    Self::default()
  }
}

impl<G: Onoro> Default for OnoroPlayer<G> {
  fn default() -> Self {
    Self {
      _phantom: PhantomData,
    }
  }
}

impl<G: Onoro> HumanPlayer for OnoroPlayer<G> {
  type Game = G;

  fn prompt_move_text(&self, _game: &G) -> String {
    "Where would you like to go?".to_owned()
  }

  fn parse_move(&self, _move_text: &str, game: &G) -> GameInterfaceResult<<G as Game>::Move> {
    Ok(game.each_move().next().unwrap())
  }
}
