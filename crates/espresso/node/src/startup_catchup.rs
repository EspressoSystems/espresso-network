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

use anyhow::ensure;
use espresso_types::SeqTypes;
use hotshot_types::{
    data::EpochNumber, epoch_membership::EpochMembershipCoordinator, traits::election::Membership,
};

/// Walk forward from the highest already-known epoch until peers can no
/// longer serve the next epoch root leaf, populating the membership with
/// stake tables for every epoch up through `N+1` (where `N` is the current
/// epoch), then acquire the DRB results of `N` and `N+1`. Returns `N`.
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
    let first_epoch = membership.first_epoch();

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

    tokio::join!(
        acquire_drb(coordinator, current, step_timeout),
        acquire_drb(coordinator, highest, step_timeout),
    );

    tracing::info!(%current, "bootstrap_epoch_window: complete");
    Ok(current)
}

/// Acquire `epoch`'s DRB result, from storage, peers or by computing it.
///
/// The walk only derives stake tables, and consensus resolves leaders from
/// its first view. A leader lookup that finds no DRB fails and starts catchup
/// for it, so a failure here costs the node its first views, not its startup.
async fn acquire_drb(
    coordinator: &EpochMembershipCoordinator<SeqTypes>,
    epoch: EpochNumber,
    step_timeout: Duration,
) {
    match tokio::time::timeout(step_timeout, coordinator.ensure_leader_membership(epoch)).await {
        Ok(Ok(_)) => tracing::info!(%epoch, "bootstrap_epoch_window: acquired drb result"),
        Ok(Err(err)) => tracing::warn!(
            %epoch,
            %err,
            "bootstrap_epoch_window: failed to acquire drb result",
        ),
        Err(_) => tracing::warn!(
            %epoch,
            timeout_secs = step_timeout.as_secs(),
            "bootstrap_epoch_window: acquiring drb result timed out",
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use alloy::primitives::U256;
    use async_lock::Mutex;
    use espresso_types::{
        ChainConfig, EpochCommittees, L1Client,
        traits::{MembershipPersistence, PersistenceOptions, SequencerPersistence},
        v0_3::Fetcher,
    };
    use hotshot_types::{PeerConfig, ValidatorConfig};
    use tempfile::TempDir;

    use super::*;
    use crate::{catchup::NullStateCatchup, persistence::fs};

    const EPOCH_HEIGHT: u64 = 100;

    /// The walk loads stake tables without their DRB results, and consensus
    /// needs the current epoch's DRB to resolve a leader in its first view.
    ///
    /// Storage holds tables for epochs 2 to 4 and peers serve nothing, so the
    /// walk stops at 4 and the current epoch is 3.
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn bootstrap_acquires_drb_of_current_and_next_epoch() {
        let dir = TempDir::new().unwrap();
        let persistence = fs::Options::new(dir.path().into()).create().await.unwrap();
        for epoch in (2..=4).map(EpochNumber::new) {
            persistence
                .store_stake(epoch, Default::default(), None, None)
                .await
                .unwrap();
            persistence
                .store_drb_result(epoch, [*epoch as u8; 32])
                .await
                .unwrap();
        }
        let coordinator = coordinator(persistence);
        let current = EpochNumber::new(3);
        let next = EpochNumber::new(4);
        assert!(coordinator.snapshot(current).is_none());

        let bootstrapped =
            bootstrap_epoch_window(&coordinator, EPOCH_HEIGHT, Duration::from_secs(10))
                .await
                .unwrap();

        assert_eq!(bootstrapped, current);
        assert!(coordinator.membership_for_epoch(current).is_ok());
        assert!(coordinator.membership_for_epoch(next).is_ok());
    }

    fn coordinator(persistence: fs::Persistence) -> EpochMembershipCoordinator<SeqTypes> {
        let fetcher = Fetcher::new(
            Arc::new(NullStateCatchup::default()),
            Arc::new(Mutex::new(persistence.clone())),
            L1Client::new(vec!["http://localhost:3331".parse().unwrap()]).unwrap(),
            ChainConfig::default(),
        );
        let peers: Vec<PeerConfig<SeqTypes>> = (0..4)
            .map(|i| {
                ValidatorConfig::<SeqTypes>::generated_from_seed_indexed(
                    [42u8; 32],
                    i,
                    U256::from(100),
                    true,
                )
                .public_config()
            })
            .collect();
        let committees = EpochCommittees::new(
            peers.clone(),
            peers,
            None,
            fetcher,
            EPOCH_HEIGHT,
            EpochNumber::genesis(),
        )
        .unwrap();
        EpochMembershipCoordinator::new(committees, EPOCH_HEIGHT, &Arc::new(persistence))
    }
}
