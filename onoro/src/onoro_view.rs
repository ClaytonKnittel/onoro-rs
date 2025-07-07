use crate::{compress::Compress, groups::SymmetryClassContainer, Move, MoveGenerator, Onoro16View};
use std::{
  cell::UnsafeCell,
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

use abstract_game::{Game, GameMoveGenerator, GameResult};

use crate::{
  canonicalize::{board_symm_state, BoardSymmetryState},
  groups::{SymmetryClass, C2, D3, D6, K4},
  hash::HashTable,
  hex_pos::{HexPos, HexPosOffset},
  tile_hash::HashGroup,
  Onoro, PawnColor, TileState,
};

/// Always generate hash tables for the full game. Only a part of the tables
/// will be used for smaller games.
type ViewHashTable<G> = HashTable<16, 256, G>;

#[derive(Clone, Debug)]
struct CanonicalView {
  initialized: bool,
  symm_class: SymmetryClass,
  op_ord: u8,
  hash: u64,
}

impl CanonicalView {
  fn new() -> CanonicalView {
    CanonicalView {
      initialized: false,
      symm_class: SymmetryClass::C,
      op_ord: 0,
      hash: 0,
    }
  }

  fn get_symm_class(&self) -> SymmetryClass {
    debug_assert!(self.initialized);
    self.symm_class
  }

  fn get_op_ord(&self) -> u8 {
    debug_assert!(self.initialized);
    self.op_ord
  }

  fn get_hash(&self) -> u64 {
    debug_assert!(self.initialized);
    self.hash
  }
}

/// A wrapper over Onoro states that caches the hash of the game state and it's
/// canonicalizing symmetry operations. These cached values are used for quicker
/// equality comparison between different Onoro game states which may be in
/// different orientations.
#[derive(Debug)]
pub struct OnoroView<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> {
  onoro: Onoro<N, N2, ADJ_CNT_SIZE>,
  view: UnsafeCell<CanonicalView>,
}

impl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> OnoroView<N, N2, ADJ_CNT_SIZE> {
  /// TODO: Make new lazy
  pub fn new(onoro: Onoro<N, N2, ADJ_CNT_SIZE>) -> Self {
    Self {
      onoro,
      view: CanonicalView::new().into(),
    }
  }

  pub fn onoro(&self) -> &Onoro<N, N2, ADJ_CNT_SIZE> {
    &self.onoro
  }

  fn canon_view(&self) -> &CanonicalView {
    unsafe { &*self.view.get() }
  }

  fn maybe_initialize_canonical_view(&self) {
    if self.canon_view().initialized {
      return;
    }

    let symm_state = board_symm_state(&self.onoro);
    let (hash, op_ord) = match symm_state.symm_class {
      SymmetryClass::C => Self::find_canonical_orientation_d6(&self.onoro, &symm_state),
      SymmetryClass::V => Self::find_canonical_orientation_d3(&self.onoro, &symm_state),
      SymmetryClass::E => Self::find_canonical_orientation_k4(&self.onoro, &symm_state),
      SymmetryClass::CV => Self::find_canonical_orientation_c2_cv(&self.onoro, &symm_state),
      SymmetryClass::CE => Self::find_canonical_orientation_c2_ce(&self.onoro, &symm_state),
      SymmetryClass::EV => Self::find_canonical_orientation_c2_ev(&self.onoro, &symm_state),
      SymmetryClass::Trivial => Self::find_canonical_orientation_trivial(&self.onoro, &symm_state),
    };

    unsafe {
      *self.view.get() = CanonicalView {
        initialized: true,
        symm_class: symm_state.symm_class,
        op_ord,
        hash,
      };
    }
  }

  fn find_canonical_orientation_d6(
    onoro: &Onoro<N, N2, ADJ_CNT_SIZE>,
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
    onoro: &Onoro<N, N2, ADJ_CNT_SIZE>,
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
    onoro: &Onoro<N, N2, ADJ_CNT_SIZE>,
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
    onoro: &Onoro<N, N2, ADJ_CNT_SIZE>,
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
    onoro: &Onoro<N, N2, ADJ_CNT_SIZE>,
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
    onoro: &Onoro<N, N2, ADJ_CNT_SIZE>,
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
    onoro: &Onoro<N, N2, ADJ_CNT_SIZE>,
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
    self.maybe_initialize_canonical_view();

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
    view1: &OnoroView<N, N2, ADJ_CNT_SIZE>,
    view2: &OnoroView<N, N2, ADJ_CNT_SIZE>,
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

impl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> PartialEq
  for OnoroView<N, N2, ADJ_CNT_SIZE>
{
  fn eq(&self, other: &Self) -> bool {
    self.maybe_initialize_canonical_view();
    other.maybe_initialize_canonical_view();

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

impl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> Eq
  for OnoroView<N, N2, ADJ_CNT_SIZE>
{
}

impl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> Hash
  for OnoroView<N, N2, ADJ_CNT_SIZE>
{
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.maybe_initialize_canonical_view();
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
unsafe impl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> Send
  for OnoroView<N, N2, ADJ_CNT_SIZE>
{
}
unsafe impl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> Sync
  for OnoroView<N, N2, ADJ_CNT_SIZE>
{
}

impl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> Display
  for OnoroView<N, N2, ADJ_CNT_SIZE>
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    self.maybe_initialize_canonical_view();

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

pub struct ViewMoveGenerator<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> {
  move_gen: MoveGenerator<N, N2, ADJ_CNT_SIZE>,
}

impl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> GameMoveGenerator
  for ViewMoveGenerator<N, N2, ADJ_CNT_SIZE>
{
  type Item = Move;
  type Game = OnoroView<N, N2, ADJ_CNT_SIZE>;

  fn next(&mut self, view: &Self::Game) -> Option<Self::Item> {
    self.move_gen.next(view.onoro())
  }
}

impl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> Game
  for OnoroView<N, N2, ADJ_CNT_SIZE>
{
  type Move = Move;
  type MoveGenerator = ViewMoveGenerator<N, N2, ADJ_CNT_SIZE>;
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
    let Some((start_pawn_pos, start_pawn_color)) = self.pawns().min_by_key(|(pos, _)| *pos) else {
      return 0;
    };

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

    position_bits
      .into_iter()
      .chain(color_bits.iter().map(|c| matches!(c, PawnColor::Black)))
      .enumerate()
      .fold(0, |acc, (idx, set)| {
        acc | ((if set { 1 } else { 0 }) << idx)
      })
  }

  fn decompress(repr: u64) -> Self {
    todo!()
  }
}

impl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> Clone
  for OnoroView<N, N2, ADJ_CNT_SIZE>
{
  fn clone(&self) -> Self {
    Self {
      onoro: self.onoro.clone(),
      view: self.canon_view().clone().into(),
    }
  }
}

#[cfg(test)]
mod tests {
  use googletest::{assert_that, gtest, prelude::container_eq};
  use itertools::{Either, Itertools};

  use crate::{
    groups::SymmetryClass, hex_pos::HexPosOffset, Onoro16, Onoro16View, OnoroView, PawnColor,
  };

  fn build_view(board_layout: &str) -> Onoro16View {
    let view = OnoroView::new(Onoro16::from_board_string(board_layout).unwrap());
    view.maybe_initialize_canonical_view();
    view
  }

  fn verify_pawn_iter<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize>(
    view1: &OnoroView<N, N2, ADJ_CNT_SIZE>,
    view2: &OnoroView<N, N2, ADJ_CNT_SIZE>,
  ) {
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

  fn expect_view_eq<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize>(
    view1: &OnoroView<N, N2, ADJ_CNT_SIZE>,
    view2: &OnoroView<N, N2, ADJ_CNT_SIZE>,
  ) {
    assert_eq!(view1, view2);
    verify_pawn_iter(view1, view2);
  }

  fn expect_view_ne<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize>(
    view1: &OnoroView<N, N2, ADJ_CNT_SIZE>,
    view2: &OnoroView<N, N2, ADJ_CNT_SIZE>,
  ) {
    assert_ne!(view1, view2);
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
}
