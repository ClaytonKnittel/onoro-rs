use onoro::hex_pos::HexPos;

use crate::OnoroImpl;

pub trait CheckWinBenchmark {
  fn bench_check_win(&self, last_move: HexPos) -> bool;
}

impl<const N: usize> CheckWinBenchmark for OnoroImpl<N> {
  fn bench_check_win(&self, last_move: HexPos) -> bool {
    self.check_win(last_move)
  }
}
