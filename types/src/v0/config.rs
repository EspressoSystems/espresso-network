use std::{collections::BTreeMap, num::NonZeroUsize, time::Duration};

use hotshot_types::{
    network::{
        BuilderType, CombinedNetworkConfig, Libp2pConfig, NetworkConfig, RandomBuilderConfig,
    },
    HotShotConfig, PeerConfig, ValidatorConfig,
};
use serde::{Deserialize, Serialize};
use tide_disco::Url;
use vbs::version::Version;
use vec1::Vec1;

use crate::{PubKey, SeqTypes};

/// This struct defines the public Hotshot validator configuration.
/// Private key and state key pairs are excluded for security reasons.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PublicValidatorConfig {
    public_key: PubKey,
    stake_value: u64,
    is_da: bool,
    private_key: String,
    state_public_key: String,
    state_key_pair: String,
}

impl From<ValidatorConfig<SeqTypes>> for PublicValidatorConfig {
    fn from(v: ValidatorConfig<SeqTypes>) -> Self {
        let ValidatorConfig::<SeqTypes> {
            public_key,
            private_key: _,
            stake_value,
            state_public_key,
            state_private_key: _,
            is_da,
        } = v;

        Self {
            public_key,
            stake_value: stake_value.to::<u64>(),
            is_da,
            state_public_key: state_public_key.to_string(),
            private_key: "*****".into(),
            state_key_pair: "*****".into(),
        }
    }
}

/// This struct defines the public Hotshot configuration parameters.
/// Our config module features a GET endpoint accessible via the route `/hotshot` to display the hotshot config parameters.
/// Hotshot config has sensitive information like private keys and such fields are excluded from this struct.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PublicHotShotConfig {
    start_threshold: (u64, u64),
    num_nodes_with_stake: NonZeroUsize,
    known_nodes_with_stake: Vec<PeerConfig<SeqTypes>>,
    known_da_nodes: Vec<PeerConfig<SeqTypes>>,
    #[serde(default)]
    da_committees: BTreeMap<Version, BTreeMap<u64, Vec<PeerConfig<SeqTypes>>>>,
    da_staked_committee_size: usize,
    fixed_leader_for_gpuvid: usize,
    next_view_timeout: u64,
    view_sync_timeout: Duration,
    num_bootstrap: usize,
    builder_timeout: Duration,
    data_request_delay: Duration,
    builder_urls: Vec1<Url>,
    start_proposing_view: u64,
    stop_proposing_view: u64,
    start_voting_view: u64,
    stop_voting_view: u64,
    start_proposing_time: u64,
    stop_proposing_time: u64,
    start_voting_time: u64,
    stop_voting_time: u64,
    epoch_height: u64,
    epoch_start_block: u64,
    #[serde(default = "default_stake_table_capacity")]
    stake_table_capacity: usize,
    #[serde(default = "default_drb_difficulty")]
    drb_difficulty: u64,
    #[serde(default = "default_drb_upgrade_difficulty")]
    drb_upgrade_difficulty: u64,
}

fn default_stake_table_capacity() -> usize {
    hotshot_types::light_client::DEFAULT_STAKE_TABLE_CAPACITY
}

/// Default DRB difficulty, set to 0 (intended to be overwritten)
fn default_drb_difficulty() -> u64 {
    0
}

/// Default DRB upgrade difficulty, set to 0 (intended to be overwritten)
fn default_drb_upgrade_difficulty() -> u64 {
    0
}

