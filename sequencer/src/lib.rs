pub mod api;
pub mod catchup;
pub mod context;
pub mod genesis;
mod proposal_fetcher;
mod request_response;

mod external_event_handler;
pub mod options;
pub mod state_signature;

mod restart_tests;

mod message_compat_tests;

use std::sync::Arc;

use alloy::primitives::U256;
use anyhow::Context;
use async_lock::RwLock;
use catchup::StatePeers;
use context::SequencerContext;
use espresso_types::{
    traits::{EventConsumer, MembershipPersistence},
    BackoffParams, EpochCommittees, L1ClientOptions, NodeState, PubKey, SeqTypes,
    SolverAuctionResultsProvider, ValidatedState,
};
use genesis::L1Finalized;
// Should move `STAKE_TABLE_CAPACITY` in the sequencer repo when we have variate stake table support
use hotshot_libp2p_networking::network::behaviours::dht::store::persistent::DhtNoPersistence;
use libp2p::Multiaddr;
use network::libp2p::split_off_peer_id;
use options::Identity;
use proposal_fetcher::ProposalFetcherConfig;
use tokio::select;
use tracing::info;
use url::Url;
pub mod persistence;
pub mod state;
use std::{fmt::Debug, marker::PhantomData, time::Duration};

use derivative::Derivative;
use espresso_types::v0::traits::SequencerPersistence;
pub use genesis::Genesis;
use hotshot::{
    traits::implementations::{
        derive_libp2p_multiaddr, derive_libp2p_peer_id, CdnMetricsValue, CdnTopic,
        CombinedNetworks, GossipConfig, KeyPair, Libp2pNetwork, MemoryNetwork, PushCdnNetwork,
        RequestResponseConfig, WrappedSignatureKey,
    },
    types::SignatureKey,
    MarketplaceConfig,
};
use hotshot_orchestrator::client::{get_complete_config, OrchestratorClient};
use hotshot_types::{
    data::ViewNumber,
    epoch_membership::EpochMembershipCoordinator,
    light_client::{StateKeyPair, StateSignKey},
    signature_key::{BLSPrivKey, BLSPubKey},
    traits::{
        metrics::{Metrics, NoMetrics},
        network::ConnectedNetwork,
        node_implementation::{NodeImplementation, NodeType, Versions},
    },
    utils::BuilderCommitment,
    ValidatorConfig,
};
pub use options::Options;
use serde::{Deserialize, Serialize};
use vbs::version::{StaticVersion, StaticVersionType};
pub mod network;

mod run;
pub use run::main;

/// The Sequencer node is generic over the hotshot CommChannel.
#[derive(Derivative, Serialize, Deserialize)]
#[derivative(
    Copy(bound = ""),
    Debug(bound = ""),
    Default(bound = ""),
    PartialEq(bound = ""),
    Eq(bound = ""),
    Hash(bound = "")
)]
pub struct Node<N: ConnectedNetwork<PubKey>, P: SequencerPersistence>(PhantomData<fn(&N, &P)>);

// Using derivative to derive Clone triggers the clippy lint
// https://rust-lang.github.io/rust-clippy/master/index.html#/incorrect_clone_impl_on_copy_type
impl<N: ConnectedNetwork<PubKey>, P: SequencerPersistence> Clone for Node<N, P> {
    fn clone(&self) -> Self {
        *self
    }
}

pub type SequencerApiVersion = StaticVersion<0, 1>;

impl<N: ConnectedNetwork<PubKey>, P: SequencerPersistence> NodeImplementation<SeqTypes>
    for Node<N, P>
{
    type Network = N;
    type Storage = Arc<P>;
    type AuctionResultsProvider = SolverAuctionResultsProvider;
}

#[derive(Clone, Debug)]
pub struct NetworkParams {
    /// The address where a CDN marshal is located
    pub cdn_endpoint: String,
    pub orchestrator_url: Url,
    pub state_relay_server_url: Url,
    pub private_staking_key: BLSPrivKey,
    pub private_state_key: StateSignKey,
    pub state_peers: Vec<Url>,
    pub config_peers: Option<Vec<Url>>,
    pub catchup_backoff: BackoffParams,
    /// The address to advertise as our public API's URL
    pub public_api_url: Option<Url>,

    /// The address to send to other Libp2p nodes to contact us
    pub libp2p_advertise_address: String,
    /// The address to bind to for Libp2p
    pub libp2p_bind_address: String,
    /// The (optional) bootstrap node addresses for Libp2p. If supplied, these will
    /// override the bootstrap nodes specified in the config file.
    pub libp2p_bootstrap_nodes: Option<Vec<Multiaddr>>,

    /// The heartbeat interval
    pub libp2p_heartbeat_interval: Duration,

    /// The number of past heartbeats to gossip about
    pub libp2p_history_gossip: usize,
    /// The number of past heartbeats to remember the full messages for
    pub libp2p_history_length: usize,

    /// The target number of peers in the mesh
    pub libp2p_mesh_n: usize,
    /// The maximum number of peers in the mesh
    pub libp2p_mesh_n_high: usize,
    /// The minimum number of peers in the mesh
    pub libp2p_mesh_n_low: usize,
    /// The minimum number of mesh peers that must be outbound
    pub libp2p_mesh_outbound_min: usize,

