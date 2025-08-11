use std::{collections::HashMap, fmt::Debug};

use itertools::Itertools;

use crate::{
  error::{OnoroError, OnoroResult},
  Onoro, OnoroIndex, OnoroMove, OnoroMoveWrapper, OnoroPawn, PawnColor,
};

pub const BOARD_POSITIONS: [&str; 12] = [
  ". W
    B B",
  ". W B
    B . W
     W B .",
  ". . . . . . . 
    . . W . . . . 
     . W W B B B . 
      . . . . W . . 
       . . . . W . . 
        . . . B B . . 
         . . . B B . . 
          . . . . W . . 
           . . . . W . . 
            . . . B W . . 
             . . . . . . .",
  ". . . . . . . . . . . 
    . . . . . . . . B W . 
     . . B B . . B W W W . 
      . . W . . W B . . . . 
       . . B . B . . . . . . 
        . B W W . . . . . . . 
         . . . . . . . . . . .",
  ". . . . . . . . . . . . 
    . . . . . . . . . W W . 
     . . . . . . . . . W . . 
      . . . . . B W . B . . . 
       . . . . . W B . B . . . 
        . . . . W . B W . . . . 
         . . . W . . . . . . . . 
          . . B . . . . . . . . . 
           . B B . . . . . . . . . 
            . . . . . . . . . . . .",
  ". . . . . . . . . . . . 
    . . . . . . . . . . B . 
     . . . . . . . W B W W . 
      . . . . . B B . . . . . 
       . . . W W . . . . . . . 
        . . B . W . . . . . . . 
         . B W . B B . . . . . . 
          . . . . W . . . . . . . 
           . . . . . . . . . . . .",
  ". . . . . . . . 
    . . . . . W . . 
     . . . . B B B . 
      . . B W . B . . 
       . . W . W . B . 
        . B W . B W W . 
         . . . . . W . . 
          . . . . . . . .",
  ". . . . . . . . . . . 
    . . . . . . . . . W . 
     . . . B W . . . W B . 
      . . W . W . B W . . . 
       . B B B . . W . . . . 
        . . . B B W . . . . . 
         . . . . . . . . . . .",
  ". . . . . . . . . . . 
    . . . . . . . . W B . 
     . . . . . . . B W . . 
      . . . . . . . B . . . 
       . . W . . . . W . . . 
        . B W . . . B . . . . 
         . . W B B W B . . . . 
          . . . . W . . . . . . 
           . . . . . . . . . . .",
  ". . . . . . . 
    . . . . . W . 
     . . . . W W . 
      . B W B . . . 
       . B B W W . . 
        . . . B . . . 
         . . . B . . . 
          . . . W B B . 
           . . . . W . . 
            . . . . . . .",
  ". . . . . . . . . . . . 
    . . . . . . . . . W W . 
     . . . . . . . . B B . . 
      . . . . . . W B W . . . 
       . . . . . B . B . . . . 
        . . . . . B . . . . . . 
         . . . W W B . . . . . . 
          . B W . . . . . . . . . 
           . W . . . . . . . . . . 
            . . . . . . . . . . . .",
  ". . . . . . 
    . . . B . . 
     . . W W W . 
      . . . B . . 
       . . B B . . 
        . . B . . . 
         . . W B . . 
          . . . W . . 
           . . . W . . 
            . . B . . . 
             . . B . . . 
              . W W . . . 
               . . . . . .",
];

pub type TupleMove = OnoroMoveWrapper<(i32, i32)>;

#[derive(Clone, Debug)]
pub struct ComparableMove<M: OnoroMove> {
  t_move: TupleMove,
  original: M,
}

impl<M: OnoroMove> ComparableMove<M> {
  pub fn original(&self) -> M {
    self.original.clone()
  }
}

impl<M1: OnoroMove, M2: OnoroMove> PartialEq<ComparableMove<M2>> for ComparableMove<M1> {
  fn eq(&self, other: &ComparableMove<M2>) -> bool {
    self.t_move.eq(&other.t_move)
  }
}
impl<M1: OnoroMove> Eq for ComparableMove<M1> {}

pub trait OnoroFactory {
  type T: Onoro<Index: Debug, Move: Debug, Pawn: Debug> + Clone + Debug;

  fn from_board_string(board_string: &str) -> OnoroResult<Self::T> {
    Ok(Self::T::from_board_string(board_string)?)
  }
}

