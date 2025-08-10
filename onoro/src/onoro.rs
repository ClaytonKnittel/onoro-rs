#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PawnColor {
  Black,
  White,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TileState {
  Empty,
  Black,
  White,
}

impl From<PawnColor> for TileState {
  fn from(value: PawnColor) -> Self {
    match value {
      PawnColor::Black => TileState::Black,
      PawnColor::White => TileState::White,
    }
  }
}

pub trait OnoroIndex {
  /// Constructs an index from raw coordinates. Will only be called when
  /// constructing a starting position.
  ///
  /// x and y will be within 0..n_pawns
  fn from_coords(x: u32, y: u32) -> Self;

  /// Returns the x-coordinate of the index. The value is only meaningful
  /// relative to other indexes.
  fn x(&self) -> i32;

  /// Returns the y-coordinate of the index. The value is only meaningful
  /// relative to other indexes.
  fn y(&self) -> i32;
}

pub trait OnoroMove<Index: OnoroIndex> {
  fn make_phase1(pos: Index) -> Self;
}

pub trait OnoroPawn<Index: OnoroIndex> {
  /// The position of this pawn on the board.
  fn pos(&self) -> Index;

  /// The color of this pawn.
  fn color(&self) -> PawnColor;
}

pub trait Onoro {
  type Index: OnoroIndex + Copy;
  type Move: OnoroMove<Self::Index>;
  type Pawn: OnoroPawn<Self::Index>;

  /// Returns the color of the player whose turn it is.
  fn turn(&self) -> PawnColor;

  /// Returns the number of pawns on the board currently.
  fn pawns_in_play(&self) -> u32;

  /// If the game is finished, returns `Some(<player color who won>)`, or `None`
  /// if the game is not over yet.
  fn finished(&self) -> Option<PawnColor>;

  /// Given a position on the board, returns the tile state of that position,
  /// i.e. the color of the piece on that tile, or `Empty` if no piece is there.
  fn get_tile(&self, idx: Self::Index) -> TileState;

  /// Returns an iterator over all pawns in the game. The order does not matter.
  fn pawns(&self) -> impl Iterator<Item = Self::Pawn> + '_;

  fn pawns_mathematica_list(&self) -> String {
    format!(
      "{{{}}}",
      self
        .pawns()
        .map(|pawn| format!("{{{},{}}}", pawn.pos().x(), pawn.pos().y()))
        .reduce(|acc, coord| acc + "," + &coord)
        .unwrap()
    )
  }

  /// Returns true if the game is in phase 1, meaning the move made by the next
  /// player is to place a new pawn on the board, not to move an existing pawn.
  fn in_phase1(&self) -> bool;

  /// Returns an iterator over all legal moves that can be made from this state.
  fn each_move(&self) -> impl Iterator<Item = Self::Move>;

  /// Makes a move, mutating the game state.
  fn make_move(&mut self, m: Self::Move);

  /// Make move without checking that we are in the right phase. This is used by
  /// the game constructors to place the first pawn on an empty board.
  ///
  /// # Safety
  ///
  /// This function should not be called outside the Onoro trait.
  unsafe fn make_move_unchecked(&mut self, m: Self::Move) {
    self.make_move(m);
  }
}
