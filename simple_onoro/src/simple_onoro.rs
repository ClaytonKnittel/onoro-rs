use std::collections::HashMap;

use onoro::{Onoro, OnoroIndex, OnoroMove, OnoroPawn, PawnColor, TileState};

struct Pos {
  x: i32,
  y: i32,
}

#[derive(Clone, Copy)]
pub struct Index {}

impl OnoroIndex for Index {
  fn from_coords(x: u32, y: u32) -> Self {
    todo!()
  }

  fn x(&self) -> i32 {
    todo!()
  }

  fn y(&self) -> i32 {
    todo!()
  }
}

pub struct Move {}

impl OnoroMove<Index> for Move {
  fn make_phase1(pos: Index) -> Self {
    todo!()
  }
}

pub struct Pawn {}

impl OnoroPawn<Index> for Pawn {
  fn pos(&self) -> Index {
    todo!()
  }

  fn color(&self) -> PawnColor {
    todo!()
  }
}

pub struct SimpleOnoro {
  positions: HashMap<Pos, Pawn>,
}

impl Onoro for SimpleOnoro {
  type Index = Index;
  type Move = Move;
  type Pawn = Pawn;

  unsafe fn new() -> Self {
    todo!()
  }

  fn pawns_per_player() -> usize {
    todo!()
  }

  fn turn(&self) -> PawnColor {
    todo!()
  }

  fn pawns_in_play(&self) -> u32 {
    todo!()
  }

  fn finished(&self) -> Option<PawnColor> {
    todo!()
  }

  fn get_tile(&self, idx: Index) -> TileState {
    todo!()
  }

  fn pawns(&self) -> impl Iterator<Item = Pawn> + '_ {
    todo!();
    std::iter::empty()
  }

  fn in_phase1(&self) -> bool {
    todo!()
  }

  fn each_move(&self) -> impl Iterator<Item = Move> {
    todo!();
    std::iter::empty()
  }

  fn make_move(&mut self, m: Move) {
    todo!()
  }
}
