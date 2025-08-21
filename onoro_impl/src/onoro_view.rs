use std::{
  collections::{HashMap, HashSet},
  fmt::Display,
  hash::Hash,
};

use algebra::{
  group::{Group, Trivial},
  monoid::Monoid,
  ordinal::Ordinal,
  semigroup::Semigroup,
};

use abstract_game::{Game, GameMoveIterator, GameResult};
use onoro::{
  Compress, Onoro, PawnColor, TileState,
  error::{OnoroError, OnoroResult},
  groups::{C2, D3, D6, K4, SymmetryClass, SymmetryClassContainer},
  hex_pos::{HexPos, HexPosOffset},
};

use crate::{
  MoveGenerator, OnoroImpl,
  canonicalize::{BoardSymmetryState, board_symm_state},
  hash::HashTable,
  r#move::Move,
  onoro_defs::{Onoro16, Onoro16View},
  tile_hash::HashGroup,
};

/// Always generate hash tables for the full game. Only a part of the tables
/// will be used for smaller games.
type ViewHashTable<G> = HashTable<16, G>;

#[derive(Clone, Debug)]
pub struct CanonicalView {
  symm_class: SymmetryClass,
  op_ord: u8,
  hash: u64,
}

impl CanonicalView {
  fn get_symm_class(&self) -> SymmetryClass {
    self.symm_class
  }

  fn get_op_ord(&self) -> u8 {
    self.op_ord
  }

  fn get_hash(&self) -> u64 {
    self.hash
  }
}

/// A wrapper over Onoro states that caches the hash of the game state and it's
/// canonicalizing symmetry operations. These cached values are used for quicker
/// equality comparison between different Onoro game states which may be in
/// different orientations.
#[derive(Clone, Debug)]
pub struct OnoroView<const N: usize> {
  onoro: OnoroImpl<N>,
  view: CanonicalView,
}

impl<const N: usize> OnoroView<N> {
  /// TODO: Make new lazy
  pub fn new(onoro: OnoroImpl<N>) -> Self {
    let view = Self::find_canonical_view(&onoro);
    Self { onoro, view }
  }

  pub fn onoro(&self) -> &OnoroImpl<N> {
    &self.onoro
  }

  fn canon_view(&self) -> &CanonicalView {
    &self.view
  }

  pub fn find_canonical_view(onoro: &OnoroImpl<N>) -> CanonicalView {
    let symm_state = board_symm_state(onoro);
    let (hash, op_ord) = match symm_state.symm_class {
      SymmetryClass::C => Self::find_canonical_orientation_d6(onoro, &symm_state),
      SymmetryClass::V => Self::find_canonical_orientation_d3(onoro, &symm_state),
      SymmetryClass::E => Self::find_canonical_orientation_k4(onoro, &symm_state),
      SymmetryClass::CV => Self::find_canonical_orientation_c2_cv(onoro, &symm_state),
      SymmetryClass::CE => Self::find_canonical_orientation_c2_ce(onoro, &symm_state),
      SymmetryClass::EV => Self::find_canonical_orientation_c2_ev(onoro, &symm_state),
      SymmetryClass::Trivial => Self::find_canonical_orientation_trivial(onoro, &symm_state),
    };

    CanonicalView {
      symm_class: symm_state.symm_class,
      op_ord,
      hash,
    }
  }

  fn find_canonical_orientation_d6(
    onoro: &OnoroImpl<N>,
    symm_state: &BoardSymmetryState,
  ) -> (u64, u8) {
    static D6T: ViewHashTable<D6> = HashTable::new_c();
    let hash = HashGroup::<D6>::new(D6T.hash(onoro, symm_state));

    // Try all symmetries of the board state with invariant center of mass,
    // choose the symmetry with the numerically smallest hash code.
    D6::for_each()
      .map(|op| (hash.apply(&op).hash(), op.ord() as u8))
      .min_by(|(hash1, _op1), (hash2, _op2)| hash1.cmp(hash2))
      .unwrap()
  }

