use onoro::{Compress, Onoro16View, error::OnoroResult};
use rand::{Rng, distr::Uniform};

fn ncr(n: u64, k: u64) -> u64 {
  debug_assert!(n > 0);
  debug_assert!(k <= n);

  fn ncrh(n: u64, k: u64) -> u64 {
    if k == 0 {
      1
    } else {
      (n * ncrh(n - 1, k - 1)) / k
    }
  }

  let k = if 2 * k > n { n - k } else { k };
  ncrh(n, k)
}

fn rand_fixed_bits(seed: u64, n: u64, k: u64) -> u64 {
  debug_assert!(k <= n);
  debug_assert!(n <= u64::BITS as u64);
  debug_assert!(seed < ncr(n, k));
  if n == k {
    (1u64 << n) - 1
  } else if k == 0 {
    0
  } else {
    let bound = ncr(n - 1, k);
    if seed < bound {
      rand_fixed_bits(seed, n - 1, k)
    } else {
      (1u64 << (n - 1)) | rand_fixed_bits(seed - bound, n - 1, k - 1)
    }
  }
}

struct RandomCompressedBoard<R: Rng> {
  rng: R,
  colors_distribution: Uniform<u64>,
  positions_distribution: Uniform<u64>,
}

impl<R: Rng> RandomCompressedBoard<R> {
  fn new(rng: R) -> OnoroResult<Self> {
    Ok(Self {
      rng,
      colors_distribution: Uniform::new(0, ncr(16, 8))?,
      positions_distribution: Uniform::new(0, ncr(45, 15))?,
    })
  }

  fn random_compressed_val(&mut self) -> u64 {
    let v1 = rand_fixed_bits(self.rng.sample(self.colors_distribution), 16, 8);
    let v2 = rand_fixed_bits(self.rng.sample(self.positions_distribution), 45, 15);
    v1 | (v2 << 16)
  }
}

fn main() {
  let mut rng = RandomCompressedBoard::new(rand::rng()).unwrap();

  const ITERS: u64 = 1_000_000_000;

  let onoro = (0..)
    .find_map(|_| Onoro16View::decompress(rng.random_compressed_val()).ok())
    .unwrap();
  println!("{onoro}");

  let count = (0..ITERS)
    .map(|_| rng.random_compressed_val())
    .filter_map(|val| Some((val, Onoro16View::decompress(val).ok()?)))
    .filter(|(val, onoro)| onoro.compress() == *val)
    .count();
  println!("{count} / {ITERS} ({})", count as f64 / ITERS as f64);
}