    /// The maximum gossip message size
    pub libp2p_max_gossip_transmit_size: usize,

    /// The maximum direct message size
    pub libp2p_max_direct_transmit_size: u64,

    /// The maximum number of IHAVE messages to accept from a Libp2p peer within a heartbeat
    pub libp2p_max_ihave_length: usize,

    /// The maximum number of IHAVE messages to accept from a Libp2p peer within a heartbeat
    pub libp2p_max_ihave_messages: usize,

    /// The time period that message hashes are stored in the cache
    pub libp2p_published_message_ids_cache_time: Duration,

    /// The time to wait for a Libp2p message requested through IWANT following an IHAVE advertisement
    pub libp2p_iwant_followup_time: Duration,

    /// The maximum number of Libp2p messages we will process in a given RPC
    pub libp2p_max_messages_per_rpc: Option<usize>,

    /// How many times we will allow a peer to request the same message id through IWANT gossip before we start ignoring them
    pub libp2p_gossip_retransmission: u32,

    /// If enabled newly created messages will always be sent to all peers that are subscribed to the topic and have a good enough score
    pub libp2p_flood_publish: bool,

    /// The time period that Libp2p message hashes are stored in the cache
    pub libp2p_duplicate_cache_time: Duration,

    /// Time to live for Libp2p fanout peers
    pub libp2p_fanout_ttl: Duration,

    /// Initial delay in each Libp2p heartbeat
    pub libp2p_heartbeat_initial_delay: Duration,

    /// How many Libp2p peers we will emit gossip to at each heartbeat
    pub libp2p_gossip_factor: f64,

    /// Minimum number of Libp2p peers to emit gossip to during a heartbeat
    pub libp2p_gossip_lazy: usize,
}

pub struct L1Params {
    pub urls: Vec<Url>,
    pub options: L1ClientOptions,
}

