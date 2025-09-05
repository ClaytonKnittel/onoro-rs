use crate::{Game, GameResult, Score, Solver};

pub struct SimpleSolver;

impl Solver for SimpleSolver {
  fn solve<G: Game>(&mut self, game: &G, depth: u32) -> Score {
    debug_assert!(matches!(game.finished(), GameResult::NotFinished));
    if depth == 0 {
      return Score::no_info();
    }

    game
      .each_move()
      .map(|m| game.with_move(m))
      .map(|next_game| match next_game.finished() {
        GameResult::Win(player) => {
          if player == game.current_player() {
            Score::win(1)
          } else {
            Score::lose(1)
          }
        }
        GameResult::Tie => Score::guaranteed_tie(),
        GameResult::NotFinished => self.solve(&next_game, depth - 1),
      })
      .max()
      .unwrap_or(Score::lose(1))
  }
}
