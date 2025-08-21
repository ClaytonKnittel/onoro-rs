use num_traits::PrimInt;

pub trait IterOnes {
  /// Given an integer, returns an iterator over the bit indices with ones.
  fn iter_ones(self) -> impl Iterator<Item = u32>;
}

impl<I: PrimInt> IterOnes for I {
  fn iter_ones(self) -> impl Iterator<Item = u32> {
    let if_ne_zero = |value: I| (value != I::zero()).then_some(value);
    std::iter::successors(if_ne_zero(self), move |&value| {
      let value = value & (value - I::one());
      if_ne_zero(value)
    })
    .map(|mask| mask.trailing_zeros())
  }
}
