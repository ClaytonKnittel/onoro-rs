use std::{
  env,
  net::{Ipv6Addr, SocketAddrV6},
};
use tokio::task::JoinHandle;
use warp::Filter;

pub fn create_static_file_server() -> JoinHandle<()> {
  tokio::spawn(async {
    tracing_subscriber::fmt()
      .with_max_level(tracing::Level::WARN)
      .init();

    let log_filter = warp::trace::request();

    let route = warp::fs::dir(
      env::current_dir()
        .unwrap()
        .parent()
        .unwrap()
        .join("web/dist/dev/static"),
    )
    .with(log_filter);

    warp::serve(route)
      .run(SocketAddrV6::new(
        Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0),
        2001,
        0,
        0,
      ))
      .await;
  })
}
