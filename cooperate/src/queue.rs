use std::{ptr::null_mut, sync::atomic::Ordering};

use seize::{AtomicPtr, Guard, Linked};

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

  pub fn push(&self, item: *mut Linked<Item>) {
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

  pub fn pop(&self, guard: &Guard<'_>) -> Option<*mut Linked<Item>> {
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

#[cfg(test)]
mod tests {
  use std::{ptr::null_mut, sync::Arc, thread};

  use seize::{Collector, Linked};

  use super::{Queue, QueueItem};

  struct TestItem {
    val: u64,
    next: *mut Linked<Self>,
  }

  impl TestItem {
    fn new(val: u64) -> Self {
      Self {
        val,
        next: null_mut(),
      }
    }
  }

  impl QueueItem for TestItem {
    fn next(&self) -> *mut Linked<Self> {
      self.next
    }

    fn set_next(&mut self, next: *mut Linked<Self>) {
      self.next = next;
    }
  }

  #[test]
  fn test_queue_empty() {
    let q = Queue::<TestItem>::new();
    let collector = Collector::new();
    let guard = collector.enter();

    assert_eq!(q.pop(&guard), None);
  }

  #[test]
  fn test_queue() {
    let q = Queue::<TestItem>::new();
    let collector = Collector::new();
    let guard = collector.enter();

    let item1 = collector.link_boxed(TestItem::new(0));
    q.push(item1);
    assert_eq!(q.pop(&guard), Some(item1));
  }

  #[test]
  fn test_queue_mt() {
    let q = Arc::new(Queue::<TestItem>::new());
    let collector = Arc::new(Collector::new());
    const NUM_THREADS: u32 = 8;
    const NUM_ITERS: u32 = 50_000;

    let mut threads: Vec<_> = (0..NUM_THREADS)
      .map(|t_id| {
        let q = q.clone();
        let collector = collector.clone();
        thread::spawn(move || {
          for i in 0..NUM_ITERS {
            q.push(collector.link_boxed(TestItem::new((NUM_THREADS * i + t_id) as u64)));
          }
        })
      })
      .collect();

    while let Some(thread) = threads.pop() {
      thread.join().unwrap();
    }

    let guard = collector.enter();
    let mut last_vals: Vec<_> = (0..NUM_THREADS).map(|_| NUM_ITERS).collect();
    while let Some(test_item) = q.pop(&guard) {
      let val = unsafe { &*test_item }.val;
      let t_id = (val % NUM_THREADS as u64) as usize;
      let i = val / NUM_THREADS as u64;
      assert_eq!(last_vals[t_id], i as u32 + 1);
      last_vals[t_id] -= 1;
    }

    for i in 0..NUM_THREADS as usize {
      assert_eq!(last_vals[i], 0);
    }
  }
}