#[allow(clippy::too_many_arguments)]
pub async fn init_node<P: SequencerPersistence + MembershipPersistence, V: Versions>(
    genesis: Genesis,
    network_params: NetworkParams,
    metrics: &dyn Metrics,
    persistence: P,
    l1_params: L1Params,
    seq_versions: V,
    event_consumer: impl EventConsumer + 'static,
    is_da: bool,
    identity: Identity,
    marketplace_config: MarketplaceConfig<SeqTypes, Node<network::Production, P>>,
    proposal_fetcher_config: ProposalFetcherConfig,
) -> anyhow::Result<SequencerContext<network::Production, P, V>> {
    // Expose git information via status API.
    metrics
        .text_family(
            "version".into(),
            vec!["rev".into(), "desc".into(), "timestamp".into()],
        )
        .create(vec![
            env!("VERGEN_GIT_SHA").into(),
            env!("VERGEN_GIT_DESCRIBE").into(),
            env!("VERGEN_GIT_COMMIT_TIMESTAMP").into(),
        ]);

    // Expose Node Entity Information via the status/metrics API
    metrics
        .text_family(
            "node_identity_general".into(),
            vec![
                "name".into(),
                "company_name".into(),
                "company_website".into(),
                "operating_system".into(),
                "node_type".into(),
                "network_type".into(),
            ],
        )
        .create(vec![
            identity.node_name.unwrap_or("".into()),
            identity.company_name.unwrap_or("".into()),
            identity
                .company_website
                .map(|u| u.into())
                .unwrap_or("".into()),
            identity.operating_system.unwrap_or("".into()),
            identity.node_type.unwrap_or("".into()),
            identity.network_type.unwrap_or("".into()),
        ]);

    // Expose Node Identity Location via the status/metrics API
    metrics
        .text_family(
            "node_identity_location".into(),
            vec!["country".into(), "latitude".into(), "longitude".into()],
        )
        .create(vec![
            identity.country_code.unwrap_or("".into()),
            identity
                .latitude
                .map(|l| l.to_string())
                .unwrap_or("".into()),
            identity
                .longitude
                .map(|l| l.to_string())
                .unwrap_or("".into()),
        ]);

    // Stick our public key in `metrics` so it is easily accessible via the status API.
    let pub_key = BLSPubKey::from_private(&network_params.private_staking_key);
    metrics
        .text_family("node".into(), vec!["key".into()])
        .create(vec![pub_key.to_string()]);

    // Parse the Libp2p bind and advertise addresses to multiaddresses
    let libp2p_bind_address = derive_libp2p_multiaddr(&network_params.libp2p_bind_address)
        .with_context(|| {
            format!(
                "Failed to derive Libp2p bind address of {}",
                &network_params.libp2p_bind_address
            )
        })?;
    let libp2p_advertise_address =
        derive_libp2p_multiaddr(&network_params.libp2p_advertise_address).with_context(|| {
            format!(
                "Failed to derive Libp2p advertise address of {}",
                &network_params.libp2p_advertise_address
            )
        })?;

    info!("Libp2p bind address: {}", libp2p_bind_address);
    info!("Libp2p advertise address: {}", libp2p_advertise_address);

    // Orchestrator client
    let orchestrator_client = OrchestratorClient::new(network_params.orchestrator_url);
    let state_key_pair = StateKeyPair::from_sign_key(network_params.private_state_key);
    let validator_config = ValidatorConfig {
        public_key: pub_key,
        private_key: network_params.private_staking_key,
        stake_value: U256::ONE,
        state_public_key: state_key_pair.ver_key(),
        state_private_key: state_key_pair.sign_key(),
        is_da,
    };

    // Derive our Libp2p public key from our private key
    let libp2p_public_key = derive_libp2p_peer_id::<<SeqTypes as NodeType>::SignatureKey>(
        &validator_config.private_key,
    )
    .with_context(|| "Failed to derive Libp2p peer ID")?;

    // Print the libp2p public key
    info!("Starting Libp2p with PeerID: {}", libp2p_public_key);

    let (mut network_config, wait_for_orchestrator) = match (
        persistence.load_config().await?,
        network_params.config_peers,
    ) {
        (Some(config), _) => {
            tracing::info!("loaded network config from storage, rejoining existing network");
            (config, false)
        },
        // If we were told to fetch the config from an already-started peer, do so.
        (None, Some(peers)) => {
            tracing::info!(?peers, "loading network config from peers");
            let peers = StatePeers::<SequencerApiVersion>::from_urls(
                peers,
                network_params.catchup_backoff,
                &NoMetrics,
            );
            let config = peers.fetch_config(validator_config.clone()).await?;

            tracing::info!(
                node_id = config.node_index,
                stake_table = ?config.config.known_nodes_with_stake,
                "loaded config",
            );
            persistence.save_config(&config).await?;
            (config, false)
        },
        // Otherwise, this is a fresh network; load from the orchestrator.
        (None, None) => {
            tracing::info!("loading network config from orchestrator");
            tracing::error!(
                "waiting for other nodes to connect, DO NOT RESTART until fully connected"
            );
            let config = get_complete_config(
                &orchestrator_client,
                validator_config.clone(),
                // Register in our Libp2p advertise address and public key so other nodes
                // can contact us on startup
                Some(libp2p_advertise_address),
                Some(libp2p_public_key),
            )
            .await?
            .0;

            tracing::info!(
                node_id = config.node_index,
                stake_table = ?config.config.known_nodes_with_stake,
                "loaded config",
            );
            persistence.save_config(&config).await?;
            tracing::error!("all nodes connected");
            (config, true)
        },
    };

    if let Some(upgrade) = genesis.upgrades.get(&V::Upgrade::VERSION) {
        upgrade.set_hotshot_config_parameters(&mut network_config.config);
    }

    let epoch_height = genesis.epoch_height.unwrap_or_default();
    let epoch_start_block = genesis.epoch_start_block.unwrap_or_default();

    tracing::info!("setting epoch_height={epoch_height:?}");
    tracing::info!("setting epoch_start_block={epoch_start_block:?}");
    network_config.config.epoch_height = epoch_height;
    network_config.config.epoch_start_block = epoch_start_block;

    // If the `Libp2p` bootstrap nodes were supplied via the command line, override those
    // present in the config file.
    if let Some(bootstrap_nodes) = network_params.libp2p_bootstrap_nodes {
        if let Some(libp2p_config) = network_config.libp2p_config.as_mut() {
            // If the libp2p configuration is present, we can override the bootstrap nodes.

            // Split off the peer ID from the addresses
            libp2p_config.bootstrap_nodes = bootstrap_nodes
                .into_iter()
                .map(split_off_peer_id)
                .collect::<Result<Vec<_>, _>>()
                .with_context(|| "Failed to parse peer ID from bootstrap node")?;
        } else {
            // If not, don't try launching with them. Eventually we may want to
            // provide a default configuration here instead.
            tracing::warn!("No libp2p configuration found, ignoring supplied bootstrap nodes");
        }
    }

    let node_index = network_config.node_index;

    // If we are a DA node, we need to subscribe to the DA topic
    let topics = {
        let mut topics = vec![CdnTopic::Global];
        if is_da {
            topics.push(CdnTopic::Da);
        }
        topics
    };

    // Initialize the push CDN network (and perform the initial connection)
    let cdn_network = PushCdnNetwork::new(
        network_params.cdn_endpoint,
        topics,
        KeyPair {
            public_key: WrappedSignatureKey(validator_config.public_key),
            private_key: validator_config.private_key.clone(),
        },
        CdnMetricsValue::new(metrics),
    )
    .with_context(|| format!("Failed to create CDN network {node_index}"))?;

    // Configure gossipsub based on the command line options
    let gossip_config = GossipConfig {
        heartbeat_interval: network_params.libp2p_heartbeat_interval,
        history_gossip: network_params.libp2p_history_gossip,
        history_length: network_params.libp2p_history_length,
        mesh_n: network_params.libp2p_mesh_n,
        mesh_n_high: network_params.libp2p_mesh_n_high,
        mesh_n_low: network_params.libp2p_mesh_n_low,
        mesh_outbound_min: network_params.libp2p_mesh_outbound_min,
        max_ihave_messages: network_params.libp2p_max_ihave_messages,
        max_transmit_size: network_params.libp2p_max_gossip_transmit_size,
        max_ihave_length: network_params.libp2p_max_ihave_length,
        published_message_ids_cache_time: network_params.libp2p_published_message_ids_cache_time,
        iwant_followup_time: network_params.libp2p_iwant_followup_time,
        max_messages_per_rpc: network_params.libp2p_max_messages_per_rpc,
        gossip_retransmission: network_params.libp2p_gossip_retransmission,
        flood_publish: network_params.libp2p_flood_publish,
        duplicate_cache_time: network_params.libp2p_duplicate_cache_time,
        fanout_ttl: network_params.libp2p_fanout_ttl,
        heartbeat_initial_delay: network_params.libp2p_heartbeat_initial_delay,
        gossip_factor: network_params.libp2p_gossip_factor,
        gossip_lazy: network_params.libp2p_gossip_lazy,
    };

    // Configure request/response based on the command line options
    let request_response_config = RequestResponseConfig {
        request_size_maximum: network_params.libp2p_max_direct_transmit_size,
        response_size_maximum: network_params.libp2p_max_direct_transmit_size,
    };

    let l1_client = l1_params
        .options
        .with_metrics(metrics)
        .connect(l1_params.urls)
        .with_context(|| "failed to create L1 client")?;
    genesis.validate_fee_contract(&l1_client).await?;

    l1_client.spawn_tasks().await;
    let l1_genesis = match genesis.l1_finalized {
        L1Finalized::Block(b) => b,
        L1Finalized::Number { number } => l1_client.wait_for_finalized_block(number).await,
        L1Finalized::Timestamp { timestamp } => {
            l1_client
                .wait_for_finalized_block_with_timestamp(U256::from(timestamp.unix_timestamp()))
                .await
        },
    };

    let mut genesis_state = ValidatedState {
        chain_config: genesis.chain_config.into(),
        ..Default::default()
    };
    for (address, amount) in genesis.accounts {
        tracing::info!(%address, %amount, "Prefunding account for demo");
        genesis_state.prefund_account(address, amount);
    }

    let peers = catchup::local_and_remote(
        persistence.clone(),
        StatePeers::<SequencerApiVersion>::from_urls(
            network_params.state_peers,
            network_params.catchup_backoff,
            metrics,
        ),
    )
    .await;
    // Create the HotShot membership
    let membership = EpochCommittees::new_stake(
        network_config.config.known_nodes_with_stake.clone(),
        network_config.config.known_da_nodes.clone(),
        l1_client.clone(),
        genesis.chain_config,
        peers.clone(),
        persistence.clone(),
    );

    let membership: Arc<RwLock<EpochCommittees>> = Arc::new(RwLock::new(membership));
    let coordinator =
        EpochMembershipCoordinator::new(membership, network_config.config.epoch_height);

    let instance_state = NodeState {
        chain_config: genesis.chain_config,
        l1_client,
        genesis_header: genesis.header,
        genesis_state,
        l1_genesis: Some(l1_genesis),
        node_id: node_index,
        upgrades: genesis.upgrades,
        current_version: V::Base::VERSION,
        epoch_height: Some(epoch_height),
        peers,
        coordinator: coordinator.clone(),
    };

    // Initialize the Libp2p network
    let network = {
        let p2p_network = Libp2pNetwork::from_config(
            network_config.clone(),
            DhtNoPersistence,
            coordinator.membership().clone(),
            gossip_config,
            request_response_config,
            libp2p_bind_address,
            &validator_config.public_key,
            // We need the private key so we can derive our Libp2p keypair
            // (using https://docs.rs/blake3/latest/blake3/fn.derive_key.html)
            &validator_config.private_key,
            hotshot::traits::implementations::Libp2pMetricsValue::new(metrics),
        )
        .await
        .with_context(|| {
            format!(
                "Failed to create libp2p network on node {node_index}; binding to {:?}",
                network_params.libp2p_bind_address
            )
        })?;

        tracing::warn!("Waiting for at least one connection to be initialized");
        select! {
            _ = cdn_network.wait_for_ready() => {
                tracing::warn!("CDN connection initialized");
            },
            _ = p2p_network.wait_for_ready() => {
                tracing::warn!("P2P connection initialized");
            },
        };

        // Combine the CDN and P2P networks
        Arc::from(CombinedNetworks::new(
            cdn_network,
            p2p_network,
            Some(Duration::from_secs(1)),
        ))
    };

    let mut ctx = SequencerContext::init(
        network_config,
        validator_config,
        coordinator,
        instance_state,
        persistence,
        network,
        Some(network_params.state_relay_server_url),
        metrics,
        genesis.stake_table.capacity,
        event_consumer,
        seq_versions,
        marketplace_config,
        proposal_fetcher_config,
    )
    .await?;
    if wait_for_orchestrator {
        ctx = ctx.wait_for_orchestrator(orchestrator_client);
    }
    Ok(ctx)
}

