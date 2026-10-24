//! Startup stake-table catchup for new-protocol nodes.
//!
//! Cliquenet only connects to validators in the stake tables of epochs `N-1`, `N` and `N+1`, where
//! `N` is the current epoch. "The stake tables cliquenet connects to" in this module means those
//! three. On a fresh-join or cold-restart node, no consensus messages can be received until those
//! stake tables are populated, so the existing reactive catchup (triggered by an unknown-epoch
//! proposal) never fires.
//!
//! LC contract: the `LightClient` contract on L1.
//!
//! [`catch_up_cliquenet_stake_tables`] observes what the node knows, plans the catchup and executes
//! it:
//!
//! 1. From storage: `reload_stake` put the stored epochs in memory before this module runs.
//!    Nothing is fetched for them.
//! 2. LC contract catchup: the LC contract vouches for its header `F`. The epoch roots below `F`
//!    are fetched with Merkle proofs against `F` for the epochs of `epoch(F)-3 ..= epoch(F)+1`
//!    that storage lacks. Failures are retried until the deadline. Missing epochs without state
//!    peers fail at once.
//! 3. Peer catchup: from the highest epoch with an epoch root, one epoch at a time, derived from
//!    QC-verified leaves. Steps up to the epoch of the median peer's chain tip are retried until
//!    the deadline. Above it, up to the epoch of the highest reported tip, the first failure
//!    marks the live tip and peer catchup stops there. Without state peers, or on a chain of no
//!    blocks, peer catchup is skipped. With peers, the block heights are read after LC contract
//!    catchup, for a few passes. If none answers in that time, peer catchup is skipped with a
//!    warning: it fetches through the same peers. If the deadline passes first, startup fails.
//!
//! All stages share one deadline.
//!
//! Called from validator boot (`SequencerContext::init`), follower boot (`init_with_source`) and
//! the follower mid-run skip-ahead (`Follower::catch_up_skipped_epochs`).
//! `init_node_state` runs `reload_stake` first. The returned current epoch `N` goes to
//! `coordinator_network.apply_epoch`, which dials the validators of those stake tables before
//! consensus starts.

use std::{collections::BTreeSet, ops::RangeInclusive, time::Duration};

use anyhow::{Context, anyhow, ensure};
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
use tokio::time::{Instant, error::Elapsed, sleep, timeout, timeout_at};
use url::Url;
use vec1::Vec1;

/// First wait between two retries, doubled up to `MAX_BACKOFF`.
const INITIAL_BACKOFF: Duration = Duration::from_secs(1);
/// Longest wait between two retries.
const MAX_BACKOFF: Duration = Duration::from_secs(60);
/// Time to read the peers' block heights. Up to three passes when peers fail fast: at 0, 1 and 3
/// initial backoffs, plus slack.
const PEER_HEIGHT_BUDGET: Duration = INITIAL_BACKOFF.saturating_mul(5);
/// Epochs past `epoch(F)` that the median peer tip epoch may reach, `F` being the LC contract
/// header.
const MAX_TIP_EPOCHS_PAST_LC_CONTRACT: u64 = 2;

/// Catches up the stake tables of `N-1`, `N`, `N+1`, the epochs cliquenet connects to.
///
/// `read_lc_contract_state` returns the LC contract state, or `LightClientStateSol::default()`
/// when there is none. Its failures are retried until the deadline.
///
/// Preconditions: `reload_stake` should have run before this: it populates the membership from
/// local persistence, so those epochs are taken from storage.
pub(crate) async fn catch_up_cliquenet_stake_tables<C: Client, Fut>(
    coordinator: &EpochMembershipCoordinator<SeqTypes>,
    epoch_height: u64,
    read_lc_contract_state: impl Fn() -> Fut,
    state_peers: &[(Url, C)],
    params: &StartupCatchupParams,
) -> Result<CatchupOutcome, StartupCatchupError>
where
    Fut: Future<Output = anyhow::Result<LightClientStateSol>>,
{
    if epoch_height == 0 {
        return Err(StartupCatchupError::EpochsDisabled);
    }

    let started = Instant::now();
    let catchup = StartupCatchup {
        coordinator,
        state_peers,
        params,
        blocks_per_epoch: epoch_height,
        deadline: started + params.total_timeout,
    };
    let snapshot = SourcesSnapshot::create(&catchup, read_lc_contract_state).await?;
    let outcome = catchup.execute(snapshot).await?;
    tracing::info!(
        target: "announce::bootstrap",
        current_epoch = %outcome.current,
        duration_ms = started.elapsed().as_millis() as u64,
        "cliquenet stake tables ready",
    );
    Ok(outcome)
}

/// What a finished startup catchup reached.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CatchupOutcome {
    /// The current epoch `N`.
    pub current: EpochNumber,
    /// The highest epoch peer catchup had to reach, retrying until the deadline. Stake tables above
    /// it are best-effort. `None` when peer catchup was skipped for lack of peer block heights.
    pub required: Option<EpochNumber>,
}

/// Why the startup catchup failed.
#[derive(Debug, thiserror::Error)]
pub(crate) enum StartupCatchupError {
    /// `epoch_height` is zero.
    #[error("epochs are required: epoch_height must be positive")]
    EpochsDisabled,

    /// The membership has no first epoch to seed from.
    #[error("first_epoch not seeded; genesis stake table missing")]
    FirstEpochUnset,

    /// A retried stage ran out of time, `source` being its last failure.
    #[error("startup catchup deadline exceeded after {attempts} attempts while {stage}")]
    DeadlineExceeded {
        stage: Stage,
        attempts: usize,
        #[source]
        source: anyhow::Error,
    },

