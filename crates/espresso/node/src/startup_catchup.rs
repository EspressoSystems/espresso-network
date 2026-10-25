//! Startup stake-table catchup for new-protocol nodes.
//!
//! Purpose: bring the node into the cliquenet window of the current epoch `N`, the stake tables of
//! `N-1`, `N` and `N+1`. Cliquenet dials only those validators, and until it does no consensus
//! message arrives. From there consensus and request-response over cliquenet take over, and the
//! configured state peers are not used anymore. Booting without HTTP state peers, from cliquenet
//! alone, is future work.
//!
//! Model: a stake table is the L1 stake-table events up to the cutoff L1 block in its epoch root
//! header (`l1_finalized`). The root of epoch `e-2` derives the table of epoch `e`, through
//! `add_epoch_root`. Catchup obtains verified epoch root headers, cheapest source first:
//!
//! 1. From storage: tables are loaded and re-derived by `init_node_state` before this module
//!    runs.
//! 2. LC contract catchup: headers proven by Merkle proofs against the header `F` the LC contract
//!    (the `LightClient` contract on L1) holds.
//! 3. Peer catchup: QC-verified leaves, one epoch at a time, up to the goal.
//!
//! Assumptions:
//!
//! - Epochs are always on: `epoch_height > 0`. No pre-epoch chains exist.
//! - L1 is reachable, unless a restart starts from storage because no state peer answers.
//! - State peers serve the availability and light-client APIs.
//! - State peers are trusted for liveness and block height during initial sync; everything they
//!   serve is verified (LC contract proofs, QCs).
//! - The goal is the median of the peers' reported tips: one bad peer among 3 or more cannot move
//!   it or fail the fleet's restarts. Operators configure at least 3 independent state peers.
//! - Failure at [`STARTUP_CATCHUP_DEADLINE`] exits the node. Operators restart it.
//!
//! Validator: [`catch_up_cliquenet_stake_tables`] reads the peer heights first. With a median it
//! catches up to the epoch after the median tip's, then checks `N` and `N+1`. With no height it
//! starts from the stake tables storage holds, else at genesis while the LC contract is inside the
//! seeded epochs, else fails with [`StartupCatchupError::StatePeersUnavailable`]. Consensus
//! catches up DRBs on demand.
//!
//! Follower: not on cliquenet, so its upstreams stay its source. Before it stores a block of
//! epoch `E`, it checks that the stake table of `E+1` is held, and runs [`catch_up_stake_tables`]
//! with goal `E+1` if not. No peer heights are read. Boot runs the same check for the first block
//! it will ingest, so a config or peer error exits the node at once instead of warning on every
//! poll.

use std::{collections::BTreeSet, ops::RangeInclusive, time::Duration};

use anyhow::{Context, anyhow};
use derive_more::Display;
use espresso_types::{Header, SeqTypes};
use futures::future::{join_all, try_join_all};
use hotshot_contract_adapter::{sol_types::LightClientStateSol, u256_to_field};
use hotshot_query_service::node::BlockId;
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    light_client::CircuitField,
    traits::{block_contents::BlockHeader, election::Membership},
    utils::{epoch_from_block_number, root_block_in_epoch},
};
use light_client::client::{Client, QueryServiceClient};
use tokio::time::{Instant, sleep, timeout, timeout_at};
use url::Url;
use vec1::Vec1;

/// First wait between two retries, doubled up to `MAX_BACKOFF`.
const INITIAL_BACKOFF: Duration = Duration::from_secs(1);
/// Longest wait between two retries.
const MAX_BACKOFF: Duration = Duration::from_secs(60);

/// Longest the startup catchup retries, in total for the LC contract state read, LC contract
/// catchup and peer catchup. At the deadline the node exits with an error.
pub(crate) const STARTUP_CATCHUP_DEADLINE: Duration = Duration::from_secs(15 * 60);

/// Validator boot: catch up to the median peer tip, then check `N` and `N+1`, the epochs
/// cliquenet dials before consensus starts. Returns the current epoch `N`.
///
/// When no state peer reports a height, see [`highest_epoch_without_peers`].
pub(crate) async fn catch_up_cliquenet_stake_tables<C: Client, Fut>(
    coordinator: &EpochMembershipCoordinator<SeqTypes>,
    epoch_height: u64,
    read_lc_contract_state: impl Fn() -> Fut,
    state_peers: &[(Url, C)],
    params: &StartupCatchupParams,
) -> Result<EpochNumber, StartupCatchupError>
where
    Fut: Future<Output = anyhow::Result<Option<LightClientStateSol>>>,
{
    let started = Instant::now();
    let deadline = started + params.total_timeout;
    let heights = read_peer_block_heights(state_peers, params.step_timeout).await;
    let highest_epoch = if heights.is_empty() {
        highest_epoch_without_peers(
            coordinator,
            epoch_height,
            read_lc_contract_state,
            params,
            deadline,
        )
        .await?
    } else {
        let goal = match median_peer_tip_epoch(&heights, epoch_height) {
            Some(tip_epoch) => tip_epoch + 1,
            None => first_epoch(coordinator)? + 1,
        };
        let catchup = StartupCatchup::new(coordinator, state_peers, params, epoch_height, deadline);
        catch_up_until_goal(&catchup, read_lc_contract_state, goal).await?
    };

    let current = check_cliquenet_epochs(coordinator, highest_epoch)?;
    tracing::info!(
        target: "announce::startup_catchup",
        current_epoch = %current,
        duration_ms = started.elapsed().as_millis() as u64,
        "cliquenet stake tables ready",
    );
    Ok(current)
}

/// LC contract catchup, then peer catchup up to `goal`. Returns the highest epoch with a stake
/// table; `Ok` implies the goal's epoch is held.
///
/// `read_lc_contract_state` returns the LC contract state, `None` when there is no LC contract.
/// Its failures are retried until the deadline.
pub(crate) async fn catch_up_stake_tables<C: Client, Fut>(
    coordinator: &EpochMembershipCoordinator<SeqTypes>,
    epoch_height: u64,
    read_lc_contract_state: impl Fn() -> Fut,
    state_peers: &[(Url, C)],
    params: &StartupCatchupParams,
    goal: EpochNumber,
) -> Result<EpochNumber, StartupCatchupError>
where
    Fut: Future<Output = anyhow::Result<Option<LightClientStateSol>>>,
{
    let deadline = Instant::now() + params.total_timeout;
    let catchup = StartupCatchup::new(coordinator, state_peers, params, epoch_height, deadline);
    catch_up_until_goal(&catchup, read_lc_contract_state, goal).await
}

