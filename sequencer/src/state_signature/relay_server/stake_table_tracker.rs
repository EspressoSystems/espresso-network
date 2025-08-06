use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
};

use alloy::primitives::U256;
use async_lock::RwLock;
use espresso_contract_deployer::network_config::{
    fetch_epoch_config_from_sequencer, fetch_stake_table_from_sequencer,
};
use hotshot_types::{
    data::EpochNumber,
    light_client::StateVerKey,
    stake_table::one_honest_threshold,
    traits::{node_implementation::ConsensusTime, signature_key::StakeTableEntryType},
    utils::epoch_from_block_number,
};
use url::Url;

/// Stake table info for a specific epoch
#[derive(Clone, Debug, Default)]
pub struct StakeTableInfo {
    /// Minimum weight to form an available state signature bundle
    pub threshold: U256,
    /// Stake table: map(vk, weight)
    pub known_nodes: HashMap<StateVerKey, U256>,
}

/// Tracks the stake table info for each epoch
pub struct StakeTableTrackerInner {
    /// Sequencer endpoint to query for stake table info
    sequencer_url: Url,

    /// Blocks per epoch, should be initialized from the sequencer
    blocks_per_epoch: Option<u64>,

    /// Epoch start block, should be initialized from the sequencer
    epoch_start_block: Option<u64>,

    /// Stake table info for each epoch
    stake_table_infos: HashMap<u64, StakeTableInfo>,

    /// Genesis stake table info
    genesis_stake_table_info: Option<StakeTableInfo>,

    /// Queue for garbage collection
    gc_queue: BTreeSet<u64>,
}

/// Tracks the stake table info for each epoch
pub struct StakeTableTracker {
    inner: Arc<RwLock<StakeTableTrackerInner>>,
}

impl StakeTableTracker {
    pub fn new(sequencer_url: Url) -> Self {
        Self {
            inner: Arc::new(RwLock::new(StakeTableTrackerInner {
                sequencer_url,
                blocks_per_epoch: None,
                epoch_start_block: None,
                stake_table_infos: HashMap::new(),
                genesis_stake_table_info: None,
                gc_queue: BTreeSet::new(),
            })),
        }
    }

    /// Return the genesis stake table info
    pub async fn get_genesis_stake_table_info(&self) -> anyhow::Result<StakeTableInfo> {
        let read_guard = self.inner.read().await;
        if let Some(stake_table_info) = &read_guard.genesis_stake_table_info {
            return Ok(stake_table_info.clone());
        }
        drop(read_guard);
        let mut write_guard = self.inner.write().await;
        let genesis_stake_table =
            fetch_stake_table_from_sequencer(&write_guard.sequencer_url, None).await?;
        let genesis_total_stake = genesis_stake_table.total_stakes();

        if let Some(stake_table_info) = &write_guard.genesis_stake_table_info {
            return Ok(stake_table_info.clone());
        }

        let genesis_stake_table_info = StakeTableInfo {
            threshold: one_honest_threshold(genesis_total_stake),
            known_nodes: genesis_stake_table
                .into_iter()
                .map(|entry| (entry.state_ver_key, entry.stake_table_entry.stake()))
                .collect(),
        };

        write_guard.genesis_stake_table_info = Some(genesis_stake_table_info.clone());

        Ok(genesis_stake_table_info)
    }

    /// Return the stake table info for the given block height
    /// If the block height is older than the epoch start block, return the genesis stake table info
    pub async fn get_stake_table_info_for_block(
        &self,
        block_height: u64,
    ) -> anyhow::Result<StakeTableInfo> {
        let read_guard = self.inner.read().await;
        let (blocks_per_epoch, epoch_start_block) =
            if let Some(blocks_per_epoch) = read_guard.blocks_per_epoch {
                (blocks_per_epoch, read_guard.epoch_start_block.unwrap())
            } else {
                let mut write_guard = self.inner.write().await;
                if let Some(blocks_per_epoch) = write_guard.blocks_per_epoch {
                    (blocks_per_epoch, write_guard.epoch_start_block.unwrap())
                } else {
                    let (blocks_per_epoch, epoch_start_block) =
                        fetch_epoch_config_from_sequencer(&write_guard.sequencer_url).await?;
                    write_guard.blocks_per_epoch.get_or_insert(blocks_per_epoch);
                    write_guard
                        .epoch_start_block
                        .get_or_insert(epoch_start_block);
                    (blocks_per_epoch, epoch_start_block)
                }
            };
        if block_height < epoch_start_block {
            drop(read_guard);
            return self.get_genesis_stake_table_info().await;
        }
        let epoch = epoch_from_block_number(block_height, blocks_per_epoch);
        if let Some(stake_table_info) = read_guard.stake_table_infos.get(&epoch) {
            return Ok(stake_table_info.clone());
        }
        let mut write_guard = self.inner.write().await;
        if let Some(stake_table_info) = write_guard.stake_table_infos.get(&epoch) {
            return Ok(stake_table_info.clone());
        }

        let stake_table = fetch_stake_table_from_sequencer(
            &write_guard.sequencer_url,
            Some(EpochNumber::new(epoch)),
        )
        .await?;
        let total_stake = stake_table.total_stakes();

        let stake_table_info = StakeTableInfo {
            threshold: one_honest_threshold(total_stake),
            known_nodes: stake_table
                .into_iter()
                .map(|entry| (entry.state_ver_key, entry.stake_table_entry.stake()))
                .collect(),
        };

        write_guard
            .stake_table_infos
            .insert(epoch, stake_table_info.clone());
        write_guard.gc_queue.insert(epoch);
        // Remove the stake table info if it's older than 2 epochs
        while let Some(&old_epoch) = write_guard.gc_queue.first() {
            if old_epoch >= epoch - 2 {
                break;
            }
            write_guard.stake_table_infos.remove(&old_epoch);
            write_guard.gc_queue.pop_first();
            tracing::debug!(%old_epoch, "garbage collected for epoch");
        }

        Ok(stake_table_info)
    }
}