impl From<HotShotConfig<SeqTypes>> for PublicHotShotConfig {
    fn from(v: HotShotConfig<SeqTypes>) -> Self {
        // Destructure all fields from HotShotConfig to return an error
        // if new fields are added to HotShotConfig. This makes sure that we handle
        // all fields appropriately and do not miss any updates.
        let HotShotConfig::<SeqTypes> {
            start_threshold,
            num_nodes_with_stake,
            known_nodes_with_stake,
            known_da_nodes,
            da_committees,
            da_staked_committee_size,
            fixed_leader_for_gpuvid,
            next_view_timeout,
            view_sync_timeout,
            num_bootstrap,
            builder_timeout,
            data_request_delay,
            builder_urls,
            start_proposing_view,
            stop_proposing_view,
            start_voting_view,
            stop_voting_view,
            start_proposing_time,
            stop_proposing_time,
            start_voting_time,
            stop_voting_time,
            epoch_height,
            epoch_start_block,
            stake_table_capacity,
            drb_difficulty,
            drb_upgrade_difficulty,
        } = v;

        Self {
            start_threshold,
            num_nodes_with_stake,
            known_nodes_with_stake,
            known_da_nodes,
            da_committees,
            da_staked_committee_size,
            fixed_leader_for_gpuvid,
            next_view_timeout,
            view_sync_timeout,
            num_bootstrap,
            builder_timeout,
            data_request_delay,
            builder_urls,
            start_proposing_view,
            stop_proposing_view,
            start_voting_view,
            stop_voting_view,
            start_proposing_time,
            stop_proposing_time,
            start_voting_time,
            stop_voting_time,
            epoch_height,
            epoch_start_block,
            stake_table_capacity,
            drb_difficulty,
            drb_upgrade_difficulty,
        }
    }
}

impl PublicHotShotConfig {
    pub fn into_hotshot_config(self) -> HotShotConfig<SeqTypes> {
        HotShotConfig {
            start_threshold: self.start_threshold,
            num_nodes_with_stake: self.num_nodes_with_stake,
            known_nodes_with_stake: self.known_nodes_with_stake,
            known_da_nodes: self.known_da_nodes,
            da_committees: self.da_committees,
            da_staked_committee_size: self.da_staked_committee_size,
            fixed_leader_for_gpuvid: self.fixed_leader_for_gpuvid,
            next_view_timeout: self.next_view_timeout,
            view_sync_timeout: self.view_sync_timeout,
            num_bootstrap: self.num_bootstrap,
            builder_timeout: self.builder_timeout,
            data_request_delay: self.data_request_delay,
            builder_urls: self.builder_urls,
            start_proposing_view: self.start_proposing_view,
            stop_proposing_view: self.stop_proposing_view,
            start_voting_view: self.start_voting_view,
            stop_voting_view: self.stop_voting_view,
            start_proposing_time: self.start_proposing_time,
            stop_proposing_time: self.stop_proposing_time,
            start_voting_time: self.start_voting_time,
            stop_voting_time: self.stop_voting_time,
            epoch_height: self.epoch_height,
            epoch_start_block: self.epoch_start_block,
            stake_table_capacity: self.stake_table_capacity,
            drb_difficulty: self.drb_difficulty,
            drb_upgrade_difficulty: self.drb_upgrade_difficulty,
        }
    }

    pub fn known_nodes_with_stake(&self) -> Vec<PeerConfig<SeqTypes>> {
        self.known_nodes_with_stake.clone()
    }

    pub fn known_da_nodes(&self) -> Vec<PeerConfig<SeqTypes>> {
        self.known_da_nodes.clone()
    }

    pub fn blocks_per_epoch(&self) -> u64 {
        self.epoch_height
    }

