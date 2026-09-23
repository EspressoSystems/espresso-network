//! Startup stake-table catchup for new-protocol nodes.
//!
//! Cliquenet only connects to validators in the current epoch's stake table
//! window (`N-1`, `N`, `N+1`). On a fresh-join or cold-restart node, no
//! consensus messages can be received until those stake tables are populated,
//! so the existing reactive catchup (triggered by an unknown-epoch proposal)
//! never fires.
//!
//! [`bootstrap_epoch_window`] drives the existing catchup machinery
//! synchronously at startup: it walks forward one epoch at a time from the
//! highest already-known epoch (loaded from persistence by `reload_stake`)
//! until peers can no longer serve the next epoch root leaf — which is the
//! point at which the live network currently is.

use std::time::Duration;

use anyhow::{Context, ensure};
use espresso_types::SeqTypes;
use hotshot::HotShotInitializer;
use hotshot_types::{
    HotShotConfig,
    data::EpochNumber,
    drb::{INITIAL_DRB_RESULT, drb_difficulty_selector},
    epoch_membership::EpochMembershipCoordinator,
    traits::{election::Membership, storage::Storage},
    utils::epoch_from_block_number,
};
use vbs::version::Version;

/// Load what this node already knows about the stake tables into the membership: the DRB
/// difficulty, the DA committees active at `version`, the first epoch, and the epoch roots and DRB
/// results persisted around the anchor.
///
/// Must run before [`bootstrap_epoch_window`], which walks forward from the first epoch.
pub(crate) async fn seed_membership<S>(
    coordinator: &EpochMembershipCoordinator<SeqTypes>,
    initializer: &HotShotInitializer<SeqTypes>,
    config: &HotShotConfig<SeqTypes>,
    version: Version,
    storage: &S,
) where
    S: Storage<SeqTypes>,
{
    coordinator.set_drb_difficulty_selector(drb_difficulty_selector(config));

    let membership = coordinator.membership();
    for da_committee in &config.da_committees {
        if version >= da_committee.start_version {
            membership.add_da_committee(
                da_committee.start_epoch.into(),
                da_committee.committee.clone(),
            );
        }
    }

    let first_epoch = EpochNumber::new(epoch_from_block_number(
        config.epoch_start_block,
        config.epoch_height,
    ));
    membership.set_first_epoch(first_epoch, INITIAL_DRB_RESULT);

    let mut start_epoch_info = initializer.start_epoch_info().to_vec();
    start_epoch_info.sort_by_key(|info| info.epoch);
    for info in &start_epoch_info {
        if let Some(block_header) = &info.block_header
            && let Err(err) = coordinator.add_epoch_root(block_header.clone()).await
        {
            tracing::error!(epoch = %info.epoch, err = %format_args!("{err:#}"), "failed to add epoch root");
        }
    }
    for info in start_epoch_info {
        membership.add_drb_result(info.epoch, info.drb_result);
    }

    let Some(high_qc_block) = initializer.high_qc().data.block_number else {
        return;
    };
    let next_epoch = EpochNumber::new(epoch_from_block_number(
        high_qc_block + 1,
        config.epoch_height,
    )) + 1;
    if let Ok(drb_result) = storage.load_drb_result(next_epoch).await
        && let Ok(stake_table) = coordinator.stake_table_for_epoch(Some(next_epoch))
    {
        stake_table.add_drb_result(drb_result);
    }
}

/// Walk forward from the highest already-known epoch until peers can no
/// longer serve the next epoch root leaf, populating the membership with
/// stake tables for every epoch up through `N+1` (where `N` is the current
/// epoch). Returns `N`.
///
/// Preconditions: `reload_stake` should have run before this — it populates
/// the membership from local persistence so the walk skips epochs we
/// already know.
pub async fn bootstrap_epoch_window(
    coordinator: &EpochMembershipCoordinator<SeqTypes>,
    epoch_height: u64,
    step_timeout: Duration,
) -> anyhow::Result<EpochNumber> {
    if epoch_height == 0 {
        // Pre-epoch chain: epochs aren't enabled yet, the non-epoch
        // committee path is what gets used.
        return Ok(EpochNumber::genesis());
    }

    let membership = coordinator.membership();
    let first_epoch = membership
        .first_epoch()
        .context("first_epoch not seeded; genesis stake table missing")?;

    // Catchup resumes from the latest consecutive pair of stake tables we
    // hold, so a gap near the tip is bridged by the step that runs into it
    // rather than having to be found here.
    let mut highest = membership.highest_known_epoch().unwrap_or(first_epoch + 1);

    tracing::info!(
        %first_epoch,
        starting_from = %highest,
        "bootstrap_epoch_window: walking forward",
    );

    // Walk forward; each successful iteration drives `add_epoch_root` via
    // the existing catchup machinery, persisting the new stake table. The
    // walk terminates when the catchup chain returns an error
    // (`Ok(Err(_))`) or a single step exceeds the hang bound (`Err(_)`).
    loop {
        let target = highest + 1;
        let result =
            tokio::time::timeout(step_timeout, coordinator.wait_for_stake_table(target)).await;
        match result {
            Ok(Ok(_)) => {
                tracing::info!(%target, "bootstrap_epoch_window: derived stake table");
                highest = target;
            },
            Ok(Err(err)) => {
                tracing::info!(
                    %target,
                    %err,
                    "bootstrap_epoch_window: catchup failed; treating as live tip",
                );
                break;
            },
            Err(_) => {
                tracing::warn!(
                    %target,
                    timeout_secs = step_timeout.as_secs(),
                    "bootstrap_epoch_window: catchup timed out; proceeding with a possibly \
                     stale epoch window",
                );
                break;
            },
        }
    }

    // `highest` corresponds to N+1 (the leaf at root_block_in_epoch(N-1) is
    // the last finalized one peers can serve). So current epoch N = highest - 1.
    let current = EpochNumber::new(highest.saturating_sub(1));

    ensure!(
        membership.snapshot(current).is_some(),
        "missing stake table for current epoch {current} after bootstrap"
    );
    ensure!(
        membership.snapshot(highest).is_some(),
        "missing stake table for next epoch {highest} after bootstrap"
    );

    tracing::info!(%current, "bootstrap_epoch_window: complete");
    Ok(current)
}