  fn find_canonical_orientation_d3(
    onoro: &OnoroImpl<N>,
    symm_state: &BoardSymmetryState,
  ) -> (u64, u8) {
    static D3T: ViewHashTable<D3> = HashTable::new_v();
    let hash = HashGroup::<D3>::new(D3T.hash(onoro, symm_state));

    // Try all symmetries of the board state with invariant center of mass,
    // choose the symmetry with the numerically smallest hash code.
    D3::for_each()
      .map(|op| (hash.apply(&op).hash(), op.ord() as u8))
      .min_by(|(hash1, _op1), (hash2, _op2)| hash1.cmp(hash2))
      .unwrap()
  }

  fn find_canonical_orientation_k4(
    onoro: &OnoroImpl<N>,
    symm_state: &BoardSymmetryState,
  ) -> (u64, u8) {
    static K4T: ViewHashTable<K4> = HashTable::new_e();
    let hash = HashGroup::<K4>::new(K4T.hash(onoro, symm_state));

    // Try all symmetries of the board state with invariant center of mass,
    // choose the symmetry with the numerically smallest hash code.
    K4::for_each()
      .map(|op| (hash.apply(&op).hash(), op.ord() as u8))
      .min_by(|(hash1, _op1), (hash2, _op2)| hash1.cmp(hash2))
      .unwrap()
  }

  fn find_canonical_orientation_c2_cv(
    onoro: &OnoroImpl<N>,
    symm_state: &BoardSymmetryState,
  ) -> (u64, u8) {
    static C2CVT: ViewHashTable<C2> = HashTable::new_cv();
    let hash = HashGroup::<C2>::new(C2CVT.hash(onoro, symm_state));

    // Try all symmetries of the board state with invariant center of mass,
    // choose the symmetry with the numerically smallest hash code.
    C2::for_each()
      .map(|op| (hash.apply(&op).hash(), op.ord() as u8))
      .min_by(|(hash1, _op1), (hash2, _op2)| hash1.cmp(hash2))
      .unwrap()
  }

  fn find_canonical_orientation_c2_ce(
    onoro: &OnoroImpl<N>,
    symm_state: &BoardSymmetryState,
  ) -> (u64, u8) {
    static C2CET: ViewHashTable<C2> = HashTable::new_ce();
    let hash = HashGroup::<C2>::new(C2CET.hash(onoro, symm_state));

    // Try all symmetries of the board state with invariant center of mass,
    // choose the symmetry with the numerically smallest hash code.
    C2::for_each()
      .map(|op| (hash.apply(&op).hash(), op.ord() as u8))
      .min_by(|(hash1, _op1), (hash2, _op2)| hash1.cmp(hash2))
      .unwrap()
  }

  fn find_canonical_orientation_c2_ev(
    onoro: &OnoroImpl<N>,
    symm_state: &BoardSymmetryState,
  ) -> (u64, u8) {
    static C2EVT: ViewHashTable<C2> = HashTable::new_ev();
    let hash = HashGroup::<C2>::new(C2EVT.hash(onoro, symm_state));

    // Try all symmetries of the board state with invariant center of mass,
    // choose the symmetry with the numerically smallest hash code.
    C2::for_each()
      .map(|op| (hash.apply(&op).hash(), op.ord() as u8))
      .min_by(|(hash1, _op1), (hash2, _op2)| hash1.cmp(hash2))
      .unwrap()
  }

  fn find_canonical_orientation_trivial(
    onoro: &OnoroImpl<N>,
    symm_state: &BoardSymmetryState,
  ) -> (u64, u8) {
    static TT: ViewHashTable<Trivial> = HashTable::new_trivial();
    (TT.hash(onoro, symm_state), Trivial::identity().ord() as u8)
  }

