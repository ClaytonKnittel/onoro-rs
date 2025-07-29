use crate::error::OnoroResult;

pub trait Compress: Sized {
  type Repr;

  fn compress(&self) -> Self::Repr;

  fn decompress(repr: Self::Repr) -> OnoroResult<Self>;
}
