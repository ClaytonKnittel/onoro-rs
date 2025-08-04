use std::collections::HashMap;

use onoro::{Move, Pawn, PawnColor};

struct Pos {
  x: i32,
  y: i32,
}

pub struct SimpleOnoro {
  positions: HashMap<Pos, Pawn>,
}

impl SimpleOnoro {
  pub fn default_start() -> Self {
    todo!();
  }

  pub fn finished(&self) -> Option<PawnColor> {
    todo!();
  }

  pub fn pawns(&self) -> impl Iterator<Item = Pawn> + '_ {
    todo!();
  }

  pub fn in_phase1(&self) -> bool {
    todo!();
  }

  pub fn each_move(&self) -> impl Iterator<Item = Move> {}

  pub fn make_move(&mut self, m: Move) {}
}
