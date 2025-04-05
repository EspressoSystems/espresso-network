// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Orchestrator for manipulating nodes and recording results during a run of `HotShot` tests

/// The orchestrator's clients
pub mod client;

use std::{
    collections::{HashMap, HashSet},
    fs,
    fs::OpenOptions,
    io::{self, ErrorKind},
    time::Duration,
};

use alloy::primitives::U256;
use async_lock::RwLock;
use client::{BenchResults, BenchResultsDownloadConfig};
use csv::Writer;
use futures::{stream::FuturesUnordered, FutureExt, StreamExt};
use hotshot_types::{
    network::{BuilderType, NetworkConfig, PublicKeysFile},
    traits::{
        node_implementation::NodeType,
        signature_key::{SignatureKey, StakeTableEntryType},
    },
    PeerConfig,
};
use libp2p_identity::{
    ed25519::{Keypair as EdKeypair, SecretKey},
    Keypair, PeerId,
};
use multiaddr::Multiaddr;
use surf_disco::Url;
use tide_disco::{
    api::ApiError,
    error::ServerError,
    method::{ReadState, WriteState},
    Api, App, RequestError,
};
use vbs::{
    version::{StaticVersion, StaticVersionType},
    BinarySerializer,
};

/// Orchestrator is not, strictly speaking, bound to the network; it can have its own versioning.
/// Orchestrator Version (major)
pub const ORCHESTRATOR_MAJOR_VERSION: u16 = 0;
/// Orchestrator Version (minor)
pub const ORCHESTRATOR_MINOR_VERSION: u16 = 1;
/// Orchestrator Version as a type
pub type OrchestratorVersion =
    StaticVersion<ORCHESTRATOR_MAJOR_VERSION, ORCHESTRATOR_MINOR_VERSION>;
/// Orchestrator Version as a type-binding instance
pub const ORCHESTRATOR_VERSION: OrchestratorVersion = StaticVersion {};

/// Generate an keypair based on a `seed` and an `index`
/// # Panics
/// This panics if libp2p is unable to generate a secret key from the seed
#[must_use]
pub fn libp2p_generate_indexed_identity(seed: [u8; 32], index: u64) -> Keypair {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&seed);
    hasher.update(&index.to_le_bytes());
    let new_seed = *hasher.finalize().as_bytes();
    let sk_bytes = SecretKey::try_from_bytes(new_seed).unwrap();
    <EdKeypair as From<SecretKey>>::from(sk_bytes).into()
}

/// The state of the orchestrator
#[derive(Default, Clone)]
#[allow(clippy::struct_excessive_bools)]
struct OrchestratorState<TYPES: NodeType> {
    /// Tracks the latest node index we have generated a configuration for
    latest_index: u16,
    /// Tracks the latest temporary index we have generated for init validator's key pair
    tmp_latest_index: u16,
    /// The network configuration
    config: NetworkConfig<TYPES>,
    /// Whether the network configuration has been updated with all the peer's public keys/configs
    peer_pub_ready: bool,
    /// A map from public keys to `(node_index, is_da)`.
    pub_posted: HashMap<Vec<u8>, (u64, bool)>,
    /// Whether nodes should start their HotShot instances
    /// Will be set to true once all nodes post they are ready to start
    start: bool,
    /// The total nodes that have posted they are ready to start
    nodes_connected: HashSet<PeerConfig<TYPES>>,
    /// The results of the benchmarks
    bench_results: BenchResults,
    /// The number of nodes that have posted their results
    nodes_post_results: u64,
    /// Whether the orchestrator can be started manually
    manual_start_allowed: bool,
    /// Whether we are still accepting new keys for registration
    accepting_new_keys: bool,
    /// Builder address pool
    builders: Vec<Url>,
    /// whether we are using a fixed stake table, disabling public key registration
    fixed_stake_table: bool,
}

