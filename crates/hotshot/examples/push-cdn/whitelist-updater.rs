// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! The whitelist is an adaptor that is able to update the allowed public keys for
//! all brokers. Right now, we do this by asking the orchestrator for the list of
//! allowed public keys. In the future, we will pull the stake table from the L1.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use cdn_broker::reexports::discovery::{DiscoveryClient, Embedded, Redis};
use clap::Parser;
use espresso_types::SeqTypes;
use hotshot_types::{traits::signature_key::SignatureKey, PeerConfig};
use sequencer::api::data_source::StakeTableWithEpochNumber;
use tokio::{task::JoinSet, time::timeout};
use tracing::{error, warn};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The query node endpoints that we will fetch the stake table from.
    /// We will use the highest epoch number from all of the nodes. These nodes
    /// need to be trusted.
    #[arg(short, long)]
    query_node_urls: Vec<String>,

    /// The CDN database endpoint (including scheme) to connect to.
    /// With the local discovery feature, this is a file path.
    /// With the remote (redis) discovery feature, this is a redis URL (e.g. `redis://127.0.0.1:6789`).
    #[arg(short, long)]
    database_endpoint: String,

    /// Whether or not to use the local database client
    #[arg(short, long)]
    local_database: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse the command line arguments
    let args = Args::parse();

    // If no query node URLs are provided, stop here
    if args.query_node_urls.is_empty() {
        error!("No query node URLs provided, stopping");
        return Ok(());
    }

    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create a join set to fetch the stake table from all of the query nodes
    let mut join_set = JoinSet::new();

    // Spawn a task to get the stake table from each query node
    for query_node_url in args.query_node_urls {
        join_set.spawn(async move {
            // Get the current stake table from the node
            let stake_table_and_epoch_number = timeout(
                Duration::from_secs(10),
                get_current_stake_table(&query_node_url),
            )
            .await
            .with_context(|| "timed out while fetching stake table from query node")?
            .with_context(|| "failed to fetch stake table from query node")?;

            // Extract the stake table and epoch number
            let (stake_table, epoch_number) = (
                stake_table_and_epoch_number.stake_table,
                stake_table_and_epoch_number.epoch,
            );

            anyhow::Ok((query_node_url, stake_table, epoch_number))
        });
    }

    // Collect the stake tables and epoch numbers from each query node
    let mut all_results = BTreeMap::new();
    while let Some(result) = join_set.join_next().await {
        // Extract the task's result
        match result {
            Ok(Ok((query_node_url, stake_table, epoch_number))) => {
                // Add the stake table, epoch number, and query node URL to the list
                all_results.insert(epoch_number, (query_node_url, stake_table));
            },
            Ok(Err(e)) => {
                warn!("Failed to fetch stake table from query node: {:?}", e);
                continue;
            },
            Err(e) => {
                warn!("Failed to join on task: {:?}", e);
                continue;
            },
        };
    }

    // Return early if there were no successful results
    if all_results.is_empty() {
        error!("No successful results from query nodes, stopping");
        return Ok(());
    }

    // Get the result from the node to respond with the highest epoch number
    let (epoch_number, (query_node_url, mut stake_table)) = all_results.pop_last().unwrap();

    // If there is an epoch number, we should fetch the next epoch and combine the two lists. If not,
    // epochs are not enabled yet, so just use the list we got
    if let Some(epoch_number) = epoch_number {
        // Fetch the stake table for the next epoch
        let next_epoch_stake_table = get_stake_table(&query_node_url, *epoch_number + 1)
            .await
            .with_context(|| "failed to fetch stake table for the next epoch")?;

        // Merge the tables and deduplicate the keys
        stake_table.extend(next_epoch_stake_table);
        stake_table.sort_by_key(|peer| peer.stake_table_entry.stake_key);
        stake_table.dedup_by_key(|peer| peer.stake_table_entry.stake_key);
    }

    // Extrapolate the state_ver_keys from the config and convert them to a compatible format
    let whitelist = stake_table
        .iter()
        .map(|k| Arc::from(k.stake_table_entry.stake_key.to_bytes()))
        .collect();

    // Update the whitelist in the DB depending on whether we are using a local or remote DB
    if args.local_database {
        <Embedded as DiscoveryClient>::new(args.database_endpoint, None)
            .await?
            .set_whitelist(whitelist)
            .await?;
    } else {
        <Redis as DiscoveryClient>::new(args.database_endpoint, None)
            .await?
            .set_whitelist(whitelist)
            .await?;
    }

    Ok(())
}

/// Get the current stake table
async fn get_current_stake_table(
    query_node_url: &str,
) -> Result<StakeTableWithEpochNumber<SeqTypes>> {
    // Fetch the current stake table
    let response = reqwest::get(format!("{}/v0/node/stake-table/current", query_node_url))
        .await
        .with_context(|| "failed to fetch stake table")?;

    // Parse the response
    response
        .json()
        .await
        .with_context(|| "failed to parse stake table")
}

/// Get the stake table for a specific epoch
async fn get_stake_table(
    query_node_url: &str,
    epoch_number: u64,
) -> Result<Vec<PeerConfig<SeqTypes>>> {
    // Fetch the stake table for the given epoch number
    let response = reqwest::get(format!(
        "{}/v0/node/stake-table/{}",
        query_node_url, epoch_number
    ))
    .await
    .with_context(|| "failed to fetch stake table")?;

    // Parse the response
    response
        .json()
        .await
        .with_context(|| "failed to parse stake table")
}
