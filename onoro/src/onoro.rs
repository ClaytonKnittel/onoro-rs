use crate::{Move, Pawn, PawnColor};

pub trait Onoro {
  fn default_start() -> Self;

  fn finished(&self) -> Option<PawnColor>;

  fn pawns(&self) -> impl Iterator<Item = Pawn> + '_;

  fn in_phase1(&self) -> bool;

  fn each_move(&self) -> impl Iterator<Item = Move>;

  fn make_move(&mut self, m: Move);
}
