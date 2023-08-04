use std::{fmt::Display, hash::Hash};

use algebra::{
  group::{Group, Trivial},
  monoid::Monoid,
  ordinal::Ordinal,
};

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

/// A wrapper over Onoro states that caches the hash of the game state and it's
/// canonicalizing symmetry operations. These caches values are used for quicker
/// equality comparison between different Onoro game states which may be in
/// different orientations.
#[derive(Debug)]
pub struct OnoroView<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> {
  onoro: Onoro<N, N2, ADJ_CNT_SIZE>,
  symm_class: SymmetryClass,
  op_ord: u8,
  hash: u64,
}

impl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> OnoroView<N, N2, ADJ_CNT_SIZE> {
  pub fn new(onoro: Onoro<N, N2, ADJ_CNT_SIZE>) -> Self {
    let symm_state = board_symm_state(&onoro);

    let (hash, op_ord) = match symm_state.symm_class {
      SymmetryClass::C => Self::find_canonical_orientation_d6(&onoro, &symm_state),
      SymmetryClass::V => Self::find_canonical_orientation_d3(&onoro, &symm_state),
      SymmetryClass::E => Self::find_canonical_orientation_k4(&onoro, &symm_state),
      SymmetryClass::CV => Self::find_canonical_orientation_c2_cv(&onoro, &symm_state),
      SymmetryClass::CE => Self::find_canonical_orientation_c2_ce(&onoro, &symm_state),
      SymmetryClass::EV => Self::find_canonical_orientation_c2_ev(&onoro, &symm_state),
      SymmetryClass::Trivial => Self::find_canonical_orientation_trivial(&onoro, &symm_state),
    };

    Self {
      onoro,
      symm_class: symm_state.symm_class,
      op_ord,
      hash,
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

    let canon_op1 = G::from_ord(view1.op_ord as usize);
    let canon_op2 = G::from_ord(view2.op_ord as usize);
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
    if self.hash != other.hash || self.symm_class != other.symm_class {
      return false;
    }

    match self.symm_class {
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
    state.write_u64(self.hash);
  }
}

impl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize> Display
  for OnoroView<N, N2, ADJ_CNT_SIZE>
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "{}\n{:?}: {} ({:#018x?})",
      self.onoro,
      self.symm_class,
      match self.symm_class {
        SymmetryClass::C => D6::from_ord(self.op_ord as usize).to_string(),
        SymmetryClass::V => D3::from_ord(self.op_ord as usize).to_string(),
        SymmetryClass::E => K4::from_ord(self.op_ord as usize).to_string(),
        SymmetryClass::CV | SymmetryClass::CE | SymmetryClass::EV =>
          C2::from_ord(self.op_ord as usize).to_string(),
        SymmetryClass::Trivial => Trivial::from_ord(self.op_ord as usize).to_string(),
      },
      self.hash
    )
  }
}

#[cfg(test)]
mod tests {
  use algebra::group::Cyclic;

  use crate::{
    groups::{SymmetryClass, D6},
    hex_pos::HexPos,
    Onoro16, OnoroView,
  };

  #[test]
  #[allow(non_snake_case)]
  fn test_V_symm_simple() {
    let view1 = OnoroView::new(
      Onoro16::from_board_string(
        ". W
          B B",
      )
      .unwrap(),
    );
    let view2 = OnoroView::new(
      Onoro16::from_board_string(
        ". B
          B W",
      )
      .unwrap(),
    );

    assert_eq!(view1.symm_class, SymmetryClass::V);

    assert_eq!(view1, view2);
  }

  #[test]
  #[allow(non_snake_case)]
  fn test_C_symm_simple() {
    let view1 = OnoroView::new(
      Onoro16::from_board_string(
        ". W B
          B . W
           W B",
      )
      .unwrap(),
    );
    let view2 = OnoroView::new(
      Onoro16::from_board_string(
        ". B W
          W . B
           B W",
      )
      .unwrap(),
    );
    let view3 = OnoroView::new(
      Onoro16::from_board_string(
        ". B W
          W B B
           B W",
      )
      .unwrap(),
    );
    let view4 = OnoroView::new(
      Onoro16::from_board_string(
        ". . B
          W . .
           . B",
      )
      .unwrap(),
    );
    let view5 = OnoroView::new(
      Onoro16::from_board_string(
        ". B .
          . . B
           W .",
      )
      .unwrap(),
    );

    assert_eq!(view1.symm_class, SymmetryClass::C);
    assert_eq!(view3.symm_class, SymmetryClass::C);
    assert_eq!(view4.symm_class, SymmetryClass::C);

    assert_eq!(view1, view2);
    assert_ne!(view1, view3);
    assert_ne!(view2, view3);
    assert_ne!(view1, view4);
    assert_ne!(view2, view4);
    assert_ne!(view3, view4);
    assert_ne!(view1, view5);
    assert_ne!(view2, view5);
    assert_ne!(view3, view5);
    assert_eq!(view4, view5);
  }

