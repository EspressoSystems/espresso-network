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
//!
//! See the design discussion at `/home/brendon/.claude/plans/we-are-working-on-breezy-tower.md`.

use anyhow::{Context, ensure};
use espresso_types::SeqTypes;
use hotshot_types::{
    data::EpochNumber, epoch_membership::EpochMembershipCoordinator, traits::election::Membership,
};

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
) -> anyhow::Result<EpochNumber> {
    if epoch_height == 0 {
        // Pre-epoch chain: epochs aren't enabled yet, the non-epoch
        // committee path is what gets used.
        return Ok(EpochNumber::genesis());
    }

    let membership = coordinator.membership();
    let first_epoch = membership
        .read()
        .await
        .first_epoch()
        .context("first_epoch not seeded; genesis stake table missing")?;

    // Find the highest contiguous pair `(H, H-1)` already in memory. Both
    // are needed as the starting point of the forward walk: `add_epoch_root`
    // for epoch `K+2` requires the stake table at `K`, so to derive both
    // `H+1` (needs `H-1`) and `H+2` (needs `H`) we need `H` and `H-1`
    // present. If only `H` is present (e.g. `set_first_epoch` ran without a
    // matching reload, or persistence has gaps near the tip), the walk's
    // first iteration would otherwise fall into a deep walk-back that may
    // be unfillable from peers and would silently terminate the bootstrap
    // at a stale epoch.
    //
    // `set_first_epoch` always seeds `first_epoch` and `first_epoch + 1`,
    // so the scan terminates at worst at `first_epoch + 1`.
    let mut highest = {
        let m = membership.read().await;
        let initial = m.highest_known_epoch().unwrap_or(first_epoch + 1);
        let mut h = initial;
        while h > first_epoch + 1 && !(m.has_stake_table(h) && m.has_stake_table(h - 1)) {
            h = h - 1;
        }
        h
    };

    tracing::info!(
        %first_epoch,
        starting_from = %highest,
        "bootstrap_epoch_window: walking forward",
    );

    // Walk forward; each successful iteration drives `add_epoch_root` via
    // the existing catchup machinery, persisting the new stake table.
    loop {
        let target = highest + 1;
        match coordinator.wait_for_stake_table(target).await {
            Ok(_) => {
                tracing::info!(%target, "bootstrap_epoch_window: derived stake table");
                highest = target;
            },
            Err(err) => {
                tracing::info!(
                    %target,
                    %err,
                    "bootstrap_epoch_window: peers exhausted; treating as live tip",
                );
                break;
            },
        }
    }

    // `highest` corresponds to N+1 (the leaf at root_block_in_epoch(N-1) is
    // the last finalized one peers can serve). So current epoch N = highest - 1.
    let current = if *highest >= 1 {
        EpochNumber::new(highest.saturating_sub(1))
    } else {
        highest
    };

    let m = membership.read().await;
    ensure!(
        m.has_stake_table(current),
        "missing stake table for current epoch {current} after bootstrap"
    );
    ensure!(
        m.has_stake_table(highest),
        "missing stake table for next epoch {highest} after bootstrap"
    );

    tracing::info!(%current, "bootstrap_epoch_window: complete");
    Ok(current)
}
