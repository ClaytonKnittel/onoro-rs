pub trait OnoroIterator: Sized {
  type Item;
  type Game;

  fn next(&mut self, game: &Self::Game) -> Option<Self::Item>;

  fn to_iter(self, game: &Self::Game) -> GameIterator<'_, Self, Self::Game> {
    GameIterator {
      game,
      game_iter: self,
    }
  }
}

pub struct GameIterator<'a, GI, G> {
  game: &'a G,
  game_iter: GI,
}

impl<GI, I, G> Iterator for GameIterator<'_, GI, G>
where
  GI: OnoroIterator<Item = I, Game = G>,
{
  type Item = I;

  fn next(&mut self) -> Option<Self::Item> {
    self.game_iter.next(self.game)
  }
}

#[derive(Debug, PartialEq, Eq)]
pub enum GameResult<PlayerIdentifier> {
  NotFinished,
  Win(PlayerIdentifier),
  Tie,
}

pub trait Game: Clone + Sized {
  type Move: Copy;
  type MoveGenerator: OnoroIterator<Item = Self::Move, Game = Self>;
  type PlayerIdentifier: Eq;

  fn move_generator(&self) -> Self::MoveGenerator;
  fn each_move(&self) -> GameIterator<'_, Self::MoveGenerator, Self> {
    self.move_generator().to_iter(self)
  }

  fn make_move(&mut self, m: Self::Move);

  /// Returns the `Self::PlayerIdentifier` of the player to make the next move.
  fn current_player(&self) -> Self::PlayerIdentifier;

  /// Returns `Some(player_id)` if a player has won, otherwise `None` if no
  /// player has won yet.
  fn finished(&self) -> GameResult<Self::PlayerIdentifier>;

  fn with_move(&self, m: Self::Move) -> Self {
    let mut copy = self.clone();
    copy.make_move(m);
    copy
  }

  /// Checks each possible move of this game, and returns any move that is an
  /// immediate win for the current player, or `None` if no such move exists.
  fn search_immediate_win(&self) -> Option<Self::Move> {
    self
      .each_move()
      .find(|&m| self.with_move(m).finished() == GameResult::Win(self.current_player()))
  }
}
