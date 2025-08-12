#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OnoroState {
  /// Layout of data:
  /// ```text
  /// [0]: black's turn?
  /// [1]: overflow for black's turn
  /// [2]: finished?
  /// [3]: unused
  /// [4..=7]: turn
  /// ```
  ///
  /// Note: you can play this game with a max of 8 pawns, and turn count stops
  /// incrementing after the end of phase 1. This allows us to only use 4 bits
  /// for the turn counter.
  data: u8,
}

impl OnoroState {
  const BLACK_TURN: u8 = 0x01;
  const FINISHED: u8 = 0x04;
  const TURN_MASK: u8 = 0xf0;
  const TURN_INC: u8 = 0x10;
  const DATA_MASK: u8 = Self::BLACK_TURN | Self::FINISHED | Self::TURN_MASK;

  pub const fn new() -> Self {
    // Initialize turn to 0xf, so that after the first pawn is placed, it will
    // become 0.
    Self {
      data: Self::TURN_MASK | Self::BLACK_TURN,
    }
  }

  pub const fn turn(&self) -> u32 {
    (self.data >> 4) as u32
  }

  /// Increment the turn and swap which player's turn it is. This should be used
  /// only in phase 1, where the turn count increments.
  pub fn inc_turn(&mut self) {
    self.data = self.data.wrapping_add(Self::TURN_INC | Self::BLACK_TURN) & Self::DATA_MASK;
  }

  pub const fn black_turn(&self) -> bool {
    (self.data & Self::BLACK_TURN) != 0
  }

  /// Only swap which player's turn it is. This should be used in phase 2, when
  /// the turn stops incrementing.
  pub fn swap_player_turn(&mut self) {
    debug_assert_eq!(self.turn(), 0xf);
    self.data ^= Self::BLACK_TURN;
  }

  pub const fn finished(&self) -> bool {
    (self.data & Self::FINISHED) != 0
  }

  pub const fn set_finished(&mut self, finished: bool) {
    debug_assert!(!self.finished());
    if finished {
      self.data |= Self::FINISHED;
    }
  }
}
