use algebra::semigroup::Semigroup;
use googletest::gtest;
use onoro::{
  error::OnoroResult, groups::D6, hex_pos::HexPos, test_util::BOARD_POSITIONS, Onoro, TileState,
};
use onoro_impl::{
  benchmark_util::{board_symm_state, TestOnlyCompareViewsIgnoringHash},
  Onoro16, OnoroView, PackedIdx,
};
use rand::{rngs::StdRng, Rng, SeedableRng};
use rstest::rstest;
use rstest_reuse::{apply, template};

#[template]
#[rstest]
#[rustfmt::skip]
fn random_states(
  #[values(
    BOARD_POSITIONS[0],
    BOARD_POSITIONS[1],
    BOARD_POSITIONS[2],
    BOARD_POSITIONS[3],
    BOARD_POSITIONS[4],
    BOARD_POSITIONS[5],
    BOARD_POSITIONS[6],
    BOARD_POSITIONS[7],
    BOARD_POSITIONS[8],
    BOARD_POSITIONS[9],
    BOARD_POSITIONS[10],
    BOARD_POSITIONS[11],
  )]
  board_string: &str,
  #[values(
    0x80fd90e95c2f13e9, 0x5471d495b89053b9, 0xc627efbb2eed030b, 0x01a445d82f9f2021,
    0xe3bf475fd9109bf5, 0x00b1b38e09b284d2, 0xc375da5a36154ce3, 0xbe284ab710be8050,
    0xa52c989b265e454c, 0x8d2448bdf7208b65, 0x5891d4f6b3f61672, 0xbd204e48ab72a5cc,
    0x89fc831bddd7250c, 0x817d0a7d2c7f3663, 0x84e971050f5b4d1d, 0xe92696a9f391b10f,
    0xcad2fe42422c8091, 0x89d64462b811ac2c, 0xfc1dab4507e59064, 0x382a6788f1c2f859,
    0xa20df572a82c8f57, 0xebe379f78c1f379e, 0xfa18905aed704ab5, 0x7eb5c244139b9514,
    0x3e55be2c02f31bc6, 0xda3f253722535e21, 0xf752b0600fb4eb70, 0x26179030139e3bc7,
    0x16ca8231cd24ebd6, 0xe0fd262cb6b8a0a6, 0x19519a29f7b189c6, 0xbdee445157381297,
    0x6e57898105cb208b, 0x0936ce75456665ee, 0xbc21fc70de13043b, 0x232d7e5597e12024,
    0xd08d5beb54583534, 0x1e19b7ef94504034, 0x6de62076f9d70a28, 0x39f7ced0ec4c2cc4,
    0x99b5e55b529e3bc5, 0x3c2bd206f86c1762, 0x3dfe73f9e08fa92c, 0x0247efa155852b5f,
    0xc0c3b19cb5ea7ec8, 0x5003ac15e2118cb4, 0xdc9f9b519196f43a, 0xab64091372563a88,
    0x822c547170dc6a15, 0x843f1e3e884fedc0, 0x22e86f358318984f, 0xc0f1cbc3a1ef783b,
    0x7ec5392b810c8630, 0xcdf70898e03aa18b, 0x3581fd5371ba4a4a, 0x390a6df1491d255a,
    0xda92017aa17b7ee6, 0xc561086a3460a80a, 0x78784cd406eadd00, 0x6f49ee88d68ad9a0,
    0x5da4dbcb73145b33, 0xd1b006567cf6b5db, 0xdfbf3079c51f1ca9, 0xcb2e93a448402216,
  )] seed: u64,
) {
}

#[apply(random_states)]
#[gtest]
fn test_equal_view(board_string: &str, seed: u64) -> OnoroResult {
  let onoro = Onoro16::from_board_string(board_string)?;
  let mut rng = StdRng::seed_from_u64(seed);

  for i in 0..64 {
    // Make between 1-30 random moves, preferring more moves.

    use onoro_impl::benchmark_util::random_unfinished_state;
    let num_moves = rng.random_range(1..=30).max(rng.random_range(1..=30));
    let onoro = random_unfinished_state(&onoro, num_moves, &mut rng)?;

    // Try comparing all rotations of the board.
    for op in D6::for_each() {
      let rotated = onoro.rotated_d6_c(op);

      let view1 = OnoroView::new(onoro.clone());
      let view2 = OnoroView::new(rotated);

      assert_eq!(
        view1.hash(),
        view2.hash(),
        "Failed on iteration {i} for rotation {op}"
      );
      assert_eq!(view1, view2, "Failed on iteration {i} for rotation {op}");
    }
  }

  Ok(())
}

fn equal_slow(onoro1: &Onoro16, onoro2: &Onoro16) -> bool {
  if onoro1.pawns_in_play() != onoro2.pawns_in_play() {
    return false;
  }

  let symm_state1 = board_symm_state(onoro1);
  let origin1 = onoro1.origin(&symm_state1);

  D6::for_each().any(|op| {
    let onoro2 = onoro2.rotated_d6_c(op);

    let symm_state2 = board_symm_state(&onoro2);
    let origin2 = onoro2.origin(&symm_state2);

    onoro1.pawns().all(|pawn| {
      let relative_pos = HexPos::from(pawn.pos) - origin1;
      let onoro2_pos = relative_pos + origin2;

      PackedIdx::maybe_from(onoro2_pos)
        .is_some_and(|pos| TileState::from(pawn.color) == onoro2.get_tile(pos))
    })
  })
}

#[apply(random_states)]
#[gtest]
fn test_inequal_view(board_string: &str, seed: u64) -> OnoroResult {
  let onoro = Onoro16::from_board_string(board_string)?;
  let mut rng = StdRng::seed_from_u64(seed);

  for i in 0..64 {
    // Make between 1-30 random moves, preferring more moves.

    use onoro_impl::benchmark_util::random_unfinished_state;
    let num_moves = rng.random_range(1..=30).max(rng.random_range(1..=30));
    let onoro = random_unfinished_state(&onoro, num_moves, &mut rng)?;
    // Make 1 - 4 more random moves.
    let onoro2 = random_unfinished_state(&onoro, rng.random_range(1..=4), &mut rng)?;

    // Try comparing all rotations of the board.
    for op in D6::for_each() {
      let rotated = onoro2.rotated_d6_c(op);
      let are_equal = equal_slow(&onoro, &rotated);

      let view1 = OnoroView::new(onoro.clone());
      let view2 = OnoroView::new(rotated);

      if are_equal {
        assert_eq!(view1, view2, "Failed on iteration {i} for rotation {op}");
      } else {
        assert_ne!(view1, view2, "Failed on iteration {i} for rotation {op}");
        assert!(
          !view1.cmp_views_ignoring_hash(&view2),
          "Failed on iteration {i} for rotation {op}"
        );
      }
    }
  }

  Ok(())
}