  fn pawns_iterator<'a, G, F>(
    &'a self,
    mut apply_view_transform: F,
  ) -> impl Iterator<Item = (HexPosOffset, PawnColor)> + 'a
  where
    G: Group + Ordinal + 'a,
    F: FnMut(&HexPosOffset, &G) -> HexPosOffset + 'a,
  {
    let symm_state = board_symm_state(&self.onoro);
    let origin = self.onoro.origin(&symm_state);
    let canon_op = G::from_ord(self.canon_view().get_op_ord() as usize);

    self.onoro.pawns_typed().map(move |pawn| {
      let normalized_pos = (HexPos::from(pawn.pos) - origin).apply_d6_c(&symm_state.op);
      let normalized_pos = apply_view_transform(&normalized_pos, &canon_op);
      (normalized_pos, pawn.color)
    })
  }

  pub fn pawns(&self) -> impl Iterator<Item = (HexPosOffset, PawnColor)> + '_ {
    match self.canon_view().get_symm_class() {
      SymmetryClass::C => SymmetryClassContainer::C(self.pawns_iterator(HexPosOffset::apply_d6_c)),
      SymmetryClass::V => SymmetryClassContainer::V(self.pawns_iterator(HexPosOffset::apply_d3_v)),
      SymmetryClass::E => SymmetryClassContainer::E(self.pawns_iterator(HexPosOffset::apply_k4_e)),
      SymmetryClass::CV => {
        SymmetryClassContainer::CV(self.pawns_iterator(HexPosOffset::apply_c2_cv))
      }
      SymmetryClass::CE => {
        SymmetryClassContainer::CE(self.pawns_iterator(HexPosOffset::apply_c2_ce))
      }
      SymmetryClass::EV => {
        SymmetryClassContainer::EV(self.pawns_iterator(HexPosOffset::apply_c2_ev))
      }
      SymmetryClass::Trivial => {
        SymmetryClassContainer::Trivial(self.pawns_iterator(HexPosOffset::apply_trivial))
      }
    }
  }

  fn cmp_views<G: Group + Ordinal + Display, F>(
    view1: &OnoroView<N>,
    view2: &OnoroView<N>,
    mut apply_view_transform: F,
  ) -> bool
  where
    F: FnMut(&HexPosOffset, &G) -> HexPosOffset,
  {
    let onoro1 = &view1.onoro;
    let onoro2 = &view2.onoro;

    if onoro1.pawns_in_play() != onoro2.pawns_in_play() {
      return false;
    }

    let symm_state1 = board_symm_state(onoro1);
    let symm_state2 = board_symm_state(onoro2);
    let normalizing_op1 = symm_state1.op;
    let denormalizing_op2 = symm_state2.op.inverse();
    let origin1 = onoro1.origin(&symm_state1);
    let origin2 = onoro2.origin(&symm_state2);

    let canon_op1 = G::from_ord(view1.canon_view().get_op_ord() as usize);
    let canon_op2 = G::from_ord(view2.canon_view().get_op_ord() as usize);
    let to_view2 = canon_op2.inverse() * canon_op1;

    let same_color_turn = onoro1.player_color() == onoro2.player_color();

    onoro1.pawns().all(|pawn| {
      let normalized_pos1 = (HexPos::from(pawn.pos) - origin1).apply_d6_c(&normalizing_op1);
      let normalized_pos2 = apply_view_transform(&normalized_pos1, &to_view2);
      let pos2 = normalized_pos2.apply_d6_c(&denormalizing_op2) + origin2;

      match onoro2.get_tile(pos2.into()) {
        TileState::Black => {
          if same_color_turn {
            pawn.color == PawnColor::Black
          } else {
            pawn.color == PawnColor::White
          }
        }
        TileState::White => {
          if same_color_turn {
            pawn.color == PawnColor::White
          } else {
            pawn.color == PawnColor::Black
          }
        }
        TileState::Empty => false,
      }
    })
  }
}

impl<const N: usize> PartialEq for OnoroView<N> {
  fn eq(&self, other: &Self) -> bool {
    if self.canon_view().get_hash() != other.canon_view().get_hash()
      || self.canon_view().get_symm_class() != other.canon_view().get_symm_class()
    {
      return false;
    }

    match self.canon_view().get_symm_class() {
      SymmetryClass::C => Self::cmp_views(self, other, HexPosOffset::apply_d6_c),
      SymmetryClass::V => Self::cmp_views(self, other, HexPosOffset::apply_d3_v),
      SymmetryClass::E => Self::cmp_views(self, other, HexPosOffset::apply_k4_e),
      SymmetryClass::CV => Self::cmp_views(self, other, HexPosOffset::apply_c2_cv),
      SymmetryClass::CE => Self::cmp_views(self, other, HexPosOffset::apply_c2_ce),
      SymmetryClass::EV => Self::cmp_views(self, other, HexPosOffset::apply_c2_ev),
      SymmetryClass::Trivial => Self::cmp_views(self, other, HexPosOffset::apply_trivial),
    }
  }
}

