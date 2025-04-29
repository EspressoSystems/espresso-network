// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

#![allow(clippy::panic)]
use std::{
    collections::HashMap,
    fmt::Debug,
    fs,
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4},
    num::NonZeroUsize,
    sync::Arc,
    time::Instant,
};

use async_lock::RwLock;
use async_trait::async_trait;
use cdn_broker::reexports::crypto::signature::KeyPair;
use chrono::Utc;
use clap::{value_parser, Arg, Command, Parser};
use futures::StreamExt;
use hotshot::{
    traits::{
        implementations::{
            derive_libp2p_multiaddr, derive_libp2p_peer_id, CdnMetricsValue, CdnTopic,
            CombinedNetworks, Libp2pMetricsValue, Libp2pNetwork, PushCdnNetwork,
            WrappedSignatureKey,
        },
        BlockPayload, NodeImplementation,
    },
    types::SystemContextHandle,
    MarketplaceConfig, SystemContext,
};
use hotshot_example_types::{
    auction_results_provider_types::TestAuctionResultsProvider,
    block_types::{TestBlockHeader, TestBlockPayload, TestTransaction},
    node_types::{Libp2pImpl, PushCdnImpl},
    state_types::TestInstanceState,
    storage_types::TestStorage,
};
use hotshot_libp2p_networking::network::{
    behaviours::dht::store::persistent::DhtNoPersistence, GossipConfig, RequestResponseConfig,
};
use hotshot_orchestrator::{
    self,
    client::{get_complete_config, BenchResults, OrchestratorClient, ValidatorArgs},
};
use hotshot_testing::block_builder::{
    BuilderTask, RandomBuilderImplementation, SimpleBuilderImplementation,
    TestBuilderImplementation,
};
use hotshot_types::{
    consensus::ConsensusMetricsValue,
    data::{Leaf, TestableLeaf},
    epoch_membership::EpochMembershipCoordinator,
    event::{Event, EventType},
    network::{BuilderType, NetworkConfig, NetworkConfigFile, NetworkConfigSource},
    traits::{
        block_contents::{BlockHeader, TestableBlock},
        election::Membership,
        network::ConnectedNetwork,
        node_implementation::{ConsensusTime, NodeType, Versions},
        states::TestableState,
        storage::storage_add_drb_result,
    },
    utils::genesis_epoch_from_version,
    HotShotConfig, PeerConfig, ValidatorConfig,
};
use rand::{rngs::StdRng, SeedableRng};
use surf_disco::Url;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone)]
/// Arguments passed to the orchestrator
pub struct OrchestratorArgs<TYPES: NodeType> {
    /// The url the orchestrator runs on; this should be in the form of `http://localhost:5555` or `http://0.0.0.0:5555`
    pub url: Url,
    /// The configuration file to be used for this run
    pub config: NetworkConfig<TYPES>,
}

#[derive(Parser, Debug, Clone)]
#[command(
    name = "Multi-machine consensus",
    about = "Simulates consensus among multiple machines"
)]
/// The configuration file to be used for this run
pub struct ConfigArgs {
    /// The configuration file to be used for this run
    pub config_file: String,
}

impl Default for ConfigArgs {
    fn default() -> Self {
        Self {
            config_file: "./crates/orchestrator/run-config.toml".to_string(),
        }
    }
}

