use std::fmt::{Debug, Display};

use crate::util::{max_u32, min_u32};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScoreValue {
  CurrentPlayerWins,
  OtherPlayerWins,
  Tie,
}

#[derive(Clone, Debug)]
pub struct Score {
  pub(crate) data: (u16, u8),
}

impl Score {
  const MAX_WIN_DEPTH: u32 = 0x007f;
  const MAX_TIE_DEPTH: u32 = 0x0fff;

  pub const fn new(cur_player_wins: bool, turn_count_tie: u32, turn_count_win: u32) -> Self {
    Self {
      data: Self::pack(cur_player_wins, turn_count_tie, turn_count_win),
    }
  }

  /// Construct a `Score` that contains no information.
  pub const fn no_info() -> Self {
    Self::tie(0)
  }

  /// Construct a `Score` for the current player winning in `turn_count_win`
  /// moves.
  pub const fn win(turn_count_win: u32) -> Self {
    Score::new(true, 0, turn_count_win)
  }

  /// Construct a `Score` for the current player losing in `turn_count_lose`
  /// moves.
  pub const fn lose(turn_count_lose: u32) -> Self {
    Score::new(false, 0, turn_count_lose)
  }

  /// Construct a `Score` for no possible forcing win in `turn_count_tie` moves.
  pub const fn tie(turn_count_tie: u32) -> Self {
    Score::new(false, turn_count_tie, 0)
  }

  /// Construct a `Score` for no possible forcing win in any number of moves
  /// into the future.
  pub const fn guaranteed_tie() -> Self {
    Score::new(false, Self::MAX_TIE_DEPTH, 0)
  }

  /// Used to mark a game state as an ancestor of the current tree being
  /// explored. Will be overwritten with the actual score once its calculation
  /// is finished.
  const fn ancestor() -> Self {
    // Mark the current player as winning with turn_count_win_ = 0, which is an
    // impossible state to be in.
    Self::new(true, 0, 0)
  }

  /// The maximum depth that this score is determined to.
  pub fn determined_depth(&self) -> u32 {
    self.turn_count_tie().max(self.turn_count_win())
  }

  /// The score of the game given `depth` moves to play.
  pub fn score_at_depth(&self, depth: u32) -> ScoreValue {
    if depth <= self.turn_count_tie() {
      ScoreValue::Tie
    } else if depth >= self.turn_count_win() {
      if self.cur_player_wins() {
        ScoreValue::CurrentPlayerWins
      } else {
        ScoreValue::OtherPlayerWins
      }
    } else {
      panic!("Attempted to resolve score at undiscovered depth");
    }
  }

  pub const fn cur_player_wins(&self) -> bool {
    let (wins, _, _) = Self::unpack(self.data);
    wins
  }

  pub const fn turn_count_tie(&self) -> u32 {
    let (_, turn_count_tie, _) = Self::unpack(self.data);
    turn_count_tie
  }

  pub const fn turn_count_win(&self) -> u32 {
    let (_, _, turn_count_win) = Self::unpack(self.data);
    turn_count_win
  }

  /// Transforms a score at a given state of the game to how that score would
  /// appear from the perspective of a game state one step before it.
  ///
  /// If a winning move for one player has been found in n steps, then it is
  /// turned into a winning move for the other player in n + 1 steps.
  pub fn backstep(&self) -> Self {
    let (mut cur_player_wins, mut turn_count_tie, mut turn_count_win) = Self::unpack(self.data);
    if turn_count_win > 0 {
      turn_count_win += 1;
      cur_player_wins = !cur_player_wins;
    }
    if turn_count_tie != Self::MAX_TIE_DEPTH {
      turn_count_tie += 1;
    }

    Score::new(cur_player_wins, turn_count_tie, turn_count_win)
  }

  /// Merges the information contained in another score into this one. This
  /// assumes that the scores are compatible, i.e. they don't contain
  /// conflicting information.
  pub const fn merge(&self, other: &Self) -> Self {
    let (cur_player_wins1, turn_count_tie1, turn_count_win1) = Self::unpack(self.data);
    let (cur_player_wins2, turn_count_tie2, turn_count_win2) = Self::unpack(other.data);

    let turn_count_win = min_u32(
      turn_count_win1.wrapping_sub(1),
      turn_count_win2.wrapping_sub(1),
    )
    .wrapping_add(1);
    let turn_count_tie = max_u32(turn_count_tie1, turn_count_tie2);
    let cur_player_wins = cur_player_wins1 || cur_player_wins2;

    Score::new(cur_player_wins, turn_count_tie, turn_count_win)
  }

