// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{fs, ops::Range, path::Path, time::Duration, vec};

use alloy::primitives::U256;
use clap::ValueEnum;
use libp2p_identity::PeerId;
use multiaddr::Multiaddr;
use serde_inline_default::serde_inline_default;
use thiserror::Error;
use tracing::error;

use crate::{
    constants::{
        ORCHESTRATOR_DEFAULT_NUM_ROUNDS, ORCHESTRATOR_DEFAULT_TRANSACTIONS_PER_ROUND,
        ORCHESTRATOR_DEFAULT_TRANSACTION_SIZE, REQUEST_DATA_DELAY,
    },
    hotshot_config_file::HotShotConfigFile,
    HotShotConfig, NodeType, ValidatorConfig,
};

/// Configuration describing a libp2p node
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Libp2pConfig {
    /// The bootstrap nodes to connect to (multiaddress, serialized public key)
    pub bootstrap_nodes: Vec<(PeerId, Multiaddr)>,
}

/// configuration for combined network
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct CombinedNetworkConfig {
    /// delay duration before sending a message through the secondary network
    pub delay_duration: Duration,
}

/// a network configuration error
#[derive(Error, Debug)]
pub enum NetworkConfigError {
    /// Failed to read NetworkConfig from file
    #[error("Failed to read NetworkConfig from file")]
    ReadFromFileError(std::io::Error),
    /// Failed to deserialize loaded NetworkConfig
    #[error("Failed to deserialize loaded NetworkConfig")]
    DeserializeError(serde_json::Error),
    /// Failed to write NetworkConfig to file
    #[error("Failed to write NetworkConfig to file")]
    WriteToFileError(std::io::Error),
    /// Failed to serialize NetworkConfig
    #[error("Failed to serialize NetworkConfig")]
    SerializeError(serde_json::Error),
    /// Failed to recursively create path to NetworkConfig
    #[error("Failed to recursively create path to NetworkConfig")]
    FailedToCreatePath(std::io::Error),
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, Default, ValueEnum)]
/// configuration for builder type to use
pub enum BuilderType {
    /// Use external builder, [config.builder_url] must be
    /// set to correct builder address
    External,
    #[default]
    /// Simple integrated builder will be started and used by each hotshot node
    Simple,
    /// Random integrated builder will be started and used by each hotshot node
    Random,
}

/// Node PeerConfig keys
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(bound(deserialize = ""))]
pub struct PeerConfigKeys<TYPES: NodeType> {
    /// The peer's public key
    pub stake_table_key: TYPES::SignatureKey,
    /// the peer's state public key
    pub state_ver_key: TYPES::StateSignatureKey,
    /// the peer's stake
    pub stake: u64,
    /// whether the node is a DA node
    pub da: bool,
}

/// Options controlling how the random builder generates blocks
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct RandomBuilderConfig {
    /// How many transactions to include in a block
    pub txn_in_block: u64,
    /// How many blocks to generate per second
    pub blocks_per_second: u32,
    /// Range of how big a transaction can be (in bytes)
    pub txn_size: Range<u32>,
}

impl Default for RandomBuilderConfig {
    fn default() -> Self {
        Self {
            txn_in_block: 100,
            blocks_per_second: 1,
            txn_size: 20..100,
        }
    }
}

/// a network configuration
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(bound(deserialize = ""))]
pub struct NetworkConfig<TYPES: NodeType> {
    /// number of views to run
    pub rounds: usize,
    /// whether DA membership is determined by index.
    /// if true, the first k nodes to register form the DA committee
    /// if false, DA membership is requested by the nodes
    pub indexed_da: bool,
    /// number of transactions per view
    pub transactions_per_round: usize,
    /// password to have the orchestrator start the network,
    /// regardless of the number of nodes connected.
    pub manual_start_password: Option<String>,
    /// number of bootstrap nodes
    pub num_bootrap: usize,
    /// timeout before starting the next view
    pub next_view_timeout: u64,
    /// timeout before starting next view sync round
    pub view_sync_timeout: Duration,
    /// The maximum amount of time a leader can wait to get a block from a builder
    pub builder_timeout: Duration,
    /// time to wait until we request data associated with a proposal
    pub data_request_delay: Duration,
    /// global index of node (for testing purposes a uid)
    pub node_index: u64,
    /// unique seed (for randomness? TODO)
    pub seed: [u8; 32],
    /// size of transactions
    pub transaction_size: usize,
    /// name of the key type (for debugging)
    pub key_type_name: String,
    /// the libp2p config
    pub libp2p_config: Option<Libp2pConfig>,
    /// the hotshot config
    pub config: HotShotConfig<TYPES>,
    /// The address for the Push CDN's "marshal", A.K.A. load balancer
    pub cdn_marshal_address: Option<String>,
    /// combined network config
    pub combined_network_config: Option<CombinedNetworkConfig>,
    /// the commit this run is based on
    pub commit_sha: String,
    /// builder to use
    pub builder: BuilderType,
    /// random builder config
    pub random_builder: Option<RandomBuilderConfig>,
    /// The list of public keys that are allowed to connect to the orchestrator
    pub public_keys: Vec<PeerConfigKeys<TYPES>>,
}

