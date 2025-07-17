//! Main entry for libp2p_test

mod api;
mod config;
mod types;

use std::time::Duration;

use anyhow::Result;
use hotshot_example_types::node_types::TestTypes;
use tokio::time::timeout;

use crate::config::AppConfig;

const REPLY_TIMEOUT: Duration = Duration::from_secs(10);

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let config = AppConfig::from_file()?;
    if config.send_mode {
        api::run_sender::<TestTypes>(config).await
    } else {
        timeout(REPLY_TIMEOUT, api::run_receiver::<TestTypes>(config)).await?
    }
}
