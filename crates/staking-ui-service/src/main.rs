use std::{path::PathBuf, process::exit, sync::Arc};

use async_lock::RwLock;
use clap::Parser;
use futures::{FutureExt, future::try_join_all};
use staking_ui_service::{
    Result, app,
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

    /// Port for the HTTP server.
    #[clap(short, long, env = "ESPRESSO_SATKING_SERVICE_PORT")]
    port: u16,
}

impl Options {
    async fn run(self) -> Result<()> {
        let l1_input = RpcStream::new(self.l1_options).await?;
        let storage = sql::Persistence::new(&self.storage).await?;

        // Get genesis state.
        let (id, timestamp) = l1_input.genesis(self.stake_table).await?;
        let genesis = PersistentSnapshot::genesis(id, timestamp);

        // Create server state.
        let l1 = Arc::new(RwLock::new(l1::State::new(storage, genesis).await?));
        let app = app::State::new(l1.clone());

        // Create tasks that will run in parallel.
        let l1_task = l1::State::subscribe(l1, l1_input);
        let http_task = app.serve(self.port);

        // Run all tasks. Terminate if any background task fails (they should all run forever, but
        // if one does fail it is better to loudly crash than to continue running in some weird
        // state).
        try_join_all([l1_task.boxed(), http_task.boxed()]).await?;
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
