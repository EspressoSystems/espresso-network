//! Startup stake-table catchup for new-protocol nodes.
//!
//! [`catch_up_over_cliquenet`] is the validator boot: it reads the LC contract's voting stake
//! table, then either starts from storage or fills the epoch window over the node's cliquenet
//! instance.
//!
//! [`bootstrap_epoch_window`] is the follower startup. Cliquenet only connects to validators in the
//! current epoch's stake table window (`N-1`, `N`, `N+1`), and a follower holds no stake, so it
//! drives the existing catchup machinery synchronously: it walks forward one epoch at a time from
//! the highest already-known epoch (loaded from persistence by `reload_stake`) until peers can no
//! longer serve the next epoch root leaf, which is the point at which the live network currently
//! is.
#![expect(
    dead_code,
    unused_variables,
    reason = "skeleton: bodies and wiring follow"
)]

use std::{
    collections::{BTreeMap, BTreeSet},
    ops::RangeInclusive,
    time::Duration,
};

use anyhow::{Context, ensure};
use async_trait::async_trait;
use espresso_types::{
    AuthenticatedValidatorMap, Header, LcContractStakeTableMatchError, LcContractVotingStakeTable,
    NodeState, PubKey, RegisteredValidatorMap, SeqTypes,
    v0_3::{EventKey, StakeTableEvent},
};
use hotshot_new_protocol::utils::CertifiedLeafChain;
use hotshot_types::{
    PeerConfig, PeerConnectInfo,
    data::{EpochNumber, Leaf2},
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    simple_certificate::{Certificate1, Certificate2},
    stake_table::HSStakeTable,
    traits::election::Membership,
};
use indexmap::IndexMap;
use light_client::consensus::header::HeaderProof;
use thiserror::Error;
use vbs::version::Version;
use vec1::Vec1;

/// All startup steps together; the node exits at expiry.
pub(crate) const STARTUP_CATCHUP_DEADLINE: Duration = Duration::from_secs(15 * 60);

/// Bootstrap peers asked per fill request.
pub(crate) const STARTUP_FAN_OUT: usize = 5;

/// Validator boot: LC contract read, then `StartupPath::choose`: from storage
/// (`apply_epoch(anchor_epoch)` by the caller, returns `anchor_epoch`) or the fill (dial `P`, fill
/// the window over cliquenet, returns `C`).
///
/// - L1 values (`ST(now)`, candidate `ST(E')`) are candidates: they dial and verify the anchor
///   leaf; exact tables come only from `add_epoch_root` on proven root headers.
/// - Prover lag: the verified walk from `E'` to `C` over cliquenet, one verified leaf and one
///   header proof per epoch; no HTTP fallback.
/// - Own key: in `P`, fill; registered but not in `P`, re-run the L1 phase per L1 finality tick
///   (INFO per wait); not registered, exit `OwnKeyNotRegistered`.
/// - Speed: independent requests (header proofs, DRB leaves) are in flight together; `startup
///   catchup done` logs `duration_ms` per stage.
/// - Exits instead of recovering: `NoLcContractCommitmentMatch`; `OwnKeyNotRegistered`;
///   `InstalledCommitmentMismatch` when committee `E'` installed from `root(E'-2)` differs from the
///   LC contract candidate (both commitments are logged at ERROR first); `DeadlineExceeded` naming
///   the step, including no reachable window peer after a complete committee turnover during the
///   lag.
/// - Network: `net` borrows the node's instance before the coordinator owns it. The fill sends with
///   `unicast` and reads with `receive()`, never through `client_api()`; it never calls
///   `apply_epoch` and never sets `shared.epoch`, so the caller's `apply_epoch(C)` installs the
///   window and drops candidate peers. Messages the fill consumes never reach the coordinator.
///   Inbound requests are ignored.
/// - Recent storage skips the fill; gap: prover lag plus long downtime with `anchor_epoch >= E' -
///   1` but `C > anchor_epoch + 1` starts at `anchor_epoch` and relies on post-start catchup, as
///   today.
/// - Membership reads use the non-spawning `membership().snapshot(epoch)`; the fill never calls
///   `stake_table_for_epoch` or `membership_for_epoch`.
/// - For `E' <= first + 1` the seeded genesis table is the candidate; `find` is skipped.
pub(crate) async fn catch_up_over_cliquenet(
    l1: &impl StartupL1Reader,
    net: &mut impl StartupCliquenet,
    membership: &EpochMembershipCoordinator<SeqTypes>,
    params: &StartupCatchupParams,
) -> Result<EpochNumber, StartupCatchupError> {
    todo!()
}

