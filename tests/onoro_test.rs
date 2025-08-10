use onoro::Onoro;
use rstest::rstest;
use rstest_reuse::{apply, template};

#[template]
#[rstest]
fn onoro_fixture(
  #[values(
    onoro::Onoro16::default_start(),
    ai_gen_onoro::OnoroGame::default_start()
  )]
  onoro: impl Onoro,
) {
}

#[apply(onoro_fixture)]
fn test_pawns_in_play(onoro: impl Onoro) {
  assert_eq!(onoro.pawns_in_play(), 3);
}