impl<TYPES: NodeType> OrchestratorState<TYPES> {
    /// create a new [`OrchestratorState`]
    pub fn new(network_config: NetworkConfig<TYPES>) -> Self {
        let mut peer_pub_ready = false;
        let mut fixed_stake_table = false;

        if network_config.config.known_nodes_with_stake.is_empty() {
            println!("No nodes were loaded from the config file. Nodes will be allowed to register dynamically.");
        } else {
            println!("Initializing orchestrator with fixed stake table.");
            peer_pub_ready = true;
            fixed_stake_table = true;
        }

        let builders = if matches!(network_config.builder, BuilderType::External) {
            network_config.config.builder_urls.clone().into()
        } else {
            vec![]
        };

        OrchestratorState {
            latest_index: 0,
            tmp_latest_index: 0,
            config: network_config,
            peer_pub_ready,
            pub_posted: HashMap::new(),
            nodes_connected: HashSet::new(),
            start: false,
            bench_results: BenchResults::default(),
            nodes_post_results: 0,
            manual_start_allowed: true,
            accepting_new_keys: true,
            builders,
            fixed_stake_table,
        }
    }

    /// Output the results to a csv file according to orchestrator state
    pub fn output_to_csv(&self) {
        let output_csv = BenchResultsDownloadConfig {
            commit_sha: self.config.commit_sha.clone(),
            total_nodes: self.config.config.num_nodes_with_stake.into(),
            da_committee_size: self.config.config.da_staked_committee_size,
            fixed_leader_for_gpuvid: self.config.config.fixed_leader_for_gpuvid,
            transactions_per_round: self.config.transactions_per_round,
            transaction_size: self.bench_results.transaction_size_in_bytes,
            rounds: self.config.rounds,
            partial_results: self.bench_results.partial_results.clone(),
            avg_latency_in_sec: self.bench_results.avg_latency_in_sec,
            minimum_latency_in_sec: self.bench_results.minimum_latency_in_sec,
            maximum_latency_in_sec: self.bench_results.maximum_latency_in_sec,
            throughput_bytes_per_sec: self.bench_results.throughput_bytes_per_sec,
            total_transactions_committed: self.bench_results.total_transactions_committed,
            total_time_elapsed_in_sec: self.bench_results.total_time_elapsed_in_sec,
            total_num_views: self.bench_results.total_num_views,
            failed_num_views: self.bench_results.failed_num_views,
            committee_type: self.bench_results.committee_type.clone(),
        };
        // Open the CSV file in append mode
        let results_csv_file = OpenOptions::new()
            .create(true)
            .append(true) // Open in append mode
            .open("scripts/benchmarks_results/results.csv")
            .unwrap();
        // Open a file for writing
        let mut wtr = Writer::from_writer(results_csv_file);
        let _ = wtr.serialize(output_csv);
        let _ = wtr.flush();
        println!("Results successfully saved in scripts/benchmarks_results/results.csv");
    }
}

