/// A trait for finite sets. The size of the set should depend entirely on the
/// type of the set, not its construction.
pub trait Finite {
  const SIZE: usize;

  fn size() -> usize {
    Self::SIZE
  }
}
