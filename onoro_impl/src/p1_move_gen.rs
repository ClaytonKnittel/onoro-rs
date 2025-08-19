use abstract_game::GameMoveIterator;
use num_traits::{PrimInt, Unsigned};
use onoro::hex_pos::HexPos;

use crate::{
  FilterNullPackedIdx, Move, OnoroImpl, PackedIdx,
  util::{CoordLimits, MinAndMax, likely, packed_positions_coord_limits},
};

#[derive(Clone, Copy)]
enum Basis {
  ///```text
  /// Γ
  ///  \
  ///   \
  ///    +--->
  ///```
  XvY,
  ///```text
  ///    7
  ///   /
  ///  /
  /// +--->
  ///```
  XvXY,
  ///```text
  /// Γ     7
  ///  \   /
  ///   \ /
  ///    +
  ///```
  XYvY,
}

struct DetermineBasisOutput {
  basis: Basis,
  corner: HexPos,
  width: u8,
  use_u128: bool,
}

fn determine_basis<const N: usize>(coord_limits: CoordLimits) -> DetermineBasisOutput {
  let x = coord_limits.x();
  let y = coord_limits.y();
  let xy = coord_limits.xy();

  let dx = x.delta();
  let dy = y.delta();
  let dxy = xy.delta();

  let build_output =
    |basis: Basis, corner: HexPos, coord1: MinAndMax<u32>, coord2: MinAndMax<u32>| {
      DetermineBasisOutput {
        basis,
        corner,
        width: (coord1.delta() + 3) as u8,
        // We will represent the board with a bitvector, where each bit corresponds
        // to a tile and is set if there is a pawn there. We want a 1-tile padding
        // around the perimeter so we can also represent the neighbor candidates
        // with a bitvec.
        use_u128: (coord1.delta() + 3) * (coord2.delta() + 3) > u64::BITS,
      }
    };

  let max = dx.max(dy).max(dxy);
  if dxy == max {
    let x_y_corner = HexPos::new(x.min() - 1, y.min() - 1);
    build_output(Basis::XvY, x_y_corner, x, y)
  } else if dy == max {
    let xy_y_corner = HexPos::new(
      x.min() - 1,
      x.min() + xy.max() - PackedIdx::xy_offset::<N>(),
    );
    build_output(Basis::XYvY, xy_y_corner, xy, x)
  } else {
    debug_assert_eq!(dx, max);
    let x_xy_corner = HexPos::new(
      y.min() + PackedIdx::xy_offset::<N>() - xy.min(),
      y.min() - 1,
    );
    build_output(Basis::XvXY, x_xy_corner, y, xy)
  }
}

/// An indexing schema for mapping HexPos <-> bitvector index.
struct BoardVecIndexer {
  /// The basis that is used for indexing positions.
  basis: Basis,
  /// The corner of the minimal bounding box containing all placed pawns, with
  /// a 1-tile perimeter of empty tiles.
  corner: HexPos,
  /// The width of this minimal bounding box.
  width: u8,
}

impl BoardVecIndexer {
  fn new(basis: Basis, corner: HexPos, width: u8) -> Self {
    Self {
      basis,
      corner,
      width,
    }
  }

  const fn coords(&self, pos: PackedIdx) -> (u32, u32) {
    let x = pos.x();
    let y = pos.y();
    let cx = self.corner.x();
    let cy = self.corner.y();
    match self.basis {
      Basis::XvY => (x - cx, y - cy),
      Basis::XvXY => (y - cy, y + cx - x - cy),
      Basis::XYvY => (x + cy - y - cx, x - cx),
    }
  }

  const fn pos_from_coords(&self, coords: (u32, u32)) -> PackedIdx {
    let (x, y) = coords;
    let cx = self.corner.x();
    let cy = self.corner.y();
    match self.basis {
      Basis::XvY => PackedIdx::new(x + cx, y + cy),
      Basis::XvXY => PackedIdx::new(x + cx - y, x + cy),
      Basis::XYvY => PackedIdx::new(y + cx, y + cy - x),
    }
  }

  /// Maps a `PackedIdx` from the Onoro state to an index in the board bitvec.
  fn index(&self, pos: PackedIdx) -> usize {
    let (c1, c2) = self.coords(pos);
    debug_assert!(c1 < self.width as u32);
    c2 as usize * self.width as usize + c1 as usize
  }