pub fn empty_builder_commitment() -> BuilderCommitment {
    BuilderCommitment::from_bytes([])
}

#[cfg(any(test, feature = "testing"))]
pub mod testing {
    use std::{
        collections::{BTreeMap, HashMap},
        time::Duration,
    };

    use alloy::primitives::U256;
    use async_lock::RwLock;
    use catchup::NullStateCatchup;
    use committable::Committable;
    use espresso_types::{
        eth_signature_key::EthKeyPair,
        v0::traits::{EventConsumer, NullEventConsumer, PersistenceOptions, StateCatchup},
        Event, FeeAccount, L1Client, MarketplaceVersion, NetworkConfig, PubKey, SeqTypes,
        Transaction, Upgrade,
    };
    use futures::{
        future::join_all,
        stream::{Stream, StreamExt},
    };
    use hotshot::{
        traits::{
            implementations::{MasterMap, MemoryNetwork},
            BlockPayload,
        },
        types::EventType::Decide,
    };
    use hotshot_stake_table::vec_based::StakeTable;
    use hotshot_testing::block_builder::{
        BuilderTask, SimpleBuilderImplementation, TestBuilderImplementation,
    };
    use hotshot_types::{
        event::LeafInfo,
        light_client::{CircuitField, StateKeyPair, StateVerKey},
        traits::{
            block_contents::BlockHeader,
            metrics::NoMetrics,
            network::Topic,
            signature_key::{BuilderSignatureKey, StakeTableEntryType},
            stake_table::StakeTableScheme,
        },
        HotShotConfig, PeerConfig,
    };
    use marketplace_builder_core::{
        hooks::NoHooks,
        service::{BuilderConfig, GlobalState},
    };
    use portpicker::pick_unused_port;
    use tokio::spawn;
    use vbs::version::Version;

