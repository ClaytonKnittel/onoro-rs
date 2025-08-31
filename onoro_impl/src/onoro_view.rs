use std::{
  collections::{HashMap, HashSet},
  fmt::{Debug, Display},
  hash::Hash,
};

use algebra::{
  finite::Finite,
  group::{Group, Trivial},
  ordinal::Ordinal,
};

use abstract_game::{Game, GameMoveIterator, GameResult};
use onoro::{
  Compress, Onoro, PawnColor, TileState,
  error::{OnoroError, OnoroResult},
  groups::{C2, D3, D6, K4, SymmetryClass},
  hex_pos::{HexPos, HexPosOffset},
};

use crate::{
  MoveGenerator, OnoroImpl, PackedIdx,
  canonical_view::CanonicalView,
  canonicalize::board_symm_state,
  r#move::Move,
  onoro_defs::{Onoro16, Onoro16View},
  pawn_list::PawnList8,
  util::{unlikely, unreachable},
};

/// A wrapper over Onoro states that caches the hash of the game state and it's
/// canonicalizing symmetry operations. These cached values are used for quicker
/// equality comparison between different Onoro game states which may be in
/// different orientations.
#[derive(Clone)]
pub struct OnoroView<const N: usize> {
  onoro: OnoroImpl<N>,
  view: CanonicalView,
}

impl<const N: usize> OnoroView<N> {
  pub fn new(onoro: OnoroImpl<N>) -> Self {
    let view = CanonicalView::find_canonical_view(&onoro);
    Self { onoro, view }
  }

  pub fn onoro(&self) -> &OnoroImpl<N> {
    &self.onoro
  }

  fn canon_view(&self) -> &CanonicalView {
    &self.view
  }

  pub fn hash(&self) -> u64 {
    self.canon_view().hash()
  }

