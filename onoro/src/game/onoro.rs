use std::{cmp, fmt::Display};

use num_traits::Signed;
use union_find::ConstUnionFind;

use crate::{make_onoro_error, util::broadcast_u8_to_u64};

use super::{
  error::{OnoroError, OnoroResult},
  hex_pos::{HexPos, HexPos16, HexPos32},
  onoro_state::OnoroState,
  packed_idx::{IdxOffset, PackedIdx},
  packed_score::PackedScore,
  r#move::Move,
  score::Score,
};

#[derive(Debug, PartialEq, Eq)]
pub enum TileState {
  Empty,
  Black,
  White,
}

/// An Onoro game state with `N / 2` pawns per player.
///
/// Note: both `N`, the total number of pawns in the game, and `N2`, the square
/// of `N`, must be provided. This is due to a limitation in the rust compiler,
/// generic const expressions are still experimental. See:
/// https://github.com/rust-lang/rust/issues/76560.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Onoro<const N: usize, const N2: usize> {
  /// Array of indexes of pawn positions. Odd entries (even index) are black
  /// pawns, the others are white. Filled from lowest to highest index as the
  /// first phase proceeds.
  pawn_poses: [PackedIdx; N],
  score: PackedScore<OnoroState>,
  // Sum of all HexPos's of pieces on the board
  sum_of_mass: HexPos16,
  hash: u64,
}

impl<const N: usize, const N2: usize> Onoro<N, N2> {
  /// Don't publicly expose the constructor, since it produces an invalid board
  /// state. Any constructor returning an owned instance of `Onoro` _must_ make
  /// at least one move after initializing an `Onoro` with this function.
  fn new() -> Self {
    Self {
      pawn_poses: [PackedIdx::null(); N],
      score: PackedScore::new(Score::tie(0), OnoroState::new()),
      sum_of_mass: HexPos16::origin(),
      hash: 0,
    }
  }

