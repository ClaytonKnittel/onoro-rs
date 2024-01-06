mod error;
mod file_server;
mod initialize;
mod proto;
mod socket_init;

#[tokio::main]
async fn main() {
  initialize::init().await;
}
