use std::{fmt::Display, io::BufRead, marker::PhantomData};

use abstract_game::{
  error::{GameInterfaceError, GameInterfaceResult},
  interactive::{human_player::HumanPlayer, line_reader::GameMoveLineReader},
  Game,
};

use crate::{
  board_printer::display_on_hexagonal_grid, Onoro, OnoroMoveWrapper, OnoroPawn, PawnColor,
};

pub struct OnoroPlayer<G> {
  _phantom: PhantomData<G>,
}

impl<G: Onoro> OnoroPlayer<G> {
  pub fn new() -> Self {
    Self::default()
  }

  fn board_with_labeled_moves(game: &G) -> impl Display {
    display_on_hexagonal_grid(
      game
        .pawns()
        .map(|pawn| {
          (
            pawn.pos().into(),
            match pawn.color() {
              PawnColor::Black => 'B',
              PawnColor::White => 'W',
            },
          )
        })
        .chain(game.each_move().enumerate().map(|(i, m)| {
          (
            match game.to_move_wrapper(&m) {
              OnoroMoveWrapper::Phase1 { to } => to.into(),
              OnoroMoveWrapper::Phase2 { from, .. } => from.into(),
            },
            (b'a' + i as u8) as char,
          )
        }))
        .collect(),
    )
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

  fn parse_move<I: BufRead>(
    &self,
    mut move_reader: GameMoveLineReader<I>,
    game: &G,
  ) -> GameInterfaceResult<<G as Game>::Move> {
    let move_text = move_reader.next_line()?;
    let mut chars = move_text.chars();
    let c = chars
      .next()
      .ok_or_else(|| GameInterfaceError::MalformedMove("No move was given!".to_owned()))?;
    if chars.next().is_some() {
      return Err(GameInterfaceError::MalformedMove(format!(
        "{move_text} is not a single letter"
      )));
    }

    if !c.is_alphabetic() {
      return Err(GameInterfaceError::MalformedMove(format!(
        "{move_text} is not a letter"
      )));
    }

    let move_idx = c as u8 - b'a';

    game.each_move().nth(move_idx as usize).ok_or_else(|| {
      GameInterfaceError::MalformedMove(format!("{move_text} is not a valid move letter"))
    })
  }
}

impl<G: Onoro> Default for OnoroPlayer<G> {
  fn default() -> Self {
    Self {
      _phantom: PhantomData,
    }
  }
}