  fn pawns_equal_with_transform<F>(
    onoro1: &OnoroImpl<N>,
    onoro2: &OnoroImpl<N>,
    mut apply_view_transform: F,
  ) -> bool
  where
    F: FnMut(&HexPosOffset) -> HexPosOffset,
  {
    let symm_state1 = board_symm_state(onoro1);
    let symm_state2 = board_symm_state(onoro2);
    let normalizing_op1 = symm_state1.op;
    let denormalizing_op2 = symm_state2.op.inverse();
    let origin1 = onoro1.origin(&symm_state1);
    let origin2 = onoro2.origin(&symm_state2);

    let same_color_turn = onoro1.player_color() == onoro2.player_color();

    onoro1.pawns().all(|pawn| {
      let normalized_pos1 = (HexPos::from(pawn.pos) - origin1).apply_d6_c(&normalizing_op1);
      let normalized_pos2 = apply_view_transform(&normalized_pos1);
      let pos2 = normalized_pos2.apply_d6_c(&denormalizing_op2) + origin2;

      let Some(pos2) = PackedIdx::maybe_from(pos2) else {
        return false;
      };

      match onoro2.get_tile(pos2) {
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

  #[target_feature(enable = "sse4.1")]
  fn pawns_equal_with_transform_fast(
    onoro1: &OnoroImpl<N>,
    onoro2: &OnoroImpl<N>,
    symm_class: SymmetryClass,
    to_view2: u8,
  ) -> bool {
    if const { N != 16 } {
      unreachable()
    }

    let symm_state1 = board_symm_state(onoro1);
    let symm_state2 = board_symm_state(onoro2);
    let normalizing_op1 = symm_state1.op;
    let normalizing_op2 = symm_state2.op;
    let origin1 = onoro1.origin(&symm_state1);
    let origin2 = onoro2.origin(&symm_state2);

    let pawn_poses1: &[PackedIdx; 16] =
      unsafe { (onoro1.pawn_poses() as &[_]).try_into().unwrap_unchecked() };
    let pawn_poses2: &[PackedIdx; 16] =
      unsafe { (onoro2.pawn_poses() as &[_]).try_into().unwrap_unchecked() };
    let black_pawns1 = PawnList8::extract_black_pawns(pawn_poses1, origin1);
    let white_pawns1 = PawnList8::extract_white_pawns(pawn_poses1, origin1);
    let black_pawns2 = PawnList8::extract_black_pawns(pawn_poses2, origin2);
    let white_pawns2 = PawnList8::extract_white_pawns(pawn_poses2, origin2);

    let black_pawns1 = black_pawns1
      .apply_d6_c(&normalizing_op1)
      .apply(symm_class, to_view2);
    let white_pawns1 = white_pawns1
      .apply_d6_c(&normalizing_op1)
      .apply(symm_class, to_view2);
    let black_pawns2 = black_pawns2.apply_d6_c(&normalizing_op2);
    let white_pawns2 = white_pawns2.apply_d6_c(&normalizing_op2);

    let (black_pawns2, white_pawns2) = if onoro1.player_color() == onoro2.player_color() {
      (black_pawns2, white_pawns2)
    } else {
      (white_pawns2, black_pawns2)
    };

    black_pawns1.equal_ignoring_order(black_pawns2)
      && white_pawns1.equal_ignoring_order(white_pawns2)
  }

  fn cmp_views_in_symm_class<G: Group + Ordinal + Clone + Display, F>(
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

    let canon_op1 = G::from_ord(view1.canon_view().op_ord() as usize);
    let canon_op2 = G::from_ord(view2.canon_view().op_ord() as usize);
    let to_view2 = canon_op2.inverse() * canon_op1;

    let pawns_equal =
      Self::pawns_equal_with_transform(onoro1, onoro2, |pos| apply_view_transform(pos, &to_view2));

    // In the extremely unlikely case of a hash collision, the best-guess
    // canonical orientations may not have produced equal orientations. As a
    // fallback, we can simply check every other possible view transform.
    if unlikely(!pawns_equal) {
      return G::for_each().skip(1).any(|op| {
        debug_assert!(op != G::identity());
        let to_view2 = op * to_view2.clone();
        Self::pawns_equal_with_transform(onoro1, onoro2, |pos| apply_view_transform(pos, &to_view2))
      });
    }

    pawns_equal
  }

  #[target_feature(enable = "sse4.1")]
  fn cmp_views_in_symm_class_fast(view1: &OnoroView<N>, view2: &OnoroView<N>) -> bool {
    let onoro1 = &view1.onoro;
    let onoro2 = &view2.onoro;

    if onoro1.pawns_in_play() != onoro2.pawns_in_play() {
      return false;
    }

    let op1 = view1.canon_view().op_ord() as usize;
    let op2 = view2.canon_view().op_ord() as usize;
    let symm_class = view1.canon_view().symm_class();
    let to_view2 = match symm_class {
      SymmetryClass::C => (D6::from_ord(op2).inverse() * D6::from_ord(op1)).ord(),
      SymmetryClass::V => (D3::from_ord(op2).inverse() * D3::from_ord(op1)).ord(),
      SymmetryClass::E => (K4::from_ord(op2).inverse() * K4::from_ord(op1)).ord(),
      SymmetryClass::CV | SymmetryClass::CE | SymmetryClass::EV => {
        (C2::from_ord(op2).inverse() * C2::from_ord(op1)).ord()
      }
      SymmetryClass::Trivial => 0,
    };

    let pawns_equal =
      Self::pawns_equal_with_transform_fast(onoro1, onoro2, symm_class, to_view2 as u8);

    // In the extremely unlikely case of a hash collision, the best-guess
    // canonical orientations may not have produced equal orientations. As a
    // fallback, we can simply check every other possible view transform.
    if unlikely(!pawns_equal) {
      let group_size = match symm_class {
        SymmetryClass::C => D6::SIZE,
        SymmetryClass::V => D3::SIZE,
        SymmetryClass::E => K4::SIZE,
        SymmetryClass::CV | SymmetryClass::CE | SymmetryClass::EV => C2::SIZE,
        SymmetryClass::Trivial => Trivial::SIZE,
      };
      return (1..group_size).any(|i| {
        let to_view2 = (to_view2 + i) % group_size;
        Self::pawns_equal_with_transform_fast(onoro1, onoro2, symm_class, to_view2 as u8)
      });
    }

    pawns_equal
  }

  fn cmp_views_dispatch(&self, other: &Self) -> bool {
    match self.canon_view().symm_class() {
      SymmetryClass::C => Self::cmp_views_in_symm_class(self, other, HexPosOffset::apply_d6_c),
      SymmetryClass::V => Self::cmp_views_in_symm_class(self, other, HexPosOffset::apply_d3_v),
      SymmetryClass::E => Self::cmp_views_in_symm_class(self, other, HexPosOffset::apply_k4_e),
      SymmetryClass::CV => Self::cmp_views_in_symm_class(self, other, HexPosOffset::apply_c2_cv),
      SymmetryClass::CE => Self::cmp_views_in_symm_class(self, other, HexPosOffset::apply_c2_ce),
      SymmetryClass::EV => Self::cmp_views_in_symm_class(self, other, HexPosOffset::apply_c2_ev),
      SymmetryClass::Trivial => {
        Self::cmp_views_in_symm_class(self, other, HexPosOffset::apply_trivial)
      }
    }
  }

  pub(crate) fn cmp_views(&self, other: &Self) -> bool {
    if self.canon_view().symm_class() != other.canon_view().symm_class() {
      return false;
    }

    #[cfg(target_feature = "sse4.1")]
    if const { N == 16 } {
      return unsafe { Self::cmp_views_in_symm_class_fast(self, other) };
    }
    self.cmp_views_dispatch(other)
  }
}

impl<const N: usize> PartialEq for OnoroView<N> {
  fn eq(&self, other: &Self) -> bool {
    if self.canon_view().hash() != other.canon_view().hash() {
      return false;
    }

    self.cmp_views(other)
  }
}

impl<const N: usize> Eq for OnoroView<N> {}

impl<const N: usize> Hash for OnoroView<N> {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    state.write_u64(self.canon_view().hash());
  }
}

impl<const N: usize> Display for OnoroView<N> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let symm_state = board_symm_state(self.onoro());
    let rotated = self.onoro().rotated_d6_c(symm_state.op);
    let _rotated = match self.canon_view().symm_class() {
      SymmetryClass::C => rotated.rotated_d6_c(D6::from_ord(self.canon_view().op_ord() as usize)),
      SymmetryClass::V => rotated.rotated_d3_v(D3::from_ord(self.canon_view().op_ord() as usize)),
      SymmetryClass::E => rotated.rotated_k4_e(K4::from_ord(self.canon_view().op_ord() as usize)),
      SymmetryClass::CV => rotated.rotated_c2_cv(C2::from_ord(self.canon_view().op_ord() as usize)),
      SymmetryClass::CE => rotated.rotated_c2_ce(C2::from_ord(self.canon_view().op_ord() as usize)),
      SymmetryClass::EV => rotated.rotated_c2_ev(C2::from_ord(self.canon_view().op_ord() as usize)),
      SymmetryClass::Trivial => rotated,
    };

    write!(
      f,
      "{}\n{:?}: canon: {}, normalize: {} ({:#018x?})",
      self.onoro,
      self.canon_view().symm_class(),
      symm_state.op,
      match self.canon_view().symm_class() {
        SymmetryClass::C => D6::from_ord(self.canon_view().op_ord() as usize).to_string(),
        SymmetryClass::V => D3::from_ord(self.canon_view().op_ord() as usize).to_string(),
        SymmetryClass::E => K4::from_ord(self.canon_view().op_ord() as usize).to_string(),
        SymmetryClass::CV | SymmetryClass::CE | SymmetryClass::EV =>
          C2::from_ord(self.canon_view().op_ord() as usize).to_string(),
        SymmetryClass::Trivial =>
          Trivial::from_ord(self.canon_view().op_ord() as usize).to_string(),
      },
      self.canon_view().hash()
    )
  }
}

impl<const N: usize> Debug for OnoroView<N> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{self}")
  }
}

