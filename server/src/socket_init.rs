use serde::Deserialize;
use std::time::Duration;

use async_sockets::{
  AsyncSocket, AsyncSocketContext, AsyncSocketEmitters, AsyncSocketListeners, AsyncSocketOptions,
  AsyncSocketResponders, Status,
};
use onoro::Onoro16;
use tokio::task::JoinHandle;

use crate::proto::GameStateProto;

#[derive(AsyncSocketEmitters)]
enum ServerEmitEvents {}

#[derive(AsyncSocketListeners)]
enum ClientEmitEvents {}

#[derive(AsyncSocketEmitters)]
enum ToClientRequests {}

#[derive(Deserialize)]
enum FromClientResponses {}

#[derive(AsyncSocketListeners)]
enum FromClientRequests {
  NewGame {},
}

#[derive(AsyncSocketResponders)]
enum ToClientResponses {
  NewGame { game: GameStateProto },
}

async fn handle_connect_event(_context: AsyncSocketContext<ServerEmitEvents>) {}

async fn handle_call_event(
  event: FromClientRequests,
  _context: AsyncSocketContext<ServerEmitEvents>,
) -> Status<ToClientResponses> {
  match event {
    FromClientRequests::NewGame {} => Status::Ok(ToClientResponses::NewGame {
      game: GameStateProto::from_onoro(&Onoro16::default_start()),
    }),
  }
}

async fn handle_emit_event(
  event: ClientEmitEvents,
  _context: AsyncSocketContext<ServerEmitEvents>,
) {
  match event {}
}

pub fn create_socket_endpoint() -> JoinHandle<()> {
  tokio::spawn(async {
    AsyncSocket::new(
      AsyncSocketOptions::new()
        .with_path("onoro")
        .with_port(2345)
        .with_timeout(Duration::from_secs(10)),
      handle_connect_event,
      handle_emit_event,
      handle_call_event,
    )
    .start_server()
    .await
  })
}
