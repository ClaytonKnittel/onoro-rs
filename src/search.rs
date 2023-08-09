use std::{
  collections::HashSet,
  hash::Hash,
  sync::{atomic::AtomicU64, Arc},
  thread,
};

use onoro::{Move, Onoro16, Onoro16View, OnoroView, Score, ScoreValue};

use crate::onoro_table::{BuildPassThroughHasher, OnoroTable};

#[derive(Clone, Debug, Default)]
pub struct Metrics {
  pub n_states: u64,
  pub n_misses: u64,
  pub n_hits: u64,
  pub n_leaves: u64,
}

impl std::ops::Add for Metrics {
  type Output = Self;

  fn add(self, rhs: Self) -> Self::Output {
    Self {
      n_states: self.n_states + rhs.n_states,
      n_misses: self.n_misses + rhs.n_misses,
      n_hits: self.n_hits + rhs.n_hits,
      n_leaves: self.n_leaves + rhs.n_leaves,
    }
  }
}

impl std::ops::AddAssign for Metrics {
  fn add_assign(&mut self, rhs: Self) {
    *self = self.clone() + rhs;
  }
}

pub fn find_best_move(
  onoro: &Onoro16,
  depth: u32,
  metrics: &mut Metrics,
) -> (Option<Score>, Option<Move>) {
  // Can't score games that are already over.
  debug_assert!(onoro.finished().is_none());

  metrics.n_states += 1;

  if depth == 0 {
    metrics.n_leaves += 1;
    return (Some(Score::tie(0)), None);
  }

  let mut best_score = None;
  let mut best_move = None;

  // First, check if any move ends the game.
  for m in onoro.each_move() {
    let mut g = onoro.clone();
    g.make_move(m);
    if g.finished().is_some() {
      metrics.n_leaves += 1;
      return (Some(Score::win(1)), Some(m));
    }
  }

  metrics.n_misses += 1;

  for m in onoro.each_move() {
    let mut g = onoro.clone();
    g.make_move(m);

    let (score, _) = find_best_move(&g, depth - 1, metrics);
    let score = match score {
      Some(score) => score.backstep(),
      // Consider winning by no legal moves as not winning until after the
      // other player's attempt at making a move, since all game states that
      // don't have 4 in a row of a pawn are considered a tie.
      None => Score::win(2),
    };

    match best_score.clone() {
      Some(best_score_val) => {
        if score.better(&best_score_val) {
          best_score = Some(score.clone());
          best_move = Some(m);
        }
      }
      None => {
        best_score = Some(score.clone());
        best_move = Some(m);
      }
    }

    // Stop the search early if there's already a winning move.
    if score.score_at_depth(depth) == ScoreValue::CurrentPlayerWins {
      break;
    }
  }

  (best_score, best_move)
}

pub fn find_best_move_table(
  onoro: &Onoro16,
  table: Arc<OnoroTable>,
  depth: u32,
  metrics: &mut Metrics,
) -> (Option<Score>, Option<Move>) {
  // Can't score games that are already over.
  debug_assert!(onoro.finished().is_none());
  debug_assert!(onoro.validate().map_err(|res| panic!("{}", res)).is_ok());

  if depth < 2 {
    return find_best_move(onoro, depth, metrics);
  }

  metrics.n_states += 1;

  if depth == 0 {
    metrics.n_leaves += 1;
    return (Some(Score::tie(0)), None);
  }

  let mut best_score = None;
  let mut best_move = None;

  // First, check if any move ends the game.
  for m in onoro.each_move() {
    let mut g = onoro.clone();
    g.make_move(m);
    if g.finished().is_some() {
      metrics.n_leaves += 1;
      return (Some(Score::win(1)), Some(m));
    }
  }

  metrics.n_misses += 1;

  for m in onoro.each_move() {
    let mut g = onoro.clone();
    g.make_move(m);

    let mut view = OnoroView::new(g);

    let score = table
      .get(&view)
      .map(|view| view.onoro().score())
      .and_then(|score| {
        if score.determined(depth - 1) {
          metrics.n_states += 1;
          metrics.n_hits += 1;
          Some(score)
        } else {
          None
        }
      })
      .unwrap_or_else(|| {
        let (score, _) = find_best_move_table(view.onoro(), table.clone(), depth - 1, metrics);
        let score = match score {
          Some(score) => score,
          // Consider winning by no legal moves as not winning until after the
          // other player's attempt at making a move, since all game states that
          // don't have 4 in a row of a pawn are considered a tie.
          None => Score::win(1),
        };

        view.mut_onoro().set_score(score.clone());
        table.update(&mut view);

        view.onoro().score()
      });

    let score = score.backstep();
    match best_score.clone() {
      Some(best_score_val) => {
        if score.better(&best_score_val) {
          best_score = Some(score.clone());
          best_move = Some(m);
        }
      }
      None => {
        best_score = Some(score.clone());
        best_move = Some(m);
      }
    }

    // Stop the search early if there's already a winning move.
    if score.score_at_depth(depth) == ScoreValue::CurrentPlayerWins {
      best_score = Some(score.break_early());
      break;
    }
  }

  (best_score, best_move)
}

