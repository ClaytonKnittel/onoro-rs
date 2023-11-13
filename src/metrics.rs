#[derive(Clone, Debug, Default)]
pub struct Metrics {
  pub n_states: u64,
  pub n_hits: u64,
  pub n_misses: u64,
  pub n_leaves: u64,
}

impl Metrics {
  pub fn new() -> Self {
    Self::default()
  }
}

impl std::ops::Add for Metrics {
  type Output = Self;

  fn add(self, rhs: Self) -> Self::Output {
    Self {
      n_states: self.n_states + rhs.n_states,
      n_hits: self.n_hits + rhs.n_hits,
      n_misses: self.n_misses + rhs.n_misses,
      n_leaves: self.n_leaves + rhs.n_leaves,
    }
  }
}

impl std::ops::AddAssign for Metrics {
  fn add_assign(&mut self, rhs: Self) {
    *self = self.clone() + rhs;
  }
}
