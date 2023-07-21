use std::{cmp, fmt::Display};

use union_find::ConstUnionFind;

use crate::{make_onoro_error, util::broadcast_u8_to_u64};

use super::{
  error::{OnoroError, OnoroResult},
  hex_pos::{HexPos, HexPosOffset},
  onoro_state::OnoroState,
  packed_hex_pos::PackedHexPos,
  packed_idx::{IdxOffset, PackedIdx},
  packed_score::PackedScore,
  r#move::Move,
  score::Score,
};

/// For move generation, the number of bits to use per-tile (for counting
/// adjacencies).
const TILE_BITS: usize = 2;
const TILE_MASK: u64 = (1u64 << TILE_BITS) - 1;

/// The minimum number of neighbors each pawn must have.
const MIN_NEIGHBORS_PER_PAWN: u64 = 2;

const fn adjacency_count_size(n: usize) -> usize {
  (n * n * TILE_BITS + 63) / 64
}

#[macro_export]
macro_rules! onoro_type {
  ($n:literal) => {
    Onoro<$n, { $n * $n }, { adjacency_count_size($n) }>
  };
}

pub type Onoro8 = onoro_type!(8);
pub type Onoro16 = onoro_type!(16);

#[derive(Debug, PartialEq, Eq)]
pub enum TileState {
  Empty,
  Black,
  White,
}

/// An Onoro game state with `N / 2` pawns per player.
///
/// Note: All of `N`, the total number of pawns in the game, `N2`, the square of
/// `N`, and `ADJ_CNT_SIZE`, which depends on `N`, must be provided. This is due
/// to a limitation in the rust compiler, generic const expressions are still
/// experimental. See: https://github.com/rust-lang/rust/issues/76560.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Onoro<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> {
  /// Array of indexes of pawn positions. Odd entries (even index) are black
  /// pawns, the others are white. Filled from lowest to highest index as the
  /// first phase proceeds.
  pawn_poses: [PackedIdx; N],
  score: PackedScore<OnoroState>,
  // Sum of all HexPos's of pieces on the board
  sum_of_mass: PackedHexPos,
  hash: u64,
}

impl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> Onoro<N, N2, ADJ_CNT_SIZE> {
  /// Don't publicly expose the constructor, since it produces an invalid board
  /// state. Any constructor returning an owned instance of `Onoro` _must_ make
  /// at least one move after initializing an `Onoro` with this function.
  fn new() -> Self {
    Self {
      pawn_poses: [PackedIdx::null(); N],
      score: PackedScore::new(Score::tie(0), OnoroState::new()),
      sum_of_mass: HexPos::zero().into(),
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

  /// Converts a `HexPos` to an ordinal, which is a unique mapping from valid
  /// `HexPos`s on the board to the range 0..N2.
  pub const fn hex_pos_ord(pos: &HexPos) -> usize {
    pos.x() as usize + (pos.y() as usize) * N
  }

  /// The inverse of `self.hex_pos_ord`.
  pub const fn ord_to_hex_pos(ord: usize) -> HexPos {
    HexPos::new((ord % N) as u32, (ord / N) as u32)
  }

  /// If the game is finished, returns `Some(<player color who won>)`, or `None`
  /// if the game is not over yet.
  pub fn finished(&self) -> Option<PawnColor> {
    if self.onoro_state().finished() {
      if self.onoro_state().black_turn() {
        Some(PawnColor::White)
      } else {
        Some(PawnColor::Black)
      }
    } else {
      None
    }
  }

  pub fn pawns_in_play(&self) -> u32 {
    self.onoro_state().turn() + 1
  }

  pub fn pawns(&self) -> PawnIterator<'_, N, N2, ADJ_CNT_SIZE> {
    PawnIterator {
      onoro: self,
      pawn_idx: 0,
      one_color: false,
    }
  }

  pub fn color_pawns(&self, color: PawnColor) -> PawnIterator<'_, N, N2, ADJ_CNT_SIZE> {
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

  /// The color of the current player as a `PawnColor`.
  pub fn player_color(&self) -> PawnColor {
    if self.onoro_state().black_turn() {
      PawnColor::Black
    } else {
      PawnColor::White
    }
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
        self.place_pawn(pawn_idx, to);
      }
      Move::Phase2Move { to, from_idx } => {
        self.move_pawn(from_idx as usize, to);
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

  pub fn each_p1_move(&self) -> P1MoveIterator<'_, N, N2, ADJ_CNT_SIZE> {
    debug_assert!(self.in_phase1());
    P1MoveIterator {
      onoro: self,
      pawn_iter: self.pawns(),
      neighbor_iter: None,
      adjacency_counts: [0; ADJ_CNT_SIZE],
    }
  }

  pub fn each_p2_move(&self) -> P2MoveIterator<'_, N, N2, ADJ_CNT_SIZE> {
    debug_assert!(!self.in_phase1());
    P2MoveIterator::new(self)
  }

