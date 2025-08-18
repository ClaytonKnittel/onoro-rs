use abstract_game::GameMoveIterator;
use onoro::{Onoro, OnoroIndex, PawnColor};

use crate::{
  Move, OnoroImpl, PackedIdx,
  p1_move_gen::P1MoveGenerator,
  util::{IterOnes, unreachable},
};

#[derive(Clone, Copy, Debug)]
enum PawnConnectedMobility {
  /// The pawn is free to move anywhere that is available. This means this pawn
  /// is not an articulation point.
  Free,
  /// The pawn is a cutting point and currently connects two disjoint groups.
  ///
  /// When considering positions to move this pawn to, in order to maintain the
  /// connectedness of the game, one adjacent pawn at the new location must
  /// have discovery times between this pawn's discovery time and `exit_time`
  /// (exclusive), and another must be outside this range.
  CuttingPoint {
    /// The time we returned from exploring the subtree of this pawn.
    exit_time: u32,
  },
  /// The pawn connects 3 disjoint groups and is thus immobile.
  ///
  /// Any pawn that connects 3 disjoint groups is immobile, as there is no
  /// configuration of pawns with two points that join 3 disjoint groups.
  ///
  /// Here is an example which uses 20 pawns, moving the pawn at `*` to `_`:
  /// ```text
  /// . . P P P P P
  ///  . P . . . P .
  ///   P . P P _ . .
  ///    P . P . P . .
  ///     P . * P P . .
  ///      P P . . . . .
  ///       P . . . . . .
  /// ```
  Immobile,
}

impl Default for PawnConnectedMobility {
  fn default() -> Self {
    Self::Free
  }
}

#[derive(Clone, Copy, Default)]
struct PawnMeta {
  /// The discovery index of this pawn when doing the depth-first exploration
  /// of the pawn graph.
  discovery_time: u32,

  /// A mask of the neighbors of this pawn in the pawn metadata/pawn_poses
  /// lists. Each bit corresponds to an index in these lists in the same order.
  neighbor_index_mask: u16,

  // Below only relevant to current player's pawns:
  /// The time we have returned from exploring the subtree of this pawn.
  ///
  /// Note that this can happen at most twice, since we can have at most three
  /// disconnected coming out of a single tile. However, there is no legal move
  /// which disconnects the board into 3 groups and reconnects them in another
  /// location with 16 or fewer total pawns.
  connected_mobility: PawnConnectedMobility,
}

impl PawnMeta {
  fn has_two_neighbors(&self) -> bool {
    self.neighbor_index_mask.count_ones() == 2
  }
}

pub struct P2MoveGenerator<const N: usize> {
  pawn_meta: [PawnMeta; N],
  p1_move_gen: P1MoveGenerator<N>,
  cur_tile: PackedIdx,
  neighbor_mask: u16,
  pawn_index: usize,
}

impl<const N: usize> P2MoveGenerator<N> {
  pub fn new(onoro: &OnoroImpl<N>) -> Self {
    debug_assert!(!onoro.in_phase1());
    Self::from_pawn_poses(onoro.pawn_poses(), matches!(onoro.turn(), PawnColor::Black))
  }

  pub fn from_pawn_poses(pawn_poses: &[PackedIdx; N], black_turn: bool) -> Self {
    let p1_move_gen = P1MoveGenerator::from_pawn_poses(pawn_poses);
    let pawn_meta = Self::build_pawn_meta(pawn_poses, &p1_move_gen);

    Self {
      pawn_meta,
      p1_move_gen,
      pawn_index: N - 2 + !black_turn as usize,
      neighbor_mask: 0,
      cur_tile: PackedIdx::null(),
    }
  }

  fn neighbors(
    pawn_index: usize,
    pawn_poses: &[PackedIdx; N],
    p1_move_gen: &P1MoveGenerator<N>,
  ) -> impl Iterator<Item = usize> {
    let indexer = p1_move_gen.indexer();

    let pawn_index = indexer.index(pawn_poses[pawn_index]);
    p1_move_gen.neighbors(pawn_index).map(|neighbor_index| {
      let neighbor_pos = indexer.pos_from_index(neighbor_index);
      pawn_poses
        .iter()
        .enumerate()
        .find(|&(_, &pos)| pos == neighbor_pos)
        .unwrap()
        .0
    })
  }

