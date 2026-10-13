use protocol_mcp_gateway::{app_with_dataset_dir, dataset_dir_from_args};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let dataset_dir = match dataset_dir_from_args(std::env::args().skip(1)) {
        Ok(dir) => dir,
        Err(sentence) => {
            eprintln!("The gateway did not start: {sentence}");
            std::process::exit(2);
        }
    };
    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("Gateway running on {}", listener.local_addr().unwrap());
    axum::serve(listener, app_with_dataset_dir(dataset_dir)).await.unwrap();
}
