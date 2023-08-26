use std::fmt::{Debug, Display};

use crate::Score;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackedScore<P> {
  /// Third entry is extra data that is packed inside the score struct to save
  /// memory. This should have size 1 for score to be minimally sized.
  data: (u16, u8, P),
}

impl<P> PackedScore<P> {
  pub const fn new(score: Score, packed_data: P) -> Self {
    Self {
      data: (score.data.0, score.data.1, packed_data),
    }
  }

  pub const fn score(&self) -> Score {
    Score {
      data: (self.data.0, self.data.1),
    }
  }

  pub const fn packed_data(&self) -> &P {
    &self.data.2
  }

  pub fn mut_packed_data(&mut self) -> &mut P {
    &mut self.data.2
  }
}

impl<P> Display for PackedScore<P>
where
  P: Display,
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{} ({})", self.score(), self.packed_data())
  }
}
