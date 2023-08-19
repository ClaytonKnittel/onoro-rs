#[derive(Clone, Copy)]
pub struct ParSearchOptions {
  pub n_threads: u32,
  pub unit_depth: u32,
}

impl ParSearchOptions {
  pub fn with_n_threads(&self, n_threads: u32) -> Self {
    Self { n_threads, ..*self }
  }

  pub fn with_unit_depth(&self, unit_depth: u32) -> Self {
    Self {
      unit_depth,
      ..*self
    }
  }
}

impl Default for ParSearchOptions {
  fn default() -> Self {
    Self {
      n_threads: 4,
      unit_depth: 3,
    }
  }
}