async fn catch_up_until_goal<C: Client, Fut>(
    catchup: &StartupCatchup<'_, C>,
    read_lc_contract_state: impl Fn() -> Fut,
    goal: EpochNumber,
) -> Result<EpochNumber, StartupCatchupError>
where
    Fut: Future<Output = anyhow::Result<Option<LightClientStateSol>>>,
{
    let StartupCatchup {
        coordinator,
        params,
        blocks_per_epoch: epoch_height,
        ..
    } = *catchup;
    let membership = coordinator.membership();
    let first_epoch = first_epoch(coordinator)?;

    let held = membership.epochs_with_root();
    let mut highest_held = held.last().copied().unwrap_or(first_epoch + 1);

    let lc_contract_state =
        read_lc_contract_state_retrying(read_lc_contract_state, params, catchup.deadline).await?;
    let lc_catchup = lc_contract_state
        .and_then(|state| LcContractCatchup::missing(&state, epoch_height, first_epoch, &held));
    if let Some(lc_catchup) = lc_catchup {
        catchup.catch_up_from_lc_contract(&lc_catchup).await?;
        highest_held = highest_held.max(*lc_catchup.missing_epochs.last());
    }

    catchup.catch_up_from_peers(highest_held, goal).await?;
    Ok(highest_held.max(goal))
}

/// No state peer reported a height: the highest epoch storage derived, else genesis while the LC
/// contract is still inside the seeded epochs. Fails when the chain has progressed past them.
async fn highest_epoch_without_peers<Fut>(
    coordinator: &EpochMembershipCoordinator<SeqTypes>,
    epoch_height: u64,
    read_lc_contract_state: impl Fn() -> Fut,
    params: &StartupCatchupParams,
    deadline: Instant,
) -> Result<EpochNumber, StartupCatchupError>
where
    Fut: Future<Output = anyhow::Result<Option<LightClientStateSol>>>,
{
    if let Some(highest) = coordinator.membership().epochs_with_root().last().copied() {
        tracing::warn!(
            target: "announce::startup_catchup",
            highest_epoch = %highest,
            "no state peer reported a block height; starting from storage"
        );
        return Ok(highest);
    }
    let first_epoch = first_epoch(coordinator)?;
    let state = read_lc_contract_state_retrying(read_lc_contract_state, params, deadline).await?;
    let past_seeded_epochs = state
        .and_then(|state| lc_contract_epochs(state.blockHeight, epoch_height, first_epoch))
        .is_some();
    if past_seeded_epochs {
        return Err(StartupCatchupError::StatePeersUnavailable);
    }
    tracing::info!(
        target: "announce::startup_catchup",
        "no state peer reported a block height; starting at genesis"
    );
    Ok(first_epoch + 1)
}

/// Read the LC contract state, retrying until `deadline`.
async fn read_lc_contract_state_retrying<Fut>(
    read_lc_contract_state: impl Fn() -> Fut,
    params: &StartupCatchupParams,
    deadline: Instant,
) -> Result<Option<LightClientStateSol>, StartupCatchupError>
where
    Fut: Future<Output = anyhow::Result<Option<LightClientStateSol>>>,
{
    retry(deadline, Stage::LcContractStateRead, |_, left| {
        let read = timeout(params.step_timeout.min(left), read_lc_contract_state());
        async move { read.await.context("attempt timed out")? }
    })
    .await
}

pub(crate) fn first_epoch(
    coordinator: &EpochMembershipCoordinator<SeqTypes>,
) -> Result<EpochNumber, StartupCatchupError> {
    coordinator
        .membership()
        .first_epoch()
        .ok_or(StartupCatchupError::FirstEpochUnset)
}

/// The block height each state peer reports. One pass, each read bounded by `step_timeout`, no
/// retry. Logs each peer URL and height once. Peers that fail or time out are omitted.
async fn read_peer_block_heights<C: Client>(
    state_peers: &[(Url, C)],
    step_timeout: Duration,
) -> Vec<u64> {
    let reads = state_peers.iter().map(|(url, peer)| async move {
        let height = timeout(step_timeout, peer.block_height())
            .await
            .context("timed out")
            .and_then(|height| height);
        match &height {
            Ok(height) => {
                tracing::info!(target: "announce::startup_catchup", %url, height, "peer block height")
            },
            Err(err) => tracing::info!(
                target: "announce::startup_catchup",
                %url,
                err = format!("{err:#}"),
                "peer block height unavailable"
            ),
        }
        height.ok()
    });
    join_all(reads).await.into_iter().flatten().collect()
}

/// Check the stake tables of `highest_epoch - 1` and `highest_epoch`. Returns the current epoch
/// `N = highest_epoch - 1`.
fn check_cliquenet_epochs(
    coordinator: &EpochMembershipCoordinator<SeqTypes>,
    highest_epoch: EpochNumber,
) -> Result<EpochNumber, StartupCatchupError> {
    // `highest_epoch` is `N+1`, so `N = highest_epoch - 1`.
    let current = EpochNumber::new(highest_epoch.saturating_sub(1));
    let membership = coordinator.membership();
    for epoch in [current, highest_epoch] {
        if membership.snapshot(epoch).is_none() {
            return Err(StartupCatchupError::MissingStakeTable { epoch });
        }
    }
    Ok(current)
}

/// Time bounds of the startup catchup.
#[derive(Clone, Copy, Debug)]
pub struct StartupCatchupParams {
    /// Bound on a single LC contract catchup fetch, LC contract state read or block height read.
    pub step_timeout: Duration,
    /// How long all stages may keep retrying, together, before startup gives up.
    pub total_timeout: Duration,
}

/// Why the startup catchup failed.
#[derive(Debug, thiserror::Error)]
pub(crate) enum StartupCatchupError {
    /// The membership has no first epoch to seed from.
    #[error("first_epoch not seeded; genesis stake table missing")]
    FirstEpochUnset,

    /// No state peer is configured or answers, and storage and genesis cannot serve the chain.
    #[error("startup catchup needs a state peer serving the availability and light-client APIs")]
    StatePeersUnavailable,

    /// A retried stage ran out of time, `source` being its last failure.
    #[error("startup catchup deadline exceeded after {attempts} attempts while {stage}")]
    DeadlineExceeded {
        stage: Stage,
        attempts: usize,
        #[source]
        source: anyhow::Error,
    },

