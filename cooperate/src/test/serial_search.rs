use std::{
  fmt::Display,
  hash::{BuildHasher, Hash, RandomState},
};

use abstract_game::{Game, GameResult, Score, ScoreValue};

use crate::table::Table;

fn check_score<G, H>(game: G, score: Score, table: &Table<G, H>)
where
  G: Game + Hash + Eq,
  H: BuildHasher + Clone,
{
  if let Some(cached_score) = table.get(&game) {
    debug_assert!(cached_score.compatible(&score));
  }
  table.update(game, score);
}

/// A serial, non-cached min-max search of the game state.
///
/// TODO: make this alpha-beta search.
pub fn find_best_move_serial_table<G, H>(
  game: &G,
  depth: u32,
  table: &Table<G, H>,
) -> (Option<Score>, Option<G::Move>)
where
  G: Clone + Display + Game + Hash + Eq,
  H: BuildHasher + Clone,
{
  // Can't score games that are already over.
  debug_assert!(game.finished() == GameResult::NotFinished);

  if depth == 0 {
    return (Some(Score::no_info()), None);
  }

  if let Some(cached_score) = table.get(game) {
    if cached_score.determined(depth) {
      return (Some(cached_score), None);
    }
  }

  let mut best_score = None;
  let mut best_move = None;

  for m in game.each_move() {
    let mut g = game.clone();
    g.make_move(m);

    match g.finished() {
      GameResult::Win(player) => {
        if player == game.current_player() {
          check_score(game.clone(), Score::win(1), table);
          return (Some(Score::win(1)), Some(m));
        } else {
          check_score(game.clone(), Score::lose(1), table);
          return (Some(Score::lose(1)), Some(m));
        }
      }
      GameResult::Tie => {
        check_score(game.clone(), Score::tie(1), table);
        return (Some(Score::tie(1)), None);
      }
      GameResult::NotFinished => {}
    }

    let (score, _) = find_best_move_serial_table(&g, depth - 1, table);
    let score = match score {
      Some(score) => score.backstep(),
      // Consider winning by no legal moves as not winning until after the
      // other player's attempt at making a move, since all game states that
      // aren't explicitly winning are considered a tie.
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

  if let Some(ref score) = best_score {
    check_score(game.clone(), score.clone(), table);
  }
  (best_score, best_move)
}

pub fn find_best_move_serial<G>(
  game: &G,
  depth: u32,
) -> (Option<Score>, Option<G::Move>, Table<G, RandomState>)
where
  G: Display + Clone + Game + Hash + PartialEq + Eq,
{
  let table = Table::new();

  let (score, m) = find_best_move_serial_table(game, depth, &table);
  (score, m, table)
}
