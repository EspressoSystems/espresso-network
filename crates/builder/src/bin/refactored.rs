use std::{num::NonZeroUsize, path::PathBuf, sync::Arc, time::Duration};

use anyhow::{bail, Context};
use builder::non_permissioned::build_instance_state;
use clap::Parser;
use espresso_types::{eth_signature_key::EthKeyPair, parse_duration, SeqTypes, SequencerVersions};
use hotshot_builder_refactored::service::{BuilderConfig, GlobalState};
use hotshot_builder_shared::utils::EventServiceStream;
use hotshot_types::traits::node_implementation::Versions;
use sequencer::{Genesis, L1Params, SequencerApiVersion};
use sequencer_utils::logging;
use tokio::select;
use url::Url;
use vbs::version::StaticVersionType;

#[derive(Parser, Clone, Debug)]
struct NonPermissionedBuilderOptions {
    /// URL of hotshot events API running on Espresso Sequencer DA committee node
    /// The builder will subscribe to this server to receive hotshot events
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_HOTSHOT_EVENT_STREAMING_API_URL",
        default_value = "http://localhost:22001"
    )]
    hotshot_event_streaming_url: Url,

    /// Mnemonic phrase for builder account.
    ///
    /// This is the address fees will be charged to.
    /// It must be funded with ETH in the Espresso fee ledger
    #[clap(long, env = "ESPRESSO_BUILDER_ETH_MNEMONIC")]
    eth_mnemonic: String,

    /// Index of a funded account derived from eth-mnemonic.
    #[clap(long, env = "ESPRESSO_BUILDER_ETH_ACCOUNT_INDEX", default_value = "8")]
    eth_account_index: u32,

    /// Url we will use for RPC communication with L1.
    #[clap(long, env = "ESPRESSO_BUILDER_L1_PROVIDER")]
    l1_provider_url: Url,

    /// Peer nodes use to fetch missing state
    #[clap(long, env = "ESPRESSO_SEQUENCER_STATE_PEERS", value_delimiter = ',')]
    state_peers: Vec<Url>,

    /// Port to run the builder server on.
    #[clap(short, long, env = "ESPRESSO_BUILDER_SERVER_PORT")]
    port: u16,

    /// BUILDER TRANSACTIONS CHANNEL CAPACITY
    #[clap(long, env = "ESPRESSO_BUILDER_TX_CHANNEL_CAPACITY")]
    pub tx_channel_capacity: NonZeroUsize,

    /// BUILDER HS EVENTS CHANNEL CAPACITY
    #[clap(long, env = "ESPRESSO_BUILDER_EVENT_CHANNEL_CAPACITY")]
    pub event_channel_capacity: NonZeroUsize,

    /// NETWORK INITIAL NODE COUNT
    #[clap(short, long, env = "ESPRESSO_BUILDER_INIT_NODE_COUNT")]
    node_count: NonZeroUsize,

    /// The amount of time a builder can wait before timing out a request to the API.
    #[clap(
        short,
        long,
        env = "ESPRESSO_BUILDER_WEBSERVER_RESPONSE_TIMEOUT_DURATION",
        default_value = "1s",
        value_parser = parse_duration
    )]
    max_api_timeout_duration: Duration,

    /// The amount of time a builder can wait before incrementing the max block size.
    #[clap(
        short = 'M',
        long,
        env = "ESPRESSO_BUILDER_MAX_BLOCK_SIZE_INCREMENT_PERIOD",
        default_value = "3600s",
        value_parser = parse_duration
    )]
    max_block_size_increment_period: Duration,

    /// The amount of time a builder can wait before incrementing the max block size.
    #[clap(
        long,
        env = "ESPRESSO_BUILDER_TX_STATUS_CACHE_SIZE",
        default_value = "819200"
    )]
    tx_status_cache_size: usize,

    /// Path to TOML file containing genesis state.
    #[clap(long, name = "GENESIS_FILE", env = "ESPRESSO_BUILDER_GENESIS_FILE")]
    genesis_file: PathBuf,

    #[clap(flatten)]
    logging: logging::Config,
}

