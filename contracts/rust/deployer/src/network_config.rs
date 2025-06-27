//! Fetching hotshot network config

use std::time::Duration;

use alloy::{
    primitives::U256,
    transports::http::reqwest::{self, Url},
};
use anyhow::{Context, Result};
use espresso_types::SeqTypes;
use hotshot_contract_adapter::{
    field_to_u256,
    sol_types::{LightClientStateSol, StakeTableStateSol},
};
use hotshot_types::{
    PeerConfig,
    stake_table::HSStakeTable,
    traits::node_implementation::{ConsensusTime, NodeType},
};
use tokio::time::sleep;
use vbs::version::StaticVersion;

/// Returns both genesis light client state and stake table state
pub async fn light_client_genesis(
    sequencer_url: &Url,
    stake_table_capacity: usize,
) -> anyhow::Result<(LightClientStateSol, StakeTableStateSol)> {
    let st = fetch_stake_table_from_sequencer(sequencer_url, None)
        .await
        .with_context(|| "Failed to initialize stake table")?;
    light_client_genesis_from_stake_table(&st, stake_table_capacity)
}

/// Fetch the stake table from a sequencer node given the epoch number
///
/// Does not error, runs until the stake table is provided.
pub async fn fetch_stake_table_from_sequencer(
    sequencer_url: &Url,
    epoch: Option<<SeqTypes as NodeType>::Epoch>,
) -> Result<HSStakeTable<SeqTypes>> {
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

#[inline]
/// derive the genesis light client state and stake table state from initial set of `PeerConfig`
pub fn light_client_genesis_from_stake_table(
    st: &HSStakeTable<SeqTypes>,
    stake_table_capacity: usize,
) -> anyhow::Result<(LightClientStateSol, StakeTableStateSol)> {
    let st_state = st.commitment(stake_table_capacity)?;
    Ok((
        LightClientStateSol {
            viewNum: 0,
            blockHeight: 0,
            blockCommRoot: U256::from(0u32),
        },
        StakeTableStateSol {
            blsKeyComm: field_to_u256(st_state.bls_key_comm),
            schnorrKeyComm: field_to_u256(st_state.schnorr_key_comm),
            amountComm: field_to_u256(st_state.amount_comm),
            threshold: field_to_u256(st_state.threshold),
        },
    ))
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
