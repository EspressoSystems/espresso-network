//! Startup stake-table catchup for new-protocol nodes.
//!
//! Purpose: bring the node to the stake tables it needs at the current epoch `N`, the epochs
//! cliquenet dials and the ones their rewards read. Until cliquenet dials them no consensus message
//! arrives. From there consensus and request-response over cliquenet take over, and the configured
//! state peers are not used anymore.
//!
//! Model: a stake table is the L1 stake-table events up to the cutoff L1 block in its epoch root
//! header (`l1_finalized`). The root of epoch `e-2` derives the table of epoch `e`, through
//! `add_epoch_root`. Each missing epoch is filled from the first source that serves it:
//!
//! 1. Memory or storage: a held table, or a stored root header re-derived with L1 events.
//! 2. LC contract proof: the root header proven by a Merkle proof against the header `F` the LC
//!    contract (the `LightClient` contract on L1) holds.
//! 3. Derivation: QC-verified leaves from the state peers, one epoch at a time.
//!
//! Assumptions:
//!
//! - Epochs are always on: `epoch_height > 0`. No pre-epoch chains exist.
//! - L1 is reachable.
//! - State peers serve the availability and light-client APIs. A follower passes its upstreams;
//!   `StatePeersUnavailable` is judged on the list passed in.
//! - Failure at [`STARTUP_CATCHUP_DEADLINE`] exits the validator. Operators restart it.
//!
//! Validator: [`catch_up_cliquenet_stake_tables`] takes `N` as the maximum of the floor (`first`,
//! the epoch of `F`, the epoch of the anchor leaf) and the median tip of the state peers.
//!
//! Follower: not on cliquenet, so its upstreams stay its source. Before it stores a block it runs
//! [`ensure_stake_tables`] for the epoch of the block.

use std::{ops::RangeInclusive, time::Duration};

use anyhow::{Context, anyhow, ensure};
use derive_more::Display;
use espresso_types::{Header, LcContractHeader, SeqTypes};
use futures::future::join_all;
use hotshot_query_service::node::BlockId;
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    traits::{block_contents::BlockHeader, election::Membership},
    utils::{epoch_from_block_number, root_block_in_epoch},
};
use light_client::client::{Client, QueryServiceClient};
use tokio::{
    sync::OnceCell,
    time::{Instant, sleep, timeout, timeout_at},
};
use url::Url;

/// First wait between two retries, doubled up to `MAX_BACKOFF`.
const INITIAL_BACKOFF: Duration = Duration::from_secs(1);
/// Longest wait between two retries.
const MAX_BACKOFF: Duration = Duration::from_secs(60);

/// Total budget of every retried stage of one call. At the deadline the caller exits.
pub(crate) const STARTUP_CATCHUP_DEADLINE: Duration = Duration::from_secs(15 * 60);

/// Validator boot. `F` read once; `floor = max(first, epoch(F), anchor_epoch)`;
/// `install_stored_roots(required_epochs(floor))`; peer heights, retried only when that returned
/// `false` and state peers are configured, else one pass (no nonzero height logs `info` at
/// `floor == first`, `warn` above); `N = max(floor, median tip)`; `ensure(required_epochs(floor))`
/// then `ensure(required_epochs(N))`. Returns `N`.
pub(crate) async fn catch_up_cliquenet_stake_tables<C: Client, Fut>(
    coordinator: &EpochMembershipCoordinator<SeqTypes>,
    read_lc_contract_header: impl Fn() -> Fut,
    state_peers: &[(Url, C)],
    params: &StartupCatchupParams,
    anchor_epoch: Option<EpochNumber>,
) -> Result<EpochNumber, StartupCatchupError>
where
    Fut: Future<Output = anyhow::Result<Option<LcContractHeader>>>,
{
    let started = Instant::now();
    let catchup = StartupCatchup::new(coordinator, state_peers, params, read_lc_contract_header)?;
    let first = catchup.first_epoch;
    let lc_contract_header = catchup.lc_contract_header().await?;
    let lc_contract_epoch =
        lc_contract_header.map(|header| block_epoch(header.height, catchup.epoch_height));
    let floor = current_epoch_floor(first, lc_contract_epoch, anchor_epoch);
    let floor_epochs = required_epochs(floor, first);

    let own_sources_suffice = catchup.install_stored_roots(&floor_epochs).await?;
    let heights = if own_sources_suffice || state_peers.is_empty() {
        let remaining = catchup.deadline.saturating_duration_since(Instant::now());
        read_peer_block_heights(state_peers, params.step_timeout.min(remaining)).await
    } else {
        catchup.peer_block_heights_retried().await?
    };
    let median_tip_epoch = median_peer_tip_epoch(&heights, catchup.epoch_height);
    if heights.is_empty() && own_sources_suffice {
        log_no_peer_height(floor, first);
    }
    let current = median_tip_epoch.map_or(floor, |median| median.max(floor));
    tracing::info!(
        target: "announce::startup_catchup",
        %floor,
        ?lc_contract_epoch,
        ?anchor_epoch,
        ?median_tip_epoch,
        current_epoch = %current,
        "current epoch"
    );

    catchup.ensure(floor_epochs).await?;
    catchup.ensure(required_epochs(current, first)).await?;
    tracing::info!(
        target: "announce::startup_catchup",
        current_epoch = %current,
        duration_ms = started.elapsed().as_millis() as u64,
        "cliquenet stake tables ready",
    );
    Ok(current)
}

/// Follower, before a block of epoch `current` is stored, with a fresh deadline:
/// `install_stored_roots(required_epochs(current))`; on a miss `F` is read, then
/// `ensure(required_epochs(epoch(F)))` when `epoch(F) < current`, then
/// `ensure(required_epochs(current))`.
pub(crate) async fn ensure_stake_tables<C: Client, Fut>(
    coordinator: &EpochMembershipCoordinator<SeqTypes>,
    read_lc_contract_header: impl Fn() -> Fut,
    state_peers: &[(Url, C)],
    params: &StartupCatchupParams,
    current: EpochNumber,
) -> Result<(), StartupCatchupError>
where
    Fut: Future<Output = anyhow::Result<Option<LcContractHeader>>>,
{
    let catchup = StartupCatchup::new(coordinator, state_peers, params, read_lc_contract_header)?;
    let need = required_epochs(current, catchup.first_epoch);
    if catchup.install_stored_roots(&need).await? {
        return Ok(());
    }
    if !state_peers.is_empty()
        && let Some(header) = catchup.lc_contract_header().await?
    {
        let lc_contract_epoch = block_epoch(header.height, catchup.epoch_height);
        if lc_contract_epoch < current {
            catchup
                .ensure(required_epochs(lc_contract_epoch, catchup.first_epoch))
                .await?;
        }
    }
    catchup.ensure(need).await
}

/// One `QueryServiceClient` per URL, to read block heights and light-client proofs from.
pub(crate) fn state_peer_clients(urls: &[Url]) -> Vec<(Url, QueryServiceClient)> {
    urls.iter()
        .map(|url| (url.clone(), QueryServiceClient::new(url.clone())))
        .collect()
}

/// Lowest `N` the node accepts: `first` raised by the LC contract epoch and the anchor epoch.
/// A `None` bound is absent. Pure.
fn current_epoch_floor(
    first: EpochNumber,
    lc_contract_epoch: Option<EpochNumber>,
    anchor_epoch: Option<EpochNumber>,
) -> EpochNumber {
    lc_contract_epoch
        .into_iter()
        .chain(anchor_epoch)
        .fold(first, EpochNumber::max)
}

/// Tables a node at `current` holds: `max(first + 2, current - 4) ..= current + 1`, saturating.
/// `current - 4`: the reward of `current - 1` reads the table of `current - 4`. Empty when
/// `current + 1 < first + 2`. Pure.
fn required_epochs(current: EpochNumber, first: EpochNumber) -> RangeInclusive<EpochNumber> {
    let start = (first.u64() + 2).max(current.u64().saturating_sub(4));
    EpochNumber::new(start)..=EpochNumber::new(current.u64() + 1)
}

/// Whether the root deriving `epoch`, `root(epoch - 2)`, has a proof against `F`:
/// `root_block_in_epoch(epoch - 2, H) < header.height`, `epoch - 2` saturating. Callers pass
/// `epoch >= first + 2`. Below `first + 2` the root block is 0 and the test is
/// `0 < header.height`. Pure.
fn provable_from_lc_contract(
    epoch: EpochNumber,
    header: &LcContractHeader,
    epoch_height: u64,
) -> bool {
    root_block_in_epoch(epoch.u64().saturating_sub(2), epoch_height) < header.height
}

/// Epoch of block `height`.
fn block_epoch(height: u64, epoch_height: u64) -> EpochNumber {
    EpochNumber::new(epoch_from_block_number(height, epoch_height))
}