  fn next_move_with_neighbors(&mut self, pawn_poses: &[PackedIdx; N]) -> Option<(PackedIdx, u16)> {
    let (pos, neighbors) = self.p1_move_gen.next_move_pos_with_neighbors()?;
    Some((
      pos,
      neighbors.fold(0, |neighbor_mask, neighbor_index| {
        let neighbor_pos = self.p1_move_gen.indexer().pos_from_index(neighbor_index);
        let neighbor_index = pawn_poses
          .iter()
          .enumerate()
          .find(|&(_, &pos)| pos == neighbor_pos)
          .unwrap()
          .0;
        neighbor_mask | (1 << neighbor_index)
      }),
    ))
  }

  fn recursor(
    pawn_index: usize,
    parent_index: usize,
    pawn_meta: &mut [PawnMeta; N],
    ecas: &mut [u32; N],
    time: &mut u32,
    pawn_poses: &[PackedIdx; N],
    p1_move_gen: &P1MoveGenerator<N>,
  ) {
    let meta = &mut pawn_meta[pawn_index];
    meta.discovery_time = *time;
    ecas[pawn_index] = *time;
    *time += 1;

    for neighbor_index in Self::neighbors(pawn_index, pawn_poses, p1_move_gen) {
      pawn_meta[pawn_index].neighbor_index_mask |= 1 << neighbor_index;
      if neighbor_index == parent_index {
        continue;
      }

      let neighbor_t = pawn_meta[neighbor_index].discovery_time;
      if neighbor_t != 0 {
        ecas[pawn_index] = ecas[pawn_index].min(neighbor_t);
        continue;
      }

      Self::recursor(
        neighbor_index,
        pawn_index,
        pawn_meta,
        ecas,
        time,
        pawn_poses,
        p1_move_gen,
      );

      let neighbor_eca = ecas[neighbor_index];
      ecas[pawn_index] = ecas[pawn_index].min(neighbor_eca);

      let meta = &mut pawn_meta[pawn_index];
      if neighbor_eca >= meta.discovery_time {
        meta.connected_mobility = match meta.connected_mobility {
          PawnConnectedMobility::Free => PawnConnectedMobility::CuttingPoint { exit_time: *time },
          PawnConnectedMobility::CuttingPoint { .. } => PawnConnectedMobility::Immobile,
          PawnConnectedMobility::Immobile => unreachable(),
        }
      }
    }
  }

  fn build_pawn_meta(
    pawn_poses: &[PackedIdx; N],
    p1_move_gen: &P1MoveGenerator<N>,
  ) -> [PawnMeta; N] {
    let mut pawn_meta = [PawnMeta::default(); N];
    let mut ecas = [0u32; N];
    let mut time = 1;
    pawn_meta[0].discovery_time = time;
    ecas[0] = time;
    time += 1;

    let mut previous_exit_time = 0;
    let mut connect_mobility = None;
    for neighbor_index in Self::neighbors(0, pawn_poses, p1_move_gen) {
      pawn_meta[0].neighbor_index_mask |= 1 << neighbor_index;
      if pawn_meta[neighbor_index].discovery_time != 0 {
        continue;
      }

      Self::recursor(
        neighbor_index,
        0,
        &mut pawn_meta,
        &mut ecas,
        &mut time,
        pawn_poses,
        p1_move_gen,
      );

      connect_mobility = Some(match connect_mobility {
        None => PawnConnectedMobility::Free,
        Some(PawnConnectedMobility::Free) => PawnConnectedMobility::CuttingPoint {
          exit_time: previous_exit_time,
        },
        Some(PawnConnectedMobility::CuttingPoint { .. }) => PawnConnectedMobility::Immobile,
        Some(PawnConnectedMobility::Immobile) => unreachable(),
      });

      previous_exit_time = time;
    }

    pawn_meta[0].connected_mobility = connect_mobility.unwrap_or_default();

    pawn_meta
  }