async fn start_builder(
    global_state: Arc<GlobalState<SeqTypes>>,
    event_service_url: Url,
    builder_api_url: Url,
) -> anyhow::Result<()> {
    let app = Arc::clone(&global_state).into_app()?;

    tracing::info!(%event_service_url, "Connecting to event service");
    let event_stream = EventServiceStream::<SeqTypes, sequencer::SequencerApiVersion>::connect(
        event_service_url.clone(),
    )
    .await
    .context("Couldn't connect to event stream")?;

    tracing::info!(%builder_api_url, "Starting builder");
    select! {
        event_loop_exit = Arc::clone(&global_state).start_event_loop(event_stream) => {
            tracing::error!(?event_loop_exit, "Builder event loop quit unexpectedly");
        }
        api_exit = app.serve(builder_api_url, SequencerApiVersion::instance()) => {
            tracing::error!(?api_exit, "Builder API App quit unexpectedly");
        }
    }

    bail!("Builder quit unexpectedly")
}

async fn run<V: Versions>(
    genesis: Genesis,
    opt: NonPermissionedBuilderOptions,
) -> anyhow::Result<()> {
    let l1_params = L1Params {
        urls: vec![opt.l1_provider_url],
        options: Default::default(),
    };

    let builder_key_pair = EthKeyPair::from_mnemonic(&opt.eth_mnemonic, opt.eth_account_index)?;
    let builder_server_url: Url = format!("http://0.0.0.0:{}", opt.port).parse().unwrap();

    let base_fee = genesis.max_base_fee();
    let instance_state = build_instance_state::<V>(genesis, l1_params, opt.state_peers);

    // make the txn timeout as 1/4 of the api_response_timeout_duration
    let txn_timeout_duration = opt.max_api_timeout_duration / 4;
    let protocol_max_block_size = instance_state.chain_config.max_block_size.into();

    let config = BuilderConfig {
        builder_keys: (builder_key_pair.fee_account(), builder_key_pair),
        max_api_waiting_time: opt.max_api_timeout_duration,
        max_block_size_increment_period: opt.max_block_size_increment_period,
        maximize_txn_capture_timeout: txn_timeout_duration,
        txn_garbage_collect_duration: Duration::from_secs(60),
        txn_channel_capacity: opt.event_channel_capacity.get(),
        tx_status_cache_capacity: opt.tx_status_cache_size,
        base_fee: base_fee.as_u64().context(
            "the base fee exceeds the maximum amount that a builder can pay (defined by u64::MAX)",
        )?,
    };
    tracing::info!(?config, "Assembled builder config");

    let global_state: Arc<GlobalState<SeqTypes>> = GlobalState::<SeqTypes>::new(
        config,
        instance_state,
        protocol_max_block_size,
        opt.node_count.into(),
    );

    start_builder(
        global_state,
        opt.hotshot_event_streaming_url,
        builder_server_url,
    )
    .await
}

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let opt = NonPermissionedBuilderOptions::parse();
    opt.logging.init();

    let genesis = Genesis::from_file(&opt.genesis_file)?;
    tracing::info!(?genesis, "genesis");

    let base = genesis.base_version;
    let upgrade = genesis.upgrade_version;

    match (base, upgrade) {
        #[cfg(all(feature = "fee", feature = "pos"))]
        (espresso_types::FeeVersion::VERSION, espresso_types::EpochVersion::VERSION) => {
            run::<SequencerVersions<espresso_types::FeeVersion, espresso_types::EpochVersion>>(
                genesis, opt,
            )
            .await
        },
        #[cfg(feature = "pos")]
        (espresso_types::EpochVersion::VERSION, _) => {
            // Specifying V0_0 disables upgrades
            run::<SequencerVersions<espresso_types::EpochVersion, espresso_types::V0_0>>(
                genesis, opt,
            )
            .await
        },
        #[cfg(feature = "fee")]
        (espresso_types::FeeVersion::VERSION, _) => {
            // Specifying V0_0 disables upgrades
            run::<SequencerVersions<espresso_types::FeeVersion, espresso_types::V0_0>>(genesis, opt)
                .await
        },
        _ => panic!(
            "Invalid base ({base}) and upgrade ({upgrade}) versions specified in the toml file."
        ),
    }
}