/// An api exposed by the orchestrator
pub trait OrchestratorApi<TYPES: NodeType> {
    /// Post an identity to the orchestrator. Takes in optional
    /// arguments so others can identify us on the Libp2p network.
    /// # Errors
    /// If we were unable to serve the request
    fn post_identity(
        &mut self,
        libp2p_address: Option<Multiaddr>,
        libp2p_public_key: Option<PeerId>,
    ) -> Result<u16, ServerError>;
    /// post endpoint for each node's config
    /// # Errors
    /// if unable to serve
    fn post_getconfig(&mut self, _node_index: u16) -> Result<NetworkConfig<TYPES>, ServerError>;
    /// get endpoint for the next available temporary node index
    /// # Errors
    /// if unable to serve
    fn get_tmp_node_index(&mut self) -> Result<u16, ServerError>;
    /// post endpoint for each node's public key
    /// # Errors
    /// if unable to serve
    fn register_public_key(
        &mut self,
        pubkey: &mut Vec<u8>,
        is_da: bool,
        libp2p_address: Option<Multiaddr>,
        libp2p_public_key: Option<PeerId>,
    ) -> Result<(u64, bool), ServerError>;
    /// post endpoint for whether or not all peers public keys are ready
    /// # Errors
    /// if unable to serve
    fn peer_pub_ready(&self) -> Result<bool, ServerError>;
    /// get endpoint for the network config after all peers public keys are collected
    /// # Errors
    /// if unable to serve
    fn post_config_after_peer_collected(&mut self) -> Result<NetworkConfig<TYPES>, ServerError>;
    /// get endpoint for whether or not the run has started
    /// # Errors
    /// if unable to serve
    fn get_start(&self) -> Result<bool, ServerError>;
    /// post endpoint for the results of the run
    /// # Errors
    /// if unable to serve
    fn post_run_results(&mut self, metrics: BenchResults) -> Result<(), ServerError>;
    /// A node POSTs its public key to let the orchestrator know that it is ready
    /// # Errors
    /// if unable to serve
    fn post_ready(&mut self, peer_config: &PeerConfig<TYPES>) -> Result<(), ServerError>;
    /// post endpoint for manually starting the orchestrator
    /// # Errors
    /// if unable to serve
    fn post_manual_start(&mut self, password_bytes: Vec<u8>) -> Result<(), ServerError>;
    /// post endpoint for registering a builder with the orchestrator
    /// # Errors
    /// if unable to serve
    fn post_builder(&mut self, builder: Url) -> Result<(), ServerError>;
    /// get endpoints for builders
    /// # Errors
    /// if not all builders are registered yet
    fn get_builders(&self) -> Result<Vec<Url>, ServerError>;
}

impl<TYPES: NodeType> OrchestratorState<TYPES>
where
    TYPES::SignatureKey: serde::Serialize + Clone + SignatureKey + 'static,
{
    /// register a node with an unknown public key.
    /// this method should be used when we don't have a fixed stake table
    fn register_unknown(
        &mut self,
        pubkey: &mut Vec<u8>,
        da_requested: bool,
        libp2p_address: Option<Multiaddr>,
        libp2p_public_key: Option<PeerId>,
    ) -> Result<(u64, bool), ServerError> {
        if let Some((node_index, is_da)) = self.pub_posted.get(pubkey) {
            return Ok((*node_index, *is_da));
        }

        if !self.accepting_new_keys {
            return Err(ServerError {
                status: tide_disco::StatusCode::FORBIDDEN,
                message:
                    "Network has been started manually, and is no longer registering new keys."
                        .to_string(),
            });
        }

        let node_index = self.pub_posted.len() as u64;

        // Deserialize the public key
        let staked_pubkey = PeerConfig::<TYPES>::from_bytes(pubkey).unwrap();

        self.config
            .config
            .known_nodes_with_stake
            .push(staked_pubkey.clone());

        let mut added_to_da = false;

        let da_full =
            self.config.config.known_da_nodes.len() >= self.config.config.da_staked_committee_size;

        #[allow(clippy::nonminimal_bool)]
        // We add the node to the DA committee depending on either its node index or whether it requested membership.
        //
        // Since we issue `node_index` incrementally, if we are deciding DA membership by node_index
        // we only need to check that the DA committee is not yet full.
        //
        // Note: this logically simplifies to (self.config.indexed_da || da_requested) && !da_full,
        // but writing it that way makes it a little less clear to me.
        if (self.config.indexed_da || (!self.config.indexed_da && da_requested)) && !da_full {
            self.config.config.known_da_nodes.push(staked_pubkey);
            added_to_da = true;
        }

        self.pub_posted
            .insert(pubkey.clone(), (node_index, added_to_da));

        // If the orchestrator is set up for libp2p and we have supplied the proper
        // Libp2p data, add our node to the list of bootstrap nodes.
        if self.config.libp2p_config.clone().is_some() {
            if let (Some(libp2p_public_key), Some(libp2p_address)) =
                (libp2p_public_key, libp2p_address)
            {
                // Push to our bootstrap nodes
                self.config
                    .libp2p_config
                    .as_mut()
                    .unwrap()
                    .bootstrap_nodes
                    .push((libp2p_public_key, libp2p_address));
            }
        }

        tracing::error!("Posted public key for node_index {node_index}");

        // node_index starts at 0, so once it matches `num_nodes_with_stake`
        // we will have registered one node too many. hence, we want `node_index + 1`.
        if node_index + 1 >= (self.config.config.num_nodes_with_stake.get() as u64) {
            self.peer_pub_ready = true;
            self.accepting_new_keys = false;
        }
        Ok((node_index, added_to_da))
    }

    /// register a node on the fixed stake table, which was loaded at startup
    fn register_from_list(
        &mut self,
        pubkey: &mut Vec<u8>,
        da_requested: bool,
        libp2p_address: Option<Multiaddr>,
        libp2p_public_key: Option<PeerId>,
    ) -> Result<(u64, bool), ServerError> {
        // if we've already registered this node before, we just retrieve its info from `pub_posted`
        if let Some((node_index, is_da)) = self.pub_posted.get(pubkey) {
            return Ok((*node_index, *is_da));
        }

        // Deserialize the public key
        let staked_pubkey = PeerConfig::<TYPES>::from_bytes(pubkey).unwrap();

        // Check if the node is allowed to connect, returning its index and config entry if so.
        let Some((node_index, node_config)) =
            self.config.public_keys.iter().enumerate().find(|keys| {
                keys.1.stake_table_key == staked_pubkey.stake_table_entry.public_key()
            })
        else {
            return Err(ServerError {
                status: tide_disco::StatusCode::FORBIDDEN,
                message: "You are unauthorized to register with the orchestrator".to_string(),
            });
        };

        // Check that our recorded DA status for the node matches what the node actually requested
        if node_config.da != da_requested {
            return Err(ServerError {
                status: tide_disco::StatusCode::BAD_REQUEST,
                message: format!("Mismatch in DA status in registration for node {}. DA requested: {}, expected: {}", node_index, da_requested, node_config.da),
          });
        }

        let added_to_da = node_config.da;

        self.pub_posted
            .insert(pubkey.clone(), (node_index as u64, added_to_da));

        // If the orchestrator is set up for libp2p and we have supplied the proper
        // Libp2p data, add our node to the list of bootstrap nodes.
        if self.config.libp2p_config.clone().is_some() {
            if let (Some(libp2p_public_key), Some(libp2p_address)) =
                (libp2p_public_key, libp2p_address)
            {
                // Push to our bootstrap nodes
                self.config
                    .libp2p_config
                    .as_mut()
                    .unwrap()
                    .bootstrap_nodes
                    .push((libp2p_public_key, libp2p_address));
            }
        }

        tracing::error!("Node {node_index} has registered.");

        Ok((node_index as u64, added_to_da))
    }
}

