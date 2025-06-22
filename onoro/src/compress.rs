pub trait Compress {
  type Repr;

  fn compress(&self) -> Self::Repr;

  fn decompress(repr: Self::Repr) -> Self;
}
