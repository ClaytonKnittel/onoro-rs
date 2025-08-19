use abstract_game::GameMoveIterator;
use itertools::Either;
use num_traits::{PrimInt, Unsigned};
use onoro::hex_pos::HexPos;

use crate::{
  Move, OnoroImpl, PackedIdx,
  board_vec_indexer::{Basis, BoardVecIndexer, DetermineBasisOutput, determine_basis},
  util::{likely, packed_positions_coord_limits},
};

struct Impl<I> {
  /// A bitvector representation of the board, with bits set if there is a pawn
  /// present in the corresponding tile. This includes a 1-tile perimeter of
  /// empty tiles so that `board_vec` and `neighbor_candidates` can use the
  /// same indexer.
  board_vec: I,
  /// A bitvector of all tiles with at least one pawn neighbor which are not
  /// occupied by a pawn already.
  neighbor_candidates: I,
  indexer: BoardVecIndexer,
}

impl<I: Unsigned + PrimInt> Impl<I> {
  /// Initializes the move generator, which builds the board vec and neighbor
  /// candidates masks.
  fn new_impl<const N: usize>(
    basis: Basis,
    corner: HexPos,
    width: u8,
    pawn_poses: &[PackedIdx; N],
  ) -> Self {
    let indexer = BoardVecIndexer::new(basis, corner, width);
    let (board_vec, neighbor_candidates) = indexer.build_bitvecs(pawn_poses);
    Self {
      board_vec,
      neighbor_candidates,
      indexer,
    }
  }

  /// Finds the tile index for the next move we can make, or `None` if all
  /// moves have been found. Returns the tile index for the next move, and an
  /// iterator over the tile indices of that move's neighbors.
  fn next_with_neighbors_impl(&mut self) -> Option<(usize, impl Iterator<Item = u32> + use<I>)> {
    let mut neighbor_candidates = self.neighbor_candidates;
    while neighbor_candidates != I::zero() {
      let index = neighbor_candidates.trailing_zeros() as usize;
      neighbor_candidates = neighbor_candidates & (neighbor_candidates - I::one());

      let neighbors_mask: I = self.indexer.neighbors_mask(index);
      let neighbors_mask = neighbors_mask & self.board_vec;
      if neighbors_mask.count_ones() >= 2 {
        self.neighbor_candidates = neighbor_candidates;
        return Some((index, neighbors_mask.iter_ones()));
      }
    }

    // No need to store neighbor_candidates again, since we typically don't
    // call next() again after None is returned.
    None
  }

  /// Finds the tile index for the next move we can make, or `None` if all
  /// moves have been found.
  fn next_impl(&mut self) -> Option<usize> {
    self.next_with_neighbors_impl().map(|(index, _)| index)
  }

  /// Returns an iterator over the indices of the neighbors of the pawn at the
  /// given index.
  fn neighbors(&self, index: usize) -> impl Iterator<Item = u32> {
    let neighbors_mask: I = self.indexer.neighbors_mask(index);
    (neighbors_mask & self.board_vec).iter_ones()
  }
}

impl Impl<u64> {
  fn new<const N: usize>(
    basis: Basis,
    corner: HexPos,
    width: u8,
    pawn_poses: &[PackedIdx; N],
  ) -> Self {
    Self::new_impl(basis, corner, width, pawn_poses)
  }

  fn next_with_neighbors(&mut self) -> Option<(usize, impl Iterator<Item = u32> + use<>)> {
    self.next_with_neighbors_impl()
  }

  fn next(&mut self) -> Option<usize> {
    self.next_impl()
  }
}

impl Impl<u128> {
  #[cold]
  fn new<const N: usize>(
    basis: Basis,
    corner: HexPos,
    width: u8,
    pawn_poses: &[PackedIdx; N],
  ) -> Self {
    Self::new_impl(basis, corner, width, pawn_poses)
  }

  #[cold]
  fn next_with_neighbors(&mut self) -> Option<(usize, impl Iterator<Item = u32> + use<>)> {
    self.next_with_neighbors_impl()
  }

  #[cold]
  fn next(&mut self) -> Option<usize> {
    self.next_impl()
  }
}