    /// LC contract catchup has epochs to fetch and no state peer to fetch them from.
    #[error(
        "LC contract catchup at LC contract height {lc_contract_height} needs a state peer \
         serving the light-client API"
    )]
    NoStatePeers { lc_contract_height: u64 },

    /// An epoch cliquenet connects to has no stake table after catchup.
    #[error("missing stake table for epoch {epoch} after startup catchup")]
    MissingStakeTable { epoch: EpochNumber },
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

/// A retried stage of the startup catchup.
#[derive(Clone, Copy, Debug, Display)]
pub(crate) enum Stage {
    /// Reading the LC contract state.
    #[display("reading the LC contract state")]
    LcContractStateRead,
    /// Fetching and adding the LC contract epoch roots.
    #[display("LC contract catchup")]
    LcContractCatchup,
    /// Reading the peers' block heights.
    #[display("reading the peer block heights")]
    PeerHeights,
    /// Deriving the stake table of `epoch`, at most the epoch after the median peer tip epoch.
    #[display("peer catchup of epoch {epoch} up to the epoch after the median tip")]
    PeerCatchup { epoch: EpochNumber },
}

/// Time bounds of the startup catchup.
#[derive(Clone, Copy, Debug)]
pub struct StartupCatchupParams {
    /// Bound on a single peer catchup step, LC contract catchup fetch, LC contract state read or
    /// block height read.
    pub step_timeout: Duration,
    /// How long LC contract catchup and peer catchup may keep retrying, together, before startup
    /// gives up.
    pub total_timeout: Duration,
}

/// One `QueryServiceClient` per URL, to read block heights and light-client proofs from.
pub(crate) fn state_peer_clients(urls: &[Url]) -> Vec<(Url, QueryServiceClient)> {
    urls.iter()
        .map(|url| (url.clone(), QueryServiceClient::new(url.clone())))
        .collect()
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

impl<C: Client> StartupCatchup<'_, C> {
    /// Run LC contract catchup, read the peer block heights, plan and run peer catchup, and
    /// prepare the epochs cliquenet connects to.
    async fn execute(
        &self,
        mut snapshot: SourcesSnapshot,
    ) -> Result<CatchupOutcome, StartupCatchupError> {
        if let Some(catchup) = snapshot.lc_contract_catchup() {
            self.catch_up_from_lc_contract(&catchup).await?;
        }
        snapshot.peer_block_heights = self.read_peer_block_heights().await?;
        let plan = snapshot.plan();
        self.log_plan(&snapshot, &plan);

        let highest_epoch = match plan.peer_catchup {
            Some(catchup) => {
                self.catch_up_from_peers(plan.highest_held_epoch, &catchup)
                    .await?
            },
            None => plan.highest_held_epoch,
        };
        let current = self
            .prepare_cliquenet_epochs(snapshot.first_epoch, highest_epoch)
            .await?;
        let required = snapshot
            .peer_catchup()
            .map(|catchup| catchup.median_peer_tip_epoch + 1);
        Ok(CatchupOutcome { current, required })
    }
}

/// Snapshot of the catchup sources: local storage and the LC contract before LC contract catchup,
/// the state peers after it.
#[derive(Clone, Debug, PartialEq)]
struct SourcesSnapshot {
    /// Blocks in one epoch.
    blocks_per_epoch: u64,
    /// First epoch of the chain, whose stake table and the next one are seeded from genesis.
    first_epoch: EpochNumber,
    /// From local storage. Epochs with an epoch root loaded by `reload_stake`.
    stored_epochs: BTreeSet<EpochNumber>,
    /// From the LC contract, with the header `F` at its block height. Default without any.
    lc_contract_state: LightClientStateSol,
    /// From the state peers. Block heights (block counts), read after LC contract catchup. Empty
    /// without state peers, or when none answered.
    peer_block_heights: Vec<u64>,
}

/// What to fetch, from the sources snapshot.
#[derive(Clone, Debug, PartialEq)]
struct CatchupPlan {
    /// `None`: storage holds the LC contract epochs, or there are none.
    lc_contract_catchup: Option<LcContractCatchup>,
    /// Where peer catchup starts: the highest of the stored rooted epochs and the fetched epochs,
    /// else `first_epoch + 1`.
    highest_held_epoch: EpochNumber,
    /// `None`: no usable peer block height, or the highest peer tip is already held.
    peer_catchup: Option<PeerCatchup>,
}

impl SourcesSnapshot {
    /// Collect the sources known before any catchup: stored epochs and the LC contract state.
    async fn create<C: Client, Fut>(
        catchup: &StartupCatchup<'_, C>,
        read_lc_contract_state: impl Fn() -> Fut,
    ) -> Result<Self, StartupCatchupError>
    where
        Fut: Future<Output = anyhow::Result<LightClientStateSol>>,
    {
        let membership = catchup.coordinator.membership();
        let first_epoch = membership
            .first_epoch()
            .ok_or(StartupCatchupError::FirstEpochUnset)?;
        // Epochs added after this read (mid-run reactive catchup) are fetched again, harmlessly.
        let stored_epochs = membership.epochs_with_root();

        let lc_contract_state = retry(
            catchup.deadline,
            Stage::LcContractStateRead,
            1,
            |_, remaining| {
                let read = timeout(
                    catchup.params.step_timeout.min(remaining),
                    read_lc_contract_state(),
                );
                async move { read.await.context("attempt timed out")? }
            },
        )
        .await?;

        let snapshot = Self {
            blocks_per_epoch: catchup.blocks_per_epoch,
            first_epoch,
            stored_epochs,
            lc_contract_state,
            peer_block_heights: vec![],
        };
        tracing::info!(
            target: "announce::bootstrap",
            %first_epoch,
            stored = ?epoch_numbers(&snapshot.stored_epochs),
            lc_contract_height = snapshot.lc_contract_state.blockHeight,
            "sources snapshot",
        );
        Ok(snapshot)
    }