    use super::*;
    use crate::persistence::no_storage::{self, NoStorage};

    const STAKE_TABLE_CAPACITY_FOR_TEST: u64 = 10;
    const BUILDER_CHANNEL_CAPACITY_FOR_TEST: usize = 128;

    struct MarketplaceBuilderImplementation {
        global_state: Arc<GlobalState<SeqTypes, NoHooks<SeqTypes>>>,
    }

    impl BuilderTask<SeqTypes> for MarketplaceBuilderImplementation {
        fn start(
            self: Box<Self>,
            stream: Box<
                dyn Stream<Item = hotshot::types::Event<SeqTypes>>
                    + std::marker::Unpin
                    + Send
                    + 'static,
            >,
        ) {
            spawn(async move {
                let res = self.global_state.start_event_loop(stream).await;
                tracing::error!(?res, "Testing marketplace builder service exited");
            });
        }
    }

    pub async fn run_marketplace_builder<const NUM_NODES: usize>(
        port: Option<u16>,
    ) -> (Box<dyn BuilderTask<SeqTypes>>, Url) {
        let builder_key_pair = TestConfig::<0>::builder_key();
        let port = port.unwrap_or_else(|| pick_unused_port().expect("No ports available"));

        // This should never fail.
        let url: Url = format!("http://localhost:{port}")
            .parse()
            .expect("Failed to parse builder URL");

        // create the global state
        let global_state = GlobalState::new(
            BuilderConfig {
                builder_keys: (builder_key_pair.fee_account(), builder_key_pair),
                api_timeout: Duration::from_secs(60),
                tx_capture_timeout: Duration::from_millis(100),
                txn_garbage_collect_duration: Duration::from_secs(60),
                txn_channel_capacity: BUILDER_CHANNEL_CAPACITY_FOR_TEST,
                tx_status_cache_capacity: 81920,
                base_fee: 10,
            },
            NoHooks(PhantomData),
        );

        // Create and spawn the tide-disco app to serve the builder APIs
        let app = Arc::clone(&global_state)
            .into_app()
            .expect("Failed to create builder tide-disco app");

        spawn(
            app.serve(
                format!("http://0.0.0.0:{port}")
                    .parse::<Url>()
                    .expect("Failed to parse builder listener"),
                MarketplaceVersion::instance(),
            ),
        );

        // Pass on the builder task to be injected in the testing harness
        (
            Box::new(MarketplaceBuilderImplementation { global_state }),
            url,
        )
    }

    pub async fn run_test_builder<const NUM_NODES: usize>(
        port: Option<u16>,
    ) -> (Box<dyn BuilderTask<SeqTypes>>, Url) {
        let port = port.unwrap_or_else(|| pick_unused_port().expect("No ports available"));

        // This should never fail.
        let url: Url = format!("http://localhost:{port}")
            .parse()
            .expect("Failed to parse builder URL");

        (
            <SimpleBuilderImplementation as TestBuilderImplementation<SeqTypes>>::start(
                NUM_NODES,
                format!("http://0.0.0.0:{port}")
                    .parse()
                    .expect("Failed to parse builder listener"),
                (),
                HashMap::new(),
            )
            .await,
            url,
        )
    }

    pub struct TestConfigBuilder<const NUM_NODES: usize> {
        config: HotShotConfig<SeqTypes>,
        priv_keys: Vec<BLSPrivKey>,
        state_key_pairs: Vec<StateKeyPair>,
        master_map: Arc<MasterMap<PubKey>>,
        l1_url: Url,
        state_relay_url: Option<Url>,
        builder_port: Option<u16>,
        marketplace_builder_port: Option<u16>,
        upgrades: BTreeMap<Version, Upgrade>,
    }

    impl<const NUM_NODES: usize> TestConfigBuilder<NUM_NODES> {
        pub fn builder_port(mut self, builder_port: Option<u16>) -> Self {
            self.builder_port = builder_port;
            self
        }

        pub fn marketplace_builder_port(mut self, port: Option<u16>) -> Self {
            self.marketplace_builder_port = port;
            self
        }