pub struct ViewMoveGenerator<const N: usize>(MoveGenerator<N>);

impl<const N: usize> GameMoveIterator for ViewMoveGenerator<N> {
  type Item = Move;
  type Game = OnoroView<N>;

  fn next(&mut self, view: &Self::Game) -> Option<Self::Item> {
    self.0.next(view.onoro())
  }
}

impl<const N: usize> Game for OnoroView<N> {
  type Move = Move;
  type MoveGenerator = ViewMoveGenerator<N>;
  type PlayerIdentifier = PawnColor;

  fn move_generator(&self) -> Self::MoveGenerator {
    ViewMoveGenerator(self.onoro().each_move_gen())
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
    let pawn_colors: HashMap<HexPosOffset, _> = self
      .onoro()
      .pawns()
      .map(|pawn| (pawn.pos.into(), pawn.color))
      .collect();
    let (start_pawn_pos, start_pawn_color) = self
      .onoro()
      .pawns()
      .map(|pawn| (pawn.pos.into(), pawn.color))
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
    expect_that, gtest,
    prelude::{any, anything},
  };

  use onoro::{
    Compress, Onoro, PawnColor, error::OnoroResult, groups::SymmetryClass, hex_pos::HexPosOffset,
  };

  use super::{Onoro16, Onoro16View, OnoroView};