impl<const N: usize> Eq for OnoroView<N> {}

impl<const N: usize> Hash for OnoroView<N> {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    state.write_u64(self.canon_view().get_hash());
  }
}

/// Send/Sync rely on the OnoroView being initialized before being shared
/// between threads. This assumption is safe because the view is inserted when
/// it's inserted into the hash table.
///
/// Technically, since the CanonicalView is deterministically computed, it
/// doesn't matter if there is a race to write it to the UnsafeCell, since all
/// threads would be writing the same data to the same locations.
unsafe impl<const N: usize> Send for OnoroView<N> {}
unsafe impl<const N: usize> Sync for OnoroView<N> {}

impl<const N: usize> Display for OnoroView<N> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let symm_state = board_symm_state(self.onoro());
    let rotated = self.onoro().rotated_d6_c(symm_state.op);
    let _rotated = match self.canon_view().get_symm_class() {
      SymmetryClass::C => {
        rotated.rotated_d6_c(D6::from_ord(self.canon_view().get_op_ord() as usize))
      }
      SymmetryClass::V => {
        rotated.rotated_d3_v(D3::from_ord(self.canon_view().get_op_ord() as usize))
      }
      SymmetryClass::E => {
        rotated.rotated_k4_e(K4::from_ord(self.canon_view().get_op_ord() as usize))
      }
      SymmetryClass::CV => {
        rotated.rotated_c2_cv(C2::from_ord(self.canon_view().get_op_ord() as usize))
      }
      SymmetryClass::CE => {
        rotated.rotated_c2_ce(C2::from_ord(self.canon_view().get_op_ord() as usize))
      }
      SymmetryClass::EV => {
        rotated.rotated_c2_ev(C2::from_ord(self.canon_view().get_op_ord() as usize))
      }
      SymmetryClass::Trivial => rotated,
    };

    write!(
      f,
      "{}\n{:?}: canon: {}, normalize: {} ({:#018x?})",
      self.onoro,
      self.canon_view().get_symm_class(),
      symm_state.op,
      match self.canon_view().get_symm_class() {
        SymmetryClass::C => D6::from_ord(self.canon_view().get_op_ord() as usize).to_string(),
        SymmetryClass::V => D3::from_ord(self.canon_view().get_op_ord() as usize).to_string(),
        SymmetryClass::E => K4::from_ord(self.canon_view().get_op_ord() as usize).to_string(),
        SymmetryClass::CV | SymmetryClass::CE | SymmetryClass::EV =>
          C2::from_ord(self.canon_view().get_op_ord() as usize).to_string(),
        SymmetryClass::Trivial =>
          Trivial::from_ord(self.canon_view().get_op_ord() as usize).to_string(),
      },
      self.canon_view().get_hash()
    )
  }
}

pub struct ViewMoveGenerator<const N: usize> {
  move_gen: MoveGenerator<N>,
}

impl<const N: usize> GameMoveIterator for ViewMoveGenerator<N> {
  type Item = Move;
  type Game = OnoroView<N>;

  fn next(&mut self, view: &Self::Game) -> Option<Self::Item> {
    self.move_gen.next(view.onoro())
  }
}

impl<const N: usize> Game for OnoroView<N> {
  type Move = Move;
  type MoveGenerator = ViewMoveGenerator<N>;
  type PlayerIdentifier = PawnColor;

  fn move_generator(&self) -> Self::MoveGenerator {
    ViewMoveGenerator {
      move_gen: self.onoro().each_move_gen(),
    }
  }

  fn make_move(&mut self, m: Self::Move) {
    let mut onoro = self.onoro().clone();
    onoro.make_move(m);
    *self = OnoroView::new(onoro);
  }

  fn current_player(&self) -> Self::PlayerIdentifier {
    self.onoro().player_color()
  }

  fn finished(&self) -> GameResult<Self::PlayerIdentifier> {
    match self.onoro().finished() {
      Some(color) => GameResult::Win(color),
      None => GameResult::NotFinished,
    }
  }
}