impl<TYPES: NodeType> OrchestratorApi<TYPES> for OrchestratorState<TYPES>
where
    TYPES::SignatureKey: serde::Serialize + Clone + SignatureKey + 'static,
{
    /// Post an identity to the orchestrator. Takes in optional
    /// arguments so others can identify us on the Libp2p network.
    /// # Errors
    /// If we were unable to serve the request
    fn post_identity(
        &mut self,
        libp2p_address: Option<Multiaddr>,
        libp2p_public_key: Option<PeerId>,
    ) -> Result<u16, ServerError> {
        let node_index = self.latest_index;
        self.latest_index += 1;

        if usize::from(node_index) >= self.config.config.num_nodes_with_stake.get() {
            return Err(ServerError {
                status: tide_disco::StatusCode::BAD_REQUEST,
                message: "Network has reached capacity".to_string(),
            });
        }

        // If the orchestrator is set up for libp2p and we have supplied the proper
        // Libp2p data, add our node to the list of bootstrap nodes.
        if self.config.libp2p_config.clone().is_some() {
            if let (Some(libp2p_public_key), Some(libp2p_address)) =
                (libp2p_public_key, libp2p_address)
            {
                // Push to our bootstrap nodes
                self.config
                    .libp2p_config
                    .as_mut()
                    .unwrap()
                    .bootstrap_nodes
                    .push((libp2p_public_key, libp2p_address));
            }
        }
        Ok(node_index)
    }

    // Assumes nodes will set their own index that they received from the
    // 'identity' endpoint
    fn post_getconfig(&mut self, _node_index: u16) -> Result<NetworkConfig<TYPES>, ServerError> {
        Ok(self.config.clone())
    }

    // Assumes one node do not get twice
    fn get_tmp_node_index(&mut self) -> Result<u16, ServerError> {
        let tmp_node_index = self.tmp_latest_index;
        self.tmp_latest_index += 1;

        if usize::from(tmp_node_index) >= self.config.config.num_nodes_with_stake.get() {
            return Err(ServerError {
                status: tide_disco::StatusCode::BAD_REQUEST,
                message: "Node index getter for key pair generation has reached capacity"
                    .to_string(),
            });
        }
        Ok(tmp_node_index)
    }

    #[allow(clippy::cast_possible_truncation)]
    fn register_public_key(
        &mut self,
        pubkey: &mut Vec<u8>,
        da_requested: bool,
        libp2p_address: Option<Multiaddr>,
        libp2p_public_key: Option<PeerId>,
    ) -> Result<(u64, bool), ServerError> {
        if self.fixed_stake_table {
            self.register_from_list(pubkey, da_requested, libp2p_address, libp2p_public_key)
        } else {
            self.register_unknown(pubkey, da_requested, libp2p_address, libp2p_public_key)
        }
    }

    fn peer_pub_ready(&self) -> Result<bool, ServerError> {
        if !self.peer_pub_ready {
            return Err(ServerError {
                status: tide_disco::StatusCode::BAD_REQUEST,
                message: "Peer's public configs are not ready".to_string(),
            });
        }
        Ok(self.peer_pub_ready)
    }

    fn post_config_after_peer_collected(&mut self) -> Result<NetworkConfig<TYPES>, ServerError> {
        if !self.peer_pub_ready {
            return Err(ServerError {
                status: tide_disco::StatusCode::BAD_REQUEST,
                message: "Peer's public configs are not ready".to_string(),
            });
        }

        Ok(self.config.clone())
    }

    fn get_start(&self) -> Result<bool, ServerError> {
        // println!("{}", self.start);
        if !self.start {
            return Err(ServerError {
                status: tide_disco::StatusCode::BAD_REQUEST,
                message: "Network is not ready to start".to_string(),
            });
        }
        Ok(self.start)
    }

    // Assumes nodes do not post 'ready' twice
    fn post_ready(&mut self, peer_config: &PeerConfig<TYPES>) -> Result<(), ServerError> {
        // If we have not disabled registration verification.
        // Is this node allowed to connect?
        if !self
            .config
            .config
            .known_nodes_with_stake
            .contains(peer_config)
        {
            return Err(ServerError {
                status: tide_disco::StatusCode::FORBIDDEN,
                message: "You are unauthorized to register with the orchestrator".to_string(),
            });
        }

        // `HashSet::insert()` returns whether the node was newly inserted (true) or not
        if self.nodes_connected.insert(peer_config.clone()) {
            tracing::error!(
                "Node {peer_config} connected. Total nodes connected: {}",
                self.nodes_connected.len()
            );
        }

        // i.e. nodes_connected >= num_nodes_with_stake * (start_threshold.0 / start_threshold.1)
        if self.nodes_connected.len() as u64 * self.config.config.start_threshold.1
            >= (self.config.config.num_nodes_with_stake.get() as u64)
                * self.config.config.start_threshold.0
        {
            self.accepting_new_keys = false;
            self.manual_start_allowed = false;
            self.start = true;
        }

        Ok(())
    }

    /// Manually start the network
    fn post_manual_start(&mut self, password_bytes: Vec<u8>) -> Result<(), ServerError> {
        if !self.manual_start_allowed {
            return Err(ServerError {
            status: tide_disco::StatusCode::FORBIDDEN,
            message: "Configs have already been distributed to nodes, and the network can no longer be started manually.".to_string(),
          });
        }

        let password = String::from_utf8(password_bytes)
            .expect("Failed to decode raw password as UTF-8 string.");

        // Check that the password matches
        if self.config.manual_start_password != Some(password) {
            return Err(ServerError {
                status: tide_disco::StatusCode::FORBIDDEN,
                message: "Incorrect password.".to_string(),
            });
        }

        let registered_nodes_with_stake = self.config.config.known_nodes_with_stake.len();
        let registered_da_nodes = self.config.config.known_da_nodes.len();

        if registered_da_nodes > 1 {
            self.config.config.num_nodes_with_stake =
                std::num::NonZeroUsize::new(registered_nodes_with_stake)
                    .expect("Failed to convert to NonZeroUsize; this should be impossible.");

            self.config.config.da_staked_committee_size = registered_da_nodes;
        } else {
            return Err(ServerError {
                status: tide_disco::StatusCode::FORBIDDEN,
                message: format!("We cannot manually start the network, because we only have {registered_nodes_with_stake} nodes with stake registered, with {registered_da_nodes} DA nodes.")
            });
        }

        self.accepting_new_keys = false;
        self.manual_start_allowed = false;
        self.peer_pub_ready = true;
        self.start = true;

        Ok(())
    }

    // Aggregates results of the run from all nodes
    fn post_run_results(&mut self, metrics: BenchResults) -> Result<(), ServerError> {
        if metrics.total_transactions_committed != 0 {
            // Deal with the bench results
            if self.bench_results.total_transactions_committed == 0 {
                self.bench_results = metrics;
            } else {
                // Deal with the bench results from different nodes
                let cur_metrics = self.bench_results.clone();
                self.bench_results.avg_latency_in_sec = (metrics.avg_latency_in_sec
                    * metrics.num_latency
                    + cur_metrics.avg_latency_in_sec * cur_metrics.num_latency)
                    / (metrics.num_latency + cur_metrics.num_latency);
                self.bench_results.num_latency += metrics.num_latency;
                self.bench_results.minimum_latency_in_sec = metrics
                    .minimum_latency_in_sec
                    .min(cur_metrics.minimum_latency_in_sec);
                self.bench_results.maximum_latency_in_sec = metrics
                    .maximum_latency_in_sec
                    .max(cur_metrics.maximum_latency_in_sec);
                self.bench_results.throughput_bytes_per_sec = metrics
                    .throughput_bytes_per_sec
                    .max(cur_metrics.throughput_bytes_per_sec);
                self.bench_results.total_transactions_committed = metrics
                    .total_transactions_committed
                    .max(cur_metrics.total_transactions_committed);
                self.bench_results.total_time_elapsed_in_sec = metrics
                    .total_time_elapsed_in_sec
                    .max(cur_metrics.total_time_elapsed_in_sec);
                self.bench_results.total_num_views =
                    metrics.total_num_views.min(cur_metrics.total_num_views);
                self.bench_results.failed_num_views =
                    metrics.failed_num_views.max(cur_metrics.failed_num_views);
            }
        }
        self.nodes_post_results += 1;
        if self.bench_results.partial_results == "Unset" {
            self.bench_results.partial_results = "One".to_string();
            self.bench_results.printout();
            self.output_to_csv();
        }
        if self.bench_results.partial_results == "One"
            && self.nodes_post_results >= (self.config.config.da_staked_committee_size as u64 / 2)
        {
            self.bench_results.partial_results = "HalfDA".to_string();
            self.bench_results.printout();
            self.output_to_csv();
        }
        if self.bench_results.partial_results == "HalfDA"
            && self.nodes_post_results >= (self.config.config.num_nodes_with_stake.get() as u64 / 2)
        {
            self.bench_results.partial_results = "Half".to_string();
            self.bench_results.printout();
            self.output_to_csv();
        }
        if self.bench_results.partial_results != "Full"
            && self.nodes_post_results >= (self.config.config.num_nodes_with_stake.get() as u64)
        {
            self.bench_results.partial_results = "Full".to_string();
            self.bench_results.printout();
            self.output_to_csv();
        }
        Ok(())
    }

    fn post_builder(&mut self, builder: Url) -> Result<(), ServerError> {
        self.builders.push(builder);
        Ok(())
    }

    fn get_builders(&self) -> Result<Vec<Url>, ServerError> {
        if !matches!(self.config.builder, BuilderType::External)
            && self.builders.len() != self.config.config.da_staked_committee_size
        {
            return Err(ServerError {
                status: tide_disco::StatusCode::NOT_FOUND,
                message: "Not all builders are registered yet".to_string(),
            });
        }
        Ok(self.builders.clone())
    }
}