/// the source of the network config
pub enum NetworkConfigSource {
    /// we source the network configuration from the orchestrator
    Orchestrator,
    /// we source the network configuration from a config file on disk
    File,
}

impl<TYPES: NodeType> NetworkConfig<TYPES> {
    /// Get a temporary node index for generating a validator config
    #[must_use]
    pub fn generate_init_validator_config(
        cur_node_index: u16,
        is_da: bool,
    ) -> ValidatorConfig<TYPES> {
        // This cur_node_index is only used for key pair generation, it's not bound with the node,
        // later the node with the generated key pair will get a new node_index from orchestrator.
        ValidatorConfig::generated_from_seed_indexed(
            [0u8; 32],
            cur_node_index.into(),
            U256::from(1),
            is_da,
        )
    }

    /// Loads a `NetworkConfig` from a file.
    ///
    /// This function takes a file path as a string, reads the file, and then deserializes the contents into a `NetworkConfig`.
    ///
    /// # Arguments
    ///
    /// * `file` - A string representing the path to the file from which to load the `NetworkConfig`.
    ///
    /// # Returns
    ///
    /// This function returns a `Result` that contains a `NetworkConfig` if the file was successfully read and deserialized, or a `NetworkConfigError` if an error occurred.
    ///
    /// # Errors
    ///
    /// This function will return an error if the file cannot be read or if the contents cannot be deserialized into a `NetworkConfig`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// # use hotshot_orchestrator::config::NetworkConfig;
    /// // # use hotshot::traits::election::static_committee::StaticElectionConfig;
    /// let file = "/path/to/my/config".to_string();
    /// // NOTE: broken due to staticelectionconfig not being importable
    /// // cannot import staticelectionconfig from hotshot without creating circular dependency
    /// // making this work probably involves the `types` crate implementing a dummy
    /// // electionconfigtype just to make this example work
    /// let config = NetworkConfig::<TYPES, StaticElectionConfig>::from_file(file).unwrap();
    /// ```
    pub fn from_file(file: String) -> Result<Self, NetworkConfigError> {
        // read from file
        let data = match fs::read(file) {
            Ok(data) => data,
            Err(e) => {
                return Err(NetworkConfigError::ReadFromFileError(e));
            },
        };

        // deserialize
        match serde_json::from_slice(&data) {
            Ok(data) => Ok(data),
            Err(e) => Err(NetworkConfigError::DeserializeError(e)),
        }
    }

    /// Serializes the `NetworkConfig` and writes it to a file.
    ///
    /// This function takes a file path as a string, serializes the `NetworkConfig` into JSON format using `serde_json` and then writes the serialized data to the file.
    ///
    /// # Arguments
    ///
    /// * `file` - A string representing the path to the file where the `NetworkConfig` should be saved.
    ///
    /// # Returns
    ///
    /// This function returns a `Result` that contains `()` if the `NetworkConfig` was successfully serialized and written to the file, or a `NetworkConfigError` if an error occurred.
    ///
    /// # Errors
    ///
    /// This function will return an error if the `NetworkConfig` cannot be serialized or if the file cannot be written.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// # use hotshot_orchestrator::config::NetworkConfig;
    /// let file = "/path/to/my/config".to_string();
    /// let config = NetworkConfig::from_file(file);
    /// config.to_file(file).unwrap();
    /// ```
    pub fn to_file(&self, file: String) -> Result<(), NetworkConfigError> {
        // ensure the directory containing the config file exists
        if let Some(dir) = Path::new(&file).parent() {
            if let Err(e) = fs::create_dir_all(dir) {
                return Err(NetworkConfigError::FailedToCreatePath(e));
            }
        }

        // serialize
        let serialized = match serde_json::to_string_pretty(self) {
            Ok(data) => data,
            Err(e) => {
                return Err(NetworkConfigError::SerializeError(e));
            },
        };

        // write to file
        match fs::write(file, serialized) {
            Ok(()) => Ok(()),
            Err(e) => Err(NetworkConfigError::WriteToFileError(e)),
        }
    }
}

