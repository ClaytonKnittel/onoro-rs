use algebra::{finite::Finite, ordinal::Ordinal};
use googletest::{expect_eq, gtest};
use onoro::{error::OnoroResult, groups::D6, test_util::BOARD_POSITIONS, Onoro};
use onoro_impl::{benchmark_util::random_unfinished_state, Onoro16, OnoroView};
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
    0x97153c8af3d7e275, 0x90ab05c07f4dee63, 0xe6a0fc271d33e8e5, 0x96cb8992055cce5d, 
    0x4cb0dfe9cc4bb4af, 0x3d2f9250cdde3443, 0x8703b21deed1f04a, 0x716b7a74453435ef, 
    0xee7cfaa0c8170b2f, 0xf545523cc1e64c7b, 0xa0ea6f5a36f1a897, 0xdfa93fc7825ea486, 
    0x2bf223a942ce00bb, 0x311ac379ea41a488, 0x9f6f27e10a3eaa77, 0xf63e1632b562e5d3, 
    0xf5a34e4fa171907e, 0x3ab223e7bce6dd78, 0xa93ec14213360a66, 0x517ad2805f8daf09, 
    0x584909fb557b8b1f, 0xe2b21057d1d0b424, 0x5da53a371840d302, 0x2faf26c97bec2d33, 
    0x413bda9b5bb9f6b2, 0xbdffa6440d8087aa, 0x4966731d7cec3109, 0x470f363fc0ee08d8, 
    0x8feaf45a900ee957, 0xb3a9fd9b48262b6d, 0xd8178e5463678586, 0x9c8ecc78eaf466f1, 
    0x1c9b1088faae1363, 0x5421f5e479fa892a, 0x73ea9b7f6a2c77a5, 0xd63fae2d81e444df, 
    0xf965bf272882061b, 0xd21faa7bbc961f89, 0x1f496d5f65195611, 0xc7c41a686fa0e2a7, 
    0x315c2b51477965cc, 0xc374cfb12a125972, 0xf2b1fc1f53c2e70e, 0xd74e3979e25d765d, 
    0xf08675facb18b01c, 0x9ba572c296e29b7c, 0xbd22321f2e158a2d, 0xc7b3200030477a2f, 
    0x5562a955466067a1, 0x7a647f495bdade39, 0x1679752ee8729181, 0xc18c19bba79b4100, 
    0x33381bcf841d8a3d, 0x475c35a092ba15e0, 0xa0e79d7dc8283064, 0xbb37452cc9ca9742, 
    0x23dcb634ee995173, 0x2e8a4d4792e97010, 0xfab52d9193d6059c, 0x781c13179f1fbb38, 
    0xa6674c5fe54fe018, 0x389c7d749ce997ae, 0x234125fdc7dc23ad, 0xea617d7730315597,
  )] seed: u64,
) {
}

#[apply(random_states)]
#[gtest]
fn test_random_exploration(board_string: &str, seed: u64) -> OnoroResult {
  let onoro = Onoro16::from_board_string(board_string)?;
  let mut rng = StdRng::seed_from_u64(seed);

  // Make between 1-30 random moves, preferring more moves.
  let num_moves = rng.gen_range(1..=30).max(rng.gen_range(1..=30));
  let onoro = random_unfinished_state(&onoro, num_moves, &mut rng)?;

  // Randomly rotate the board.
  let ord = rng.gen_range(0..D6::SIZE);
  let op = D6::from_ord(ord);
  let rotated = onoro.rotated_d6_c(op);

  let view1 = OnoroView::new(onoro);
  let view2 = OnoroView::new(rotated);

  expect_eq!(view1.canon_view().hash(), view2.canon_view().hash());
  expect_eq!(view1, view2);

  Ok(())
}
