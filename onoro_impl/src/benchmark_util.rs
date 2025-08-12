use onoro::hex_pos::HexPos;

use crate::OnoroImpl;

pub trait CheckWinBenchmark {
  fn bench_check_win(&self, last_move: HexPos) -> bool;
}

impl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> CheckWinBenchmark
  for OnoroImpl<N, N2, ADJ_CNT_SIZE>
{
  fn bench_check_win(&self, last_move: HexPos) -> bool {
    self.check_win(last_move)
  }
}
