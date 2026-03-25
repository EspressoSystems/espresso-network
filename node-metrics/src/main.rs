use clap::Parser;
use hotshot::helpers::initialize_logging;
use node_metrics::{Options, run_standalone_service};

#[tokio::main]
async fn main() {
    initialize_logging();

    run_standalone_service(Options::parse()).await;
}
