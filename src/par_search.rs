use std::sync::Arc;

use onoro::{Move, Onoro16, OnoroView, Score, ScoreValue};
use rayon::prelude::*;

use crate::{metrics::Metrics, onoro_table::OnoroTable, search::find_best_move};

struct SearchWorker {
  table: Arc<OnoroTable>,
  metrics: Metrics,
}

impl SearchWorker {
  fn new(table: Arc<OnoroTable>) -> Self {
    Self {
      table,
      metrics: Metrics::new(),
    }
  }

  pub fn best_move(onoro: &Onoro16, depth: u32) -> (Option<Score>, Option<Move>, Metrics) {
    let mut s = Self::new(Arc::new(OnoroTable::new()));
    let (score, m) = s.best_move_impl(onoro, depth);
    (score, m, s.metrics)
  }

  fn best_move_impl<'a>(&mut self, onoro: &Onoro16, depth: u32) -> (Option<Score>, Option<Move>) {
    // Can't score games that are already over.
    debug_assert!(onoro.finished().is_none());
    debug_assert!(onoro.validate().map_err(|res| panic!("{}", res)).is_ok());

    let table = self.table.clone();

    if depth < 2 {
      return find_best_move(&onoro, depth, &mut self.metrics);
    }

    self.metrics.n_states += 1;

    if depth == 0 {
      self.metrics.n_leaves += 1;
      return (Some(Score::tie(0)), None);
    }

    // First, check if any move ends the game.
    for m in onoro.each_move() {
      let mut g = onoro.clone();
      g.make_move(m);
      if g.finished().is_some() {
        self.metrics.n_leaves += 1;
        return (Some(Score::win(1)), Some(m));
      }
    }

    let mut best_score = None;
    let mut best_move = None;
    self.metrics.n_misses += 1;

    // TODO: mark visited states as score = ancestor(), then skip over those
    // opportunistically until all children are being explored, then start
    // exploring the ancestors.
    for m in onoro.each_move().into_par_iter() {
      let mut g = onoro.clone();
      g.make_move(m);

      let mut view = OnoroView::new(g);

      let score = table
        .get(&view)
        .map(|view| view.onoro().score())
        .and_then(|score| {
          if score.determined(depth - 1) {
            self.metrics.n_states += 1;
            self.metrics.n_hits += 1;
            Some(score)
          } else {
            None
          }
        })
        .unwrap_or_else(|| {
          let (score, _) = self.best_move_impl(view.onoro(), depth - 1);
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
}