fn to_tuple_move(m: OnoroMoveWrapper<impl OnoroIndex>) -> TupleMove {
  match m {
    OnoroMoveWrapper::Phase1 { to } => OnoroMoveWrapper::Phase1 {
      to: (to.x(), to.y()),
    },
    OnoroMoveWrapper::Phase2 { from, to } => OnoroMoveWrapper::Phase2 {
      from: (from.x(), from.y()),
      to: (to.x(), to.y()),
    },
  }
}

fn ordered_moves<T: Onoro>(onoro: &T) -> OnoroResult<Vec<ComparableMove<T::Move>>> {
  let mut moves = onoro
    .each_move()
    .map(|m| ComparableMove {
      t_move: to_tuple_move(onoro.to_move_wrapper(&m)),
      original: m,
    })
    .collect_vec();
  if !moves
    .iter()
    .map(|ComparableMove { t_move, .. }| matches!(t_move, OnoroMoveWrapper::Phase1 { .. }))
    .all_equal()
  {
    return Err(
      OnoroError::new("All moves must be uniformly phase 1 or phase 2".to_owned()).into(),
    );
  }

  moves.sort_by(
    |ComparableMove { t_move: lhs, .. }, ComparableMove { t_move: rhs, .. }| match (lhs, rhs) {
      (OnoroMoveWrapper::Phase1 { to: lhs }, OnoroMoveWrapper::Phase1 { to: rhs }) => {
        (lhs.x(), lhs.y()).cmp(&(rhs.x(), rhs.y()))
      }
      (
        OnoroMoveWrapper::Phase2 {
          from: lhs_from,
          to: lhs_to,
        },
        OnoroMoveWrapper::Phase2 {
          from: rhs_from,
          to: rhs_to,
        },
      ) => (lhs_from.x(), lhs_from.y())
        .cmp(&(rhs_from.x(), rhs_from.y()))
        .then((lhs_to.x(), lhs_to.y()).cmp(&(rhs_to.x(), rhs_to.y()))),
      _ => unreachable!(),
    },
  );

  Ok(moves)
}

/// Returns a list of all the moves that can be made from this position, sorted
/// and normalized with respect to translation.
pub fn normalized_ordered_moves<T: Onoro>(onoro: &T) -> OnoroResult<Vec<ComparableMove<T::Move>>> {
  let ordered_moves = ordered_moves(onoro)?;

  let min_x = ordered_moves
    .iter()
    .map(|ComparableMove { t_move, .. }| match t_move {
      OnoroMoveWrapper::Phase1 { to } => to.x(),
      OnoroMoveWrapper::Phase2 { from, to } => from.x().min(to.x()),
    })
    .min()
    .unwrap();
  let min_y = ordered_moves
    .iter()
    .map(|ComparableMove { t_move, .. }| match t_move {
      OnoroMoveWrapper::Phase1 { to } => to.y(),
      OnoroMoveWrapper::Phase2 { from, to } => from.y().min(to.y()),
    })
    .min()
    .unwrap();

  Ok(
    ordered_moves
      .into_iter()
      .map(|ComparableMove { t_move, original }| ComparableMove {
        t_move: match t_move {
          OnoroMoveWrapper::Phase1 { to } => OnoroMoveWrapper::Phase1 {
            to: (to.x() - min_x, to.y() - min_y),
          },
          OnoroMoveWrapper::Phase2 { from, to } => OnoroMoveWrapper::Phase2 {
            from: (from.x() - min_x, from.y() - min_y),
            to: (to.x() - min_x, to.y() - min_y),
          },
        },
        original,
      })
      .collect(),
  )
}

fn pawn_map<T: Onoro>(onoro: &T) -> HashMap<(i32, i32), PawnColor> {
  let min_x = onoro.pawns().map(|pawn| pawn.pos().x()).min().unwrap();
  let min_y = onoro.pawns().map(|pawn| pawn.pos().y()).min().unwrap();
  onoro
    .pawns()
    .map(|pawn| {
      (
        (pawn.pos().x() - min_x, pawn.pos().y() - min_y),
        pawn.color(),
      )
    })
    .collect()
}

#[derive(Debug, Clone, Copy)]
pub struct OnoroCmp<'a, T: Onoro + Debug>(pub &'a T);
impl<'a, T: Onoro + Debug, U: Onoro + Debug> PartialEq<OnoroCmp<'a, U>> for OnoroCmp<'a, T> {
  fn eq(&self, other: &OnoroCmp<'a, U>) -> bool {
    pawn_map(self.0) == pawn_map(other.0)
  }
}
impl<'a, T: Onoro + Debug> Eq for OnoroCmp<'a, T> {}
