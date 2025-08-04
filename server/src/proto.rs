use async_sockets::Status;
use bytes::BytesMut;
use itertools::interleave;
use onoro::{Move, Onoro, OnoroImpl, PackedIdx, Pawn, PawnColor};
use prost::Message;
use serde::{
  de::{self, Visitor},
  ser, Deserialize, Deserializer, Serialize, Serializer,
};

use crate::error::Error;

mod proto_impl {
  include!(concat!(env!("OUT_DIR"), "/onoro.proto.rs"));
}

#[derive(Debug)]
struct BytesMutVisitor;

impl<'de> Visitor<'de> for BytesMutVisitor {
  type Value = BytesMut;

  fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    formatter.write_str("Expecting bytes")
  }

  fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
  where
    E: serde::de::Error,
  {
    Ok(BytesMut::from(v))
  }
}

pub struct GameStateProto {
  game_state: proto_impl::GameState,
}

impl GameStateProto {
  pub fn from_onoro<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize>(
    onoro: &OnoroImpl<N, N2, ADJ_CNT_SIZE>,
  ) -> Self {
    Self {
      game_state: proto_impl::GameState {
        pawns: onoro
          .pawns()
          .map(|pawn| proto_impl::game_state::Pawn {
            x: Some(pawn.pos.x() as i32),
            y: Some(pawn.pos.y() as i32),
            black: Some(pawn.color == PawnColor::Black),
          })
          .collect(),
        black_turn: Some(onoro.player_color() == PawnColor::Black),
        turn_num: Some(onoro.pawns_in_play() - 1),
        finished: Some(onoro.finished().is_some()),
      },
    }
  }

  pub fn to_onoro<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize>(
    &self,
  ) -> Result<OnoroImpl<N, N2, ADJ_CNT_SIZE>, Error> {
    let mut black_moves = Vec::new();
    let mut while_moves = Vec::new();

    let [min_x, min_y] = self
      .game_state
      .pawns
      .iter()
      .filter_map(|pawn| Some([pawn.x?, pawn.y?]))
      .reduce(|[min_x, min_y], [x, y]| [min_x.min(x), min_y.min(y)])
      .map(Ok)
      .unwrap_or(Err(Error::ProtoDecode("No valid pawns".into())))?;

    for pawn_proto in &self.game_state.pawns {
      let x = (pawn_proto.x().wrapping_sub(min_x).wrapping_add(1)) as u32;
      let y = (pawn_proto.y().wrapping_sub(min_y).wrapping_add(1)) as u32;
      if x >= N as u32 || y >= N as u32 {
        return Err(Error::ProtoDecode(format!(
          "x/y out of bounds: {} {}",
          pawn_proto.x(),
          pawn_proto.y()
        )));
      }
      let m = Move::Phase1Move {
        to: PackedIdx::new(x, y),
      };

      if pawn_proto.black() {
        black_moves.push(m);
      } else {
        while_moves.push(m);
      }
    }

    if black_moves.len() > N || while_moves.len() > N {
      return Err(Error::ProtoDecode(format!(
        "Too many pawns in board: {} black and {} white",
        black_moves.len(),
        while_moves.len()
      )));
    }

    if black_moves.is_empty() {
      return Err(Error::ProtoDecode(
        "Must have at least one black pawn placed, since they are the first player.".into(),
      ));
    }

    if !((black_moves.len() - 1)..=black_moves.len()).contains(&while_moves.len()) {
      return Err(Error::ProtoDecode(format!(
        "There must be either one fewer or equally many white pawns as there are black. Found {} black and {} white.",
        black_moves.len(), while_moves.len()
      )));
    }

    let mut game = unsafe { OnoroImpl::new() };
    unsafe {
      game.make_move_unchecked(black_moves[0]);
    }
    for m in interleave(while_moves, black_moves.into_iter().skip(1)) {
      game.make_move(m);
    }

    Ok(game)
  }
}

impl<const N: usize, const N2: usize, const ADJ_CNT_SIZE: usize>
  From<&OnoroImpl<N, N2, ADJ_CNT_SIZE>> for GameStateProto
{
  fn from(onoro: &OnoroImpl<N, N2, ADJ_CNT_SIZE>) -> Self {
    Self::from_onoro(onoro)
  }
}

impl Serialize for GameStateProto {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let mut buf = BytesMut::new();
    self
      .game_state
      .encode(&mut buf)
      .map_err(ser::Error::custom)?;
    serializer.serialize_bytes(&buf)
  }
}

impl<'de> Deserialize<'de> for GameStateProto {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    let buf = deserializer.deserialize_bytes(BytesMutVisitor)?;
    let game_state = proto_impl::GameState::decode(buf).map_err(de::Error::custom)?;
    Ok(GameStateProto { game_state })
  }
}
