use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
    time::Duration,
};

use alloy::primitives::U256;
use anyhow::Context;
use async_lock::RwLock;
use espresso_types::SeqTypes;
// use espresso_contract_deployer::network_config::{
//     fetch_epoch_config_from_sequencer, fetch_stake_table_from_sequencer,
// };
use hotshot_types::{
    data::EpochNumber,
    light_client::StateVerKey,
    stake_table::one_honest_threshold,
    traits::{node_implementation::ConsensusTime, signature_key::StakeTableEntryType},
    utils::epoch_from_block_number,
    PeerConfig,
};
use hotshot_types::{stake_table::HSStakeTable, traits::node_implementation::NodeType};
use tokio::time::sleep;
use url::Url;
use vbs::version::StaticVersion;

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
    stake_table_infos: HashMap<u64, Arc<StakeTableInfo>>,

    /// Genesis stake table info
    genesis_stake_table_info: Option<Arc<StakeTableInfo>>,

    /// Queue for garbage collection
    gc_queue: BTreeSet<u64>,
}

/// Number of epochs to keep the stake table info
const PRUNE_GAP: u64 = 2;

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
    pub async fn genesis_stake_table_info(&self) -> anyhow::Result<Arc<StakeTableInfo>> {
        tracing::trace!("Acquire read lock for genesis stake table info");
        let read_guard = self.inner.read().await;
        if let Some(stake_table_info) = &read_guard.genesis_stake_table_info {
            return Ok(stake_table_info.clone());
        }
        tracing::trace!("Drop read lock for genesis stake table info");
        drop(read_guard);
        tracing::trace!("Acquire write lock for genesis stake table info");
        let mut write_guard = self.inner.write().await;

        if let Some(stake_table_info) = &write_guard.genesis_stake_table_info {
            return Ok(stake_table_info.clone());
        }

        let genesis_stake_table =
            fetch_stake_table_from_sequencer(&write_guard.sequencer_url, None).await?;
        let genesis_total_stake = genesis_stake_table.total_stakes();

        tracing::debug!("Fetching genesis stake table from sequencer");
        let genesis_stake_table_info = Arc::new(StakeTableInfo {
            threshold: one_honest_threshold(genesis_total_stake),
            known_nodes: genesis_stake_table
                .into_iter()
                .map(|entry| (entry.state_ver_key, entry.stake_table_entry.stake()))
                .collect(),
        });
        tracing::debug!("Genesis stake table info updated");

        write_guard.genesis_stake_table_info = Some(genesis_stake_table_info.clone());
        tracing::trace!("Drop write lock for genesis stake table info");

        Ok(genesis_stake_table_info)
    }

    /// Return the stake table info for the given block height
    /// If the block height is older than the epoch start block, return the genesis stake table info
    pub async fn stake_table_info_for_block(
        &self,
        block_height: u64,
    ) -> anyhow::Result<Arc<StakeTableInfo>> {
        tracing::debug!("Fetch stake table for block {block_height}");

        tracing::trace!("Acquire read lock for stake table info");
        let read_guard = self.inner.read().await;
        let (blocks_per_epoch, epoch_start_block) =
            if let Some(blocks_per_epoch) = read_guard.blocks_per_epoch {
                let epoch_start_block = read_guard.epoch_start_block.unwrap();
                tracing::trace!("Drop read lock for stake table info");
                drop(read_guard);
                (blocks_per_epoch, epoch_start_block)
            } else {
                tracing::trace!("Drop read lock for stake table info");
                drop(read_guard);
                tracing::trace!("Acquire write lock for stake table info");
                let mut write_guard = self.inner.write().await;
                if let Some(blocks_per_epoch) = write_guard.blocks_per_epoch {
                    (blocks_per_epoch, write_guard.epoch_start_block.unwrap())
                } else {
                    tracing::debug!("Fetching epoch config from sequencer");
                    let (blocks_per_epoch, epoch_start_block) =
                        fetch_epoch_config_from_sequencer(&write_guard.sequencer_url).await?;
                    write_guard.blocks_per_epoch.get_or_insert(blocks_per_epoch);
                    write_guard
                        .epoch_start_block
                        .get_or_insert(epoch_start_block);
                    tracing::debug!(
                        "Fetched epoch config from sequencer: blocks_per_epoch: {}, \
                         epoch_start_block: {}",
                        blocks_per_epoch,
                        epoch_start_block
                    );
                    tracing::trace!("Drop write lock for stake table info");
                    drop(write_guard);
                    (blocks_per_epoch, epoch_start_block)
                }
            };
        if block_height <= epoch_start_block || blocks_per_epoch == 0 {
            return self.genesis_stake_table_info().await;
        }

        let epoch = epoch_from_block_number(block_height, blocks_per_epoch);
        tracing::trace!("Acquire read lock for stake table info");
        let read_guard = self.inner.read().await;
        if let Some(stake_table_info) = read_guard.stake_table_infos.get(&epoch) {
            return Ok(stake_table_info.clone());
        }
        tracing::trace!("Drop read lock for stake table info");
        drop(read_guard);
        tracing::trace!("Acquire write lock for stake table info");
        let mut write_guard = self.inner.write().await;
        if let Some(stake_table_info) = write_guard.stake_table_infos.get(&epoch) {
            return Ok(stake_table_info.clone());
        }

        tracing::debug!("Fetching stake table for epoch {} from sequencer", epoch);
        let stake_table = fetch_stake_table_from_sequencer(
            &write_guard.sequencer_url,
            Some(EpochNumber::new(epoch)),
        )
        .await?;
        let total_stake = stake_table.total_stakes();

        let stake_table_info = Arc::new(StakeTableInfo {
            threshold: one_honest_threshold(total_stake),
            known_nodes: stake_table
                .into_iter()
                .map(|entry| (entry.state_ver_key, entry.stake_table_entry.stake()))
                .collect(),
        });

        write_guard
            .stake_table_infos
            .insert(epoch, stake_table_info.clone());
        write_guard.gc_queue.insert(epoch);
        tracing::debug!("Stake table info for epoch {} updated", epoch);
        // Remove the stake table info if it's older than 2 epochs
        while let Some(&old_epoch) = write_guard.gc_queue.first() {
            if epoch < PRUNE_GAP || old_epoch >= epoch - PRUNE_GAP {
                break;
            }
            write_guard.stake_table_infos.remove(&old_epoch);
            write_guard.gc_queue.pop_first();
            tracing::debug!(%old_epoch, "garbage collected for epoch");
        }
        tracing::trace!("Drop write lock for stake table info");

        Ok(stake_table_info)
    }
}