/// Reads the orchestrator initialization config from the command line
/// # Panics
/// If unable to read the config file from the command line
#[allow(clippy::too_many_lines)]
pub fn read_orchestrator_init_config<TYPES: NodeType>() -> (NetworkConfig<TYPES>, Url) {
    // assign default setting
    let mut orchestrator_url = Url::parse("http://localhost:4444").unwrap();
    let mut args = ConfigArgs::default();
    // start reading from the command line
    let matches = Command::new("orchestrator")
        .arg(
            Arg::new("config_file")
                .short('c')
                .long("config_file")
                .value_name("FILE")
                .help("Sets a custom config file with default values, some might be changed if they are set manually in the command line")
                .required(true),
        )
        .arg(
            Arg::new("total_nodes")
                .short('n')
                .long("total_nodes")
                .value_name("NUM")
                .help("Sets the total number of nodes")
                .required(false),
        )
        .arg(
            Arg::new("da_committee_size")
                .short('d')
                .long("da_committee_size")
                .value_name("NUM")
                .help("Sets the size of the data availability committee")
                .required(false),
        )
        .arg(
            Arg::new("transactions_per_round")
                .short('t')
                .long("transactions_per_round")
                .value_name("NUM")
                .help("Sets the number of transactions per round")
                .required(false),
        )
        .arg(
            Arg::new("transaction_size")
                .short('s')
                .long("transaction_size")
                .value_name("NUM")
                .help("Sets the size of each transaction in bytes")
                .required(false),
        )
        .arg(
            Arg::new("rounds")
                .short('r')
                .long("rounds")
                .value_name("NUM")
                .help("Sets the number of rounds to run")
                .required(false),
        )
        .arg(
            Arg::new("commit_sha")
                .short('o')
                .long("commit_sha")
                .value_name("SHA")
                .help("Sets the commit sha to output in the results")
                .required(false),
        )
        .arg(
            Arg::new("orchestrator_url")
                .short('u')
                .long("orchestrator_url")
                .value_name("URL")
                .help("Sets the url of the orchestrator")
                .required(false),
        )
        .arg(
            Arg::new("fixed_leader_for_gpuvid")
                .short('f')
                .long("fixed_leader_for_gpuvid")
                .value_name("BOOL")
                .help("Sets the number of fixed leader for gpu vid, only be used when leaders running on gpu")
                .required(false),
        )
        .arg(
            Arg::new("builder")
                .short('b')
                .long("builder")
                .value_name("BUILDER_TYPE")
                .value_parser(value_parser!(BuilderType))
                .help("Sets type of builder. `simple` or `random` to run corresponding integrated builder, `external` to use the one specified by `[config.builder_url]` in config")
                .required(false),
        )
        .arg(
            Arg::new("cdn_marshal_address")
                .short('m')
                .long("cdn_marshal_address")
                .value_name("URL")
                .help("Sets the url for cdn_broker_marshal_endpoint")
                .required(false),
        )
        .get_matches();

    if let Some(config_file_string) = matches.get_one::<String>("config_file") {
        args = ConfigArgs {
            config_file: config_file_string.clone(),
        };
    } else {
        error!("No config file provided, we'll use the default one.");
    }
    let mut config: NetworkConfig<TYPES> = load_config_from_file::<TYPES>(&args.config_file);

    if let Some(total_nodes_string) = matches.get_one::<String>("total_nodes") {
        config.config.num_nodes_with_stake = total_nodes_string.parse::<NonZeroUsize>().unwrap();
        config.config.known_nodes_with_stake =
            vec![PeerConfig::default(); config.config.num_nodes_with_stake.get() as usize];
        error!(
            "config.config.total_nodes: {:?}",
            config.config.num_nodes_with_stake
        );
    }
    if let Some(da_committee_size_string) = matches.get_one::<String>("da_committee_size") {
        config.config.da_staked_committee_size = da_committee_size_string.parse::<usize>().unwrap();
    }
    if let Some(fixed_leader_for_gpuvid_string) =
        matches.get_one::<String>("fixed_leader_for_gpuvid")
    {
        config.config.fixed_leader_for_gpuvid =
            fixed_leader_for_gpuvid_string.parse::<usize>().unwrap();
    }
    if let Some(transactions_per_round_string) = matches.get_one::<String>("transactions_per_round")
    {
        config.transactions_per_round = transactions_per_round_string.parse::<usize>().unwrap();
    }
    if let Some(transaction_size_string) = matches.get_one::<String>("transaction_size") {
        config.transaction_size = transaction_size_string.parse::<usize>().unwrap();
    }
    if let Some(rounds_string) = matches.get_one::<String>("rounds") {
        config.rounds = rounds_string.parse::<usize>().unwrap();
    }
    if let Some(commit_sha_string) = matches.get_one::<String>("commit_sha") {
        config.commit_sha = commit_sha_string.to_string();
    }
    if let Some(orchestrator_url_string) = matches.get_one::<String>("orchestrator_url") {
        orchestrator_url = Url::parse(orchestrator_url_string).unwrap();
    }
    if let Some(builder_type) = matches.get_one::<BuilderType>("builder") {
        config.builder = *builder_type;
    }
    if let Some(cdn_marshal_address_string) = matches.get_one::<String>("cdn_marshal_address") {
        config.cdn_marshal_address = Some(cdn_marshal_address_string.to_string());
    }

    (config, orchestrator_url)
}

/// Reads a network configuration from a given filepath
/// # Panics
/// if unable to convert the config file into toml
/// # Note
/// This derived config is used for initialization of orchestrator,
/// therefore `known_nodes_with_stake` will be an initialized
/// vector full of the node's own config.
#[must_use]
pub fn load_config_from_file<TYPES: NodeType>(config_file: &str) -> NetworkConfig<TYPES> {
    let config_file_as_string: String = fs::read_to_string(config_file)
        .unwrap_or_else(|_| panic!("Could not read config file located at {config_file}"));
    let config_toml: NetworkConfigFile<TYPES> =
        toml::from_str::<NetworkConfigFile<TYPES>>(&config_file_as_string)
            .expect("Unable to convert config file to TOML");

    let mut config: NetworkConfig<TYPES> = config_toml.into();

    // initialize it with size for better assignment of peers' config
    config.config.known_nodes_with_stake =
        vec![PeerConfig::default(); config.config.num_nodes_with_stake.get() as usize];

    config
}

