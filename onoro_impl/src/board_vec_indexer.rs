use num_traits::PrimInt;
use onoro::hex_pos::HexPos;

use crate::{
  FilterNullPackedIdx, PackedIdx,
  util::{CoordLimits, MinAndMax},
};

/// The choice of basis to use for the minimum bounding parallelogram of the
/// pawns on the board. In the diagrams below, x is the first coordinate axis,
/// and y is the second. A `-` sign in front of the axis label means the arrow
/// is pointing in the negative direction.
#[derive(Clone, Copy)]
pub enum Basis {
  ///```text
  /// +y
  /// Γ
  ///  \
  ///   \
  ///    +---> +x
  ///```
  XvY,
  ///```text
  ///    +x
  ///    7
  ///   /
  ///  /
  /// +---> -y
  ///```
  XvXY,
  ///```text
  /// -x    +y
  /// Γ     7
  ///  \   /
  ///   \ /
  ///    +
  ///```
  XYvY,
}

pub struct DetermineBasisOutput {
  pub basis: Basis,
  pub corner: HexPos,
  pub width: u8,
  pub use_u128: bool,
}

pub fn determine_basis<const N: usize>(coord_limits: CoordLimits) -> DetermineBasisOutput {
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
pub struct BoardVecIndexer {
  /// The basis that is used for indexing positions.
  basis: Basis,
  /// The corner of the minimal bounding box containing all placed pawns, with
  /// a 1-tile perimeter of empty tiles.
  corner: HexPos,
  /// The width of this minimal bounding box.
  width: u8,
}

impl BoardVecIndexer {
  pub fn new(basis: Basis, corner: HexPos, width: u8) -> Self {
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
  pub fn index(&self, pos: PackedIdx) -> usize {
    let (c1, c2) = self.coords(pos);
    debug_assert!(c1 < self.width as u32);
    c2 as usize * self.width as usize + c1 as usize
  }

  /// Maps an index from the board bitvec to a `PackedIdx` in the Onoro state.
  pub fn pos_from_index(&self, index: u32) -> PackedIdx {
    debug_assert!((3..=16).contains(&self.width));
    let c1 = index % self.width as u32;
    let c2 = index / self.width as u32;
    self.pos_from_coords((c1, c2))
  }

  /// Builds both the board bitvec and neighbor candidates. The board bitvec
  /// has a 1 in each index corresponding to an occupied tile, and the neighbor
  /// candidates have a 1 in each index corresponding to an empty neighbor of
  /// any pawn.
  pub fn build_bitvecs<I: PrimInt>(&self, pawn_poses: &[PackedIdx]) -> (I, I) {
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
  pub fn neighbors_mask<I: PrimInt>(&self, index: usize) -> I {
    let lesser_neighbors_mask = unsafe { I::from(0x3 | (0x1 << self.width)).unwrap_unchecked() };
    let greater_neighbors_mask = unsafe { I::from(0x2 | (0x3 << self.width)).unwrap_unchecked() };

    let lesser_neighbors = (lesser_neighbors_mask << index) >> (self.width as usize + 1);
    let greater_neighbors = greater_neighbors_mask << index;

    lesser_neighbors | greater_neighbors
  }
}

#[cfg(test)]
mod tests {
  use onoro::OnoroIndex;

  use crate::{PackedIdx, util::packed_positions_coord_limits};

  use super::*;

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
            assert!(
              indexer.index(p2)
                < if use_u128 { u128::BITS } else { u64::BITS } as usize - width as usize
            );
          }
        }
      }
    }
  }
}