#[cfg(test)]
mod test {
    use std::{
        sync::Arc,
        time::{Duration, Instant},
    };

    use alloy::node_bindings::Anvil;
    use espresso_types::{
        ChainConfig, FeeAccount, MockSequencerVersions, NamespaceId, Transaction,
    };
    use futures::stream::StreamExt;
    use hotshot::{
        traits::BlockPayload,
        types::{
            BLSPubKey,
            EventType::{self},
            SignatureKey,
        },
    };
    use hotshot_builder_api::v0_2::block_info::{
        AvailableBlockData, AvailableBlockHeaderInputV1, AvailableBlockInfo,
    };
    use hotshot_builder_refactored::service::BuilderConfig;
    use hotshot_types::{
        data::Leaf2,
        traits::{node_implementation::NodeType, signature_key::BuilderSignatureKey as _},
    };
    use portpicker::pick_unused_port;
    use sequencer::{
        api::{
            options::HotshotEvents,
            test_helpers::{TestNetwork, TestNetworkConfigBuilder},
            Options,
        },
        persistence::{self},
        testing::TestConfigBuilder,
        SequencerApiVersion,
    };
    use sequencer_utils::test_utils::setup_test;
    use surf_disco::Client;
    use tempfile::TempDir;
    use vbs::version::StaticVersion;

    use super::*;