#[derive(Clone, Copy)]
pub struct ParSearchOptions {
  pub n_threads: u32,
  pub unit_depth: u32,
}

impl ParSearchOptions {
  pub fn with_n_threads(&self, n_threads: u32) -> Self {
    Self { n_threads, ..*self }
  }

  pub fn with_unit_depth(&self, unit_depth: u32) -> Self {
    Self {
      unit_depth,
      ..*self
    }
  }
}

impl Default for ParSearchOptions {
  fn default() -> Self {
    Self {
      n_threads: 4,
      unit_depth: 3,
    }
  }
}

#[derive(Clone)]
struct ParUnit {
  view: Onoro16View,
  depth: u32,
}

impl Hash for ParUnit {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.view.hash(state);
  }
}

impl PartialEq for ParUnit {
  fn eq(&self, other: &Self) -> bool {
    self.view == other.view
  }
}

impl Eq for ParUnit {}

fn fill_queue(
  onoro: &Onoro16,
  queued_units: &mut HashSet<ParUnit, BuildPassThroughHasher>,
  depth: u32,
  total_depth: u32,
  options: ParSearchOptions,
  roots: &mut Vec<ParUnit>,
) {
  // Can't score games that are already over.
  debug_assert!(onoro.finished().is_none());
  debug_assert!(onoro.validate().map_err(|res| panic!("{}", res)).is_ok());

  if depth == total_depth.saturating_sub(options.unit_depth) {
    return;
  }

  for m in onoro.each_move() {
    let mut g = onoro.clone();
    g.make_move(m);

    if g.finished().is_some() {
      // Skip completed games.
      continue;
    }

    let view = OnoroView::new(g);
    let key = ParUnit { view, depth };

    if depth == total_depth.saturating_sub(options.unit_depth - 1) {
      if let Some(unit) = queued_units.take(&key) {
        queued_units.insert(ParUnit {
          view: unit.view,
          depth: depth.max(unit.depth),
        });
      } else {
        queued_units.insert(key.clone());
        roots.push(key);
      }
    } else {
      fill_queue(
        key.view.onoro(),
        queued_units,
        depth - 1,
        total_depth,
        options,
        roots,
      );
    }
  }
}

pub fn find_best_move_par(
  onoro: &Onoro16,
  table: Arc<OnoroTable>,
  depth: u32,
  options: ParSearchOptions,
  metrics: &mut Metrics,
) -> (Option<Score>, Option<Move>) {
  let mut roots = vec![];

  let mut queued_states = HashSet::with_hasher(BuildPassThroughHasher);
  fill_queue(onoro, &mut queued_states, depth, depth, options, &mut roots);
  debug_assert_eq!(queued_states.len(), roots.len());
  println!("Num states to explore: {}", queued_states.len());

  let (sender, receiver) = multiqueue::mpmc_queue::<ParUnit>(roots.len() as u64);
  // Populate the sender with all of the roots.
  roots
    .into_iter()
    .for_each(|root| sender.try_send(root).unwrap());

  let threads: Vec<_> = (0..options.n_threads)
    .map(|_| {
      let receiver = receiver.clone();
      let table = table.clone();
      thread::spawn(move || {
        let mut metrics = Metrics::default();
        for unit in receiver.try_iter() {
          find_best_move_table(
            unit.view.onoro(),
            table.clone(),
            depth.saturating_sub(options.unit_depth),
            &mut metrics,
          );
        }

        metrics
      })
    })
    .collect();

  threads
    .into_iter()
    .map(|thread| thread.join().unwrap())
    .for_each(|metric| {
      *metrics += metric;
    });

  find_best_move_table(onoro, table, depth, metrics)
}
