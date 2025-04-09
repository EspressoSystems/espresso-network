use std::sync::Arc;

use clap::Parser;
#[allow(unused_imports)]
use espresso_types::{
    traits::NullEventConsumer, FeeVersion, MarketplaceVersion, SequencerVersions,
    SolverAuctionResultsProvider, V0_0,
};
use futures::future::FutureExt;
use hotshot::MarketplaceConfig;
use hotshot_types::traits::{metrics::NoMetrics, node_implementation::Versions};
use vbs::version::StaticVersionType;

use super::{
    api::{self, data_source::DataSourceOptions},
    context::SequencerContext,
    init_node, network,
    options::{Modules, Options},
    persistence, Genesis, L1Params, NetworkParams,
};

pub async fn main() -> anyhow::Result<()> {
    let opt = Options::parse();
    opt.logging.init();

    let modules = opt.modules();
    tracing::warn!(?modules, "sequencer starting up");

    let genesis = Genesis::from_file(&opt.genesis_file)?;
    tracing::info!(?genesis, "genesis");

    let base = genesis.base_version;
    let upgrade = genesis.upgrade_version;

    match (base, upgrade) {
        #[cfg(all(feature = "fee", feature = "pos"))]
        (FeeVersion::VERSION, espresso_types::EpochVersion::VERSION) => {
            run(
                genesis,
                modules,
                opt,
                SequencerVersions::<espresso_types::FeeVersion, espresso_types::EpochVersion>::new(
                ),
            )
            .await
        },
        #[cfg(feature = "pos")]
        (espresso_types::EpochVersion::VERSION, _) => {
            run(
                genesis,
                modules,
                opt,
                // Specifying V0_0 disables upgrades
                SequencerVersions::<espresso_types::EpochVersion, espresso_types::V0_0>::new(),
            )
            .await
        },
        #[cfg(all(feature = "fee", feature = "marketplace"))]
        (FeeVersion::VERSION, espresso_types::MarketplaceVersion::VERSION) => {
            run(
                genesis,
                modules,
                opt,
                SequencerVersions::<FeeVersion, MarketplaceVersion>::new(),
            )
            .await
        },
        #[cfg(feature = "fee")]
        (FeeVersion::VERSION, _) => {
            run(
                genesis,
                modules,
                opt,
                SequencerVersions::<FeeVersion, espresso_types::V0_0>::new(),
            )
            .await
        },
        #[cfg(feature = "marketplace")]
        (espresso_types::MarketplaceVersion::VERSION, _) => {
            run(
                genesis,
                modules,
                opt,
                SequencerVersions::<espresso_types::MarketplaceVersion, espresso_types::V0_0>::new(
                ),
            )
            .await
        },
        _ => panic!(
            "Invalid base ({base}) and upgrade ({upgrade}) versions specified in the toml file."
        ),
    }
}

async fn run<V>(
    genesis: Genesis,
    mut modules: Modules,
    opt: Options,
    versions: V,
) -> anyhow::Result<()>
where
    V: Versions,
{
    if let Some(storage) = modules.storage_fs.take() {
        run_with_storage(genesis, modules, opt, storage, versions).await
    } else if let Some(storage) = modules.storage_sql.take() {
        run_with_storage(genesis, modules, opt, storage, versions).await
    } else {
        // Persistence is required. If none is provided, just use the local file system.
        run_with_storage(
            genesis,
            modules,
            opt,
            persistence::fs::Options::default(),
            versions,
        )
        .await
    }
}

async fn run_with_storage<S, V>(
    genesis: Genesis,
    modules: Modules,
    opt: Options,
    storage_opt: S,
    versions: V,
) -> anyhow::Result<()>
where
    S: DataSourceOptions,
    V: Versions,
{
    let ctx = init_with_storage(genesis, modules, opt, storage_opt, versions).await?;

    // Start doing consensus.
    ctx.start_consensus().await;
    ctx.join().await;

    Ok(())
}