  fn is_valid_move(&self, onoro: &OnoroImpl<N>) -> bool {
    let dst_neighbors = self.neighbor_mask & !(1 << self.pawn_index);

    let meta = self.pawn_meta[self.pawn_index];
    // Check that board connectedness is satisfied.
    match meta.connected_mobility {
      PawnConnectedMobility::Free => {}
      PawnConnectedMobility::CuttingPoint { exit_time } => {
        let mut contains_subtree = false;
        let mut contains_supertree = false;
        for neighbor_index in dst_neighbors.iter_ones() {
          let neighbor_meta = self.pawn_meta[neighbor_index as usize];
          if (meta.discovery_time..exit_time).contains(&neighbor_meta.discovery_time) {
            contains_subtree = true;
          } else {
            contains_supertree = true;
          }
        }

        if !contains_subtree || !contains_supertree {
          return false;
        }
      }
      PawnConnectedMobility::Immobile => return false,
    }

    // Check that this pawn has enough neighbors to move here after excluding
    // itself from the neighbors list.
    if dst_neighbors.count_ones() < 2 {
      return false;
    }

    // Check that all dangling neighbors are satisfied.
    for neighbor_index in meta.neighbor_index_mask.iter_ones() {
      let neighbor_pos = onoro.pawn_poses()[neighbor_index as usize];
      let neighbor_meta = self.pawn_meta[neighbor_index as usize];
      if neighbor_meta.has_two_neighbors() && !neighbor_pos.adjacent(self.cur_tile) {
        return false;
      }
    }

    true
  }
}

impl<const N: usize> GameMoveIterator for P2MoveGenerator<N> {
  type Item = Move;
  type Game = OnoroImpl<N>;

  fn next(&mut self, onoro: &Self::Game) -> Option<Self::Item> {
    loop {
      if self.pawn_index >= N - 2 {
        let (pos, neighbor_mask) = self.next_move_with_neighbors(onoro.pawn_poses())?;
        self.cur_tile = pos;
        self.neighbor_mask = neighbor_mask;
        self.pawn_index -= N - 2;
      } else {
        self.pawn_index += 2;
      }

      if self.is_valid_move(onoro) {
        break;
      }
    }

    Some(Move::Phase2Move {
      to: self.cur_tile,
      from_idx: self.pawn_index as u32,
    })
  }
}

#[cfg(test)]
mod tests {
  use std::collections::{HashMap, HashSet};

  use abstract_game::GameMoveIterator;
  use googletest::{gtest, prelude::*};
  use itertools::Itertools;
  use onoro::{
    Onoro, OnoroIndex,
    error::OnoroResult,
    hex_pos::{HexPos, HexPosOffset},
  };
  use rstest::rstest;
  use rstest_reuse::{apply, template};

  use crate::{
    Move, Onoro8, Onoro16, OnoroImpl, PackedIdx,
    p2_move_gen::{P2MoveGenerator, PawnConnectedMobility},
    util::IterOnes,
  };

  struct Meta {
    discovery_time: i32,
    earliest_connected_ancestor: i32,
    is_cut: bool,
  }
  impl Meta {
    fn new() -> Self {
      Self {
        discovery_time: -1,
        earliest_connected_ancestor: -1,
        is_cut: false,
      }
    }
  }

  fn recursor(pos: HexPos, parent: HexPos, time: &mut u32, poses: &mut HashMap<HexPos, Meta>) {
    let meta = poses.get_mut(&pos).unwrap();
    meta.discovery_time = *time as i32;
    meta.earliest_connected_ancestor = *time as i32;
    *time += 1;

    for neighbor in pos.each_neighbor().filter(|&pos| pos != parent) {
      let Some(neighbor_meta) = poses.get_mut(&neighbor) else {
        continue;
      };
      let neighbor_t = neighbor_meta.discovery_time;

      if neighbor_t != -1 {
        let meta = poses.get_mut(&pos).unwrap();
        meta.earliest_connected_ancestor = meta.earliest_connected_ancestor.min(neighbor_t);
        continue;
      }

      recursor(neighbor, pos, time, poses);

      let neighbor_meta = poses.get_mut(&neighbor).unwrap();
      let neighbor_eca = neighbor_meta.earliest_connected_ancestor;

      let meta = poses.get_mut(&pos).unwrap();
      meta.earliest_connected_ancestor = meta.earliest_connected_ancestor.min(neighbor_eca);

      if neighbor_eca >= meta.discovery_time {
        meta.is_cut = true;
      }
    }
  }

