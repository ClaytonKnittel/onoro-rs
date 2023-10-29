use std::{
  alloc::{alloc, Layout},
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
      _p: PhantomData::default(),
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
      *self.slot_mut(self.size) = el;
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
}