  pub fn default_start() -> Self {
    let mid_idx = ((Self::board_width() - 1) / 2) as u32;
    let mut game = Self::new();
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

  pub const fn hex_pos_ord(pos: &HexPos32) -> usize {
    pos.x() as usize + (pos.y() as usize) * N
  }

  pub fn pawns_in_play(&self) -> u32 {
    self.onoro_state().turn() + 1
  }

  pub fn pawns<'a>(&'a self) -> PawnIterator<'a, N, N2> {
    PawnIterator {
      onoro: self,
      pawn_idx: 0,
      one_color: false,
    }
  }

  pub fn color_pawns<'a>(&'a self, color: PawnColor) -> PawnIterator<'a, N, N2> {
    PawnIterator {
      onoro: self,
      pawn_idx: match color {
        PawnColor::Black => 0,
        PawnColor::White => 1,
      },
      one_color: true,
    }
  }

  pub fn score(&self) -> Score {
    self.score.score()
  }

  fn onoro_state(&self) -> &OnoroState {
    self.score.packed_data()
  }

  fn mut_onoro_state(&mut self) -> &mut OnoroState {
    self.score.mut_packed_data()
  }

  /// Returns the width of the game board. This is also the upper bound on the
  /// x and y coordinate values in PackedIdx.
  pub const fn board_width() -> usize {
    N
  }

  /// Returns the total number of tiles in the game board.
  pub const fn board_size() -> usize {
    Self::board_width() * Self::board_width()
  }

  pub const fn symm_state_table_width() -> usize {
    N
  }

  /// Returns the size of the symm state table, in terms of number of elements.
  pub const fn symm_state_table_size() -> usize {
    Self::symm_state_table_width() * Self::symm_state_table_width()
  }

  pub fn in_phase1(&self) -> bool {
    self.onoro_state().turn() < 0xf
  }

  unsafe fn make_move_unchecked(&mut self, m: Move) {
    match m {
      Move::Phase1Move { to } => {
        // Increment the turn first, so self.onoro_state().turn() is 0 for turn
        // 1.
        self.mut_onoro_state().inc_turn();
        let pawn_idx = self.onoro_state().turn() as usize;
        self.set_tile(pawn_idx, to);
      }
      Move::Phase2Move { to, from_idx } => {
        self.set_tile(from_idx as usize, to);
        self.mut_onoro_state().swap_player_turn();
      }
    }
  }

  pub fn make_move(&mut self, m: Move) {
    match m {
      Move::Phase1Move { to: _ } => {
        debug_assert!(self.in_phase1());
      }
      Move::Phase2Move { to: _, from_idx: _ } => {
        debug_assert!(!self.in_phase1());
      }
    }
    unsafe { self.make_move_unchecked(m) }
  }

  /// Sets the pawn at index `i` to `pos`. This will mutate the state of the
  /// game.
  ///
  /// Important: this will not update `self.onoro_state().turn()` or
  /// `self.onoro_state().black_turn()`, the caller is responsible for doing so.
  fn set_tile(&mut self, i: usize, pos: PackedIdx) {
    let mut com_offset: HexPos32 = pos.into();

    let prev_idx = self.pawn_poses[i];
    if prev_idx != PackedIdx::null() {
      com_offset -= prev_idx.into();
    }

    self.pawn_poses[i] = pos;
    // The amount to shift the whole board by. This will keep pawns off the
    // outer perimeter.
    let shift = Self::calc_move_shift(&pos);
    // Only shift the pawns if we have to, to avoid extra memory
    // reading/writing.
    if shift != IdxOffset::identity() {
      self.pawn_poses.iter_mut().for_each(|pos| {
        if *pos != PackedIdx::null() {
          *pos += shift;
        }
      });
    }

    self.sum_of_mass += com_offset.into();
    self.mut_onoro_state().set_hashed(false);

    // Check for a win
    let finished = self.check_win((pos + shift).into());
    self.mut_onoro_state().set_finished(finished);
  }

  /// Given the position of a newly placed/moved pawn, returns the offset to
  /// apply to all positions on the board.
  fn calc_move_shift(m: &PackedIdx) -> IdxOffset {
    let mut offset = IdxOffset::new(0, 0);

    if m.y() == 0 {
      offset = IdxOffset::new(0, 1);
    } else if m.y() == Self::board_width() as u32 - 1 {
      offset = IdxOffset::new(0, -1);
    }
    if m.x() == 0 {
      offset += IdxOffset::new(1, 0);
    } else if m.x() == Self::board_width() as u32 - 1 {
      offset += IdxOffset::new(-1, 0);
    }

    offset
  }

  fn check_win(&self, last_move: HexPos32) -> bool {
    // Bitvector of positions occupied by pawns of this color along the 3 lines
    // extending out from last_move. Intentionally leave a zero bit between each
    // of the 3 sets so they can't form a continuous string of 1's across
    // borders.
    // - s[0-15]: line running along the x-axis, with bit i corresponding to
    //     (x, i)
    // - s[17-32]: line running along the line x = y, with bit i corresponding to
    //     (x - min(x, y) + i, y - min(x, y) + i).
    // - s[34-49]: line running along the y-axis, with bit i corresponding to
    //     (i, y)
    // let mut s = (0x1u64 << last_move.x())
    //   | (0x20000u64 << cmp::min(last_move.x(), last_move.y()))
    //   | (0x400000000u64 << last_move.y());
    let mut s = 0;

    // Unsafe pawn iteration: rely on the fact that idx_t::null_idx() will not
    // complete a line in the first phase of the game (can't reach the border
    // without being able to move pawns), and for phase two, all pawns are
    // placed, so this is safe.
    for i in (0..N)
      // If it is currently the black player's turn, then white placed the last
      // piece at `last_move`, so check if white is winning. Otherwise, check if
      // black is winning.
      .skip(self.onoro_state().black_turn() as usize)
      .step_by(2)
    {
      let pos: HexPos32 = self.pawn_poses[i].into();
      let delta = pos - last_move;
      let dx = delta.x();
      let dy = delta.y();

      s |= if dy == 0 { 0x1u64 } else { 0 } << pos.x();
      s |= if dx == dy { 0x20000u64 } else { 0 } << cmp::min(pos.x(), pos.y());
      s |= if dx == 0 { 0x400000000u64 } else { 0 } << pos.y();
    }

    // Check if any 4 bits in a row are set:
    s = s & (s << 2);
    s = s & (s << 1);
    s != 0
  }

  /// Given a position on the board, returns the tile state of that position,
  /// i.e. the color of the piece on that tile, or `Empty` if no piece is there.
  ///
  /// TODO: perf benchmark this against `get_tile`.
  #[cfg(test)]
  fn get_tile_slow(&self, idx: PackedIdx) -> TileState {
    if idx == PackedIdx::null() {
      return TileState::Empty;
    }

    match self
      .pawn_poses
      .iter()
      .enumerate()
      .find(|(_, &pos)| pos == idx)
    {
      Some((idx, _)) => {
        if idx % 2 == 0 {
          TileState::Black
        } else {
          TileState::White
        }
      }
      None => TileState::Empty,
    }
  }

  /// Given a position on the board, returns the tile state of that position,
  /// i.e. the color of the piece on that tile, or `Empty` if no piece is there.
  fn get_tile(&self, idx: PackedIdx) -> TileState {
    if idx == PackedIdx::null() {
      return TileState::Empty;
    }

    let pawn_poses_ptr = self.pawn_poses.as_ptr() as *const u64;

    // Read the internal representation of `idx` as a `u8`, and spread it across
    // all 8 bytes of a `u64` mask.
    let mask = broadcast_u8_to_u64(unsafe { idx.bytes() });

    for i in 0..N / 8 {
      let xor_search = mask ^ unsafe { *pawn_poses_ptr.offset(i as isize) };

      let zero_mask =
        (xor_search.wrapping_sub(0x0101010101010101u64)) & !xor_search & 0x8080808080808080u64;
      if zero_mask != 0 {
        let set_bit_idx = zero_mask.trailing_zeros();
        // Find the parity of `set_bit_idx` / 8. Black has the even indices,
        // white has the odd.
        if (set_bit_idx & 0x8) == 0 {
          return TileState::Black;
        } else {
          return TileState::White;
        }
      }
    }

    // only necessary if NPawns not a multiple of eight
    for i in 8 * (N / 8)..N {
      if self.pawn_poses[i] == idx {
        if i % 2 == 0 {
          return TileState::Black;
        } else {
          return TileState::White;
        }
      }
    }

    return TileState::Empty;
  }

  pub fn validate(&self) -> OnoroResult<()> {
    let mut n_b_pawns = 0u32;
    let mut n_w_pawns = 0u32;
    let mut sum_of_mass = HexPos32::origin();

    let mut uf = ConstUnionFind::<N2>::new();

    for pawn in self.pawns() {
      sum_of_mass += pawn.pos.into();

      match pawn.color {
        PawnColor::Black => {
          n_b_pawns += 1;
        }
        PawnColor::White => {
          n_w_pawns += 1;
        }
      };

      match (self.get_tile(pawn.pos), &pawn.color) {
        (TileState::Black, PawnColor::Black) => {}
        (TileState::White, PawnColor::White) => {}
        (TileState::Empty, _) => {
          return Err(make_onoro_error!(
            "Unexpected empty tile found with `get_tile` at `pawn.pos` ({}) from pawn returned by iterator.",
            pawn
          ));
        }
        (get_tile_color, _) => {
          return Err(make_onoro_error!(
            "Mismatched tile colors for iterator pawn ({}), `get_tile` returns color {:?} at this position",
            pawn,
            get_tile_color
          ));
        }
      }

      // TODO: union find for contiguous group of pawns.
      HexPos32::from(pawn.pos)
        .each_top_left_neighbor()
        .for_each(|neighbor_pos| {
          if self.get_tile(neighbor_pos.into()) != TileState::Empty {
            uf.union(
              Self::hex_pos_ord(&HexPos32::from(pawn.pos)),
              Self::hex_pos_ord(&neighbor_pos),
            );
          }
        });
    }

    if n_b_pawns + n_w_pawns != self.pawns_in_play() {
      return Err(make_onoro_error!(
        "Expected {} pawns in play, but found {}",
        self.pawns_in_play(),
        n_b_pawns + n_w_pawns
      ));
    }

    if self.in_phase1() && self.onoro_state().black_turn() as u32 != (self.onoro_state().turn() & 1)
    {
      return Err(make_onoro_error!(
        "Expected black turn to be {}, but was {}",
        self.onoro_state().turn() & 1,
        self.onoro_state().black_turn()
      ));
    }

    if n_b_pawns
      != n_w_pawns
        + if self.onoro_state().black_turn() {
          0
        } else {
          1
        }
    {
      return Err(make_onoro_error!(
        "Expected {} black pawns and {} white pawns, but found {} and {}",
        (self.pawns_in_play() + 1) / 2,
        self.pawns_in_play() / 2,
        n_b_pawns,
        n_w_pawns
      ));
    }

    if HexPos16::from(sum_of_mass) != self.sum_of_mass {
      return Err(make_onoro_error!(
        "Sum of mass not correct: expect {}, but have {}",
        sum_of_mass,
        self.sum_of_mass
      ));
    }

    let empty_tiles = Self::board_size() - self.pawns_in_play() as usize;
    let pawn_groups = uf.unique_sets() - empty_tiles;

    if pawn_groups != 1 {
      return Err(make_onoro_error!(
        "Expected 1 contiguous pawn group, but found {}",
        pawn_groups
      ));
    }

    Ok(())
  }
}