        pub fn state_relay_url(mut self, url: Url) -> Self {
            self.state_relay_url = Some(url);
            self
        }

        pub fn l1_url(mut self, l1_url: Url) -> Self {
            self.l1_url = l1_url;
            self
        }

        pub fn upgrades<V: Versions>(mut self, upgrades: BTreeMap<Version, Upgrade>) -> Self {
            let upgrade = upgrades.get(&<V as Versions>::Upgrade::VERSION).unwrap();
            upgrade.set_hotshot_config_parameters(&mut self.config);
            self.upgrades = upgrades;
            self
        }

        pub fn epoch_height(mut self, epoch_height: u64) -> Self {
            self.config.epoch_height = epoch_height;
            self
        }

        pub fn build(self) -> TestConfig<NUM_NODES> {
            TestConfig {
                config: self.config,
                priv_keys: self.priv_keys,
                state_key_pairs: self.state_key_pairs,
                master_map: self.master_map,
                l1_url: self.l1_url,
                state_relay_url: self.state_relay_url,
                marketplace_builder_port: self.marketplace_builder_port,
                builder_port: self.builder_port,
                upgrades: self.upgrades,
            }
        }
    }

    impl<const NUM_NODES: usize> Default for TestConfigBuilder<NUM_NODES> {
        fn default() -> Self {
            let num_nodes = NUM_NODES;

            // Generate keys for the nodes.
            let seed = [0; 32];
            let (pub_keys, priv_keys): (Vec<_>, Vec<_>) = (0..num_nodes)
                .map(|i| <PubKey as SignatureKey>::generated_from_seed_indexed(seed, i as u64))
                .unzip();
            let state_key_pairs = (0..num_nodes)
                .map(|i| StateKeyPair::generate_from_seed_indexed(seed, i as u64))
                .collect::<Vec<_>>();
            let known_nodes_with_stake = pub_keys
                .iter()
                .zip(&state_key_pairs)
                .map(|(pub_key, state_key_pair)| PeerConfig::<SeqTypes> {
                    stake_table_entry: pub_key.stake_table_entry(U256::from(1)),
                    state_ver_key: state_key_pair.ver_key(),
                })
                .collect::<Vec<_>>();

            let master_map = MasterMap::new();

            let config: HotShotConfig<SeqTypes> = HotShotConfig {
                fixed_leader_for_gpuvid: 0,
                num_nodes_with_stake: num_nodes.try_into().unwrap(),
                known_da_nodes: known_nodes_with_stake.clone(),
                known_nodes_with_stake: known_nodes_with_stake.clone(),
                next_view_timeout: Duration::from_secs(5).as_millis() as u64,
                num_bootstrap: 1usize,
                da_staked_committee_size: num_nodes,
                view_sync_timeout: Duration::from_secs(1),
                data_request_delay: Duration::from_secs(1),
                builder_urls: vec1::vec1![Url::parse(&format!(
                    "http://127.0.0.1:{}",
                    pick_unused_port().unwrap()
                ))
                .unwrap()],
                builder_timeout: Duration::from_secs(1),
                start_threshold: (
                    known_nodes_with_stake.clone().len() as u64,
                    known_nodes_with_stake.clone().len() as u64,
                ),
                start_proposing_view: 0,
                stop_proposing_view: 0,
                start_voting_view: 0,
                stop_voting_view: 0,
                start_proposing_time: 0,
                start_voting_time: 0,
                stop_proposing_time: 0,
                stop_voting_time: 0,
                epoch_height: 300,
                epoch_start_block: 0,
            };

            Self {
                config,
                priv_keys,
                state_key_pairs,
                master_map,
                l1_url: "http://localhost:8545".parse().unwrap(),
                state_relay_url: None,
                builder_port: None,
                marketplace_builder_port: None,
                upgrades: Default::default(),
            }
        }
    }

    #[derive(Clone)]
    pub struct TestConfig<const NUM_NODES: usize> {
        config: HotShotConfig<SeqTypes>,
        priv_keys: Vec<BLSPrivKey>,
        state_key_pairs: Vec<StateKeyPair>,
        master_map: Arc<MasterMap<PubKey>>,
        l1_url: Url,
        state_relay_url: Option<Url>,
        builder_port: Option<u16>,
        marketplace_builder_port: Option<u16>,
        upgrades: BTreeMap<Version, Upgrade>,
    }

    impl<const NUM_NODES: usize> TestConfig<NUM_NODES> {
        pub fn num_nodes(&self) -> usize {
            self.priv_keys.len()
        }

        pub fn hotshot_config(&self) -> &HotShotConfig<SeqTypes> {
            &self.config
        }

        pub fn set_builder_urls(&mut self, builder_urls: vec1::Vec1<Url>) {
            self.config.builder_urls = builder_urls;
        }

        pub fn marketplace_builder_port(&self) -> Option<u16> {
            self.marketplace_builder_port
        }

        pub fn builder_port(&self) -> Option<u16> {
            self.builder_port
        }

        pub fn l1_url(&self) -> Url {
            self.l1_url.clone()
        }

        pub fn upgrades(&self) -> BTreeMap<Version, Upgrade> {
            self.upgrades.clone()
        }

