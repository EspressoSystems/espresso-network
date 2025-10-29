use std::{path::PathBuf, process::exit, sync::Arc};

use async_lock::RwLock;
use clap::Parser;
use staking_ui_service::{
    Result,
    input::l1::{self, PersistentSnapshot, RpcStream, options::L1ClientOptions},
    persistence::sql,
    types::common::Address,
};

/// The backend service for the Espresso Network Staking UI.
#[derive(Debug, Parser)]
struct Options {
    /// L1 client options.
    #[clap(flatten)]
    l1_options: L1ClientOptions,

    /// Address of deployed stake table contract.
    #[clap(long, env = "ESPRESSO_STAKING_SERVICE_STAKE_TABLE")]
    stake_table: Address,

    /// Location for persistent storage.
    #[clap(long, env = "ESPRESSO_STAKING_SERVICE_STORAGE")]
    storage: PathBuf,
}

impl Options {
    async fn run(self) -> Result<()> {
        let l1_input = RpcStream::new(self.l1_options).await?;
        let storage = sql::Persistence::new(&self.storage).await?;

        // Get genesis state.
        let (id, timestamp) = l1_input.genesis(self.stake_table).await?;
        let genesis = PersistentSnapshot::genesis(id, timestamp);

        let l1_state = Arc::new(RwLock::new(l1::State::new(storage, genesis).await?));
        l1::State::subscribe(l1_state, l1_input).await;
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let opt = Options::parse();
    if let Err(err) = opt.run().await {
        eprintln!("service failed: {err:#}");
        exit(1);
    }
}
