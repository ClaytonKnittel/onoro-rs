use itertools::interleave;

use crate::{
  error::OnoroError,
  hex_pos::HexPosOffset,
  onoro_util::{pawns_from_board_string, BoardLayoutPawns},
  Move, PackedIdx, Pawn, PawnColor, TileState,
};

pub trait Onoro: Sized {
  /// Initializes an empty game. This should not be called outside the `Onoro`
  /// trait.
  ///
  /// # Safety
  ///
  /// Any constructor returning an owned instance of `Onoro` _must_ make at
  /// least one move after initializing an `Onoro` with this function.
  unsafe fn new() -> Self;

  /// Returns the number of pawns each player has.
  fn pawns_per_player() -> usize;

  /// Returns the color of the player whose turn it is.
  fn turn(&self) -> PawnColor;

  /// Returns the number of pawns on the board currently.
  fn pawns_in_play(&self) -> u32;

  /// If the game is finished, returns `Some(<player color who won>)`, or `None`
  /// if the game is not over yet.
  fn finished(&self) -> Option<PawnColor>;

  /// Given a position on the board, returns the tile state of that position,
  /// i.e. the color of the piece on that tile, or `Empty` if no piece is there.
  fn get_tile(&self, idx: PackedIdx) -> TileState;

  /// Returns an iterator over all pawns in the game. The order does not matter.
  fn pawns(&self) -> impl Iterator<Item = Pawn> + '_;

  fn pawns_mathematica_list(&self) -> String {
    format!(
      "{{{}}}",
      self
        .pawns()
        .map(|pawn| format!("{{{},{}}}", pawn.pos.x(), pawn.pos.y()))
        .reduce(|acc, coord| acc + "," + &coord)
        .unwrap()
    )
  }

  /// Returns true if the game is in phase 1, meaning the move made by the next
  /// player is to place a new pawn on the board, not to move an existing pawn.
  fn in_phase1(&self) -> bool;

  /// Returns an iterator over all legal moves that can be made from this state.
  fn each_move(&self) -> impl Iterator<Item = Move>;

  /// Makes a move, mutating the game state.
  fn make_move(&mut self, m: Move);

  /// Make move without checking that we are in the right phase. This is used by
  /// the game constructors to place the first pawn on an empty board.
  ///
  /// # Safety
  ///
  /// This function should not be called outside the Onoro trait.
  unsafe fn make_move_unchecked(&mut self, m: Move) {
    self.make_move(m);
  }

  /// Returns the width of the game board, e.g. the maximum distance between
  /// two pawns in any legal board configuration.
  fn board_width() -> usize {
    2 * Self::pawns_per_player()
  }

  /// Returns the total number of tiles in the game board that would fit any
  /// legal configuration of pawns.
  fn board_size() -> usize {
    Self::board_width() * Self::board_width()
  }

  fn default_start() -> Self {
    let mid_idx = ((Self::board_width() - 1) / 2) as u32;
    let mut game = unsafe { Self::new() };
    unsafe {
      game.make_move_unchecked(Move::Phase1Move {
        to: PackedIdx::new(mid_idx, mid_idx),
      });
    }
    game.make_move(Move::Phase1Move {
      to: PackedIdx::new(mid_idx + 1, mid_idx + 1),
    });
    game.make_move(Move::Phase1Move {
      to: PackedIdx::new(mid_idx + 1, mid_idx),
    });
    game
  }

  fn from_board_string(board_layout: &str) -> Result<Self, OnoroError> {
    let BoardLayoutPawns {
      black_pawns,
      white_pawns,
    } = pawns_from_board_string(board_layout, Self::pawns_per_player())?;

    let mut game = unsafe { Self::new() };
    unsafe {
      game.make_move_unchecked(Move::Phase1Move { to: black_pawns[0] });
    }
    for pos in interleave(white_pawns, black_pawns.into_iter().skip(1)) {
      game.make_move(Move::Phase1Move { to: pos });
    }

    Ok(game)
  }

  fn from_pawns(mut pawns: Vec<(HexPosOffset, PawnColor)>) -> Result<Self, String> {
    let n_pawns = pawns.len();
    debug_assert!(n_pawns <= 2 * Self::pawns_per_player());
    let (min_x, min_y) = pawns
      .iter()
      .fold((i32::MAX, i32::MAX), |(min_x, min_y), (pos, _)| {
        (min_x.min(pos.x()), min_y.min(pos.y()))
      });

    if pawns.iter().any(|(pos, _)| {
      pos.x() - min_x >= Self::board_width() as i32 - 1
        || pos.y() - min_y >= Self::board_width() as i32 - 1
    }) {
      return Err("Pawns stretch beyond the maximum allowed size of the board, meaning this state is invalid.".to_owned());
    }

    let black_count = pawns
      .iter()
      .filter(|(_, color)| matches!(color, PawnColor::Black))
      .count();
    let white_count = n_pawns - black_count;
    if !((black_count - 1)..=black_count).contains(&white_count) {
      return Err(format!(
        "There must be either one fewer or equally many white pawns as there are black. Found {black_count} black and {white_count} white.",
      ));
    }

    // Move all black pawns to the front.
    pawns.sort_by_key(|(_, color)| matches!(color, PawnColor::White));
    for i in 0..(n_pawns - 1) / 2 {
      pawns.swap(2 * i + 1, n_pawns.div_ceil(2) + i);
    }
    debug_assert!(pawns
      .iter()
      .enumerate()
      .all(|(idx, (_, color))| { (idx % 2 == 0) == matches!(color, PawnColor::Black) }));

    Ok(Self::from_packed_idxs(pawns.into_iter().map(|(pos, _)| {
      PackedIdx::new((pos.x() - min_x + 1) as u32, (pos.y() - min_y + 1) as u32)
    })))
  }

  fn from_packed_idxs(pawns: impl IntoIterator<Item = PackedIdx>) -> Self {
    let mut game = unsafe { Self::new() };
    for idx in pawns {
      unsafe {
        game.make_move_unchecked(Move::Phase1Move { to: idx });
      }
    }
    game
  }

  fn hex_start() -> Self {
    Self::from_board_string(
      ". B W
        W . B
         B W .",
    )
    .unwrap()
  }

  fn display(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self.turn() {
      PawnColor::Black => writeln!(f, "black:")?,
      PawnColor::White => writeln!(f, "white:")?,
    }

    let ((min_x, min_y), (max_x, max_y)) = self.pawns().fold(
      ((Self::board_width(), Self::board_width()), (0, 0)),
      |((min_x, min_y), (max_x, max_y)), pawn| {
        (
          (
            min_x.min(pawn.pos.x() as usize),
            min_y.min(pawn.pos.y() as usize),
          ),
          (
            max_x.max(pawn.pos.x() as usize),
            max_y.max(pawn.pos.y() as usize),
          ),
        )
      },
    );

    let min_x = min_x.saturating_sub(1);
    let min_y = min_y.saturating_sub(1);
    let max_x = (max_x + 1).min(Self::board_width() - 1);
    let max_y = (max_y + 1).min(Self::board_width() - 1);

    for y in (min_y..=max_y).rev() {
      write!(f, "{: <width$}", "", width = max_y - y)?;
      for x in min_x..=max_x {
        write!(
          f,
          "{}",
          match self.get_tile(PackedIdx::new(x as u32, y as u32)) {
            TileState::Black => "B",
            TileState::White => "W",
            TileState::Empty => ".",
          }
        )?;

        if x < Self::board_width() - 1 {
          write!(f, " ")?;
        }
      }

      if y > min_y {
        writeln!(f)?;
      }
    }

    Ok(())
  }
}