/// Bounds and identity of one startup catchup.
#[derive(Clone, Debug)]
pub struct StartupCatchupParams {
    /// The node's own BLS key, checked against the bootstrap peers and the L1 registry.
    pub own_key: PubKey,
    /// Capacity of the stake table commitment, as in the LC contract genesis.
    pub stake_table_capacity: usize,
    /// Versions the root deriving `E'` may carry: upgrade base and target.
    pub candidate_versions: Vec<Version>,
    /// One L1 read, request, or observation wait.
    pub step_timeout: Duration,
    /// Epoch of the anchor leaf the node loaded from storage (`initializer.anchor_leaf()`);
    /// genesis on a fresh node.
    pub anchor_epoch: EpochNumber,
    /// All steps together, [`STARTUP_CATCHUP_DEADLINE`] in production; exit at expiry.
    pub deadline: Duration,
}

/// The startup catchup's L1 reads. `Send + Sync` so `&impl StartupL1Reader` futures are `Send`.
#[async_trait]
pub(crate) trait StartupL1Reader: Send + Sync {
    /// The L1 finalized block.
    async fn l1_finalized_block(&self) -> anyhow::Result<u64>;
    /// `finalizedState` and `votingStakeTableState` at `l1_block`.
    async fn lc_contract_voting_stake_table(
        &self,
        l1_block: u64,
    ) -> anyhow::Result<LcContractVotingStakeTable>;
    /// Every stored stake table event up to `to_l1_block`.
    async fn stake_table_events(
        &self,
        to_l1_block: u64,
    ) -> anyhow::Result<Vec<(EventKey, StakeTableEvent)>>;
    /// The next L1 finalized block above `after`; the wait while the own key is registered but not
    /// in `P`.
    async fn next_l1_finalized_block(&self, after: u64) -> anyhow::Result<u64>;
}

#[async_trait]
impl StartupL1Reader for NodeState {
    async fn l1_finalized_block(&self) -> anyhow::Result<u64> {
        todo!()
    }

    async fn lc_contract_voting_stake_table(
        &self,
        l1_block: u64,
    ) -> anyhow::Result<LcContractVotingStakeTable> {
        todo!()
    }

    async fn stake_table_events(
        &self,
        to_l1_block: u64,
    ) -> anyhow::Result<Vec<(EventKey, StakeTableEvent)>> {
        todo!()
    }

    async fn next_l1_finalized_block(&self, after: u64) -> anyhow::Result<u64> {
        todo!()
    }
}

/// The node's cliquenet during the fill, before the coordinator owns it. `Send` so the
/// `&mut self` futures `async_trait` boxes as `Send` hold `&mut Self` across awaits.
#[async_trait]
pub(crate) trait StartupCliquenet: Send {
    /// Add `peers` as parties and dial them.
    fn dial(&mut self, peers: &BootstrapPeers) -> anyhow::Result<()>;

    /// Inbound connections accepted so far; zero with outbound peers up triggers the firewall WARN.
    fn accepted_inbound_connections(&self) -> usize;

    /// Each request fans out to `STARTUP_FAN_OUT` bootstrap peers, first verified answer wins, all
    /// in flight at once, within `wait` for the whole batch; results in request order, an
    /// unanswered entry is `Err`. `VerifiedLeaf` and `DrbLeaf` fetch `Leaf(height)` then
    /// `Cert2(cert2 height)`; `RootHeaderProof` fetches `HeaderProof`.
    ///
    /// Peers with no answer, an error answer, or an answer failing verification log at DEBUG, never
    /// WARN; per-stage answered and unanswered counts go to the `startup catchup done` INFO line.
    ///
    /// Correlation: one `receive()` loop routes each response by `RequestHash`, the blake3 of
    /// `Request::to_bytes()` that every response frame carries. Requests with identical bytes share
    /// one entry and answer every request that waits on it. `receive()` errors `InvalidSender` and
    /// `Serialize` are logged and skipped; `NetworkError::is_critical()` ends the batch. Payloads
    /// are framed by `frame_external_message`, sent as `MessageType::External` through
    /// `Sender::unicast`, which writes them verbatim.
    async fn fetch_concurrently(
        &mut self,
        requests: &Vec1<StartupRequest>,
        wait: Duration,
    ) -> Vec<anyhow::Result<StartupResponse>>;

    /// Next cert1 or cert2 received within `wait`, unverified.
    async fn next_observed_certificate(&mut self, wait: Duration) -> Option<ObservedCertificate>;
}

