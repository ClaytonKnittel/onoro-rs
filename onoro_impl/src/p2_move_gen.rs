use abstract_game::GameMoveIterator;
use onoro::Onoro;

use crate::{Move, OnoroImpl, p1_move_gen::P1MoveGenerator};

#[derive(Clone, Copy, Default)]
struct PawnMeta {
  /// The discovery index of this pawn when doing the depth-first exploration
  /// of the pawn graph.
  discovery_time: u32,
  /// If true, this pawn has only two neighbors, meaning if we move a pawn
  /// adjacent to it, it must be placed adjacent to it.
  has_two_neighbors: bool,

  // Below only relevant to current player's pawns:
  /// Each time we have returned from exploring a subtree of this pawn.
  ///
  /// Note that this can happen at most twice, since we can have at most three
  /// disconnected coming out of a single tile. For non-root tiles, one of the
  /// branches must be the parent. For the root tile, this will happen three
  /// times, but we don't need to record the time of the third return, since
  /// that time would be larger than the discovery time of every tile.
  exit_times: (u32, u32),
  /// If true, this pawn is an articulation point.
  is_cut: bool,
}

impl PawnMeta {
  fn is_root(&self) -> bool {
    self.discovery_time == 1
  }
}

pub struct P2MoveGenerator<const N: usize> {
  pawn_meta: [PawnMeta; N],
  p1_move_gen: P1MoveGenerator<N>,
}

impl<const N: usize> P2MoveGenerator<N> {
  pub fn new(onoro: &OnoroImpl<N>) -> Self {
    debug_assert!(!onoro.in_phase1());

    let p1_move_gen = P1MoveGenerator::new(onoro);
    let pawn_meta = Self::build_pawn_meta(onoro, &p1_move_gen);

    Self {
      pawn_meta,
      p1_move_gen,
    }
  }

  fn recursor(
    pawn_meta: &mut [PawnMeta; N],
    ecas: &mut [u32; N],
    p1_move_gen: &P1MoveGenerator<N>,
  ) {
  }

  fn build_pawn_meta(onoro: &OnoroImpl<N>, p1_move_gen: &P1MoveGenerator<N>) -> [PawnMeta; N] {
    let indexer = p1_move_gen.indexer();
    let mut pawn_meta = [PawnMeta::default(); N];
    let mut ecas = [0u32; N];
    pawn_meta[0].discovery_time = 1;
    ecas[0] = 1;

    for neighbor_index in p1_move_gen.neighbors(indexer.index(onoro.pawn_poses()[0])) {}

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
  use itertools::Itertools;
  use onoro::hex_pos::HexPos;

  use crate::PackedIdx;

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

  fn find_articulation_points_simple(pawn_poses: &[PackedIdx]) -> impl Iterator<Item = PackedIdx> {
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
  }

  #[gtest]
  fn test_no_articulation_points() {
    let poses = [
      PackedIdx::new(3, 3),
      PackedIdx::new(4, 3),
      PackedIdx::new(4, 4),
    ];

    expect_that!(
      find_articulation_points_simple(&poses).collect_vec(),
      is_empty()
    );
  }

  #[gtest]
  fn test_one_articulation_point() {
    let poses = [
      PackedIdx::new(3, 3),
      PackedIdx::new(4, 3),
      PackedIdx::new(3, 4),
    ];

    expect_that!(
      find_articulation_points_simple(&poses).collect_vec(),
      unordered_elements_are![&PackedIdx::new(3, 3)]
    );
  }

  #[gtest]
  fn test_articulation_points_fidget_spinner() {
    let poses = [
      PackedIdx::new(2, 2),
      PackedIdx::new(3, 3),
      PackedIdx::new(4, 3),
      PackedIdx::new(3, 4),
    ];

    expect_that!(
      find_articulation_points_simple(&poses).collect_vec(),
      unordered_elements_are![&PackedIdx::new(3, 3)]
    );
  }

  #[gtest]
  fn test_articulation_points_ring() {
    let poses = [
      PackedIdx::new(2, 2),
      PackedIdx::new(2, 3),
      PackedIdx::new(3, 4),
      PackedIdx::new(4, 4),
      PackedIdx::new(4, 3),
      PackedIdx::new(3, 2),
    ];

    expect_that!(
      find_articulation_points_simple(&poses).collect_vec(),
      is_empty()
    );
  }

  #[gtest]
  fn test_articulation_points_c_shape() {
    let poses = [
      PackedIdx::new(2, 2),
      PackedIdx::new(2, 3),
      PackedIdx::new(3, 4),
      PackedIdx::new(4, 4),
      PackedIdx::new(4, 3),
    ];

    expect_that!(
      find_articulation_points_simple(&poses).collect_vec(),
      unordered_elements_are![
        &PackedIdx::new(2, 3),
        &PackedIdx::new(3, 4),
        &PackedIdx::new(4, 4),
      ]
    );
  }
}