/// Runs the orchestrator
pub async fn run_orchestrator<TYPES: NodeType>(
    OrchestratorArgs { url, config }: OrchestratorArgs<TYPES>,
) {
    println!("Starting orchestrator",);
    let _ = hotshot_orchestrator::run_orchestrator::<TYPES>(config, url).await;
}

/// Helper function to calculate the number of transactions to send per node per round
#[allow(clippy::cast_possible_truncation)]
fn calculate_num_tx_per_round(
    node_index: u64,
    total_num_nodes: usize,
    transactions_per_round: usize,
) -> usize {
    transactions_per_round / total_num_nodes
        + usize::from(
            (total_num_nodes)
                < (transactions_per_round % total_num_nodes) + 1 + (node_index as usize),
        )
}

/// Helper function to generate transactions a given node should send
fn generate_transactions<TYPES: NodeType<Transaction = TestTransaction>>(
    node_index: u64,
    rounds: usize,
    transactions_to_send_per_round: usize,
    transaction_size: usize,
) -> Vec<TestTransaction>
where
    <TYPES as NodeType>::ValidatedState: TestableState<TYPES>,
    <TYPES as NodeType>::BlockPayload: TestableBlock<TYPES>,
{
    let mut txn_rng = StdRng::seed_from_u64(node_index);
    let mut transactions = Vec::new();

    for _ in 0..rounds {
        for _ in 0..transactions_to_send_per_round {
            let txn = <TYPES::ValidatedState>::create_random_transaction(
                None,
                &mut txn_rng,
                transaction_size as u64,
            );

            transactions.push(txn);
        }
    }
    transactions
}

/// Defines the behavior of a "run" of the network with a given configuration
#[async_trait]
pub trait RunDa<
    TYPES: NodeType<InstanceState = TestInstanceState>,
    NETWORK: ConnectedNetwork<TYPES::SignatureKey>,
    NODE: NodeImplementation<
        TYPES,
        Network = NETWORK,
        Storage = TestStorage<TYPES>,
        AuctionResultsProvider = TestAuctionResultsProvider<TYPES>,
    >,
    V: Versions,