    /// Decide what to fetch: pure in the snapshot.
    fn plan(&self) -> CatchupPlan {
        let lc_contract_catchup = self.lc_contract_catchup();
        let highest_stored = self.stored_epochs.last().copied();
        let highest_fetched = lc_contract_catchup
            .as_ref()
            .map(|catchup| *catchup.missing_epochs.last());
        let highest_held_epoch = highest_stored
            .max(highest_fetched)
            .unwrap_or(self.first_epoch + 1);
        let peer_catchup = self
            .peer_catchup()
            .filter(|catchup| !catchup.done(highest_held_epoch));
        CatchupPlan {
            lc_contract_catchup,
            highest_held_epoch,
            peer_catchup,
        }
    }

    /// The LC contract epochs storage lacks, `None` when it holds them all.
    fn lc_contract_catchup(&self) -> Option<LcContractCatchup> {
        let epochs = self.lc_contract_epochs()?;
        let missing_epochs: Vec<EpochNumber> = (epochs.start().u64()..=epochs.end().u64())
            .map(EpochNumber::new)
            .filter(|epoch| !self.stored_epochs.contains(epoch))
            .collect();
        Some(LcContractCatchup {
            lc_contract_height: self.lc_contract_state.blockHeight,
            block_comm_root: u256_to_field(self.lc_contract_state.blockCommRoot),
            missing_epochs: Vec1::try_from_vec(missing_epochs).ok()?,
        })
    }

    /// The epochs peer catchup targets, `None` without a usable block height.
    fn peer_catchup(&self) -> Option<PeerCatchup> {
        let median_peer_tip_epoch = self.median_peer_tip_epoch()?;
        let highest_peer_tip_epoch = self.highest_peer_tip_epoch()?;
        // Epochs past the LC contract one cannot be vouched for, so a few lying peers cannot hold
        // peer catchup there.
        let cap = self.lc_contract_epochs().map(|_| {
            EpochNumber::new(self.lc_contract_epoch().u64() + MAX_TIP_EPOCHS_PAST_LC_CONTRACT)
        });
        Some(PeerCatchup {
            median_peer_tip_epoch: cap
                .map_or(median_peer_tip_epoch, |cap| median_peer_tip_epoch.min(cap)),
            highest_peer_tip_epoch,
        })
    }

    /// Epoch of the LC contract header `F`.
    fn lc_contract_epoch(&self) -> EpochNumber {
        EpochNumber::new(epoch_from_block_number(
            self.lc_contract_state.blockHeight,
            self.blocks_per_epoch,
        ))
    }

    /// The epochs LC contract catchup covers: `epoch(F)-3 ..= epoch(F)+1`, clamped above the two
    /// epochs seeded from genesis.
    ///
    /// `block_reward(n)` and `block_reward(n+1)` need the stake table of three epochs earlier, and
    /// the rest are the stake tables cliquenet connects to. `None` when `F` is before the first
    /// epoch's end, so there is no root to
    /// prove. Every root in the range is below `F`: the root of `e-2` lies in epoch `e-2 < n`.
    fn lc_contract_epochs(&self) -> Option<RangeInclusive<EpochNumber>> {
        let n = self.lc_contract_epoch().u64();
        let start = n.saturating_sub(3).max(*self.first_epoch + 2);
        (start <= n + 1).then(|| EpochNumber::new(start)..=EpochNumber::new(n + 1))
    }

    /// Epoch of the last block of a chain of `block_height` blocks, `None` for no blocks.
    fn epoch_of_last_block(&self, block_height: u64) -> Option<EpochNumber> {
        let tip = block_height.checked_sub(1)?;
        Some(EpochNumber::new(epoch_from_block_number(
            tip,
            self.blocks_per_epoch,
        )))
    }

    /// Epoch of the last block of the median peer, before the cap. The median of an even count is
    /// the lower one.
    fn median_peer_tip_epoch(&self) -> Option<EpochNumber> {
        let mut heights = self.peer_block_heights.clone();
        heights.sort_unstable();
        let median = heights.get(heights.len().checked_sub(1)? / 2)?;
        self.epoch_of_last_block(*median)
    }