/// No nonzero peer height in one pass: the floor is the only evidence of the current epoch.
fn log_no_peer_height(floor: EpochNumber, first: EpochNumber) {
    if floor == first {
        tracing::info!(
            target: "announce::startup_catchup",
            "no state peer reported a block height; starting at the first epoch"
        );
    } else {
        tracing::warn!(
            target: "announce::startup_catchup",
            %floor,
            "no state peer reported a block height; starting at the floor"
        );
    }
}

/// The epochs of `range`, ascending.
fn epochs(range: &RangeInclusive<EpochNumber>) -> impl Iterator<Item = EpochNumber> {
    (range.start().u64()..=range.end().u64()).map(EpochNumber::new)
}

/// Epoch of the last block of the median peer chain, lower median. `None` for no heights or a
/// median of 0. Callers pass the heights of `read_peer_block_heights`, nonzero. Pure.
fn median_peer_tip_epoch(peer_block_heights: &[u64], blocks_per_epoch: u64) -> Option<EpochNumber> {
    let mut heights = peer_block_heights.to_vec();
    heights.sort_unstable();
    let median = heights.get(heights.len().checked_sub(1)? / 2)?;
    let tip = median.checked_sub(1)?;
    Some(block_epoch(tip, blocks_per_epoch))
}

/// What every stage shares; `first_epoch` resolved once; `F` read at most once per call.
struct StartupCatchup<'a, C, R> {
    /// Membership to fill, with the catchup machinery behind it.
    coordinator: &'a EpochMembershipCoordinator<SeqTypes>,
    /// State peers serving block heights and light-client proofs.
    state_peers: &'a [(Url, C)],
    /// Step timeout and total timeout.
    params: &'a StartupCatchupParams,
    /// Reads `F`; `None` without an LC contract.
    read_lc_contract_header: R,
    /// `F` once read; `None` inside means no LC contract.
    lc_contract_header: OnceCell<Option<LcContractHeader>>,
    /// Blocks in one epoch; positive.
    epoch_height: u64,
    /// First epoch; it and the next are seeded from genesis.
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
    /// `FirstEpochUnset` without a first epoch, `EpochHeightZero` for `epoch_height == 0`.
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
        let epoch_height = coordinator.epoch_height().u64();
        if epoch_height == 0 {
            return Err(StartupCatchupError::EpochHeightZero);
        }
        Ok(Self {
            coordinator,
            state_peers,
            params,
            read_lc_contract_header,
            lc_contract_header: OnceCell::new(),
            epoch_height,
            first_epoch,
            deadline: Instant::now() + params.total_timeout,
        })
    }

    /// `F`, read once per call through `retry` (Stage `LcContractHeaderRead`), each attempt
    /// bounded by `step_timeout`.
    async fn lc_contract_header(&self) -> Result<Option<LcContractHeader>, StartupCatchupError> {
        let header = self
            .lc_contract_header
            .get_or_try_init(|| self.read_lc_contract_header_logged())
            .await?;
        Ok(header.clone())
    }

    /// The retried read of `F`, with its log line.
    async fn read_lc_contract_header_logged(
        &self,
    ) -> Result<Option<LcContractHeader>, StartupCatchupError> {
        let header = retry(
            self.deadline,
            Stage::LcContractHeaderRead,
            |_, remaining| {
                let read = timeout(
                    self.params.step_timeout.min(remaining),
                    (self.read_lc_contract_header)(),
                );
                async move { read.await.context("attempt timed out")? }
            },
        )
        .await?;
        match &header {
            Some(header) => tracing::info!(
                target: "announce::startup_catchup",
                height = header.height,
                epoch = %block_epoch(header.height, self.epoch_height),
                "lc contract header"
            ),
            None => tracing::info!(
                target: "announce::startup_catchup",
                header = "none",
                "lc contract header"
            ),
        }
        Ok(header)
    }

    /// In memory or in storage as a table. No side effect.
    async fn held(&self, epoch: EpochNumber) -> bool {
        self.coordinator.membership().load_stake_table(epoch).await
    }

    /// Own sources for `range`, ascending: each epoch not held is `install_from_stored_root`ed.
    /// `Ok(true)` iff every epoch of `range` was held or installed from its stored root. No peer.
    async fn install_stored_roots(
        &self,
        range: &RangeInclusive<EpochNumber>,
    ) -> Result<bool, StartupCatchupError> {
        let mut all_held = true;
        for epoch in epochs(range) {
            all_held &= self.held(epoch).await || self.install_from_stored_root(epoch).await?;
        }
        Ok(all_held)
    }

    /// `retry` (Stage `PeerHeights`) around `read_peer_block_heights` until one height arrives,
    /// each pass bounded by `step_timeout`. The one-pass variant is `read_peer_block_heights`
    /// itself; the caller picks.
    async fn peer_block_heights_retried(&self) -> Result<Vec<u64>, StartupCatchupError> {
        retry(
            self.deadline,
            Stage::PeerHeights,
            |_, remaining| async move {
                let step_timeout = self.params.step_timeout.min(remaining);
                let heights = read_peer_block_heights(self.state_peers, step_timeout).await;
                ensure!(!heights.is_empty(), "no state peer reported a block height");
                Ok(heights)
            },
        )
        .await
    }

    /// `Ok` iff every epoch of `need` is held, ascending. Per missing epoch: stored root, else
    /// `StatePeersUnavailable { epoch }` without state peers, else proof when provable, else
    /// derivation. Each source is one retry unit.
    async fn ensure(&self, need: RangeInclusive<EpochNumber>) -> Result<(), StartupCatchupError> {
        for epoch in epochs(&need) {
            self.ensure_epoch(epoch).await?;
        }
        Ok(())
    }

    /// One epoch of `ensure`: the first source that serves it.
    async fn ensure_epoch(&self, epoch: EpochNumber) -> Result<(), StartupCatchupError> {
        if self.held(epoch).await || self.install_from_stored_root(epoch).await? {
            return Ok(());
        }
        if self.state_peers.is_empty() {
            return Err(StartupCatchupError::StatePeersUnavailable { epoch });
        }
        if self.install_proven_epoch_root(epoch).await? {
            return Ok(());
        }
        self.derive_stake_table(epoch).await
    }

    /// `Ok(false)` when no root header for `epoch` is stored; `StorageRead` when the read fails;
    /// else `retry` (Stage `StoredRootDerivation`) around `coordinator.add_epoch_root(header)`,
    /// L1 events only, each attempt bounded by the time left.
    async fn install_from_stored_root(
        &self,
        epoch: EpochNumber,
    ) -> Result<bool, StartupCatchupError> {
        let header = self
            .coordinator
            .membership()
            .stored_epoch_root(epoch)
            .await
            .map_err(|source| StartupCatchupError::StorageRead { epoch, source })?;
        let Some(header) = header else {
            return Ok(false);
        };
        retry(
            self.deadline,
            Stage::StoredRootDerivation { epoch },
            |_, remaining| {
                let add = timeout(remaining, self.coordinator.add_epoch_root(header.clone()));
                async move {
                    add.await
                        .context("attempt timed out")?
                        .map_err(anyhow::Error::from)
                }
            },
        )
        .await?;
        tracing::info!(target: "announce::startup_catchup", %epoch, "installed from stored root");
        Ok(true)
    }

    /// `Ok(false)` when `F` (from `lc_contract_header`, the cell) is `None` or `epoch` is not
    /// `provable_from_lc_contract`; else `retry` (Stage `LcContractProof`): the proven root of
    /// `epoch` from the peer `attempt % len`, fetch bounded by `step_timeout`, then
    /// `add_epoch_root` bounded by the time left, and `Ok(true)`.
    async fn install_proven_epoch_root(
        &self,
        epoch: EpochNumber,
    ) -> Result<bool, StartupCatchupError> {
        let Some(lc_contract) = self.lc_contract_header().await? else {
            return Ok(false);
        };
        if !provable_from_lc_contract(epoch, &lc_contract, self.epoch_height) {
            return Ok(false);
        }
        retry(
            self.deadline,
            Stage::LcContractProof { epoch },
            |attempt, remaining| {
                let lc_contract = &lc_contract;
                async move {
                    let (url, peer) = &self.state_peers[attempt % self.state_peers.len()];
                    let fetch =
                        fetch_proven_epoch_root(peer, lc_contract, epoch, self.epoch_height);
                    let header = timeout(self.params.step_timeout.min(remaining), fetch)
                        .await
                        .context("attempt timed out")?
                        .with_context(|| format!("peer {url}"))?;
                    timeout_at(self.deadline, self.coordinator.add_epoch_root(header))
                        .await
                        .context("adding the epoch root timed out")?
                        .map_err(anyhow::Error::from)
                }
            },
        )
        .await?;
        tracing::info!(
            target: "announce::startup_catchup",
            %epoch,
            "installed from lc contract proof"
        );
        Ok(true)
    }

    /// `retry` (Stage `Derivation`) around `coordinator.wait_for_stake_table(epoch)`, each
    /// attempt bounded by the time left. A timed-out attempt may leave the coordinator's fetch
    /// task running.
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
                        .map_err(anyhow::Error::from)
                }
            },
        )
        .await?;
        tracing::info!(target: "announce::startup_catchup", %epoch, "derived from peers");
        Ok(())
    }
}

