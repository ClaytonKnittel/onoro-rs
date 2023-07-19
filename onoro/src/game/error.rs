use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub struct OnoroError {
  message: String,
}

impl OnoroError {
  pub(crate) fn new(message: &str) -> Self {
    OnoroError {
      message: message.to_owned(),
    }
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
    $crate::game::onoro::OnoroError::new(&format!($($args),+))
  };
}

pub type OnoroResult<T> = Result<T, OnoroError>;