  fn build_view(board_layout: &str) -> Onoro16View {
    OnoroView::new(Onoro16::from_board_string(board_layout).unwrap())
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

    assert_eq!(view1.canon_view().symm_class(), SymmetryClass::V);

    assert_eq!(&view1, &view2);
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

    assert_eq!(view1.canon_view().symm_class(), SymmetryClass::C);
    assert_eq!(view3.canon_view().symm_class(), SymmetryClass::C);
    assert_eq!(view4.canon_view().symm_class(), SymmetryClass::C);

    assert_eq!(&view1, &view2);
    assert_ne!(&view1, &view3);
    assert_ne!(&view2, &view3);
    assert_ne!(&view1, &view4);
    assert_ne!(&view2, &view4);
    assert_ne!(&view3, &view4);
    assert_ne!(&view1, &view5);
    assert_ne!(&view2, &view5);
    assert_ne!(&view3, &view5);
    assert_eq!(&view4, &view5);
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

    assert_eq!(view1.canon_view().symm_class(), SymmetryClass::C);
    assert_eq!(view3.canon_view().symm_class(), SymmetryClass::C);

    assert_eq!(&view1, &view2);
    assert_ne!(&view1, &view3);
    assert_ne!(&view2, &view3);
    assert_ne!(&view1, &view4);
    assert_ne!(&view2, &view4);
    assert_eq!(&view3, &view4);
  }

  #[test]
  #[allow(non_snake_case)]
  fn test_C_symm_3() {
    let view1 = build_view(
      ". . . . . . .
        . . . . B B .
         . . B W B W .
          . . B B W . .
           . B W B W . .
            . W W . . . .
             . . . . . . .",
    );
    let view2 = build_view(
      ". . . . . . .
        . . . W W . .
         . . B W B W .
          . . B B W . .
           . B W B W . .
            . . B B . . .
             . . . . . . .",
    );

    assert_eq!(view1.canon_view().symm_class(), SymmetryClass::C);
    assert_eq!(view2.canon_view().symm_class(), SymmetryClass::C);

    assert_eq!(&view1, &view2);
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

    assert_eq!(view1.canon_view().symm_class(), SymmetryClass::E);
    assert_eq!(view3.canon_view().symm_class(), SymmetryClass::E);

    assert_eq!(&view1, &view2);
    assert_ne!(&view1, &view3);
    assert_ne!(&view2, &view3);
    assert_ne!(&view1, &view4);
    assert_ne!(&view2, &view4);
    assert_eq!(&view3, &view4);
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

    assert_eq!(view1.canon_view().symm_class(), SymmetryClass::CV);
    assert_eq!(view3.canon_view().symm_class(), SymmetryClass::CV);

    assert_eq!(&view1, &view2);
    assert_ne!(&view1, &view3);
    assert_ne!(&view2, &view3);
    assert_ne!(&view1, &view4);
    assert_ne!(&view2, &view4);
    assert_eq!(&view3, &view4);
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
      assert_eq!(views[i].canon_view().symm_class(), SymmetryClass::CV);
      for j in 0..i {
        let view1 = &views[j];
        let view2 = &views[i];

        assert_eq!(view1, view2);
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

    assert_eq!(view1.canon_view().symm_class(), SymmetryClass::CE);
    assert_eq!(view3.canon_view().symm_class(), SymmetryClass::CE);

    assert_eq!(&view1, &view2);
    assert_ne!(&view1, &view3);
    assert_ne!(&view2, &view3);
    assert_ne!(&view1, &view4);
    assert_ne!(&view2, &view4);
    assert_eq!(&view3, &view4);
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

    assert_eq!(view1.canon_view().symm_class(), SymmetryClass::EV);
    assert_eq!(view3.canon_view().symm_class(), SymmetryClass::EV);

    assert_eq!(&view1, &view2);
    assert_ne!(&view1, &view3);
    assert_ne!(&view2, &view3);
    assert_ne!(&view1, &view4);
    assert_ne!(&view2, &view4);
    assert_eq!(&view3, &view4);
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

    assert_eq!(view1.canon_view().symm_class(), SymmetryClass::Trivial);
    assert_eq!(view3.canon_view().symm_class(), SymmetryClass::Trivial);

    assert_eq!(&view1, &view2);
    assert_ne!(&view1, &view3);
    assert_ne!(&view2, &view3);
    assert_ne!(&view1, &view4);
    assert_ne!(&view2, &view4);
    assert_eq!(&view3, &view4);
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
    assert_eq!(&view, &compress_round_trip(&view)?);
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
    assert_eq!(&view, &compress_round_trip(&view)?);
    Ok(())
  }
}
