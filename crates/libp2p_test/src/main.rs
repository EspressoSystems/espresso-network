//! Main entry for libp2p_test

mod api;
mod config;
mod types;

use anyhow::Result;
use hotshot_example_types::node_types::TestTypes;

use crate::config::AppConfig;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let config = AppConfig::from_file()?;
    if config.send_mode {
        api::run_sender::<TestTypes>(config).await
    } else {
        api::run_receiver::<TestTypes>(config).await
    }
}
