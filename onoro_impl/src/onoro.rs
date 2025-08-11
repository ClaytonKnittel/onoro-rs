use std::{
  cmp,
  fmt::{Debug, Display},
};

use abstract_game::{GameIterator, GameMoveGenerator};
use algebra::group::Group;
use itertools::interleave;
use onoro::{
  Color, Colored, Onoro, OnoroMoveWrapper, OnoroPawn, PawnColor, TileState,
  error::OnoroResult,
  groups::{C2, D3, D6, K4},
  hex_pos::{HexPos, HexPosOffset},
  make_onoro_error,
};
use union_find::ConstUnionFind;

use crate::{
  canonicalize::{BoardSymmetryState, board_symm_state},
  r#move::Move,
  onoro_state::OnoroState,
  packed_hex_pos::PackedHexPos,
  packed_idx::{IdxOffset, PackedIdx},
  util::broadcast_u8_to_u64,
};

/// For move generation, the number of bits to use per-tile (for counting
/// adjacencies).
pub(crate) const TILE_BITS: usize = 2;
const TILE_MASK: u64 = (1u64 << TILE_BITS) - 1;

/// The minimum number of neighbors each pawn must have.
const MIN_NEIGHBORS_PER_PAWN: u64 = 2;

/// An Onoro game state with `N / 2` pawns per player.
///
/// Note: All of `N`, the total number of pawns in the game, `N2`, the square of
/// `N`, and `ADJ_CNT_SIZE`, which depends on `N`, must be provided. This is due
/// to a limitation in the rust compiler, generic const expressions are still
/// experimental. See: https://github.com/rust-lang/rust/issues/76560.
#[derive(Clone)]
#[repr(align(8))]
pub struct OnoroImpl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> {
  /// Array of indexes of pawn positions. Odd entries (even index) are black
  /// pawns, the others are white. Filled from lowest to highest index as the
  /// first phase proceeds.
  pawn_poses: [PackedIdx; N],
  state: OnoroState,
  // Sum of all HexPos's of pieces on the board
  sum_of_mass: PackedHexPos,
}

impl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> OnoroImpl<N, N2, ADJ_CNT_SIZE> {
  /// Constructs an identical Onoro game rotated by `op`.
  fn rotated<G: Group, OpFn: FnMut(&HexPosOffset, &G) -> HexPosOffset>(
    &self,
    op: G,
    mut op_fn: OpFn,
  ) -> Self {
    let mut game = unsafe { Self::new() };

    let mut black_pawns = Vec::new();
    let mut white_pawns = Vec::new();
    let symm_state = board_symm_state(self);
    let origin = self.origin(&symm_state);
    let center = HexPos::new(N as u32 / 2, N as u32 / 2);
    for pawn in self.pawns() {
      let pos = HexPos::from(pawn.pos) - origin;
      let pos = op_fn(&pos, &op);

      match pawn.color {
        PawnColor::Black => {
          black_pawns.push(pos + center);
        }
        PawnColor::White => {
          white_pawns.push(pos + center);
        }
      }
    }

    unsafe {
      game.make_move_unchecked(Move::Phase1Move {
        to: black_pawns[0].into(),
      });
    }
    for pos in interleave(white_pawns, black_pawns.into_iter().skip(1)) {
      game.make_move(Move::Phase1Move { to: pos.into() })
    }

    if !self.in_phase1() && !self.onoro_state().black_turn() {
      game.mut_onoro_state().swap_player_turn();
    }

    game
  }

  pub fn rotated_d6_c(&self, op: D6) -> Self {
    self.rotated(op, HexPosOffset::apply_d6_c)
  }

  pub fn rotated_d3_v(&self, op: D3) -> Self {
    self.rotated(op, HexPosOffset::apply_d3_v)
  }

  pub fn rotated_k4_e(&self, op: K4) -> Self {
    self.rotated(op, HexPosOffset::apply_k4_e)
  }

  pub fn rotated_c2_cv(&self, op: C2) -> Self {
    self.rotated(op, HexPosOffset::apply_c2_cv)
  }

  pub fn rotated_c2_ce(&self, op: C2) -> Self {
    self.rotated(op, HexPosOffset::apply_c2_ce)
  }

  pub fn rotated_c2_ev(&self, op: C2) -> Self {
    self.rotated(op, HexPosOffset::apply_c2_ev)
  }