    /// Test the non-permissioned builder core
    /// It creates a memory hotshot network and launches the hotshot event streaming api
    /// Builder subscrived to this api, and server the hotshot client request and the private mempool tx submission
    #[tokio::test(flavor = "multi_thread")]
    async fn test_builder() {
        setup_test();

        let query_port = pick_unused_port().expect("No ports free");

        let event_port = pick_unused_port().expect("No ports free");
        let event_service_url: Url = format!("http://localhost:{event_port}").parse().unwrap();

        let builder_port = pick_unused_port().expect("No ports free");
        let builder_api_url: Url = format!("http://localhost:{builder_port}").parse().unwrap();

        // Set up and start the network
        let anvil = Anvil::new().spawn();
        let l1 = anvil.endpoint().parse().unwrap();
        let network_config = TestConfigBuilder::default().l1_url(l1).build();

        let tmpdir = TempDir::new().unwrap();

        let config = TestNetworkConfigBuilder::default()
            .api_config(
                Options::with_port(query_port)
                    .submit(Default::default())
                    .query_fs(
                        Default::default(),
                        persistence::fs::Options::new(tmpdir.path().to_owned()),
                    )
                    .hotshot_events(HotshotEvents {
                        events_service_port: event_port,
                    }),
            )
            .network_config(network_config)
            .build();
        let network = TestNetwork::new(config, MockSequencerVersions::new()).await;

        let builder_keys = FeeAccount::generated_from_seed_indexed([201_u8; 32], 2011_u64);

        let config = BuilderConfig {
            builder_keys: builder_keys.clone(),
            max_api_waiting_time: Duration::from_millis(2000),
            max_block_size_increment_period: Duration::from_secs(60),
            maximize_txn_capture_timeout: Duration::from_millis(500),
            txn_garbage_collect_duration: Duration::from_secs(60),
            txn_channel_capacity: 512,
            tx_status_cache_capacity: 8192,
            base_fee: ChainConfig::default().base_fee.as_u64().expect(
                "the base fee exceeds the maximum amount that a builder can pay (defined by \
                 u64::MAX)",
            ),
        };

        let global_state: Arc<GlobalState<SeqTypes>> = GlobalState::<SeqTypes>::new(
            config,
            network.server.node_state(),
            network
                .server
                .node_state()
                .chain_config
                .max_block_size
                .into(),
            network.cfg.num_nodes(),
        );

        tokio::spawn(start_builder(
            global_state,
            event_service_url.clone(),
            builder_api_url.clone(),
        ));

        let events_service_client = Client::<
            hotshot_events_service::events::Error,
            SequencerApiVersion,
        >::new(event_service_url.clone());
        events_service_client.connect(None).await;

        let builder_client =
            Client::<hotshot_builder_api::v0_1::builder::Error, StaticVersion<0, 1>>::new(
                builder_api_url.clone(),
            );
        assert!(builder_client.connect(Some(Duration::from_secs(60))).await);

        let (hotshot_client_pub_key, hotshot_client_private_key) =
            <BLSPubKey as SignatureKey>::generated_from_seed_indexed([207_u8; 32], 2011_u64);
        let txn = Transaction::new(NamespaceId::from(1_u32), vec![1, 2, 3]);

        builder_client
            .post::<()>("txn_submit/submit")
            .body_json(&txn)
            .unwrap()
            .send()
            .await
            .unwrap();

        let start = Instant::now();
        let mut subscribed_events = network.server.event_stream().await;
        let (available_block_info, view_num) = loop {
            if start.elapsed() > Duration::from_secs(10) {
                panic!("Didn't get a quorum proposal in 10 seconds");
            }

            let event = subscribed_events.next().await.unwrap();
            tracing::warn!("Event: {:?}", event.event);
            if let EventType::QuorumProposal { proposal, .. } = event.event {
                let next_view_number = proposal.data.view_number() + 1;
                let parent_commitment =
                    Leaf2::from_quorum_proposal(&proposal.data).payload_commitment();
                let encoded_signature = <SeqTypes as NodeType>::SignatureKey::sign(
                    &hotshot_client_private_key,
                    parent_commitment.as_ref(),
                )
                .expect("Claim block signing failed");
                let available_blocks = builder_client
                    .get::<Vec<AvailableBlockInfo<SeqTypes>>>(&format!(
                        "block_info/availableblocks/{parent_commitment}/{next_view_number}/\
                         {hotshot_client_pub_key}/{encoded_signature}"
                    ))
                    .send()
                    .await
                    .expect("Error getting available blocks");
                assert!(!available_blocks.is_empty());
                break (available_blocks, next_view_number);
            }
        };

        let builder_commitment = available_block_info[0].block_hash.clone();
        let encoded_signature = <SeqTypes as NodeType>::SignatureKey::sign(
            &hotshot_client_private_key,
            builder_commitment.as_ref(),
        )
        .expect("Claim block signing failed");

        let available_block_data = builder_client
            .get::<AvailableBlockData<SeqTypes>>(&format!(
                "block_info/claimblock/{builder_commitment}/{view_num}/{hotshot_client_pub_key}/\
                 {encoded_signature}"
            ))
            .send()
            .await
            .unwrap();

        assert_eq!(
            available_block_data
                .block_payload
                .transactions(&available_block_data.metadata)
                .collect::<Vec<_>>(),
            vec![txn]
        );

        builder_client
            .get::<AvailableBlockHeaderInputV1<SeqTypes>>(&format!(
                "block_info/claimheaderinput/{builder_commitment}/{view_num}/\
                 {hotshot_client_pub_key}/{encoded_signature}"
            ))
            .send()
            .await
            .unwrap();

        let addr = builder_client
            .get::<FeeAccount>("block_info/builderaddress")
            .send()
            .await
            .unwrap();
        assert_eq!(addr, builder_keys.0);
    }
}
