//! The state blocks are validated and applied against: the genesis file, L1, and the stake tables
//! membership derives from both.

use std::{num::NonZeroU64, sync::Arc, time::Duration};

use alloy::primitives::{Address, U256};
use anyhow::Context;
use async_lock::Mutex;
use espresso_genesis::{Genesis, L1Finalized};
use espresso_types::{
    EpochCommittees, EpochRewardsCalculator, L1BlockInfo, L1Client, L1ClientOptions, NodeState,
    RECENT_STAKE_TABLES_LIMIT, ValidatedState,
    traits::{MembershipPersistence, StateCatchup},
    v0_3::Fetcher,
};
use hotshot_types::{
    epoch_membership::EpochMembershipCoordinator,
    network::NetworkConfig,
    traits::{metrics::Metrics, storage::Storage},
};
use moka::future::Cache;
use url::Url;

use crate::SeqTypes;

/// The node state of the chain `genesis` starts, with stake tables kept in `store` and read from
/// the L1 at `l1`. Waits for L1 to finalize the chain's L1 genesis block.
///
/// A consensus node keeps its stake tables in its consensus store and catches up from API
/// services; an API service keeps them in its chain store and follows the chain itself.
pub async fn node_state<S>(
    genesis: &Genesis,
    network_config: &NetworkConfig<SeqTypes>,
    l1: Vec<Url>,
    store: S,
    state_catchup: Arc<dyn StateCatchup>,
    metrics: &dyn Metrics,
) -> anyhow::Result<NodeState>
where
    S: MembershipPersistence + Storage<SeqTypes> + Clone,
{
    // Before the L1 work below, which can wait a long time for L1.
    let first_epoch = genesis.first_epoch()?;
    let epoch_height = genesis.epoch_height.unwrap_or_default();
    let epoch_length =
        NonZeroU64::new(epoch_height).context("genesis needs a positive epoch_height")?;

    let l1_client = L1ClientOptions::default()
        .with_metrics(metrics)
        .connect(l1)
        .context("connecting to L1")?;
    genesis.validate_fee_contract(&l1_client).await?;
    l1_client.spawn_tasks().await;
    let l1_genesis = l1_genesis(genesis, &l1_client).await;
    tracing::info!(?l1_genesis, "L1 genesis found");

    let mut genesis_state = ValidatedState {
        chain_config: genesis.header.chain_config.into(),
        ..Default::default()
    };
    for (address, amount) in &genesis.accounts {
        tracing::warn!(%address, %amount, "prefunding an account");
        genesis_state.prefund_account(*address, *amount);
    }

    let fetcher = Fetcher::new(
        state_catchup.clone(),
        Arc::new(Mutex::new(store.clone())),
        l1_client.clone(),
        genesis.chain_config,
    );
    prefetch_stake_table_events(
        &fetcher,
        &l1_client,
        &l1_genesis,
        genesis.chain_config.stake_table_contract,
    )
    .await?;
    fetcher.spawn_update_loop().await;
    let block_reward = fetcher.fetch_fixed_block_reward().await.ok();
    let mut membership = EpochCommittees::new(
        network_config.config.known_nodes_with_stake.clone(),
        network_config.config.known_da_nodes.clone(),
        block_reward,
        fetcher,
        epoch_height,
        first_epoch,
    )?;
    membership.reload_stake(RECENT_STAKE_TABLES_LIMIT).await;
    let coordinator = EpochMembershipCoordinator::new(membership, epoch_length, &store);

    Ok(NodeState {
        chain_config: genesis.chain_config,
        genesis_chain_config: genesis.header.chain_config,
        l1_client,
        genesis_header: genesis.header.clone(),
        genesis_state,
        l1_genesis: Some(l1_genesis),
        node_id: network_config.node_index,
        upgrades: genesis.upgrades.clone(),
        current_version: genesis.base_version,
        epoch_height: Some(epoch_height),
        state_catchup,
        coordinator,
        genesis_version: genesis.genesis_version,
        epoch_start_block: genesis.epoch_start_block.unwrap_or_default(),
        epoch_rewards_calculator: Arc::new(Mutex::new(EpochRewardsCalculator::new())),
        light_client_contract_address: Cache::builder().max_capacity(1).build(),
        token_contract_address: Cache::builder().max_capacity(1).build(),
        finalized_hotshot_height: Cache::builder()
            .max_capacity(1)
            .time_to_live(Duration::from_secs(30))
            .build(),
    })
}

/// The L1 block the chain starts from, once L1 has finalized it.
async fn l1_genesis(genesis: &Genesis, l1: &L1Client) -> L1BlockInfo {
    match &genesis.l1_finalized {
        L1Finalized::Block(block) => *block,
        L1Finalized::Number { number } => l1.wait_for_finalized_block(*number).await,
        L1Finalized::Timestamp { timestamp } => {
            l1.wait_for_finalized_block_with_timestamp(U256::from(timestamp.unix_timestamp()))
                .await
        },
    }
}

/// Reads the stake table contract's whole history once, so the first stake tables do not each
/// read it.
async fn prefetch_stake_table_events(
    fetcher: &Fetcher,
    l1: &L1Client,
    l1_genesis: &L1BlockInfo,
    stake_table_contract: Option<Address>,
) -> anyhow::Result<()> {
    let Some(contract) = stake_table_contract else {
        return Ok(());
    };
    let finalized = l1
        .wait_for_latest_finalized_at_least(l1_genesis.number)
        .await;
    tracing::info!(%contract, to_block = finalized.number, "prefetching stake table events");
    let events = fetcher
        .fetch_and_store_stake_table_events(contract, finalized.number)
        .await
        .context("prefetching stake table events")?;
    tracing::info!(events = events.len(), "prefetched stake table events");
    Ok(())
}
