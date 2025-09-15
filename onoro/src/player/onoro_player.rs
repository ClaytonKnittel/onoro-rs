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

  fn board_with_labeled_moves(game: &G) -> String {
    "".to_owned()
  }
}

impl<G: Onoro> HumanPlayer for OnoroPlayer<G> {
  type Game = G;

  fn prompt_move_text(&self, game: &G) -> String {
    format!(
      "{}\nWhere would you like to go?",
      Self::board_with_labeled_moves(game)
    )
  }

  fn parse_move(&self, _move_text: &str, game: &G) -> GameInterfaceResult<<G as Game>::Move> {
    Ok(game.each_move().next().unwrap())
  }
}

impl<G: Onoro> Default for OnoroPlayer<G> {
  fn default() -> Self {
    Self {
      _phantom: PhantomData,
    }
  }
}