/// Get the epoch-related  from the sequencer's `PublicHotShotConfig` struct
/// return (blocks_per_epoch, epoch_start_block)
pub async fn fetch_epoch_config_from_sequencer(sequencer_url: &Url) -> anyhow::Result<(u64, u64)> {
    // Request the configuration until it is successful
    loop {
        let url = sequencer_url.join("config/hotshot").unwrap();
        match reqwest::get(url.clone()).await {
            Ok(resp) => {
                let value: serde_json::Value = resp
                    .json()
                    .await
                    .with_context(|| format!("Failed to parse the json object from url {url}"))?;
                let blocks_per_epoch =
                    value["config"]["epoch_height"]
                        .as_u64()
                        .ok_or(anyhow::anyhow!(
                            "Failed to parse epoch_height from hotshot config"
                        ))?;
                let epoch_start_block =
                    value["config"]["epoch_start_block"]
                        .as_u64()
                        .ok_or(anyhow::anyhow!(
                            "Failed to parse epoch_start_block from hotshot config"
                        ))?;
                break Ok((blocks_per_epoch, epoch_start_block));
            },
            Err(e) => {
                tracing::error!(%url, "Failed to fetch the network config: {e}");
                sleep(Duration::from_secs(5)).await;
            },
        }
    }
}

/// Fetch the stake table from a sequencer node given the epoch number
///
/// Does not error, runs until the stake table is provided.
pub async fn fetch_stake_table_from_sequencer(
    sequencer_url: &Url,
    epoch: Option<<SeqTypes as NodeType>::Epoch>,
) -> anyhow::Result<HSStakeTable<SeqTypes>> {
    tracing::info!("Initializing stake table from node for epoch {epoch:?}");

    loop {
        match epoch {
            Some(epoch) => match surf_disco::Client::<
                tide_disco::error::ServerError,
                StaticVersion<0, 1>,
            >::new(sequencer_url.clone())
            .get::<Vec<PeerConfig<SeqTypes>>>(&format!("node/stake-table/{}", epoch.u64()))
            .send()
            .await
            {
                Ok(resp) => break Ok(resp.into()),
                Err(e) => {
                    let url = sequencer_url
                        .join(&format!("node/stake-table/{}", epoch.u64()))
                        .unwrap();
                    tracing::error!(%url, "Failed to fetch the stake table: {e}");
                    sleep(Duration::from_secs(5)).await;
                },
            },
            None => {
                let url = sequencer_url.join("config/hotshot").unwrap();
                match reqwest::get(url.clone()).await {
                    Ok(resp) => {
                        let value: serde_json::Value = resp.json().await.with_context(|| {
                            format!("Failed to parse the json object from url {url}")
                        })?;
                        let known_nodes_with_stake =
                            serde_json::from_str::<Vec<PeerConfig<SeqTypes>>>(
                                &value["config"]["known_nodes_with_stake"].to_string(),
                            )
                            .with_context(|| "Failed to parse the stake table")?;
                        break Ok(known_nodes_with_stake.into());
                    },
                    Err(e) => {
                        tracing::error!(%url, "Failed to fetch the network config: {e}");
                        sleep(Duration::from_secs(5)).await;
                    },
                }
            },
        }
    }
}
