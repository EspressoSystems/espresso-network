use std::{process::exit, sync::Arc};

use async_lock::RwLock;
use clap::{Parser, ValueEnum};
use futures::{FutureExt, future::try_join_all};
use log_panics::BacktraceMode;
use staking_ui_service::{
    Result, app,
    input::{
        espresso::{
            self,
            client::{QueryServiceClient, QueryServiceOptions},
        },
        l1::{
            self, RpcCatchup, RpcStream, Snapshot, metadata::HttpMetadataFetcher,
            options::L1ClientOptions,
        },
    },
    persistence::sql,
};
use tokio::time::sleep;
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

    /// Espresso client options.
    #[clap(flatten)]
    espresso_options: QueryServiceOptions,

    /// Persistence options.
    #[clap(flatten)]
    persistence: sql::PersistenceOptions,

    /// Port for the HTTP server.
    #[clap(
        short,
        long,
        env = "ESPRESSO_STAKING_SERVICE_PORT",
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

        let storage = sql::Persistence::new(&self.persistence)
            .await
            .map_err(|err| err.context("opening storage"))?;

        // Get genesis state.
        let l1_provider = self
            .l1_options
            .provider()
            .map_err(|err| err.context("creating L1 provider"))?
            .0;
        let genesis_block = loop {
            // We can fail to load the genesis block for various reasons, e.g. the stake table
            // contract is not deployed yet, or the initialization block has not finalized. These
            // are usually encountered in tests where we are deploying stuff and starting up the
            // service around the same time, but it is also possible to fail here in production, if,
            // say, the L1 provider has a failure. In any case our best chance of avoiding manual
            // intervention is to retry until we succeed, or until the container orchtestrator gives
            // up and restarts the service. (Since we haven't started the HTTP server yet, the
            // service will not be "healthy" at this stage).
            match l1::provider::load_genesis(&l1_provider, self.l1_options.stake_table_address)
                .await
            {
                Ok(genesis) => break genesis,
                Err(err) => {
                    tracing::warn!("error loading L1 genesis, will retry: {err:#}");
                    sleep(self.l1_options.l1_retry_delay).await;
                }
            }
        };
        tracing::info!(?genesis_block, "loaded L1 genesis");
        let genesis = Snapshot::empty(genesis_block);

        // Connect to L1.
        let l1_catchup = RpcCatchup::new(&self.l1_options)?;
        let l1_input = RpcStream::new(self.l1_options)
            .await
            .map_err(|err| err.context("opening L1 RPC stream"))?;

        // Connect to Espresso.
        let espresso_input = QueryServiceClient::new(self.espresso_options)
            .await
            .map_err(|err| err.context("connecting to Espresso query service"))?;

        // Create server state.
        let l1 = Arc::new(RwLock::new(
            l1::State::new(
                storage.clone(),
                HttpMetadataFetcher::default(),
                genesis,
                &l1_catchup,
            )
            .await
            .map_err(|err| err.context("initializing L1 state"))?,
        ));
        let espresso = Arc::new(RwLock::new(
            espresso::State::new(storage, espresso_input)
                .await
                .map_err(|err| err.context("initializing Espresso state"))?,
        ));
        let app = app::State::new(l1.clone(), espresso.clone());

        // Create tasks that will run in parallel.
        let l1_task = l1::State::subscribe(l1, l1_input);
        let espresso_task = espresso::State::update_task(espresso);
        let http_task = app.serve(self.port);

        // Run all tasks. Terminate if any background task fails (they should all run forever, but
        // if one does fail it is better to loudly crash than to continue running in some weird
        // state).
        try_join_all([l1_task.boxed(), espresso_task.boxed(), http_task.boxed()]).await?;
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
    use espresso_types::{FeeVersion, SequencerVersions};
    use hotshot_query_service::data_source::SqlDataSource;
    use portpicker::pick_unused_port;
    use sequencer::api;
    use sequencer::api::data_source::testing::TestableSequencerDataSource;
    use sequencer::api::sql::DataSource;
    use sequencer::api::test_helpers::{TestNetwork, TestNetworkConfigBuilder};
    use sequencer::testing::TestConfigBuilder;
    use staking_ui_service::input::espresso::testing::start_pos_network;
    use staking_ui_service::types::common::L1BlockId;
    use staking_ui_service::types::global::ActiveNodeSetSnapshot;
    use staking_ui_service::{Error, input::l1::testing::ContractDeployment};
    use surf_disco::Client;
    use tempfile::tempdir;
    use tide_disco::{Error as _, StatusCode};
    use tokio::{task::spawn, time::sleep};
    use vbs::version::StaticVersion;

    use super::*;

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn e2e_smoke_test() {
        let port = pick_unused_port().unwrap();
        let tmp = tempdir().unwrap();
        let espresso_port = pick_unused_port().unwrap();
        let (mut network, deployment, _storage) = start_pos_network(espresso_port).await;
        let espresso_url = format!("http://localhost:{espresso_port}").parse().unwrap();

        let opt = Options {
            l1_options: L1ClientOptions {
                http_providers: vec![deployment.rpc_url],
                l1_ws_provider: None,
                stake_table_address: deployment.stake_table_addr,
                reward_contract_address: deployment.reward_claim_addr,
                ..Default::default()
            },
            espresso_options: QueryServiceOptions::new(espresso_url),
            port,
            persistence: sql::PersistenceOptions {
                path: tmp.path().join("temp.db"),
                max_connections: 5,
            },
            log_format: Some(LogFormat::Json),
        };

        let task = spawn(async move {
            opt.run().await.unwrap();
        });

        let client: Client<Error, StaticVersion<0, 1>> = Client::new(
            format!("http://localhost:{port}/v0/staking/")
                .parse()
                .unwrap(),
        );
        sleep(Duration::from_secs(1)).await;
        client.connect(None).await;

        // Check that L1 blocks and Espresso blocks are increasing.
        let initial_l1_block: L1BlockId = client.get("l1/block/latest").send().await.unwrap();
        // Espresso API may take a moment to become available.
        let initial_espresso_block = loop {
            let active_node_set: ActiveNodeSetSnapshot =
                match client.get("nodes/active").send().await {
                    Ok(set) => set,
                    Err(err) => {
                        assert_eq!(
                            err.status(),
                            StatusCode::SERVICE_UNAVAILABLE,
                            "unexpected error from Espresso API: {err:#}"
                        );
                        tracing::info!("waiting for Espresso API to become available");
                        sleep(Duration::from_secs(1)).await;
                        continue;
                    }
                };
            assert_eq!(active_node_set.nodes.len(), network.peers.len() + 1);
            break active_node_set.espresso_block;
        };
        tracing::info!(
            ?initial_l1_block,
            ?initial_espresso_block,
            "client connected"
        );
        // Wait for L1 block to increase.
        loop {
            sleep(Duration::from_secs(1)).await;
            let l1_block: L1BlockId = client.get("l1/block/latest").send().await.unwrap();
            if l1_block.number > initial_l1_block.number {
                tracing::info!(?initial_l1_block, ?l1_block, "L1 block increased");
                break;
            }
            tracing::info!("waiting for L1 block to increase");
        }
        // Wait for Espresso block to increase.
        loop {
            sleep(Duration::from_secs(1)).await;
            let espresso_block = client
                .get::<ActiveNodeSetSnapshot>("nodes/active")
                .send()
                .await
                .unwrap()
                .espresso_block;
            if espresso_block.block > initial_espresso_block.block {
                tracing::info!(
                    ?initial_espresso_block,
                    ?espresso_block,
                    "Espresso block increased"
                );
                break;
            }
            tracing::info!("waiting for Espresso block to increase");
        }

        task.abort();
        let _ = task.await;

        network.stop_consensus().await;
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_startup_before_pos() {
        // Start L1 and deploy contracts.
        let anvil = Anvil::new()
            .block_time(1)
            .args(["--slots-in-an-epoch", "0"])
            .spawn();
        let rpc_url = anvil.endpoint_url();
        let deployment = ContractDeployment::deploy(rpc_url.clone())
            .await
            .expect("Failed to deploy contracts");

        // Start Espresso on pre-POS version.
        let espresso_port = pick_unused_port().unwrap();
        let test_config = TestConfigBuilder::<1>::default()
            .anvil_provider(anvil)
            .build();
        let storage = DataSource::create_storage().await;
        let persistence =
            <DataSource as TestableSequencerDataSource>::persistence_options(&storage);
        let options = SqlDataSource::options(&storage, api::Options::with_port(espresso_port))
            .config(Default::default());
        let config = TestNetworkConfigBuilder::with_num_nodes()
            .api_config(options.clone())
            .persistences([persistence.clone()])
            .network_config(test_config)
            .build();
        let mut network =
            TestNetwork::new(config, SequencerVersions::<FeeVersion, FeeVersion>::new()).await;

        // Start staking service.
        let port = pick_unused_port().unwrap();
        let tmp = tempdir().unwrap();
        let opt = Options {
            l1_options: L1ClientOptions {
                http_providers: vec![deployment.rpc_url],
                l1_ws_provider: None,
                stake_table_address: deployment.stake_table_addr,
                reward_contract_address: deployment.reward_claim_addr,
                ..Default::default()
            },
            espresso_options: QueryServiceOptions::new(
                format!("http://localhost:{espresso_port}").parse().unwrap(),
            ),
            port,
            persistence: sql::PersistenceOptions {
                path: tmp.path().join("temp.db"),
                max_connections: 5,
            },
            log_format: Some(LogFormat::Json),
        };

        let task = spawn(async move {
            opt.run().await.unwrap();
        });

        let client: Client<Error, StaticVersion<0, 1>> = Client::new(
            format!("http://localhost:{port}/v0/staking/")
                .parse()
                .unwrap(),
        );
        sleep(Duration::from_secs(1)).await;
        client.connect(None).await;

        // We should be able to use the L1-based part of the API even though the Espresso-based part
        // isn't ready yet.
        let initial_l1_block: L1BlockId = client.get("l1/block/latest").send().await.unwrap();
        tracing::info!(?initial_l1_block, "client connected");
        // Wait for L1 block to increase.
        loop {
            sleep(Duration::from_secs(1)).await;
            let l1_block: L1BlockId = client.get("l1/block/latest").send().await.unwrap();
            if l1_block.number > initial_l1_block.number {
                tracing::info!(?initial_l1_block, ?l1_block, "L1 block increased");
                break;
            }
            tracing::info!("waiting for L1 block to increase");
        }

        // The Espresso API should not be ready yet.
        let err = client
            .get::<ActiveNodeSetSnapshot>("nodes/active")
            .send()
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::SERVICE_UNAVAILABLE);

        task.abort();
        let _ = task.await;

        network.stop_consensus().await;
    }
}
