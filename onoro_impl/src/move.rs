use std::fmt::Display;

use onoro::{OnoroMove, hex_pos::HexPos};

use super::packed_idx::PackedIdx;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
  Phase1Move {
    /// Position to place the pawn at.
    to: PackedIdx,
  },
  Phase2Move {
    /// Position to move the pawn to.
    to: PackedIdx,
    /// Position in pawn_poses array to move pawn from.
    from_idx: u32,
  },
}

impl OnoroMove for Move {
  type Index = PackedIdx;

  fn make_phase1(pos: PackedIdx) -> Self {
    Self::Phase1Move { to: pos }
  }
}

impl Display for Move {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Move::Phase1Move { to } => write!(f, "{}", HexPos::from(*to)),
      Move::Phase2Move { to, from_idx } => write!(f, "{} from idx {from_idx}", HexPos::from(*to)),
    }
  }
}
