use onoro::{
  Onoro,
  groups::{D6, SymmetryClass},
  hex_pos::HexPosOffset,
};

use crate::{
  OnoroImpl,
  packed_hex_pos::PackedHexPos,
  util::{likely, max_u32, min_u32, unreachable},
};

/// Describes the layout of the game state, and provides enough information to
/// canonicalize the state for hash computation.
#[derive(Clone, Copy, Debug)]
pub struct BoardSymmetryState {
  /// The group operation to perform on the board before calculating the hash.
  /// This is used to align board states on all symmetry axes which the board
  /// isn't possibly symmetric about itself.
  pub op: D6,

  /// The symmetry class this board state belongs in, which depends on where the
  /// center of mass lies. If the location of the center of mass is symmetric to
  /// itself under some group operations, then those symmetries must be checked
  /// when looking up in the hash table.
  pub symm_class: SymmetryClass,

  /// The offset to apply when calculating the integer-coordinate, symmetry
  /// invariant "center of mass"
  center_offset: COMOffset,
}

impl BoardSymmetryState {
  const fn blank() -> Self {
    Self {
      op: D6::const_identity(),
      symm_class: SymmetryClass::C,
      center_offset: COMOffset::X0Y0,
    }
  }

  pub const fn center_offset(&self) -> HexPosOffset {
    match self.center_offset {
      COMOffset::X0Y0 => HexPosOffset::new(0, 0),
      COMOffset::X1Y0 => HexPosOffset::new(1, 0),
      COMOffset::X0Y1 => HexPosOffset::new(0, 1),
      COMOffset::X1Y1 => HexPosOffset::new(1, 1),
    }
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum COMOffset {
  /// Offset by (0, 0)
  X0Y0,
  /// Offset by (1, 0)
  X1Y0,
  /// Offset by (0, 1)
  X0Y1,
  /// Offset by (1, 1)
  X1Y1,
}

/// Mapping to offsets to apply when calculating the integer-coordinate,
/// symmetry invariant "center of mass"
///
/// Mapping from regions of the tiling unit square to the offset from the
/// coordinate in the bottom left corner of the unit square to the center of the
/// hex tile this region is a part of, indexed by the D6 symmetry op associated
/// with the region. See the description of genSymmStateTable() for this mapping
/// from symmetry op to region.
const fn board_symm_state_op_to_com_offset(op: D6) -> COMOffset {
  match op {
    D6::Rot(0) => COMOffset::X0Y0,
    D6::Rot(1) => COMOffset::X0Y1,
    D6::Rot(2) => COMOffset::X1Y1,
    D6::Rot(3) => COMOffset::X1Y1,
    D6::Rot(4) => COMOffset::X1Y0,
    D6::Rot(5) => COMOffset::X0Y0,
    D6::Rfl(0) => COMOffset::X0Y1,
    D6::Rfl(1) => COMOffset::X0Y0,
    D6::Rfl(2) => COMOffset::X0Y0,
    D6::Rfl(3) => COMOffset::X1Y0,
    D6::Rfl(4) => COMOffset::X1Y1,
    D6::Rfl(5) => COMOffset::X1Y1,
    _ => unreachable(),
  }
}

/// Returns the symmetry state operation corresponding to the point (x, y) in
/// the unit square scaled by `n_pawns`.
///
/// `n_pawns` is the number of pawns currently in play.
///
/// `(x, y)` are elements of `{0, 1, ... n_pawns-1} x {0, 1, ... n_pawns-1}`
#[allow(clippy::collapsible_else_if)]
const fn symm_state_op(x: u32, y: u32, n_pawns: u32) -> D6 {
  // (x2, y2) is (x, y) folded across the line y = x
  let x2 = max_u32(x, y);
  let y2 = min_u32(x, y);

  // (x3, y3) is (x2, y2) folded across the line y = n_pawns - x
  let x3 = min_u32(x2, n_pawns - y2);
  let y3 = min_u32(y2, n_pawns - x2);

  let c1 = y < x;
  let c2 = x2 + y2 < n_pawns;
  let c3a = y3 + n_pawns <= 2 * x3;
  let c3b = 2 * y3 <= x3;

  if c1 {
    if c2 {
      if c3a {
        D6::Rfl(3)
      } else if c3b {
        D6::Rot(0)
      } else {
        D6::Rfl(1)
      }
    } else {
      if c3a {
        D6::Rot(4)
      } else if c3b {
        D6::Rfl(5)
      } else {
        D6::Rot(2)
      }
    }
  } else {
    if c2 {
      if c3a {
        D6::Rot(1)
      } else if c3b {
        D6::Rfl(2)
      } else {
        D6::Rot(5)
      }
    } else {
      if c3a {
        D6::Rfl(0)
      } else if c3b {
        D6::Rot(3)
      } else {
        D6::Rfl(4)
      }
    }
  }
}

/// Returns the symmetry class corresponding to the point (x, y) in the unit
/// square scaled by `n_pawns`.
///
/// `n_pawns` is the number of pawns currently in play.
///
/// (x, y) are elements of {0, 1, ... n_pawns-1} x {0, 1, ... n_pawns-1}
const fn symm_state_class(x: u32, y: u32, n_pawns: u32) -> SymmetryClass {
  // (x2, y2) is (x, y) folded across the line y = x
  let x2 = max_u32(x, y);
  let y2 = min_u32(x, y);

  // (x3, y3) is (x2, y2) folded across the line y = n_pawns - x
  let x3 = min_u32(x2, n_pawns - y2);
  let y3 = min_u32(y2, n_pawns - x2);

  // Calculate the symmetry class of this position.
  if x == 0 && y == 0 {
    SymmetryClass::C
  } else if 3 * x2 == 2 * n_pawns && 3 * y2 == n_pawns {
    SymmetryClass::V
  } else if 2 * x2 == n_pawns && (y2 == 0 || 2 * y2 == n_pawns) {
    SymmetryClass::E
  } else if 2 * y3 == x3 || (x2 + y2 == n_pawns && 3 * y2 < n_pawns) {
    SymmetryClass::CV
  } else if x2 == y2 || y2 == 0 {
    SymmetryClass::CE
  } else if y3 + n_pawns == 2 * x3 || (x2 + y2 == n_pawns && 3 * y2 > n_pawns) {
    SymmetryClass::EV
  } else {
    SymmetryClass::Trivial
  }
}

/// The purpose of the symmetry table is to provide a quick way to canonicalize
/// boards when computing and checking for symmetries.
const fn gen_symm_state_table<const N: usize>() -> [[BoardSymmetryState; N]; N] {
  // Populate the table with dummy values for `BoardSymmetryState`, which will
  // be overwritten below. This is because const initialization of arrays is
  // clunky in rust.
  let mut table: [[BoardSymmetryState; N]; N] = [[BoardSymmetryState::blank(); N]; N];

  let mut y = 0;
  while y < N {
    let mut x = 0;
    while x < N {
      let op = symm_state_op(x as u32, y as u32, N as u32);
      let center_offset = board_symm_state_op_to_com_offset(op);
      table[y][x] = BoardSymmetryState {
        op,
        symm_class: symm_state_class(x as u32, y as u32, N as u32),
        center_offset,
      };

      x += 1;
    }

    y += 1;
  }

  table
}

fn compute_board_symm_state(sum_of_mass: PackedHexPos, pawns_in_play: u32) -> BoardSymmetryState {
  let x = sum_of_mass.x() as u32 % pawns_in_play;
  let y = sum_of_mass.y() as u32 % pawns_in_play;

  let op = symm_state_op(x, y, pawns_in_play);
  let symm_class = symm_state_class(x, y, pawns_in_play);
  let center_offset = board_symm_state_op_to_com_offset(op);

  BoardSymmetryState {
    op,
    symm_class,
    center_offset,
  }
}

/// The purpose of the symmetry state is to provide a quick way to canonicalize
/// boards when computing and checking for symmetries. Since the center of mass
/// transforms the same as tiles under symmetry operations, we can use the
/// position of the center of mass to prune the list of possible layouts of
/// boards symmetric to this one. For example, if the center of mass does not
/// lie on any symmetry lines, then if we orient the center of mass in the same
/// segment of the origin hexagon, all other game boards which are symmetric to
/// this one will have oriented their center of masses to the same position,
/// meaning the coordinates of all pawns in both boards will be the same.
///
/// We choose to place the center of mass within the triangle extending from the
/// center of the origin hex to the center of its right edge (+x), and up to its
/// top-right vertex. This triangle has coordinates (0, 0), (1/2, 0), (2/3, 1/3)
/// in HexPos space.
///
/// A unit square centered at (1/2, 1/2) in HexPos space is a possible unit tile
/// for the hexagonal grid (keep in mind that the hexagons are not regular
/// hexagons in HexPos space). Pictured below is a mapping from regions on this
/// unit square to D6 operations (about the origin) to transform the points
/// within the corresponding region to a point within the designated triangle
/// defined above.
///
/// ```text
/// +-------------------------------+
/// |`            /    r3     _ _ / |
/// |  `    s0   /       _ _    /   |
/// |    `      /   _ _       /     |
/// |  r1  `   / _          /       |
/// |     _ _`v     s4    /        /|
/// |  _     / `        /         / |
/// e       /    `    /     r2   /  |
/// |  s2  /       `e           /   |
/// |     /  r5   /  `         / s5 |
/// |    /      /      `      /    -|
/// |   /     /    s1    `   /- -   |
/// |  /    /            - `v    r4 |
/// | /   /         - -    / `      |
/// |/  /      - -        /    `    |
/// | /   - -      r0    /  s3   `  |
/// +-------------------e-----------+
/// ```
///
/// This image is composed of lines:
/// ```text
///  y = 2x
///  y = 1/2(x + 1)
///  y = x
///  y = 1 - x
///  y = 1/2x
///  y = 2x - 1
/// ```
///
/// These lines divie the unit square into 12 equally-sized regions in cartesian
/// space, and listed in each region is the D6 group operation to map that
/// region to the designated triangle.
///
/// Since the lines given above are the symmetry lines of the hexagonal grid, we
/// can use them to determine which symmetry group the board state belongs in.
///
/// Let (x, y) = (n_pawns * (com.x % 1), n_pawns * (com.y % 1)) be the folded
/// center of mass within the unit square, scaled by n_pawns in play. Note that
/// x and y are integers.
///
/// Let (x2, y2) = (max(x, y), min(x, y)) be (x, y) folded across the symmetry
/// line y = x. Note that the diagram above is also symmetryc about y = x, save
/// for the group operations in the regions.
///
/// - C is the symmetry group D6 about the origin, which is only possible when
///   the center of mass lies on the origin, so (x, y) = (0, 0).
/// - V is the symmetry group D3 about a vertex, which are labeled as 'v' in the
///   diagram. These are the points (2/3 n_pawns, 1/3 n_pawns) and (1/3
///   n_pawns, 2/3 n_pawns), or (x2, y2) = (2/3 n_pawns, 1/3 n_pawns).
/// - E is the symmetry group K4 about the center of an edge, which are labeled
///   as 'e' in the diagram. These are the points (1/2 n_pawns, 0), (1/2
///   n_pawns, 1/2 n_pawns), and (0, 1/2 n_pawns), or (x2, y2) = (1/2 n_pawns,
///   0) or (1/2 n_pawns, 1/2 n_pawns).
/// - CV is the symmetry group C2 about a line passing through the center of the
///   origin hex and one of its vertices.
/// - CE is the symmetry group C2 about a line passing through the center of the
///   origin hex and the center of one of its edges.
/// - EV is the symmetry group C2 about a line tangent to one of the edges of
///   the origin hex.
/// - TRIVIAL is a group with no symmetries other than the identity, so all
///   board states with center of masses which don't lie on any symmetry lines
///   are part of this group.
///
/// In the case that the center of mass lies on a symmetry line/point, it is
/// classified into one of 6 symmetry groups above. These symmetry groups are
/// subgroups of D6, and are uniquely defined by the remaining symmetries after
/// canonicalizing the symmetry line/point by the operations given in the
/// graphic. As an example, the e's on the graphic will all be mapped to the e
/// in the bottom center of the graphic, but there are 4 possible orientations
/// of the board with this constraint applied. The group of these 4 orientations
/// is K4 (C2 + C2), which is precisely the symmetries of the infinite hexagonal
/// grid centered at the midpoint of an edge (nix translation). This also means
/// that it does not matter which of the 4 group operations we choose to apply
/// to the game state when canonicalizing if the center of mass lies on an e,
/// since they are symmetries of each other in this K4 group.
#[inline(always)]
pub fn board_symm_state<const N: usize>(onoro: &OnoroImpl<N>) -> BoardSymmetryState {
  let sum_of_mass = onoro.sum_of_mass();
  let pawns_in_play = onoro.pawns_in_play();

  if const { N == 16 } && likely(pawns_in_play == N as u32) {
    const SYMM_TABLE_16: [[BoardSymmetryState; 16]; 16] = gen_symm_state_table::<16>();

    let x = sum_of_mass.x() as u32 % N as u32;
    let y = sum_of_mass.y() as u32 % N as u32;
    return SYMM_TABLE_16[y as usize][x as usize];
  }

  compute_board_symm_state(sum_of_mass, pawns_in_play)
}

#[cfg(test)]
mod tests {
  use std::collections::HashSet;

  use algebra::semigroup::Semigroup;
  use itertools::Itertools;
  use onoro::{
    groups::{D6, SymmetryClass},
    hex_pos::HexPosOffset,
  };

  use crate::canonicalize::{
    COMOffset, board_symm_state_op_to_com_offset, symm_state_class, symm_state_op,
  };

  struct PawnsConfig {
    n_pawns: u32,
    x: u32,
    y: u32,
  }

  fn all_pawn_counts_and_coordinates() -> impl Iterator<Item = PawnsConfig> {
    (1..=16).flat_map(|n_pawns| {
      (0..n_pawns)
        .cartesian_product(0..n_pawns)
        .map(move |(x, y)| PawnsConfig { n_pawns, x, y })
    })
  }

  /// ```text
  /// +--------------E----------------F
  /// |`            /           . .   |
  /// |  `         /       . .        |
  /// |    `      /   . .             |
  /// |      `   / .                  |
  /// |     _ _`v                    .|
  /// |  _     . \                  . |
  /// D       .    \               .  |
  /// |      .       \            .   |
  /// |     .          \         .    |
  /// |    .             \      .    -C
  /// |   .                \   .- -   |
  /// |  .                 . `v       |
  /// | .             . .    / `      |
  /// |.         . .        /    `    |
  /// |     . .            /       `  |
  /// A-------------------B-----------+
  /// ```
  fn com_to_origin_offset(x: u32, y: u32, n_pawns: u32) -> COMOffset {
    debug_assert!(x < n_pawns);
    debug_assert!(y < n_pawns);

    let bottom_left = x < n_pawns - y;
    let below_a_e = 2 * x > y;
    let below_d_f = x + n_pawns > 2 * y;
    let above_a_c = 2 * y > x;
    let above_b_f = y + n_pawns > 2 * x;

    if bottom_left && below_d_f && above_b_f {
      COMOffset::X0Y0
    } else if !below_a_e && !below_d_f {
      COMOffset::X0Y1
    } else if !above_a_c && !above_b_f {
      COMOffset::X1Y0
    } else {
      debug_assert!(!bottom_left && below_a_e && above_a_c);
      COMOffset::X1Y1
    }
  }

  #[test]
  fn test_board_symm_state_op_to_com_offset() {
    for PawnsConfig { n_pawns, y, x } in all_pawn_counts_and_coordinates() {
      assert_eq!(
        board_symm_state_op_to_com_offset(symm_state_op(x, y, n_pawns)),
        com_to_origin_offset(x, y, n_pawns),
        "With {n_pawns} pawns at ({x}, {y})"
      );
    }
  }

  fn deduce_symmetry_class(x: u32, y: u32, n_pawns: u32) -> SymmetryClass {
    // Try all symmetry operations and classify by how many symmetries are produced.
    let hex_pos = HexPosOffset::new(x as i32, y as i32);
    let rotate = |pos: HexPosOffset, op: D6| -> HexPosOffset {
      let pos = pos.apply_d6_c(&op);
      HexPosOffset::new(
        pos.x().rem_euclid(n_pawns as i32),
        pos.y().rem_euclid(n_pawns as i32),
      )
    };
    let symmetries: HashSet<_> = D6::for_each().map(|op| rotate(hex_pos, op)).collect();

    match symmetries.len() {
      1 => SymmetryClass::C,
      2 => SymmetryClass::V,
      3 => SymmetryClass::E,
      6 => {
        if rotate(hex_pos, D6::Rfl(0)) == hex_pos
          || rotate(hex_pos, D6::Rfl(2)) == hex_pos
          || rotate(hex_pos, D6::Rfl(4)) == hex_pos
        {
          SymmetryClass::CE
        } else {
          let middle_third_x = 3 * x > n_pawns && 3 * x < 2 * n_pawns;
          let middle_third_y = 3 * y > n_pawns && 3 * y < 2 * n_pawns;
          if (rotate(hex_pos, D6::Rfl(1)) == hex_pos && middle_third_y)
            || (rotate(hex_pos, D6::Rfl(3)) == hex_pos && middle_third_x)
            || (rotate(hex_pos, D6::Rfl(5)) == hex_pos && middle_third_x)
          {
            SymmetryClass::EV
          } else {
            SymmetryClass::CV
          }
        }
      }
      12 => SymmetryClass::Trivial,
      _ => unreachable!(),
    }
  }

  #[test]
  fn test_symm_state_class() {
    for PawnsConfig { n_pawns, y, x } in all_pawn_counts_and_coordinates() {
      assert_eq!(
        symm_state_class(x, y, n_pawns),
        deduce_symmetry_class(x, y, n_pawns),
        "With {n_pawns} pawns at ({x}, {y})"
      );
    }
  }
}
