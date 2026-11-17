// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{fs, ops::Range, path::Path};

use clap::ValueEnum;
use thiserror::Error;

use crate::{HotShotConfig, NodeType, PeerConnectInfo, hotshot_config_file::HotShotConfigFile};

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
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
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
    pub connect_info: Option<PeerConnectInfo>,
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

/// The network config of a node: its index and the consensus config.
///
/// Configs a node stored in earlier releases carry more fields, for the orchestrator that
/// started their network; they are ignored when read.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(bound(deserialize = ""))]
pub struct NetworkConfig<TYPES: NodeType> {
    /// The node's position in the genesis stake table.
    pub node_index: u64,
    /// the hotshot config
    pub config: HotShotConfig<TYPES>,
}

impl<TYPES: NodeType> NetworkConfig<TYPES> {
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
    /// let config = NetworkConfig::default();
    /// config.to_file("/path/to/my/config".to_string()).unwrap();
    /// ```
    pub fn to_file(&self, file: String) -> Result<(), NetworkConfigError> {
        // ensure the directory containing the config file exists
        if let Some(dir) = Path::new(&file).parent()
            && let Err(e) = fs::create_dir_all(dir)
        {
            return Err(NetworkConfigError::FailedToCreatePath(e));
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
            node_index: 0,
            config: HotShotConfigFile::hotshot_config_5_nodes_10_da().into(),
        }
    }
}
