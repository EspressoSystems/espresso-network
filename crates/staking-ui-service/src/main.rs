use std::{path::PathBuf, process::exit, sync::Arc};

use async_lock::RwLock;
use clap::{Parser, ValueEnum};
use futures::{FutureExt, future::try_join_all};
use log_panics::BacktraceMode;
use staking_ui_service::{
    Result, app,
    input::l1::{self, RpcStream, Snapshot, options::L1ClientOptions},
    persistence::sql,
};
use tracing_subscriber::{EnvFilter, fmt::format::FmtSpan};

/// Controls how logs are displayed and how backtraces are logged on panic.
///
/// The values here match the possible values of `RUST_LOG_FORMAT`, and their corresponding behavior
/// on backtrace logging is:
/// * `full`: print a prettified dump of the stack trace and span trace to stdout, optimized for
///   human readability rather than machine parsing
/// * `compact`: output the default panic message, with backtraces controlled by `RUST_BACKTRACE`
/// * `json`: output the panic message and stack trace as a tracing event. This in turn works with
///   the behavior of the tracing subscriber with `RUST_LOG_FORMAT=json` to output the event in a
///   machine-parseable, JSON format.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
enum LogFormat {
    #[default]
    Full,
    Compact,
    Json,
}

/// The backend service for the Espresso Network Staking UI.
#[derive(Debug, Parser)]
struct Options {
    /// L1 client options.
    #[clap(flatten)]
    l1_options: L1ClientOptions,

    /// Location for persistent storage.
    #[clap(long, env = "ESPRESSO_STAKING_SERVICE_STORAGE")]
    storage: PathBuf,

    /// Port for the HTTP server.
    #[clap(
        short,
        long,
        env = "ESPRESSO_SATKING_SERVICE_PORT",
        default_value = "8080"
    )]
    port: u16,

    /// Formatting options for tracing.
    #[clap(long, env = "RUST_LOG_FORMAT")]
    log_format: Option<LogFormat>,
}

impl Options {
    async fn run(self) -> Result<()> {
        self.init_logging();

        // Get genesis state.
        let l1_provider = self.l1_options.provider()?.0;
        let genesis_block =
            l1::provider::load_genesis(&l1_provider, self.l1_options.stake_table_address).await?;
        let genesis = Snapshot::empty(genesis_block);

        let l1_input = RpcStream::new(self.l1_options).await?;
        let storage = sql::Persistence::new(&self.storage).await?;

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

    fn init_logging(&self) {
        // Parse the `RUST_LOG_SPAN_EVENTS` environment variable
        let span_event_filter = match std::env::var("RUST_LOG_SPAN_EVENTS") {
            Ok(val) => val
                .split(',')
                .map(|s| match s.trim() {
                    "new" => FmtSpan::NEW,
                    "enter" => FmtSpan::ENTER,
                    "exit" => FmtSpan::EXIT,
                    "close" => FmtSpan::CLOSE,
                    "active" => FmtSpan::ACTIVE,
                    "full" => FmtSpan::FULL,
                    _ => FmtSpan::NONE,
                })
                .fold(FmtSpan::NONE, |acc, x| acc | x),
            Err(_) => FmtSpan::NONE,
        };

        let subscriber = tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .with_span_events(span_event_filter);

        // Conditionally initialize in `json` mode
        if let LogFormat::Json = self.log_format.unwrap_or_default() {
            let _ = subscriber.json().try_init();
            log_panics::Config::new()
                .backtrace_mode(BacktraceMode::Resolved)
                .install_panic_hook();
        } else {
            let _ = subscriber.try_init();
        }
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

#[cfg(test)]
mod test {
    use std::time::Duration;

    use alloy::node_bindings::Anvil;
    use portpicker::pick_unused_port;
    use staking_ui_service::types::common::L1BlockId;
    use staking_ui_service::{Error, input::l1::testing::ContractDeployment};
    use surf_disco::Client;
    use tempfile::tempdir;
    use tokio::{task::spawn, time::sleep};
    use vbs::version::StaticVersion;

    use super::*;

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn e2e_smoke_test() {
        let port = pick_unused_port().unwrap();
        let tmp = tempdir().unwrap();
        let anvil = Anvil::new()
            .block_time(1)
            .args(["--slots-in-an-epoch", "0"])
            .spawn();
        let deployment = ContractDeployment::deploy(anvil.endpoint_url())
            .await
            .unwrap();

        let opt = Options {
            l1_options: L1ClientOptions {
                http_providers: vec![anvil.endpoint_url()],
                l1_ws_provider: Some(vec![anvil.ws_endpoint_url()]),
                stake_table_address: deployment.stake_table_addr,
                reward_contract_address: deployment.reward_claim_addr,
                ..Default::default()
            },
            port,
            storage: tmp.path().join("staking-ui-storage"),
            log_format: Some(LogFormat::Json),
        };

        let task = spawn(opt.run());

        let client: Client<Error, StaticVersion<0, 1>> =
            Client::new(format!("http://localhost:{port}").parse().unwrap());
        sleep(Duration::from_secs(1)).await;
        client.connect(None).await;

        // Check that L1 blocks are increasing.
        let initial_l1_block: L1BlockId = client.get("l1/block/latest").send().await.unwrap();
        tracing::info!(?initial_l1_block, "client connected");
        loop {
            sleep(Duration::from_secs(1)).await;
            let l1_block: L1BlockId = client.get("l1/block/latest").send().await.unwrap();
            if l1_block.number > initial_l1_block.number {
                tracing::info!(?initial_l1_block, ?l1_block, "L1 block increased");
                break;
            }
            tracing::info!("waiting for L1 block to increase");
        }

        task.abort();
        let _ = task.await;
    }
}
