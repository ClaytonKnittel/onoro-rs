use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub struct OnoroError {
  message: String,
}

impl Error for OnoroError {}

impl Display for OnoroError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "Error: {}", self.message)
  }
}

pub type OnoroResult<T> = Result<T, OnoroError>;
