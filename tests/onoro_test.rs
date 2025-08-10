use onoro::Onoro;
use rstest::rstest;
use rstest_reuse::{apply, template};

#[template]
#[rstest]
fn onoro(
  #[values(
    onoro::Onoro16::default_start(),
    ai_gen_onoro::OnoroGame::default_start()
  )]
  onoro: impl Onoro,
) {
}

#[apply(onoro)]
fn test_default_start(onoro: impl Onoro) {}