pub(crate) async fn init_with_storage<S, V>(
    genesis: Genesis,
    modules: Modules,
    opt: Options,
    mut storage_opt: S,
    versions: V,
) -> anyhow::Result<SequencerContext<network::Production, S::Persistence, V>>
where
    S: DataSourceOptions,
    V: Versions,
{
    let (private_staking_key, private_state_key) = opt.private_keys()?;
    let l1_params = L1Params {
        urls: opt.l1_provider_url,
        options: opt.l1_options,
    };

    let network_params = NetworkParams {
        cdn_endpoint: opt.cdn_endpoint,
        libp2p_advertise_address: opt.libp2p_advertise_address,
        libp2p_bind_address: opt.libp2p_bind_address,
        libp2p_bootstrap_nodes: opt.libp2p_bootstrap_nodes,
        orchestrator_url: opt.orchestrator_url,
        state_relay_server_url: opt.state_relay_server_url,
        public_api_url: opt.public_api_url,
        private_staking_key,
        private_state_key,
        state_peers: opt.state_peers,
        config_peers: opt.config_peers,
        catchup_backoff: opt.catchup_backoff,
        libp2p_history_gossip: opt.libp2p_history_gossip,
        libp2p_history_length: opt.libp2p_history_length,
        libp2p_max_ihave_length: opt.libp2p_max_ihave_length,
        libp2p_max_ihave_messages: opt.libp2p_max_ihave_messages,
        libp2p_max_gossip_transmit_size: opt.libp2p_max_gossip_transmit_size,
        libp2p_max_direct_transmit_size: opt.libp2p_max_direct_transmit_size,
        libp2p_mesh_outbound_min: opt.libp2p_mesh_outbound_min,
        libp2p_mesh_n: opt.libp2p_mesh_n,
        libp2p_mesh_n_high: opt.libp2p_mesh_n_high,
        libp2p_heartbeat_interval: opt.libp2p_heartbeat_interval,
        libp2p_mesh_n_low: opt.libp2p_mesh_n_low,
        libp2p_published_message_ids_cache_time: opt.libp2p_published_message_ids_cache_time,
        libp2p_iwant_followup_time: opt.libp2p_iwant_followup_time,
        libp2p_max_messages_per_rpc: opt.libp2p_max_messages_per_rpc,
        libp2p_gossip_retransmission: opt.libp2p_gossip_retransmission,
        libp2p_flood_publish: opt.libp2p_flood_publish,
        libp2p_duplicate_cache_time: opt.libp2p_duplicate_cache_time,
        libp2p_fanout_ttl: opt.libp2p_fanout_ttl,
        libp2p_heartbeat_initial_delay: opt.libp2p_heartbeat_initial_delay,
        libp2p_gossip_factor: opt.libp2p_gossip_factor,
        libp2p_gossip_lazy: opt.libp2p_gossip_lazy,
    };

    let marketplace_config = MarketplaceConfig {
        auction_results_provider: Arc::new(SolverAuctionResultsProvider {
            url: opt.auction_results_solver_url,
            marketplace_path: opt.marketplace_solver_path,
            results_path: opt.auction_results_path,
        }),
        fallback_builder_url: opt.fallback_builder_url,
    };
    let proposal_fetcher_config = opt.proposal_fetcher_config;

    let persistence = storage_opt.create().await?;

    // Initialize HotShot. If the user requested the HTTP module, we must initialize the handle in
    // a special way, in order to populate the API with consensus metrics. Otherwise, we initialize
    // the handle directly, with no metrics.
    let ctx = match modules.http {
        Some(http_opt) => {
            // Add optional API modules as requested.
            let mut http_opt = api::Options::from(http_opt);
            if let Some(query) = modules.query {
                http_opt = storage_opt.enable_query_module(http_opt, query);
            }
            if let Some(submit) = modules.submit {
                http_opt = http_opt.submit(submit);
            }
            if let Some(status) = modules.status {
                http_opt = http_opt.status(status);
            }

            if let Some(catchup) = modules.catchup {
                http_opt = http_opt.catchup(catchup);
            }
            if let Some(hotshot_events) = modules.hotshot_events {
                http_opt = http_opt.hotshot_events(hotshot_events);
            }
            if let Some(explorer) = modules.explorer {
                http_opt = http_opt.explorer(explorer);
            }
            if let Some(config) = modules.config {
                http_opt = http_opt.config(config);
            }

            http_opt
                .serve(move |metrics, consumer| {
                    async move {
                        init_node(
                            genesis,
                            network_params,
                            &*metrics,
                            persistence,
                            l1_params,
                            versions,
                            consumer,
                            opt.is_da,
                            opt.identity,
                            marketplace_config,
                            proposal_fetcher_config,
                        )
                        .await
                    }
                    .boxed()
                })
                .await?
        },
        None => {
            init_node(
                genesis,
                network_params,
                &NoMetrics,
                persistence,
                l1_params,
                versions,
                NullEventConsumer,
                opt.is_da,
                opt.identity,
                marketplace_config,
                proposal_fetcher_config,
            )
            .await?
        },
    };

    Ok(ctx)
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use espresso_types::{MockSequencerVersions, PubKey};
    use hotshot_types::{light_client::StateKeyPair, traits::signature_key::SignatureKey};
    use portpicker::pick_unused_port;
    use sequencer_utils::test_utils::setup_test;
    use surf_disco::{error::ClientError, Client, Url};
    use tempfile::TempDir;
    use tokio::spawn;
    use vbs::version::Version;

    use super::*;
    use crate::{
        api::options::Http,
        genesis::{L1Finalized, StakeTableConfig},
        persistence::fs,
        SequencerApiVersion,
    };

    #[tokio::test(flavor = "multi_thread")]
    async fn test_startup_before_orchestrator() {
        setup_test();

        let (pub_key, priv_key) = PubKey::generated_from_seed_indexed([0; 32], 0);
        let state_key = StateKeyPair::generate_from_seed_indexed([0; 32], 0);

        let port = pick_unused_port().unwrap();
        let tmp = TempDir::new().unwrap();

        let genesis_file = tmp.path().join("genesis.toml");
        let genesis = Genesis {
            chain_config: Default::default(),
            stake_table: StakeTableConfig { capacity: 10 },
            accounts: Default::default(),
            l1_finalized: L1Finalized::Number { number: 0 },
            header: Default::default(),
            upgrades: Default::default(),
            base_version: Version { major: 0, minor: 1 },
            upgrade_version: Version { major: 0, minor: 2 },
            epoch_height: None,
            epoch_start_block: None,
        };
        genesis.to_file(&genesis_file).unwrap();

        let modules = Modules {
            http: Some(Http::with_port(port)),
            query: Some(Default::default()),
            storage_fs: Some(fs::Options::new(tmp.path().into())),
            ..Default::default()
        };
        let opt = Options::parse_from([
            "sequencer",
            "--private-staking-key",
            &priv_key.to_tagged_base64().expect("valid key").to_string(),
            "--private-state-key",
            &state_key
                .sign_key_ref()
                .to_tagged_base64()
                .expect("valid key")
                .to_string(),
            "--genesis-file",
            &genesis_file.display().to_string(),
        ]);

        // Start the sequencer in a background task. This process will not complete, because it will
        // be waiting for the orchestrator, but it should at least start up the API server and
        // populate some metrics.
        tracing::info!(port, "starting sequencer");
        let task = spawn(async move {
            if let Err(err) = init_with_storage(
                genesis,
                modules,
                opt,
                fs::Options::new(tmp.path().into()),
                MockSequencerVersions::new(),
            )
            .await
            {
                tracing::error!("failed to start sequencer: {err:#}");
            }
        });

        // The healthcheck should eventually come up even though the node is waiting for the
        // orchestrator.
        tracing::info!("waiting for API to start");
        let url: Url = format!("http://localhost:{port}").parse().unwrap();
        let client = Client::<ClientError, SequencerApiVersion>::new(url.clone());
        assert!(client.connect(Some(Duration::from_secs(60))).await);
        client.get::<()>("healthcheck").send().await.unwrap();

        // The metrics should include information about the node and software version. surf-disco
        // doesn't currently support fetching a plaintext file, so we use a raw reqwest client.
        let res = reqwest::get(url.join("/status/metrics").unwrap())
            .await
            .unwrap();
        assert!(res.status().is_success(), "{}", res.status());
        let metrics = res.text().await.unwrap();
        let lines = metrics.lines().collect::<Vec<_>>();
        assert!(
            lines.contains(&format!("consensus_node{{key=\"{pub_key}\"}} 1").as_str()),
            "{lines:#?}"
        );
        assert!(
            lines.contains(
                &format!(
                    "consensus_version{{desc=\"{}\",rev=\"{}\",timestamp=\"{}\"}} 1",
                    env!("VERGEN_GIT_DESCRIBE"),
                    env!("VERGEN_GIT_SHA"),
                    env!("VERGEN_GIT_COMMIT_TIMESTAMP"),
                )
                .as_str()
            ),
            "{lines:#?}"
        );

        task.abort();
    }
}