impl Compress for Onoro16View {
  type Repr = u64;

  fn compress(&self) -> u64 {
    let pawn_colors: HashMap<_, _> = self.pawns().collect();
    let (start_pawn_pos, start_pawn_color) = self
      .pawns()
      .min_by_key(|(pos, _)| *pos)
      .expect("Cannot compress empty onoro board");

    let mut known_tiles = HashSet::<HexPosOffset>::new();
    known_tiles.insert(start_pawn_pos);
    for empty_pos in start_pawn_pos.each_top_left_neighbor() {
      known_tiles.insert(empty_pos);
    }

    let mut position_bits = Vec::<bool>::new();
    let mut color_bits = vec![start_pawn_color];

    let mut pawn_stack = vec![start_pawn_pos];

    while let Some(current_pawn) = pawn_stack.pop() {
      for neighbor_pos in current_pawn.each_neighbor() {
        if !known_tiles.insert(neighbor_pos) {
          continue;
        }

        let neighbor_color = pawn_colors.get(&neighbor_pos);
        position_bits.push(neighbor_color.is_some());

        if let Some(&color) = neighbor_color {
          color_bits.push(color);
          pawn_stack.push(neighbor_pos);
        }
      }
    }

    color_bits
      .iter()
      .map(|c| matches!(c, PawnColor::Black))
      .chain(position_bits)
      .enumerate()
      .fold(0, |acc, (idx, set)| {
        acc | ((if set { 1 } else { 0 }) << idx)
      })
  }

  fn decompress(repr: u64) -> OnoroResult<Self> {
    const PAWN_COUNT: usize = 16;

    if (repr & 0xffff).count_ones() != PAWN_COUNT as u32 / 2 {
      return Err(OnoroError::new("Expect 8 of the 16 pawns to be white.".to_owned()).into());
    }
    if (repr & !0xffff).count_ones() != PAWN_COUNT as u32 - 1 {
      return Err(OnoroError::new(format!("Expect {PAWN_COUNT} pawns.")).into());
    }

    let mut board = HashMap::<HexPosOffset, TileState>::new();

    let mut pawn_stack = vec![HexPosOffset::origin()];
    let mut position_bits_idx = PAWN_COUNT;
    let mut color_bits_idx = 1;
    board.insert(
      HexPosOffset::origin(),
      if (repr & 1) != 0 {
        TileState::Black
      } else {
        TileState::White
      },
    );
    for empty_pos in HexPosOffset::origin().each_top_left_neighbor() {
      board.insert(empty_pos, TileState::Empty);
    }

    for _ in 1..PAWN_COUNT {
      let pawn = pawn_stack
        .pop()
        .ok_or_else(|| OnoroError::new("Unexpected end of stack".to_owned()))?;

      let mut neighbor_count = 0;
      for neighbor_pos in pawn.each_neighbor() {
        if let Some(tile) = board.get_mut(&neighbor_pos) {
          if matches!(tile, TileState::Black | TileState::White) {
            neighbor_count += 1;
          }
          continue;
        }
        debug_assert!(position_bits_idx < u64::BITS as usize);
        if ((repr >> position_bits_idx) & 1) != 0 {
          pawn_stack.push(neighbor_pos);
          let color = if ((repr >> color_bits_idx) & 1) != 0 {
            TileState::Black
          } else {
            TileState::White
          };
          neighbor_count += 1;
          color_bits_idx += 1;
          debug_assert!(color_bits_idx <= PAWN_COUNT);
          board.insert(neighbor_pos, color);
        } else {
          board.insert(neighbor_pos, TileState::Empty);
        }
        position_bits_idx += 1;
      }

      if neighbor_count < 2 {
        return Err(OnoroError::new("Not enough neighbors of pawn!".to_owned()).into());
      }
    }

    let pawn = pawn_stack
      .pop()
      .ok_or_else(|| OnoroError::new("Unexpected end of stack".to_owned()))?;
    let neighbor_count = pawn
      .each_neighbor()
      .filter_map(|neighbor_pos| board.get(&neighbor_pos))
      .filter(|tile| matches!(tile, TileState::Black | TileState::White))
      .count();
    if neighbor_count < 2 {
      return Err(OnoroError::new("Not enough neighbors of pawn!".to_owned()).into());
    }

    let onoro = Onoro16::from_pawns(
      board
        .into_iter()
        .filter_map(|(pos, state)| match state {
          TileState::Empty => None,
          TileState::Black => Some((pos, PawnColor::Black)),
          TileState::White => Some((pos, PawnColor::White)),
        })
        .collect(),
    )?;
    Ok(Onoro16View::new(onoro))
  }
}

