// Serve the gateway on port 3000 with a chosen physics pool, for measuring.
// Usage: serve_with_pool <permits> <dataset directory>
use protocol_mcp_gateway::{app_with_permits, prepare_dataset_dir};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let mut arguments = std::env::args().skip(1);
    let permits: usize = arguments.next().unwrap().parse().unwrap();
    let dir = prepare_dataset_dir(std::path::Path::new(&arguments.next().unwrap())).unwrap();
    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app_with_permits(permits, Some(dir))).await.unwrap();
}
