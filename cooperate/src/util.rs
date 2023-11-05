use std::marker::PhantomData;

/// A simple iterator which performs a function when iterated over, but yields
/// no elements.
pub struct TransparentIterator<Fn: FnOnce(), T> {
  function: Option<Fn>,
  _p: PhantomData<T>,
}

impl<Fn: FnOnce(), T> TransparentIterator<Fn, T> {
  pub fn new(function: Fn) -> Self {
    Self {
      function: Some(function),
      _p: PhantomData,
    }
  }
}

impl<Fn: FnOnce(), T> Iterator for TransparentIterator<Fn, T> {
  type Item = T;

  fn next(&mut self) -> Option<Self::Item> {
    if let Some(function) = self.function.take() {
      function();
    }
    None
  }
}

#[cfg(test)]
mod tests {
  use super::TransparentIterator;

  #[test]
  fn test_transparent_iterator() {
    let mut v = 0;
    let v_ref = &mut v;
    for _ in (0..10).chain(TransparentIterator::new(|| *v_ref = 1)) {}
    assert_eq!(v, 1);
  }
}
