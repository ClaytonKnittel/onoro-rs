use std::{
  cell::UnsafeCell,
  ops::{Deref, DerefMut},
};

/// Can be used to make a type Send + Sync. Unsafe, as the user must know that
/// the type is never concurrently accessed for race conditions to not be
/// possible.
pub struct NullLock<T> {
  data: UnsafeCell<T>,
}

impl<T> NullLock<T> {
  pub unsafe fn new(item: T) -> Self {
    Self {
      data: UnsafeCell::new(item),
    }
  }

  #[allow(clippy::mut_from_ref)]
  pub unsafe fn lock(&self) -> &mut T {
    unsafe { &mut *self.data.get() }
  }
}

unsafe impl<T> Send for NullLock<T> {}
unsafe impl<T> Sync for NullLock<T> {}

impl<T> Deref for NullLock<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    unsafe { &*self.data.get() }
  }
}

impl<T> DerefMut for NullLock<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.data.get_mut()
  }
}
