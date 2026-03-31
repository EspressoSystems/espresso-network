use clap::Parser;
use espresso_types::traits::NullEventConsumer;
use futures::future::FutureExt;
use hotshot_types::traits::metrics::NoMetrics;

use super::{
    Genesis, L1Params, NetworkParams,
    api::{self, data_source::DataSourceOptions},
    context::SequencerContext,
    init_node, network,
    options::{Modules, Options},
    persistence,
};
use crate::keyset::KeySet;

pub async fn main() -> anyhow::Result<()> {
    let opt = Options::parse();
    opt.logging.init();

    let mut modules = opt.modules();
    tracing::warn!(?modules, "sequencer starting up");

    let genesis = Genesis::from_file(&opt.genesis_file)?;
    tracing::warn!(?genesis, "genesis");

    if let Some(storage) = modules.storage_fs.take() {
        run_with_storage(genesis, modules, opt, storage).await
    } else if let Some(storage) = modules.storage_sql.take() {
        run_with_storage(genesis, modules, opt, storage).await
    } else {
        // Persistence is required. If none is provided, just use the local file system.
        run_with_storage(genesis, modules, opt, persistence::fs::Options::default()).await
    }
}

async fn run_with_storage<S>(
    genesis: Genesis,
    modules: Modules,
    opt: Options,
    storage_opt: S,
) -> anyhow::Result<()>
where
    S: DataSourceOptions,
{
    let ctx = init_with_storage(genesis, modules, opt, storage_opt).await?;

    // Start doing consensus.
    ctx.start_consensus().await;
    ctx.join().await;

    Ok(())
}

pub async fn init_with_storage<S>(
    genesis: Genesis,
    modules: Modules,
    opt: Options,
    mut storage_opt: S,
) -> anyhow::Result<SequencerContext<network::Production, S::Persistence>>
where
    S: DataSourceOptions,
{
    let KeySet {
        staking,
        state,
        x25519,
    } = opt.key_set.try_into()?;
    let l1_params = L1Params {
        urls: opt.l1_provider_url,
        options: opt.l1_options,
    };

    let network_params = NetworkParams {
        cdn_endpoint: opt.cdn_endpoint,
        cliquenet_bind_addr: opt.cliquenet_bind_address,
        x25519_secret_key: x25519,
        libp2p_advertise_address: opt.libp2p_advertise_address,
        libp2p_bind_address: opt.libp2p_bind_address,
        libp2p_bootstrap_nodes: opt.libp2p_bootstrap_nodes,
        orchestrator_url: opt.orchestrator_url,
        builder_urls: opt.builder_urls,
        state_relay_server_url: opt.state_relay_server_url,
        public_api_url: opt.public_api_url,
        private_staking_key: staking,
        private_state_key: state,
        state_peers: opt.state_peers,
        config_peers: opt.config_peers,
        catchup_backoff: opt.catchup_backoff,
        catchup_base_timeout: opt.catchup_base_timeout,
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
            if let Some(light_client) = modules.light_client {
                http_opt = http_opt.light_client(light_client);
            }
            if let Some(config) = modules.config {
                http_opt = http_opt.config(config);
            }

            http_opt
                .serve(move |metrics, consumer, storage| {
                    async move {
                        init_node(
                            genesis,
                            network_params,
                            metrics,
                            persistence,
                            l1_params,
                            storage,
                            consumer,
                            opt.is_da,
                            opt.identity,
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
                Box::new(NoMetrics),
                persistence,
                l1_params,
                None,
                NullEventConsumer,
                opt.is_da,
                opt.identity,
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

    use espresso_types::PubKey;
    use hotshot_types::{light_client::StateKeyPair, traits::signature_key::SignatureKey, x25519};
    use surf_disco::{Client, Url, error::ClientError};
    use tagged_base64::TaggedBase64;
    use tempfile::TempDir;
    use test_utils::reserve_tcp_port;
    use tokio::spawn;
    use vbs::version::Version;

    use super::*;
    use crate::{
        SequencerApiVersion,
        api::options::Http,
        genesis::{L1Finalized, StakeTableConfig},
        persistence::fs,
    };

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_startup_before_orchestrator() {
        let (pub_key, priv_key) = PubKey::generated_from_seed_indexed([0; 32], 0);
        let state_key = StateKeyPair::generate_from_seed_indexed([0; 32], 0);
        let x25519_kp = x25519::Keypair::generate().unwrap();

        let port1 = reserve_tcp_port().expect("OS should have ephemeral ports available");
        let port2 = reserve_tcp_port().expect("OS should have ephemeral ports available");
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
            drb_difficulty: None,
            drb_upgrade_difficulty: None,
            epoch_start_block: None,
            stake_table_capacity: None,
            genesis_version: Version { major: 0, minor: 1 },
            da_committees: None,
        };
        genesis.to_file(&genesis_file).unwrap();

        let modules = Modules {
            http: Some(Http::with_port(port1)),
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
            "--private-x25519-key",
            &TaggedBase64::try_from(x25519_kp.secret_key())
                .expect("valid key")
                .to_string(),
            "--cliquenet-bind-address",
            &format!("127.0.0.1:{port2}"),
            "--genesis-file",
            &genesis_file.display().to_string(),
        ]);

        // Start the sequencer in a background task. This process will not complete, because it will
        // be waiting for the orchestrator, but it should at least start up the API server and
        // populate some metrics.
        tracing::info!(port = %port1, "starting sequencer");
        let task = spawn(async move {
            if let Err(err) =
                init_with_storage(genesis, modules, opt, fs::Options::new(tmp.path().into())).await
            {
                tracing::error!("failed to start sequencer: {err:#}");
            }
        });

        // The healthcheck should eventually come up even though the node is waiting for the
        // orchestrator.
        tracing::info!("waiting for API to start");
        let url: Url = format!("http://localhost:{port1}").parse().unwrap();
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
                    espresso_utils::build_info::GIT_DESCRIBE,
                    espresso_utils::build_info::GIT_SHA,
                    espresso_utils::build_info::GIT_COMMIT_TIMESTAMP,
                )
                .as_str()
            ),
            "{lines:#?}"
        );
        let build_info_line = lines
            .iter()
            .find(|l| l.starts_with("consensus_build_info{"));
        assert!(
            build_info_line.is_some(),
            "missing consensus_build_info metric: {lines:#?}"
        );
        let build_info_line = build_info_line.unwrap();
        assert!(
            build_info_line.contains("modified="),
            "expected modified= in build_info: {lines:#?}"
        );
        assert!(
            build_info_line.contains("features="),
            "expected features= in build_info: {lines:#?}"
        );
        assert!(
            build_info_line.contains("testing"),
            "expected testing in features: {lines:#?}"
        );

        task.abort();
    }
}