/// Sets up all API routes
#[allow(clippy::too_many_lines)]
fn define_api<TYPES, State, VER>() -> Result<Api<State, ServerError, VER>, ApiError>
where
    TYPES: NodeType,
    State: 'static + Send + Sync + ReadState + WriteState,
    <State as ReadState>::State: Send + Sync + OrchestratorApi<TYPES>,
    TYPES::SignatureKey: serde::Serialize,
    VER: StaticVersionType + 'static,
{
    let api_toml = toml::from_str::<toml::Value>(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/api.toml"
    )))
    .expect("API file is not valid toml");
    let mut api = Api::<State, ServerError, VER>::new(api_toml)?;
    api.post("post_identity", |req, state| {
        async move {
            // Read the bytes from the body
            let mut body_bytes = req.body_bytes();
            body_bytes.drain(..12);

            // Decode the libp2p data so we can add to our bootstrap nodes (if supplied)
            let Ok((libp2p_address, libp2p_public_key)) =
                vbs::Serializer::<OrchestratorVersion>::deserialize(&body_bytes)
            else {
                return Err(ServerError {
                    status: tide_disco::StatusCode::BAD_REQUEST,
                    message: "Malformed body".to_string(),
                });
            };

            // Call our state function to process the request
            state.post_identity(libp2p_address, libp2p_public_key)
        }
        .boxed()
    })?
    .post("post_getconfig", |req, state| {
        async move {
            let node_index = req.integer_param("node_index")?;
            state.post_getconfig(node_index)
        }
        .boxed()
    })?
    .post("get_tmp_node_index", |_req, state| {
        async move { state.get_tmp_node_index() }.boxed()
    })?
    .post("post_pubkey", |req, state| {
        async move {
            let is_da = req.boolean_param("is_da")?;
            // Read the bytes from the body
            let mut body_bytes = req.body_bytes();
            body_bytes.drain(..12);

            // Decode the libp2p data so we can add to our bootstrap nodes (if supplied)
            let Ok((mut pubkey, libp2p_address, libp2p_public_key)) =
                vbs::Serializer::<OrchestratorVersion>::deserialize(&body_bytes)
            else {
                return Err(ServerError {
                    status: tide_disco::StatusCode::BAD_REQUEST,
                    message: "Malformed body".to_string(),
                });
            };

            state.register_public_key(&mut pubkey, is_da, libp2p_address, libp2p_public_key)
        }
        .boxed()
    })?
    .get("peer_pubconfig_ready", |_req, state| {
        async move { state.peer_pub_ready() }.boxed()
    })?
    .post("post_config_after_peer_collected", |_req, state| {
        async move { state.post_config_after_peer_collected() }.boxed()
    })?
    .post(
        "post_ready",
        |req, state: &mut <State as ReadState>::State| {
            async move {
                let mut body_bytes = req.body_bytes();
                body_bytes.drain(..12);
                // Decode the payload-supplied pubkey
                let Some(pubkey) = PeerConfig::<TYPES>::from_bytes(&body_bytes) else {
                    return Err(ServerError {
                        status: tide_disco::StatusCode::BAD_REQUEST,
                        message: "Malformed body".to_string(),
                    });
                };
                state.post_ready(&pubkey)
            }
            .boxed()
        },
    )?
    .post(
        "post_manual_start",
        |req, state: &mut <State as ReadState>::State| {
            async move {
                let password = req.body_bytes();
                state.post_manual_start(password)
            }
            .boxed()
        },
    )?
    .get("get_start", |_req, state| {
        async move { state.get_start() }.boxed()
    })?
    .post("post_results", |req, state| {
        async move {
            let metrics: Result<BenchResults, RequestError> = req.body_json();
            state.post_run_results(metrics.unwrap())
        }
        .boxed()
    })?
    .post("post_builder", |req, state| {
        async move {
            // Read the bytes from the body
            let mut body_bytes = req.body_bytes();
            body_bytes.drain(..12);

            let Ok(urls) =
                vbs::Serializer::<OrchestratorVersion>::deserialize::<Vec<Url>>(&body_bytes)
            else {
                return Err(ServerError {
                    status: tide_disco::StatusCode::BAD_REQUEST,
                    message: "Malformed body".to_string(),
                });
            };

            let mut futures = urls
                .into_iter()
                .map(|url| async {
                    let client: surf_disco::Client<ServerError, OrchestratorVersion> =
                        surf_disco::client::Client::builder(url.clone()).build();
                    if client.connect(Some(Duration::from_secs(2))).await {
                        Some(url)
                    } else {
                        None
                    }
                })
                .collect::<FuturesUnordered<_>>()
                .filter_map(futures::future::ready);

            if let Some(url) = futures.next().await {
                state.post_builder(url)
            } else {
                Err(ServerError {
                    status: tide_disco::StatusCode::BAD_REQUEST,
                    message: "No reachable addresses".to_string(),
                })
            }
        }
        .boxed()
    })?
    .get("get_builders", |_req, state| {
        async move { state.get_builders() }.boxed()
    })?;
    Ok(api)
}