#[cfg(test)]
mod tests {
  use googletest::{
    assert_that, expect_that, gtest,
    prelude::{any, anything, container_eq},
  };
  use itertools::{Either, Itertools};

  use onoro::{
    Compress, Onoro, PawnColor, error::OnoroResult, groups::SymmetryClass, hex_pos::HexPosOffset,
  };

  use super::{Onoro16, Onoro16View, OnoroView};

  fn build_view(board_layout: &str) -> Onoro16View {
    OnoroView::new(Onoro16::from_board_string(board_layout).unwrap())
  }

  fn verify_pawn_iter<const N: usize>(view1: &OnoroView<N>, view2: &OnoroView<N>) {
    let (mut b1, mut w1): (Vec<_>, Vec<_>) =
      view1.pawns().partition_map(|(pos, color)| match color {
        PawnColor::Black => Either::Left(pos),
        PawnColor::White => Either::Right(pos),
      });
    let (mut b2, mut w2): (Vec<_>, Vec<_>) =
      view2.pawns().partition_map(|(pos, color)| match color {
        PawnColor::Black => Either::Left(pos),
        PawnColor::White => Either::Right(pos),
      });

    let pos_cmp = |pos1: &HexPosOffset, pos2: &HexPosOffset| {
      pos1.x().cmp(&pos2.x()).then(pos1.y().cmp(&pos2.y()))
    };
    b1.sort_by(pos_cmp);
    b2.sort_by(pos_cmp);
    w1.sort_by(pos_cmp);
    w2.sort_by(pos_cmp);
    assert_that!(b1, container_eq(b2));
    assert_that!(w1, container_eq(w2));
  }

  fn expect_view_eq<const N: usize>(view1: &OnoroView<N>, view2: &OnoroView<N>) {
    assert_eq!(view1, view2);
    verify_pawn_iter(view1, view2);
  }

  fn expect_view_ne<const N: usize>(view1: &OnoroView<N>, view2: &OnoroView<N>) {
    assert_ne!(view1, view2);
  }

  fn compress_round_trip(onoro: &Onoro16View) -> OnoroResult<Onoro16View> {
    OnoroView::decompress(onoro.compress())
  }

  #[gtest]
  #[allow(non_snake_case)]
  fn test_V_symm_simple() {
    let view1 = build_view(
      ". W
        B B",
    );
    let view2 = build_view(
      ". B
        B W",
    );

    assert_eq!(view1.canon_view().get_symm_class(), SymmetryClass::V);

    expect_view_eq(&view1, &view2);
  }

  #[test]
  #[allow(non_snake_case)]
  fn test_C_symm_simple() {
    let view1 = build_view(
      ". W B
        B . W
         W B",
    );
    let view2 = build_view(
      ". B W
        W . B
         B W",
    );
    let view3 = build_view(
      ". B W
        W B B
         B W",
    );
    let view4 = build_view(
      ". . B
        W . .
         . B",
    );
    let view5 = build_view(
      ". B .
        . . B
         W .",
    );

    assert_eq!(view1.canon_view().get_symm_class(), SymmetryClass::C);
    assert_eq!(view3.canon_view().get_symm_class(), SymmetryClass::C);
    assert_eq!(view4.canon_view().get_symm_class(), SymmetryClass::C);

    expect_view_eq(&view1, &view2);
    expect_view_ne(&view1, &view3);
    expect_view_ne(&view2, &view3);
    expect_view_ne(&view1, &view4);
    expect_view_ne(&view2, &view4);
    expect_view_ne(&view3, &view4);
    expect_view_ne(&view1, &view5);
    expect_view_ne(&view2, &view5);
    expect_view_ne(&view3, &view5);
    expect_view_eq(&view4, &view5);
  }