  /// Maps an index from the board bitvec to a `PackedIdx` in the Onoro state.
  fn pos_from_index(&self, index: u32) -> PackedIdx {
    debug_assert!((3..=16).contains(&self.width));
    let c1 = index % self.width as u32;
    let c2 = index / self.width as u32;
    self.pos_from_coords((c1, c2))
  }

  /// Builds both the board bitvec and neighbor candidates. The board bitvec
  /// has a 1 in each index corresponding to an occupied tile, and the neighbor
  /// candidates have a 1 in each index corresponding to an empty neighbor of
  /// any pawn.
  fn build_bitvecs<I: PrimInt>(&self, pawn_poses: &[PackedIdx]) -> (I, I) {
    let width = self.width as usize;

    let board = pawn_poses
      .iter()
      .filter_null()
      .fold(I::zero(), |board_vec, &pos| {
        let index = self.index(pos);
        debug_assert!(index > width);
        board_vec | (I::one() << index)
      });

    // All neighbors are -(width+1), -width, -1, +1, +width, +(width+1) in
    // index space.
    let neighbor_candidates = (board >> (width + 1))
      | (board >> width)
      | (board >> 1)
      | (board << 1)
      | (board << width)
      | (board << (width + 1));

    (board, neighbor_candidates & !board)
  }

  /// Constructs a mask of the 6 neighbors of a tile at the given bitvector
  /// index.
  fn neighbors_mask<I: PrimInt>(&self, index: usize) -> I {
    let lesser_neighbors_mask = unsafe { I::from(0x3 | (0x1 << self.width)).unwrap_unchecked() };
    let greater_neighbors_mask = unsafe { I::from(0x2 | (0x3 << self.width)).unwrap_unchecked() };

    let lesser_neighbors = (lesser_neighbors_mask << index) >> (self.width as usize + 1);
    let greater_neighbors = greater_neighbors_mask << index;

    lesser_neighbors | greater_neighbors
  }
}

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

  /// Finds the next move we can make, or `None` if all moves have been found.
  fn next_impl(&mut self) -> Option<Move> {
    let mut neighbor_candidates = self.neighbor_candidates;
    while neighbor_candidates != I::zero() {
      let index = neighbor_candidates.trailing_zeros() as usize;
      neighbor_candidates = neighbor_candidates & (neighbor_candidates - I::one());

      let neighbors_mask: I = self.indexer.neighbors_mask(index);
      if (neighbors_mask & self.board_vec).count_ones() >= 2 {
        self.neighbor_candidates = neighbor_candidates;
        return Some(Move::Phase1Move {
          to: self.indexer.pos_from_index(index as u32),
        });
      }
    }

    // No need to store neighbor_candidates again, since we typically don't
    // call next() again after None is returned.
    None
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

  fn next(&mut self) -> Option<Move> {
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
  fn next(&mut self) -> Option<Move> {
    self.next_impl()
  }
}

enum ImplContainer {
  /// We use this repr when the board bitvec is small enough to fit in a u64,
  /// including a 1-tile padding around the perimeter. This is much faster to
  /// operate on than a u128.
  Small(Impl<u64>),
  /// We need to support any board size. The largest possible board is 8 x 8
  /// (see test_worst_case below), which, with a 1-tile padding, requires 81
  /// bits for the board bitvec.
  Large(Box<Impl<u128>>),
}

/// The phase 1 move generator, where not all pawns have been placed and a move
/// consists of adding a new pawn to the board adjacent to at least 2 other
/// pawns.
pub struct P1MoveGenerator<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> {
  impl_container: ImplContainer,
}

impl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize>
  P1MoveGenerator<N, N2, ADJ_CNT_SIZE>
{
  #[cfg(test)]
  fn indexer(&self) -> &BoardVecIndexer {
    match &self.impl_container {
      ImplContainer::Small(impl_) => &impl_.indexer,
      ImplContainer::Large(impl_) => &impl_.indexer,
    }
  }
}

impl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize>
  P1MoveGenerator<N, N2, ADJ_CNT_SIZE>
{
  pub fn new(onoro: &OnoroImpl<N, N2, ADJ_CNT_SIZE>) -> Self {
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
        impl_container: ImplContainer::Small(Impl::<u64>::new(
          basis,
          corner,
          width,
          onoro.pawn_poses(),
        )),
      }
    } else {
      P1MoveGenerator {
        impl_container: ImplContainer::Large(
          Impl::<u128>::new(basis, corner, width, onoro.pawn_poses()).into(),
        ),
      }
    }
  }
}