> where
    <TYPES as NodeType>::ValidatedState: TestableState<TYPES>,
    <TYPES as NodeType>::BlockPayload: TestableBlock<TYPES>,
    TYPES: NodeType<Transaction = TestTransaction>,
    Leaf<TYPES>: TestableLeaf,
    Self: Sync,
{
    /// Initializes networking, returns self
    async fn initialize_networking(
        config: NetworkConfig<TYPES>,
        validator_config: ValidatorConfig<TYPES>,
        libp2p_advertise_address: Option<String>,
        membership: &Arc<RwLock<<TYPES as NodeType>::Membership>>,
    ) -> Self;

    /// Initializes the genesis state and HotShot instance; does not start HotShot consensus
    /// # Panics if it cannot generate a genesis block, fails to initialize HotShot, or cannot
    /// get the anchored view
    /// Note: sequencing leaf does not have state, so does not return state
    async fn initialize_state_and_hotshot(
        &self,
        membership: Arc<RwLock<<TYPES as NodeType>::Membership>>,
    ) -> SystemContextHandle<TYPES, NODE, V> {
        let initializer = hotshot::HotShotInitializer::<TYPES>::from_genesis::<V>(
            TestInstanceState::default(),
            self.config().config.epoch_height,
            self.config().config.epoch_start_block,
            vec![],
        )
        .await
        .expect("Couldn't generate genesis block");

        let config = self.config();
        let validator_config = self.validator_config();

        // Get KeyPair for certificate Aggregation
        let pk = validator_config.public_key.clone();
        let sk = validator_config.private_key.clone();
        let state_sk = validator_config.state_private_key.clone();

        let network = self.network();

        let marketplace_config = MarketplaceConfig {
            auction_results_provider: TestAuctionResultsProvider::<TYPES>::default().into(),
            // TODO: we need to pass a valid fallback builder url here somehow
            fallback_builder_url: config.config.builder_urls.first().clone(),
        };
        let epoch_height = config.config.epoch_height;

        let storage = TestStorage::<TYPES>::default();

        SystemContext::init(
            pk,
            sk,
            state_sk,
            config.node_index,
            config.config,
            EpochMembershipCoordinator::new(
                membership,
                Some(storage_add_drb_result(storage.clone())),
                epoch_height,
            ),
            Arc::from(network),
            initializer,
            ConsensusMetricsValue::default(),
            storage,
            marketplace_config,
        )
        .await
        .expect("Could not init hotshot")
        .0
    }

    /// Starts HotShot consensus, returns when consensus has finished
    #[allow(clippy::too_many_lines)]
    async fn run_hotshot(
        &self,
        context: SystemContextHandle<TYPES, NODE, V>,
        transactions: &mut Vec<TestTransaction>,
        transactions_to_send_per_round: u64,
        transaction_size_in_bytes: u64,
    ) -> BenchResults {
        let NetworkConfig {
            rounds, node_index, ..
        } = self.config();

        let mut total_transactions_committed = 0;
        let mut total_transactions_sent = 0;
        let mut minimum_latency = 1000;
        let mut maximum_latency = 0;
        let mut total_latency = 0;
        let mut num_latency = 0;

        info!("Starting HotShot example!");
        let start = Instant::now();

        let mut event_stream = context.event_stream();
        let mut anchor_view: TYPES::View = <TYPES::View as ConsensusTime>::genesis();
        let mut num_successful_commits = 0;

        context.hotshot.start_consensus().await;

        loop {
            match event_stream.next().await {
                None => {
                    panic!("Error! Event stream completed before consensus ended.");
                },
                Some(Event { event, .. }) => {
                    match event {
                        EventType::Error { error } => {
                            error!("Error in consensus: {:?}", error);
                            // TODO what to do here
                        },
                        EventType::Decide {
                            leaf_chain,
                            qc: _,
                            block_size,
                        } => {
                            let current_timestamp = Utc::now().timestamp();
                            // this might be a obob
                            if let Some(leaf_info) = leaf_chain.first() {
                                let leaf = &leaf_info.leaf;
                                info!("Decide event for leaf: {}", *leaf.view_number());

                                // iterate all the decided transactions to calculate latency
                                if let Some(block_payload) = &leaf.block_payload() {
                                    for tx in
                                        block_payload.transactions(leaf.block_header().metadata())
                                    {
                                        let restored_timestamp_vec =
                                            tx.bytes()[tx.bytes().len() - 8..].to_vec();
                                        let restored_timestamp = i64::from_be_bytes(
                                            restored_timestamp_vec.as_slice().try_into().unwrap(),
                                        );
                                        let cur_latency = current_timestamp - restored_timestamp;
                                        total_latency += cur_latency;
                                        num_latency += 1;
                                        minimum_latency =
                                            std::cmp::min(minimum_latency, cur_latency);
                                        maximum_latency =
                                            std::cmp::max(maximum_latency, cur_latency);
                                    }
                                }

                                let new_anchor = leaf.view_number();
                                if new_anchor >= anchor_view {
                                    anchor_view = leaf.view_number();
                                }

                                // send transactions
                                for _ in 0..transactions_to_send_per_round {
                                    // append current timestamp to the tx to calc latency
                                    let timestamp = Utc::now().timestamp();
                                    let mut tx = transactions.remove(0).into_bytes();
                                    let mut timestamp_vec = timestamp.to_be_bytes().to_vec();
                                    tx.append(&mut timestamp_vec);

                                    () = context
                                        .submit_transaction(TestTransaction::new(tx))
                                        .await
                                        .unwrap();
                                    total_transactions_sent += 1;
                                }
                            }

                            if let Some(size) = block_size {
                                total_transactions_committed += size;
                                debug!("[{node_index}] got block with size: {:?}", size);
                            }

                            num_successful_commits += leaf_chain.len();
                            if num_successful_commits >= rounds {
                                break;
                            }

                            if leaf_chain.len() > 1 {
                                warn!("Leaf chain is greater than 1 with len {}", leaf_chain.len());
                            }
                            // when we make progress, submit new events
                        },
                        EventType::ReplicaViewTimeout { view_number } => {
                            warn!("Timed out as a replicas in view {:?}", view_number);
                        },
                        EventType::ViewTimeout { view_number } => {
                            warn!("Timed out in view {:?}", view_number);
                        },
                        _ => {}, // mostly DA proposal
                    }
                },
            }
        }
        // Panic if we don't have the genesis epoch, there is no recovery from that
        let num_eligible_leaders = context
            .hotshot
            .membership_coordinator
            .membership_for_epoch(genesis_epoch_from_version::<V, TYPES>())
            .await
            .unwrap()
            .stake_table()
            .await
            .len();
        let consensus_lock = context.hotshot.consensus();
        let consensus_reader = consensus_lock.read().await;
        let total_num_views = usize::try_from(consensus_reader.locked_view().u64()).unwrap();
        drop(consensus_reader);
        // `failed_num_views` could include uncommitted views
        let failed_num_views = total_num_views - num_successful_commits;
        // When posting to the orchestrator, note that the total number of views also include un-finalized views.
        println!("[{node_index}]: Total views: {total_num_views}, Failed views: {failed_num_views}, num_successful_commits: {num_successful_commits}");
        // Output run results
        let total_time_elapsed = start.elapsed(); // in seconds
        println!("[{node_index}]: {rounds} rounds completed in {total_time_elapsed:?} - Total transactions sent: {total_transactions_sent} - Total transactions committed: {total_transactions_committed} - Total commitments: {num_successful_commits}");
        if total_transactions_committed != 0 {
            // prevent division by 0
            let total_time_elapsed_sec = std::cmp::max(total_time_elapsed.as_secs(), 1u64);
            // extra 8 bytes for timestamp
            let throughput_bytes_per_sec = total_transactions_committed
                * (transaction_size_in_bytes + 8)
                / total_time_elapsed_sec;
            let avg_latency_in_sec = total_latency / num_latency;
            println!("[{node_index}]: throughput: {throughput_bytes_per_sec} bytes/sec, avg_latency: {avg_latency_in_sec} sec.");

            BenchResults {
                partial_results: "Unset".to_string(),
                avg_latency_in_sec,
                num_latency,
                minimum_latency_in_sec: minimum_latency,
                maximum_latency_in_sec: maximum_latency,
                throughput_bytes_per_sec,
                total_transactions_committed,
                transaction_size_in_bytes: transaction_size_in_bytes + 8, // extra 8 bytes for timestamp
                total_time_elapsed_in_sec: total_time_elapsed.as_secs(),
                total_num_views,
                failed_num_views,
                committee_type: format!(
                    "{} with {num_eligible_leaders} eligible leaders",
                    std::any::type_name::<TYPES::Membership>()
                ),
            }
        } else {
            // all values with zero
            BenchResults::default()
        }
    }

    /// Returns the underlying network for this run
    fn network(&self) -> NETWORK;

    /// Returns the config for this run
    fn config(&self) -> NetworkConfig<TYPES>;

    /// Returns the validator config with private signature keys for this run.
    fn validator_config(&self) -> ValidatorConfig<TYPES>;
}