  fn find_articulation_points_simple<const N: usize>(
    pawn_poses: &[PackedIdx; N],
  ) -> Vec<PackedIdx> {
    let mut poses: HashMap<_, _> = pawn_poses
      .iter()
      .map(|&pos| (HexPos::from(pos), Meta::new()))
      .collect();

    let pos: HexPos = pawn_poses[0].into();
    let meta = poses.get_mut(&pos).unwrap();
    meta.discovery_time = 0;
    meta.earliest_connected_ancestor = 0;
    let mut time = 1;

    #[allow(clippy::filter_map_bool_then)]
    let neighbor_count = pos
      .each_neighbor()
      .filter_map(|neighbor| {
        poses
          .get(&neighbor)
          .is_some_and(|meta| meta.discovery_time == -1)
          .then(|| recursor(neighbor, pos, &mut time, &mut poses))
      })
      .count();

    (neighbor_count > 1)
      .then_some(pos.into())
      .into_iter()
      .chain(
        poses
          .into_iter()
          .filter_map(|(pos, meta)| meta.is_cut.then_some(pos.into())),
      )
      .collect()
  }

  fn find_articulation_points<const N: usize>(pawn_poses: &[PackedIdx; N]) -> Vec<PackedIdx> {
    let move_gen = P2MoveGenerator::from_pawn_poses(pawn_poses, true);
    move_gen
      .pawn_meta
      .into_iter()
      .enumerate()
      .filter_map(|(idx, meta)| {
        (!matches!(meta.connected_mobility, PawnConnectedMobility::Free)).then_some(pawn_poses[idx])
      })
      .collect()
  }

  fn find_immobile_points<const N: usize>(pawn_poses: &[PackedIdx; N]) -> Vec<PackedIdx> {
    let move_gen = P2MoveGenerator::from_pawn_poses(pawn_poses, true);
    move_gen
      .pawn_meta
      .into_iter()
      .enumerate()
      .filter_map(|(idx, meta)| {
        matches!(meta.connected_mobility, PawnConnectedMobility::Immobile)
          .then_some(pawn_poses[idx])
      })
      .collect()
  }

  #[template]
  #[rstest]
  fn test_find_articulation_points<const N: usize>(
    #[values(find_articulation_points, find_articulation_points_simple)]
    find_articulation_points: impl FnOnce(&[PackedIdx; N]) -> Vec<PackedIdx>,
  ) {
  }

  #[apply(test_find_articulation_points)]
  #[gtest]
  fn test_no_articulation_points(
    find_articulation_points: impl FnOnce(&[PackedIdx; 3]) -> Vec<PackedIdx>,
  ) {
    let poses = [
      PackedIdx::new(3, 3),
      PackedIdx::new(4, 3),
      PackedIdx::new(4, 4),
    ];

    expect_that!(find_articulation_points(&poses), is_empty());
  }

  #[apply(test_find_articulation_points)]
  #[gtest]
  fn test_one_articulation_point(
    find_articulation_points: impl FnOnce(&[PackedIdx; 5]) -> Vec<PackedIdx>,
  ) {
    let poses = [
      PackedIdx::new(3, 3),
      PackedIdx::new(4, 3),
      PackedIdx::new(3, 2),
      PackedIdx::new(3, 4),
      PackedIdx::new(2, 3),
    ];

    expect_that!(
      find_articulation_points(&poses),
      unordered_elements_are![&PackedIdx::new(3, 3)]
    );
  }

  #[apply(test_find_articulation_points)]
  #[gtest]
  fn test_articulation_points_fidget_spinner(
    find_articulation_points: impl FnOnce(&[PackedIdx; 10]) -> Vec<PackedIdx>,
  ) {
    let poses = [
      // Bottom left blade
      PackedIdx::new(4, 4),
      PackedIdx::new(3, 3),
      PackedIdx::new(4, 3),
      // Center
      PackedIdx::new(5, 5),
      // Right blade
      PackedIdx::new(6, 5),
      PackedIdx::new(7, 5),
      PackedIdx::new(7, 6),
      // Top blade
      PackedIdx::new(5, 6),
      PackedIdx::new(5, 7),
      PackedIdx::new(4, 6),
    ];

    expect_that!(
      find_articulation_points(&poses),
      unordered_elements_are![
        &PackedIdx::new(5, 5),
        &PackedIdx::new(4, 4),
        &PackedIdx::new(5, 6),
        &PackedIdx::new(6, 5)
      ]
    );
  }

