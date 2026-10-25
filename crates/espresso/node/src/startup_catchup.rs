//! Startup stake-table catchup for new-protocol nodes.
//!
//! Purpose: bring the node to the stake tables of `N-1`, `N` and `N+1`, the epochs cliquenet
//! dials, for the current epoch `N`. Until cliquenet dials them no consensus message arrives. From
//! there consensus and request-response over cliquenet take over, and the configured state peers
//! are not used anymore. Booting without HTTP state peers, from cliquenet alone, is future work.
//!
//! Model: a stake table is the L1 stake-table events up to the cutoff L1 block in its epoch root
//! header (`l1_finalized`). The root of epoch `e-2` derives the table of epoch `e`, through
//! `add_epoch_root`. Catchup obtains verified epoch root headers, cheapest source first:
//!
//! 1. From storage: tables are loaded and re-derived by `init_node_state` before this module
//!    runs.
//! 2. LC contract catchup: headers proven by Merkle proofs against the header `F` the LC contract
//!    (the `LightClient` contract on L1) holds.
//! 3. Derivation: QC-verified leaves from the state peers, one epoch at a time.
//!
//! Assumptions:
//!
//! - Epochs are always on: `epoch_height > 0`. No pre-epoch chains exist.
//! - L1 is reachable, unless a restart starts from storage because no state peer answers.
//! - State peers serve the availability and light-client APIs.
//! - State peers are the list the membership's catchup fetcher uses; `StatePeersUnavailable` is
//!   judged on it.
//! - State peers are trusted for liveness and block height during initial sync; everything they
//!   serve is verified (LC contract proofs, QCs).
//! - `N` comes from the median of the peers' reported tips: one bad peer among 3 or more cannot
//!   move it or fail the fleet's restarts. Operators configure at least 3 independent state peers.
//! - Block rewards of the epochs cliquenet dials come from the LC contract range (five epochs, the
//!   two topmost complete) or from contiguous derivation above it.
//! - Failure at [`STARTUP_CATCHUP_DEADLINE`] exits the node. Operators restart it.
//!
//! Validator: [`catch_up_cliquenet_stake_tables`] reads the peer heights first. [`current_epoch`]
//! gives `N` from the median tip, else from storage. Without either, the LC contract header
//! decides: inside the seeded epochs the node starts at genesis, else it fails with
//! [`StartupCatchupError::StatePeersUnavailable`]. Consensus catches up DRBs on demand.
//!
//! Follower: not on cliquenet, so its upstreams stay its source. Before it stores a block it runs
//! [`ensure_stake_tables`] for [`stake_table_needed_by_block`]; the held check inside makes this
//! free per block. Boot runs it for the first block it will ingest.

use std::{ops::RangeInclusive, time::Duration};

use anyhow::{Context, anyhow};
use derive_more::Display;
use espresso_types::{Header, LcContractHeader, SeqTypes};
use futures::future::{join_all, try_join_all};
use hotshot_query_service::node::BlockId;
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
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

/// Longest the startup catchup retries, in total for the LC contract header read, LC contract
/// catchup and derivation. At the deadline the node exits with an error.
pub(crate) const STARTUP_CATCHUP_DEADLINE: Duration = Duration::from_secs(15 * 60);

/// Validator boot. Returns `N`; the tables of `N-1 ..= N+1` are held.
///
/// Peer heights first. `current_epoch` gives `N` from the median tip, else from storage. Without
/// either, the LC contract header decides: inside the seeded epochs the node starts at genesis,
/// else `StatePeersUnavailable`.
pub(crate) async fn catch_up_cliquenet_stake_tables<C: Client, Fut>(
    coordinator: &EpochMembershipCoordinator<SeqTypes>,
    read_lc_contract_header: impl Fn() -> Fut,
    state_peers: &[(Url, C)],
    params: &StartupCatchupParams,
) -> Result<EpochNumber, StartupCatchupError>
where
    Fut: Future<Output = anyhow::Result<Option<LcContractHeader>>>,
{
    let started = Instant::now();
    let catchup = StartupCatchup::new(coordinator, state_peers, params, read_lc_contract_header)?;
    let heights = read_peer_block_heights(state_peers, params.step_timeout).await;
    let median_tip = median_peer_tip_epoch(&heights, catchup.epoch_height);
    let highest_rooted = coordinator.membership().highest_rooted_epoch();
    let current = match current_epoch(median_tip, highest_rooted) {
        Some(current) => {
            if heights.is_empty() {
                tracing::warn!(
                    target: "announce::startup_catchup",
                    current_epoch = %current,
                    "no state peer reported a block height; starting from storage"
                );
            }
            current
        },
        None => catchup.current_epoch_without_evidence(&heights).await?,
    };
    catchup.ensure(cliquenet_epochs(current)).await?;
    tracing::info!(
        target: "announce::startup_catchup",
        current_epoch = %current,
        duration_ms = started.elapsed().as_millis() as u64,
        "cliquenet stake tables ready",
    );
    Ok(current)
}

/// [`StartupCatchup::ensure`] with a fresh deadline.
pub(crate) async fn ensure_stake_tables<C: Client, Fut>(
    coordinator: &EpochMembershipCoordinator<SeqTypes>,
    read_lc_contract_header: impl Fn() -> Fut,
    state_peers: &[(Url, C)],
    params: &StartupCatchupParams,
    need: RangeInclusive<EpochNumber>,
) -> Result<(), StartupCatchupError>
where
    Fut: Future<Output = anyhow::Result<Option<LcContractHeader>>>,
{
    StartupCatchup::new(coordinator, state_peers, params, read_lc_contract_header)?
        .ensure(need)
        .await
}