// Push CDN

/// Represents a Push CDN-based run
pub struct PushCdnDaRun<TYPES: NodeType> {
    /// The underlying configuration
    config: NetworkConfig<TYPES>,
    /// The private validator config
    validator_config: ValidatorConfig<TYPES>,
    /// The underlying network
    network: PushCdnNetwork<TYPES::SignatureKey>,
}

#[async_trait]
impl<
        TYPES: NodeType<
            Transaction = TestTransaction,
            BlockPayload = TestBlockPayload,
            BlockHeader = TestBlockHeader,
            InstanceState = TestInstanceState,
        >,
        NODE: NodeImplementation<
            TYPES,
            Network = PushCdnNetwork<TYPES::SignatureKey>,
            Storage = TestStorage<TYPES>,
            AuctionResultsProvider = TestAuctionResultsProvider<TYPES>,
        >,
        V: Versions,
    > RunDa<TYPES, PushCdnNetwork<TYPES::SignatureKey>, NODE, V> for PushCdnDaRun<TYPES>
where
    <TYPES as NodeType>::ValidatedState: TestableState<TYPES>,
    <TYPES as NodeType>::BlockPayload: TestableBlock<TYPES>,
    Leaf<TYPES>: TestableLeaf,
    Self: Sync,
{
    async fn initialize_networking(
        config: NetworkConfig<TYPES>,
        validator_config: ValidatorConfig<TYPES>,
        _libp2p_advertise_address: Option<String>,
        _membership: &Arc<RwLock<<TYPES as NodeType>::Membership>>,
    ) -> PushCdnDaRun<TYPES> {
        // Convert to the Push-CDN-compatible type
        let keypair = KeyPair {
            public_key: WrappedSignatureKey(validator_config.public_key.clone()),
            private_key: validator_config.private_key.clone(),
        };

        // See if we should be DA, subscribe to the DA topic if so
        let mut topics = vec![CdnTopic::Global];
        if validator_config.is_da {
            topics.push(CdnTopic::Da);
        }

        // Create the network and await the initial connection
        let network = PushCdnNetwork::new(
            config
                .cdn_marshal_address
                .clone()
                .expect("`cdn_marshal_address` needs to be supplied for a push CDN run"),
            topics,
            keypair,
            CdnMetricsValue::default(),
        )
        .expect("failed to create network");

        // Wait for the network to be ready
        network.wait_for_ready().await;

        PushCdnDaRun {
            config,
            validator_config,
            network,
        }
    }

    fn network(&self) -> PushCdnNetwork<TYPES::SignatureKey> {
        self.network.clone()
    }

    fn config(&self) -> NetworkConfig<TYPES> {
        self.config.clone()
    }

    fn validator_config(&self) -> ValidatorConfig<TYPES> {
        self.validator_config.clone()
    }
}

// Libp2p

/// Represents a libp2p-based run
pub struct Libp2pDaRun<TYPES: NodeType> {
    /// The underlying network configuration
    config: NetworkConfig<TYPES>,
    /// The private validator config
    validator_config: ValidatorConfig<TYPES>,
    /// The underlying network
    network: Libp2pNetwork<TYPES>,
}