        pub async fn init_nodes<V: Versions>(
            &self,
            bind_version: V,
        ) -> Vec<SequencerContext<network::Memory, NoStorage, V>> {
            join_all((0..self.num_nodes()).map(|i| async move {
                self.init_node(
                    i,
                    ValidatedState::default(),
                    no_storage::Options,
                    NullStateCatchup::default(),
                    &NoMetrics,
                    STAKE_TABLE_CAPACITY_FOR_TEST,
                    NullEventConsumer,
                    bind_version,
                    Default::default(),
                    Url::parse(&format!(
                        "http://localhost:{}",
                        self.marketplace_builder_port.unwrap_or_default()
                    ))
                    .unwrap(),
                )
                .await
            }))
            .await
        }

        pub fn stake_table(&self) -> StakeTable<BLSPubKey, StateVerKey, CircuitField> {
            let mut st = StakeTable::<BLSPubKey, StateVerKey, CircuitField>::new(
                STAKE_TABLE_CAPACITY_FOR_TEST as usize,
            );
            self.config
                .known_nodes_with_stake
                .iter()
                .for_each(|config| {
                    st.register(
                        *config.stake_table_entry.key(),
                        config.stake_table_entry.stake(),
                        config.state_ver_key.clone(),
                    )
                    .unwrap()
                });
            st.advance();
            st.advance();
            st
        }

        #[allow(clippy::too_many_arguments)]
        pub async fn init_node<V: Versions, P: PersistenceOptions>(
            &self,
            i: usize,
            mut state: ValidatedState,
            mut persistence_opt: P,
            catchup: impl StateCatchup + 'static,
            metrics: &dyn Metrics,
            stake_table_capacity: u64,
            event_consumer: impl EventConsumer + 'static,
            bind_version: V,
            upgrades: BTreeMap<Version, Upgrade>,
            marketplace_builder_url: Url,
        ) -> SequencerContext<network::Memory, P::Persistence, V> {
            let config = self.config.clone();
            let my_peer_config = &config.known_nodes_with_stake[i];
            let is_da = config.known_da_nodes.contains(my_peer_config);

            // Create our own (private, local) validator config
            let validator_config = ValidatorConfig {
                public_key: my_peer_config.stake_table_entry.stake_key,
                private_key: self.priv_keys[i].clone(),
                stake_value: my_peer_config.stake_table_entry.stake_amount,
                state_public_key: self.state_key_pairs[i].ver_key(),
                state_private_key: self.state_key_pairs[i].sign_key(),
                is_da,
            };

            let topics = if is_da {
                vec![Topic::Global, Topic::Da]
            } else {
                vec![Topic::Global]
            };

            let network = Arc::new(MemoryNetwork::new(
                &my_peer_config.stake_table_entry.stake_key,
                &self.master_map,
                &topics,
                None,
            ));

            // Make sure the builder account is funded.
            let builder_account = Self::builder_key().fee_account();
            tracing::info!(%builder_account, "prefunding builder account");
            state.prefund_account(builder_account, U256::MAX.into());

            let persistence = persistence_opt.create().await.unwrap();

            let chain_config = state.chain_config.resolve().unwrap_or_default();
            let l1_client =
                L1Client::new(vec![self.l1_url.clone()]).expect("failed to create L1 client");
            let peers = catchup::local_and_remote(persistence.clone(), catchup).await;
            // Create the HotShot membership
            let membership = EpochCommittees::new_stake(
                config.known_nodes_with_stake.clone(),
                config.known_da_nodes.clone(),
                l1_client.clone(),
                chain_config,
                peers.clone(),
                persistence.clone(),
            );
            let membership = Arc::new(RwLock::new(membership));

            let coordinator = EpochMembershipCoordinator::new(membership, 100);

            let node_state = NodeState::new(
                i as u64,
                chain_config,
                l1_client,
                peers,
                V::Base::VERSION,
                coordinator.clone(),
            )
            .with_current_version(V::Base::version())
            .with_genesis(state)
            .with_epoch_height(config.epoch_height)
            .with_upgrades(upgrades);

            tracing::info!(
                i,
                key = %my_peer_config.stake_table_entry.stake_key,
                state_key = %my_peer_config.state_ver_key,
                "starting node",
            );

            let persistence = persistence_opt.create().await.unwrap();
            SequencerContext::init(
                NetworkConfig {
                    config,
                    // For testing, we use a fake network, so the rest of the network config beyond
                    // the base consensus config does not matter.
                    ..Default::default()
                },
                validator_config,
                coordinator,
                node_state,
                persistence,
                network,
                self.state_relay_url.clone(),
                metrics,
                stake_table_capacity,
                event_consumer,
                bind_version,
                MarketplaceConfig::<SeqTypes, Node<network::Memory, P::Persistence>> {
                    auction_results_provider: Arc::new(SolverAuctionResultsProvider::default()),
                    fallback_builder_url: marketplace_builder_url,
                },
                Default::default(),
            )
            .await
            .unwrap()
        }

        pub fn builder_key() -> EthKeyPair {
            FeeAccount::generated_from_seed_indexed([1; 32], 0).1
        }
    }

