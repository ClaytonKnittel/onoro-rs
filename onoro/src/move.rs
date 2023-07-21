use std::fmt::Display;

use super::{hex_pos::HexPos, packed_idx::PackedIdx};

#[derive(Clone, Debug, PartialEq, Eq)]
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

impl Display for Move {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Move::Phase1Move { to } => write!(f, "{}", HexPos::from(*to)),
      Move::Phase2Move { to, from_idx } => write!(f, "{} TODO: from", HexPos::from(*to)),
    }
  }
}
