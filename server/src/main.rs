mod file_server;
mod initialize;
mod socket_init;

#[tokio::main]
async fn main() {
  initialize::init().await;
}