  #[test]
  #[allow(non_snake_case)]
  fn test_C_symm_2() {
    let view1 = OnoroView::new(
      Onoro16::from_board_string(
        ". W B
          . . .
           W B .",
      )
      .unwrap(),
    );
    let view2 = OnoroView::new(
      Onoro16::from_board_string(
        ". B .
          W . B
           . W .",
      )
      .unwrap(),
    );
    let view3 = OnoroView::new(
      Onoro16::from_board_string(
        ". W .
          B . B
           . W .",
      )
      .unwrap(),
    );
    let view4 = OnoroView::new(
      Onoro16::from_board_string(
        ". . W
          B . B
           W . .",
      )
      .unwrap(),
    );

    assert_eq!(view1.symm_class, SymmetryClass::C);
    assert_eq!(view3.symm_class, SymmetryClass::C);

    assert_eq!(view1, view2);
    assert_ne!(view1, view3);
    assert_ne!(view2, view3);
    assert_ne!(view1, view4);
    assert_ne!(view2, view4);
    assert_eq!(view3, view4);
  }

  #[test]
  #[allow(non_snake_case)]
  fn test_E_symm_simple() {
    let view1 = OnoroView::new(
      Onoro16::from_board_string(
        ". W . B
          . . . .
           W . B .",
      )
      .unwrap(),
    );
    let view2 = OnoroView::new(
      Onoro16::from_board_string(
        ". . B .
          . . . B
           W . . .
            . W . .",
      )
      .unwrap(),
    );
    let view3 = OnoroView::new(
      Onoro16::from_board_string(
        ". . W .
          . . . B
           B . . .
            . W . .",
      )
      .unwrap(),
    );
    let view4 = OnoroView::new(
      Onoro16::from_board_string(
        ". . W .
          B . . .
           . . B .
            W . . .",
      )
      .unwrap(),
    );

    assert_eq!(view1.symm_class, SymmetryClass::E);
    assert_eq!(view3.symm_class, SymmetryClass::E);

    assert_eq!(view1, view2);
    assert_ne!(view1, view3);
    assert_ne!(view2, view3);
    assert_ne!(view1, view4);
    assert_ne!(view2, view4);
    assert_eq!(view3, view4);
  }

  #[test]
  #[allow(non_snake_case)]
  fn test_CV_symm_simple() {
    let view1 = OnoroView::new(
      Onoro16::from_board_string(
        ". B . .
          W B . .
           . . . .
            . . . W",
      )
      .unwrap(),
    );
    let view2 = OnoroView::new(
      Onoro16::from_board_string(
        ". . . W
          . . . .
           . . . .
            . . . .
             . B . .
              W B . .",
      )
      .unwrap(),
    );
    let view3 = OnoroView::new(
      Onoro16::from_board_string(
        ". . B B
          . . W .
           . . . .
            . . . .
             . . . .
              W . . .",
      )
      .unwrap(),
    );
    let view4 = OnoroView::new(
      Onoro16::from_board_string(
        ". . . . . B
          . . . . W B
           . . . . . .
            W . . . . .",
      )
      .unwrap(),
    );

    assert_eq!(view1.symm_class, SymmetryClass::CV);
    assert_eq!(view3.symm_class, SymmetryClass::CV);

    assert_eq!(view1, view2);
    assert_ne!(view1, view3);
    assert_ne!(view2, view3);
    assert_ne!(view1, view4);
    assert_ne!(view2, view4);
    assert_eq!(view3, view4);
  }

  #[test]
  #[allow(non_snake_case)]
  fn test_CE_symm_simple() {
    let view1 = OnoroView::new(
      Onoro16::from_board_string(
        ". B
          W B
           . W",
      )
      .unwrap(),
    );
    let view2 = OnoroView::new(
      Onoro16::from_board_string(
        ". W .
          B B W",
      )
      .unwrap(),
    );
    let view3 = OnoroView::new(
      Onoro16::from_board_string(
        ". . B
          W B W",
      )
      .unwrap(),
    );
    let view4 = OnoroView::new(
      Onoro16::from_board_string(
        ". . W
          . B B
           W . .",
      )
      .unwrap(),
    );

    assert_eq!(view1.symm_class, SymmetryClass::CE);
    assert_eq!(view3.symm_class, SymmetryClass::CE);

    assert_eq!(view1, view2);
    assert_ne!(view1, view3);
    assert_ne!(view2, view3);
    assert_ne!(view1, view4);
    assert_ne!(view2, view4);
    assert_eq!(view3, view4);
  }

  #[test]
  #[allow(non_snake_case)]
  fn test_EV_symm_simple() {
    let view1 = OnoroView::new(
      Onoro16::from_board_string(
        ". B . B
          . . . .
           . . W .
            B . W .",
      )
      .unwrap(),
    );
    let view2 = OnoroView::new(
      Onoro16::from_board_string(
        ". . . B
          W W . .
           . . . B
            B . . .",
      )
      .unwrap(),
    );
    let view3 = OnoroView::new(
      Onoro16::from_board_string(
        ". . . B
          . . B .
           B . . W
            . . . .
             W . . .",
      )
      .unwrap(),
    );
    let view4 = OnoroView::new(
      Onoro16::from_board_string(
        ". W . B
          . . . .
           . . B .
            W . B .",
      )
      .unwrap(),
    );

    assert_eq!(view1.symm_class, SymmetryClass::EV);
    assert_eq!(view3.symm_class, SymmetryClass::EV);

    assert_eq!(view1, view2);
    assert_ne!(view1, view3);
    assert_ne!(view2, view3);
    assert_ne!(view1, view4);
    assert_ne!(view2, view4);
    assert_eq!(view3, view4);
  }
}