    /// Epoch of the last block of the highest peer.
    fn highest_peer_tip_epoch(&self) -> Option<EpochNumber> {
        self.epoch_of_last_block(*self.peer_block_heights.iter().max()?)
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

impl<C: Client> StartupCatchup<'_, C> {
    /// Add the epoch roots LC contract catchup fetched, from the peers in turn until one serves.
    ///
    /// Only the fetch is bounded by `step_timeout`: adding the roots can scan L1 events for longer,
    /// and cancelling it would restart that scan.
    async fn catch_up_from_lc_contract(
        &self,
        catchup: &LcContractCatchup,
    ) -> Result<(), StartupCatchupError> {
        if self.state_peers.is_empty() {
            return Err(StartupCatchupError::NoStatePeers {
                lc_contract_height: catchup.lc_contract_height,
            });
        }
        retry(
            self.deadline,
            Stage::LcContractCatchup,
            self.state_peers.len(),
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
}

/// Peer catchup: the epochs to derive from the peers' chain tips.
#[derive(Clone, Copy, Debug, PartialEq)]
struct PeerCatchup {
    /// Epoch of the median peer's last block, capped at
    /// `epoch(F) + MAX_TIP_EPOCHS_PAST_LC_CONTRACT`. Peer catchup must reach the epoch after it,
    /// and retries until the deadline on the way.
    median_peer_tip_epoch: EpochNumber,
    /// Epoch of the highest peer's last block, at least `median_peer_tip_epoch`. Peer catchup
    /// stops at the epoch after it, trying each step above the median once.
    highest_peer_tip_epoch: EpochNumber,
}

impl PeerCatchup {
    /// Whether deriving `epoch` must succeed: it is at most the epoch after the median peer tip
    /// epoch.
    fn required(&self, epoch: EpochNumber) -> bool {
        epoch <= self.median_peer_tip_epoch + 1
    }

    /// Whether peer catchup ends with `highest_held_epoch` as the highest epoch reached.
    fn done(&self, highest_held_epoch: EpochNumber) -> bool {
        highest_held_epoch > self.highest_peer_tip_epoch
    }
}

impl<C: Client> StartupCatchup<'_, C> {
    /// The block heights the peers report, each read bounded by `step_timeout`.
    ///
    /// Passes over the peers repeat with backoff until one answers, for at most
    /// `PEER_HEIGHT_BUDGET` and the deadline. Empty without peers, or when none answered in that
    /// time: peer catchup would fetch through the same peers, so it is skipped. Fails when the
    /// deadline passes first.
    async fn read_peer_block_heights(&self) -> Result<Vec<u64>, StartupCatchupError> {
        if self.state_peers.is_empty() {
            return Ok(vec![]);
        }
        let deadline = self.deadline.min(Instant::now() + PEER_HEIGHT_BUDGET);
        let read = retry(deadline, Stage::PeerHeights, 1, |_, remaining| async move {
            let bound = self.params.step_timeout.min(remaining);
            let reads = self
                .state_peers
                .iter()
                .map(|(_, peer)| timeout(bound, peer.block_height()));
            let heights: Vec<u64> = join_all(reads)
                .await
                .into_iter()
                .filter_map(|height| height.ok()?.ok())
                .collect();
            ensure!(!heights.is_empty(), "no state peer reported a block height");
            Ok(heights)
        })
        .await;
        match read {
            Ok(heights) => Ok(heights),
            Err(err) if Instant::now() >= self.deadline => Err(err),
            Err(err) => {
                tracing::warn!(
                    err = format!("{:#}", anyhow::Error::from(err)),
                    "startup catchup: no state peer reported a block height; skipping peer catchup"
                );
                Ok(vec![])
            },
        }
    }

    /// Derive the stake table of one epoch at a time above `highest_held_epoch`, the highest epoch
    /// with an epoch root. Returns the highest epoch reached when peer catchup ends.
    ///
    /// Steps `catchup` requires are retried until the deadline. Above them, the first step peers
    /// cannot serve marks the live tip, and reaching the highest tip epoch ends peer catchup.
    async fn catch_up_from_peers(
        &self,
        highest_held_epoch: EpochNumber,
        catchup: &PeerCatchup,
    ) -> Result<EpochNumber, StartupCatchupError> {
        let coordinator = self.coordinator;
        let step_timeout = self.params.step_timeout;
        let mut highest = highest_held_epoch;
        while !catchup.done(highest) {
            let next = highest + 1;
            let derive = move |_, bound: Duration| async move {
                let bound = step_timeout.min(bound);
                let step = timeout(bound, coordinator.wait_for_stake_table(next));
                step.await
                    .with_context(|| format!("timed out after {bound:?}"))?
                    .map(drop)
                    .map_err(|err| anyhow!("{err:#}"))
            };
            if catchup.required(next) {
                let stage = Stage::PeerCatchup { epoch: next };
                retry(self.deadline, stage, 1, derive).await?;
            } else if let Err(err) = derive(0, step_timeout).await {
                if err.is::<Elapsed>() {
                    tracing::warn!(%next, %err, "startup catchup: peer catchup timed out; proceeding with possibly stale cliquenet stake tables");
                } else {
                    tracing::info!(
                        target: "announce::bootstrap", %next, %err, "peer catchup: at the live tip"
                    );
                }
                break;
            }
            tracing::debug!(%next, "startup catchup: derived stake table from peers");
            highest = next;
        }
        Ok(highest)
    }
}

impl<C: Client> StartupCatchup<'_, C> {
    /// Check the stake tables of `highest_epoch - 1` and `highest_epoch` and prepare the DRBs of
    /// `N-1`, `N` and `N+1`. Returns the current epoch `N = highest_epoch - 1`.
    async fn prepare_cliquenet_epochs(
        &self,
        first_epoch: EpochNumber,
        highest_epoch: EpochNumber,
    ) -> Result<EpochNumber, StartupCatchupError> {
        // `highest_epoch` corresponds to N+1 (the leaf at root_block_in_epoch(N-1) is
        // the last finalized one peers can serve). So current epoch N = highest_epoch - 1.
        let current = EpochNumber::new(highest_epoch.saturating_sub(1));
        let membership = self.coordinator.membership();
        for epoch in [current, highest_epoch] {
            if membership.snapshot(epoch).is_none() {
                return Err(StartupCatchupError::MissingStakeTable { epoch });
            }
        }
        // `current - 1` is skipped when seeded from genesis, which already has its DRB.
        let previous = current.checked_sub(1).filter(|e| *e > *first_epoch + 1);
        for epoch in previous.into_iter().chain([*current, *highest_epoch]) {
            self.prepare_drb(EpochNumber::new(epoch)).await;
        }
        Ok(current)
    }

    /// Make the DRB of `epoch` available before consensus starts. A failure only delays it:
    /// consensus retries through reactive catchup.
    async fn prepare_drb(&self, epoch: EpochNumber) {
        // A missing DRB starts a catchup that ends in `drb_ready`; its error is the start notice.
        if self.coordinator.membership_for_epoch(Some(epoch)).is_ok() {
            return;
        }
        let ready = self.coordinator.wait_for_catchup(epoch);
        match timeout(self.params.step_timeout, ready).await {
            Ok(Ok(_)) => tracing::debug!(%epoch, "startup catchup: DRB ready"),
            Ok(Err(err)) => {
                tracing::warn!(%epoch, %err, "startup catchup: DRB not ready; consensus will catch up")
            },
            Err(_) => {
                tracing::warn!(%epoch, "startup catchup: DRB timed out; consensus will catch up")
            },
        }
    }
}

impl<C: Client> StartupCatchup<'_, C> {
    /// Log `plan` as a milestone, and warn when the median peer tip epoch is capped.
    fn log_plan(&self, snapshot: &SourcesSnapshot, plan: &CatchupPlan) {
        let capped = snapshot
            .peer_catchup()
            .zip(snapshot.median_peer_tip_epoch())
            .filter(|(catchup, uncapped)| catchup.median_peer_tip_epoch < *uncapped);
        if let Some((catchup, uncapped)) = capped {
            tracing::warn!(
                peer_tip_epoch = %uncapped,
                median_peer_tip_epoch = %catchup.median_peer_tip_epoch,
                "startup catchup: median peer tip past the LC contract epochs; capping it"
            );
        }
        tracing::info!(
            target: "announce::bootstrap",
            lc_contract_epochs = ?plan.lc_contract_catchup.as_ref().map(|c| epoch_numbers(&c.missing_epochs)),
            start = %plan.highest_held_epoch,
            median_peer_tip_epoch = ?plan.peer_catchup.map(|c| c.median_peer_tip_epoch.u64()),
            highest_peer_tip_epoch = ?plan.peer_catchup.map(|c| c.highest_peer_tip_epoch.u64()),
            "catchup plan",
        );
    }
}

