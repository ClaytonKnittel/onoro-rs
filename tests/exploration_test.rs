use googletest::{assert_that, gtest, prelude::container_eq};
use onoro::{
  error::OnoroResult,
  test_util::{normalized_ordered_moves, OnoroFactory},
  Onoro,
};
use rand::Rng;
use rstest::rstest;
use rstest_reuse::{apply, template};

struct Onoro16Factory;
impl OnoroFactory for Onoro16Factory {
  type T = onoro_impl::Onoro16;
}

struct AiOnoroFactory;
impl OnoroFactory for AiOnoroFactory {
  type T = ai_gen_onoro::OnoroGame;
}

#[template]
#[rstest]
fn onoro_pairs(
  #[values((Onoro16Factory, AiOnoroFactory))] factories: (impl OnoroFactory, impl OnoroFactory),
) {
}

#[apply(onoro_pairs)]
#[gtest]
fn test_random_exploration<T: OnoroFactory, U: OnoroFactory>(_factories: (T, U)) -> OnoroResult {
  use rand::{rngs::StdRng, SeedableRng};
  let mut onoro1 = T::from_board_string(
    ". W
      B B",
  )?;
  let mut onoro2 = U::from_board_string(
    ". W
      B B",
  )?;

  let mut rng = StdRng::seed_from_u64(123456);

  for _ in 0..2 {
    let m1 = normalized_ordered_moves(&onoro1)?;
    let m2 = normalized_ordered_moves(&onoro2)?;
    assert_that!(m1, container_eq(m2.clone()));

    let move_idx = rng.gen_range(0..m1.len());
    onoro1.make_move(m1[move_idx].original());
    onoro2.make_move(m2[move_idx].original());
  }

  Ok(())
}
