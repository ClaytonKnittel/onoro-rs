use std::collections::HashMap;

use onoro::{Move, Onoro, PackedIdx, Pawn, PawnColor, TileState};

struct Pos {
  x: i32,
  y: i32,
}

pub struct SimpleOnoro {
  positions: HashMap<Pos, Pawn>,
}

impl Onoro for SimpleOnoro {
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

  fn get_tile(&self, idx: PackedIdx) -> TileState {
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