  #[apply(test_find_articulation_points)]
  #[gtest]
  fn test_articulation_points_filled_hex(
    find_articulation_points: impl FnOnce(&[PackedIdx; 7]) -> Vec<PackedIdx>,
  ) {
    let poses = [
      PackedIdx::new(2, 2),
      PackedIdx::new(2, 3),
      PackedIdx::new(3, 4),
      PackedIdx::new(4, 4),
      PackedIdx::new(4, 3),
      PackedIdx::new(3, 2),
      PackedIdx::new(3, 3),
    ];

    expect_that!(find_articulation_points(&poses), is_empty());
  }

  #[apply(test_find_articulation_points)]
  #[gtest]
  fn test_articulation_points_ring(
    find_articulation_points: impl FnOnce(&[PackedIdx; 6]) -> Vec<PackedIdx>,
  ) {
    let poses = [
      PackedIdx::new(2, 2),
      PackedIdx::new(2, 3),
      PackedIdx::new(3, 4),
      PackedIdx::new(4, 4),
      PackedIdx::new(4, 3),
      PackedIdx::new(3, 2),
    ];

    expect_that!(find_articulation_points(&poses), is_empty());
  }

  #[apply(test_find_articulation_points)]
  #[gtest]
  fn test_articulation_points_c_shape(
    find_articulation_points: impl FnOnce(&[PackedIdx; 7]) -> Vec<PackedIdx>,
  ) {
    let poses = [
      PackedIdx::new(2, 2),
      PackedIdx::new(1, 2),
      PackedIdx::new(2, 3),
      PackedIdx::new(3, 4),
      PackedIdx::new(4, 4),
      PackedIdx::new(4, 3),
      PackedIdx::new(5, 4),
    ];

    expect_that!(
      find_articulation_points(&poses),
      unordered_elements_are![
        &PackedIdx::new(2, 3),
        &PackedIdx::new(3, 4),
        &PackedIdx::new(4, 4),
      ]
    );
  }

  #[gtest]
  fn test_immobile_fidget_spinner() {
    let poses = [
      // Bottom left blade
      PackedIdx::new(4, 4),
      PackedIdx::new(3, 3),
      PackedIdx::new(4, 3),
      // Center
      PackedIdx::new(5, 5),
      // Right blade
      PackedIdx::new(6, 5),
      PackedIdx::new(7, 5),
      PackedIdx::new(7, 6),
      // Top blade
      PackedIdx::new(5, 6),
      PackedIdx::new(5, 7),
      PackedIdx::new(4, 6),
    ];

    expect_that!(
      find_immobile_points(&poses),
      unordered_elements_are![&PackedIdx::new(5, 5)]
    );
  }

  #[gtest]
  fn test_immobile_starting_point_fidget_spinner() {
    let poses = [
      // Center
      PackedIdx::new(5, 5),
      // Bottom left blade
      PackedIdx::new(4, 4),
      PackedIdx::new(3, 3),
      PackedIdx::new(4, 3),
      // Right blade
      PackedIdx::new(6, 5),
      PackedIdx::new(7, 5),
      PackedIdx::new(7, 6),
      // Top blade
      PackedIdx::new(5, 6),
      PackedIdx::new(5, 7),
      PackedIdx::new(4, 6),
    ];

    expect_that!(
      find_immobile_points(&poses),
      unordered_elements_are![&PackedIdx::new(5, 5)]
    );
  }

