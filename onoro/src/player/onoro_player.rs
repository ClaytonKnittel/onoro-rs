use std::{collections::HashSet, fmt::Display, io::BufRead, marker::PhantomData};

use abstract_game::{
  error::{GameInterfaceError, GameInterfaceResult},
  interactive::{
    human_player::HumanPlayer, line_reader::GameMoveLineReader, player::MakeMoveControl,
  },
  Game,
};
use itertools::Either;

use crate::{
  board_printer::display_on_hexagonal_grid, hex_pos::HexPosOffset, Onoro, OnoroMoveWrapper,
  OnoroPawn, PawnColor,
};

enum Phase {
  Initial,
  Phase2To { from: HexPosOffset },
}

pub struct OnoroPlayer<G> {
  phase: Phase,
  _phantom: PhantomData<G>,
}

impl<G: Onoro> OnoroPlayer<G> {
  pub fn new() -> Self {
    Self::default()
  }

  fn board_with_labeled_moves(game: &G) -> impl Display {
    let moves = if game.in_phase1() {
      Either::Left(game.each_move().enumerate().map(|(i, m)| {
        (
          match game.to_move_wrapper(&m) {
            OnoroMoveWrapper::Phase1 { to } => to.into(),
            OnoroMoveWrapper::Phase2 { .. } => unreachable!(),
          },
          (b'a' + i as u8) as char,
        )
      }))
    } else {
      let srcs: HashSet<HexPosOffset> = game
        .each_move()
        .map(|m| match game.to_move_wrapper(&m) {
          OnoroMoveWrapper::Phase2 { from, .. } => from.into(),
          _ => unreachable!(),
        })
        .collect();
      let dsts: HashSet<HexPosOffset> = game
        .each_move()
        .map(|m| match game.to_move_wrapper(&m) {
          OnoroMoveWrapper::Phase2 { to, .. } => to.into(),
          _ => unreachable!(),
        })
        .collect();

      let srcs_len = srcs.len();
      Either::Right(
        srcs
          .into_iter()
          .enumerate()
          .map(|(i, pos)| (pos, (b'a' + i as u8) as char))
          .chain(
            dsts
              .into_iter()
              .enumerate()
              .map(move |(i, pos)| (pos, (b'a' + (srcs_len + i) as u8) as char)),
          ),
      )
    };

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
        .chain(moves)
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
  ) -> GameInterfaceResult<MakeMoveControl<<G as Game>::Move>> {
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

    let m = game.each_move().nth(move_idx as usize).ok_or_else(|| {
      GameInterfaceError::MalformedMove(format!("{move_text} is not a valid move letter"))
    })?;

    Ok(MakeMoveControl::Done(m))
  }
}

impl<G: Onoro> Default for OnoroPlayer<G> {
  fn default() -> Self {
    Self {
      phase: Phase::Initial,
      _phantom: PhantomData,
    }
  }
}