/// An unverified certificate read off the wire.
#[derive(Clone, Debug)]
pub(crate) enum ObservedCertificate {
    /// A certificate of a proposal.
    Cert1(Certificate1<SeqTypes>),
    /// A certificate of a decided leaf.
    Cert2(Certificate2<SeqTypes>),
}

impl ObservedCertificate {
    /// Epoch the certificate claims, if any (`HasEpoch`).
    pub fn epoch(&self) -> Option<EpochNumber> {
        todo!()
    }

    /// Verifies the certificate against `table` through `Verifiable::check`, which also runs the
    /// well-formedness and genesis-view checks. Entries come from
    /// `StakeTableEntries::from(table).0` and the threshold from
    /// `supermajority_threshold(table.total_stakes())`.
    pub fn verify(
        &self,
        table: &HSStakeTable<SeqTypes>,
        epoch_height: u64,
        upgrade_lock: &UpgradeLock<SeqTypes>,
    ) -> anyhow::Result<()> {
        todo!()
    }
}

/// Newest cert2-verified leaf; older headers are proven against its block Merkle root.
#[derive(Clone, Debug)]
pub(crate) struct VerifiedAnchorLeaf {
    leaf: Leaf2<SeqTypes>,
}

impl VerifiedAnchorLeaf {
    /// Only constructor: the leaf `CertifiedLeafChain::verify` returned.
    pub fn from_verified(leaf: Leaf2<SeqTypes>) -> Self {
        Self { leaf }
    }

    /// Epoch of the leaf (`Leaf2::epoch`); `None` before epochs are enabled.
    pub fn epoch(&self, epoch_height: u64) -> Option<EpochNumber> {
        self.leaf.epoch(epoch_height)
    }

    /// Block height of the leaf.
    pub fn height(&self) -> u64 {
        self.leaf.height()
    }

    /// Header at `expected_height`, if `proof` holds against this leaf's `block_merkle_tree_root`
    /// and proves that height. `HeaderProof::verify` checks the proof against the header it
    /// carries, not against the height requested, so the height check is here.
    pub fn verify_header_proof(
        &self,
        proof: HeaderProof,
        expected_height: u64,
    ) -> anyhow::Result<Header> {
        todo!()
    }
}

/// Candidate keys dialed during the fill; `apply_epoch(C)` removes those outside the window.
#[derive(Clone, Debug)]
pub(crate) struct BootstrapPeers(BTreeMap<PubKey, PeerConnectInfo>);

impl BootstrapPeers {
    /// `ST(now) ∪ ST(E')`; connect info from `ST(now)` first, else from `ST(E')`. Pure.
    ///
    /// `l1_finalized_candidate_stake_table` is `ST(now)`. `voting_epoch_peers` is `ST(E')` in
    /// committee order: `LcContractCandidateStakeTable::peer_configs`, or the seeded genesis
    /// table for `E' <= first + 1`.
    ///
    /// `ST(now)` is computed once at boot and covers a lagging prover with normal churn. A complete
    /// committee turnover during the lag leaves no reachable window peer; the startup then exits at
    /// the deadline with stage `ObserveCertificates`.
    pub fn new(
        l1_finalized_candidate_stake_table: &AuthenticatedValidatorMap,
        voting_epoch_peers: &IndexMap<PubKey, PeerConfig<SeqTypes>>,
    ) -> Self {
        todo!()
    }

    /// Where the own key stands at `b_fin`: in `P`, registered but not in `P`, or not registered.
    /// Pure.
    ///
    /// Check order: `P` first, then the L1 validator registry. Not registered (never registered, or
    /// exited, which removes the validator from the registry) exits with `OwnKeyNotRegistered`;
    /// registered but not in `P` waits for the next L1 finality tick and re-runs the L1 phase.
    pub fn own_key_status(
        &self,
        own_key: &PubKey,
        l1_finalized_registry: &RegisteredValidatorMap,
    ) -> OwnKeyStatus {
        todo!()
    }

    /// Whether `key` is one of the bootstrap peers.
    pub fn contains(&self, key: &PubKey) -> bool {
        self.0.contains_key(key)
    }

    /// Keys with their connect info, for `StartupCliquenet::dial`.
    pub fn iter(&self) -> impl Iterator<Item = (&PubKey, &PeerConnectInfo)> {
        self.0.iter()
    }
}

