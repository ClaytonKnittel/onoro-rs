pub struct OnoroState {
  /// Layout of data:
  /// [0..3]: turn
  /// [4]: black's turn?
  /// [5]: finished?
  /// [6]: hashed?
  /// [7]: unused
  ///
  /// Note: you can play this game with a max of 8 pawns, and turn count stops
  /// incrementing after the end of phase 1. This allows us to only use 4 bits
  /// for the turn counter.
  data: u8,
}

impl OnoroState {
  pub fn new() -> Self {
    Self {
      data: Self::pack(0, true, false, false),
    }
  }

  pub fn turn(&self) -> u32 {
    let (turn, _, _, _) = Self::unpack(self.data);
    turn
  }

  pub fn black_turn(&self) -> bool {
    let (_, black_turn, _, _) = Self::unpack(self.data);
    black_turn
  }

  pub fn finished(&self) -> bool {
    let (_, _, finished, _) = Self::unpack(self.data);
    finished
  }

  pub fn hashed(&self) -> bool {
    let (_, _, _, hashed) = Self::unpack(self.data);
    hashed
  }

  fn pack(turn: u32, black_turn: bool, finished: bool, hashed: bool) -> u8 {
    debug_assert!(turn < 0x10);

    (turn
      | if black_turn { 0x10 } else { 0 }
      | if finished { 0x20 } else { 0 }
      | if hashed { 0x40 } else { 0 }) as u8
  }

  fn unpack(data: u8) -> (u32, bool, bool, bool) {
    let turn = (data & 0x0f) as u32;
    let black_turn = (data & 0x10) != 0;
    let finished = (data & 0x20) != 0;
    let hashed = (data & 0x40) != 0;

    (turn, black_turn, finished, hashed)
  }
}