#[async_trait]
impl<
        TYPES: NodeType<
            Transaction = TestTransaction,
            BlockPayload = TestBlockPayload,
            BlockHeader = TestBlockHeader,
            InstanceState = TestInstanceState,
        >,
        NODE: NodeImplementation<
            TYPES,
            Network = Libp2pNetwork<TYPES>,
            Storage = TestStorage<TYPES>,
            AuctionResultsProvider = TestAuctionResultsProvider<TYPES>,
        >,
        V: Versions,
    > RunDa<TYPES, Libp2pNetwork<TYPES>, NODE, V> for Libp2pDaRun<TYPES>
where
    <TYPES as NodeType>::ValidatedState: TestableState<TYPES>,
    <TYPES as NodeType>::BlockPayload: TestableBlock<TYPES>,
    Leaf<TYPES>: TestableLeaf,
    Self: Sync,
{
    async fn initialize_networking(
        config: NetworkConfig<TYPES>,
        validator_config: ValidatorConfig<TYPES>,
        libp2p_advertise_address: Option<String>,
        membership: &Arc<RwLock<<TYPES as NodeType>::Membership>>,
    ) -> Libp2pDaRun<TYPES> {
        // Extrapolate keys for ease of use
        let public_key = &validator_config.public_key;
        let private_key = &validator_config.private_key;

        // In an example, we can calculate the libp2p bind address as a function
        // of the advertise address.
        let bind_address = if let Some(libp2p_advertise_address) = libp2p_advertise_address {
            let libp2p_advertise_address: SocketAddrV4 = libp2p_advertise_address
                .parse()
                .expect("failed to parse advertise address");

            // If we have supplied one, use it
            SocketAddr::new(
                IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                libp2p_advertise_address.port(),
            )
            .to_string()
        } else {
            // If not, index a base port with our node index
            SocketAddr::new(
                IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                8000 + (u16::try_from(config.node_index)
                    .expect("failed to create advertise address")),
            )
            .to_string()
        };

        // Derive the bind address
        let bind_address =
            derive_libp2p_multiaddr(&bind_address).expect("failed to derive bind address");

        // Create the Libp2p network
        let libp2p_network = Libp2pNetwork::from_config(
            config.clone(),
            DhtNoPersistence,
            Arc::clone(membership),
            GossipConfig::default(),
            RequestResponseConfig::default(),
            bind_address,
            public_key,
            private_key,
            Libp2pMetricsValue::default(),
        )
        .await
        .expect("failed to create libp2p network");

        // Wait for the network to be ready
        libp2p_network.wait_for_ready().await;

        Libp2pDaRun {
            config,
            validator_config,
            network: libp2p_network,
        }
    }

    fn network(&self) -> Libp2pNetwork<TYPES> {
        self.network.clone()
    }

    fn config(&self) -> NetworkConfig<TYPES> {
        self.config.clone()
    }

    fn validator_config(&self) -> ValidatorConfig<TYPES> {
        self.validator_config.clone()
    }
}

// Combined network

/// Represents a combined-network-based run
pub struct CombinedDaRun<TYPES: NodeType> {
    /// The underlying network configuration
    config: NetworkConfig<TYPES>,
    /// The private validator config
    validator_config: ValidatorConfig<TYPES>,
    /// The underlying network
    network: CombinedNetworks<TYPES>,
}

#[async_trait]
impl<
        TYPES: NodeType<
            Transaction = TestTransaction,
            BlockPayload = TestBlockPayload,
            BlockHeader = TestBlockHeader,
            InstanceState = TestInstanceState,
        >,
        NODE: NodeImplementation<
            TYPES,
            Network = CombinedNetworks<TYPES>,
            Storage = TestStorage<TYPES>,
            AuctionResultsProvider = TestAuctionResultsProvider<TYPES>,
        >,
        V: Versions,
    > RunDa<TYPES, CombinedNetworks<TYPES>, NODE, V> for CombinedDaRun<TYPES>
where
    <TYPES as NodeType>::ValidatedState: TestableState<TYPES>,
    <TYPES as NodeType>::BlockPayload: TestableBlock<TYPES>,
    Leaf<TYPES>: TestableLeaf,
    Self: Sync,
{
    async fn initialize_networking(
        config: NetworkConfig<TYPES>,
        validator_config: ValidatorConfig<TYPES>,
        libp2p_advertise_address: Option<String>,
        membership: &Arc<RwLock<<TYPES as NodeType>::Membership>>,
    ) -> CombinedDaRun<TYPES> {
        // Initialize our Libp2p network
        let libp2p_network: Libp2pDaRun<TYPES> = <Libp2pDaRun<TYPES> as RunDa<
            TYPES,
            Libp2pNetwork<TYPES>,
            Libp2pImpl,
            V,
        >>::initialize_networking(
            config.clone(),
            validator_config.clone(),
            libp2p_advertise_address.clone(),
            membership,
        )
        .await;

        // Initialize our CDN network
        let cdn_network: PushCdnDaRun<TYPES> = <PushCdnDaRun<TYPES> as RunDa<
            TYPES,
            PushCdnNetwork<TYPES::SignatureKey>,
            PushCdnImpl,
            V,
        >>::initialize_networking(
            config.clone(),
            validator_config.clone(),
            libp2p_advertise_address,
            membership,
        )
        .await;

        // Create our combined network config
        let delay_duration = config
            .combined_network_config
            .as_ref()
            .map(|config| config.delay_duration);

        // Create our combined network
        let network =
            CombinedNetworks::new(cdn_network.network, libp2p_network.network, delay_duration);

        // Return the run configuration
        CombinedDaRun {
            config,
            validator_config,
            network,
        }
    }

    fn network(&self) -> CombinedNetworks<TYPES> {
        self.network.clone()
    }

    fn config(&self) -> NetworkConfig<TYPES> {
        self.config.clone()
    }

    fn validator_config(&self) -> ValidatorConfig<TYPES> {
        self.validator_config.clone()
    }
}