/// Own key against `P` first, then the L1 validator registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OwnKeyStatus {
    /// The key is a bootstrap peer: the fill runs.
    InBootstrapPeers,
    /// The fill waits: the L1 phase re-runs on each L1 finality tick, logging the key and `b_fin`.
    RegisteredNotInPeers,
    /// The startup exits with `OwnKeyNotRegistered`.
    NotRegistered,
}

/// Which startup path the node takes, decided after the LC contract read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StartupPath {
    /// Storage is recent: `apply_epoch(anchor_epoch)` and start, as today's restart.
    FromStorage { anchor_epoch: EpochNumber },
    /// Run the fill.
    Fill,
}

impl StartupPath {
    /// `FromStorage` iff `anchor_epoch >= lc_contract_voting_epoch - 1` and the tables of
    /// `max(first, anchor_epoch - 1) ..= anchor_epoch + 1` are in `held_tables`; else `Fill`. Pure.
    ///
    /// `held_tables` is read through `membership().snapshot(epoch)`, never `stake_table_for_epoch`.
    ///
    /// Gap: prover lag plus long downtime with `anchor_epoch >= E' - 1` but `C > anchor_epoch + 1`
    /// starts at `anchor_epoch` and relies on post-start catchup, as today.
    pub fn choose(
        anchor_epoch: EpochNumber,
        lc_contract_voting_epoch: EpochNumber,
        first_epoch: EpochNumber,
        held_tables: &BTreeSet<EpochNumber>,
    ) -> Self {
        todo!()
    }
}

/// What the fill loop knows.
#[derive(Clone, Debug)]
pub(crate) struct StartupProgress {
    /// Blocks per epoch; `next_step` derives request heights from it.
    pub epoch_height: u64,
    /// First epoch with a stake table of its own.
    pub first_epoch: EpochNumber,
    /// `E'`: epoch whose table the LC contract commits to.
    pub lc_contract_voting_epoch: EpochNumber,
    /// Epochs whose stake tables are installed.
    pub held_tables: BTreeSet<EpochNumber>,
    /// Epochs whose DRB results are installed.
    pub held_drbs: BTreeSet<EpochNumber>,
    /// Height of the newest verified anchor leaf; its epoch follows from `epoch_height`.
    pub anchor_height: Option<u64>,
    /// Highest epoch of a verified certificate.
    pub cliquenet_current_epoch: Option<EpochNumber>,
    /// Highest epoch of a certificate no held quorum verifies.
    pub highest_unverified_claim: Option<EpochNumber>,
}

impl StartupProgress {
    /// The next action of the fill loop. Pure.
    ///
    /// - Batching: every root header proof of the missing window epochs and every missing DRB leaf
    ///   go into one `Fetch` batch once the anchor is held. Results are applied in ascending epoch
    ///   order, and DRB leaves are verified after the tables they need.
    /// - Prover lag: while `highest_unverified_claim` exceeds the held quorums, the batch is
    ///   `RootHeaderProof` of `X+1` (from `root(X-1)` against the anchor of `X`), then
    ///   `VerifiedLeaf` of `X+1`. No epoch cap; the deadline bounds it, then the node exits naming
    ///   the step.
    /// - While claims are only at `E'-1`, it returns `ObserveCertificates` until the chain enters
    ///   `E'`.
    pub fn next_step(&self) -> StartupStep {
        todo!()
    }

    /// `max(first + 2, C - 3) ..= C + 1` with `C = cliquenet_current_epoch`; `None` until `C` is
    /// known. Pure.
    pub fn epoch_window(&self) -> Option<RangeInclusive<EpochNumber>> {
        todo!()
    }
}

/// Next action of the fill loop.
#[derive(Clone, Debug)]
pub(crate) enum StartupStep {
    /// Read inbound cert1 and cert2 until one verifies or the chain enters `E'`.
    ObserveCertificates,
    /// Every request whose inputs are held, issued together by `fetch_concurrently`.
    Fetch(Vec1<StartupRequest>),
    /// The window and the DRBs are held; `cliquenet_current_epoch` is `C`.
    Done {
        cliquenet_current_epoch: EpochNumber,
    },
}

/// One fill request; independent of the others in its batch.
///
/// The heights are derivable from the epoch (`epoch_height`), but the adapter has no
/// `epoch_height`, so the planner carries them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum StartupRequest {
    /// `Leaf(first block of epoch)` and its cert2; verified under `ST(epoch)`, it becomes the
    /// anchor.
    VerifiedLeaf { epoch: EpochNumber, height: u64 },
    /// Header proof of `root(epoch - 2)` at snapshot `anchor_height`; installs the table of
    /// `epoch`.
    RootHeaderProof {
        epoch: EpochNumber,
        anchor_height: u64,
        root_height: u64,
    },
    /// `Leaf(transition_block_for_epoch(epoch - 1))` and its cert2; carries the DRB of `epoch`.
    DrbLeaf { epoch: EpochNumber, height: u64 },
}