    pub fn epoch_start_block(&self) -> u64 {
        self.epoch_start_block
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PublicNetworkConfig {
    rounds: usize,
    indexed_da: bool,
    transactions_per_round: usize,
    manual_start_password: Option<String>,
    num_bootrap: usize,
    next_view_timeout: u64,
    view_sync_timeout: Duration,
    builder_timeout: Duration,
    data_request_delay: Duration,
    node_index: u64,
    seed: [u8; 32],
    transaction_size: usize,
    key_type_name: String,
    libp2p_config: Option<Libp2pConfig>,
    config: PublicHotShotConfig,
    cdn_marshal_address: Option<String>,
    combined_network_config: Option<CombinedNetworkConfig>,
    commit_sha: String,
    builder: BuilderType,
    random_builder: Option<RandomBuilderConfig>,
}

impl From<NetworkConfig<SeqTypes>> for PublicNetworkConfig {
    fn from(cfg: NetworkConfig<SeqTypes>) -> Self {
        Self {
            rounds: cfg.rounds,
            indexed_da: cfg.indexed_da,
            transactions_per_round: cfg.transactions_per_round,
            manual_start_password: Some("*****".into()),
            num_bootrap: cfg.num_bootrap,
            next_view_timeout: cfg.next_view_timeout,
            view_sync_timeout: cfg.view_sync_timeout,
            builder_timeout: cfg.builder_timeout,
            data_request_delay: cfg.data_request_delay,
            node_index: cfg.node_index,
            seed: cfg.seed,
            transaction_size: cfg.transaction_size,
            key_type_name: cfg.key_type_name,
            libp2p_config: cfg.libp2p_config,
            config: cfg.config.into(),
            cdn_marshal_address: cfg.cdn_marshal_address,
            combined_network_config: cfg.combined_network_config,
            commit_sha: cfg.commit_sha,
            builder: cfg.builder,
            random_builder: cfg.random_builder,
        }
    }
}

impl PublicNetworkConfig {
    pub fn into_network_config(
        self,
        my_own_validator_config: ValidatorConfig<SeqTypes>,
    ) -> anyhow::Result<NetworkConfig<SeqTypes>> {
        let node_index = self
            .config
            .known_nodes_with_stake
            .iter()
            .position(|peer| peer.stake_table_entry.stake_key == my_own_validator_config.public_key)
            .unwrap_or(0) as u64;

        Ok(NetworkConfig {
            rounds: self.rounds,
            indexed_da: self.indexed_da,
            transactions_per_round: self.transactions_per_round,
            manual_start_password: self.manual_start_password,
            num_bootrap: self.num_bootrap,
            next_view_timeout: self.next_view_timeout,
            view_sync_timeout: self.view_sync_timeout,
            builder_timeout: self.builder_timeout,
            data_request_delay: self.data_request_delay,
            node_index,
            seed: self.seed,
            transaction_size: self.transaction_size,
            key_type_name: self.key_type_name,
            libp2p_config: self.libp2p_config,
            config: self.config.into_hotshot_config(),
            cdn_marshal_address: self.cdn_marshal_address,
            combined_network_config: self.combined_network_config,
            commit_sha: self.commit_sha,
            builder: self.builder,
            random_builder: self.random_builder,
            public_keys: Vec::new(),
        })
    }

    pub fn hotshot_config(&self) -> PublicHotShotConfig {
        self.config.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::PublicNetworkConfig;

    #[test]
    fn test_deserialize_from_old_config() {
        // pulled from decaf node
        let json_str = r#"
        {
  "rounds": 100,
  "indexed_da": false,
  "transactions_per_round": 10,
  "manual_start_password": "*****",
  "num_bootrap": 5,
  "next_view_timeout": 10,
  "view_sync_timeout": {
    "secs": 2,
    "nanos": 0
  },
  "builder_timeout": {
    "secs": 10,
    "nanos": 0
  },
  "data_request_delay": {
    "secs": 2,
    "nanos": 500000000
  },
  "node_index": 1,
  "seed": [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
  "transaction_size": 100,
  "key_type_name": "jf_signature::bls_over_bn254::VerKey",
  "libp2p_config": {
    "bootstrap_nodes": []
  },
  "config": {
    "start_threshold": [100, 100],
    "num_nodes_with_stake": 100,
    "known_nodes_with_stake": [],
    "known_da_nodes": [],
    "da_staked_committee_size": 100,
    "fixed_leader_for_gpuvid": 1,
    "next_view_timeout": 12000,
    "view_sync_timeout": {
      "secs": 1,
      "nanos": 0
    },
    "num_bootstrap": 5,
    "builder_timeout": {
      "secs": 8,
      "nanos": 0
    },
    "data_request_delay": {
      "secs": 5,
      "nanos": 0
    },
    "builder_urls": [
      "https://builder.decaf.testnet.espresso.network/"
    ],
    "start_proposing_view": 0,
    "stop_proposing_view": 0,
    "start_voting_view": 0,
    "stop_voting_view": 0,
    "start_proposing_time": 0,
    "stop_proposing_time": 0,
    "start_voting_time": 0,
    "stop_voting_time": 0,
    "epoch_height": 3000,
    "epoch_start_block": 3160636
  },
  "cdn_marshal_address": null,
  "combined_network_config": null,
  "commit_sha": "",
  "builder": "Simple",
  "random_builder": null
}
        "#;
        let _public_config: PublicNetworkConfig = serde_json::from_str(json_str).unwrap();
    }
}
