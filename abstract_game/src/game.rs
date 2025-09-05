/// Arbitrary labels to assign to each of the two players of a game. `Player1`
/// does not need to be the first player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GamePlayer {
  Player1,
  Player2,
}

impl GamePlayer {
  pub fn is_p1(&self) -> bool {
    matches!(self, GamePlayer::Player1)
  }

  pub fn opposite(&self) -> Self {
    match self {
      Self::Player1 => Self::Player2,
      Self::Player2 => Self::Player1,
    }
  }
}

pub trait GameMoveIterator: Sized {
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
  GI: GameMoveIterator<Item = I, Game = G>,
{
  type Item = I;

  fn next(&mut self) -> Option<Self::Item> {
    self.game_iter.next(self.game)
  }
}

#[derive(Debug, PartialEq, Eq)]
pub enum GameResult {
  NotFinished,
  Win(GamePlayer),
  Tie,
}

pub trait Game: Clone + Sized {
  type Move: Copy;
  type MoveGenerator: GameMoveIterator<Item = Self::Move, Game = Self>;

  fn move_generator(&self) -> Self::MoveGenerator;
  fn each_move(&self) -> GameIterator<'_, Self::MoveGenerator, Self> {
    self.move_generator().to_iter(self)
  }

  fn make_move(&mut self, m: Self::Move);

  /// Returns the which player is to make the next move.
  fn current_player(&self) -> GamePlayer;

  /// Returns `Some(player_1_won)` if a player has won, otherwise `None` if no
  /// player has won yet.
  fn finished(&self) -> GameResult;

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