/// The answer to one `StartupRequest`, unverified.
#[derive(Clone, Debug)]
pub(crate) enum StartupResponse {
    /// Answer to `VerifiedLeaf` and `DrbLeaf`.
    LeafChain(Box<CertifiedLeafChain<SeqTypes>>),
    /// Answer to `RootHeaderProof`.
    HeaderProof(Box<HeaderProof>),
}

/// Why the startup catchup exits. Every variant exits the node; none is recovered from.
#[derive(Debug, Error)]
pub(crate) enum StartupCatchupError {
    /// No L1 block and version reproduce `votingStakeTableState`; our L1 view differs from the LC
    /// contract's. Exits, does not recover. Wraps the matcher's error; its
    /// `NoLcContractCommitmentMatch` variant is the one operators see.
    #[error(transparent)]
    LcContractStakeTableMatch(#[from] LcContractStakeTableMatchError),
    /// The own BLS key is not in the L1 validator registry at `b_fin` (never registered or exited).
    /// Exits, does not recover; the operator registers the key.
    #[error("own key {key} is not registered at the L1 finalized block")]
    OwnKeyNotRegistered { key: Box<PubKey> },
    /// Committee `E'` installed from the proven `root(E'-2)` header commits to another table than
    /// the LC contract candidate. Exits, does not recover; both commitments are logged at ERROR
    /// first.
    #[error("installed committee of epoch {epoch} differs from the LC contract candidate")]
    InstalledCommitmentMismatch { epoch: EpochNumber },
    /// A step ran out of time. `stage` names it; a lagging prover with no serving peer and a
    /// committee turnover with no reachable window peer both end here. Exits, does not recover: no
    /// HTTP fallback, the operator retries.
    #[error("startup deadline exceeded at {stage:?} after {attempts} attempts")]
    DeadlineExceeded {
        stage: StartupStage,
        attempts: usize,
        source: anyhow::Error,
    },
}

/// A retried stage, named in `DeadlineExceeded`. Each stage exits at the deadline, does not
/// recover.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StartupStage {
    /// L1 RPC unreachable or a getter failed. Exits at the deadline, does not recover.
    L1Read,
    /// No certificate from a window peer verified: no window peer reachable, or the network is
    /// stalled. Exits at the deadline, does not recover.
    ObserveCertificates,
    /// No bootstrap peer served a verifying leaf chain for `epoch`. Exits at the deadline, does not
    /// recover.
    FetchVerifiedLeaf { epoch: EpochNumber },
    /// No bootstrap peer served a header proof of `root(epoch - 2)` that holds against the anchor.
    /// Exits at the deadline, does not recover.
    InstallEpoch { epoch: EpochNumber },
    /// No bootstrap peer served the transition leaf carrying the DRB of `epoch`. Exits at the
    /// deadline, does not recover.
    InstallDrb { epoch: EpochNumber },
}

/// Follower startup: walk forward from the highest already-known epoch until peers can no
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

#[cfg(test)]
mod tests {
    mod pure {
        #[test]
        #[ignore = "skeleton"]
        fn progress_epoch_window_bounds_ok() {
            todo!()
        }

        #[test]
        #[ignore = "skeleton"]
        fn progress_next_step_batches_independent_ok() {
            todo!()
        }

        #[test]
        #[ignore = "skeleton"]
        fn wait_for_voting_epoch_ok() {
            todo!()
        }

        #[test]
        #[ignore = "skeleton"]
        fn recent_storage_starts_without_fill_ok() {
            todo!()
        }

        #[test]
        #[ignore = "skeleton"]
        fn stale_storage_runs_fill_ok() {
            todo!()
        }

        #[test]
        #[ignore = "skeleton"]
        fn forged_high_claim_ok() {
            todo!()
        }

        #[test]
        #[ignore = "skeleton"]
        fn bootstrap_peers_union_ok() {
            todo!()
        }

        #[test]
        #[ignore = "skeleton"]
        fn header_proof_wrong_height_fails() {
            todo!()
        }

        #[test]
        #[ignore = "skeleton"]
        fn header_proof_wrong_root_fails() {
            todo!()
        }
    }
}
