use abstract_game::OnoroIterator;

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

  fn build_bitvecs(&self, pawn_poses: &[PackedIdx]) -> (u64, u64) {
    let width = self.width as u32;
    let neighbors_mask = 0x3 | (0x5 << width) | (0x6 << (2 * width));

    let (board, neighbor_candidates) = pawn_poses
      .iter()
      .filter(|&&pos| pos != PackedIdx::null())
      .fold((0, 0), |(board_vec, neighbors_vec), &pos| {
        let index = self.index(pos);
        debug_assert!(index > width);
        (
          board_vec | (1u64 << index),
          neighbors_vec | (neighbors_mask << (index - width - 1)),
        )
      });

    (board, neighbor_candidates & !board)
  }

  fn neighbors_mask(&self, index: u32) -> u64 {
    let lesser_neighbors_mask = 0x3 | (0x1 << self.width);
    let greater_neighbors_mask = 0x2 | (0x3 << self.width);

    let lesser_neighbors = (lesser_neighbors_mask << index) >> (self.width + 1);
    let greater_neighbors = greater_neighbors_mask << index;

    lesser_neighbors | greater_neighbors
  }
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

    let (board_vec, neighbor_candidates) = indexer.build_bitvecs(pawn_poses);

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

      let neighbors_mask = self.indexer.neighbors_mask(index);
      if (neighbors_mask & self.board_vec).count_ones() >= 2 {
        self.neighbor_candidates = neighbor_candidates;
        return Some(Move::Phase1Move {
          to: self.indexer.pos_from_index(index),
        });
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
  use onoro::{Onoro, error::OnoroResult, hex_pos::HexPos, test_util::BOARD_POSITIONS};
  use rstest::rstest;
  use rstest_reuse::{apply, template};

  use crate::{
    Onoro16, PackedIdx,
    p1_move_gen::{BoardVecIndexer, P1MoveGenerator},
  };

  fn build_board_vec(pawn_poses: &[PackedIdx], indexer: &BoardVecIndexer) -> u64 {
    pawn_poses
      .iter()
      .filter(|&&pos| pos != PackedIdx::null())
      .map(|&pos| 1u64 << indexer.index(pos))
      .sum()
  }

  fn neighbors_mask(pos: PackedIdx, indexer: &BoardVecIndexer) -> u64 {
    let mut neighbors = 0;
    let rel_pos = HexPos::from(pos) - HexPos::from(indexer.lower_left);
    for offset in HexPos::neighbor_offsets() {
      if (rel_pos.x() + offset.x()) < 0
        || (rel_pos.y() + offset.y()) < 0
        || rel_pos.x() + offset.x() >= indexer.width as i32
        || pos.y() as i32 + offset.y() >= 0x10
        || rel_pos.x() + offset.x() + (rel_pos.y() + offset.y()) * indexer.width as i32
          >= u64::BITS as i32
      {
        continue;
      }

      let neighbor = HexPos::from(pos) + offset;
      debug_assert!(neighbor.x() >= indexer.lower_left.x());
      debug_assert!(neighbor.y() >= indexer.lower_left.y());

      let neighbor_idx = indexer.index(PackedIdx::new(neighbor.x(), neighbor.y()));
      neighbors |= 1u64 << neighbor_idx;
    }
    neighbors
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

      neighbors |= neighbors_mask(pos, indexer);
    }

    neighbors & !board_vec
  }

  #[template]
  fn test_build(
    #[values(
      Onoro16::default_start(),
      Onoro16::from_board_string(BOARD_POSITIONS[0]).unwrap(),
      Onoro16::from_board_string(BOARD_POSITIONS[1]).unwrap(),
    )]
    onoro: Onoro16,
  ) {
  }

  #[apply(test_build)]
  #[rstest]
  fn test_build_board_vec(onoro: Onoro16) {
    let move_gen = P1MoveGenerator::<_, 1, 1>::new(onoro.pawn_poses());
    let indexer = &move_gen.indexer;
    let board_vec = build_board_vec(onoro.pawn_poses(), indexer);

    assert_eq!(move_gen.board_vec, board_vec);
  }

  #[apply(test_build)]
  #[rstest]
  fn test_build_possible_neighbors_vec(onoro: Onoro16) {
    let move_gen = P1MoveGenerator::<_, 1, 1>::new(onoro.pawn_poses());
    let indexer = &move_gen.indexer;

    let board_vec = build_board_vec(onoro.pawn_poses(), indexer);
    let neighbor_candidates = all_possible_neighbors(board_vec, indexer);

    assert_eq!(
      move_gen.neighbor_candidates, neighbor_candidates,
      "{:#016x} vs. {:#016x}",
      move_gen.neighbor_candidates, neighbor_candidates
    );
  }

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