/// Run `attempt` with its index and the time left until it succeeds, backing off after every
/// `per_pass` failures. Errors at `deadline`.
async fn retry<T, Fut: Future<Output = anyhow::Result<T>>>(
    deadline: Instant,
    stage: Stage,
    per_pass: usize,
    mut attempt: impl FnMut(usize, Duration) -> Fut,
) -> Result<T, StartupCatchupError> {
    let mut backoff = INITIAL_BACKOFF;
    let mut count = 0usize;
    let mut last_err = anyhow!("the deadline passed before the first attempt");
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(StartupCatchupError::DeadlineExceeded {
                stage,
                attempts: count,
                source: last_err,
            });
        }

        match attempt(count, remaining).await {
            Ok(value) => return Ok(value),
            Err(err) => last_err = err,
        }
        count += 1;
        if count.is_power_of_two() {
            tracing::warn!(
                attempt = count,
                err = format!("{last_err:#}"),
                "startup catchup: {stage} failed; retrying"
            );
        }

        if count.is_multiple_of(per_pass) {
            let remaining = deadline.saturating_duration_since(Instant::now());
            sleep(backoff.min(remaining)).await;
            backoff = (backoff * 2).min(MAX_BACKOFF);
        }
    }
}

/// The numbers of `epochs`, for logs.
fn epoch_numbers<'a>(epochs: impl IntoIterator<Item = &'a EpochNumber>) -> Vec<u64> {
    epochs.into_iter().map(|epoch| epoch.u64()).collect()
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

    /// Sources snapshot of a chain of `EPOCH_HEIGHT`-block epochs, with `lc_contract_height` the
    /// block height of the LC contract state (0 for none).
    fn snapshot(
        lc_contract_height: u64,
        stored: &[u64],
        peer_block_heights: &[u64],
    ) -> SourcesSnapshot {
        SourcesSnapshot {
            blocks_per_epoch: EPOCH_HEIGHT,
            first_epoch: EpochNumber::new(FIRST_EPOCH),
            stored_epochs: stored.iter().copied().map(EpochNumber::new).collect(),
            lc_contract_state: LightClientStateSol {
                blockHeight: lc_contract_height,
                ..Default::default()
            },
            peer_block_heights: peer_block_heights.to_vec(),
        }
    }

    /// LC contract catchup of every LC contract epoch over the chain of `chain()`.
    fn catchup_for(state: &LightClientStateSol) -> LcContractCatchup {
        SourcesSnapshot {
            lc_contract_state: state.clone(),
            ..snapshot(0, &[], &[])
        }
        .plan()
        .lc_contract_catchup
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
        let snapshot = SourcesSnapshot {
            blocks_per_epoch,
            ..snapshot(lc_contract_height, &[], &[])
        };
        let epochs = snapshot
            .lc_contract_epochs()
            .map(|epochs| epochs.start().u64()..=epochs.end().u64());
        assert_eq!(epochs, expected);
    }

    /// `lc_contract_height` 77 is in epoch `N = 8`: the LC contract epochs are `5 ..= 9`. Epoch
    /// numbers of the peers' tips are those of the chain's last block.
    #[rstest]
    #[case::fresh_join(77, &[], &[78], Some(&[5, 6, 7, 8, 9][..]), 9, None)]
    #[case::fresh_join_behind_tip(77, &[], &[88], Some(&[5, 6, 7, 8, 9][..]), 9, Some((9, 9)))]
    #[case::restart_full_storage(77, &[5, 6, 7, 8, 9], &[78], None, 9, None)]
    #[case::restart_stale_storage(77, &[3, 4], &[78], Some(&[5, 6, 7, 8, 9][..]), 9, None)]
    #[case::restart_partial_storage(77, &[5, 6, 7], &[78], Some(&[8, 9][..]), 9, None)]
    #[case::rootless_epoch_in_storage(77, &[5, 6, 8, 9], &[78], Some(&[7][..]), 9, None)]
    #[case::rootless_top_epoch_in_storage(77, &[5, 6, 7, 8], &[78], Some(&[9][..]), 9, None)]
    #[case::restart_ahead_of_lc_contract(77, &[5, 6, 7, 8, 9, 10, 11], &[98], None, 11, None)]
    #[case::stored_above_with_gap(77, &[5, 6, 7, 8, 9, 12], &[78], None, 12, None)]
    #[case::lc_contract_at_root_boundary(75, &[], &[78], Some(&[5, 6, 7, 8, 9][..]), 9, None)]
    #[case::no_lc_contract_state(0, &[3, 4], &[88], None, 4, Some((9, 9)))]
    #[case::no_lc_contract_state_fresh(0, &[], &[88], None, 2, Some((9, 9)))]
    #[case::lc_contract_before_first_epoch_root(10, &[], &[88], None, 2, Some((9, 9)))]
    #[case::prover_lag_cap(77, &[], &[10_000], Some(&[5, 6, 7, 8, 9][..]), 9, Some((10, 1000)))]
    #[case::no_lc_contract_state_no_cap(0, &[], &[10_000], None, 2, Some((1000, 1000)))]
    #[case::cap_below_start(77, &[5, 6, 7, 8, 9, 10, 11], &[10_000], None, 11, Some((10, 1000)))]
    #[case::no_peer_heights(77, &[], &[], Some(&[5, 6, 7, 8, 9][..]), 9, None)]
    #[case::empty_peer_list(77, &[5, 6, 7, 8, 9], &[], None, 9, None)]
    #[case::empty_chain(77, &[], &[0], Some(&[5, 6, 7, 8, 9][..]), 9, None)]
    #[case::lying_high_peer(77, &[], &[78, 98], Some(&[5, 6, 7, 8, 9][..]), 9, Some((8, 10)))]
    #[case::median_ignores_liar(77, &[], &[88, 88, 10_000], Some(&[5, 6, 7, 8, 9][..]), 9, Some((9, 1000)))]
    #[case::even_takes_the_lower(77, &[], &[78, 10_000], Some(&[5, 6, 7, 8, 9][..]), 9, Some((8, 1000)))]
    #[case::liars_in_majority(77, &[], &[78, 10_000, 10_000], Some(&[5, 6, 7, 8, 9][..]), 9, Some((10, 1000)))]
    fn plan_cases(
        #[case] lc_contract_height: u64,
        #[case] stored: &[u64],
        #[case] peer_block_heights: &[u64],
        #[case] lc_contract_epochs: Option<&[u64]>,
        #[case] highest_held_epoch: u64,
        #[case] peer_tip_epochs: Option<(u64, u64)>,
    ) {
        let plan = snapshot(lc_contract_height, stored, peer_block_heights).plan();

        let missing = plan
            .lc_contract_catchup
            .map(|catchup| epoch_numbers(&catchup.missing_epochs));
        assert_eq!(missing.as_deref(), lc_contract_epochs);
        assert_eq!(plan.highest_held_epoch.u64(), highest_held_epoch);
        let peer_tip_epochs_planned = plan.peer_catchup.map(|catchup| {
            (
                catchup.median_peer_tip_epoch.u64(),
                catchup.highest_peer_tip_epoch.u64(),
            )
        });
        assert_eq!(peer_tip_epochs_planned, peer_tip_epochs);
    }

    /// Test chain of `LC_CONTRACT_HEIGHT + 1` blocks and the LC contract state over it.
    ///
    /// Peers report that height, so peer catchup targets epoch `N + 1`. `mock_block_height(0)`
    /// skips peer catchup.
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
        /// The epochs `load_stake` was asked for.
        asked: Arc<std::sync::Mutex<Vec<u64>>>,
    }

    impl SeededStore {
        /// Whether `load_stake` was asked for `epoch`.
        fn loaded(&self, epoch: u64) -> bool {
            self.asked.lock().unwrap().contains(&epoch)
        }

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
                asked: Arc::default(),
            }
        }
    }

    #[async_trait]
    impl MembershipPersistence for SeededStore {
        async fn load_stake(&self, epoch: EpochNumber) -> anyhow::Result<Option<StakeTuple>> {
            self.asked.lock().unwrap().push(epoch.u64());
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
        let outcome = catch_up_cliquenet_stake_tables(
            coordinator,
            EPOCH_HEIGHT,
            || async { lc_contract_state.clone().context("no lc_contract_state") },
            &peer_list(state_peers),
            &params,
        )
        .await?;
        Ok(outcome.current)
    }

    fn has_table(coordinator: &EpochMembershipCoordinator<SeqTypes>, epoch: u64) -> bool {
        coordinator
            .membership()
            .snapshot(EpochNumber::new(epoch))
            .is_some()
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

    #[test_log::test(tokio::test)]
    async fn invalid_proof_at_first_epoch_fails() {
        let (client, lc_contract_state) = chain().await;
        let catchup = catchup_for(&lc_contract_state);
        let first_root =
            root_block_in_epoch(catchup.missing_epochs.first().u64() - 2, EPOCH_HEIGHT);
        client.return_invalid_proof(first_root as usize).await;

        let err = catchup
            .fetch_proven_epoch_roots(&client, EPOCH_HEIGHT)
            .await
            .unwrap_err();

        assert_matches!(err, LcContractProofError::EpochRootInvalid { .. });
    }

    /// A verification failure is retried, not read as the tip, and the retry completes the catchup.
    #[test_log::test(tokio::test)]
    async fn lc_contract_catchup_retries_an_invalid_proof() {
        let (client, lc_contract_state) = chain().await;
        let coordinator = fresh_coordinator(&client).await;
        let root = root_block_in_epoch(N - 1, EPOCH_HEIGHT) as usize;
        client.return_invalid_proof(root).await;
        let heal = client.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            heal.remember_leaf(root).await;
        });

        let current = run_catchup(
            &coordinator,
            Some(lc_contract_state),
            &[client.clone(), client],
            PARAMS,
        )
        .await
        .unwrap();

        assert_eq!(current, EpochNumber::new(N));
        assert!(has_table(&coordinator, N + 1));
    }

    #[test_log::test(tokio::test)]
    async fn wrong_lc_contract_root_fails() {
        let (client, lc_contract_state) = chain().await;
        let catchup = LcContractCatchup {
            block_comm_root: catchup_for(&lc_contract_state).block_comm_root
                + CircuitField::from(1u64),
            ..catchup_for(&lc_contract_state)
        };

        let err = catchup
            .fetch_proven_epoch_roots(&client, EPOCH_HEIGHT)
            .await
            .unwrap_err();

        assert_matches!(err, LcContractProofError::RootMismatch { .. });
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

    /// After a restart `reload_stake` holds the epochs, so no working peer is needed.
    #[test_log::test(tokio::test)]
    async fn restart_boots_from_storage() {
        let (client, lc_contract_state) = chain().await;
        let store = SeededStore::for_chain(&client, (N - 4)..=(N + 1)).await;
        let mut committees = committees(store);
        committees.reload_stake(10).await;
        let coordinator = coordinator_over(committees);
        client.forget_leaf(LC_CONTRACT_HEIGHT as usize).await;

        let no_peers =
            run_catchup(&coordinator, Some(lc_contract_state.clone()), &[], PARAMS).await;
        let failing = run_catchup(
            &coordinator,
            Some(lc_contract_state),
            std::slice::from_ref(&client),
            PARAMS,
        )
        .await;

        assert_eq!(no_peers.unwrap(), EpochNumber::new(N));
        assert_eq!(failing.unwrap(), EpochNumber::new(N));
    }

    /// A failure below the epoch the peers' chain tip needs is retried, not read as the tip.
    #[test_log::test(tokio::test)]
    async fn peer_catchup_retries_below_the_median_tip() {
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

        let current = run_catchup(
            &coordinator,
            Some(LightClientStateSol::default()),
            &[client],
            PARAMS,
        )
        .await
        .unwrap();

        assert_eq!(current, EpochNumber::new(N));
        assert!(has_table(&coordinator, N + 1));
    }

    #[test_log::test(tokio::test)]
    async fn peer_catchup_below_the_median_tip_fails_at_the_deadline() {
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
                stage: Stage::PeerCatchup { epoch },
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

    /// Peers that agree on the tip and an uncapped median peer tip epoch end peer catchup there,
    /// without a probe past it, even though the store holds a later epoch.
    #[test_log::test(tokio::test)]
    async fn peer_catchup_stops_at_the_highest_tip() {
        let (client, lc_contract_state) = chain().await;
        client.mock_block_height(LC_CONTRACT_HEIGHT + 1).await;
        let store = SeededStore::for_chain(&client, (N - 3)..=(N + 2)).await;
        let coordinator = coordinator(store.clone());

        let current = run_catchup(&coordinator, Some(lc_contract_state), &[client], PARAMS)
            .await
            .unwrap();

        assert_eq!(current, EpochNumber::new(N));
        assert!(!store.loaded(N + 2));
    }

    /// A lying high peer does not fail peer catchup: epochs above the median tip are best-effort.
    #[test_log::test(tokio::test)]
    async fn lying_high_peer_does_not_fail_peer_catchup() {
        let (client, lc_contract_state) = chain().await;
        client.mock_block_height(LC_CONTRACT_HEIGHT + 1).await;
        let (liar, _) = chain().await;
        liar.mock_block_height(98).await;
        let coordinator = coordinator(SeededStore::for_chain(&client, (N - 3)..=(N + 2)).await);
        let started = Instant::now();

        let current = run_catchup(
            &coordinator,
            Some(lc_contract_state),
            &[client, liar],
            PARAMS,
        )
        .await
        .unwrap();

        assert_eq!(current, EpochNumber::new(N + 1));
        assert!(started.elapsed() < PARAMS.step_timeout);
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

    /// A peer that does not answer the first pass is asked again, and its height ends peer catchup.
    #[test_log::test(tokio::test(start_paused = true))]
    async fn silent_peer_answers_on_a_later_pass() {
        let (client, lc_contract_state) = chain().await;
        client.mock_block_height(LC_CONTRACT_HEIGHT + 1).await;
        client.fail_block_heights(2).await;
        let store = SeededStore::for_chain(&client, (N - 3)..=(N + 2)).await;
        let coordinator = coordinator(store.clone());

        let current = run_catchup(&coordinator, Some(lc_contract_state), &[client], PARAMS)
            .await
            .unwrap();

        assert_eq!(current, EpochNumber::new(N));
        assert!(!store.loaded(N + 2));
    }

    /// State peers are configured but none reports a height: startup continues from storage, within
    /// the deadline.
    #[rstest]
    #[case::default_deadline(PARAMS.total_timeout)]
    #[case::short_deadline(PEER_HEIGHT_BUDGET * 2)]
    #[test_log::test(tokio::test(start_paused = true))]
    async fn no_peer_height_starts_from_storage(#[case] deadline: Duration) {
        let (client, lc_contract_state) = chain().await;
        let store = SeededStore::for_chain(&client, (N - 4)..=(N + 1)).await;
        let mut committees = committees(store);
        committees.reload_stake(10).await;
        let coordinator = coordinator_over(committees);
        client.fail_block_heights(usize::MAX).await;
        let params = StartupCatchupParams {
            total_timeout: deadline,
            ..PARAMS
        };
        let started = Instant::now();

        let current = run_catchup(&coordinator, Some(lc_contract_state), &[client], params)
            .await
            .unwrap();

        assert_eq!(current, EpochNumber::new(N));
        assert!(started.elapsed() <= deadline);
    }

    /// LC contract catchup fills fresh storage, then no peer reports a height: startup ends at
    /// the LC contract epoch, without peer catchup.
    #[test_log::test(tokio::test(start_paused = true))]
    async fn lc_contract_catchup_then_no_peer_height_skips_peer_catchup() {
        let (client, lc_contract_state) = chain().await;
        let store = SeededStore::for_chain(&client, (N - 3)..=(N + 2)).await;
        let coordinator = coordinator(store.clone());
        client.fail_block_heights(usize::MAX).await;

        let current = run_catchup(&coordinator, Some(lc_contract_state), &[client], PARAMS)
            .await
            .unwrap();

        assert_eq!(current, EpochNumber::new(N));
        for epoch in (N - 3)..=(N + 1) {
            assert!(has_table(&coordinator, epoch), "epoch {epoch}");
        }
        assert!(!store.loaded(N + 2));
    }

    /// A spent deadline fails the peer height read instead of reading as no peer answering.
    #[test_log::test(tokio::test)]
    async fn spent_deadline_fails_the_peer_height_read() {
        let (client, _) = chain().await;
        let coordinator = fresh_coordinator(&client).await;
        let peers = peer_list(&[client]);
        let catchup = StartupCatchup {
            coordinator: &coordinator,
            state_peers: &peers,
            params: &PARAMS,
            blocks_per_epoch: EPOCH_HEIGHT,
            deadline: Instant::now(),
        };

        let err = catchup.read_peer_block_heights().await.unwrap_err();

        assert_matches!(
            err,
            StartupCatchupError::DeadlineExceeded {
                stage: Stage::PeerHeights,
                ..
            }
        );
    }

    /// The deadline falling inside the height budget fails the read instead of reading as no peer
    /// answering.
    #[test_log::test(tokio::test(start_paused = true))]
    async fn deadline_inside_the_height_budget_fails_the_peer_height_read() {
        let (client, _) = chain().await;
        let coordinator = fresh_coordinator(&client).await;
        client.fail_block_heights(usize::MAX).await;
        let peers = peer_list(&[client]);
        let catchup = StartupCatchup {
            coordinator: &coordinator,
            state_peers: &peers,
            params: &PARAMS,
            blocks_per_epoch: EPOCH_HEIGHT,
            deadline: Instant::now() + PEER_HEIGHT_BUDGET / 2,
        };

        let err = catchup.read_peer_block_heights().await.unwrap_err();

        assert_matches!(
            err,
            StartupCatchupError::DeadlineExceeded {
                stage: Stage::PeerHeights,
                ..
            }
        );
    }

    /// A peer ahead of the median extends peer catchup, best-effort, to the epoch of its tip.
    #[test_log::test(tokio::test)]
    async fn peer_catchup_continues_to_the_highest_tip() {
        let (client, lc_contract_state) = chain().await;
        client.mock_block_height(LC_CONTRACT_HEIGHT + 1).await;
        let (ahead, _) = chain().await;
        ahead.mock_block_height(98).await;
        let store = SeededStore::for_chain(&client, (N - 3)..=(N + 3)).await;
        let coordinator = coordinator(store.clone());

        let current = run_catchup(
            &coordinator,
            Some(lc_contract_state),
            &[client, ahead],
            PARAMS,
        )
        .await
        .unwrap();

        assert_eq!(current, EpochNumber::new(N + 2));
        assert!(!store.loaded(N + 4));
    }

    /// LC contract epochs cannot be fetched without peers, so startup fails at once.
    #[test_log::test(tokio::test)]
    async fn lc_contract_catchup_without_peers_fails_immediately() {
        let (client, lc_contract_state) = chain().await;
        let coordinator = fresh_coordinator(&client).await;
        let started = Instant::now();

        let err = run_catchup(&coordinator, Some(lc_contract_state), &[], PARAMS)
            .await
            .unwrap_err();

        assert_matches!(
            err,
            StartupCatchupError::NoStatePeers { lc_contract_height } if lc_contract_height == LC_CONTRACT_HEIGHT
        );
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    /// Without LC contract state there is nothing to fetch, so no peers is fine and peer catchup is
    /// skipped.
    #[test_log::test(tokio::test)]
    async fn genesis_lc_contract_state_without_peers_skips_peer_catchup() {
        let (client, _) = chain().await;
        let coordinator = fresh_coordinator(&client).await;

        let current = run_catchup(
            &coordinator,
            Some(LightClientStateSol::default()),
            &[],
            PARAMS,
        )
        .await
        .unwrap();

        assert_eq!(current, EpochNumber::new(FIRST_EPOCH));
        assert!(!has_table(&coordinator, N - 3));
    }

    /// An LC contract state read that keeps failing is retried until the deadline, not read as no
    /// LC contract state.
    #[test_log::test(tokio::test)]
    async fn lc_contract_state_failure_aborts_at_the_deadline() {
        let (client, _) = chain().await;
        let coordinator = fresh_coordinator(&client).await;
        let params = StartupCatchupParams {
            total_timeout: Duration::from_millis(300),
            ..PARAMS
        };

        let err = run_catchup(&coordinator, None, &[client], params)
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
                Ok(lc_contract_state.clone())
            },
            &peer_list(&[client]),
            &PARAMS,
        )
        .await
        .unwrap()
        .current;

        assert_eq!(reads.load(Ordering::Relaxed), 2);
        assert_eq!(current, EpochNumber::new(N));
        assert!(has_table(&coordinator, N + 1));
    }

    /// A peer that reports a huge chain cannot make peer catchup require more than the slack past
    /// the LC contract epoch. Past it peer catchup is best-effort and ends at the first epoch peers
    /// cannot serve.
    #[test_log::test(tokio::test)]
    async fn huge_peer_height_is_capped_by_the_lc_contract_header() {
        let (client, lc_contract_state) = chain().await;
        client.mock_block_height(10_000).await;
        let store =
            SeededStore::for_chain(&client, (N - 3)..=(N + 1 + MAX_TIP_EPOCHS_PAST_LC_CONTRACT))
                .await;
        let coordinator = coordinator(store.clone());

        let current = run_catchup(&coordinator, Some(lc_contract_state), &[client], PARAMS)
            .await
            .unwrap();

        assert_eq!(
            current,
            EpochNumber::new(N + MAX_TIP_EPOCHS_PAST_LC_CONTRACT)
        );
        assert!(store.loaded(N + 2 + MAX_TIP_EPOCHS_PAST_LC_CONTRACT));
    }
}
