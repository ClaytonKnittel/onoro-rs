use abstract_game::GameMoveIterator;
use onoro::Onoro;

use crate::{Move, OnoroImpl, PackedIdx, p1_move_gen::P1MoveGenerator};

#[derive(Clone, Copy, Default)]
struct PawnMeta {
  /// The discovery index of this pawn when doing the depth-first exploration
  /// of the pawn graph.
  discovery_time: u32,
  /// If true, this pawn has only two neighbors, meaning if we move a pawn
  /// adjacent to it, it must be placed adjacent to it.
  has_two_neighbors: bool,

  // Below only relevant to current player's pawns:
  /// The time we have returned from exploring the subtree of this pawn.
  ///
  /// Note that this can happen at most twice, since we can have at most three
  /// disconnected coming out of a single tile. However, there is no legal move
  /// which disconnects the board into 3 groups and reconnects them in another
  /// location with 16 or fewer total pawns.
  exit_time: u32,
  /// If true, this pawn is an articulation point.
  is_cut: bool,
  /// If true, this pawn is connecting three disjoint groups. Any pawn in this
  /// position is immobile, as there is no configuration of pawns with two
  /// points that join 3 disjoint groups.
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
  is_immobile: bool,
}

impl PawnMeta {
  fn is_root(&self) -> bool {
    self.discovery_time == 1
  }

  fn is_cut(&self) -> bool {
    self.is_cut
  }

  fn is_immobile(&self) -> bool {
    self.is_immobile
  }
}

pub struct P2MoveGenerator<const N: usize> {
  pawn_meta: [PawnMeta; N],
  p1_move_gen: P1MoveGenerator<N>,
}

impl<const N: usize> P2MoveGenerator<N> {
  pub fn new(onoro: &OnoroImpl<N>) -> Self {
    debug_assert!(!onoro.in_phase1());
    Self::from_pawn_poses(onoro.pawn_poses())
  }

  pub fn from_pawn_poses(pawn_poses: &[PackedIdx; N]) -> Self {
    let p1_move_gen = P1MoveGenerator::from_pawn_poses(pawn_poses);
    let pawn_meta = Self::build_pawn_meta(pawn_poses, &p1_move_gen);

    Self {
      pawn_meta,
      p1_move_gen,
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

    for neighbor_index in Self::neighbors(pawn_index, pawn_poses, p1_move_gen)
      .filter(|&neighbor_index| neighbor_index != parent_index)
    {
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
        meta.is_cut = true;

        if meta.exit_time == 0 {
          meta.exit_time = *time;
        } else {
          meta.is_immobile = true;
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

    let mut neighbor_count = 0;
    for neighbor_index in Self::neighbors(0, pawn_poses, p1_move_gen) {
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
      neighbor_count += 1;
    }

    if neighbor_count > 1 {
      pawn_meta[0].is_cut = true;
    }
    if neighbor_count > 2 {
      pawn_meta[0].is_immobile = true;
    }

    pawn_meta
  }
}

impl<const N: usize> GameMoveIterator for P2MoveGenerator<N> {
  type Item = Move;
  type Game = OnoroImpl<N>;

  fn next(&mut self, _game: &Self::Game) -> Option<Self::Item> {
    None
  }
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;

  use googletest::{gtest, prelude::*};
  use onoro::hex_pos::HexPos;
  use rstest::rstest;
  use rstest_reuse::{apply, template};

  use crate::{PackedIdx, p2_move_gen::P2MoveGenerator};

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
    let move_gen = P2MoveGenerator::from_pawn_poses(pawn_poses);
    move_gen
      .pawn_meta
      .into_iter()
      .enumerate()
      .filter_map(|(idx, meta)| meta.is_cut().then_some(pawn_poses[idx]))
      .collect()
  }

  fn find_immobile_points<const N: usize>(pawn_poses: &[PackedIdx; N]) -> Vec<PackedIdx> {
    let move_gen = P2MoveGenerator::from_pawn_poses(pawn_poses);
    move_gen
      .pawn_meta
      .into_iter()
      .enumerate()
      .filter_map(|(idx, meta)| meta.is_immobile().then_some(pawn_poses[idx]))
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
    find_articulation_points: impl FnOnce(&[PackedIdx; 3]) -> Vec<PackedIdx>,
  ) {
    let poses = [
      PackedIdx::new(3, 3),
      PackedIdx::new(4, 3),
      PackedIdx::new(3, 4),
    ];

    expect_that!(
      find_articulation_points(&poses),
      unordered_elements_are![&PackedIdx::new(3, 3)]
    );
  }

  #[apply(test_find_articulation_points)]
  #[gtest]
  fn test_articulation_points_fidget_spinner(
    find_articulation_points: impl FnOnce(&[PackedIdx; 4]) -> Vec<PackedIdx>,
  ) {
    let poses = [
      PackedIdx::new(2, 2),
      PackedIdx::new(3, 3),
      PackedIdx::new(4, 3),
      PackedIdx::new(3, 4),
    ];

    expect_that!(
      find_articulation_points(&poses),
      unordered_elements_are![&PackedIdx::new(3, 3)]
    );
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
    find_articulation_points: impl FnOnce(&[PackedIdx; 5]) -> Vec<PackedIdx>,
  ) {
    let poses = [
      PackedIdx::new(2, 2),
      PackedIdx::new(2, 3),
      PackedIdx::new(3, 4),
      PackedIdx::new(4, 4),
      PackedIdx::new(4, 3),
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
      PackedIdx::new(2, 2),
      PackedIdx::new(3, 3),
      PackedIdx::new(4, 3),
      PackedIdx::new(3, 4),
    ];

    expect_that!(
      find_immobile_points(&poses),
      unordered_elements_are![&PackedIdx::new(3, 3)]
    );
  }

  #[gtest]
  fn test_immobile_starting_point_fidget_spinner() {
    let poses = [
      PackedIdx::new(3, 3),
      PackedIdx::new(2, 2),
      PackedIdx::new(4, 3),
      PackedIdx::new(3, 4),
    ];

    expect_that!(
      find_immobile_points(&poses),
      unordered_elements_are![&PackedIdx::new(3, 3)]
    );
  }
}