enum ImplContainer {
  /// We use this repr when the board bitvec is small enough to fit in a u64,
  /// including a 1-tile padding around the perimeter. This is much faster to
  /// operate on than a u128.
  Small(Impl<u64>),
  /// We need to support any board size. The largest possible board is 8 x 9
  /// (see test_worst_case below), which, with a 1-tile padding, requires 110
  /// bits for the board bitvec.
  Large(Box<Impl<u128>>),
}

/// The phase 1 move generator, where not all pawns have been placed and a move
/// consists of adding a new pawn to the board adjacent to at least 2 other
/// pawns.
pub struct P1MoveGenerator<const N: usize> {
  impl_container: ImplContainer,
}

impl<const N: usize> P1MoveGenerator<N> {
  pub fn indexer(&self) -> &BoardVecIndexer {
    match &self.impl_container {
      ImplContainer::Small(impl_) => &impl_.indexer,
      ImplContainer::Large(impl_) => &impl_.indexer,
    }
  }
}

impl<const N: usize> P1MoveGenerator<N> {
  pub fn new(onoro: &OnoroImpl<N>) -> Self {
    Self::from_pawn_poses(onoro.pawn_poses())
  }

  pub fn from_pawn_poses(pawn_poses: &[PackedIdx; N]) -> Self {
    // Compute the bounding parallelogram of the pawns that have been placed,
    // which is min/max x/y in coordinate space.
    let coord_limits = packed_positions_coord_limits(onoro.pawn_poses());
    let DetermineBasisOutput {
      basis,
      corner,
      width,
      use_u128,
    } = determine_basis::<N>(coord_limits);

    // Specialize for the case where the board bitvec fits in a u64, which is
    // by far the most common. Only in pathological cases will we need more
    // than 64 bits.
    if likely(!use_u128) {
      P1MoveGenerator {
        impl_container: ImplContainer::Small(Impl::<u64>::new(basis, corner, width, pawn_poses)),
      }
    } else {
      P1MoveGenerator {
        impl_container: ImplContainer::Large(
          Impl::<u128>::new(basis, corner, width, pawn_poses).into(),
        ),
      }
    }
  }

  /// Returns a tuple of (neighbor index iterator, neighbor count), where the
  /// iterator is guaranteed to yield "neighbor count" elements.
  pub fn neighbors(&self, index: usize) -> impl Iterator<Item = u32> {
    match &self.impl_container {
      ImplContainer::Small(impl_) => Either::Left(impl_.neighbors(index)),
      ImplContainer::Large(impl_) => Either::Right(impl_.neighbors(index)),
    }
  }

  pub fn next_move_index(&mut self) -> Option<usize> {
    match &mut self.impl_container {
      ImplContainer::Small(impl_) => impl_.next(),
      ImplContainer::Large(impl_) => impl_.next(),
    }
  }

  pub fn next_move_pos(&mut self) -> Option<PackedIdx> {
    self
      .next_move_index()
      .map(|index| self.indexer().pos_from_index(index as u32))
  }

  pub fn next_move_pos_with_neighbors(
    &mut self,
  ) -> Option<(PackedIdx, impl Iterator<Item = u32> + use<N>)> {
    let (index, iter) = match &mut self.impl_container {
      ImplContainer::Small(impl_) => {
        let (index, iter) = impl_.next_with_neighbors()?;
        (index, Either::Left(iter))
      }
      ImplContainer::Large(impl_) => {
        let (index, iter) = impl_.next_with_neighbors()?;
        (index, Either::Right(iter))
      }
    };
    Some((self.indexer().pos_from_index(index as u32), iter))
  }
}

impl<const N: usize> GameMoveIterator for P1MoveGenerator<N> {
  type Item = Move;
  type Game = OnoroImpl<N>;

  fn next(&mut self, _onoro: &Self::Game) -> Option<Self::Item> {
    self.next_move_pos().map(|to| Move::Phase1Move { to })
  }
}

#[cfg(test)]
mod tests {
  use abstract_game::GameMoveIterator;
  use onoro::{Onoro, error::OnoroResult, hex_pos::HexPos, test_util::BOARD_POSITIONS};
  use rstest::rstest;
  use rstest_reuse::{apply, template};

  use crate::{
    FilterNullPackedIdx, Onoro16, PackedIdx,
    p1_move_gen::{BoardVecIndexer, ImplContainer, P1MoveGenerator},
  };