  /// Adds a new pawn to the game board at index `i`, without checking what was
  /// there before or verifying that `i` was the correct place to put the pawn.
  /// This will mutate the game state to accomodate the change.
  ///
  ///  Important: this will not update `self.onoro_state().turn()` or
  /// `self.onoro_state().black_turn()`, the caller is responsible for doing so.
  fn place_pawn(&mut self, i: usize, pos: PackedIdx) {
    unsafe {
      *self.pawn_poses.get_unchecked_mut(i) = pos;
    }

    self.sum_of_mass = (HexPos::from(self.sum_of_mass) + pos.into()).into();
    self.mut_onoro_state().set_hashed(false);
    self.adjust_to_new_pawn_and_check_win(pos);
  }

  /// Moves the pawn at index `i` to pos `pos`, mutating the game state to
  /// accomodate the change.
  ///
  ///  Important: this will not update `self.onoro_state().turn()` or
  /// `self.onoro_state().black_turn()`, the caller is responsible for doing so.
  fn move_pawn(&mut self, i: usize, pos: PackedIdx) {
    let mut com_offset: HexPosOffset = pos.into();

    let prev_idx = unsafe { *self.pawn_poses.get_unchecked(i) };
    debug_assert_ne!(prev_idx, PackedIdx::null());
    com_offset -= prev_idx.into();

    unsafe {
      *self.pawn_poses.get_unchecked_mut(i) = pos;
    }

    self.sum_of_mass = (HexPos::from(self.sum_of_mass) + com_offset).into();
    self.mut_onoro_state().set_hashed(false);
    self.adjust_to_new_pawn_and_check_win(pos);
  }

  /// Adjust the game state to accomodate a new pawn at position `pos`. This may
  /// shift all pawns on the board. This will also check if the new pawn has
  /// caused the current player to win, and set onoro_state().finished if they
  /// have.
  fn adjust_to_new_pawn_and_check_win(&mut self, pos: PackedIdx) {
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

  fn check_win(&self, last_move: HexPos) -> bool {
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
      let pos: HexPos = unsafe { *self.pawn_poses.get_unchecked(i) }.into();
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
      let xor_search = mask ^ unsafe { *pawn_poses_ptr.add(i) };

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
      if unsafe { *self.pawn_poses.get_unchecked(i) } == idx {
        if i % 2 == 0 {
          return TileState::Black;
        } else {
          return TileState::White;
        }
      }
    }

    TileState::Empty
  }