    /// An epoch cliquenet connects to has no stake table after catchup.
    #[error("missing stake table for epoch {epoch} after startup catchup")]
    MissingStakeTable { epoch: EpochNumber },
}

/// A retried stage of the startup catchup.
#[derive(Clone, Copy, Debug, Display)]
pub(crate) enum Stage {
    /// Reading the LC contract state.
    #[display("reading the LC contract state")]
    LcContractStateRead,
    /// Fetching and adding the LC contract epoch roots.
    #[display("LC contract catchup")]
    LcContractCatchup,
    /// Deriving the stake table of `epoch`, on the way to the epoch `goal`.
    #[display("peer catchup of epoch {epoch} toward epoch {goal}")]
    PeerCatchup {
        epoch: EpochNumber,
        goal: EpochNumber,
    },
}

/// One `QueryServiceClient` per URL, to read block heights and light-client proofs from.
pub(crate) fn state_peer_clients(urls: &[Url]) -> Vec<(Url, QueryServiceClient)> {
    urls.iter()
        .map(|url| (url.clone(), QueryServiceClient::new(url.clone())))
        .collect()
}

/// The epochs LC contract catchup covers, `epoch(F)-3 ..= epoch(F)+1` for the LC contract header
/// `F` at `lc_contract_height`, clamped above the two epochs seeded from `first_epoch`.
///
/// `None` when `F` is before the first epoch's end. Pure.
fn lc_contract_epochs(
    lc_contract_height: u64,
    blocks_per_epoch: u64,
    first_epoch: EpochNumber,
) -> Option<RangeInclusive<EpochNumber>> {
    let epoch = epoch_from_block_number(lc_contract_height, blocks_per_epoch);
    let start = epoch.saturating_sub(3).max(*first_epoch + 2);
    (start <= epoch + 1).then(|| EpochNumber::new(start)..=EpochNumber::new(epoch + 1))
}

/// Epoch of the last block of the median peer chain (lower median for an even count), `None` for
/// no peers or no blocks. Pure.
fn median_peer_tip_epoch(peer_block_heights: &[u64], blocks_per_epoch: u64) -> Option<EpochNumber> {
    let mut heights = peer_block_heights.to_vec();
    heights.sort_unstable();
    let median = heights.get(heights.len().checked_sub(1)? / 2)?;
    let tip = median.checked_sub(1)?;
    Some(EpochNumber::new(epoch_from_block_number(
        tip,
        blocks_per_epoch,
    )))
}

/// What every stage of the catchup shares.
struct StartupCatchup<'a, C> {
    /// Membership to fill, with the catchup machinery behind it.
    coordinator: &'a EpochMembershipCoordinator<SeqTypes>,
    /// State peers serving block heights and light-client proofs.
    state_peers: &'a [(Url, C)],
    /// Step timeout and total timeout.
    params: &'a StartupCatchupParams,
    /// Blocks in one epoch.
    blocks_per_epoch: u64,
    /// When every retry of every stage gives up.
    deadline: Instant,
}

impl<'a, C: Client> StartupCatchup<'a, C> {
    fn new(
        coordinator: &'a EpochMembershipCoordinator<SeqTypes>,
        state_peers: &'a [(Url, C)],
        params: &'a StartupCatchupParams,
        blocks_per_epoch: u64,
        deadline: Instant,
    ) -> Self {
        Self {
            coordinator,
            state_peers,
            params,
            blocks_per_epoch,
            deadline,
        }
    }

    /// Fetch the proven epoch roots of `catchup` from the peers in turn, then add them.
    ///
    /// Only the fetch is bounded by `step_timeout`: adding the roots can scan L1 events for longer,
    /// and cancelling it would restart that scan.
    async fn catch_up_from_lc_contract(
        &self,
        catchup: &LcContractCatchup,
    ) -> Result<(), StartupCatchupError> {
        if self.state_peers.is_empty() {
            return Err(StartupCatchupError::StatePeersUnavailable);
        }
        retry(
            self.deadline,
            Stage::LcContractCatchup,
            |attempt, remaining| async move {
                let (url, peer) = &self.state_peers[attempt % self.state_peers.len()];
                let headers = timeout(
                    self.params.step_timeout.min(remaining),
                    catchup.fetch_proven_epoch_roots(peer, self.blocks_per_epoch),
                )
                .await
                .context("attempt timed out")?
                .with_context(|| format!("peer {url}"))?;
                timeout_at(self.deadline, async {
                    for header in headers {
                        let height = header.height();
                        self.coordinator
                            .add_epoch_root(header)
                            .await
                            .map_err(|err| anyhow!("adding the epoch root at {height}: {err}"))?;
                    }
                    anyhow::Ok(())
                })
                .await
                .context("adding the epoch roots timed out")?
            },
        )
        .await
    }

    /// Derive one epoch at a time from `highest_held_epoch + 1` up to `goal_epoch`, each retried.
    async fn catch_up_from_peers(
        &self,
        highest_held_epoch: EpochNumber,
        goal_epoch: EpochNumber,
    ) -> Result<(), StartupCatchupError> {
        if goal_epoch > highest_held_epoch && self.state_peers.is_empty() {
            return Err(StartupCatchupError::StatePeersUnavailable);
        }
        for epoch in (*highest_held_epoch + 1..=*goal_epoch).map(EpochNumber::new) {
            let stage = Stage::PeerCatchup {
                epoch,
                goal: goal_epoch,
            };
            retry(self.deadline, stage, |_, remaining| {
                let step = timeout(remaining, self.coordinator.wait_for_stake_table(epoch));
                async move {
                    step.await
                        .context("attempt timed out")?
                        .map(drop)
                        .map_err(|err| anyhow!("{err:#}"))
                }
            })
            .await?;
            tracing::debug!(%epoch, "startup catchup: derived stake table from peers");
        }
        Ok(())
    }
}

