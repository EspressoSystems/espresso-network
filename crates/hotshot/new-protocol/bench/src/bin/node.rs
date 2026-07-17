use anyhow::Result;
use clap::Parser;
use hotshot_new_protocol::logging::init_logging;
use hotshot_new_protocol_bench::config::NodeConfig;

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();

    let cfg = NodeConfig::parse();
    hotshot_new_protocol_bench::node::run(cfg).await
}