/// The table a block at `height` needs held before it is stored: the epoch after the block's.
/// Pure.
pub(crate) fn stake_table_needed_by_block(height: u64, epoch_height: u64) -> EpochNumber {
    EpochNumber::new(epoch_from_block_number(height, epoch_height) + 1)
}

/// Where the chain is: the median tip's epoch, else `highest_rooted - 1`. `None` without either.
/// Pure.
fn current_epoch(
    median_tip: Option<EpochNumber>,
    highest_rooted: Option<EpochNumber>,
) -> Option<EpochNumber> {
    median_tip.or_else(|| highest_rooted.map(|epoch| EpochNumber::new(epoch.saturating_sub(1))))
}

/// Whether a node with no other evidence starts at genesis: `epoch(F) <= first`, so no epoch above
/// the seeded ones is derivable from `F` yet. No header counts as inside. Pure.
fn lc_contract_header_inside_seeded_epochs(
    header: Option<&LcContractHeader>,
    first: EpochNumber,
    epoch_height: u64,
) -> bool {
    header.is_none_or(|header| epoch_from_block_number(header.height, epoch_height) <= *first)
}

/// `N-1 ..= N+1`, the epochs cliquenet dials. Pure.
fn cliquenet_epochs(current: EpochNumber) -> RangeInclusive<EpochNumber> {
    EpochNumber::new(current.saturating_sub(1))..=current + 1
}

/// Epochs provable from `F` worth installing for `top_needed`: `max(first + 2, top - 4) ..= top`
/// with `top = min(top_needed, epoch(F) + 1)`. Empty when `top < first + 2`. Pure.
fn lc_contract_epochs(
    header: &LcContractHeader,
    top_needed: EpochNumber,
    first: EpochNumber,
    epoch_height: u64,
) -> RangeInclusive<EpochNumber> {
    let top = top_needed
        .u64()
        .min(epoch_from_block_number(header.height, epoch_height) + 1);
    let start = (*first + 2).max(top.saturating_sub(4));
    EpochNumber::new(start)..=EpochNumber::new(top)
}

/// The epochs of `range`, ascending.
fn epochs(range: &RangeInclusive<EpochNumber>) -> impl Iterator<Item = EpochNumber> {
    (range.start().u64()..=range.end().u64()).map(EpochNumber::new)
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

/// What every stage shares. `first_epoch` is resolved once: the only `FirstEpochUnset` site.
struct StartupCatchup<'a, C, R> {
    /// Membership to fill, with the catchup machinery behind it.
    coordinator: &'a EpochMembershipCoordinator<SeqTypes>,
    /// State peers serving block heights and light-client proofs.
    state_peers: &'a [(Url, C)],
    /// Step timeout and total timeout.
    params: &'a StartupCatchupParams,
    /// Reads the LC contract header, `None` without an LC contract.
    read_lc_contract_header: R,
    /// Blocks in one epoch.
    epoch_height: u64,
    /// First epoch of the chain; it and the next are seeded from genesis.
    first_epoch: EpochNumber,
    /// When every retry of every stage gives up.
    deadline: Instant,
}