  fn get_board_vec<const N: usize>(move_gen: &P1MoveGenerator<N>) -> u128 {
    match &move_gen.impl_container {
      ImplContainer::Small(impl_) => impl_.board_vec as u128,
      ImplContainer::Large(impl_) => impl_.board_vec,
    }
  }

  fn get_neighbor_candidates<const N: usize>(move_gen: &P1MoveGenerator<N>) -> u128 {
    match &move_gen.impl_container {
      ImplContainer::Small(impl_) => impl_.neighbor_candidates as u128,
      ImplContainer::Large(impl_) => impl_.neighbor_candidates,
    }
  }

  fn build_board_vec(pawn_poses: &[PackedIdx], indexer: &BoardVecIndexer) -> u128 {
    pawn_poses
      .iter()
      .filter_null()
      .map(|&pos| 1 << indexer.index(pos))
      .sum()
  }

  fn neighbors_mask(pos: PackedIdx, indexer: &BoardVecIndexer) -> u128 {
    let mut neighbors = 0;
    for offset in HexPos::neighbor_offsets() {
      let neighbor = HexPos::from(pos) + offset;
      let neighbor_idx = indexer.index(PackedIdx::new(neighbor.x(), neighbor.y()));
      neighbors |= 1 << neighbor_idx;
    }
    neighbors
  }

  /// Returns a mask of all tiles that are empty and adjacent to a pawn on the
  /// board.
  fn all_possible_neighbors(board_vec: u128, indexer: &BoardVecIndexer) -> u128 {
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
    let move_gen = P1MoveGenerator::new(&onoro);
    let indexer = &move_gen.indexer();
    let board_vec = build_board_vec(onoro.pawn_poses(), indexer);

    assert_eq!(get_board_vec(&move_gen), board_vec);
  }

  #[apply(test_build)]
  #[rstest]
  fn test_build_possible_neighbors_vec(onoro: Onoro16) {
    let move_gen = P1MoveGenerator::new(&onoro);
    let indexer = &move_gen.indexer();

    let board_vec = build_board_vec(onoro.pawn_poses(), indexer);
    let neighbor_candidates = all_possible_neighbors(board_vec, indexer);

    assert_eq!(
      get_neighbor_candidates(&move_gen),
      neighbor_candidates,
      "{:#016x} vs. {:#016x}",
      get_neighbor_candidates(&move_gen),
      neighbor_candidates
    );
  }

  #[test]
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

    let move_gen = P1MoveGenerator::new(&worst_case);
    assert_eq!(move_gen.to_iter(&worst_case).count(), 25);

    Ok(())
  }

  #[test]
  fn test_line_x() -> OnoroResult {
    let line_x = Onoro16::from_board_string(
      ". B . . . . . . . . . . . .
        W B W B W B W B W B W B W B
         . . . . . . . . . . . . W .",
    )?;

    let move_gen = P1MoveGenerator::new(&line_x);
    assert_eq!(move_gen.to_iter(&line_x).count(), 26);

    Ok(())
  }

  #[test]
  fn test_line_y() -> OnoroResult {
    let line_y = Onoro16::from_board_string(
      ". B .
        W B .
         . W .
          . B .
           . W .
            . B .
             . W .
              . B .
               . W .
                . B .
                 . W .
                  . B .
                   . W B
                    . W .",
    )?;

    let move_gen = P1MoveGenerator::new(&line_y);
    assert_eq!(move_gen.to_iter(&line_y).count(), 26);

    Ok(())
  }

  #[test]
  fn test_line_xy() -> OnoroResult {
    let line_xy = Onoro16::from_board_string(
      ". . . . . . . . . . . . W B
        . . . . . . . . . . . . W .
         . . . . . . . . . . . B . .
          . . . . . . . . . . W . . .
           . . . . . . . . . B . . . .
            . . . . . . . . W . . . . .
             . . . . . . . B . . . . . .
              . . . . . . W . . . . . . .
               . . . . . B . . . . . . . .
                . . . . W . . . . . . . . .
                 . . . B . . . . . . . . . .
                  . . W . . . . . . . . . . .
                   . B . . . . . . . . . . . .
                    B W . . . . . . . . . . . .",
    )?;

    let move_gen = P1MoveGenerator::new(&line_xy);
    assert_eq!(move_gen.to_iter(&line_xy).count(), 26);

    Ok(())
  }
}
