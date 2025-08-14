use abstract_game::OnoroIterator;
use onoro::hex_pos::HexPos;

use crate::{IdxOffset, Move, OnoroImpl, PackedIdx, util::packed_positions_bounding_box};

struct BoardVecIndexer {
  lower_left: PackedIdx,
  width: u8,
}

impl BoardVecIndexer {
  fn new(lower_left: PackedIdx, width: u8) -> Self {
    Self { lower_left, width }
  }

  fn index(&self, pos: PackedIdx) -> u32 {
    let d = unsafe { PackedIdx::from_idx_offset(pos - self.lower_left) };
    d.y() * self.width as u32 + d.x()
  }

  fn pos_from_index(&self, index: u32) -> PackedIdx {
    let x = index % self.width as u32;
    let y = index / self.width as u32;
    self.lower_left + IdxOffset::new(x as i32, y as i32)
  }

  fn neighbors_mask(&self, pos: PackedIdx) -> u64 {
    let mut neighbors = 0;
    let rel_pos = HexPos::from(pos) - HexPos::from(self.lower_left);
    for offset in HexPos::neighbor_offsets() {
      if (rel_pos.x() + offset.x()) < 0
        || (rel_pos.y() + offset.y()) < 0
        || pos.x() as i32 + offset.x() >= self.width as i32
        || pos.y() as i32 + offset.y() >= 0x10
        || rel_pos.x() + offset.x() + (rel_pos.y() + offset.y()) * self.width as i32
          >= u64::BITS as i32
      {
        continue;
      }

      let neighbor = HexPos::from(pos) + offset;
      debug_assert!(neighbor.x() >= self.lower_left.x());
      debug_assert!(neighbor.y() >= self.lower_left.y());

      let neighbor_idx = self.index(PackedIdx::new(neighbor.x(), neighbor.y()));
      neighbors |= 1u64 << neighbor_idx;
    }
    neighbors
  }
}

fn build_board_vec(pawn_poses: &[PackedIdx], indexer: &BoardVecIndexer) -> u64 {
  pawn_poses
    .iter()
    .filter(|&&pos| pos != PackedIdx::null())
    .map(|&pos| 1u64 << indexer.index(pos))
    .sum()
}

/// Returns a mask of all tiles that are empty and adjacent to a pawn on the
/// board.
fn all_possible_neighbors(board_vec: u64, indexer: &BoardVecIndexer) -> u64 {
  let mut neighbors = 0;
  let mut temp_board = board_vec;
  while temp_board != 0 {
    let index = temp_board.trailing_zeros();
    temp_board &= temp_board - 1;

    let pos = indexer.pos_from_index(index);
    debug_assert_ne!(pos.x(), 0);
    debug_assert_ne!(pos.y(), 0);
    debug_assert!(pos.x() < indexer.width as u32);

    neighbors |= indexer.neighbors_mask(pos);
  }

  neighbors & !board_vec
}

pub struct P1MoveGenerator<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> {
  board_vec: u64,
  neighbor_candidates: u64,
  indexer: BoardVecIndexer,
}

impl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize>
  P1MoveGenerator<N, N2, ADJ_CNT_SIZE>
{
  pub fn new(pawn_poses: &[PackedIdx; N]) -> Self {
    let (lower_left, upper_right) = packed_positions_bounding_box(pawn_poses);
    let delta = unsafe { PackedIdx::from_idx_offset(upper_right - lower_left) };

    let width = delta.x() + 3;
    let height = delta.y() + 3;

    let indexer = BoardVecIndexer::new(lower_left + IdxOffset::new(-1, -1), width as u8);

    if width * height > u64::BITS {
      todo!("Fallback to slow move generator if we can't fit the board in a u64");
    }

    let board_vec = build_board_vec(pawn_poses, &indexer);
    let neighbor_candidates = all_possible_neighbors(board_vec, &indexer);

    Self {
      board_vec,
      neighbor_candidates,
      indexer,
    }
  }
}

impl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> OnoroIterator
  for P1MoveGenerator<N, N2, ADJ_CNT_SIZE>
{
  type Item = Move;
  type Game = OnoroImpl<N, N2, ADJ_CNT_SIZE>;

  fn next(&mut self, _onoro: &Self::Game) -> Option<Self::Item> {
    let mut neighbor_candidates = self.neighbor_candidates;
    while neighbor_candidates != 0 {
      let index = neighbor_candidates.trailing_zeros();
      neighbor_candidates &= neighbor_candidates - 1;

      let pos = self.indexer.pos_from_index(index);

      let neighbors_mask = self.indexer.neighbors_mask(pos);
      if (neighbors_mask & self.board_vec).count_ones() >= 2 {
        self.neighbor_candidates = neighbor_candidates;
        return Some(Move::Phase1Move { to: pos });
      }
    }

    // No need to store neighbor_candidates again, since we typically don't
    // call next() again after None is returned.
    None
  }
}

#[cfg(test)]
mod tests {
  use abstract_game::OnoroIterator;
  use onoro::{Onoro, error::OnoroResult};

  use crate::{Onoro16, p1_move_gen::P1MoveGenerator};

  #[test]
  #[ignore]
  fn test_worst_case() -> OnoroResult {
    let worst_case = Onoro16::from_board_string(
      ". W . . . . . .
        B B . . . . . .
         . W . . . . . .
          . B . . . . . .
           . W . . . . . .
            . B . . . . . .
             . W . . . . . .
              . B W B W B W B
               . . . . . . W .",
    )?;

    let mut move_gen = P1MoveGenerator::new(worst_case.pawn_poses());
    move_gen.next(&worst_case);

    Ok(())
  }
}