impl<'a, C, R, Fut> StartupCatchup<'a, C, R>
where
    C: Client,
    R: Fn() -> Fut,
    Fut: Future<Output = anyhow::Result<Option<LcContractHeader>>>,
{
    fn new(
        coordinator: &'a EpochMembershipCoordinator<SeqTypes>,
        state_peers: &'a [(Url, C)],
        params: &'a StartupCatchupParams,
        read_lc_contract_header: R,
    ) -> Result<Self, StartupCatchupError> {
        let first_epoch = coordinator
            .membership()
            .first_epoch()
            .ok_or(StartupCatchupError::FirstEpochUnset)?;
        Ok(Self {
            coordinator,
            state_peers,
            params,
            read_lc_contract_header,
            epoch_height: coordinator.epoch_height().u64(),
            first_epoch,
            deadline: Instant::now() + params.total_timeout,
        })
    }

    /// `N` for a node whose peers gave no median tip and whose storage holds no rooted epoch:
    /// `first` when the LC contract header is inside the seeded epochs, else
    /// `StatePeersUnavailable`.
    async fn current_epoch_without_evidence(
        &self,
        heights: &[u64],
    ) -> Result<EpochNumber, StartupCatchupError> {
        let header = self.read_lc_contract_header_retrying().await?;
        let inside = lc_contract_header_inside_seeded_epochs(
            header.as_ref(),
            self.first_epoch,
            self.epoch_height,
        );
        if !inside {
            return Err(StartupCatchupError::StatePeersUnavailable);
        }
        if heights.is_empty() {
            tracing::info!(
                target: "announce::startup_catchup",
                "no state peer reported a block height; starting at genesis"
            );
        }
        Ok(self.first_epoch)
    }

    /// In memory or storage.
    async fn held(&self, epoch: EpochNumber) -> bool {
        self.coordinator.membership().load_stake_table(epoch).await
    }

    /// The epochs of `range` that are [held](Self::held), ascending.
    async fn held_epochs(&self, range: &RangeInclusive<EpochNumber>) -> Vec<EpochNumber> {
        let mut held = Vec::new();
        for epoch in epochs(range) {
            if self.held(epoch).await {
                held.push(epoch);
            }
        }
        held
    }

    /// `Ok` iff every derivable epoch of `need` is held, `max(need.start, first + 2)` and above.
    /// Epochs below are seeded or before the chain.
    ///
    /// 1. Every derivable epoch held: return without I/O.
    /// 2. `StatePeersUnavailable` when one is missing and there are no state peers.
    /// 3. Read the LC contract header, retried. With one, install its `lc_contract_epochs` for the
    ///    derivable range, minus the held ones, from the state peers.
    /// 4. `derive_stake_table` each derivable epoch still missing, ascending.
    async fn ensure(&self, need: RangeInclusive<EpochNumber>) -> Result<(), StartupCatchupError> {
        let derivable =
            EpochNumber::new(need.start().u64().max(*self.first_epoch + 2))..=*need.end();
        if self.held_epochs(&derivable).await.len() == epochs(&derivable).count() {
            return Ok(());
        }
        if self.state_peers.is_empty() {
            return Err(StartupCatchupError::StatePeersUnavailable);
        }
        if let Some(header) = self.read_lc_contract_header_retrying().await? {
            self.install_lc_contract_epochs(&header, &derivable).await?;
        }
        for epoch in epochs(&derivable) {
            if !self.held(epoch).await {
                self.derive_stake_table(epoch).await?;
            }
        }
        Ok(())
    }

    /// `retry` until the deadline, each attempt bounded by `step_timeout`.
    async fn read_lc_contract_header_retrying(
        &self,
    ) -> Result<Option<LcContractHeader>, StartupCatchupError> {
        retry(self.deadline, Stage::LcContractHeaderRead, |_, left| {
            let read = timeout(
                self.params.step_timeout.min(left),
                (self.read_lc_contract_header)(),
            );
            async move { read.await.context("attempt timed out")? }
        })
        .await
    }

    /// Fetch the proven epoch roots of the `lc_contract_epochs` of `header` for `derivable` that
    /// are not held, from the peers in turn, then add them. The missing set is recomputed per
    /// attempt. Returns at once when nothing is missing.
    ///
    /// The fetch is bounded by `step_timeout`. Adding a root can scan L1 events for longer than a
    /// step, and cancelling restarts the scan.
    async fn install_lc_contract_epochs(
        &self,
        header: &LcContractHeader,
        derivable: &RangeInclusive<EpochNumber>,
    ) -> Result<(), StartupCatchupError> {
        let range = lc_contract_epochs(
            header,
            *derivable.end(),
            self.first_epoch,
            self.epoch_height,
        );
        retry(
            self.deadline,
            Stage::LcContractCatchup,
            |attempt, remaining| {
                let range = &range;
                async move {
                    let held = self.held_epochs(range).await;
                    let Some(catchup) = LcContractCatchup::missing(header, range, &held) else {
                        return Ok(());
                    };
                    let (url, peer) = &self.state_peers[attempt % self.state_peers.len()];
                    let headers = timeout(
                        self.params.step_timeout.min(remaining),
                        catchup.fetch_proven_epoch_roots(peer, self.epoch_height),
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
                                .map_err(|err| {
                                    anyhow!("adding the epoch root at {height}: {err}")
                                })?;
                        }
                        anyhow::Ok(())
                    })
                    .await
                    .context("adding the epoch roots timed out")?
                }
            },
        )
        .await
    }

    /// `retry` around `coordinator.wait_for_stake_table(epoch)`, each attempt bounded by the time
    /// left.
    ///
    /// A timed-out attempt drops its await and the attempt entries it owns. The fetch task the
    /// coordinator spawned for the current step keeps running, and the next attempt may fetch the
    /// same epoch again.
    async fn derive_stake_table(&self, epoch: EpochNumber) -> Result<(), StartupCatchupError> {
        retry(
            self.deadline,
            Stage::Derivation { epoch },
            |_, remaining| {
                let step = timeout(remaining, self.coordinator.wait_for_stake_table(epoch));
                async move {
                    step.await
                        .context("attempt timed out")?
                        .map(drop)
                        .map_err(|err| anyhow!("{err:#}"))
                }
            },
        )
        .await?;
        tracing::debug!(%epoch, "startup catchup: derived stake table from peers");
        Ok(())
    }
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

/// Time bounds of the startup catchup.
#[derive(Clone, Copy, Debug)]
pub struct StartupCatchupParams {
    /// Bound on a single LC contract catchup fetch, LC contract header read or block height read.
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
}

/// A retried stage of the startup catchup.
#[derive(Clone, Copy, Debug, Display)]
pub(crate) enum Stage {
    /// Reading the LC contract header.
    #[display("reading the LC contract header")]
    LcContractHeaderRead,
    /// Fetching and adding the LC contract epoch roots.
    #[display("LC contract catchup")]
    LcContractCatchup,
    /// Deriving the stake table of `epoch`.
    #[display("deriving the stake table of epoch {epoch}")]
    Derivation { epoch: EpochNumber },
}

/// One `QueryServiceClient` per URL, to read block heights and light-client proofs from.
pub(crate) fn state_peer_clients(urls: &[Url]) -> Vec<(Url, QueryServiceClient)> {
    urls.iter()
        .map(|url| (url.clone(), QueryServiceClient::new(url.clone())))
        .collect()
}

/// LC contract catchup: the proven epoch roots to fetch for `missing_epochs`.
#[derive(Clone, Debug, PartialEq)]
struct LcContractCatchup {
    /// The LC contract header `F` the roots are proven against.
    header: LcContractHeader,
    /// LC contract epochs not held, ascending: `add_epoch_root(e)` reads the table of `e-1` for the
    /// block time and `e-3` for the reward.
    missing_epochs: Vec1<EpochNumber>,
}

