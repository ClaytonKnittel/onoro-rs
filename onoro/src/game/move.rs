use super::packed_idx::PackedIdx;

#[derive(Debug)]
pub enum Move {
  Phase1Move {
    /// Position to place the pawn at.
    to: PackedIdx,
  },
  Phase2Move {
    /// Position to move the pawn to.
    to: PackedIdx,
    /// Position in pawn_poses array to move pawn from.
    from: u32,
  },
}
