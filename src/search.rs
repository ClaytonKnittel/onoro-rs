use std::{collections::HashSet, hash::Hash, sync::Arc, thread};

use abstract_game::{Score, ScoreValue};
use cooperate::Metrics;
use onoro::{Move, Onoro16, Onoro16View, OnoroView};
use rand::{seq::SliceRandom, thread_rng};

use crate::{
  onoro_table::{BuildPassThroughHasher, OnoroTable},
  par_search_opts::ParSearchOptions,
};

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
      best_score = Some(score.break_early());
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

  // First, check if any move ends the game.
  for m in onoro.each_move() {
    let mut g = onoro.clone();
    g.make_move(m);
    if g.finished().is_some() {
      metrics.n_leaves += 1;
      return (Some(Score::win(1)), Some(m));
    }
  }

  let mut best_score = None;
  let mut best_move = None;
  metrics.n_misses += 1;

  // TODO: mark visited states as score = ancestor(), then skip over those
  // opportunistically until all children are being explored, then start
  // exploring the ancestors.
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

  // Randomize the order of the roots to minimize the chance of threads
  // exploring similar states.
  let mut rng = thread_rng();
  roots.shuffle(&mut rng);
}

/// TODO: keep a stack (regular HashSet) of visited onoro states on a single
/// thread to prevent repeating moves. Will need hash/eq on game state, not
/// rotationally invariant.
pub fn find_best_move_par_old(
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