impl LcContractCatchup {
    /// The epochs of `range` that `held` lacks, `None` when there are none.
    fn missing(
        header: &LcContractHeader,
        range: &RangeInclusive<EpochNumber>,
        held: &[EpochNumber],
    ) -> Option<Self> {
        let missing_epochs = epochs(range)
            .filter(|epoch| !held.contains(epoch))
            .collect();
        Some(Self {
            header: header.clone(),
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
            .header(self.header.height)
            .await
            .map_err(LcContractProofError::HeaderFetch)?;
        self.verify_lc_contract_header(&header)?;
        let root = header.block_merkle_tree_root();

        try_join_all(self.missing_epochs.iter().map(|epoch| async move {
            // The root of epoch `e - 2` derives the stake table of epoch `e`.
            let height = root_block_in_epoch(epoch.u64() - 2, blocks_per_epoch);
            let proof = peer
                .header_proof(self.header.height, BlockId::Number(height as usize))
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
        if header.height() != self.header.height {
            return Err(LcContractProofError::HeaderHeight {
                got: header.height(),
                expected: self.header.height,
            });
        }
        let state = header
            .get_light_client_state(ViewNumber::genesis())
            .map_err(LcContractProofError::HeaderState)?;
        if state.block_comm_root != self.header.block_comm_root {
            return Err(LcContractProofError::RootMismatch {
                height: self.header.height,
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
        collections::{HashMap, HashSet},
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
    use hotshot_example_types::storage_types::TestStorage;
    use hotshot_types::{
        PeerConfig, ValidatorConfig, drb::DrbResult, light_client::CircuitField,
        signature_key::BLSPubKey,
    };
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

    /// `PARAMS` with a deadline that tests of failing stages reach quickly.
    const SHORT_DEADLINE_PARAMS: StartupCatchupParams = StartupCatchupParams {
        step_timeout: Duration::from_millis(100),
        total_timeout: Duration::from_millis(300),
    };

    /// LC contract catchup of every LC contract epoch over the chain of `chain()`.
    fn catchup_for(header: &LcContractHeader) -> LcContractCatchup {
        let epochs = lc_contract_epochs(
            header,
            EpochNumber::new(N + 1),
            EpochNumber::new(FIRST_EPOCH),
            EPOCH_HEIGHT,
        );
        LcContractCatchup::missing(header, &epochs, &[]).unwrap()
    }

    /// `first_epoch = 1` seeds epochs 1 and 2.
    #[rstest]
    #[case::decaf_at_root(3000, 3000 * 1000 - 5, 1001, Some(997..=1001))]
    #[case::decaf_epoch_end(3000, 3000 * 1000, 1001, Some(997..=1001))]
    #[case::decaf_next_epoch(3000, 3000 * 1000 + 1, 1002, Some(998..=1002))]
    #[case::mainnet_at_root(40000, 40000 * 100 - 5, 101, Some(97..=101))]
    #[case::mainnet_next_epoch(40000, 40000 * 100 + 1, 102, Some(98..=102))]
    #[case::needed_below_seeded(100, 100 * 8, 2, None)]
    #[case::needed_is_epoch_of_lc_contract_plus_one(100, 100 * 8, 9, Some(5..=9))]
    #[case::needed_above_epoch_of_lc_contract_plus_one(100, 100 * 8, 12, Some(5..=9))]
    #[case::needed_far_below_lc_contract(100, 100 * 50, 10, Some(6..=10))]
    #[case::start_clamped_above_seeded(100, 100 * 5, 6, Some(3..=6))]
    #[case::lc_contract_in_first_epoch(100, 100, 5, None)]
    #[case::lc_contract_in_second_epoch(100, 100 * 2, 5, Some(3..=3))]
    #[case::lc_contract_at_genesis(100, 0, 5, None)]
    fn lc_contract_epochs_cases(
        #[case] blocks_per_epoch: u64,
        #[case] lc_contract_height: u64,
        #[case] top_needed: u64,
        #[case] expected: Option<RangeInclusive<u64>>,
    ) {
        let header = LcContractHeader {
            height: lc_contract_height,
            block_comm_root: CircuitField::from(0u64),
        };
        let epochs = lc_contract_epochs(
            &header,
            EpochNumber::new(top_needed),
            EpochNumber::new(FIRST_EPOCH),
            blocks_per_epoch,
        );
        let epochs = (!epochs.is_empty()).then(|| epochs.start().u64()..=epochs.end().u64());
        assert_eq!(epochs, expected);
    }

    #[rstest]
    #[case::genesis_block(0, 2)]
    #[case::first_block(1, 2)]
    #[case::last_block_of_epoch(10, 2)]
    #[case::first_block_of_next_epoch(11, 3)]
    #[case::mid_epoch(95, 11)]
    fn stake_table_needed_by_block_cases(#[case] height: u64, #[case] expected: u64) {
        let epoch = stake_table_needed_by_block(height, EPOCH_HEIGHT);
        assert_eq!(epoch, EpochNumber::new(expected));
    }

    #[rstest]
    #[case::median_wins_over_storage(Some(8), Some(12), Some(8))]
    #[case::median_without_storage(Some(3), None, Some(3))]
    #[case::storage_only(None, Some(9), Some(8))]
    #[case::storage_at_epoch_zero(None, Some(0), Some(0))]
    #[case::no_evidence(None, None, None)]
    fn current_epoch_cases(
        #[case] median_tip: Option<u64>,
        #[case] highest_rooted: Option<u64>,
        #[case] expected: Option<u64>,
    ) {
        let current = current_epoch(
            median_tip.map(EpochNumber::new),
            highest_rooted.map(EpochNumber::new),
        );
        assert_eq!(current, expected.map(EpochNumber::new));
    }

    #[rstest]
    #[case::mid_chain(5, 4..=6)]
    #[case::first_epoch(1, 0..=2)]
    #[case::epoch_zero(0, 0..=1)]
    fn cliquenet_epochs_cases(#[case] current: u64, #[case] expected: RangeInclusive<u64>) {
        let epochs = cliquenet_epochs(EpochNumber::new(current));
        assert_eq!(epochs.start().u64()..=epochs.end().u64(), expected);
    }

    /// `first_epoch = 1`, epoch 1 ends at block 10.
    #[rstest]
    #[case::no_lc_contract(None, true)]
    #[case::genesis(Some(0), true)]
    #[case::end_of_first_epoch(Some(EPOCH_HEIGHT), true)]
    #[case::start_of_second_epoch(Some(EPOCH_HEIGHT + 1), false)]
    #[case::far_past(Some(10 * EPOCH_HEIGHT), false)]
    fn lc_contract_header_inside_seeded_epochs_cases(
        #[case] height: Option<u64>,
        #[case] expected: bool,
    ) {
        let header = height.map(|height| LcContractHeader {
            height,
            block_comm_root: CircuitField::from(0u64),
        });
        let inside = lc_contract_header_inside_seeded_epochs(
            header.as_ref(),
            EpochNumber::new(FIRST_EPOCH),
            EPOCH_HEIGHT,
        );
        assert_eq!(inside, expected);
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

    /// The LC contract header over the block at `height` of `client`.
    async fn lc_header_at(client: &TestClient, height: u64) -> LcContractHeader {
        let header = client.header(height).await.unwrap();
        let block_comm_root = header
            .get_light_client_state(ViewNumber::genesis())
            .unwrap()
            .block_comm_root;
        LcContractHeader {
            height: header.height(),
            block_comm_root,
        }
    }

    /// Test chain of `LC_CONTRACT_HEIGHT + 1` blocks and the LC contract header over it.
    ///
    /// Peers report that height, so the current epoch is `N`.
    async fn chain() -> (TestClient, LcContractHeader) {
        let client = TestClient::with_epoch_height(EPOCH_HEIGHT);
        let header = lc_header_at(&client, LC_CONTRACT_HEIGHT).await;
        (client, header)
    }

    #[tokio::test]
    async fn lc_contract_header_verification() {
        let (client, lc_header) = chain().await;
        let catchup = catchup_for(&lc_header);
        let header = client.header(LC_CONTRACT_HEIGHT).await.unwrap();
        catchup.verify_lc_contract_header(&header).unwrap();

        let wrong_root = LcContractCatchup {
            header: LcContractHeader {
                block_comm_root: catchup.header.block_comm_root + CircuitField::from(1u64),
                ..catchup.header.clone()
            },
            ..catchup.clone()
        };
        let err = wrong_root.verify_lc_contract_header(&header).unwrap_err();
        assert_matches!(
            err,
            LcContractProofError::RootMismatch { height } if height == LC_CONTRACT_HEIGHT
        );

        let wrong_height = LcContractCatchup {
            header: LcContractHeader {
                height: LC_CONTRACT_HEIGHT + 1,
                ..catchup.header.clone()
            },
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
    ///
    /// Tables answer from `visible` epochs only. `hidden` starts with none, and `add_epoch_root`
    /// reveals the table of the epoch it adds by storing its root first.
    #[derive(Clone, Debug, Default)]
    struct SeededStore {
        tables: Arc<std::sync::Mutex<HashMap<u64, StakeTuple>>>,
        roots: Arc<std::sync::Mutex<HashMap<u64, Header>>>,
        visible: Arc<std::sync::Mutex<HashSet<u64>>>,
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
            let visible = tables.keys().copied().collect();
            Self {
                tables: Arc::new(std::sync::Mutex::new(tables)),
                roots: Arc::new(std::sync::Mutex::new(headers)),
                visible: Arc::new(std::sync::Mutex::new(visible)),
            }
        }

        /// No table answers until revealed.
        fn hidden(self) -> Self {
            self.visible.lock().unwrap().clear();
            self
        }

        fn reveal(&self, epochs: RangeInclusive<u64>) {
            self.visible.lock().unwrap().extend(epochs);
        }

        fn visible_table(&self, epoch: u64) -> Option<StakeTuple> {
            let visible = self.visible.lock().unwrap().contains(&epoch);
            let table = self.tables.lock().unwrap().get(&epoch).cloned();
            table.filter(|_| visible)
        }
    }

    #[async_trait]
    impl MembershipPersistence for SeededStore {
        async fn load_stake(&self, epoch: EpochNumber) -> anyhow::Result<Option<StakeTuple>> {
            Ok(self.visible_table(epoch.u64()))
        }

        async fn load_latest_stake(
            &self,
            _limit: u64,
        ) -> anyhow::Result<Option<Vec<IndexedStake>>> {
            let visible = self.visible.lock().unwrap();
            let tables = self.tables.lock().unwrap();
            let stakes = tables
                .iter()
                .filter(|(epoch, _)| visible.contains(epoch))
                .map(|(epoch, (validators, reward, hash))| {
                    let stake = (validators.clone(), *reward);
                    (EpochNumber::new(*epoch), stake, *hash)
                })
                .collect();
            Ok(Some(stakes))
        }

        async fn load_drb_result(&self, epoch: EpochNumber) -> anyhow::Result<Option<DrbResult>> {
            Ok(self.visible_table(epoch.u64()).map(|_| [0; 32]))
        }

        async fn load_epoch_root(&self, epoch: EpochNumber) -> anyhow::Result<Option<Header>> {
            Ok(self.roots.lock().unwrap().get(&epoch.u64()).cloned())
        }

        async fn store_epoch_root(&self, epoch: EpochNumber, _: Header) -> anyhow::Result<()> {
            self.visible.lock().unwrap().insert(epoch.u64());
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
    /// serves epoch roots, so only the store and LC contract catchup can supply a table.
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
        let store = SeededStore::for_chain(client, (N - 3)..=(N + 1)).await;
        coordinator(store.hidden())
    }

    /// Storage holding the tables of `N-4 ..= N+1`, loaded into memory as `init_node_state` does.
    async fn restarted_coordinator(client: &TestClient) -> EpochMembershipCoordinator<SeqTypes> {
        let store = SeededStore::for_chain(client, (N - 4)..=(N + 1)).await;
        let mut committees = committees(store);
        committees.reload_stake(10).await;
        coordinator_over(committees)
    }

    fn peer_list(clients: &[TestClient]) -> Vec<(Url, TestClient)> {
        let url: Url = "http://peer.invalid".parse().unwrap();
        clients.iter().map(|c| (url.clone(), c.clone())).collect()
    }

    async fn run_catchup(
        coordinator: &EpochMembershipCoordinator<SeqTypes>,
        lc_contract_header: Option<LcContractHeader>,
        state_peers: &[TestClient],
        params: StartupCatchupParams,
    ) -> Result<EpochNumber, StartupCatchupError> {
        catch_up_cliquenet_stake_tables(
            coordinator,
            || async { Ok(lc_contract_header.clone()) },
            &peer_list(state_peers),
            &params,
        )
        .await
    }

    async fn run_ensure(
        coordinator: &EpochMembershipCoordinator<SeqTypes>,
        lc_contract_header: Option<LcContractHeader>,
        state_peers: &[TestClient],
        params: StartupCatchupParams,
        need: RangeInclusive<u64>,
    ) -> Result<(), StartupCatchupError> {
        ensure_stake_tables(
            coordinator,
            || async { Ok(lc_contract_header.clone()) },
            &peer_list(state_peers),
            &params,
            EpochNumber::new(*need.start())..=EpochNumber::new(*need.end()),
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

    #[test_log::test(tokio::test)]
    async fn fresh_join_catches_up_from_lc_contract() {
        let (client, header) = chain().await;
        let coordinator = fresh_coordinator(&client).await;

        let current = run_catchup(&coordinator, Some(header), &[client], PARAMS)
            .await
            .unwrap();

        assert_eq!(current, EpochNumber::new(N));
        for epoch in (N - 3)..=(N + 1) {
            assert!(has_table(&coordinator, epoch), "epoch {epoch}");
        }
        assert_eq!(
            coordinator.membership().highest_rooted_epoch(),
            Some(EpochNumber::new(N + 1))
        );
    }

    #[test_log::test(tokio::test)]
    async fn lc_contract_catchup_rotates_past_a_wrong_chain() {
        let (client, header) = chain().await;
        let (other, _) = chain().await;
        let coordinator = fresh_coordinator(&client).await;

        let current = run_catchup(&coordinator, Some(header), &[other, client], PARAMS)
            .await
            .unwrap();

        assert_eq!(current, EpochNumber::new(N));
        assert!(has_table(&coordinator, N + 1));
    }

    /// An epoch root the peer cannot serve at first must be retried, not read as the tip.
    #[test_log::test(tokio::test)]
    async fn lc_contract_catchup_retries_a_transient_failure() {
        let (client, header) = chain().await;
        let coordinator = fresh_coordinator(&client).await;
        let flaky_root = root_block_in_epoch(N - 1, EPOCH_HEIGHT) as usize;
        client.forget_leaf(flaky_root).await;
        let heal = client.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            heal.remember_leaf(flaky_root).await;
        });

        let current = run_catchup(&coordinator, Some(header), &[client], PARAMS)
            .await
            .unwrap();

        assert_eq!(current, EpochNumber::new(N));
        assert!(has_table(&coordinator, N + 1));
    }

    #[test_log::test(tokio::test)]
    async fn lc_contract_header_unavailable_aborts_at_the_deadline() {
        let (client, header) = chain().await;
        let coordinator = fresh_coordinator(&client).await;
        client.forget_leaf(LC_CONTRACT_HEIGHT as usize).await;

        let params = StartupCatchupParams {
            total_timeout: Duration::from_millis(300),
            ..PARAMS
        };
        let err = run_catchup(&coordinator, Some(header), &[client], params)
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

    /// A failed LC contract header read is retried and LC contract catchup then runs.
    #[test_log::test(tokio::test)]
    async fn lc_contract_header_read_failure_is_retried() {
        let (client, header) = chain().await;
        let coordinator = fresh_coordinator(&client).await;
        let reads = AtomicUsize::new(0);

        let current = catch_up_cliquenet_stake_tables(
            &coordinator,
            || async {
                ensure!(reads.fetch_add(1, Ordering::Relaxed) > 0, "L1 not ready");
                Ok(Some(header.clone()))
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

    /// The LC contract header read shares the deadline with the stages after it.
    #[test_log::test(tokio::test)]
    async fn lc_contract_read_consumes_the_deadline() {
        let (client, _) = chain().await;
        let coordinator = coordinator(SeededStore::default());

        let err = ensure_stake_tables(
            &coordinator,
            std::future::pending::<anyhow::Result<Option<LcContractHeader>>>,
            &peer_list(&[client]),
            &SHORT_DEADLINE_PARAMS,
            EpochNumber::new(N)..=EpochNumber::new(N),
        )
        .await
        .unwrap_err();

        assert_matches!(
            err,
            StartupCatchupError::DeadlineExceeded {
                stage: Stage::LcContractHeaderRead,
                ..
            }
        );
    }

    /// Held epochs skip LC contract catchup.
    #[test_log::test(tokio::test)]
    async fn restart_boots_from_storage() {
        let (client, header) = chain().await;
        let coordinator = restarted_coordinator(&client).await;
        client.forget_leaf(LC_CONTRACT_HEIGHT as usize).await;

        let current = run_catchup(
            &coordinator,
            Some(header),
            std::slice::from_ref(&client),
            PARAMS,
        )
        .await
        .unwrap();

        assert_eq!(current, EpochNumber::new(N));
    }

    /// No peer height and stored epochs start from storage, without reading the LC contract.
    #[test_log::test(tokio::test)]
    async fn no_peer_height_boots_from_storage() {
        let (client, _) = chain().await;
        let coordinator = restarted_coordinator(&client).await;
        client.fail_block_height().await;
        let reads = AtomicUsize::new(0);

        let current = catch_up_cliquenet_stake_tables(
            &coordinator,
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

    /// The median tip decides `N`, even when storage holds tables above it.
    #[test_log::test(tokio::test)]
    async fn median_peer_tip_selects_the_current_epoch_over_storage() {
        let (client, _) = chain().await;
        client.mock_block_height(LC_CONTRACT_HEIGHT + 1).await;
        let store = SeededStore::for_chain(&client, (N - 3)..=(N + 3)).await;
        let coordinator = coordinator(store);
        hold(&coordinator, (N - 3)..=(N + 3)).await;
        assert_eq!(
            coordinator.membership().highest_rooted_epoch(),
            Some(EpochNumber::new(N + 3))
        );

        let current = run_catchup(&coordinator, None, &[client], PARAMS)
            .await
            .unwrap();

        assert_eq!(current, EpochNumber::new(N));
    }

    /// Nothing held beyond the seeded epochs: the LC contract inside the first epoch (it ends at
    /// block 10) starts at genesis, one block further the chain is past the seeded epochs and
    /// storage cannot serve it.
    #[rstest]
    #[case::no_lc_contract(None, true)]
    #[case::in_first_epoch(Some(EPOCH_HEIGHT), true)]
    #[case::in_second_epoch(Some(EPOCH_HEIGHT + 1), false)]
    #[case::far_past(Some(5 * EPOCH_HEIGHT), false)]
    #[test_log::test(tokio::test)]
    async fn no_peer_height_and_nothing_derived(
        #[case] lc_height: Option<u64>,
        #[case] starts_at_genesis: bool,
    ) {
        let (client, _) = chain().await;
        let coordinator = coordinator(SeededStore::default());
        client.fail_block_height().await;
        let header = lc_height.map(|height| LcContractHeader {
            height,
            block_comm_root: CircuitField::from(0u64),
        });

        let result = run_catchup(&coordinator, header, &[client], PARAMS).await;

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

        let err = catch_up_cliquenet_stake_tables(
            &coordinator,
            || async { Err(anyhow!("L1 not ready")) },
            &peer_list(&[client]),
            &SHORT_DEADLINE_PARAMS,
        )
        .await
        .unwrap_err();

        assert_matches!(
            err,
            StartupCatchupError::DeadlineExceeded {
                stage: Stage::LcContractHeaderRead,
                ..
            }
        );
    }

    #[test_log::test(tokio::test)]
    async fn no_state_peers_configured_continues_from_the_held_epochs() {
        let (client, header) = chain().await;
        let coordinator = restarted_coordinator(&client).await;

        let current = run_catchup(&coordinator, Some(header), &[], PARAMS)
            .await
            .unwrap();

        assert_eq!(current, EpochNumber::new(N));
    }

    /// A genesis boot has nothing to catch up, so it needs no state peers.
    #[test_log::test(tokio::test)]
    async fn genesis_boot_without_state_peers() {
        let coordinator = coordinator(SeededStore::default());
        let no_peers: &[(Url, TestClient)] = &[];

        let current =
            catch_up_cliquenet_stake_tables(&coordinator, || async { Ok(None) }, no_peers, &PARAMS)
                .await
                .unwrap();

        assert_eq!(current, EpochNumber::new(FIRST_EPOCH));
    }

    #[test_log::test(tokio::test)]
    async fn no_peers_configured_with_a_missing_table_fails_before_the_lc_read() {
        let coordinator = coordinator(SeededStore::default());
        let no_peers: &[(Url, TestClient)] = &[];
        let reads = AtomicUsize::new(0);

        let result = ensure_stake_tables(
            &coordinator,
            || async {
                reads.fetch_add(1, Ordering::Relaxed);
                Err(anyhow!("L1 not ready"))
            },
            no_peers,
            &PARAMS,
            EpochNumber::new(N)..=EpochNumber::new(N),
        )
        .await;

        assert_matches!(result, Err(StartupCatchupError::StatePeersUnavailable));
        assert_eq!(reads.load(Ordering::Relaxed), 0);
    }

    /// Every needed epoch held: no LC contract read, no peers needed.
    #[test_log::test(tokio::test)]
    async fn needed_tables_held_skip_the_lc_contract_read() {
        let (client, _) = chain().await;
        let coordinator = restarted_coordinator(&client).await;
        let no_peers: &[(Url, TestClient)] = &[];
        let reads = AtomicUsize::new(0);

        ensure_stake_tables(
            &coordinator,
            || async {
                reads.fetch_add(1, Ordering::Relaxed);
                Err(anyhow!("L1 not ready"))
            },
            no_peers,
            &PARAMS,
            EpochNumber::new(N - 1)..=EpochNumber::new(N + 1),
        )
        .await
        .unwrap();

        assert_eq!(reads.load(Ordering::Relaxed), 0);
    }

    #[test_log::test(tokio::test)]
    async fn table_in_storage_but_not_memory_counts_as_held() {
        let (client, _) = chain().await;
        let store = SeededStore::for_chain(&client, (N - 4)..=(N + 1)).await;
        let coordinator = coordinator(store);
        assert!(!has_table(&coordinator, N));
        let no_peers: &[(Url, TestClient)] = &[];

        ensure_stake_tables(
            &coordinator,
            || async { Err(anyhow!("L1 not ready")) },
            no_peers,
            &PARAMS,
            EpochNumber::new(N - 1)..=EpochNumber::new(N + 1),
        )
        .await
        .unwrap();

        assert!(has_table(&coordinator, N));
    }

    /// A failed derivation step is retried until the epoch is held.
    #[test_log::test(tokio::test)]
    async fn derivation_retries_each_step_until_the_epoch_is_held() {
        let (client, _) = chain().await;
        let store = SeededStore::for_chain(&client, (N - 4)..=(N + 1))
            .await
            .hidden();
        store.reveal((N - 4)..=(N - 3));
        let coordinator = coordinator(store.clone());
        hold(&coordinator, (N - 4)..=(N - 3)).await;
        let late = store.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            late.reveal((N - 2)..=(N + 1));
        });

        run_ensure(&coordinator, None, &[client], PARAMS, (N - 1)..=(N + 1))
            .await
            .unwrap();

        assert!(has_table(&coordinator, N - 1));
        assert!(has_table(&coordinator, N + 1));
    }

    /// A derivation step that keeps failing ends startup at the deadline.
    #[test_log::test(tokio::test)]
    async fn derivation_fails_at_the_deadline() {
        let (client, _) = chain().await;
        let store = SeededStore::for_chain(&client, (N - 4)..=(N + 1))
            .await
            .hidden();
        store.reveal((N - 4)..=(N - 3));
        let coordinator = coordinator(store);
        hold(&coordinator, (N - 4)..=(N - 3)).await;

        let err = run_ensure(
            &coordinator,
            None,
            &[client],
            SHORT_DEADLINE_PARAMS,
            (N - 1)..=(N + 1),
        )
        .await
        .unwrap_err();

        assert_matches!(
            err,
            StartupCatchupError::DeadlineExceeded {
                stage: Stage::Derivation { epoch },
                ..
            } if epoch == EpochNumber::new(N - 1)
        );
    }

    /// A bad proof after the first good ones fails the whole fetch.
    #[test_log::test(tokio::test)]
    async fn invalid_proof_mid_range_fails() {
        let (client, header) = chain().await;
        let catchup = catchup_for(&header);
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
        let (client, header) = chain().await;
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
            Some(header),
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

    /// A held epoch without its root, above the rooted ones, does not select the current epoch.
    #[test_log::test(tokio::test)]
    async fn rootless_top_epoch_does_not_select_the_current_epoch() {
        let (client, _) = chain().await;
        client.fail_block_height().await;
        let store = SeededStore::for_chain(&client, (N - 3)..=(N + 2)).await;
        store.roots.lock().unwrap().remove(&(N + 2));
        let coordinator = coordinator(store);
        hold(&coordinator, (N - 3)..=(N + 2)).await;
        assert!(has_table(&coordinator, N + 2));
        assert_eq!(
            coordinator.membership().highest_rooted_epoch(),
            Some(EpochNumber::new(N + 1))
        );

        let current = run_catchup(&coordinator, None, &[client], PARAMS)
            .await
            .unwrap();

        assert_eq!(current, EpochNumber::new(N));
    }

    /// A goal far below `F` installs the proofs at the goal, not at `F`. The membership has no peer
    /// to derive from, so only proofs can supply the tables.
    #[test_log::test(tokio::test)]
    async fn follower_goal_far_below_the_lc_contract_uses_proofs_at_the_goal() {
        let client = TestClient::with_epoch_height(EPOCH_HEIGHT);
        let header = lc_header_at(&client, 20 * EPOCH_HEIGHT + 7).await;
        let goal = N + 3;
        let store = SeededStore::for_chain(&client, (goal - 4)..=goal).await;
        let coordinator = coordinator(store.hidden());

        run_ensure(&coordinator, Some(header), &[client], PARAMS, goal..=goal)
            .await
            .unwrap();

        for epoch in (goal - 4)..=goal {
            assert!(has_table(&coordinator, epoch), "epoch {epoch}");
        }
        for epoch in (FIRST_EPOCH + 2)..(goal - 4) {
            assert!(!has_table(&coordinator, epoch), "epoch {epoch}");
        }
    }

    /// Held `3..=5` and a lone table at `8`: the goal `7` is in the gap, and `6` below it is
    /// filled too.
    #[rstest]
    #[case::lc_contract_header(true)]
    #[case::no_lc_contract_header(false)]
    #[test_log::test(tokio::test)]
    async fn goal_in_a_gap_is_filled(#[case] with_lc_contract_header: bool) {
        let (client, header) = chain().await;
        let store = SeededStore::for_chain(&client, 3..=8).await.hidden();
        store.reveal(3..=5);
        store.reveal(8..=8);
        let coordinator = coordinator(store.clone());
        hold(&coordinator, 3..=5).await;
        hold(&coordinator, 8..=8).await;
        let header = with_lc_contract_header.then_some(header);
        if header.is_none() {
            let late = store.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(100)).await;
                late.reveal(6..=7);
            });
        }

        run_ensure(&coordinator, header, &[client], PARAMS, 7..=7)
            .await
            .unwrap();

        assert!(has_table(&coordinator, 6));
        assert!(has_table(&coordinator, 7));
    }
}
