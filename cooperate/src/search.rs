use abstract_game::{Game, Score, ScoreValue};

use crate::Metrics;

/// A serial, non-cached min-max search of the game state.
///
/// TODO: make this alpha-beta search.
pub fn find_best_move<G: Clone + Game>(
  onoro: &G,
  depth: u32,
  metrics: &mut Metrics,
) -> (Option<Score>, Option<G::Move>) {
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