impl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> GameMoveIterator
  for P1MoveGenerator<N, N2, ADJ_CNT_SIZE>
{
  type Item = Move;
  type Game = OnoroImpl<N, N2, ADJ_CNT_SIZE>;

  fn next(&mut self, _onoro: &Self::Game) -> Option<Self::Item> {
    match &mut self.impl_container {
      ImplContainer::Small(impl_) => impl_.next(),
      ImplContainer::Large(impl_) => impl_.next(),
    }
  }
}

#[cfg(test)]
mod tests {
  use abstract_game::GameMoveIterator;
  use onoro::{Onoro, OnoroIndex, error::OnoroResult, hex_pos::HexPos, test_util::BOARD_POSITIONS};
  use rstest::rstest;
  use rstest_reuse::{apply, template};

  use crate::{
    FilterNullPackedIdx, Onoro16, PackedIdx,
    p1_move_gen::{
      BoardVecIndexer, DetermineBasisOutput, ImplContainer, P1MoveGenerator, determine_basis,
    },
    util::packed_positions_coord_limits,
  };

  fn get_board_vec<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize>(
    move_gen: &P1MoveGenerator<N, N2, ADJ_CNT_SIZE>,
  ) -> u128 {
    match &move_gen.impl_container {
      ImplContainer::Small(impl_) => impl_.board_vec as u128,
      ImplContainer::Large(impl_) => impl_.board_vec,
    }
  }

  fn get_neighbor_candidates<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize>(
    move_gen: &P1MoveGenerator<N, N2, ADJ_CNT_SIZE>,
  ) -> u128 {
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

  #[test]
  fn test_determine_basis() {
    const N: u32 = 16;
    for y1 in 1..(N - 1) {
      for x1 in 1..(N - 1) {
        let p1 = PackedIdx::new(x1, y1);
        for y2 in 1..(N - 1) {
          for x2 in 1..(N - 1) {
            if x1 == x2 && y1 == y2 {
              continue;
            }
            let p2 = PackedIdx::new(x2, y2);
            if p1.axial_distance(p2) > N - 3 {
              // These two points are farther apart than any two pawns could be.
              continue;
            }

            let mut coord_poses = [PackedIdx::null(); N as usize];
            coord_poses[0] = p1;
            coord_poses[1] = p2;
            let coord_limits = packed_positions_coord_limits(&coord_poses);

            let DetermineBasisOutput {
              basis,
              corner,
              width,
              use_u128,
            } = determine_basis::<{ N as usize }>(coord_limits);
            let indexer = BoardVecIndexer::new(basis, corner, width);

            assert!(indexer.index(p1) > width as usize);
            assert!(
              indexer.index(p1)
                < if use_u128 { u128::BITS } else { u64::BITS } as usize - width as usize
            );
          }
        }
      }
    }
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

  #[test]
  fn test_build_board_vec2() {
    let onoro = Onoro16::default_start();
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
    let worst_case = Onoro16::from_board_string(
      ". B . . . . . . . . . . . .
        W B W B W B W B W B W B W B
         . . . . . . . . . . . . W .",
    )?;

    let move_gen = P1MoveGenerator::new(&worst_case);
    assert_eq!(move_gen.to_iter(&worst_case).count(), 26);

    Ok(())
  }

  #[test]
  fn test_line_y() -> OnoroResult {
    let worst_case = Onoro16::from_board_string(
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

    let move_gen = P1MoveGenerator::new(&worst_case);
    assert_eq!(move_gen.to_iter(&worst_case).count(), 26);

    Ok(())
  }

  #[test]
  fn test_line_xy() -> OnoroResult {
    let worst_case = Onoro16::from_board_string(
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

    let move_gen = P1MoveGenerator::new(&worst_case);
    assert_eq!(move_gen.to_iter(&worst_case).count(), 26);

    Ok(())
  }
}
