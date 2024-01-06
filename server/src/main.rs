use std::env;

use warp::Filter;

#[tokio::main]
async fn main() {
  tracing_subscriber::fmt()
    .with_max_level(tracing::Level::INFO)
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

  warp::serve(route).run(([127, 0, 0, 1], 2001)).await
}