/// Runs the orchestrator
/// # Errors
/// This errors if tide disco runs into an issue during serving
/// # Panics
/// This panics if unable to register the api with tide disco
pub async fn run_orchestrator<TYPES: NodeType>(
    mut network_config: NetworkConfig<TYPES>,
    url: Url,
) -> io::Result<()>
where
    TYPES::SignatureKey: 'static + serde::Serialize,
{
    let env_password = std::env::var("ORCHESTRATOR_MANUAL_START_PASSWORD");

    if env_password.is_ok() {
        tracing::warn!("Took orchestrator manual start password from the environment variable: ORCHESTRATOR_MANUAL_START_PASSWORD={:?}", env_password);
        network_config.manual_start_password = env_password.ok();
    }

    // Try to overwrite the network_config public keys
    // from the file the env var points to, or panic.
    {
        let env_public_keys = std::env::var("ORCHESTRATOR_PUBLIC_KEYS");

        if let Ok(filepath) = env_public_keys {
            #[allow(clippy::panic)]
            let config_file_as_string: String = fs::read_to_string(filepath.clone())
                .unwrap_or_else(|_| panic!("Could not read config file located at {filepath}"));

            let file: PublicKeysFile<TYPES> =
                toml::from_str::<PublicKeysFile<TYPES>>(&config_file_as_string)
                    .expect("Unable to convert config file to TOML");

            network_config.public_keys = file.public_keys;
        }
    }

    network_config.config.known_nodes_with_stake = network_config
        .public_keys
        .iter()
        .map(|keys| PeerConfig {
            stake_table_entry: keys
                .stake_table_key
                .stake_table_entry(U256::from(keys.stake)),
            state_ver_key: keys.state_ver_key.clone(),
        })
        .collect();

    network_config.config.known_da_nodes = network_config
        .public_keys
        .iter()
        .filter(|keys| keys.da)
        .map(|keys| PeerConfig {
            stake_table_entry: keys
                .stake_table_key
                .stake_table_entry(U256::from(keys.stake)),
            state_ver_key: keys.state_ver_key.clone(),
        })
        .collect();

    let web_api =
        define_api().map_err(|_e| io::Error::new(ErrorKind::Other, "Failed to define api"));

    let state: RwLock<OrchestratorState<TYPES>> =
        RwLock::new(OrchestratorState::new(network_config));

    let mut app = App::<RwLock<OrchestratorState<TYPES>>, ServerError>::with_state(state);
    app.register_module::<ServerError, OrchestratorVersion>("api", web_api.unwrap())
        .expect("Error registering api");
    tracing::error!("listening on {:?}", url);
    app.serve(url, ORCHESTRATOR_VERSION).await
}