  pub fn print_with_move(&self, m: Move) -> String {
    let mut g = self.clone();
    g.make_move(m);

    let pawn_idx = match m {
      Move::Phase1Move { to: _ } => self.pawns_in_play(),
      Move::Phase2Move { to: _, from_idx } => from_idx,
    };

    let mut res = String::new();

    let ((min_x, min_y), (max_x, max_y)) = g.pawns().fold(
      ((N, N), (0, 0)),
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
    let max_x = (max_x + 1).min(N - 1);
    let max_y = (max_y + 1).min(N - 1);

    for y in (min_y..=max_y).rev() {
      res = format!("{res}{: <width$}", "", width = max_y - y);
      for x in min_x..=max_x {
        let pos = PackedIdx::new(x as u32, y as u32);
        let former_pawn_idx = self.get_pawn_idx(pos);
        let new_pawn_idx = g.get_pawn_idx(pos);

        res = format!(
          "{res}{}",
          match g.get_tile(pos) {
            TileState::Black =>
              if new_pawn_idx == Some(pawn_idx) {
                Colored::new("B", Color::Magenta)
              } else {
                "B".into()
              },
            TileState::White =>
              if new_pawn_idx == Some(pawn_idx) {
                Colored::new("W", Color::Magenta)
              } else {
                "W".into()
              },
            TileState::Empty =>
              if former_pawn_idx == Some(pawn_idx) {
                Colored::new(".", Color::Red)
              } else {
                ".".into()
              },
          }
        );

        if x < Self::board_width() - 1 {
          res = format!("{res} ");
        }
      }

      if y > min_y {
        res = format!("{res}\n");
      }
    }

    res
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

  pub fn pawns_gen(&self) -> PawnMoveGenerator<N, N2, ADJ_CNT_SIZE> {
    PawnMoveGenerator {
      pawn_idx: 0,
      one_color: false,
    }
  }

  pub fn pawns_typed(&self) -> GameIterator<'_, PawnMoveGenerator<N, N2, ADJ_CNT_SIZE>, Self> {
    self.pawns_gen().to_iter(self)
  }

  pub fn color_pawns_gen(&self, color: PawnColor) -> PawnMoveGenerator<N, N2, ADJ_CNT_SIZE> {
    PawnMoveGenerator {
      pawn_idx: match color {
        PawnColor::Black => 0,
        PawnColor::White => 1,
      },
      one_color: true,
    }
  }

  pub fn color_pawns_typed(
    &self,
    color: PawnColor,
  ) -> GameIterator<'_, PawnMoveGenerator<N, N2, ADJ_CNT_SIZE>, Self> {
    self.color_pawns_gen(color).to_iter(self)
  }

