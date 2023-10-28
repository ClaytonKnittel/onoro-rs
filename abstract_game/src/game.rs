pub trait GameMoveGenerator: Sized {
  type Item;
  type Game;

  fn next(&mut self, game: &Self::Game) -> Option<Self::Item>;

  fn to_iter<'a>(self, game: &'a Self::Game) -> GameIterator<'a, Self, Self::Game> {
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

impl<'a, GI, I, G> Iterator for GameIterator<'a, GI, G>
where
  GI: GameMoveGenerator<Item = I, Game = G>,
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
  type MoveGenerator: GameMoveGenerator<Item = Self::Move, Game = Self>;
  type PlayerIdentifier: Eq;

  fn move_generator(&self) -> Self::MoveGenerator;
  fn each_move<'a>(&'a self) -> GameIterator<'a, Self::MoveGenerator, Self> {
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
}
