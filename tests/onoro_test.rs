use onoro::Onoro;
use rstest::{fixture, rstest};
use rstest_reuse::{apply, template};

#[template]
#[rstest]
#[case(Box::new(Onoro16::default_start()))]
fn onoro(onoro: OnoroVariant) {}

// #[apply(onoro)]
fn test_default_start() {
  let x: Box<dyn Onoro> = Box::new(Onoro16::default_start());
}
