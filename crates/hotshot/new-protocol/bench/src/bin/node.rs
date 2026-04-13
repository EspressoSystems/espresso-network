use anyhow::Result;
use clap::Parser;
use hotshot_new_protocol_bench::config::NodeConfig;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    let cfg = NodeConfig::parse();
    hotshot_new_protocol_bench::node::run(cfg).await
}
