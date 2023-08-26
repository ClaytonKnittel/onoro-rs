pub trait Game {
  type Move: Copy;
  type MoveIterator: Iterator<Item = Self::Move>;
  type PlayerIdentifier;

  fn each_move(&self) -> Self::MoveIterator;
  fn make_move(&mut self, m: Self::Move);

  /// Returns `Some(player_id)` if a player has won, otherwise `None` if no
  /// player has won yet.
  fn finished(&self) -> Option<Self::PlayerIdentifier>;
}