  pub fn validate(&self) -> OnoroResult<()> {
    let mut n_b_pawns = 0u32;
    let mut n_w_pawns = 0u32;
    let mut sum_of_mass = HexPos::zero();

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

      HexPos::from(pawn.pos)
        .each_top_left_neighbor()
        .for_each(|neighbor_pos| {
          if self.get_tile(neighbor_pos.into()) != TileState::Empty {
            uf.union(
              Self::hex_pos_ord(&HexPos::from(pawn.pos)),
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

    if sum_of_mass != self.sum_of_mass.into() {
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

impl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> Display
  for Onoro<N, N2, ADJ_CNT_SIZE>
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    if self.onoro_state().black_turn() {
      writeln!(f, "black:")?;
    } else {
      writeln!(f, "white:")?;
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
        writeln!(f)?;
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
  board_idx: u8,
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
      HexPos::from(self.pos)
    )
  }
}

pub struct PawnIterator<'a, const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> {
  onoro: &'a Onoro<N, N2, ADJ_CNT_SIZE>,
  pawn_idx: usize,
  /// If true, only iterates over pawns of one color, otherwise iterating over
  /// all pawns.
  one_color: bool,
}

impl<'a, const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> Iterator
  for PawnIterator<'a, N, N2, ADJ_CNT_SIZE>
{
  type Item = Pawn;

  fn next(&mut self) -> Option<Self::Item> {
    if self.pawn_idx >= self.onoro.pawns_in_play() as usize {
      return None;
    }

    let pawn = Pawn {
      pos: unsafe { *self.onoro.pawn_poses.get_unchecked(self.pawn_idx) },
      color: if self.pawn_idx % 2 == 0 {
        PawnColor::Black
      } else {
        PawnColor::White
      },
      board_idx: self.pawn_idx as u8,
    };
    self.pawn_idx += if self.one_color { 2 } else { 1 };

    Some(pawn)
  }
}

pub struct P1MoveIterator<'a, const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> {
  onoro: &'a Onoro<N, N2, ADJ_CNT_SIZE>,
  pawn_iter: PawnIterator<'a, N, N2, ADJ_CNT_SIZE>,
  neighbor_iter: Option<std::array::IntoIter<HexPos, 6>>,

  /// Bitvector of 2-bit numbers per tile in the whole game board. Each number
  /// is the number of neighbors a pawn has, capping out at 2.
  adjacency_counts: [u64; ADJ_CNT_SIZE],
}

impl<'a, const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> Iterator
  for P1MoveIterator<'a, N, N2, ADJ_CNT_SIZE>
{
  type Item = Move;

  fn next(&mut self) -> Option<Self::Item> {
    loop {
      if let Some(neighbor) = self.neighbor_iter.as_mut().and_then(|iter| iter.next()) {
        if self.onoro.get_tile(neighbor.into()) != TileState::Empty {
          continue;
        }

        let ord = Onoro::<N, N2, ADJ_CNT_SIZE>::hex_pos_ord(&neighbor);
        let tb_shift = TILE_BITS * (ord % (64 / TILE_BITS));
        let tbb = unsafe { *self.adjacency_counts.get_unchecked(ord / (64 / TILE_BITS)) };
        let mask = TILE_MASK << tb_shift;
        let full_mask = (MIN_NEIGHBORS_PER_PAWN + 1) << tb_shift;

        if (tbb & mask) != full_mask {
          let tbb = tbb + (1u64 << tb_shift);
          unsafe {
            *self
              .adjacency_counts
              .get_unchecked_mut(ord / (64 / TILE_BITS)) = tbb;
          }

          if (tbb & mask) == full_mask {
            return Some(Move::Phase1Move {
              to: neighbor.into(),
            });
          }
        }
      } else if let Some(pawn) = self.pawn_iter.next() {
        self.neighbor_iter = Some(HexPos::from(pawn.pos).each_neighbor());
      } else {
        return None;
      }
    }
  }
}

struct P2PawnMeta<const N2: usize> {
  uf: ConstUnionFind<N2>,
  /// The index of the pawn being considered in `onoro.pawn_poses`.
  pawn_idx: usize,
  /// The position of the pawn being considered on the board.
  pawn_pos: PackedIdx,
  /// The number of neighbors with only one neighbor after this pawn is removed.
  /// After placing this pawn, there must be exactly `neighbors_to_satisfy`
  /// neighbors with one other neighbor, otherwise the move would have left some
  /// pawns stranded with only one neighbor.
  neighbors_to_satisfy: u32,
  /// The number of disjoint groups of pawns after removing this pawn.
  pawn_groups: u32,
  /// The index after the index into `adjacency_counts` that `adj_cnt_bitmask`
  /// was read from.
  adj_cnt_idx: usize,
  /// A local copy of `adjacency_counts[adj_cnt_idx - 1]`, which is cleared out as
  /// locations to place the pawn are considered.
  adj_cnt_bitmask: u64,
}

pub struct P2MoveIterator<'a, const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> {
  onoro: &'a Onoro<N, N2, ADJ_CNT_SIZE>,
  /// The current pawn that is being considered for moving. Only iterates over
  /// the pawns of the current player.
  pawn_iter: PawnIterator<'a, N, N2, ADJ_CNT_SIZE>,
  pawn_meta: Option<P2PawnMeta<N2>>,

  /// Bitvector of 2-bit numbers per tile in the whole game board. Each number
  /// is the number of neighbors a pawn has, capping out at 2.
  adjacency_counts: [u64; ADJ_CNT_SIZE],
}

impl<'a, const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize>
  P2MoveIterator<'a, N, N2, ADJ_CNT_SIZE>
{
  fn new(onoro: &'a Onoro<N, N2, ADJ_CNT_SIZE>) -> Self {
    Self {
      onoro,
      pawn_iter: onoro.color_pawns(onoro.player_color()),
      pawn_meta: None,
      adjacency_counts: [0; ADJ_CNT_SIZE],
    }
    .populate_neighbor_counts()
  }

  fn populate_neighbor_counts(mut self) -> Self {
    for pawn in self.onoro.pawns() {
      for neighbor in HexPos::from(pawn.pos).each_neighbor() {
        let ord = Onoro::<N, N2, ADJ_CNT_SIZE>::hex_pos_ord(&neighbor);
        let tb_shift = TILE_BITS * (ord % (64 / TILE_BITS));
        let tbb = unsafe { *self.adjacency_counts.get_unchecked(ord / (64 / TILE_BITS)) };
        let mask = TILE_MASK << tb_shift;
        let full_mask = (MIN_NEIGHBORS_PER_PAWN + 1) << tb_shift;

        if (tbb & mask) != full_mask {
          let tbb = tbb + (1u64 << tb_shift);
          unsafe {
            *self
              .adjacency_counts
              .get_unchecked_mut(ord / (64 / TILE_BITS)) = tbb;
          }
        }
      }
    }
    self
  }

  /// Prepares the iterator to consider all possible moves of the pawn at
  /// `pawn_pos`. Will update `self` with `Some` `pawn_meta`, and will decrease
  /// the adjacency count of all neighboring pawns of the one at `pawn_pos`.
  fn prepare_move_pawn(&mut self, pawn_idx: usize, pawn_pos: PackedIdx) {
    let mut uf = ConstUnionFind::new();
    let pawn_hex_pos: HexPos = pawn_pos.into();

    // Calculate the number of disjoint pawn groups after removing the pawn at
    // next_idx
    for pawn in self.onoro.pawns() {
      // Skip ourselves.
      if pawn.pos == pawn_pos {
        continue;
      }
      let pawn_ord = Onoro::<N, N2, ADJ_CNT_SIZE>::hex_pos_ord(&pawn.pos.into());

      for neighbor in HexPos::from(pawn.pos).each_top_left_neighbor() {
        if self.onoro.get_tile(neighbor.into()) != TileState::Empty && pawn_hex_pos != neighbor {
          uf.union(
            pawn_ord,
            Onoro::<N, N2, ADJ_CNT_SIZE>::hex_pos_ord(&neighbor),
          );
        }
      }
    }

    let empty_tiles =
      Onoro::<N, N2, ADJ_CNT_SIZE>::board_size() as u32 - self.onoro.pawns_in_play();
    // Note: the pawn we are moving is its own group.
    let pawn_groups = uf.unique_sets() as u32 - empty_tiles - 1;

    // number of neighbors with 1 neighbor after removing this piece
    let mut neighbors_to_satisfy = 0;
    // decrease neighbor count of all neighbors
    for neighbor in HexPos::from(pawn_pos).each_neighbor() {
      let neighbor_ord = Onoro::<N, N2, ADJ_CNT_SIZE>::hex_pos_ord(&neighbor);
      let tb_idx = neighbor_ord / (64 / TILE_BITS);
      let tb_shift = TILE_BITS * (neighbor_ord % (64 / TILE_BITS));

      unsafe {
        *self.adjacency_counts.get_unchecked_mut(tb_idx) -= 1u64 << tb_shift;
      }
      // If this neighbor has only one neighbor itself now, and it isn't empty,
      // we have to place our pawn next to it.
      if ((unsafe { *self.adjacency_counts.get_unchecked(tb_idx) } >> tb_shift) & TILE_MASK) == 1
        && self.onoro.get_tile(neighbor.into()) != TileState::Empty
      {
        neighbors_to_satisfy += 1;
      }
    }

    self.pawn_meta = Some(P2PawnMeta {
      uf,
      pawn_idx,
      pawn_pos,
      neighbors_to_satisfy,
      pawn_groups,
      adj_cnt_idx: 0,
      adj_cnt_bitmask: 0,
    });
  }

  /// Cleans up the mutated data in `self` from `prepare_move_pawn`.
  fn cleanup_pawn_visit(&mut self, pawn_pos: PackedIdx) {
    for neighbor in HexPos::from(pawn_pos).each_neighbor() {
      let neighbor_ord = Onoro::<N, N2, ADJ_CNT_SIZE>::hex_pos_ord(&neighbor);
      let tb_idx = neighbor_ord / (64 / TILE_BITS);
      let tb_shift = TILE_BITS * (neighbor_ord % (64 / TILE_BITS));

      unsafe {
        *self.adjacency_counts.get_unchecked_mut(tb_idx) += 1u64 << tb_shift;
      }
    }

    self.pawn_meta = None;
  }
}

impl<'a, const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> Iterator
  for P2MoveIterator<'a, N, N2, ADJ_CNT_SIZE>
{
  type Item = Move;

  fn next(&mut self) -> Option<Self::Item> {
    loop {
      if let Some(pawn_meta) = &mut self.pawn_meta {
        // If the adjacency counts mask is empty, we have run out of candidate
        // positions.
        if pawn_meta.adj_cnt_bitmask == 0 {
          if pawn_meta.adj_cnt_idx == ADJ_CNT_SIZE {
            // The whole board has been checked, move onto the next pawn.
            let pawn_pos = pawn_meta.pawn_pos;
            self.cleanup_pawn_visit(pawn_pos);
          } else {
            // Fetch the next array of positions from `adjacency_counts`.
            pawn_meta.adj_cnt_bitmask = self.adjacency_counts[pawn_meta.adj_cnt_idx];
            pawn_meta.adj_cnt_idx += 1;
          }
          continue;
        }

        // Find the next tile in adjacency_counts that isn't zero.
        let adjacency_counts_idx_off = (pawn_meta.adj_cnt_idx - 1) * (64 / TILE_BITS);
        let next_idx_ord_off = pawn_meta.adj_cnt_bitmask.trailing_zeros() / TILE_BITS as u32;
        let tb_shift = next_idx_ord_off * TILE_BITS as u32;
        let next_idx_ord = next_idx_ord_off as usize + adjacency_counts_idx_off;
        let clr_mask = TILE_MASK << tb_shift;

        // The tile we are considering placing a pawn at, which may be empty
        // and/or legal.
        let place_to_consider = Onoro::<N, N2, ADJ_CNT_SIZE>::ord_to_hex_pos(next_idx_ord);
        let place_to_consider_idx = PackedIdx::from(place_to_consider);

        // skip this tile if it isn't empty (this will also skip the piece's
        // old location since we haven't removed it, which we want)
        if self.onoro.get_tile(place_to_consider_idx) != TileState::Empty
          || ((pawn_meta.adj_cnt_bitmask >> tb_shift) & TILE_MASK) <= 1
        {
          pawn_meta.adj_cnt_bitmask &= !clr_mask;
          continue;
        }

        pawn_meta.adj_cnt_bitmask &= !clr_mask;

        let mut n_satisfied = 0;
        let mut g1 = None;
        let mut g2 = None;
        let mut groups_touching = 0;
        for neighbor in place_to_consider.each_neighbor() {
          if self.onoro.get_tile(neighbor.into()) == TileState::Empty {
            continue;
          }
          let neighbor_ord = Onoro::<N, N2, ADJ_CNT_SIZE>::hex_pos_ord(&neighbor);

          let tb_idx = neighbor_ord / (64 / TILE_BITS);
          let tb_shift = TILE_BITS * (neighbor_ord % (64 / TILE_BITS));
          if ((unsafe { *self.adjacency_counts.get_unchecked(tb_idx) } >> tb_shift) & TILE_MASK)
            == 1
          {
            n_satisfied += 1;
          }

          if neighbor != pawn_meta.pawn_pos.into() {
            let group_id = pawn_meta.uf.find(neighbor_ord);
            if Some(group_id) != g1 {
              if g1.is_none() {
                g1 = Some(group_id);
                groups_touching += 1;
              } else if Some(group_id) != g2 {
                g2 = Some(group_id);
                groups_touching += 1;
              }
            }
          }
        }

        if n_satisfied == pawn_meta.neighbors_to_satisfy && groups_touching == pawn_meta.pawn_groups
        {
          return Some(Move::Phase2Move {
            to: place_to_consider_idx,
            from_idx: pawn_meta.pawn_idx as u32,
          });
        }
      } else if let Some(pawn) = self.pawn_iter.next() {
        self.prepare_move_pawn(pawn.board_idx as usize, pawn.pos);
      } else {
        return None;
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::packed_idx::PackedIdx;

  use super::Onoro8;

  #[test]
  fn test_get_tile() {
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