  #[test]
  #[allow(non_snake_case)]
  fn test_C_symm_2() {
    let view1 = build_view(
      ". W B
        . . .
         W B .",
    );
    let view2 = build_view(
      ". B .
        W . B
         . W .",
    );
    let view3 = build_view(
      ". W .
        B . B
         . W .",
    );
    let view4 = build_view(
      ". . W
        B . B
         W . .",
    );

    assert_eq!(view1.canon_view().get_symm_class(), SymmetryClass::C);
    assert_eq!(view3.canon_view().get_symm_class(), SymmetryClass::C);

    expect_view_eq(&view1, &view2);
    expect_view_ne(&view1, &view3);
    expect_view_ne(&view2, &view3);
    expect_view_ne(&view1, &view4);
    expect_view_ne(&view2, &view4);
    expect_view_eq(&view3, &view4);
  }

  #[test]
  #[allow(non_snake_case)]
  fn test_E_symm_simple() {
    let view1 = build_view(
      ". W . B
        . . . .
         W . B .",
    );
    let view2 = build_view(
      ". . B .
        . . . B
         W . . .
          . W . .",
    );
    let view3 = build_view(
      ". . W .
        . . . B
         B . . .
          . W . .",
    );
    let view4 = build_view(
      ". . W .
        B . . .
         . . B .
          W . . .",
    );

    assert_eq!(view1.canon_view().get_symm_class(), SymmetryClass::E);
    assert_eq!(view3.canon_view().get_symm_class(), SymmetryClass::E);

    expect_view_eq(&view1, &view2);
    expect_view_ne(&view1, &view3);
    expect_view_ne(&view2, &view3);
    expect_view_ne(&view1, &view4);
    expect_view_ne(&view2, &view4);
    expect_view_eq(&view3, &view4);
  }

  #[test]
  #[allow(non_snake_case)]
  fn test_CV_symm_simple() {
    let view1 = build_view(
      ". B . .
        W B . .
         . . . .
          . . . W",
    );
    let view2 = build_view(
      ". . . W
        . . . .
         . . . .
          . . . .
           . B . .
            W B . .",
    );
    let view3 = build_view(
      ". . B B
        . . W .
         . . . .
          . . . .
           . . . .
            W . . .",
    );
    let view4 = build_view(
      ". . . . . B
        . . . . W B
         . . . . . .
          W . . . . .",
    );

    assert_eq!(view1.canon_view().get_symm_class(), SymmetryClass::CV);
    assert_eq!(view3.canon_view().get_symm_class(), SymmetryClass::CV);

    expect_view_eq(&view1, &view2);
    expect_view_ne(&view1, &view3);
    expect_view_ne(&view2, &view3);
    expect_view_ne(&view1, &view4);
    expect_view_ne(&view2, &view4);
    expect_view_eq(&view3, &view4);
  }

  #[test]
  #[allow(non_snake_case)]
  fn test_CV_symm_hex_start() {
    const BOARD_POSITIONS: [&str; 6] = [
      ". . B
        . B W
         W . B
          B W .",
      ". B W B
        W . B .
         B W . .",
      ". B W
        W . B
         B W B",
      ". B W
        W . B
         B W .
          B . .",
      ". . B W
        . W . B
         B B W .",
      "B B W
        W . B
         B W .",
    ];

    let views = BOARD_POSITIONS.map(build_view);
    for i in 0..views.len() {
      assert_eq!(views[i].canon_view().get_symm_class(), SymmetryClass::CV);
      for j in 0..i {
        let view1 = &views[j];
        let view2 = &views[i];

        expect_view_eq(view1, view2);
      }
    }
  }

  #[test]
  #[allow(non_snake_case)]
  fn test_CE_symm_simple() {
    let view1 = build_view(
      ". B
        W B
         . W",
    );
    let view2 = build_view(
      ". W .
        B B W",
    );
    let view3 = build_view(
      ". . B
        W B W",
    );
    let view4 = build_view(
      ". . W
        . B B
         W . .",
    );

    assert_eq!(view1.canon_view().get_symm_class(), SymmetryClass::CE);
    assert_eq!(view3.canon_view().get_symm_class(), SymmetryClass::CE);

    expect_view_eq(&view1, &view2);
    expect_view_ne(&view1, &view3);
    expect_view_ne(&view2, &view3);
    expect_view_ne(&view1, &view4);
    expect_view_ne(&view2, &view4);
    expect_view_eq(&view3, &view4);
  }

