use protocol_mcp_gateway::answer_key::{load, start_line, take_key_args};
use protocol_mcp_gateway::{app_with_dataset_dir_and_key, dataset_dir_from_args};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    let (arguments, key_args) = match take_key_args(std::env::args().skip(1)) {
        Ok(taken) => taken,
        Err(sentence) => {
            eprintln!("The gateway did not start: {sentence}");
            std::process::exit(2);
        }
    };
    let answer_key = match load(&key_args) {
        Ok(key) => key,
        Err(sentence) => {
            eprintln!("The gateway did not start: {sentence}");
            std::process::exit(2);
        }
    };
    let dataset_dir = match dataset_dir_from_args(arguments) {
        Ok(dir) => dir,
        Err(sentence) => {
            eprintln!("The gateway did not start: {sentence}");
            std::process::exit(2);
        }
    };
    eprintln!("{}", start_line("protocol_mcp_gateway", answer_key.as_deref()));
    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("Gateway running on {}", listener.local_addr().unwrap());
    axum::serve(listener, app_with_dataset_dir_and_key(dataset_dir, answer_key)).await.unwrap();
}