  const NEIGHBOR_INDEX_MASK_INPUTS: (
    [PackedIdx; 3],
    [PackedIdx; 5],
    [PackedIdx; 7],
    [PackedIdx; 10],
  ) = (
    [
      PackedIdx::new(3, 3),
      PackedIdx::new(4, 3),
      PackedIdx::new(4, 4),
    ],
    [
      PackedIdx::new(3, 3),
      PackedIdx::new(4, 3),
      PackedIdx::new(3, 2),
      PackedIdx::new(3, 4),
      PackedIdx::new(2, 3),
    ],
    [
      PackedIdx::new(2, 2),
      PackedIdx::new(1, 2),
      PackedIdx::new(2, 3),
      PackedIdx::new(3, 4),
      PackedIdx::new(4, 4),
      PackedIdx::new(4, 3),
      PackedIdx::new(5, 4),
    ],
    [
      PackedIdx::new(4, 4),
      PackedIdx::new(3, 3),
      PackedIdx::new(4, 3),
      PackedIdx::new(5, 5),
      PackedIdx::new(6, 5),
      PackedIdx::new(7, 5),
      PackedIdx::new(7, 6),
      PackedIdx::new(5, 6),
      PackedIdx::new(5, 7),
      PackedIdx::new(4, 6),
    ],
  );

