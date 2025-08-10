use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub struct OnoroError {
  message: String,
}

impl OnoroError {
  pub fn new(message: String) -> Self {
    OnoroError { message }
  }
}

impl Error for OnoroError {}

impl Display for OnoroError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "Error: {}", self.message)
  }
}

#[macro_export]
macro_rules! make_onoro_error {
  ($($args:expr),+) => {
    $crate::error::OnoroError::new(format!($($args),+)).into()
  };
}

pub type OnoroResult<T> = Result<T, Box<dyn Error + Send + Sync + 'static>>;
