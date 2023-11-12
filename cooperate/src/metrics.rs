#[derive(Clone, Debug, Default)]
pub struct Metrics {
  pub hits: u64,
  pub queues: u64,
  pub claims: u64,
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
      hits: self.hits + rhs.hits,
      queues: self.queues + rhs.queues,
      claims: self.claims + rhs.claims,
    }
  }
}

impl std::ops::AddAssign for Metrics {
  fn add_assign(&mut self, rhs: Self) {
    *self = self.clone() + rhs;
  }
}
