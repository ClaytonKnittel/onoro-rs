pub struct PassThroughHasher {
  state: u64,
}

impl std::hash::Hasher for PassThroughHasher {
  fn write(&mut self, bytes: &[u8]) {
    debug_assert!(bytes.len() == 8 && self.state == 0);
    self.state = unsafe { *(bytes.as_ptr() as *const u64) };
  }

  fn finish(&self) -> u64 {
    self.state
  }
}

#[derive(Clone)]
pub struct BuildPassThroughHasher;

impl std::hash::BuildHasher for BuildPassThroughHasher {
  type Hasher = PassThroughHasher;
  fn build_hasher(&self) -> PassThroughHasher {
    PassThroughHasher { state: 0 }
  }
}