  #[template]
  #[rstest]
  fn test_neighbor_index_mask_inputs<const N: usize>(
    #[values(
      &NEIGHBOR_INDEX_MASK_INPUTS.0,
      &NEIGHBOR_INDEX_MASK_INPUTS.1,
      &NEIGHBOR_INDEX_MASK_INPUTS.2,
      &NEIGHBOR_INDEX_MASK_INPUTS.3,
    )]
    pawn_poses: &[PackedIdx; N],
  ) {
  }

  #[apply(test_neighbor_index_mask_inputs)]
  #[gtest]
  fn test_neighbor_index_mask<const N: usize>(pawn_poses: &[PackedIdx; N]) {
    let p2_move_gen = P2MoveGenerator::from_pawn_poses(pawn_poses, true);
    for (index, pos) in pawn_poses.iter().enumerate() {
      let expected_neighbors: HashSet<_> = pos
        .neighbors()
        .filter(|neighbor| pawn_poses.iter().any(|pos| pos == neighbor))
        .map(HexPos::from)
        .collect();

      let meta = p2_move_gen.pawn_meta[index];
      let neighbor_pos_from_mask: HashSet<HexPos> = meta
        .neighbor_index_mask
        .iter_ones()
        .map(|index| pawn_poses[index as usize].into())
        .collect();

      assert_that!(neighbor_pos_from_mask, container_eq(expected_neighbors));
    }
  }

  fn lower_left<const N: usize>(onoro: &OnoroImpl<N>) -> HexPos {
    let (min_x, min_y) = onoro
      .pawn_poses()
      .iter()
      .fold((u32::MAX, u32::MAX), |(min_x, min_y), pawn_pos| {
        (min_x.min(pawn_pos.x()), min_y.min(pawn_pos.y()))
      });
    HexPos::new(min_x, min_y)
  }

  fn pawn_idx_at<const N: usize>(pos: HexPos, onoro: &OnoroImpl<N>) -> u32 {
    onoro
      .pawn_poses()
      .iter()
      .enumerate()
      .find_map(|(i, &pawn_pos)| (pawn_pos == pos.into()).then_some(i))
      .unwrap() as u32
  }

  fn phase2_moves_for(pawn_index: u32, moves: &[Move]) -> impl Iterator<Item = &Move> {
    moves.iter().filter(move |m| match m {
      Move::Phase2Move { from_idx, .. } => *from_idx == pawn_index,
      _ => unreachable!(),
    })
  }

  #[gtest]
  fn test_find_moves_simple() -> OnoroResult {
    let onoro = Onoro8::from_board_string(
      ". W W
        B B B
         W W .
          B . .",
    )?;

    let lower_left = lower_left(&onoro);

    let b1 = pawn_idx_at(lower_left, &onoro);
    let b2 = pawn_idx_at(lower_left + HexPosOffset::new(0, 2), &onoro);
    let b3 = pawn_idx_at(lower_left + HexPosOffset::new(1, 2), &onoro);
    let b4 = pawn_idx_at(lower_left + HexPosOffset::new(2, 2), &onoro);

    let move_gen = P2MoveGenerator::new(&onoro);
    let moves = move_gen.to_iter(&onoro).collect_vec();

    expect_eq!(phase2_moves_for(b1, &moves).count(), 5);
    expect_eq!(phase2_moves_for(b2, &moves).count(), 5);
    expect_eq!(phase2_moves_for(b3, &moves).count(), 7);
    expect_eq!(phase2_moves_for(b4, &moves).count(), 5);

    Ok(())
  }

  #[gtest]
  fn test_find_moves_dangling() -> OnoroResult {
    let onoro = Onoro8::from_board_string(
      ". W W B
        . B W .
         B B . .
          W . . .",
    )?;

    let lower_left = lower_left(&onoro);

    let b1 = pawn_idx_at(lower_left + HexPosOffset::new(0, 1), &onoro);
    let b2 = pawn_idx_at(lower_left + HexPosOffset::new(1, 1), &onoro);
    let b3 = pawn_idx_at(lower_left + HexPosOffset::new(1, 2), &onoro);
    let b4 = pawn_idx_at(lower_left + HexPosOffset::new(3, 3), &onoro);

    let move_gen = P2MoveGenerator::new(&onoro);
    let moves = move_gen.to_iter(&onoro).collect_vec();

    expect_eq!(phase2_moves_for(b1, &moves).count(), 1);
    expect_eq!(phase2_moves_for(b2, &moves).count(), 1);
    expect_eq!(phase2_moves_for(b3, &moves).count(), 2);
    expect_eq!(phase2_moves_for(b4, &moves).count(), 5);

    Ok(())
  }

  #[gtest]
  fn test_find_moves_disconnected() -> OnoroResult {
    let onoro = Onoro8::from_board_string(
      ". W W
        W B .
         . B .
          B B .
           W . .",
    )?;

    let lower_left = lower_left(&onoro);

    let b1 = pawn_idx_at(lower_left + HexPosOffset::new(0, 1), &onoro);
    let b2 = pawn_idx_at(lower_left + HexPosOffset::new(1, 1), &onoro);
    let b3 = pawn_idx_at(lower_left + HexPosOffset::new(1, 2), &onoro);
    let b4 = pawn_idx_at(lower_left + HexPosOffset::new(1, 3), &onoro);

    let move_gen = P2MoveGenerator::new(&onoro);
    let moves = move_gen.to_iter(&onoro).collect_vec();

    expect_eq!(phase2_moves_for(b1, &moves).count(), 1);
    expect_eq!(phase2_moves_for(b2, &moves).count(), 1);
    expect_eq!(phase2_moves_for(b3, &moves).count(), 1);
    expect_eq!(phase2_moves_for(b4, &moves).count(), 0);

    Ok(())
  }

  #[gtest]
  fn test_find_moves_immobile() -> OnoroResult {
    let onoro = Onoro16::from_board_string(
      ". . W . . . . .
        . W B . W B B W
         . . B W B W B B
          W W . . . . . .
           B . . . . . . .",
    )?;

    let lower_left = lower_left(&onoro);

    let immobile_pawn = pawn_idx_at(lower_left + HexPosOffset::new(2, 2), &onoro);

    let move_gen = P2MoveGenerator::new(&onoro);
    let moves = move_gen.to_iter(&onoro).collect_vec();
    expect_eq!(phase2_moves_for(immobile_pawn, &moves).count(), 0);

    Ok(())
  }

  #[gtest]
  fn test_find_moves_cutting_point_first() -> OnoroResult {
    let onoro = Onoro16::from_board_string(
      ". . . B B .
        . B W . B W
         . W . . W W
          W B . . W .
           B B . . . .
            W B . . . .",
    )?;

    let mut pawn_indexes = *onoro.pawn_poses();
    let cutting_pawn_index = pawn_indexes
      .iter()
      .enumerate()
      .step_by(2)
      .max_by_key(|(_, pos)| pos.x() + pos.y() * 16)
      .unwrap()
      .0;
    pawn_indexes.swap(cutting_pawn_index, 0);

    let onoro = Onoro16::from_indexes(pawn_indexes);

    let lower_left = lower_left(&onoro);

    assert_eq!(pawn_idx_at(lower_left + HexPosOffset::new(4, 5), &onoro), 0);

    let move_gen = P2MoveGenerator::new(&onoro);
    expect_that!(
      move_gen.pawn_meta[0].connected_mobility,
      pat!(PawnConnectedMobility::CuttingPoint {
        exit_time: any![7, 12]
      })
    );

    let moves = move_gen.to_iter(&onoro).collect_vec();
    expect_eq!(phase2_moves_for(0, &moves).count(), 1);

    Ok(())
  }
}