/// Run `attempt` with its index and the time left, backing off after each failure, until it
/// succeeds or `deadline` passes.
async fn retry<T, Fut: Future<Output = anyhow::Result<T>>>(
    deadline: Instant,
    stage: Stage,
    mut attempt: impl FnMut(usize, Duration) -> Fut,
) -> Result<T, StartupCatchupError> {
    let mut backoff = INITIAL_BACKOFF;
    let mut attempts = 0;
    let mut last_err = anyhow!("the deadline passed before the first attempt");
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(StartupCatchupError::DeadlineExceeded {
                stage,
                attempts,
                source: last_err,
            });
        }
        match attempt(attempts, remaining).await {
            Ok(value) => return Ok(value),
            Err(err) => last_err = err,
        }
        attempts += 1;
        if attempts.is_power_of_two() {
            tracing::warn!(
                attempts,
                err = format!("{last_err:#}"),
                "startup catchup: {stage} failed; retrying"
            );
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        sleep(backoff.min(remaining)).await;
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }
}

/// LC contract catchup: the epoch roots below `F` that storage lacks.
#[derive(Clone, Debug, PartialEq)]
struct LcContractCatchup {
    /// Block height of the LC contract header `F`.
    lc_contract_height: u64,
    /// Block commitment root the LC contract holds for `F`.
    block_comm_root: CircuitField,
    /// LC contract epochs storage lacks, ascending: `add_epoch_root(e)` reads the table of `e - 1`.
    missing_epochs: Vec1<EpochNumber>,
}

impl LcContractCatchup {
    /// The `lc_contract_epochs` of `state` that `held` lacks, `None` when there are none.
    fn missing(
        state: &LightClientStateSol,
        blocks_per_epoch: u64,
        first_epoch: EpochNumber,
        held: &BTreeSet<EpochNumber>,
    ) -> Option<Self> {
        let epochs = lc_contract_epochs(state.blockHeight, blocks_per_epoch, first_epoch)?;
        let missing_epochs = (epochs.start().u64()..=epochs.end().u64())
            .map(EpochNumber::new)
            .filter(|epoch| !held.contains(epoch))
            .collect();
        Some(Self {
            lc_contract_height: state.blockHeight,
            block_comm_root: u256_to_field(state.blockCommRoot),
            missing_epochs: Vec1::try_from_vec(missing_epochs).ok()?,
        })
    }

    /// Fetch the roots deriving the stake tables of the missing epochs from one `peer`, in
    /// ascending order. The proofs are fetched concurrently, once the header `F` is verified.
    ///
    /// Every header is verified against header `F`, which is itself verified against the light
    /// client's commitment, so a lying peer fails here and nothing it cannot prove gets added.
    async fn fetch_proven_epoch_roots<C: Client>(
        &self,
        peer: &C,
        blocks_per_epoch: u64,
    ) -> Result<Vec<Header>, LcContractProofError> {
        let header = peer
            .header(self.lc_contract_height)
            .await
            .map_err(LcContractProofError::HeaderFetch)?;
        self.verify_lc_contract_header(&header)?;
        let root = header.block_merkle_tree_root();

        try_join_all(self.missing_epochs.iter().map(|epoch| async move {
            // The root of epoch `e - 2` derives the stake table of epoch `e`.
            let height = root_block_in_epoch(epoch.u64() - 2, blocks_per_epoch);
            let proof = peer
                .header_proof(self.lc_contract_height, BlockId::Number(height as usize))
                .await
                .map_err(|source| LcContractProofError::EpochRootFetch { height, source })?;
            let header = proof
                .verify(root)
                .map_err(|source| LcContractProofError::EpochRootInvalid { height, source })?;
            if header.height() != height {
                return Err(LcContractProofError::EpochRootHeight {
                    got: header.height(),
                    expected: height,
                });
            }
            Ok(header)
        }))
        .await
    }

    /// Accept `header` only if it is the block the LC contract commits to at `F`.
    ///
    /// The view number does not enter `block_comm_root`, so any view serves.
    fn verify_lc_contract_header(&self, header: &Header) -> Result<(), LcContractProofError> {
        if header.height() != self.lc_contract_height {
            return Err(LcContractProofError::HeaderHeight {
                got: header.height(),
                expected: self.lc_contract_height,
            });
        }
        let state = header
            .get_light_client_state(ViewNumber::genesis())
            .map_err(LcContractProofError::HeaderState)?;
        if state.block_comm_root != self.block_comm_root {
            return Err(LcContractProofError::RootMismatch {
                height: self.lc_contract_height,
            });
        }
        Ok(())
    }
}