impl<const N: usize, const N2: usize> Display for Onoro<N, N2> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    if self.onoro_state().black_turn() {
      write!(f, "black:\n")?;
    } else {
      write!(f, "white:\n")?;
    }

    for y in (0..Self::board_width()).rev() {
      write!(f, "{: <width$}", "", width = Self::board_width() - y - 1)?;
      for x in 0..Self::board_width() {
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

      if y > 0 {
        write!(f, "\n")?;
      }
    }

    Ok(())
  }
}

#[derive(Debug, PartialEq, Eq)]
pub enum PawnColor {
  Black,
  White,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Pawn {
  pub pos: PackedIdx,
  pub color: PawnColor,
}

impl Display for Pawn {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "{} pawn at {}",
      match self.color {
        PawnColor::Black => "black",
        PawnColor::White => "white",
      },
      HexPos32::from(self.pos)
    )
  }
}

pub struct PawnIterator<'a, const N: usize, const N2: usize> {
  onoro: &'a Onoro<N, N2>,
  pawn_idx: usize,
  /// If true, only iterates over pawns of one color, otherwise iterating over
  /// all pawns.
  one_color: bool,
}

impl<'a, const N: usize, const N2: usize> Iterator for PawnIterator<'a, N, N2> {
  type Item = Pawn;

  fn next(&mut self) -> Option<Self::Item> {
    if self.pawn_idx >= self.onoro.pawns_in_play() as usize {
      return None;
    }

    let pawn = Pawn {
      pos: self.onoro.pawn_poses[self.pawn_idx],
      color: if self.pawn_idx % 2 == 0 {
        PawnColor::Black
      } else {
        PawnColor::White
      },
    };
    self.pawn_idx += if self.one_color { 2 } else { 1 };

    Some(pawn)
  }
}

#[cfg(test)]
mod tests {
  use crate::{game::packed_idx::PackedIdx, Onoro};

  #[test]
  fn test_get_tile() {
    type Onoro8 = Onoro<8>;
    let onoro = Onoro8::default_start();

    for y in 0..Onoro8::board_width() {
      for x in 0..Onoro8::board_width() {
        assert_eq!(
          onoro.get_tile(PackedIdx::new(x as u32, y as u32)),
          onoro.get_tile_slow(PackedIdx::new(x as u32, y as u32))
        );
      }
    }
  }
}