#[allow(clippy::too_many_lines)]
/// Main entry point for validators
/// # Panics
/// if unable to get the local ip address
pub async fn main_entry_point<
    TYPES: NodeType<
        Transaction = TestTransaction,
        BlockHeader = TestBlockHeader,
        InstanceState = TestInstanceState,
    >,
    NETWORK: ConnectedNetwork<TYPES::SignatureKey>,
    NODE: NodeImplementation<
        TYPES,
        Network = NETWORK,
        Storage = TestStorage<TYPES>,
        AuctionResultsProvider = TestAuctionResultsProvider<TYPES>,
    >,
    V: Versions,
    RUNDA: RunDa<TYPES, NETWORK, NODE, V>,
>(
    args: ValidatorArgs,
) where
    <TYPES as NodeType>::ValidatedState: TestableState<TYPES>,
    <TYPES as NodeType>::BlockPayload: TestableBlock<TYPES>,
    Leaf<TYPES>: TestableLeaf,
{
    // Initialize logging
    hotshot::helpers::initialize_logging();

    info!("Starting validator");

    let orchestrator_client: OrchestratorClient = OrchestratorClient::new(args.url.clone());

    // We assume one node will not call this twice to generate two validator_config-s with same identity.
    let validator_config = NetworkConfig::<TYPES>::generate_init_validator_config(
        orchestrator_client
            .get_node_index_for_init_validator_config()
            .await,
        // we assign nodes to the DA committee by default
        true,
    );

    // Derives our Libp2p private key from our private key, and then returns the public key of that key
    let libp2p_public_key =
        derive_libp2p_peer_id::<TYPES::SignatureKey>(&validator_config.private_key)
            .expect("failed to derive Libp2p keypair");

    // We need this to be able to register our node
    let peer_config = PeerConfig::<TYPES>::to_bytes(&validator_config.public_config()).clone();

    // Derive the advertise multiaddress from the supplied string
    let advertise_multiaddress = args.advertise_address.clone().map(|advertise_address| {
        derive_libp2p_multiaddr(&advertise_address).expect("failed to derive Libp2p multiaddr")
    });

    // conditionally save/load config from file or orchestrator
    // This is a function that will return correct complete config from orchestrator.
    // It takes in a valid args.network_config_file when loading from file, or valid validator_config when loading from orchestrator, the invalid one will be ignored.
    // It returns the complete config which also includes peer's public key and public config.
    // This function will be taken solely by sequencer right after OrchestratorClient::new,
    // which means the previous `generate_validator_config_when_init` will not be taken by sequencer, it's only for key pair generation for testing in hotshot.

    let (mut run_config, validator_config, source) = get_complete_config(
        &orchestrator_client,
        validator_config,
        advertise_multiaddress,
        Some(libp2p_public_key),
    )
    .await
    .expect("failed to get config");

    let builder_task = initialize_builder(
        &mut run_config,
        &validator_config,
        &args,
        &orchestrator_client,
    )
    .await;

    run_config.config.builder_urls = orchestrator_client
        .get_builder_addresses()
        .await
        .try_into()
        .expect("Orchestrator didn't provide any builder addresses");

    debug!(
        "Assigned urls from orchestrator: {}",
        run_config
            .config
            .builder_urls
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<String>>()
            .join(",")
    );

    let all_nodes = if cfg!(feature = "fixed-leader-election") {
        let mut vec = run_config.config.known_nodes_with_stake.clone();
        vec.truncate(run_config.config.fixed_leader_for_gpuvid);
        vec
    } else {
        run_config.config.known_nodes_with_stake.clone()
    };
    let membership = Arc::new(RwLock::new(<TYPES as NodeType>::Membership::new(
        all_nodes,
        run_config.config.known_da_nodes.clone(),
    )));

    info!("Initializing networking");
    let run = RUNDA::initialize_networking(
        run_config.clone(),
        validator_config,
        args.advertise_address,
        &membership,
    )
    .await;
    let hotshot = run.initialize_state_and_hotshot(membership).await;

    if let Some(task) = builder_task {
        task.start(Box::new(hotshot.event_stream()));
    }

    // pre-generate transactions
    let NetworkConfig {
        transaction_size,
        rounds,
        transactions_per_round,
        node_index,
        config: HotShotConfig {
            num_nodes_with_stake,
            ..
        },
        ..
    } = run_config;

    let transactions_to_send_per_round = calculate_num_tx_per_round(
        node_index,
        num_nodes_with_stake.get(),
        transactions_per_round,
    );
    let mut transactions: Vec<TestTransaction> = generate_transactions::<TYPES>(
        node_index,
        rounds,
        transactions_to_send_per_round,
        transaction_size,
    );

    if let NetworkConfigSource::Orchestrator = source {
        info!("Waiting for the start command from orchestrator");
        orchestrator_client
            .wait_for_all_nodes_ready(peer_config)
            .await;
    }

    info!("Starting HotShot");
    let bench_results = run
        .run_hotshot(
            hotshot,
            &mut transactions,
            transactions_to_send_per_round as u64,
            (transaction_size + 8) as u64, // extra 8 bytes for transaction base, see `create_random_transaction`.
        )
        .await;
    orchestrator_client.post_bench_results(bench_results).await;
}

