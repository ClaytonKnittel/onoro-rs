use std::{ptr::null_mut, sync::atomic::Ordering};

use seize::{AtomicPtr, Collector, Guard, Linked};

pub trait QueueItem: Sized {
  fn next(&self) -> *mut Linked<Self>;

  fn set_next(&mut self, next: *mut Linked<Self>);
}

/// TODO: actually make this a queue, not a stack.
pub struct Queue<Item> {
  head: AtomicPtr<Item>,
}

impl<Item> Queue<Item>
where
  Item: QueueItem,
{
  pub fn new() -> Self {
    Self {
      head: AtomicPtr::new(null_mut()),
    }
  }

  pub fn push(&mut self, mut item: *mut Linked<Item>) {
    let mut head = self.head.load(Ordering::Relaxed);
    unsafe {
      (*item).set_next(head);
    }
    while let Err(h) =
      self
        .head
        .compare_exchange_weak(head, item, Ordering::Release, Ordering::Relaxed)
    {
      head = h;
      unsafe {
        (*item).set_next(h);
      }
    }
  }

  pub fn pop(&mut self, guard: &Guard<'_>) -> Option<*mut Linked<Item>> {
    loop {
      let head = guard.protect(&self.head, Ordering::Acquire);

      if head.is_null() {
        return None;
      }

      let next = unsafe { (*head).next() };

      if self
        .head
        .compare_exchange_weak(head, next, Ordering::Release, Ordering::Relaxed)
        .is_ok()
      {
        unsafe {
          // Clear the next pointer before returning.
          (*head).set_next(null_mut());
        }
        return Some(head);
      }
    }
  }
}