impl<TYPES: NodeType> Default for NetworkConfig<TYPES> {
    fn default() -> Self {
        Self {
            rounds: ORCHESTRATOR_DEFAULT_NUM_ROUNDS,
            indexed_da: true,
            transactions_per_round: ORCHESTRATOR_DEFAULT_TRANSACTIONS_PER_ROUND,
            node_index: 0,
            seed: [0u8; 32],
            transaction_size: ORCHESTRATOR_DEFAULT_TRANSACTION_SIZE,
            manual_start_password: None,
            libp2p_config: None,
            config: HotShotConfigFile::hotshot_config_5_nodes_10_da().into(),
            key_type_name: std::any::type_name::<TYPES>().to_string(),
            cdn_marshal_address: None,
            combined_network_config: None,
            next_view_timeout: 10,
            view_sync_timeout: Duration::from_secs(2),
            num_bootrap: 5,
            builder_timeout: Duration::from_secs(10),
            data_request_delay: Duration::from_millis(2500),
            commit_sha: String::new(),
            builder: BuilderType::default(),
            random_builder: None,
            public_keys: vec![],
        }
    }
}

/// a network config stored in a file
#[serde_inline_default]
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(bound(deserialize = ""))]
pub struct PublicKeysFile<TYPES: NodeType> {
    /// The list of public keys that are allowed to connect to the orchestrator
    ///
    /// If nonempty, this list becomes the stake table and is used to determine DA membership (ignoring the node's request).
    #[serde(default)]
    pub public_keys: Vec<PeerConfigKeys<TYPES>>,
}

/// a network config stored in a file
#[serde_inline_default]
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(bound(deserialize = ""))]
pub struct NetworkConfigFile<TYPES: NodeType> {
    /// number of views to run
    #[serde_inline_default(ORCHESTRATOR_DEFAULT_NUM_ROUNDS)]
    pub rounds: usize,
    /// number of views to run
    #[serde(default)]
    pub indexed_da: bool,
    /// number of transactions per view
    #[serde_inline_default(ORCHESTRATOR_DEFAULT_TRANSACTIONS_PER_ROUND)]
    pub transactions_per_round: usize,
    /// password to have the orchestrator start the network,
    /// regardless of the number of nodes connected.
    #[serde(default)]
    pub manual_start_password: Option<String>,
    /// global index of node (for testing purposes a uid)
    #[serde(default)]
    pub node_index: u64,
    /// unique seed (for randomness? TODO)
    #[serde(default)]
    pub seed: [u8; 32],
    /// size of transactions
    #[serde_inline_default(ORCHESTRATOR_DEFAULT_TRANSACTION_SIZE)]
    pub transaction_size: usize,
    /// the hotshot config file
    #[serde(default = "HotShotConfigFile::hotshot_config_5_nodes_10_da")]
    pub config: HotShotConfigFile<TYPES>,
    /// The address of the Push CDN's "marshal", A.K.A. load balancer
    #[serde(default)]
    pub cdn_marshal_address: Option<String>,
    /// combined network config
    #[serde(default)]
    pub combined_network_config: Option<CombinedNetworkConfig>,
    /// builder to use
    #[serde(default)]
    pub builder: BuilderType,
    /// random builder configuration
    #[serde(default)]
    pub random_builder: Option<RandomBuilderConfig>,
    /// The list of public keys that are allowed to connect to the orchestrator
    ///
    /// If nonempty, this list becomes the stake table and is used to determine DA membership (ignoring the node's request).
    #[serde(default)]
    pub public_keys: Vec<PeerConfigKeys<TYPES>>,
}

impl<TYPES: NodeType> From<NetworkConfigFile<TYPES>> for NetworkConfig<TYPES> {
    fn from(val: NetworkConfigFile<TYPES>) -> Self {
        NetworkConfig {
            rounds: val.rounds,
            indexed_da: val.indexed_da,
            transactions_per_round: val.transactions_per_round,
            node_index: 0,
            num_bootrap: val.config.num_bootstrap,
            manual_start_password: val.manual_start_password,
            next_view_timeout: val.config.next_view_timeout,
            view_sync_timeout: val.config.view_sync_timeout,
            builder_timeout: val.config.builder_timeout,
            data_request_delay: val
                .config
                .data_request_delay
                .unwrap_or(Duration::from_millis(REQUEST_DATA_DELAY)),
            seed: val.seed,
            transaction_size: val.transaction_size,
            libp2p_config: Some(Libp2pConfig {
                bootstrap_nodes: Vec::new(),
            }),
            config: val.config.into(),
            key_type_name: std::any::type_name::<TYPES>().to_string(),
            cdn_marshal_address: val.cdn_marshal_address,
            combined_network_config: val.combined_network_config,
            commit_sha: String::new(),
            builder: val.builder,
            random_builder: val.random_builder,
            public_keys: val.public_keys,
        }
    }
}