/// Sets correct builder_url and registers a builder with orchestrator if this node is running one.
/// Returns a `BuilderTask` if this node is going to be running a builder.
async fn initialize_builder<
    TYPES: NodeType<
        Transaction = TestTransaction,
        BlockHeader = TestBlockHeader,
        InstanceState = TestInstanceState,
    >,
>(
    run_config: &mut NetworkConfig<TYPES>,
    validator_config: &ValidatorConfig<TYPES>,
    args: &ValidatorArgs,
    orchestrator_client: &OrchestratorClient,
) -> Option<Box<dyn BuilderTask<TYPES>>>
where
    <TYPES as NodeType>::ValidatedState: TestableState<TYPES>,
    <TYPES as NodeType>::BlockPayload: TestableBlock<TYPES>,
    Leaf<TYPES>: TestableLeaf,
{
    if !validator_config.is_da {
        return None;
    }

    let advertise_urls: Vec<Url>;
    let bind_address: Url;

    match args.builder_address {
        None => {
            let port = portpicker::pick_unused_port().expect("Failed to pick an unused port");
            advertise_urls = local_ip_address::list_afinet_netifas()
                .expect("Couldn't get list of local IP addresses")
                .into_iter()
                .map(|(_name, ip)| ip)
                .filter(|ip| !ip.is_loopback())
                .map(|ip| match ip {
                    IpAddr::V4(addr) => Url::parse(&format!("http://{addr}:{port}")).unwrap(),
                    IpAddr::V6(addr) => Url::parse(&format!("http://[{addr}]:{port}")).unwrap(),
                })
                .collect();
            bind_address = Url::parse(&format!("http://0.0.0.0:{port}")).unwrap();
        },
        Some(ref addr) => {
            bind_address = Url::parse(&format!("http://{addr}")).expect("Valid URL");
            advertise_urls = vec![bind_address.clone()];
        },
    }

    match run_config.builder {
        BuilderType::External => None,
        BuilderType::Random => {
            let builder_task =
                <RandomBuilderImplementation as TestBuilderImplementation<TYPES>>::start(
                    run_config.config.num_nodes_with_stake.into(),
                    bind_address,
                    run_config.random_builder.clone().unwrap_or_default(),
                    HashMap::new(),
                )
                .await;

            orchestrator_client
                .post_builder_addresses(advertise_urls)
                .await;

            Some(builder_task)
        },
        BuilderType::Simple => {
            let builder_task =
                <SimpleBuilderImplementation as TestBuilderImplementation<TYPES>>::start(
                    run_config.config.num_nodes_with_stake.into(),
                    bind_address,
                    (),
                    HashMap::new(),
                )
                .await;

            orchestrator_client
                .post_builder_addresses(advertise_urls)
                .await;

            Some(builder_task)
        },
    }
}

/// Base port for validator
pub const VALIDATOR_BASE_PORT: u16 = 8000;
/// Base port for builder
pub const BUILDER_BASE_PORT: u16 = 9000;

/// Generate a local address for node with index `node_index`, offsetting from port `BASE_PORT`.
/// # Panics
/// If `node_index` is too large to fit in a `u16`
#[must_use]
pub fn gen_local_address<const BASE_PORT: u16>(node_index: usize) -> SocketAddr {
    SocketAddr::new(
        IpAddr::V4(Ipv4Addr::LOCALHOST),
        BASE_PORT + (u16::try_from(node_index).expect("node index too large")),
    )
}
