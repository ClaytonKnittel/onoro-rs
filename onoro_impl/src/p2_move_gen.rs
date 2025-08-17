use abstract_game::GameMoveIterator;
use onoro::Onoro;

use crate::{Move, OnoroImpl};

fn find_articulation_points() {}

pub struct P2MoveGenerator<const N: usize> {}

impl<const N: usize> P2MoveGenerator<N> {
  pub fn new(onoro: &OnoroImpl<N>) -> Self {
    debug_assert!(!onoro.in_phase1());

    Self {}
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
}