  pub fn color_pawns(&self, color: PawnColor) -> impl Iterator<Item = Pawn> + '_ {
    self.color_pawns_typed(color)
  }

  fn onoro_state(&self) -> &OnoroState {
    &self.state
  }

  fn mut_onoro_state(&mut self) -> &mut OnoroState {
    &mut self.state
  }

  /// The color of the current player as a `PawnColor`.
  pub fn player_color(&self) -> PawnColor {
    if self.onoro_state().black_turn() {
      PawnColor::Black
    } else {
      PawnColor::White
    }
  }

  pub fn sum_of_mass(&self) -> PackedHexPos {
    self.sum_of_mass
  }

  /// Returns the origin tile, which all group operations operate with respect
  /// to. This is orientation-invariant, meaning for any symmetry of this board
  /// state, the same origin tile will be chosen.
  pub fn origin(&self, symm_state: &BoardSymmetryState) -> HexPos {
    let x = self.sum_of_mass.x() as u32;
    let y = self.sum_of_mass.y() as u32;
    let truncated_com = HexPos::new(x / self.pawns_in_play(), y / self.pawns_in_play());
    truncated_com + symm_state.center_offset
  }

  pub const fn symm_state_table_width() -> usize {
    N
  }

  /// Returns the size of the symm state table, in terms of number of elements.
  pub const fn symm_state_table_size() -> usize {
    Self::symm_state_table_width() * Self::symm_state_table_width()
  }

  pub fn each_move_gen(&self) -> MoveGenerator<N, N2, ADJ_CNT_SIZE> {
    if self.in_phase1() {
      MoveGenerator::P1Moves(self.p1_move_gen())
    } else {
      MoveGenerator::P2Moves(self.p2_move_gen())
    }
  }

  fn p1_move_gen(&self) -> P1MoveGenerator<N, N2, ADJ_CNT_SIZE> {
    debug_assert!(self.in_phase1());
    P1MoveGenerator {
      pawn_iter: self.pawns_gen(),
      neighbor_iter: None,
      adjacency_counts: [0; ADJ_CNT_SIZE],
    }
  }

  fn p2_move_gen(&self) -> P2MoveGenerator<N, N2, ADJ_CNT_SIZE> {
    debug_assert!(!self.in_phase1());
    P2MoveGenerator::new(self)
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
    self.adjust_to_new_pawn_and_check_win(pos);
  }

  /// Adjust the game state to accomodate a new pawn at position `pos`. This may
  /// shift all pawns on the board. This will also check if the new pawn has
  /// caused the current player to win, and set onoro_state().finished if they
  /// have.
  fn adjust_to_new_pawn_and_check_win(&mut self, pos: PackedIdx) {
    // The amount to shift the whole board by. This will keep pawns off the
    // outer perimeter.
    let shift = Self::calc_move_shift(pos);
    // Only shift the pawns if we have to, to avoid extra memory
    // reading/writing.
    if shift != HexPosOffset::origin() {
      let idx_offset = IdxOffset::from(shift);
      self.pawn_poses.iter_mut().for_each(|pos| {
        if *pos != PackedIdx::null() {
          *pos += idx_offset;
        }
      });
      self.sum_of_mass =
        (HexPos::from(self.sum_of_mass) + shift * (self.pawns_in_play() as i32)).into();
    }

    // Check for a win
    let finished = self.check_win(HexPos::from(pos) + shift);
    self.mut_onoro_state().set_finished(finished);
  }

  /// Given the position of a newly placed/moved pawn, returns the offset to
  /// apply to all positions on the board.
  fn calc_move_shift(m: PackedIdx) -> HexPosOffset {
    let mut offset = HexPosOffset::new(0, 0);

    if m.y() == 0 {
      offset = HexPosOffset::new(0, 1);
    } else if m.y() == Self::board_width() as u32 - 1 {
      offset = HexPosOffset::new(0, -1);
    }
    if m.x() == 0 {
      offset += HexPosOffset::new(1, 0);
    } else if m.x() == Self::board_width() as u32 - 1 {
      offset += HexPosOffset::new(-1, 0);
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
      .find(|&(_, &pos)| pos == idx)
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

  /// Given a position on the board, returns the index of the pawn with that
  /// position, or `None` if no such pawn exists.
  fn get_pawn_idx(&self, idx: PackedIdx) -> Option<u32> {
    if idx == PackedIdx::null() {
      return None;
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
        return Some(8 * i as u32 + (set_bit_idx / 8));
      }
    }

    // Only necessary if N not a multiple of eight.
    for i in 8 * (N / 8)..N {
      if unsafe { *self.pawn_poses.get_unchecked(i) } == idx {
        return Some(i as u32);
      }
    }

    None
  }

  /// Bounds checks a hex pos before turning it into a PackedIdx for lookup.
  fn get_tile_hex_pos(&self, idx: HexPos) -> TileState {
    if idx.x() >= N as u32 || idx.y() >= N as u32 {
      TileState::Empty
    } else {
      self.get_tile(idx.into())
    }
  }

  pub fn validate(&self) -> OnoroResult {
    let mut n_b_pawns = 0u32;
    let mut n_w_pawns = 0u32;
    let mut sum_of_mass = HexPos::zero();

    let mut uf = ConstUnionFind::<N2>::new();

    for pawn in self.pawns() {
      sum_of_mass += pawn.pos.into();

      if pawn.pos.x() == 0
        || pawn.pos.x() >= N as u32 - 1
        || pawn.pos.y() == 0
        || pawn.pos.y() >= N as u32 - 1
      {
        return Err(make_onoro_error!(
          "Pawn with coordinates on border of board: {}",
          pawn
        ));
      }

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
          if self.get_tile_hex_pos(neighbor_pos) != TileState::Empty {
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
        + if !self.in_phase1() || self.onoro_state().black_turn() {
          0
        } else {
          1
        }
    {
      return Err(make_onoro_error!(
        "Expected {} black pawns and {} white pawns, but found {} and {}",
        self.pawns_in_play().div_ceil(2),
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

impl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> Onoro
  for OnoroImpl<N, N2, ADJ_CNT_SIZE>
{
  type Index = PackedIdx;
  type Move = Move;
  type Pawn = Pawn;

  unsafe fn new() -> Self {
    Self {
      pawn_poses: [PackedIdx::null(); N],
      state: OnoroState::new(),
      sum_of_mass: HexPos::zero().into(),
    }
  }

  fn pawns_per_player() -> usize {
    N / 2
  }

  fn turn(&self) -> PawnColor {
    if self.onoro_state().black_turn() {
      PawnColor::Black
    } else {
      PawnColor::White
    }
  }

  fn pawns_in_play(&self) -> u32 {
    self.onoro_state().turn() + 1
  }

  fn finished(&self) -> Option<PawnColor> {
    self.onoro_state().finished().then(|| {
      if self.onoro_state().black_turn() {
        PawnColor::White
      } else {
        PawnColor::Black
      }
    })
  }

  fn get_tile(&self, idx: PackedIdx) -> TileState {
    match self.get_pawn_idx(idx) {
      Some(i) => {
        if i % 2 == 0 {
          TileState::Black
        } else {
          TileState::White
        }
      }
      None => TileState::Empty,
    }
  }

  fn pawns(&self) -> impl Iterator<Item = Pawn> + '_ {
    self.pawns_typed()
  }

  fn in_phase1(&self) -> bool {
    self.onoro_state().turn() < 0xf
  }

  fn each_move(&self) -> impl Iterator<Item = Move> {
    self.each_move_gen().to_iter(self)
  }

  fn make_move(&mut self, m: Move) {
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
        self.mut_onoro_state().swap_player_turn();
        self.move_pawn(from_idx as usize, to);
      }
    }
  }

  fn to_move_wrapper(&self, m: &Move) -> OnoroMoveWrapper<PackedIdx> {
    match *m {
      Move::Phase1Move { to } => OnoroMoveWrapper::Phase1 { to },
      Move::Phase2Move { to, from_idx } => OnoroMoveWrapper::Phase2 {
        from: *self.pawn_poses.get(from_idx as usize).unwrap(),
        to,
      },
    }
  }
}

impl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> Debug
  for OnoroImpl<N, N2, ADJ_CNT_SIZE>
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{self}")
  }
}

impl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> Display
  for OnoroImpl<N, N2, ADJ_CNT_SIZE>
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    self.display(f)
  }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Pawn {
  pub pos: PackedIdx,
  pub color: PawnColor,
  board_idx: u8,
}

impl OnoroPawn for Pawn {
  type Index = PackedIdx;

  fn pos(&self) -> PackedIdx {
    self.pos
  }

  fn color(&self) -> PawnColor {
    self.color
  }
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

pub struct PawnMoveGenerator<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> {
  pawn_idx: usize,
  /// If true, only iterates over pawns of one color, otherwise iterating over
  /// all pawns.
  one_color: bool,
}

impl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> GameMoveGenerator
  for PawnMoveGenerator<N, N2, ADJ_CNT_SIZE>
{
  type Item = Pawn;
  type Game = OnoroImpl<N, N2, ADJ_CNT_SIZE>;

  fn next(&mut self, onoro: &Self::Game) -> Option<Self::Item> {
    if self.pawn_idx >= onoro.pawns_in_play() as usize {
      return None;
    }

    let pawn = Pawn {
      pos: unsafe { *onoro.pawn_poses.get_unchecked(self.pawn_idx) },
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

pub enum MoveGenerator<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> {
  P1Moves(P1MoveGenerator<N, N2, ADJ_CNT_SIZE>),
  P2Moves(P2MoveGenerator<N, N2, ADJ_CNT_SIZE>),
}

impl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> GameMoveGenerator
  for MoveGenerator<N, N2, ADJ_CNT_SIZE>
{
  type Item = Move;
  type Game = OnoroImpl<N, N2, ADJ_CNT_SIZE>;

  fn next(&mut self, onoro: &Self::Game) -> Option<Self::Item> {
    match self {
      Self::P1Moves(p1_iter) => p1_iter.next(onoro),
      Self::P2Moves(p2_iter) => p2_iter.next(onoro),
    }
  }
}

pub struct P1MoveGenerator<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> {
  pawn_iter: PawnMoveGenerator<N, N2, ADJ_CNT_SIZE>,
  neighbor_iter: Option<std::array::IntoIter<HexPos, 6>>,

  /// Bitvector of 2-bit numbers per tile in the whole game board. Each number
  /// is the number of neighbors a pawn has, capping out at 2.
  adjacency_counts: [u64; ADJ_CNT_SIZE],
}

impl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> GameMoveGenerator
  for P1MoveGenerator<N, N2, ADJ_CNT_SIZE>
{
  type Item = Move;
  type Game = OnoroImpl<N, N2, ADJ_CNT_SIZE>;

  fn next(&mut self, onoro: &Self::Game) -> Option<Self::Item> {
    loop {
      if let Some(neighbor) = self.neighbor_iter.as_mut().and_then(|iter| iter.next()) {
        // Bypass the bounds check in get_tile_hex_pos, since we know all pawns
        // are within a margin of 1 from the border.
        if onoro.get_tile(neighbor.into()) != TileState::Empty {
          continue;
        }

        let ord = OnoroImpl::<N, N2, ADJ_CNT_SIZE>::hex_pos_ord(&neighbor);
        let tb_shift = TILE_BITS * (ord % (64 / TILE_BITS));
        let tbb = unsafe { *self.adjacency_counts.get_unchecked(ord / (64 / TILE_BITS)) };
        let mask = TILE_MASK << tb_shift;
        let full_mask = MIN_NEIGHBORS_PER_PAWN << tb_shift;

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
      } else if let Some(pawn) = self.pawn_iter.next(onoro) {
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

pub struct P2MoveGenerator<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> {
  /// The current pawn that is being considered for moving. Only iterates over
  /// the pawns of the current player.
  pawn_iter: PawnMoveGenerator<N, N2, ADJ_CNT_SIZE>,
  pawn_meta: Option<P2PawnMeta<N2>>,

  /// Bitvector of 2-bit numbers per tile in the whole game board. Each number
  /// is the number of neighbors a pawn has, capping out at 2.
  adjacency_counts: [u64; ADJ_CNT_SIZE],
}

impl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize>
  P2MoveGenerator<N, N2, ADJ_CNT_SIZE>
{
  fn new(onoro: &OnoroImpl<N, N2, ADJ_CNT_SIZE>) -> Self {
    Self {
      pawn_iter: onoro.color_pawns_gen(onoro.player_color()),
      pawn_meta: None,
      adjacency_counts: [0; ADJ_CNT_SIZE],
    }
    .populate_neighbor_counts(onoro)
  }

  fn populate_neighbor_counts(mut self, onoro: &OnoroImpl<N, N2, ADJ_CNT_SIZE>) -> Self {
    for pawn in onoro.pawns() {
      for neighbor in HexPos::from(pawn.pos).each_neighbor() {
        let ord = OnoroImpl::<N, N2, ADJ_CNT_SIZE>::hex_pos_ord(&neighbor);
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
  fn prepare_move_pawn(
    &mut self,
    pawn_idx: usize,
    pawn_pos: PackedIdx,
    onoro: &OnoroImpl<N, N2, ADJ_CNT_SIZE>,
  ) {
    let mut uf = ConstUnionFind::new();
    let pawn_hex_pos: HexPos = pawn_pos.into();

    // Calculate the number of disjoint pawn groups after removing the pawn at
    // next_idx
    for pawn in onoro.pawns() {
      // Skip ourselves.
      if pawn.pos == pawn_pos {
        continue;
      }
      let pawn_ord = OnoroImpl::<N, N2, ADJ_CNT_SIZE>::hex_pos_ord(&pawn.pos.into());

      for neighbor in HexPos::from(pawn.pos).each_top_left_neighbor() {
        // Bypass the bounds check in get_tile_hex_pos, since we know all pawns
        // are within a margin of 1 from the border.
        if onoro.get_tile(neighbor.into()) != TileState::Empty && pawn_hex_pos != neighbor {
          uf.union(
            pawn_ord,
            OnoroImpl::<N, N2, ADJ_CNT_SIZE>::hex_pos_ord(&neighbor),
          );
        }
      }
    }

    let empty_tiles = OnoroImpl::<N, N2, ADJ_CNT_SIZE>::board_size() as u32 - onoro.pawns_in_play();
    // Note: the pawn we are moving is its own group.
    let pawn_groups = uf.unique_sets() as u32 - empty_tiles - 1;

    // number of neighbors with 1 neighbor after removing this piece
    let mut neighbors_to_satisfy = 0;
    // decrease neighbor count of all neighbors
    for neighbor in HexPos::from(pawn_pos).each_neighbor() {
      let neighbor_ord = OnoroImpl::<N, N2, ADJ_CNT_SIZE>::hex_pos_ord(&neighbor);
      let tb_idx = neighbor_ord / (64 / TILE_BITS);
      let tb_shift = TILE_BITS * (neighbor_ord % (64 / TILE_BITS));

      unsafe {
        *self.adjacency_counts.get_unchecked_mut(tb_idx) -= 1u64 << tb_shift;
      }
      // If this neighbor has only one neighbor itself now, and it isn't empty,
      // we have to place our pawn next to it.
      if ((unsafe { *self.adjacency_counts.get_unchecked(tb_idx) } >> tb_shift) & TILE_MASK) == 1
        && onoro.get_tile(neighbor.into()) != TileState::Empty
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
      let neighbor_ord = OnoroImpl::<N, N2, ADJ_CNT_SIZE>::hex_pos_ord(&neighbor);
      let tb_idx = neighbor_ord / (64 / TILE_BITS);
      let tb_shift = TILE_BITS * (neighbor_ord % (64 / TILE_BITS));

      unsafe {
        *self.adjacency_counts.get_unchecked_mut(tb_idx) += 1u64 << tb_shift;
      }
    }

    self.pawn_meta = None;
  }
}

impl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> GameMoveGenerator
  for P2MoveGenerator<N, N2, ADJ_CNT_SIZE>
{
  type Item = Move;
  type Game = OnoroImpl<N, N2, ADJ_CNT_SIZE>;

  fn next(&mut self, onoro: &Self::Game) -> Option<Self::Item> {
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
        let place_to_consider = OnoroImpl::<N, N2, ADJ_CNT_SIZE>::ord_to_hex_pos(next_idx_ord);
        let place_to_consider_idx = PackedIdx::from(place_to_consider);

        // Skip this tile if it isn't empty (this will also skip the piece's
        // old location since we haven't removed it, which we want)
        if onoro.get_tile(place_to_consider_idx) != TileState::Empty
          || ((pawn_meta.adj_cnt_bitmask >> tb_shift) & TILE_MASK) <= 1
        {
          pawn_meta.adj_cnt_bitmask &= !clr_mask;
          continue;
        }

        // Clear out the neighbor counts for the location being considered
        // currently, so we don't try it again next loop.
        pawn_meta.adj_cnt_bitmask &= !clr_mask;

        // A count of the number of neighbors with only one other adjacent pawn.
        let mut n_satisfied = 0;
        // The first group ID of any neighbor from the union find.
        let mut g1 = None;
        // The second group ID of any neighbor from the union find.
        let mut g2 = None;
        // The number of distinct groups of pawns adjacent to the place being
        // considered.
        let mut groups_touching = 0;
        for neighbor in place_to_consider.each_neighbor() {
          if onoro.get_tile_hex_pos(neighbor) == TileState::Empty {
            continue;
          }
          let neighbor_ord = OnoroImpl::<N, N2, ADJ_CNT_SIZE>::hex_pos_ord(&neighbor);

          let tb_idx = neighbor_ord / (64 / TILE_BITS);
          let tb_shift = TILE_BITS * (neighbor_ord % (64 / TILE_BITS));
          if ((unsafe { *self.adjacency_counts.get_unchecked(tb_idx) } >> tb_shift) & TILE_MASK)
            == 1
          {
            n_satisfied += 1;
          }

          if neighbor != pawn_meta.pawn_pos.into() {
            let group_id = pawn_meta.uf.find(neighbor_ord);
            // There can be at most 3 distinct groups of pawns adjacent to this
            // spot, since there are 6 neighboring tiles, and each tile touches
            // two other neighbors. The first neighbor will assign its group ID
            // to `g1`, the second distinct group ID will be assigned to `g2`,
            // and if a third group ID is seen, it will reassign `g2` to it, but
            // will also update `groups_touching`. In the end, `groups_touching`
            // will be correct, which is all that matters.
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
      } else if let Some(pawn) = self.pawn_iter.next(onoro) {
        self.prepare_move_pawn(pawn.board_idx as usize, pawn.pos, onoro);
      } else {
        return None;
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use onoro::Onoro;

  use crate::{onoro_defs::Onoro8, packed_idx::PackedIdx};

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