/// The nonzero block height each state peer reports. One pass, each read bounded by
/// `step_timeout`; failures and a height of 0 (no chain yet) logged once and omitted.
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
            Ok(0) => tracing::info!(target: "announce::startup_catchup", %url, "peer has no blocks"),
            Ok(height) => {
                tracing::info!(target: "announce::startup_catchup", %url, height, "peer block height")
            },
            Err(err) => tracing::info!(
                target: "announce::startup_catchup",
                %url,
                err = %format_args!("{err:#}"),
                "peer block height unavailable"
            ),
        }
        height.ok().filter(|height| *height > 0)
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
                err = %format_args!("{last_err:#}"),
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
    /// Bound on one LC contract header read, one proof fetch, one peer height read.
    pub step_timeout: Duration,
    /// How long all stages may keep retrying, together.
    pub total_timeout: Duration,
}

/// Why the startup catchup failed.
#[derive(Debug, thiserror::Error)]
pub(crate) enum StartupCatchupError {
    /// The membership has no first epoch to seed from.
    #[error("first_epoch not seeded; genesis stake table missing")]
    FirstEpochUnset,

    /// `epoch_height == 0`; epochs are always on.
    #[error("epoch_height is 0; epochs are always on")]
    EpochHeightZero,

    /// `epoch` is missing, has no stored root, and no state peer is configured.
    #[error(
        "the stake table of epoch {epoch} is missing; startup catchup needs a state peer serving \
         the availability and light-client APIs"
    )]
    StatePeersUnavailable { epoch: EpochNumber },

    /// The stored root header of `epoch` could not be read.
    #[error("reading the stored epoch root of epoch {epoch}")]
    StorageRead {
        epoch: EpochNumber,
        #[source]
        source: anyhow::Error,
    },

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
    /// Reading the block heights of the state peers.
    #[display("reading the state peers' block heights")]
    PeerHeights,
    /// Deriving the stake table of `epoch` from its stored root header.
    #[display("deriving the stake table of epoch {epoch} from its stored root")]
    StoredRootDerivation { epoch: EpochNumber },
    /// Fetching and adding the proven root deriving `epoch`.
    #[display("fetching the LC contract proof for epoch {epoch}")]
    LcContractProof { epoch: EpochNumber },
    /// Deriving the stake table of `epoch` from peer QC leaves.
    #[display("deriving the stake table of epoch {epoch}")]
    Derivation { epoch: EpochNumber },
}

/// The root deriving `epoch`, `root(epoch - 2)`, from one `peer`: the header at `F.height`
/// verified against `block_comm_root`, then the Merkle proof of the root, then the height check.
async fn fetch_proven_epoch_root<C: Client>(
    peer: &C,
    lc_contract: &LcContractHeader,
    epoch: EpochNumber,
    epoch_height: u64,
) -> Result<Header, LcContractProofError> {
    let header = peer
        .unverified_header(lc_contract.height)
        .await
        .map_err(LcContractProofError::HeaderFetch)?;
    verify_lc_contract_header(lc_contract, &header)?;
    let root = header.block_merkle_tree_root();

    let height = root_block_in_epoch(epoch.u64().saturating_sub(2), epoch_height);
    let proof = peer
        .header_proof(lc_contract.height, BlockId::Number(height as usize))
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
}

/// Accept `header` only if it is the block the LC contract commits to at `F`.
///
/// The view number does not enter `block_comm_root`, so any view serves.
fn verify_lc_contract_header(
    lc_contract: &LcContractHeader,
    header: &Header,
) -> Result<(), LcContractProofError> {
    if header.height() != lc_contract.height {
        return Err(LcContractProofError::HeaderHeight {
            got: header.height(),
            expected: lc_contract.height,
        });
    }
    let state = header
        .get_light_client_state(ViewNumber::genesis())
        .map_err(LcContractProofError::HeaderState)?;
    if state.block_comm_root != lc_contract.block_comm_root {
        return Err(LcContractProofError::RootMismatch {
            height: lc_contract.height,
        });
    }
    Ok(())
}