/// Why a peer's LC contract header or epoch root proofs were refused.
#[derive(Debug, thiserror::Error)]
enum LcContractProofError {
    /// The peer did not serve the LC contract header.
    #[error("fetching the LC contract header")]
    HeaderFetch(#[source] anyhow::Error),

    /// The peer served a header of another height than the LC contract's.
    #[error("LC contract header has height {got}, expected {expected}")]
    HeaderHeight { got: u64, expected: u64 },

    /// The light-client state of the LC contract header is unreadable.
    #[error("reading the light client state of the LC contract header")]
    HeaderState(#[source] anyhow::Error),

    /// The LC contract header does not match the block commitment root the LC contract holds.
    #[error("LC contract header at height {height} does not match the block commitment root")]
    RootMismatch { height: u64 },

    /// The peer did not serve the proof of the epoch root.
    #[error("fetching the epoch root at height {height}")]
    EpochRootFetch {
        height: u64,
        #[source]
        source: anyhow::Error,
    },

    /// The proof of the epoch root fails against the LC contract header.
    #[error("verifying the epoch root at height {height}")]
    EpochRootInvalid {
        height: u64,
        #[source]
        source: anyhow::Error,
    },

    /// The proof is for another height than the epoch root requested.
    #[error("peer returned height {got} for the epoch root, expected {expected}")]
    EpochRootHeight { got: u64, expected: u64 },
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };

    use alloy::primitives::{Address, U256};
    use anyhow::ensure;
    use async_lock::Mutex as AsyncMutex;
    use async_trait::async_trait;
    use espresso_types::{
        AuthenticatedValidatorMap, ChainConfig, EpochCommittees, L1Client, StakeTableHash,
        mock::MockStateCatchup,
        traits::{EventsPersistenceRead, MembershipPersistence, StakeTuple},
        v0_3::{
            EventKey, Fetcher, IndexedStake, RegisteredValidator, RewardAmount, StakeTableEvent,
        },
    };
    use hotshot_contract_adapter::field_to_u256;
    use hotshot_example_types::storage_types::TestStorage;
    use hotshot_types::{PeerConfig, ValidatorConfig, drb::DrbResult, signature_key::BLSPubKey};
    use indexmap::IndexMap;
    use light_client::testing::{TestClient, random_validator};
    use pretty_assertions::assert_matches;
    use rstest::rstest;

    use super::*;

    const EPOCH_HEIGHT: u64 = 10;
    const FIRST_EPOCH: u64 = 1;
    /// Epoch of the LC contract header: the LC contract epochs are `N-3 ..= N+1`.
    const N: u64 = 8;
    /// In epoch `N`, past its root block (75).
    const LC_CONTRACT_HEIGHT: u64 = 77;

    const PARAMS: StartupCatchupParams = StartupCatchupParams {
        step_timeout: Duration::from_secs(5),
        total_timeout: Duration::from_secs(30),
    };

    /// LC contract catchup of every LC contract epoch over the chain of `chain()`.
    fn catchup_for(state: &LightClientStateSol) -> LcContractCatchup {
        LcContractCatchup::missing(
            state,
            EPOCH_HEIGHT,
            EpochNumber::new(FIRST_EPOCH),
            &BTreeSet::new(),
        )
        .unwrap()
    }

    #[rstest]
    #[case::decaf_lc_contract_at_root(3000, 3000 * 1000 - 5, Some(997..=1001))]
    #[case::decaf_after_root(3000, 3000 * 1000 - 4, Some(997..=1001))]
    #[case::decaf_epoch_end(3000, 3000 * 1000, Some(997..=1001))]
    #[case::decaf_next_epoch(3000, 3000 * 1000 + 1, Some(998..=1002))]
    #[case::mainnet_lc_contract_at_root(40000, 40000 * 100 - 5, Some(97..=101))]
    #[case::mainnet_next_epoch(40000, 40000 * 100 + 1, Some(98..=102))]
    #[case::start_is_seeded(100, 100 * 4, Some(3..=5))]
    #[case::start_clamped(100, 100 * 5, Some(3..=6))]
    #[case::start_unclamped(100, 100 * 6, Some(3..=7))]
    #[case::window_only(100, 100 * 2, Some(3..=3))]
    #[case::before_first_epoch_root(100, 50, None)]
    #[case::genesis(100, 0, None)]
    fn lc_contract_epochs_cases(
        #[case] blocks_per_epoch: u64,
        #[case] lc_contract_height: u64,
        #[case] expected: Option<RangeInclusive<u64>>,
    ) {
        // `first_epoch = 1` seeds epochs 1 and 2.
        let epochs = lc_contract_epochs(
            lc_contract_height,
            blocks_per_epoch,
            EpochNumber::new(FIRST_EPOCH),
        )
        .map(|epochs| epochs.start().u64()..=epochs.end().u64());
        assert_eq!(epochs, expected);
    }

    #[rstest]
    #[case::no_peers(&[], None)]
    #[case::one_empty_chain(&[0], None)]
    #[case::all_empty_chains(&[0, 0, 0], None)]
    #[case::odd_count(&[78, 18, 38], Some(4))]
    #[case::even_count_lower_median(&[18, 98, 38, 78], Some(4))]
    #[case::one_high_outlier(&[78, 10_000, 78], Some(8))]
    #[case::one_empty_outlier(&[0, 78, 78], Some(8))]
    fn median_peer_tip_epoch_cases(#[case] heights: &[u64], #[case] expected: Option<u64>) {
        let epoch = median_peer_tip_epoch(heights, EPOCH_HEIGHT).map(|epoch| epoch.u64());
        assert_eq!(epoch, expected);
    }

    /// Test chain of `LC_CONTRACT_HEIGHT + 1` blocks and the LC contract state over it.
    ///
    /// Peers report that height, so peer catchup targets epoch `N + 1`.
    async fn chain() -> (TestClient, LightClientStateSol) {
        let client = TestClient::with_epoch_height(EPOCH_HEIGHT);
        let header = client.header(LC_CONTRACT_HEIGHT).await.unwrap();
        let root = header
            .get_light_client_state(ViewNumber::genesis())
            .unwrap()
            .block_comm_root;
        let state = LightClientStateSol {
            blockHeight: header.height(),
            blockCommRoot: field_to_u256(root),
            ..Default::default()
        };
        (client, state)
    }

    #[tokio::test]
    async fn lc_contract_header_verification() {
        let (client, state) = chain().await;
        let catchup = catchup_for(&state);
        let header = client.header(LC_CONTRACT_HEIGHT).await.unwrap();
        catchup.verify_lc_contract_header(&header).unwrap();

        let wrong_root = LcContractCatchup {
            block_comm_root: catchup.block_comm_root + CircuitField::from(1u64),
            ..catchup.clone()
        };
        let err = wrong_root.verify_lc_contract_header(&header).unwrap_err();
        assert_matches!(
            err,
            LcContractProofError::RootMismatch { height } if height == LC_CONTRACT_HEIGHT
        );

        let wrong_height = LcContractCatchup {
            lc_contract_height: LC_CONTRACT_HEIGHT + 1,
            ..catchup
        };
        let err = wrong_height.verify_lc_contract_header(&header).unwrap_err();
        assert_matches!(
            err,
            LcContractProofError::HeaderHeight { got, expected }
                if got == LC_CONTRACT_HEIGHT && expected == LC_CONTRACT_HEIGHT + 1
        );
    }

    /// Stands in for the node's database: it holds a stake table, DRB result and epoch root header
    /// for each epoch the test lets it answer for, with the hash a verified epoch root header
    /// commits to.
    #[derive(Clone, Debug, Default)]
    struct SeededStore {
        tables: Arc<std::sync::Mutex<HashMap<u64, StakeTuple>>>,
        roots: Arc<std::sync::Mutex<HashMap<u64, Header>>>,
    }

    impl SeededStore {
        async fn for_chain(client: &TestClient, epochs: RangeInclusive<u64>) -> Self {
            let validators: AuthenticatedValidatorMap = (0..4)
                .map(|_| random_validator())
                .map(|validator| (validator.account, validator))
                .collect();
            let mut tables = HashMap::new();
            let mut headers = HashMap::new();
            for epoch in epochs {
                // The root of epoch `e - 1` commits to the stake table of epoch `e`.
                let root = root_block_in_epoch(epoch - 1, EPOCH_HEIGHT);
                let hash = client.header(root).await.unwrap().next_stake_table_hash();
                tables.insert(
                    epoch,
                    (validators.clone(), Some(RewardAmount::default()), hash),
                );
                // The root of epoch `e - 2` is the one `add_epoch_root` derives `e` from.
                let root = root_block_in_epoch(epoch - 2, EPOCH_HEIGHT);
                headers.insert(epoch, client.header(root).await.unwrap());
            }
            Self {
                tables: Arc::new(std::sync::Mutex::new(tables)),
                roots: Arc::new(std::sync::Mutex::new(headers)),
            }
        }
    }

    #[async_trait]
    impl MembershipPersistence for SeededStore {
        async fn load_stake(&self, epoch: EpochNumber) -> anyhow::Result<Option<StakeTuple>> {
            Ok(self.tables.lock().unwrap().get(&epoch.u64()).cloned())
        }

        async fn load_latest_stake(
            &self,
            _limit: u64,
        ) -> anyhow::Result<Option<Vec<IndexedStake>>> {
            let tables = self.tables.lock().unwrap();
            let stakes = tables
                .iter()
                .map(|(epoch, (validators, reward, hash))| {
                    let stake = (validators.clone(), *reward);
                    (EpochNumber::new(*epoch), stake, *hash)
                })
                .collect();
            Ok(Some(stakes))
        }

        async fn load_drb_result(&self, epoch: EpochNumber) -> anyhow::Result<Option<DrbResult>> {
            Ok(self
                .tables
                .lock()
                .unwrap()
                .contains_key(&epoch.u64())
                .then_some([0; 32]))
        }

        async fn load_epoch_root(&self, epoch: EpochNumber) -> anyhow::Result<Option<Header>> {
            Ok(self.roots.lock().unwrap().get(&epoch.u64()).cloned())
        }

        async fn store_epoch_root(&self, _: EpochNumber, _: Header) -> anyhow::Result<()> {
            Ok(())
        }

        async fn store_stake(
            &self,
            _: EpochNumber,
            _: AuthenticatedValidatorMap,
            _: Option<RewardAmount>,
            _: Option<StakeTableHash>,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn store_events(
            &self,
            _: u64,
            _: Vec<(EventKey, StakeTableEvent)>,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn load_events(
            &self,
            _: u64,
            _: u64,
        ) -> anyhow::Result<(
            Option<EventsPersistenceRead>,
            Vec<(EventKey, StakeTableEvent)>,
        )> {
            Ok((None, vec![]))
        }

        async fn delete_stake_tables(&self) -> anyhow::Result<()> {
            Ok(())
        }

        async fn store_all_validators(
            &self,
            _: EpochNumber,
            _: IndexMap<Address, RegisteredValidator<BLSPubKey>>,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn load_all_validators(
            &self,
            _: EpochNumber,
            _: u64,
            _: u64,
        ) -> anyhow::Result<Vec<RegisteredValidator<BLSPubKey>>> {
            Ok(vec![])
        }
    }

    /// A real `EpochCommittees` and coordinator over `store`, seeded like a fresh node. No peer
    /// serves epoch roots, so peer catchup ends at the first epoch above the LC contract ones.
    fn coordinator(store: SeededStore) -> EpochMembershipCoordinator<SeqTypes> {
        coordinator_over(committees(store))
    }

    fn coordinator_over(committees: EpochCommittees) -> EpochMembershipCoordinator<SeqTypes> {
        EpochMembershipCoordinator::new(
            committees,
            EPOCH_HEIGHT,
            &TestStorage::<SeqTypes>::default(),
        )
    }

    fn committees(store: SeededStore) -> EpochCommittees {
        let fetcher = Fetcher::new(
            Arc::new(MockStateCatchup::default()),
            Arc::new(AsyncMutex::new(store)),
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
        let committees =
            EpochCommittees::new_stake(peers.clone(), peers, None, fetcher, EPOCH_HEIGHT);
        committees.set_first_epoch(EpochNumber::new(FIRST_EPOCH), [0u8; 32]);
        committees
    }

    async fn fresh_coordinator(client: &TestClient) -> EpochMembershipCoordinator<SeqTypes> {
        coordinator(SeededStore::for_chain(client, (N - 3)..=(N + 1)).await)
    }

    fn peer_list(clients: &[TestClient]) -> Vec<(Url, TestClient)> {
        let url: Url = "http://peer.invalid".parse().unwrap();
        clients.iter().map(|c| (url.clone(), c.clone())).collect()
    }

    async fn run_catchup(
        coordinator: &EpochMembershipCoordinator<SeqTypes>,
        lc_contract_state: Option<LightClientStateSol>,
        state_peers: &[TestClient],
        params: StartupCatchupParams,
    ) -> Result<EpochNumber, StartupCatchupError> {
        catch_up_cliquenet_stake_tables(
            coordinator,
            EPOCH_HEIGHT,
            || async { Ok(lc_contract_state.clone()) },
            &peer_list(state_peers),
            &params,
        )
        .await
    }

    fn has_table(coordinator: &EpochMembershipCoordinator<SeqTypes>, epoch: u64) -> bool {
        coordinator
            .membership()
            .snapshot(EpochNumber::new(epoch))
            .is_some()
    }

    /// Hold `epochs` in the membership, as `load_stake_table` leaves them.
    async fn hold(coordinator: &EpochMembershipCoordinator<SeqTypes>, epochs: RangeInclusive<u64>) {
        for epoch in epochs {
            let loaded = coordinator
                .membership()
                .load_stake_table(EpochNumber::new(epoch))
                .await;
            assert!(loaded, "epoch {epoch}");
        }
    }

    fn has_epoch_root(coordinator: &EpochMembershipCoordinator<SeqTypes>, epoch: u64) -> bool {
        coordinator
            .membership()
            .epochs_with_root()
            .contains(&EpochNumber::new(epoch))
    }

    #[test_log::test(tokio::test)]
    async fn fresh_join_catches_up_from_lc_contract() {
        let (client, lc_contract_state) = chain().await;
        let coordinator = fresh_coordinator(&client).await;

        let current = run_catchup(&coordinator, Some(lc_contract_state), &[client], PARAMS)
            .await
            .unwrap();

        // Peer catchup finds no root above the LC contract state, as when the prover keeps up.
        assert_eq!(current, EpochNumber::new(N));
        for epoch in (N - 3)..=(N + 1) {
            assert!(has_table(&coordinator, epoch), "epoch {epoch}");
        }
        assert_eq!(
            coordinator.membership().epochs_with_root().last(),
            Some(&EpochNumber::new(N + 1))
        );
    }

    #[test_log::test(tokio::test)]
    async fn lc_contract_catchup_rotates_past_a_wrong_chain() {
        let (client, lc_contract_state) = chain().await;
        let (other, _) = chain().await;
        let coordinator = fresh_coordinator(&client).await;

        let current = run_catchup(
            &coordinator,
            Some(lc_contract_state),
            &[other, client],
            PARAMS,
        )
        .await
        .unwrap();

        assert_eq!(current, EpochNumber::new(N));
        assert!(has_table(&coordinator, N + 1));
    }

    /// An epoch root the peer cannot serve at first must be retried, not read as the tip.
    #[test_log::test(tokio::test)]
    async fn lc_contract_catchup_retries_a_transient_failure() {
        let (client, lc_contract_state) = chain().await;
        let coordinator = fresh_coordinator(&client).await;
        let flaky_root = root_block_in_epoch(N - 1, EPOCH_HEIGHT) as usize;
        client.forget_leaf(flaky_root).await;
        let heal = client.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            heal.remember_leaf(flaky_root).await;
        });

        let current = run_catchup(&coordinator, Some(lc_contract_state), &[client], PARAMS)
            .await
            .unwrap();

        assert_eq!(current, EpochNumber::new(N));
        assert!(has_table(&coordinator, N + 1));
    }

    #[test_log::test(tokio::test)]
    async fn lc_contract_header_unavailable_aborts_at_the_deadline() {
        let (client, lc_contract_state) = chain().await;
        let coordinator = fresh_coordinator(&client).await;
        client.forget_leaf(LC_CONTRACT_HEIGHT as usize).await;

        let params = StartupCatchupParams {
            total_timeout: Duration::from_millis(300),
            ..PARAMS
        };
        let err = run_catchup(&coordinator, Some(lc_contract_state), &[client], params)
            .await
            .unwrap_err();

        let StartupCatchupError::DeadlineExceeded {
            stage: Stage::LcContractCatchup,
            source,
            ..
        } = err
        else {
            panic!("expected the LC contract catchup deadline, got {err:?}");
        };
        assert_matches!(
            source.downcast_ref::<LcContractProofError>(),
            Some(LcContractProofError::HeaderFetch(_))
        );
    }

    /// Held epochs skip LC contract catchup, and peer catchup starts from the highest held epoch.
    #[test_log::test(tokio::test)]
    async fn restart_boots_from_storage() {
        let (client, lc_contract_state) = chain().await;
        let store = SeededStore::for_chain(&client, (N - 4)..=(N + 1)).await;
        let mut committees = committees(store);
        committees.reload_stake(10).await;
        let coordinator = coordinator_over(committees);
        client.forget_leaf(LC_CONTRACT_HEIGHT as usize).await;

        let current = run_catchup(
            &coordinator,
            Some(lc_contract_state),
            std::slice::from_ref(&client),
            PARAMS,
        )
        .await
        .unwrap();

        assert_eq!(current, EpochNumber::new(N));
    }

    /// No peer height and stored epochs start from storage, without reading the LC contract.
    #[test_log::test(tokio::test)]
    async fn no_peer_height_continues_from_the_held_epochs() {
        let (client, _) = chain().await;
        let store = SeededStore::for_chain(&client, (N - 4)..=(N + 1)).await;
        let mut committees = committees(store);
        committees.reload_stake(10).await;
        let coordinator = coordinator_over(committees);
        client.fail_block_height().await;
        let reads = AtomicUsize::new(0);

        let current = catch_up_cliquenet_stake_tables(
            &coordinator,
            EPOCH_HEIGHT,
            || async {
                reads.fetch_add(1, Ordering::Relaxed);
                Ok(None)
            },
            &peer_list(&[client]),
            &PARAMS,
        )
        .await
        .unwrap();

        assert_eq!(current, EpochNumber::new(N));
        assert_eq!(reads.load(Ordering::Relaxed), 0);
    }

    /// Nothing derived: the LC contract inside the seeded epochs (epoch 1 ends at block 10) starts
    /// at genesis, one block further the chain is past them and storage cannot serve it.
    #[rstest]
    #[case::no_lc_contract(None, true)]
    #[case::in_seeded_epochs(Some(EPOCH_HEIGHT), true)]
    #[case::past_seeded_epochs(Some(EPOCH_HEIGHT + 1), false)]
    #[test_log::test(tokio::test)]
    async fn no_peer_height_and_nothing_derived(
        #[case] lc_height: Option<u64>,
        #[case] starts_at_genesis: bool,
    ) {
        let (client, _) = chain().await;
        let coordinator = coordinator(SeededStore::default());
        client.fail_block_height().await;
        let state = lc_height.map(|height| LightClientStateSol {
            blockHeight: height,
            ..Default::default()
        });

        let result = run_catchup(&coordinator, state, &[client], PARAMS).await;

        if starts_at_genesis {
            assert_eq!(result.unwrap(), EpochNumber::new(FIRST_EPOCH));
        } else {
            assert_matches!(result, Err(StartupCatchupError::StatePeersUnavailable));
        }
    }

    #[test_log::test(tokio::test)]
    async fn all_zero_peer_heights_start_at_genesis() {
        let (client, _) = chain().await;
        let coordinator = coordinator(SeededStore::default());
        client.mock_block_height(0).await;

        let current = run_catchup(&coordinator, None, &[client], PARAMS)
            .await
            .unwrap();

        assert_eq!(current, EpochNumber::new(FIRST_EPOCH));
    }

    #[test_log::test(tokio::test)]
    async fn no_peer_height_and_failing_lc_read_aborts_at_the_deadline() {
        let (client, _) = chain().await;
        let coordinator = coordinator(SeededStore::default());
        client.fail_block_height().await;
        let params = StartupCatchupParams {
            total_timeout: Duration::from_millis(300),
            ..PARAMS
        };

        let err = catch_up_cliquenet_stake_tables(
            &coordinator,
            EPOCH_HEIGHT,
            || async { Err(anyhow!("L1 not ready")) },
            &peer_list(&[client]),
            &params,
        )
        .await
        .unwrap_err();

        assert_matches!(
            err,
            StartupCatchupError::DeadlineExceeded {
                stage: Stage::LcContractStateRead,
                ..
            }
        );
    }

    #[test_log::test(tokio::test)]
    async fn no_state_peers_configured_continues_from_the_held_epochs() {
        let (client, lc_contract_state) = chain().await;
        let store = SeededStore::for_chain(&client, (N - 4)..=(N + 1)).await;
        let mut committees = committees(store);
        committees.reload_stake(10).await;
        let coordinator = coordinator_over(committees);

        let current = run_catchup(&coordinator, Some(lc_contract_state), &[], PARAMS)
            .await
            .unwrap();

        assert_eq!(current, EpochNumber::new(N));
    }

    /// A genesis boot has nothing to catch up, so it needs no state peers.
    #[test_log::test(tokio::test)]
    async fn genesis_boot_without_state_peers() {
        let coordinator = coordinator(SeededStore::default());
        let no_peers: &[(Url, TestClient)] = &[];

        let highest = catch_up_stake_tables(
            &coordinator,
            EPOCH_HEIGHT,
            || async { Ok(None) },
            no_peers,
            &PARAMS,
            EpochNumber::new(FIRST_EPOCH + 1),
        )
        .await
        .unwrap();

        assert_eq!(highest, EpochNumber::new(FIRST_EPOCH + 1));
    }

    /// A failed peer catchup step is retried until the goal epoch is held, not read as the tip.
    #[test_log::test(tokio::test)]
    async fn peer_catchup_retries_each_step_until_the_goal() {
        let (client, _) = chain().await;
        let store = SeededStore::for_chain(&client, (N - 4)..=(N + 1)).await;
        let withheld = store.tables.lock().unwrap().remove(&(N - 1)).unwrap();
        let coordinator = coordinator(store.clone());
        for epoch in [N - 4, N - 3] {
            coordinator
                .membership()
                .load_stake_table(EpochNumber::new(epoch))
                .await;
        }
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            store.tables.lock().unwrap().insert(N - 1, withheld);
        });

        let highest = catch_up_stake_tables(
            &coordinator,
            EPOCH_HEIGHT,
            || async { Ok(Some(LightClientStateSol::default())) },
            &peer_list(&[client]),
            &PARAMS,
            EpochNumber::new(N + 1),
        )
        .await
        .unwrap();

        assert_eq!(highest, EpochNumber::new(N + 1));
        assert!(has_table(&coordinator, N + 1));
    }

    /// A peer catchup step that keeps failing ends startup at the deadline.
    #[test_log::test(tokio::test)]
    async fn peer_catchup_fails_at_the_deadline() {
        let (client, _) = chain().await;
        let store = SeededStore::for_chain(&client, (N - 4)..=(N + 1)).await;
        store.tables.lock().unwrap().remove(&(N - 1));
        let coordinator = coordinator(store);
        for epoch in [N - 4, N - 3] {
            coordinator
                .membership()
                .load_stake_table(EpochNumber::new(epoch))
                .await;
        }
        let params = StartupCatchupParams {
            total_timeout: Duration::from_millis(300),
            ..PARAMS
        };

        let err = run_catchup(
            &coordinator,
            Some(LightClientStateSol::default()),
            &[client],
            params,
        )
        .await
        .unwrap_err();

        assert_matches!(
            err,
            StartupCatchupError::DeadlineExceeded {
                stage: Stage::PeerCatchup { epoch, .. },
                ..
            } if epoch == EpochNumber::new(N - 1)
        );
    }

    /// A bad proof after the first good ones fails the whole fetch.
    #[test_log::test(tokio::test)]
    async fn invalid_proof_mid_range_fails() {
        let (client, lc_contract_state) = chain().await;
        let catchup = catchup_for(&lc_contract_state);
        client
            .return_invalid_proof(root_block_in_epoch(N - 3, EPOCH_HEIGHT) as usize)
            .await;

        let err = catchup
            .fetch_proven_epoch_roots(&client, EPOCH_HEIGHT)
            .await
            .unwrap_err();

        assert_matches!(err, LcContractProofError::EpochRootInvalid { .. });
    }

    /// A restart after an interrupted LC contract catchup holds a lone table at the start of the
    /// epochs, and the catchup completes them.
    #[test_log::test(tokio::test)]
    async fn partial_lc_contract_catchup_is_completed() {
        let (client, lc_contract_state) = chain().await;
        let coordinator = fresh_coordinator(&client).await;
        let first_root = client
            .header(root_block_in_epoch(N - 5, EPOCH_HEIGHT))
            .await;
        coordinator
            .add_epoch_root(first_root.unwrap())
            .await
            .unwrap();
        assert!(has_table(&coordinator, N - 3));
        assert!(!has_table(&coordinator, N - 4));

        let current = run_catchup(
            &coordinator,
            Some(lc_contract_state),
            std::slice::from_ref(&client),
            PARAMS,
        )
        .await
        .unwrap();

        assert_eq!(current, EpochNumber::new(N));
        for epoch in (N - 3)..=(N + 1) {
            assert!(has_table(&coordinator, epoch), "epoch {epoch}");
        }
    }

    /// A held epoch without its root, above the rooted ones, does not move where peer catchup
    /// starts.
    #[test_log::test(tokio::test)]
    async fn rootless_top_epoch_does_not_raise_the_start() {
        let (client, lc_contract_state) = chain().await;
        client.mock_block_height(LC_CONTRACT_HEIGHT + 1).await;
        let store = SeededStore::for_chain(&client, (N - 3)..=(N + 2)).await;
        store.roots.lock().unwrap().remove(&(N + 2));
        let coordinator = coordinator(store);
        hold(&coordinator, (N - 3)..=(N + 2)).await;
        assert!(has_table(&coordinator, N + 2));
        assert!(!has_epoch_root(&coordinator, N + 2));

        let current = run_catchup(&coordinator, Some(lc_contract_state), &[client], PARAMS)
            .await
            .unwrap();

        assert_eq!(current, EpochNumber::new(N));
    }

    /// A failed lc_contract_state read is retried and LC contract catchup then runs.
    #[test_log::test(tokio::test)]
    async fn lc_contract_state_failure_is_retried() {
        let (client, lc_contract_state) = chain().await;
        let coordinator = fresh_coordinator(&client).await;
        let reads = AtomicUsize::new(0);

        let current = catch_up_cliquenet_stake_tables(
            &coordinator,
            EPOCH_HEIGHT,
            || async {
                ensure!(reads.fetch_add(1, Ordering::Relaxed) > 0, "L1 not ready");
                Ok(Some(lc_contract_state.clone()))
            },
            &peer_list(&[client]),
            &PARAMS,
        )
        .await
        .unwrap();

        assert_eq!(reads.load(Ordering::Relaxed), 2);
        assert_eq!(current, EpochNumber::new(N));
        assert!(has_table(&coordinator, N + 1));
    }
}