    // Wait for decide event, make sure it matches submitted transaction. Return the block number
    // containing the transaction.
    pub async fn wait_for_decide_on_handle(
        events: &mut (impl Stream<Item = Event> + Unpin),
        submitted_txn: &Transaction,
    ) -> u64 {
        let commitment = submitted_txn.commit();

        // Keep getting events until we see a Decide event
        loop {
            let event = events.next().await.unwrap();
            tracing::info!("Received event from handle: {event:?}");

            if let Decide { leaf_chain, .. } = event.event {
                if let Some(height) = leaf_chain.iter().find_map(|LeafInfo { leaf, .. }| {
                    if leaf
                        .block_payload()
                        .as_ref()?
                        .transaction_commitments(leaf.block_header().metadata())
                        .contains(&commitment)
                    {
                        Some(leaf.block_header().block_number())
                    } else {
                        None
                    }
                }) {
                    return height;
                }
            } else {
                // Keep waiting
            }
        }
    }
}

#[cfg(test)]
mod test {

    use alloy::node_bindings::Anvil;
    use espresso_types::{Header, MockSequencerVersions, NamespaceId, Payload, Transaction};
    use futures::StreamExt;
    use hotshot::types::EventType::Decide;
    use hotshot_example_types::node_types::TestVersions;
    use hotshot_types::{
        data::vid_commitment,
        event::LeafInfo,
        traits::block_contents::{
            BlockHeader, BlockPayload, EncodeBytes, GENESIS_VID_NUM_STORAGE_NODES,
        },
    };
    use sequencer_utils::test_utils::setup_test;
    use testing::{wait_for_decide_on_handle, TestConfigBuilder};

    use self::testing::run_test_builder;
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_skeleton_instantiation() {
        setup_test();
        // Assign `config` so it isn't dropped early.
        let anvil = Anvil::new().spawn();
        let url = anvil.endpoint_url();
        const NUM_NODES: usize = 5;
        let mut config = TestConfigBuilder::<NUM_NODES>::default()
            .l1_url(url)
            .build();

        let (builder_task, builder_url) = run_test_builder::<NUM_NODES>(None).await;

        config.set_builder_urls(vec1::vec1![builder_url]);

        let handles = config.init_nodes(MockSequencerVersions::new()).await;

        let handle_0 = &handles[0];

        // Hook the builder up to the event stream from the first node
        builder_task.start(Box::new(handle_0.event_stream().await));

        let mut events = handle_0.event_stream().await;

        for handle in handles.iter() {
            handle.start_consensus().await;
        }

        // Submit target transaction to handle
        let txn = Transaction::new(NamespaceId::from(1_u32), vec![1, 2, 3]);
        handles[0]
            .submit_transaction(txn.clone())
            .await
            .expect("Failed to submit transaction");
        tracing::info!("Submitted transaction to handle: {txn:?}");

        wait_for_decide_on_handle(&mut events, &txn).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_header_invariants() {
        setup_test();

        let success_height = 30;
        // Assign `config` so it isn't dropped early.
        let anvil = Anvil::new().spawn();
        let url = anvil.endpoint_url();
        const NUM_NODES: usize = 5;
        let mut config = TestConfigBuilder::<NUM_NODES>::default()
            .l1_url(url)
            .build();

        let (builder_task, builder_url) = run_test_builder::<NUM_NODES>(None).await;

        config.set_builder_urls(vec1::vec1![builder_url]);
        let handles = config.init_nodes(MockSequencerVersions::new()).await;

        let handle_0 = &handles[0];

        let mut events = handle_0.event_stream().await;

        // Hook the builder up to the event stream from the first node
        builder_task.start(Box::new(handle_0.event_stream().await));

        for handle in handles.iter() {
            handle.start_consensus().await;
        }

        let mut parent = {
            // TODO refactor repeated code from other tests
            let (genesis_payload, genesis_ns_table) =
                Payload::from_transactions([], &ValidatedState::default(), &NodeState::mock())
                    .await
                    .unwrap();
            let genesis_commitment = {
                // TODO we should not need to collect payload bytes just to compute vid_commitment
                let payload_bytes = genesis_payload.encode();
                vid_commitment::<TestVersions>(
                    &payload_bytes,
                    &genesis_ns_table.encode(),
                    GENESIS_VID_NUM_STORAGE_NODES,
                    <TestVersions as Versions>::Base::VERSION,
                )
            };
            let genesis_state = NodeState::mock();
            Header::genesis(
                &genesis_state,
                genesis_commitment,
                empty_builder_commitment(),
                genesis_ns_table,
            )
        };

        loop {
            let event = events.next().await.unwrap();
            tracing::info!("Received event from handle: {event:?}");
            let Decide { leaf_chain, .. } = event.event else {
                continue;
            };
            tracing::info!("Got decide {leaf_chain:?}");

            // Check that each successive header satisfies invariants relative to its parent: all
            // the fields which should be monotonic are.
            for LeafInfo { leaf, .. } in leaf_chain.iter().rev() {
                let header = leaf.block_header().clone();
                if header.height() == 0 {
                    parent = header;
                    continue;
                }
                assert_eq!(header.height(), parent.height() + 1);
                assert!(header.timestamp() >= parent.timestamp());
                assert!(header.l1_head() >= parent.l1_head());
                assert!(header.l1_finalized() >= parent.l1_finalized());
                parent = header;
            }

            if parent.height() >= success_height {
                break;
            }
        }
    }
}