/// Why a peer's LC contract header or epoch root proof was refused.
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
            atomic::{AtomicBool, AtomicUsize, Ordering},
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
    /// Epoch of the LC contract header and of the peers' tip: the required epochs are `N-4 ..= N+1`.
    const N: u64 = 8;
    /// In epoch `N`, past its root block (75).
    const LC_CONTRACT_HEIGHT: u64 = 77;
    /// In epoch `N-1`, before its root block (65).
    const LC_CONTRACT_HEIGHT_BEFORE_ROOT: u64 = 62;

    const PARAMS: StartupCatchupParams = StartupCatchupParams {
        step_timeout: Duration::from_secs(5),
        total_timeout: Duration::from_secs(30),
    };

    /// `PARAMS` with a deadline that tests of failing stages reach quickly.
    const SHORT_DEADLINE_PARAMS: StartupCatchupParams = StartupCatchupParams {
        step_timeout: Duration::from_millis(100),
        total_timeout: Duration::from_millis(300),
    };

    fn epoch_range(range: &RangeInclusive<EpochNumber>) -> Option<RangeInclusive<u64>> {
        (!range.is_empty()).then(|| range.start().u64()..=range.end().u64())
    }

    /// `first_epoch = 1` seeds epochs 1 and 2.
    #[rstest]
    #[case::nothing(None, None, 1)]
    #[case::lc_contract_below_first(Some(0), None, 1)]
    #[case::lc_contract_above(Some(5), None, 5)]
    #[case::anchor_above_lc_contract(Some(5), Some(7), 7)]
    #[case::anchor_below_lc_contract(Some(7), Some(5), 7)]
    #[case::anchor_only(None, Some(4), 4)]
    fn current_epoch_floor_cases(
        #[case] lc_contract_epoch: Option<u64>,
        #[case] anchor_epoch: Option<u64>,
        #[case] expected: u64,
    ) {
        let floor = current_epoch_floor(
            EpochNumber::new(FIRST_EPOCH),
            lc_contract_epoch.map(EpochNumber::new),
            anchor_epoch.map(EpochNumber::new),
        );
        assert_eq!(floor, EpochNumber::new(expected));
    }

    /// `first_epoch = 1` seeds epochs 1 and 2.
    #[rstest]
    #[case::first_epoch(1, None)]
    #[case::second_epoch(2, Some(3..=3))]
    #[case::first_plus_two(3, Some(3..=4))]
    #[case::near_genesis(5, Some(3..=6))]
    #[case::first_plus_five(6, Some(3..=7))]
    #[case::mid_chain(8, Some(4..=9))]
    #[case::large(1000, Some(996..=1001))]
    fn required_epochs_cases(#[case] current: u64, #[case] expected: Option<RangeInclusive<u64>>) {
        let required = required_epochs(EpochNumber::new(current), EpochNumber::new(FIRST_EPOCH));
        assert_eq!(epoch_range(&required), expected);
    }

    /// The root deriving epoch 10 is block 75.
    #[rstest]
    #[case::past_the_root(10, 77, true)]
    #[case::one_past_the_root(10, 76, true)]
    #[case::at_the_root(10, 75, false)]
    #[case::before_the_root(10, 40, false)]
    #[case::below_first_plus_two(2, 1, true)]
    #[case::below_first_plus_two_at_genesis(2, 0, false)]
    fn provable_from_lc_contract_cases(
        #[case] epoch: u64,
        #[case] lc_contract_height: u64,
        #[case] expected: bool,
    ) {
        let header = LcContractHeader {
            height: lc_contract_height,
            block_comm_root: CircuitField::from(0u64),
        };
        let provable = provable_from_lc_contract(EpochNumber::new(epoch), &header, EPOCH_HEIGHT);
        assert_eq!(provable, expected);
    }

    #[rstest]
    #[case::no_peers(&[], None)]
    #[case::odd_count(&[78, 18, 38], Some(4))]
    #[case::even_count_lower_median(&[18, 98, 38, 78], Some(4))]
    #[case::one_high_outlier(&[78, 10_000, 78], Some(8))]
    #[case::one_empty_chain(&[0], None)]
    fn median_peer_tip_epoch_cases(#[case] heights: &[u64], #[case] expected: Option<u64>) {
        let epoch = median_peer_tip_epoch(heights, EPOCH_HEIGHT).map(|epoch| epoch.u64());
        assert_eq!(epoch, expected);
    }

    /// The LC contract header over the block at `height` of `client`.
    async fn lc_header_at(client: &TestClient, height: u64) -> LcContractHeader {
        let header = client.unverified_header(height).await.unwrap();
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
        let header = client.unverified_header(LC_CONTRACT_HEIGHT).await.unwrap();
        verify_lc_contract_header(&lc_header, &header).unwrap();

        let wrong_root = LcContractHeader {
            block_comm_root: lc_header.block_comm_root + CircuitField::from(1u64),
            ..lc_header.clone()
        };
        let err = verify_lc_contract_header(&wrong_root, &header).unwrap_err();
        assert_matches!(
            err,
            LcContractProofError::RootMismatch { height } if height == LC_CONTRACT_HEIGHT
        );

        let wrong_height = LcContractHeader {
            height: LC_CONTRACT_HEIGHT + 1,
            ..lc_header
        };
        let err = verify_lc_contract_header(&wrong_height, &header).unwrap_err();
        assert_matches!(
            err,
            LcContractProofError::HeaderHeight { got, expected }
                if got == LC_CONTRACT_HEIGHT && expected == LC_CONTRACT_HEIGHT + 1
        );
    }

    /// A peer at height 0 has no chain yet and does not count.
    #[tokio::test]
    async fn read_peer_block_heights_omits_zero_heights() {
        let (client, _) = chain().await;
        let (empty, _) = chain().await;
        empty.mock_block_height(0).await;

        let heights = read_peer_block_heights(
            &peer_list(&[empty.clone(), client.clone(), empty]),
            PARAMS.step_timeout,
        )
        .await;

        assert_eq!(heights, vec![LC_CONTRACT_HEIGHT + 1]);
    }

    /// Stands in for the node's database: it holds a stake table, DRB result and epoch root header
    /// for each epoch the test lets it answer for, with the hash a verified epoch root header
    /// commits to.
    ///
    /// `for_chain` holds tables only; `with_roots` adds the stored root header of epochs.
    ///
    /// Tables answer from `visible` epochs only. `hidden` starts with none, and `add_epoch_root`
    /// reveals the table of the epoch it adds by storing its root first. `visible`, `hidden()` and
    /// `reveal()` stand in for the L1 fetch of a stored epoch root; `store_stake` is a no-op.
    #[derive(Clone, Debug, Default)]
    struct SeededStore {
        tables: Arc<std::sync::Mutex<HashMap<u64, StakeTuple>>>,
        roots: Arc<std::sync::Mutex<HashMap<u64, Header>>>,
        visible: Arc<std::sync::Mutex<HashSet<u64>>>,
        fail_root_reads: Arc<AtomicBool>,
    }

    impl SeededStore {
        async fn for_chain(client: &TestClient, epochs: RangeInclusive<u64>) -> Self {
            let validators: AuthenticatedValidatorMap = (0..4)
                .map(|_| random_validator())
                .map(|validator| (validator.account, validator))
                .collect();
            let mut tables = HashMap::new();
            for epoch in epochs {
                // The root of epoch `e - 1` commits to the stake table of epoch `e`.
                let root = root_block_in_epoch(epoch - 1, EPOCH_HEIGHT);
                let hash = client
                    .unverified_header(root)
                    .await
                    .unwrap()
                    .next_stake_table_hash();
                tables.insert(
                    epoch,
                    (validators.clone(), Some(RewardAmount::default()), hash),
                );
            }
            let visible = tables.keys().copied().collect();
            Self {
                tables: Arc::new(std::sync::Mutex::new(tables)),
                visible: Arc::new(std::sync::Mutex::new(visible)),
                ..Self::default()
            }
        }

        /// Store the root header `add_epoch_root` derives each of `epochs` from.
        async fn with_roots(self, client: &TestClient, epochs: RangeInclusive<u64>) -> Self {
            for epoch in epochs {
                let root = root_block_in_epoch(epoch - 2, EPOCH_HEIGHT);
                let header = client.unverified_header(root).await.unwrap();
                self.roots.lock().unwrap().insert(epoch, header);
            }
            self
        }

        /// Every `load_epoch_root` fails.
        fn failing_root_reads(self) -> Self {
            self.fail_root_reads.store(true, Ordering::Relaxed);
            self
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
            ensure!(
                !self.fail_root_reads.load(Ordering::Relaxed),
                "storage unreadable"
            );
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
    /// serves epoch roots, so only the store, stored roots and LC contract proofs can supply a
    /// table.
    fn coordinator(store: SeededStore) -> EpochMembershipCoordinator<SeqTypes> {
        coordinator_over(committees(store))
    }

    fn coordinator_over(committees: EpochCommittees) -> EpochMembershipCoordinator<SeqTypes> {
        coordinator_with_epoch_height(committees, EPOCH_HEIGHT)
    }

    fn coordinator_with_epoch_height(
        committees: EpochCommittees,
        epoch_height: u64,
    ) -> EpochMembershipCoordinator<SeqTypes> {
        EpochMembershipCoordinator::new(
            committees,
            epoch_height,
            &TestStorage::<SeqTypes>::default(),
        )
    }

    /// Committees over `store` with the first epoch set, as `init_node_state` leaves them.
    fn committees(store: SeededStore) -> EpochCommittees {
        let committees = unseeded_committees(store);
        committees.set_first_epoch(EpochNumber::new(FIRST_EPOCH), [0u8; 32]);
        committees
    }

    fn unseeded_committees(store: SeededStore) -> EpochCommittees {
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
        EpochCommittees::new_stake(peers.clone(), peers, None, fetcher, EPOCH_HEIGHT)
    }

    /// Storage holding the tables of `first + 2 ..= N+1`, none of them visible until a root adds it.
    async fn fresh_coordinator(client: &TestClient) -> EpochMembershipCoordinator<SeqTypes> {
        let store = SeededStore::for_chain(client, (FIRST_EPOCH + 2)..=(N + 1)).await;
        coordinator(store.hidden())
    }

    /// Storage holding the tables of `epochs`, loaded into memory as `init_node_state` does.
    async fn restarted_coordinator(
        client: &TestClient,
        epochs: RangeInclusive<u64>,
    ) -> EpochMembershipCoordinator<SeqTypes> {
        let store = SeededStore::for_chain(client, epochs).await;
        restarted_coordinator_over(store).await
    }

    async fn restarted_coordinator_over(
        store: SeededStore,
    ) -> EpochMembershipCoordinator<SeqTypes> {
        let mut committees = committees(store);
        committees.reload_stake(10).await;
        coordinator_over(committees)
    }

    /// Restart in epoch `N` before its root block: the tables of `N-4 ..= N` are stored, the
    /// root header of `N+1` is stored without its table.
    async fn header_only_row_coordinator(
        client: &TestClient,
    ) -> EpochMembershipCoordinator<SeqTypes> {
        let store = SeededStore::for_chain(client, (N - 4)..=(N + 1))
            .await
            .with_roots(client, (N + 1)..=(N + 1))
            .await
            .hidden();
        store.reveal((N - 4)..=N);
        restarted_coordinator_over(store).await
    }

    fn peer_list(clients: &[TestClient]) -> Vec<(Url, TestClient)> {
        let url: Url = "http://peer.invalid".parse().unwrap();
        clients.iter().map(|c| (url.clone(), c.clone())).collect()
    }

    fn no_peers() -> Vec<(Url, TestClient)> {
        vec![]
    }

    async fn run_catchup(
        coordinator: &EpochMembershipCoordinator<SeqTypes>,
        lc_contract_header: Option<LcContractHeader>,
        state_peers: &[TestClient],
        params: StartupCatchupParams,
        anchor_epoch: Option<u64>,
    ) -> Result<EpochNumber, StartupCatchupError> {
        catch_up_cliquenet_stake_tables(
            coordinator,
            || async { Ok(lc_contract_header.clone()) },
            &peer_list(state_peers),
            &params,
            anchor_epoch.map(EpochNumber::new),
        )
        .await
    }

    /// `catch_up_cliquenet_stake_tables` with a counted LC contract header read.
    async fn run_catchup_counting_reads(
        coordinator: &EpochMembershipCoordinator<SeqTypes>,
        lc_contract_header: Option<LcContractHeader>,
        state_peers: &[TestClient],
        anchor_epoch: Option<u64>,
    ) -> (Result<EpochNumber, StartupCatchupError>, usize) {
        let reads = AtomicUsize::new(0);
        let result = catch_up_cliquenet_stake_tables(
            coordinator,
            || async {
                reads.fetch_add(1, Ordering::Relaxed);
                Ok(lc_contract_header.clone())
            },
            &peer_list(state_peers),
            &PARAMS,
            anchor_epoch.map(EpochNumber::new),
        )
        .await;
        (result, reads.load(Ordering::Relaxed))
    }

    async fn run_ensure(
        coordinator: &EpochMembershipCoordinator<SeqTypes>,
        lc_contract_header: Option<LcContractHeader>,
        state_peers: &[TestClient],
        params: StartupCatchupParams,
        current: u64,
    ) -> Result<(), StartupCatchupError> {
        ensure_stake_tables(
            coordinator,
            || async { Ok(lc_contract_header.clone()) },
            &peer_list(state_peers),
            &params,
            EpochNumber::new(current),
        )
        .await
    }

    /// `ensure_stake_tables` without peers and with an LC contract header read that fails and
    /// counts.
    async fn run_ensure_without_peers_counting_reads(
        coordinator: &EpochMembershipCoordinator<SeqTypes>,
        current: u64,
    ) -> (Result<(), StartupCatchupError>, usize) {
        let reads = AtomicUsize::new(0);
        let result = ensure_stake_tables(
            coordinator,
            || async {
                reads.fetch_add(1, Ordering::Relaxed);
                Err(anyhow!("L1 not ready"))
            },
            &no_peers(),
            &PARAMS,
            EpochNumber::new(current),
        )
        .await;
        (result, reads.load(Ordering::Relaxed))
    }

    fn has_table(coordinator: &EpochMembershipCoordinator<SeqTypes>, epoch: u64) -> bool {
        coordinator
            .membership()
            .snapshot(EpochNumber::new(epoch))
            .is_some()
    }

    fn assert_tables(
        coordinator: &EpochMembershipCoordinator<SeqTypes>,
        epochs: RangeInclusive<u64>,
    ) {
        for epoch in epochs {
            assert!(has_table(coordinator, epoch), "epoch {epoch}");
        }
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

    /// Reveal `epochs` of `store` after `delay`, like a derivation that succeeds late.
    fn reveal_later(store: &SeededStore, epochs: RangeInclusive<u64>, delay: Duration) {
        let store = store.clone();
        tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            store.reveal(epochs);
        });
    }

    /// With `F` in epoch `N`, every required epoch is proven; with `F` in `N-1` before its root,
    /// `N+1` is out of proof reach and is derived.
    #[rstest]
    #[case::lc_contract_at_current(LC_CONTRACT_HEIGHT)]
    #[case::lc_contract_one_epoch_behind(LC_CONTRACT_HEIGHT_BEFORE_ROOT)]
    #[test_log::test(tokio::test(start_paused = true))]
    async fn fresh_join_catches_up_from_lc_contract(#[case] lc_contract_height: u64) {
        let (client, _) = chain().await;
        let header = lc_header_at(&client, lc_contract_height).await;
        let store = SeededStore::for_chain(&client, (FIRST_EPOCH + 2)..=(N + 1))
            .await
            .hidden();
        reveal_later(&store, (N + 1)..=(N + 1), Duration::from_millis(100));
        let coordinator = coordinator(store);

        let current = run_catchup(&coordinator, Some(header), &[client], PARAMS, None)
            .await
            .unwrap();

        assert_eq!(current, EpochNumber::new(N));
        assert_tables(&coordinator, (N - 4)..=(N + 1));
        for epoch in (N - 1)..=(N + 1) {
            let reward = coordinator
                .membership()
                .epoch_block_reward(EpochNumber::new(epoch));
            assert!(reward.is_some(), "no block reward for epoch {epoch}");
        }
        assert_eq!(
            coordinator.membership().highest_known_epoch(),
            Some(EpochNumber::new(N + 1))
        );
    }

    /// `F` lags by one epoch: six proofs at `epoch(F)`, then derivation up to the median.
    #[test_log::test(tokio::test(start_paused = true))]
    async fn prover_lag_installs_proofs_at_the_floor_then_derives_to_the_median() {
        let (client, _) = chain().await;
        client.mock_block_height(9 * EPOCH_HEIGHT).await;
        let header = lc_header_at(&client, LC_CONTRACT_HEIGHT_BEFORE_ROOT).await;
        let store = SeededStore::for_chain(&client, (FIRST_EPOCH + 2)..=(N + 2))
            .await
            .hidden();
        reveal_later(&store, (N + 1)..=(N + 2), Duration::from_millis(100));
        let coordinator = coordinator(store);

        let current = run_catchup(&coordinator, Some(header), &[client], PARAMS, None)
            .await
            .unwrap();

        assert_eq!(current, EpochNumber::new(N + 1));
        assert_tables(&coordinator, (N - 3)..=(N + 2));
    }

    #[test_log::test(tokio::test)]
    async fn lc_contract_catchup_rotates_past_a_wrong_chain() {
        let (client, header) = chain().await;
        let (other, _) = chain().await;
        let coordinator = fresh_coordinator(&client).await;

        let current = run_catchup(&coordinator, Some(header), &[other, client], PARAMS, None)
            .await
            .unwrap();

        assert_eq!(current, EpochNumber::new(N));
        assert!(has_table(&coordinator, N + 1));
    }

    /// An epoch root the peer cannot serve at first must be retried, not read as the tip.
    #[test_log::test(tokio::test(start_paused = true))]
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

        let current = run_catchup(&coordinator, Some(header), &[client], PARAMS, None)
            .await
            .unwrap();

        assert_eq!(current, EpochNumber::new(N));
        assert!(has_table(&coordinator, N + 1));
    }

    #[test_log::test(tokio::test(start_paused = true))]
    async fn lc_contract_header_unavailable_aborts_at_the_deadline() {
        let (client, header) = chain().await;
        let coordinator = fresh_coordinator(&client).await;
        client.forget_leaf(LC_CONTRACT_HEIGHT as usize).await;

        let params = StartupCatchupParams {
            total_timeout: Duration::from_millis(300),
            ..PARAMS
        };
        let err = run_catchup(&coordinator, Some(header), &[client], params, None)
            .await
            .unwrap_err();

        let StartupCatchupError::DeadlineExceeded {
            stage: Stage::LcContractProof { .. },
            source,
            ..
        } = err
        else {
            panic!("expected the LC contract proof deadline, got {err:?}");
        };
        assert_matches!(
            source.downcast_ref::<LcContractProofError>(),
            Some(LcContractProofError::HeaderFetch(_))
        );
    }

    /// A failed LC contract header read is retried and the proofs then install.
    #[test_log::test(tokio::test(start_paused = true))]
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
            None,
        )
        .await
        .unwrap();

        assert_eq!(reads.load(Ordering::Relaxed), 2);
        assert_eq!(current, EpochNumber::new(N));
        assert!(has_table(&coordinator, N + 1));
    }

    /// The LC contract header read shares the deadline with the stages after it.
    #[test_log::test(tokio::test(start_paused = true))]
    async fn lc_contract_read_consumes_the_deadline() {
        let (client, _) = chain().await;
        let coordinator = coordinator(SeededStore::default());

        let err = ensure_stake_tables(
            &coordinator,
            std::future::pending::<anyhow::Result<Option<LcContractHeader>>>,
            &peer_list(&[client]),
            &SHORT_DEADLINE_PARAMS,
            EpochNumber::new(N),
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

    /// Held epochs skip the proofs; the LC contract header is read once.
    #[test_log::test(tokio::test)]
    async fn restart_boots_from_storage() {
        let (client, header) = chain().await;
        let coordinator = restarted_coordinator(&client, (N - 4)..=(N + 1)).await;

        let (current, reads) =
            run_catchup_counting_reads(&coordinator, Some(header), &[client], Some(N)).await;

        assert_eq!(current.unwrap(), EpochNumber::new(N));
        assert_eq!(reads, 1);
    }

    /// The validator half of the read-count pair: held tables, one read.
    #[test_log::test(tokio::test)]
    async fn held_tables_make_the_validator_read_the_lc_contract_once() {
        let (client, header) = chain().await;
        let coordinator = restarted_coordinator(&client, (N - 4)..=(N + 1)).await;

        let (current, reads) =
            run_catchup_counting_reads(&coordinator, Some(header), &[client], None).await;

        assert_eq!(current.unwrap(), EpochNumber::new(N));
        assert_eq!(reads, 1);
    }

    /// The follower half: held tables, no read at all.
    #[test_log::test(tokio::test)]
    async fn follower_held_range_makes_no_lc_contract_read() {
        let (client, _) = chain().await;
        let coordinator = restarted_coordinator(&client, (N - 4)..=(N + 1)).await;

        let (result, reads) = run_ensure_without_peers_counting_reads(&coordinator, N).await;

        result.unwrap();
        assert_eq!(reads, 0);
    }

    /// No peer answers and the header-only row of `N+1` is re-derived from its stored root.
    #[test_log::test(tokio::test)]
    async fn floor_boot_warns() {
        let (client, header) = chain().await;
        let coordinator = header_only_row_coordinator(&client).await;
        assert!(!has_table(&coordinator, N + 1));
        client.fail_next_block_heights(usize::MAX).await;

        let (current, reads) = run_catchup_counting_reads(
            &coordinator,
            Some(header),
            std::slice::from_ref(&client),
            Some(N),
        )
        .await;

        assert_eq!(current.unwrap(), EpochNumber::new(N));
        assert_eq!(reads, 1);
        assert_eq!(client.block_height_calls().await, 1);
        assert_tables(&coordinator, (N - 4)..=(N + 1));
    }

    /// A fleet restart boots within one step timeout when every peer fails its height read.
    #[test_log::test(tokio::test(start_paused = true))]
    async fn fleet_restart_boots_within_one_step_timeout() {
        let (client, header) = chain().await;
        let coordinator = header_only_row_coordinator(&client).await;
        client.fail_next_block_heights(usize::MAX).await;
        let start = Instant::now();

        let current = run_catchup(
            &coordinator,
            Some(header),
            std::slice::from_ref(&client),
            PARAMS,
            Some(N),
        )
        .await
        .unwrap();

        assert_eq!(current, EpochNumber::new(N));
        assert!(start.elapsed() <= PARAMS.step_timeout);
    }

    /// Restart in epoch `N` before its root block: no peer is asked for anything.
    #[test_log::test(tokio::test)]
    async fn restart_before_the_anchor_root_boots_without_peers() {
        let (client, _) = chain().await;
        let coordinator = header_only_row_coordinator(&client).await;
        client.fail_next_block_heights(usize::MAX).await;

        let current = run_catchup(
            &coordinator,
            None,
            std::slice::from_ref(&client),
            PARAMS,
            Some(N),
        )
        .await
        .unwrap();

        assert_eq!(current, EpochNumber::new(N));
        assert_eq!(client.block_height_calls().await, 1);
        assert!(has_table(&coordinator, N + 1));
    }

    /// Storage past the LC contract header: nothing is fetched, the newest row is re-derived.
    #[test_log::test(tokio::test)]
    async fn storage_above_the_lc_contract_refetches_nothing() {
        let (client, _) = chain().await;
        let header = lc_header_at(&client, 5 * EPOCH_HEIGHT).await;
        let coordinator = header_only_row_coordinator(&client).await;

        let current = catch_up_cliquenet_stake_tables(
            &coordinator,
            || async { Ok(Some(header.clone())) },
            &no_peers(),
            &PARAMS,
            Some(EpochNumber::new(N)),
        )
        .await
        .unwrap();

        assert_eq!(current, EpochNumber::new(N));
        assert!(has_table(&coordinator, N + 1));
    }

    /// The decided epoch, not the one after it, is `N` when the anchor lies in the last blocks of
    /// an epoch and the root of `N+2` is stored.
    #[test_log::test(tokio::test)]
    async fn anchor_in_the_last_blocks_of_an_epoch_boots_at_the_decided_epoch() {
        let (client, _) = chain().await;
        client.mock_block_height(LC_CONTRACT_HEIGHT + 1).await;
        let store = SeededStore::for_chain(&client, (N - 4)..=(N + 2))
            .await
            .with_roots(&client, (N + 2)..=(N + 2))
            .await
            .hidden();
        store.reveal((N - 4)..=(N + 1));
        let coordinator = restarted_coordinator_over(store).await;
        let anchor = block_epoch(N * EPOCH_HEIGHT, EPOCH_HEIGHT);

        let current = run_catchup(&coordinator, None, &[client], PARAMS, Some(anchor.u64()))
            .await
            .unwrap();

        assert_eq!(current, EpochNumber::new(N));
        assert!(!has_table(&coordinator, N + 2));
    }

    /// The anchor lifts `N` over a stale median, even when storage holds tables above it.
    #[test_log::test(tokio::test)]
    async fn anchor_selects_over_stale_median() {
        let (client, _) = chain().await;
        client.mock_block_height(3 * EPOCH_HEIGHT).await;
        let coordinator = restarted_coordinator(&client, (N - 4)..=(N + 3)).await;

        let current = run_catchup(&coordinator, None, &[client], PARAMS, Some(N))
            .await
            .unwrap();

        assert_eq!(current, EpochNumber::new(N));
    }

    /// The median above the floor raises `N`.
    #[test_log::test(tokio::test)]
    async fn median_above_the_floor_raises_the_current_epoch() {
        let (client, _) = chain().await;
        client.mock_block_height((N + 1) * EPOCH_HEIGHT).await;
        let coordinator = restarted_coordinator(&client, (N - 4)..=(N + 3)).await;

        let current = run_catchup(&coordinator, None, &[client], PARAMS, Some(N))
            .await
            .unwrap();

        assert_eq!(current, EpochNumber::new(N + 1));
    }

    /// The median below the LC contract epoch does not lower `N`.
    #[test_log::test(tokio::test)]
    async fn median_below_the_floor_does_not_lower_it() {
        let (client, header) = chain().await;
        client.mock_block_height(3 * EPOCH_HEIGHT).await;
        let coordinator = restarted_coordinator(&client, (N - 4)..=(N + 1)).await;

        let current = run_catchup(&coordinator, Some(header), &[client], PARAMS, None)
            .await
            .unwrap();

        assert_eq!(current, EpochNumber::new(N));
    }

    /// Two peers stay silent and one answers: the one sets the median.
    #[test_log::test(tokio::test(start_paused = true))]
    async fn lone_answering_peer_sets_the_median() {
        let (client, _) = chain().await;
        client.mock_block_height((N + 1) * EPOCH_HEIGHT).await;
        let silent = [(); 2].map(|_| TestClient::with_epoch_height(EPOCH_HEIGHT));
        for peer in &silent {
            peer.fail_next_block_heights(usize::MAX).await;
        }
        let coordinator = restarted_coordinator(&client, (N - 4)..=(N + 3)).await;
        let [first, second] = silent;

        let current = run_catchup(
            &coordinator,
            None,
            &[first, client, second],
            PARAMS,
            Some(N),
        )
        .await
        .unwrap();

        assert_eq!(current, EpochNumber::new(N + 1));
    }

    /// Half the peers at height 0: the median comes from the others, above a floor of `N-1`.
    #[test_log::test(tokio::test)]
    async fn half_zero_heights_use_the_nonzero_median() {
        let (client, _) = chain().await;
        let (empty, _) = chain().await;
        empty.mock_block_height(0).await;
        let coordinator = restarted_coordinator(&client, (N - 5)..=(N + 1)).await;

        let current = run_catchup(&coordinator, None, &[empty, client], PARAMS, Some(N - 1))
            .await
            .unwrap();

        assert_eq!(current, EpochNumber::new(N));
    }

    /// A peer reporting a height above the served chain sets `N` above it; its epoch cannot be
    /// derived and the node exits at the deadline.
    #[test_log::test(tokio::test(start_paused = true))]
    async fn lying_high_median_exits_at_the_deadline() {
        let (client, header) = chain().await;
        client.mock_block_height(20 * EPOCH_HEIGHT).await;
        let coordinator = restarted_coordinator(&client, (N - 4)..=(N + 1)).await;

        let err = run_catchup(
            &coordinator,
            Some(header),
            &[client],
            SHORT_DEADLINE_PARAMS,
            None,
        )
        .await
        .unwrap_err();

        assert_matches!(
            err,
            StartupCatchupError::DeadlineExceeded {
                stage: Stage::Derivation { .. },
                ..
            }
        );
    }

    /// Nothing held beyond the seeded epochs: the LC contract inside the first epoch (it ends at
    /// block 10) starts at genesis, one block further the chain is past the seeded epochs and
    /// the height read is retried until the deadline.
    #[rstest]
    #[case::in_first_epoch(EPOCH_HEIGHT, true)]
    #[case::in_second_epoch(EPOCH_HEIGHT + 1, false)]
    #[test_log::test(tokio::test(start_paused = true))]
    async fn no_peer_height_and_nothing_derived(
        #[case] lc_height: u64,
        #[case] starts_at_genesis: bool,
    ) {
        let (client, _) = chain().await;
        let coordinator = coordinator(SeededStore::default());
        client.fail_next_block_heights(usize::MAX).await;
        let header = LcContractHeader {
            height: lc_height,
            block_comm_root: CircuitField::from(0u64),
        };

        let result = run_catchup(&coordinator, Some(header), &[client], PARAMS, None).await;

        if starts_at_genesis {
            assert_eq!(result.unwrap(), EpochNumber::new(FIRST_EPOCH));
        } else {
            assert_matches!(
                result,
                Err(StartupCatchupError::DeadlineExceeded {
                    stage: Stage::PeerHeights,
                    ..
                })
            );
        }
    }

    /// Storage behind the LC contract header and silent peers: the heights are retried.
    #[test_log::test(tokio::test(start_paused = true))]
    async fn down_for_days_with_silent_peers_exits() {
        let (client, header) = chain().await;
        let coordinator = restarted_coordinator(&client, 3..=4).await;
        client.fail_next_block_heights(usize::MAX).await;

        let result = run_catchup(&coordinator, Some(header), &[client], PARAMS, Some(4)).await;

        assert_matches!(
            result,
            Err(StartupCatchupError::DeadlineExceeded {
                stage: Stage::PeerHeights,
                ..
            })
        );
    }

    /// Peers at height 0 give no evidence: with nothing held the heights are retried.
    #[test_log::test(tokio::test(start_paused = true))]
    async fn all_zero_heights_with_nothing_held_exit_at_the_deadline() {
        let (client, header) = chain().await;
        let coordinator = coordinator(SeededStore::default());
        client.mock_block_height(0).await;

        let result = run_catchup(&coordinator, Some(header), &[client], PARAMS, None).await;

        assert_matches!(
            result,
            Err(StartupCatchupError::DeadlineExceeded {
                stage: Stage::PeerHeights,
                ..
            })
        );
    }

    #[test_log::test(tokio::test)]
    async fn no_peers_configured_and_nothing_derived() {
        let coordinator = coordinator(SeededStore::default());
        let header = LcContractHeader {
            height: EPOCH_HEIGHT + 1,
            block_comm_root: CircuitField::from(0u64),
        };

        let result = run_catchup(&coordinator, Some(header), &[], PARAMS, None).await;

        assert_matches!(
            result,
            Err(StartupCatchupError::StatePeersUnavailable { epoch })
                if epoch == EpochNumber::new(FIRST_EPOCH + 2)
        );
    }

    /// Peers that fail the first two passes and answer the third still give `N`; two backoffs run, 1 s then 2 s.
    #[test_log::test(tokio::test(start_paused = true))]
    async fn fresh_join_retries_the_peer_height_read() {
        let (client, header) = chain().await;
        let coordinator = fresh_coordinator(&client).await;
        client.fail_next_block_heights(2).await;

        let current = run_catchup(
            &coordinator,
            Some(header),
            std::slice::from_ref(&client),
            PARAMS,
            None,
        )
        .await
        .unwrap();

        assert_eq!(client.block_height_calls().await, 3);
        assert_eq!(current, EpochNumber::new(N));
        assert_tables(&coordinator, (N - 4)..=(N + 1));
    }

    /// Inside the seeded epochs the node starts at `first` after one height pass.
    #[rstest]
    #[case::no_lc_contract_header(None)]
    #[case::inside_seeded(Some(EPOCH_HEIGHT))]
    #[test_log::test(tokio::test(start_paused = true))]
    async fn genesis_boot_with_silent_peers_starts_at_first(#[case] lc_height: Option<u64>) {
        let (client, _) = chain().await;
        let coordinator = coordinator(SeededStore::default());
        client.fail_next_block_heights(usize::MAX).await;
        let header = lc_height.map(|height| LcContractHeader {
            height,
            block_comm_root: CircuitField::from(0u64),
        });

        let current = run_catchup(
            &coordinator,
            header,
            std::slice::from_ref(&client),
            PARAMS,
            None,
        )
        .await
        .unwrap();

        assert_eq!(current, EpochNumber::new(FIRST_EPOCH));
        assert_eq!(client.block_height_calls().await, 1);
    }

    #[test_log::test(tokio::test)]
    async fn all_zero_peer_heights_start_at_genesis() {
        let (client, _) = chain().await;
        let coordinator = coordinator(SeededStore::default());
        client.mock_block_height(0).await;

        let current = run_catchup(&coordinator, None, &[client], PARAMS, None)
            .await
            .unwrap();

        assert_eq!(current, EpochNumber::new(FIRST_EPOCH));
    }

    /// With the storage complete and no peer height, the LC contract header is still read.
    #[test_log::test(tokio::test(start_paused = true))]
    async fn no_peer_height_and_failing_lc_read_aborts_at_the_deadline() {
        let (client, _) = chain().await;
        let coordinator = restarted_coordinator(&client, (N - 4)..=(N + 1)).await;
        client.fail_next_block_heights(usize::MAX).await;

        let err = catch_up_cliquenet_stake_tables(
            &coordinator,
            || async { Err(anyhow!("L1 not ready")) },
            &peer_list(&[client]),
            &SHORT_DEADLINE_PARAMS,
            Some(EpochNumber::new(N)),
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

    /// A genesis boot has nothing to catch up, so it needs no state peers.
    #[test_log::test(tokio::test)]
    async fn genesis_boot_without_state_peers() {
        let coordinator = coordinator(SeededStore::default());

        let current = catch_up_cliquenet_stake_tables(
            &coordinator,
            || async { Ok(None) },
            &no_peers(),
            &PARAMS,
            None,
        )
        .await
        .unwrap();

        assert_eq!(current, EpochNumber::new(FIRST_EPOCH));
    }

    #[test_log::test(tokio::test)]
    async fn unseeded_membership_fails_with_first_epoch_unset() {
        let coordinator = coordinator_over(unseeded_committees(SeededStore::default()));

        let result = run_catchup(&coordinator, None, &[], PARAMS, None).await;

        assert_matches!(result, Err(StartupCatchupError::FirstEpochUnset));
    }

    #[test_log::test(tokio::test)]
    async fn epoch_height_zero_fails_at_once() {
        let committees = committees(SeededStore::default());
        let coordinator = coordinator_with_epoch_height(committees, 0);

        let result = run_catchup(&coordinator, None, &[], PARAMS, None).await;

        assert_matches!(result, Err(StartupCatchupError::EpochHeightZero));
    }

    #[test_log::test(tokio::test)]
    async fn no_peers_configured_with_a_missing_table_fails_before_the_lc_read() {
        let coordinator = coordinator(SeededStore::default());

        let (result, reads) = run_ensure_without_peers_counting_reads(&coordinator, N).await;

        assert_matches!(
            result,
            Err(StartupCatchupError::StatePeersUnavailable { epoch })
                if epoch == EpochNumber::new(N - 4)
        );
        assert_eq!(reads, 0);
    }

    #[test_log::test(tokio::test)]
    async fn table_in_storage_but_not_memory_counts_as_held() {
        let (client, _) = chain().await;
        let store = SeededStore::for_chain(&client, (N - 4)..=(N + 1)).await;
        let coordinator = coordinator(store);
        assert!(!has_table(&coordinator, N));

        let (result, _) = run_ensure_without_peers_counting_reads(&coordinator, N).await;

        result.unwrap();
        assert!(has_table(&coordinator, N));
    }

    /// A held epoch without its root, above the required ones, does not select the current epoch.
    #[test_log::test(tokio::test)]
    async fn rootless_table_counts_as_held() {
        let (client, _) = chain().await;
        client.fail_next_block_heights(usize::MAX).await;
        let coordinator = restarted_coordinator(&client, (N - 4)..=(N + 2)).await;
        assert!(has_table(&coordinator, N + 2));

        let current = run_catchup(&coordinator, None, &[client], PARAMS, Some(N))
            .await
            .unwrap();

        assert_eq!(current, EpochNumber::new(N));
    }

    /// A stored root header without its table is derived before any peer is asked: the call has
    /// no peers, which a missing table would reject.
    #[test_log::test(tokio::test)]
    async fn stored_root_is_derived_before_any_peer_is_asked() {
        let (client, _) = chain().await;
        let store = SeededStore::for_chain(&client, (N - 4)..=(N + 1))
            .await
            .with_roots(&client, (N - 4)..=(N + 1))
            .await
            .hidden();
        let coordinator = coordinator(store);

        let (result, _) = run_ensure_without_peers_counting_reads(&coordinator, N).await;

        result.unwrap();
        assert_tables(&coordinator, (N - 4)..=(N + 1));
    }

    /// A stored root of an epoch no required epoch needs is not read.
    #[test_log::test(tokio::test)]
    async fn stored_root_outside_the_required_range_is_not_read() {
        let (client, _) = chain().await;
        let store = SeededStore::for_chain(&client, (N - 4)..=(N + 1))
            .await
            .with_roots(&client, (N + 4)..=(N + 4))
            .await;
        let coordinator = restarted_coordinator_over(store).await;

        let (result, _) = run_ensure_without_peers_counting_reads(&coordinator, N).await;

        result.unwrap();
        assert!(!has_table(&coordinator, N + 4));
    }

    /// A stored root whose derivation keeps failing ends startup at the deadline naming the epoch.
    #[test_log::test(tokio::test(start_paused = true))]
    async fn failed_stored_root_derivation_exits_naming_the_epoch() {
        let (client, _) = chain().await;
        // The table of `N-4` is not in storage, so deriving it needs L1 events.
        let store = SeededStore::for_chain(&client, (N - 3)..=(N + 1)).await;
        let store = store.with_roots(&client, (N - 4)..=(N - 4)).await;
        let coordinator = coordinator(store);

        let err = run_ensure(&coordinator, None, &[client], SHORT_DEADLINE_PARAMS, N)
            .await
            .unwrap_err();

        assert_matches!(
            err,
            StartupCatchupError::DeadlineExceeded {
                stage: Stage::StoredRootDerivation { epoch },
                ..
            } if epoch == EpochNumber::new(N - 4)
        );
    }

    /// A stored root derivation that fails once is retried.
    #[test_log::test(tokio::test(start_paused = true))]
    async fn stored_root_derivation_is_retried() {
        let (client, _) = chain().await;
        let store = SeededStore::for_chain(&client, (N - 3)..=(N + 1)).await;
        let store = store.with_roots(&client, (N - 4)..=(N - 4)).await;
        let late_table = SeededStore::for_chain(&client, (N - 4)..=(N - 4)).await;
        let late = store.clone();
        let coordinator = coordinator(store);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let table = late_table.tables.lock().unwrap().remove(&(N - 4)).unwrap();
            late.tables.lock().unwrap().insert(N - 4, table);
        });

        run_ensure(&coordinator, None, &[client], PARAMS, N)
            .await
            .unwrap();

        assert!(has_table(&coordinator, N - 4));
    }

    /// A storage read failure of a stored root fails at once.
    #[test_log::test(tokio::test)]
    async fn unreadable_stored_root_fails_at_once() {
        let (client, _) = chain().await;
        let store = SeededStore::for_chain(&client, (N - 4)..=(N + 1))
            .await
            .failing_root_reads()
            .hidden();
        let coordinator = coordinator(store);

        let result = run_ensure(&coordinator, None, &[client], PARAMS, N).await;

        assert_matches!(
            result,
            Err(StartupCatchupError::StorageRead { epoch, .. }) if epoch == EpochNumber::new(N - 4)
        );
    }

    /// A failed derivation step is retried until the epoch is held.
    #[test_log::test(tokio::test(start_paused = true))]
    async fn derivation_retries_each_step_until_the_epoch_is_held() {
        let (client, _) = chain().await;
        let store = SeededStore::for_chain(&client, (N - 4)..=(N + 1))
            .await
            .hidden();
        store.reveal((N - 4)..=(N - 3));
        let coordinator = coordinator(store.clone());
        hold(&coordinator, (N - 4)..=(N - 3)).await;
        reveal_later(&store, (N - 2)..=(N + 1), Duration::from_millis(100));

        run_ensure(&coordinator, None, &[client], PARAMS, N)
            .await
            .unwrap();

        assert_tables(&coordinator, (N - 4)..=(N + 1));
    }

    /// A derivation step that keeps failing ends startup at the deadline.
    #[test_log::test(tokio::test(start_paused = true))]
    async fn derivation_fails_at_the_deadline() {
        let (client, _) = chain().await;
        let store = SeededStore::for_chain(&client, (N - 4)..=(N + 1))
            .await
            .hidden();
        store.reveal((N - 4)..=(N - 3));
        let coordinator = coordinator(store);
        hold(&coordinator, (N - 4)..=(N - 3)).await;

        let err = run_ensure(&coordinator, None, &[client], SHORT_DEADLINE_PARAMS, N)
            .await
            .unwrap_err();

        assert_matches!(
            err,
            StartupCatchupError::DeadlineExceeded {
                stage: Stage::Derivation { epoch },
                ..
            } if epoch == EpochNumber::new(N - 2)
        );
    }

    /// An invalid proof, or a peer serving no proofs, fails at the deadline naming the epoch; no
    /// table is derived from the peers' QC leaves instead.
    #[rstest]
    #[case::invalid_proof_of_one_epoch(false, N - 2)]
    #[case::no_light_client_module(true, N - 4)]
    #[test_log::test(tokio::test(start_paused = true))]
    async fn invalid_proof_for_one_epoch_fails_at_the_deadline(
        #[case] no_proofs: bool,
        #[case] failing_epoch: u64,
    ) {
        let (client, header) = chain().await;
        let coordinator = fresh_coordinator(&client).await;
        if no_proofs {
            client.fail_header_proofs().await;
        } else {
            client
                .return_invalid_proof(root_block_in_epoch(failing_epoch - 2, EPOCH_HEIGHT) as usize)
                .await;
        }

        let err = run_catchup(
            &coordinator,
            Some(header),
            &[client],
            SHORT_DEADLINE_PARAMS,
            None,
        )
        .await
        .unwrap_err();

        assert_matches!(
            err,
            StartupCatchupError::DeadlineExceeded {
                stage: Stage::LcContractProof { epoch },
                ..
            } if epoch == EpochNumber::new(failing_epoch)
        );
        assert!(!has_table(&coordinator, failing_epoch));
    }

    /// A restart after an interrupted catchup holds a lone table at the start of the required
    /// epochs, and the catchup completes them.
    #[test_log::test(tokio::test)]
    async fn partial_lc_contract_catchup_is_completed() {
        let (client, header) = chain().await;
        let coordinator = fresh_coordinator(&client).await;
        let first_root = client
            .unverified_header(root_block_in_epoch(N - 6, EPOCH_HEIGHT))
            .await;
        coordinator
            .add_epoch_root(first_root.unwrap())
            .await
            .unwrap();
        assert!(has_table(&coordinator, N - 4));
        assert!(!has_table(&coordinator, N - 3));

        let current = run_catchup(
            &coordinator,
            Some(header),
            std::slice::from_ref(&client),
            PARAMS,
            None,
        )
        .await
        .unwrap();

        assert_eq!(current, EpochNumber::new(N));
        assert_tables(&coordinator, (N - 4)..=(N + 1));
    }

    /// An epoch far below `F` installs the proofs at that epoch, not at `F`; the membership has no
    /// peer to derive from, so only proofs can supply the tables.
    #[test_log::test(tokio::test)]
    async fn follower_goal_far_below_the_lc_contract_uses_proofs_at_the_goal() {
        let client = TestClient::with_epoch_height(EPOCH_HEIGHT);
        let header = lc_header_at(&client, 20 * EPOCH_HEIGHT + 7).await;
        let goal = N + 3;
        let store = SeededStore::for_chain(&client, (goal - 4)..=(goal + 1)).await;
        let coordinator = coordinator(store.hidden());

        run_ensure(&coordinator, Some(header), &[client], PARAMS, goal)
            .await
            .unwrap();

        assert_tables(&coordinator, (goal - 4)..=(goal + 1));
        assert!(!has_table(&coordinator, goal - 5));
        for epoch in (FIRST_EPOCH + 2)..(goal - 4) {
            assert!(!has_table(&coordinator, epoch), "epoch {epoch}");
        }
    }

    /// A follower target above `epoch(F)`: the proofs of `epoch(F)` come first, then the walk
    /// derives the epochs above from the proven pair. `F` is read once.
    #[test_log::test(tokio::test(start_paused = true))]
    async fn follower_target_above_the_lc_contract_installs_proofs_at_the_lc_contract_epoch() {
        let (client, header) = chain().await;
        let target = N + 7;
        let store = SeededStore::for_chain(&client, (N - 4)..=(target + 1))
            .await
            .hidden();
        reveal_later(&store, (N + 2)..=(target + 1), Duration::from_millis(100));
        let coordinator = coordinator(store);
        let reads = AtomicUsize::new(0);

        ensure_stake_tables(
            &coordinator,
            || async {
                reads.fetch_add(1, Ordering::Relaxed);
                Ok(Some(header.clone()))
            },
            &peer_list(&[client]),
            &PARAMS,
            EpochNumber::new(target),
        )
        .await
        .unwrap();

        assert_eq!(reads.load(Ordering::Relaxed), 1);
        assert_tables(&coordinator, (N - 4)..=(N + 1));
        assert_tables(&coordinator, (target - 4)..=(target + 1));
    }

    /// Held `3..=5` and a lone table at `8`: epochs `6` and `7` are in the gap.
    #[rstest]
    #[case::lc_contract_header(true)]
    #[case::no_lc_contract_header(false)]
    #[test_log::test(tokio::test(start_paused = true))]
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
            reveal_later(&store, 6..=7, Duration::from_millis(100));
        }

        run_ensure(&coordinator, header, &[client], PARAMS, 7)
            .await
            .unwrap();

        assert_tables(&coordinator, 6..=7);
    }
}
