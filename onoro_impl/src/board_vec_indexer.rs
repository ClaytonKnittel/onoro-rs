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
#[derive(Clone, Copy, Debug)]
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

  const fn hex_coords(&self, pos: HexPos) -> (u32, u32) {
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

  fn coords(&self, pos: PackedIdx) -> (u32, u32) {
    self.hex_coords(pos.into())
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

  pub fn index_from_coords(&self, (c1, c2): (u32, u32)) -> u32 {
    debug_assert!(c1 < self.width as u32);
    c2 * self.width as u32 + c1
  }

  /// Maps a `PackedIdx` from the Onoro state to an index in the board bitvec.
  pub fn index(&self, pos: PackedIdx) -> u32 {
    self.index_from_coords(self.coords(pos))
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
    let width = self.width as u32;

    let board = pawn_poses
      .iter()
      .filter_null()
      .fold(I::zero(), |board_vec, &pos| {
        let index = self.index(pos);
        debug_assert!(index > width);
        board_vec | (I::one() << index as usize)
      });

    // All neighbors are -(width+1), -width, -1, +1, +width, +(width+1) in
    // index space.
    let width = width as usize;
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
  use std::fmt::Debug;

  use onoro::{OnoroIndex, hex_pos::HexPosOffset};

  use crate::{PackedIdx, util::packed_positions_coord_limits};

  use super::*;

  fn each_index(n: u32) -> impl Iterator<Item = PackedIdx> {
    (1..(n - 1)).flat_map(move |y| (1..(n - 1)).map(move |x| PackedIdx::new(x, y)))
  }

  fn each_corner(n: u32) -> impl Iterator<Item = (PackedIdx, PackedIdx)> {
    each_index(n)
      .flat_map(move |p1| each_index(n).map(move |p2| (p1, p2)))
      .filter(move |&(p1, p2)| p1 != p2 && p1.axial_distance(p2) <= n - 3)
  }

  #[test]
  fn test_determine_basis() {
    const N: u32 = 16;
    for (p1, p2) in each_corner(N) {
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

      let width = width as u32;
      assert!(indexer.index(p1) > width);
      assert!(indexer.index(p1) < if use_u128 { u128::BITS } else { u64::BITS } - width);
      assert!(indexer.index(p2) < if use_u128 { u128::BITS } else { u64::BITS } - width);
    }
  }

  fn min_bb_area_with_padding(p1: PackedIdx, p2: PackedIdx) -> u64 {
    let x1 = p1.x() as i32;
    let y1 = p1.y() as i32;
    let x2 = p2.x() as i32;
    let y2 = p2.y() as i32;
    let dx = (x1 - x2).unsigned_abs() as u64 + 3;
    let dy = (y1 - y2).unsigned_abs() as u64 + 3;
    let dxy = ((y1 - x1) - (y2 - x2)).unsigned_abs() as u64 + 3;
    dx * dy * dxy / dx.max(dy).max(dxy)
  }

  fn check_is_bijection<I: PrimInt + Debug>(
    indexer: &BoardVecIndexer,
    p1: PackedIdx,
    p2: PackedIdx,
  ) {
    let mut board_vec = I::zero();
    let min_bb_area = min_bb_area_with_padding(p1, p2);
    assert_eq!(min_bb_area % indexer.width as u64, 0);

    let xlim = indexer.width as i32;
    let ylim = (min_bb_area / indexer.width as u64) as i32;
    for y in 0..ylim {
      for x in 0..xlim {
        if (x == 0 && y == ylim - 1) || (x == xlim - 1 && y == 0) {
          continue;
        }
        let pos = indexer.corner
          + match indexer.basis {
            Basis::XvY => HexPosOffset::new(x, y),
            Basis::XvXY => HexPosOffset::new(x - y, x),
            Basis::XYvY => HexPosOffset::new(y, y - x),
          };

        let index = indexer.index(pos.into());
        let mask = I::one() << index as usize;
        assert_eq!(board_vec & mask, I::zero());
        board_vec = board_vec | mask;

        assert_eq!(indexer.pos_from_coords((x as u32, y as u32)), pos.into());
        assert_eq!(indexer.pos_from_index(index), pos.into());
      }
    }
  }

  #[test]
  fn test_indexer_bijection() {
    const N: u32 = 16;
    for (p1, p2) in each_corner(N) {
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

      if use_u128 {
        check_is_bijection::<u128>(&indexer, p1, p2);
      } else {
        check_is_bijection::<u64>(&indexer, p1, p2);
      }
    }
  }
}
