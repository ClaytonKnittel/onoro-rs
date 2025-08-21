use itertools::Itertools;
use onoro::{
  Onoro,
  error::{OnoroError, OnoroResult},
  hex_pos::HexPos,
};
use rand::Rng;

use crate::{Move, Onoro16, OnoroImpl};

pub trait CheckWinBenchmark {
  fn bench_check_win(&self, last_move: HexPos) -> bool;
}

impl<const N: usize> CheckWinBenchmark for OnoroImpl<N> {
  fn bench_check_win(&self, last_move: HexPos) -> bool {
    self.check_win(last_move)
  }
}

pub fn make_random_move<R: Rng>(onoro: &mut Onoro16, rng: &mut R) -> Move {
  let mut moves = onoro.each_move().collect_vec();
  moves.sort_by(|&m1, &m2| match (m1, m2) {
    (Move::Phase1Move { to: to1 }, Move::Phase1Move { to: to2 }) => {
      (to1.x(), to1.y()).cmp(&(to2.x(), to2.y()))
    }

    (
      Move::Phase2Move {
        to: to1,
        from_idx: from1,
      },
      Move::Phase2Move {
        to: to2,
        from_idx: from2,
      },
    ) => (to1.x(), to1.y())
      .cmp(&(to2.x(), to2.y()))
      .then(from1.cmp(&from2)),

    // All moves should be in the same phase.
    _ => unreachable!(),
  });
  assert!(
    !moves.is_empty(),
    "No moves available in position:\n{onoro:?}"
  );
  let m = moves[rng.gen_range(0..moves.len())];
  onoro.make_move(m);
  m
}

/// Plays a random number of moves in the game, returning the number of moves
/// played until the game finished. If the game did not finish, returns
/// `num_moves + 1`.
pub fn random_playout<R: Rng>(onoro: &mut Onoro16, num_moves: usize, rng: &mut R) -> usize {
  for i in 1..=num_moves {
    make_random_move(onoro, rng);
    if onoro.finished().is_some() {
      return i;
    }
  }

  num_moves + 1
}

pub fn generate_random_unfinished_states<R: Rng>(
  count: usize,
  num_moves: usize,
  rng: &mut R,
) -> OnoroResult<Vec<Onoro16>> {
  let mut states = Vec::with_capacity(count);

  let attempts = 100 * count;
  for _ in 0..attempts {
    let mut onoro = Onoro16::default_start();
    if random_playout(&mut onoro, num_moves, rng) > num_moves {
      states.push(onoro);
    }
    if states.len() == count {
      return Ok(states);
    }
  }

  Err(
    OnoroError::new(format!(
      "Failed to generate {count} random states with {num_moves} moves after {attempts} attempts"
    ))
    .into(),
  )
}

pub fn generate_random_walks<R: Rng>(
  initial_state: &Onoro16,
  count: usize,
  rng: &mut R,
) -> OnoroResult<Vec<Vec<Move>>> {
  const MAX_MOVES: usize = 1000;
  debug_assert!(initial_state.finished().is_none());

  (0..count)
    .map(|_| {
      let mut onoro = initial_state.clone();
      let mut moves = Vec::new();
      for _ in 0..MAX_MOVES {
        if onoro.finished().is_some() {
          return Ok(moves);
        }
        moves.push(make_random_move(&mut onoro, rng));
      }

      Err(
        OnoroError::new(format!(
          "Exceeded maximum number of moves {MAX_MOVES} without finishing the game."
        ))
        .into(),
      )
    })
    .collect()
}
