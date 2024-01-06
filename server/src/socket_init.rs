use std::time::Duration;

use async_sockets::{
  AsyncSocket, AsyncSocketContext, AsyncSocketEmitters, AsyncSocketListeners, AsyncSocketOptions,
  AsyncSocketResponders, Status,
};
use tokio::task::JoinHandle;

#[derive(AsyncSocketEmitters)]
enum ServerEmitEvents {}

#[derive(AsyncSocketListeners)]
enum ClientEmitEvents {}

#[derive(AsyncSocketEmitters)]
enum ToClientRequests {}

enum FromClientResponses {}

#[derive(AsyncSocketListeners)]
enum FromClientRequests {
  Test1 { id: u64 },
}

#[derive(AsyncSocketResponders)]
enum ToClientResponses {
  Test1 { id: u64 },
}

async fn handle_connect_event(_context: AsyncSocketContext<ServerEmitEvents>) {}

async fn handle_call_event(
  event: FromClientRequests,
  _context: AsyncSocketContext<ServerEmitEvents>,
) -> Status<ToClientResponses> {
  match event {
    FromClientRequests::Test1 { id } => Status::Ok(ToClientResponses::Test1 { id: id + 1 }),
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
