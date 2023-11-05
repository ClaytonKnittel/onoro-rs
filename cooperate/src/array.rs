use std::{
  alloc::{alloc, dealloc, Layout},
  hash::Hash,
  marker::PhantomData,
  mem::{align_of, size_of},
};

pub struct Array<T> {
  size: u32,
  capacity: u32,
  els: *mut u8,
  _p: PhantomData<T>,
}

impl<T> Array<T>
where
  T: Sized,
{
  pub fn new(capacity: u32) -> Self {
    let t_size = size_of::<T>();
    let t_align = align_of::<T>();
    // TODO: arena allocate with custom GlobalAlloc
    let array =
      unsafe { alloc(Layout::from_size_align(t_size * capacity as usize, t_align).unwrap()) };

    Self {
      size: 0,
      capacity,
      els: array,
      _p: PhantomData,
    }
  }

  fn slot(&self, idx: u32) -> *const T {
    let t_size = size_of::<T>() as isize;
    unsafe {
      let slot_ptr = self.els.offset(idx as isize * t_size);
      slot_ptr.cast()
    }
  }

  fn slot_mut(&mut self, idx: u32) -> *mut T {
    let t_size = size_of::<T>() as isize;
    unsafe {
      let slot_ptr = self.els.offset(idx as isize * t_size);
      slot_ptr.cast()
    }
  }

  pub fn len(&self) -> usize {
    self.size as usize
  }

  pub fn is_empty(&self) -> bool {
    self.size == 0
  }

  pub fn is_full(&self) -> bool {
    self.size == self.capacity
  }

  pub fn capacity(&self) -> usize {
    self.capacity as usize
  }

  pub fn get(&self, idx: u32) -> &T {
    debug_assert!(idx < self.size);

    unsafe { &*self.slot(idx) }
  }

  pub fn get_mut(&mut self, idx: u32) -> &mut T {
    debug_assert!(idx < self.size);

    unsafe { &mut *self.slot_mut(idx) }
  }

  pub fn push(&mut self, el: T) {
    debug_assert!(self.size != self.capacity);

    unsafe {
      self.slot_mut(self.size).write(el);
    }
    self.size += 1;
  }

  pub fn pop(&mut self) -> T {
    debug_assert!(self.size != 0);

    self.size -= 1;
    unsafe { self.slot_mut(self.size).read() }
  }

  pub fn last(&self) -> Option<&T> {
    if self.is_empty() {
      return None;
    }

    Some(unsafe { &*self.slot(self.size - 1) })
  }

  pub fn last_mut(&mut self) -> Option<&mut T> {
    if self.is_empty() {
      return None;
    }

    Some(unsafe { &mut *self.slot_mut(self.size - 1) })
  }

  pub fn iter(&self) -> impl Iterator<Item = &T> {
    (0..self.size).map(|idx| self.get(idx))
  }
}

impl<T> Clone for Array<T>
where
  T: Clone,
{
  fn clone(&self) -> Self {
    let mut clone = Self::new(self.capacity);
    for item in self.iter() {
      clone.push(item.clone());
    }
    clone
  }
}

impl<T> Drop for Array<T> {
  fn drop(&mut self) {
    if std::mem::needs_drop::<T>() {
      (0..self.size).for_each(|idx| unsafe { self.slot_mut(idx).drop_in_place() });
    }

    let t_size = size_of::<T>();
    let t_align = align_of::<T>();
    unsafe {
      dealloc(
        self.els,
        Layout::from_size_align(t_size * self.capacity(), t_align).unwrap(),
      );
    }
  }
}

impl<T> Hash for Array<T>
where
  T: Hash,
{
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.size.hash(state);
    self.iter().for_each(|el| el.hash(state));
  }
}

impl<T> PartialEq for Array<T>
where
  T: PartialEq,
{
  fn eq(&self, other: &Self) -> bool {
    self.size == other.size && self.iter().zip(other.iter()).all(|(l, r)| l == r)
  }
}

impl<T> Eq for Array<T> where T: Eq {}

unsafe impl<T> Send for Array<T> where T: Send {}

unsafe impl<T> Sync for Array<T> where T: Sync {}

#[cfg(test)]
mod tests {
  use super::Array;

  #[test]
  fn test_nontrivial_destructor() {
    let mut a: Array<Vec<_>> = Array::new(8);

    (0..8).for_each(|idx| a.push((0..(idx * 10000 + 1)).collect()));
  }
}
