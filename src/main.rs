#[tokio::main(worker_threads = 2)]
async fn main() {
    if let Err(error) = tezbar_extension_sdk::serve(tezbar_perplexity::Perplexity).await {
        eprintln!("Perplexity protocol error: {error}");
        std::process::exit(1);
    }
}
