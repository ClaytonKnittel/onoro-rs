#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OnoroState {
  /// Layout of data:
  /// ```text
  /// [0..3]: turn
  /// [4]: black's turn?
  /// [5]: finished?
  /// [6-7]: unused
  /// ```
  ///
  /// Note: you can play this game with a max of 8 pawns, and turn count stops
  /// incrementing after the end of phase 1. This allows us to only use 4 bits
  /// for the turn counter.
  data: u8,
}

impl OnoroState {
  pub const fn new() -> Self {
    // Initialize turn to 0xf, so that after the first pawn is placed, it will
    // become 0.
    Self {
      data: Self::pack(0xf, true, false),
    }
  }

  pub const fn turn(&self) -> u32 {
    let (turn, _, _) = Self::unpack(self.data);
    turn
  }

  /// Increment the turn and swap which player's turn it is. This should be used
  /// only in phase 1, where the turn count increments.
  pub fn inc_turn(&mut self) {
    let (turn, black_turn, finished) = Self::unpack(self.data);
    self.data = Self::pack((turn + 1) & 0xf, !black_turn, finished);
  }

  pub const fn black_turn(&self) -> bool {
    let (_, black_turn, _) = Self::unpack(self.data);
    black_turn
  }

  /// Only swap which player's turn it is. This should be used in phase 2, when
  /// the turn stops incrementing.
  pub fn swap_player_turn(&mut self) {
    let (turn, black_turn, finished) = Self::unpack(self.data);
    debug_assert_eq!(turn, 0xf);
    self.data = Self::pack(turn, !black_turn, finished);
  }

  pub const fn finished(&self) -> bool {
    let (_, _, finished) = Self::unpack(self.data);
    finished
  }

  pub fn set_finished(&mut self, finished: bool) {
    let (turn, black_turn, _) = Self::unpack(self.data);
    self.data = Self::pack(turn, black_turn, finished);
  }

  const fn pack(turn: u32, black_turn: bool, finished: bool) -> u8 {
    debug_assert!(turn < 0x10);

    (turn | if black_turn { 0x10 } else { 0 } | if finished { 0x20 } else { 0 }) as u8
  }

  const fn unpack(data: u8) -> (u32, bool, bool) {
    let turn = (data & 0x0f) as u32;
    let black_turn = (data & 0x10) != 0;
    let finished = (data & 0x20) != 0;

    (turn, black_turn, finished)
  }
}
