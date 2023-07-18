use std::fmt::{Debug, Display};

use super::score::Score;

#[derive(Debug, PartialEq, Eq)]
pub struct PackedScore<P> {
  data: (u16, u8),
  /// Extra data that is packed inside the score struct to save memory. This
  /// should have size 1 for score to be minimally sized.
  packed_data: P,
}

impl<P> PackedScore<P> {
  pub fn new(score: Score, packed_data: P) -> Self {
    Self {
      data: score.data,
      packed_data,
    }
  }

  pub fn score(&self) -> Score {
    Score { data: self.data }
  }

  pub fn packed_data(&self) -> &P {
    &self.packed_data
  }

  pub fn mut_packed_data(&mut self) -> &mut P {
    &mut self.packed_data
  }
}

impl<P> Display for PackedScore<P>
where
  P: Display,
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{} ({})", self.score(), self.packed_data)
  }
}
