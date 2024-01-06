use crate::file_server::create_static_file_server;
use crate::socket_init::create_socket_endpoint;

pub async fn init() {
  match tokio::join!(create_static_file_server(), create_socket_endpoint()) {
    (Err(err), _) => {
      println!("Error joining static file server: {:?}", err);
    }
    (_, Err(err)) => {
      println!("Error joining socket server: {:?}", err);
    }
    (Ok(()), Ok(())) => {}
  }
}