  /// True if this score can be used in place of a search that goes
  /// `search_depth` moves deep (i.e. this score will equal the score calculated
  /// by a full search this deep).
  pub const fn determined(&self, search_depth: u32) -> bool {
    let (_, turn_count_tie, turn_count_win) = Self::unpack(self.data);
    (turn_count_win != 0 && search_depth >= turn_count_win) || search_depth <= turn_count_tie
  }

  /// Returns true if the two scores don't contain conflicting information, i.e.
  /// they are compatible. If true, the scores can be safely `merge`d.
  pub const fn compatible(&self, other: &Score) -> bool {
    let (cur_player_wins1, turn_count_tie1, turn_count_win1) = Self::unpack(self.data);
    let (cur_player_wins2, turn_count_tie2, turn_count_win2) = Self::unpack(other.data);

    let tc_win1 = if turn_count_win1 == 0 {
      u32::MAX
    } else {
      turn_count_win1
    };
    let tc_win2 = if turn_count_win2 == 0 {
      u32::MAX
    } else {
      turn_count_win2
    };
    let score1 = if turn_count_win1 == 0 {
      cur_player_wins2
    } else {
      cur_player_wins1
    };
    let score2 = if turn_count_win2 == 0 {
      cur_player_wins1
    } else {
      cur_player_wins2
    };

    tc_win1 > turn_count_tie2 && tc_win2 > turn_count_tie1 && score1 == score2
  }

  /// True if this score is better than `other` for the current player.
  pub const fn better(&self, other: &Score) -> bool {
    let (cur_player_wins1, turn_count_tie1, turn_count_win1) = Self::unpack(self.data);
    let (cur_player_wins2, turn_count_tie2, turn_count_win2) = Self::unpack(other.data);

    if turn_count_win2 != 0 {
      if cur_player_wins2 {
        // If both scores were wins, the better is the one with the shortest
        // path to victory.
        turn_count_win1 != 0 && cur_player_wins1 && turn_count_win1 < turn_count_win2
      } else {
        // If both scores are losses, the better is the one with the longest
        // path to losing.
        turn_count_win1 == 0 || cur_player_wins1 || turn_count_win1 > turn_count_win2
      }
    } else if turn_count_win1 != 0 {
      // If `other` is a tie and `this` is not, this is only better if it's a
      // win.
      cur_player_wins1
    } else {
      // If both scores were ties, the better is the score with the shortest
      // discovered tie depth.
      turn_count_tie1 < turn_count_tie2
    }
  }

  /// Constructs a score for a game state where not all possible next moves were
  /// explored. This sets `turn_count_tie` to 1, since we can't prove that there
  /// is no forced win out to any depth other than 1, since depth 1 is
  /// preemptively checked for immediate wins.
  pub fn break_early(&self) -> Self {
    debug_assert_ne!(self.turn_count_win(), 0);
    Score::new(self.cur_player_wins(), 1, self.turn_count_win())
  }

  const fn pack(cur_player_wins: bool, turn_count_tie: u32, turn_count_win: u32) -> (u16, u8) {
    debug_assert!(turn_count_tie < (1u32 << 12));
    debug_assert!(turn_count_win < (1u32 << 11));

    let a: u16 = (turn_count_tie | (turn_count_win << 12)) as u16;
    let b: u8 = ((turn_count_win >> 4) | if cur_player_wins { 0x80u32 } else { 0u32 }) as u8;
    (a, b)
  }

  const fn unpack((a, b): (u16, u8)) -> (bool, u32, u32) {
    let turn_count_tie = (a as u32) & Self::MAX_TIE_DEPTH;
    let turn_count_win = ((a as u32) >> 12) | ((b as u32) & Self::MAX_WIN_DEPTH);
    let cur_player_wins = ((b as u32) >> 7) != 0;

    (cur_player_wins, turn_count_tie, turn_count_win)
  }
}

impl PartialEq for Score {
  fn eq(&self, other: &Self) -> bool {
    self.data == other.data
  }
}

impl Eq for Score {}

impl Display for Score {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let (cur_player_wins, turn_count_tie, turn_count_win) = Self::unpack(self.data);

    if self == &Self::ancestor() {
      write!(f, "[ancestor]")
    } else if turn_count_win == 0 {
      if turn_count_tie == Self::MAX_TIE_DEPTH {
        write!(f, "[tie:∞]")
      } else {
        write!(f, "[tie:{turn_count_tie}]")
      }
    } else {
      write!(
        f,
        "[tie:{turn_count_tie},{}:{turn_count_win}]",
        if cur_player_wins { "cur" } else { "oth" }
      )
    }
  }
}