  #[test]
  #[allow(non_snake_case)]
  fn test_EV_symm_simple() {
    let view1 = build_view(
      ". B . B
        . . . .
         . . W .
          B . W .",
    );
    let view2 = build_view(
      ". . . B
        W W . .
         . . . B
          B . . .",
    );
    let view3 = build_view(
      ". . . B
        . . B .
         B . . W
          . . . .
           W . . .",
    );
    let view4 = build_view(
      ". W . B
        . . . .
         . . B .
          W . B .",
    );

    assert_eq!(view1.canon_view().get_symm_class(), SymmetryClass::EV);
    assert_eq!(view3.canon_view().get_symm_class(), SymmetryClass::EV);

    expect_view_eq(&view1, &view2);
    expect_view_ne(&view1, &view3);
    expect_view_ne(&view2, &view3);
    expect_view_ne(&view1, &view4);
    expect_view_ne(&view2, &view4);
    expect_view_eq(&view3, &view4);
  }

  #[test]
  #[allow(non_snake_case)]
  fn test_trivial_symm_simple() {
    let view1 = build_view(
      ". B . B
        . . . .
         . . W .
          B W W .",
    );
    let view2 = build_view(
      ". . . B
        W W . .
         W . . B
          B . . .",
    );
    let view3 = build_view(
      ". . . B
        . . B W
         B . . W
          . . . .
           W . . .",
    );
    let view4 = build_view(
      ". W . B
        . . . .
         . . B .
          W W B .",
    );

    assert_eq!(view1.canon_view().get_symm_class(), SymmetryClass::Trivial);
    assert_eq!(view3.canon_view().get_symm_class(), SymmetryClass::Trivial);

    expect_view_eq(&view1, &view2);
    expect_view_ne(&view1, &view3);
    expect_view_ne(&view2, &view3);
    expect_view_ne(&view1, &view4);
    expect_view_ne(&view2, &view4);
    expect_view_eq(&view3, &view4);
  }

  #[test]
  fn test_1() {
    let onoro_str = build_view(
      ". W
        B B",
    );
    let onoro_pawns = OnoroView::new(
      Onoro16::from_pawns(vec![
        (HexPosOffset::new(0, 0), PawnColor::Black),
        (HexPosOffset::new(1, 0), PawnColor::Black),
        (HexPosOffset::new(1, 1), PawnColor::White),
      ])
      .unwrap(),
    );

    assert_eq!(onoro_str, onoro_pawns);
  }

  #[test]
  fn test_compress_single_pawn() {
    assert_eq!(build_view("B").compress(), 0b0001);
  }

  #[gtest]
  #[allow(clippy::unusual_byte_groupings)]
  fn test_compress_two_pawns() {
    expect_that!(
      build_view("B W").compress(),
      any!(
        0b000_100_10,
        0b000_010_10,
        0b000_001_10,
        0b000_100_01,
        0b000_010_01,
        0b000_001_01
      )
    );
  }

  #[gtest]
  fn test_compress_long_board() {
    expect_that!(
      build_view(
        ". W . . . . . . . . . . . .
          B W B W B W B W B W B W B W
           . . . . . . . . . . . . B ."
      )
      .compress(),
      anything()
    );
  }

  #[gtest]
  fn compress_decompress_smoke() -> OnoroResult {
    let view = build_view(
      ". W . . . . . . . . . . . .
        B W B W B W B W B W B W B W
         . . . . . . . . . . . . B .",
    );
    expect_view_eq(&view, &compress_round_trip(&view)?);
    Ok(())
  }

  #[gtest]
  fn compress_decompress_smoke2() -> OnoroResult {
    let view = build_view(
      ". W . . . W
        B W B W B W
         . B . B . .
          . W B W . .
           . W B B . .",
    );
    expect_view_eq(&view, &compress_round_trip(&view)?);
    Ok(())
  }
}
