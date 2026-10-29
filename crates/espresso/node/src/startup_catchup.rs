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

use std::{
    collections::{BTreeMap, BTreeSet},
    ops::RangeInclusive,
    time::Duration,
};

use anyhow::{Context, anyhow, ensure};
use async_trait::async_trait;
use espresso_types::{
    AuthenticatedValidatorMap, Header, LcContractCandidateStakeTable,
    LcContractStakeTableMatchError, LcContractVotingStakeTable, NodeState, PubKey,
    RegisteredValidatorMap, SeqTypes, ValidatorSet, installed_stake_table,
    v0_3::{EventKey, StakeTableError, StakeTableEvent},
};
use hotshot_new_protocol::{cert_verifier::Verifiable, utils::CertifiedLeafChain};
use hotshot_types::{
    PeerConfig, PeerConnectInfo,
    data::{EpochNumber, Leaf2, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    simple_certificate::{Certificate1, Certificate2},
    simple_vote::HasEpoch,
    stake_table::{HSStakeTable, StakeTableEntries, supermajority_threshold},
    traits::election::{Membership, MembershipSnapshot},
    utils::{epoch_from_block_number, root_block_in_epoch, transition_block_for_epoch},
    vote::HasViewNumber,
};
use indexmap::IndexMap;
use light_client::consensus::header::HeaderProof;
use thiserror::Error;
use tokio::time::{Instant, sleep, timeout};
use vbs::version::Version;
use vec1::Vec1;
use versions::{NEW_PROTOCOL_VERSION, Upgrade};

/// All startup steps together; the node exits at expiry.
pub(crate) const STARTUP_CATCHUP_DEADLINE: Duration = Duration::from_secs(15 * 60);

/// Bootstrap peers asked per fill request.
pub(crate) const STARTUP_FAN_OUT: usize = 5;

/// First wait after a failed attempt of a stage; doubles up to `RETRY_BACKOFF_MAX`.
const RETRY_BACKOFF_INITIAL: Duration = Duration::from_secs(1);

const RETRY_BACKOFF_MAX: Duration = Duration::from_secs(60);

/// Unverifiable certificates kept to re-check after the next table is installed.
const PENDING_CERTIFICATES_LIMIT: usize = 64;

/// Target of the startup log lines.
const LOG_TARGET: &str = "announce::startup_catchup";

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
///
/// L1 reads retry for `params.deadline` each. The deadline of the fill runs from the dial, so an
/// own key that waits to enter `P` does not consume it.
pub(crate) async fn catch_up_over_cliquenet(
    l1: &impl StartupL1Reader,
    net: &mut impl StartupCliquenet,
    membership: &EpochMembershipCoordinator<SeqTypes>,
    params: &StartupCatchupParams,
) -> Result<EpochNumber, StartupCatchupError> {
    let started = Instant::now();
    let mut stats = StartupStats::default();
    let plan = match L1Phase::new(l1, membership, params).run(&mut stats).await? {
        L1Outcome::FromStorage(anchor_epoch) => return Ok(anchor_epoch),
        L1Outcome::Fill(plan) => plan,
    };

    tracing::info!(
        target: LOG_TARGET,
        count = plan.peers.iter().count(),
        own_key_included = plan.peers.contains(&params.own_key),
        "bootstrap peers"
    );
    net.dial(&plan.peers).map_err(StartupCatchupError::Dial)?;

    let current_epoch = Fill::new(net, membership, params, plan, &mut stats)
        .run()
        .await?;
    tracing::info!(
        target: LOG_TARGET,
        %current_epoch,
        duration_ms = started.elapsed().as_millis() as u64,
        l1_read = %stats.l1_read,
        observe = %stats.observe,
        verified_leaf = %stats.verified_leaf,
        header_proofs = %stats.header_proofs,
        drb_leaves = %stats.drb_leaves,
        "startup catchup done"
    );
    Ok(current_epoch)
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

impl StartupCatchupParams {
    /// Selection version of `ST(now)`: the newest candidate, the version the network runs once the
    /// upgrade is enacted.
    fn running_version(&self) -> Version {
        self.candidate_versions
            .iter()
            .max()
            .copied()
            .expect("candidate versions are never empty")
    }
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
        let finalized = self.l1_client.snapshot().await.finalized;
        Ok(finalized.context("no L1 finalized block yet")?.number)
    }

    async fn lc_contract_voting_stake_table(
        &self,
        l1_block: u64,
    ) -> anyhow::Result<LcContractVotingStakeTable> {
        NodeState::lc_contract_voting_stake_table(self, l1_block).await
    }

    async fn stake_table_events(
        &self,
        to_l1_block: u64,
    ) -> anyhow::Result<Vec<(EventKey, StakeTableEvent)>> {
        let contract = self
            .chain_config
            .stake_table_contract
            .context("no stake table contract in chain config")?;
        self.coordinator
            .membership()
            .fetcher()
            .fetch_and_store_stake_table_events(contract, to_l1_block)
            .await
    }

    async fn next_l1_finalized_block(&self, after: u64) -> anyhow::Result<u64> {
        Ok(self
            .l1_client
            .wait_for_finalized_block(after + 1)
            .await
            .number)
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
        match self {
            Self::Cert1(cert) => cert.epoch(),
            Self::Cert2(cert) => cert.epoch(),
        }
    }

    /// View of the certificate.
    pub fn view(&self) -> ViewNumber {
        match self {
            Self::Cert1(cert) => cert.view_number(),
            Self::Cert2(cert) => cert.view_number(),
        }
    }

    /// Block height the certificate claims.
    pub fn block_number(&self) -> Option<u64> {
        match self {
            Self::Cert1(cert) => cert.data.block_number,
            Self::Cert2(cert) => Some(cert.data.block_number),
        }
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
        let entries = StakeTableEntries::<SeqTypes>::from(table.clone()).0;
        let threshold = supermajority_threshold(table.total_stakes());
        match self.clone() {
            Self::Cert1(cert) => {
                cert.check(&entries, threshold, epoch_height, upgrade_lock)?;
            },
            Self::Cert2(cert) => {
                cert.check(&entries, threshold, epoch_height, upgrade_lock)?;
            },
        }
        Ok(())
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
        let header = proof.verify(self.leaf.block_header().block_merkle_tree_root())?;
        ensure!(
            header.height() == expected_height,
            "header proof is for height {}, expected {expected_height}",
            header.height()
        );
        Ok(header)
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
    /// table for `E' <= first + 1`. Keys without connect info cannot be dialed and are left out.
    ///
    /// `ST(now)` is computed once at boot and covers a lagging prover with normal churn. A complete
    /// committee turnover during the lag leaves no reachable window peer; the startup then exits at
    /// the deadline with stage `ObserveCertificates`.
    pub fn new(
        l1_finalized_candidate_stake_table: &AuthenticatedValidatorMap,
        voting_epoch_peers: &IndexMap<PubKey, PeerConfig<SeqTypes>>,
    ) -> Self {
        let mut peers: BTreeMap<_, _> = voting_epoch_peers
            .iter()
            .filter_map(|(key, peer)| Some((*key, peer.connect_info.clone()?)))
            .collect();
        for validator in l1_finalized_candidate_stake_table.values() {
            if let (Some(x25519_key), Some(p2p_addr)) =
                (validator.x25519_key, validator.p2p_addr.clone())
            {
                peers.insert(
                    *validator.stake_table_key(),
                    PeerConnectInfo {
                        x25519_key,
                        p2p_addr,
                    },
                );
            }
        }
        Self(peers)
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
        if self.contains(own_key) {
            OwnKeyStatus::InBootstrapPeers
        } else if l1_finalized_registry
            .values()
            .any(|validator| validator.stake_table_key.as_ref() == Some(own_key))
        {
            OwnKeyStatus::RegisteredNotInPeers
        } else {
            OwnKeyStatus::NotRegistered
        }
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
        let recent = anchor_epoch + 1 >= lc_contract_voting_epoch;
        let lowest = first_epoch.max(EpochNumber::new(anchor_epoch.saturating_sub(1)));
        let window_held = (*lowest..=*anchor_epoch + 1)
            .all(|epoch| held_tables.contains(&EpochNumber::new(epoch)));
        if recent && window_held {
            Self::FromStorage { anchor_epoch }
        } else {
            Self::Fill
        }
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
    /// Highest block height of a verified certificate; tells whether the transition leaf of `C`
    /// exists yet.
    pub cliquenet_current_height: Option<u64>,
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
        let voting_epoch = self.lc_contract_voting_epoch;
        let claim = self
            .cliquenet_current_epoch
            .max(self.highest_unverified_claim);
        let Some(claim) = claim.filter(|claim| *claim >= voting_epoch) else {
            return StartupStep::ObserveCertificates;
        };
        let Some(anchor_height) = self.anchor_height else {
            return self.fetch([StartupRequest::VerifiedLeaf {
                epoch: voting_epoch,
                height: self.first_block(voting_epoch),
            }]);
        };
        let anchor_epoch =
            EpochNumber::new(epoch_from_block_number(anchor_height, self.epoch_height));
        if anchor_epoch < claim {
            return self.walk_step(anchor_epoch, anchor_height);
        }
        match self.cliquenet_current_epoch {
            Some(current) => self.window_step(current, anchor_height),
            None => StartupStep::ObserveCertificates,
        }
    }

    /// `max(first + 2, C - 3) ..= C + 1` with `C = cliquenet_current_epoch`; `None` until `C` is
    /// known. Pure.
    pub fn epoch_window(&self) -> Option<RangeInclusive<EpochNumber>> {
        let current = self.cliquenet_current_epoch?;
        let lowest = (self.first_epoch + 2).max(EpochNumber::new(current.saturating_sub(3)));
        Some(lowest..=current + 1)
    }

    /// First block of `epoch`.
    fn first_block(&self, epoch: EpochNumber) -> u64 {
        (*epoch - 1) * self.epoch_height + 1
    }

    fn fetch(&self, requests: impl IntoIterator<Item = StartupRequest>) -> StartupStep {
        let requests: Vec<_> = requests.into_iter().collect();
        Vec1::try_from_vec(requests)
            .map_or_else(|_| StartupStep::ObserveCertificates, StartupStep::Fetch)
    }

    /// One step of the verified walk: the table of `X+1`, then the leaf of `X+1`.
    fn walk_step(&self, anchor_epoch: EpochNumber, anchor_height: u64) -> StartupStep {
        let next = anchor_epoch + 1;
        if self.held_tables.contains(&next) {
            return self.fetch([StartupRequest::VerifiedLeaf {
                epoch: next,
                height: self.first_block(next),
            }]);
        }
        self.fetch([StartupRequest::RootHeaderProof {
            epoch: next,
            anchor_height,
            root_height: root_block_in_epoch(*anchor_epoch - 1, self.epoch_height),
        }])
    }

    /// Every missing window table and DRB leaf in one batch, or `Done`.
    fn window_step(&self, current: EpochNumber, anchor_height: u64) -> StartupStep {
        let window = self.epoch_window().expect("current epoch is known");
        let tables = (**window.start()..=**window.end())
            .map(EpochNumber::new)
            .filter(|epoch| !self.held_tables.contains(epoch))
            .map(|epoch| StartupRequest::RootHeaderProof {
                epoch,
                anchor_height,
                root_height: root_block_in_epoch(*epoch - 2, self.epoch_height),
            });
        let drbs = [current, current + 1]
            .into_iter()
            .filter(|epoch| !self.held_drbs.contains(epoch) && self.transition_leaf_exists(*epoch))
            .map(|epoch| StartupRequest::DrbLeaf {
                epoch,
                height: transition_block_for_epoch(*epoch - 1, self.epoch_height),
            });
        let requests: Vec<_> = tables.chain(drbs).collect();
        match Vec1::try_from_vec(requests) {
            Ok(requests) => StartupStep::Fetch(requests),
            Err(_) => StartupStep::Done {
                cliquenet_current_epoch: current,
            },
        }
    }

    /// Whether the leaf carrying the DRB of `epoch` can exist: `epoch` is not past the current
    /// epoch, or a verified certificate is above the transition block of the current epoch.
    fn transition_leaf_exists(&self, epoch: EpochNumber) -> bool {
        let Some(current) = self.cliquenet_current_epoch else {
            return false;
        };
        epoch <= current
            || self.cliquenet_current_height.is_some_and(|height| {
                height > transition_block_for_epoch(*current, self.epoch_height)
            })
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

impl StartupRequest {
    /// The stage a failure of this request is reported under.
    fn stage(&self) -> StartupStage {
        match *self {
            Self::VerifiedLeaf { epoch, .. } => StartupStage::FetchVerifiedLeaf { epoch },
            Self::RootHeaderProof { epoch, .. } => StartupStage::InstallEpoch { epoch },
            Self::DrbLeaf { epoch, .. } => StartupStage::InstallDrb { epoch },
        }
    }
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
    /// The cliquenet instance refused the bootstrap peers: its network task is down. Exits, does
    /// not recover.
    #[error("failed to dial the bootstrap peers")]
    Dial(#[source] anyhow::Error),
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

/// Time, answered and unanswered requests of one kind of stage.
#[derive(Clone, Copy, Debug, Default)]
struct StageStats {
    duration: Duration,
    answered: usize,
    unanswered: usize,
}

impl std::fmt::Display for StageStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "duration_ms={} answered={} unanswered={}",
            self.duration.as_millis(),
            self.answered,
            self.unanswered
        )
    }
}

/// Per-stage statistics for the `startup catchup done` line.
#[derive(Clone, Copy, Debug, Default)]
struct StartupStats {
    l1_read: StageStats,
    observe: StageStats,
    verified_leaf: StageStats,
    header_proofs: StageStats,
    drb_leaves: StageStats,
}

impl StartupStats {
    /// The statistics of the stage `request` belongs to.
    fn of_request(&mut self, request: &StartupRequest) -> &mut StageStats {
        match request {
            StartupRequest::VerifiedLeaf { .. } => &mut self.verified_leaf,
            StartupRequest::RootHeaderProof { .. } => &mut self.header_proofs,
            StartupRequest::DrbLeaf { .. } => &mut self.drb_leaves,
        }
    }
}

/// Backoff and deadline of the retried stages.
#[derive(Debug)]
struct Retry {
    deadline: Instant,
    attempts: usize,
    delay: Duration,
}

impl Retry {
    fn new(budget: Duration) -> Self {
        Self {
            deadline: Instant::now() + budget,
            attempts: 0,
            delay: RETRY_BACKOFF_INITIAL,
        }
    }

    /// Progress was made: the next failure starts from the first backoff again.
    fn reset(&mut self) {
        self.attempts = 0;
        self.delay = RETRY_BACKOFF_INITIAL;
    }

    /// Exit if the deadline has passed.
    fn check(&self, stage: StartupStage, source: anyhow::Error) -> Result<(), StartupCatchupError> {
        if Instant::now() >= self.deadline {
            return Err(StartupCatchupError::DeadlineExceeded {
                stage,
                attempts: self.attempts,
                source,
            });
        }
        Ok(())
    }

    /// Records a failed attempt of `stage` and waits the backoff, or exits at the deadline.
    async fn failed(
        &mut self,
        stage: StartupStage,
        source: anyhow::Error,
    ) -> Result<(), StartupCatchupError> {
        self.attempts += 1;
        self.check(stage, source.context("attempt failed"))?;
        tracing::warn!(
            target: LOG_TARGET,
            ?stage,
            attempts = self.attempts,
            "startup stage failed, retrying"
        );
        sleep(self.delay.min(self.deadline - Instant::now())).await;
        self.delay = (self.delay * 2).min(RETRY_BACKOFF_MAX);
        Ok(())
    }

    /// Runs `op` until it succeeds, each attempt bounded by `step_timeout`.
    async fn run<T, Fut>(
        &mut self,
        stage: StartupStage,
        step_timeout: Duration,
        mut op: impl FnMut() -> Fut,
    ) -> Result<T, StartupCatchupError>
    where
        Fut: Future<Output = anyhow::Result<T>>,
    {
        loop {
            let result = match timeout(step_timeout, op()).await {
                Ok(result) => result,
                Err(_) => Err(anyhow!("no answer within {step_timeout:?}")),
            };
            match result {
                Ok(value) => return Ok(value),
                Err(err) => self.failed(stage, err).await?,
            }
        }
    }
}

/// Result of the L1 phase.
enum L1Outcome {
    /// Recent storage: start at this epoch without a fill.
    FromStorage(EpochNumber),
    /// Run the fill with these inputs.
    Fill(Box<FillPlan>),
}

/// What the L1 phase hands to the fill.
struct FillPlan {
    lc_contract: LcContractVotingStakeTable,
    /// The candidate `ST(E')`; `None` when `E' <= first + 1`, where the seeded table is installed.
    candidate: Option<LcContractCandidateStakeTable>,
    peers: BootstrapPeers,
}

/// The candidate `ST(E')`, if any, and the committee-ordered peers of `E'`.
type CandidateWithPeers = (
    Option<LcContractCandidateStakeTable>,
    IndexMap<PubKey, PeerConfig<SeqTypes>>,
);

/// The L1 reads of the startup: the LC contract table, the path decision, the candidates and the
/// own-key check.
struct L1Phase<'a, L> {
    l1: &'a L,
    membership: &'a EpochMembershipCoordinator<SeqTypes>,
    params: &'a StartupCatchupParams,
    first_epoch: EpochNumber,
}

impl<'a, L: StartupL1Reader> L1Phase<'a, L> {
    fn new(
        l1: &'a L,
        membership: &'a EpochMembershipCoordinator<SeqTypes>,
        params: &'a StartupCatchupParams,
    ) -> Self {
        Self {
            l1,
            membership,
            params,
            first_epoch: membership
                .membership()
                .first_epoch()
                .expect("seed_membership sets the first epoch"),
        }
    }

    /// Reads L1 until the path is decided; re-runs on each L1 finality tick while the own key is
    /// registered but not in `P`.
    async fn run(&self, stats: &mut StartupStats) -> Result<L1Outcome, StartupCatchupError> {
        let started = Instant::now();
        let step_timeout = self.params.step_timeout;
        let mut retry = Retry::new(self.params.deadline);
        let mut b_fin = retry
            .run(StartupStage::L1Read, step_timeout, || {
                self.l1.l1_finalized_block()
            })
            .await?;
        loop {
            let lc_contract = retry
                .run(StartupStage::L1Read, step_timeout, || {
                    self.l1.lc_contract_voting_stake_table(b_fin)
                })
                .await?;
            tracing::info!(
                target: LOG_TARGET,
                lc_contract_voting_epoch = %lc_contract.voting_epoch,
                l1_block = lc_contract.l1_block,
                finalized_block_height = lc_contract.finalized_block_height,
                "lc contract voting stake table"
            );
            if let Some(outcome) = self.choose_path(&lc_contract) {
                stats.l1_read.duration += started.elapsed();
                return Ok(outcome);
            }

            let events = retry
                .run(StartupStage::L1Read, step_timeout, || {
                    self.l1.stake_table_events(b_fin)
                })
                .await?;
            let (candidate, voting_epoch_peers) = self.candidate(&lc_contract, &events)?;
            let (active, registry) = self.l1_finalized_validators(&events)?;
            let peers = BootstrapPeers::new(&active, &voting_epoch_peers);
            match peers.own_key_status(&self.params.own_key, &registry) {
                OwnKeyStatus::InBootstrapPeers => {
                    stats.l1_read.duration += started.elapsed();
                    return Ok(L1Outcome::Fill(Box::new(FillPlan {
                        lc_contract,
                        candidate,
                        peers,
                    })));
                },
                OwnKeyStatus::NotRegistered => {
                    return Err(StartupCatchupError::OwnKeyNotRegistered {
                        key: Box::new(self.params.own_key),
                    });
                },
                OwnKeyStatus::RegisteredNotInPeers => {
                    tracing::info!(
                        target: LOG_TARGET,
                        key = %self.params.own_key,
                        b_fin,
                        "own key registered but not in bootstrap peers; waiting"
                    );
                    b_fin = retry
                        .run(StartupStage::L1Read, step_timeout, || {
                            self.l1.next_l1_finalized_block(b_fin)
                        })
                        .await?;
                },
            }
        }
    }

    /// `ST(now)` and the registry at `b_fin`; both empty while no validator is eligible yet.
    fn l1_finalized_validators(
        &self,
        events: &[(EventKey, StakeTableEvent)],
    ) -> Result<(AuthenticatedValidatorMap, RegisteredValidatorMap), StartupCatchupError> {
        let set = ValidatorSet::from_l1_events(
            events.iter().map(|(_, event)| event.clone()),
            self.params.running_version(),
        );
        match set {
            Ok(set) => Ok((
                set.active_validators().clone(),
                set.all_validators().clone(),
            )),
            Err(StakeTableError::NoValidValidators) => Ok(Default::default()),
            Err(err) => Err(LcContractStakeTableMatchError::from(err).into()),
        }
    }

    /// `FromStorage` when storage is recent, logging the decision; `None` for the fill.
    fn choose_path(&self, lc_contract: &LcContractVotingStakeTable) -> Option<L1Outcome> {
        let held = held_epochs(
            self.membership,
            self.first_epoch..=self.params.anchor_epoch + 1,
        );
        let path = StartupPath::choose(
            self.params.anchor_epoch,
            lc_contract.voting_epoch,
            self.first_epoch,
            &held,
        );
        match path {
            StartupPath::FromStorage { anchor_epoch } => {
                tracing::info!(
                    target: LOG_TARGET,
                    path = "start from storage",
                    %anchor_epoch,
                    lc_contract_voting_epoch = %lc_contract.voting_epoch,
                    "startup path"
                );
                Some(L1Outcome::FromStorage(anchor_epoch))
            },
            StartupPath::Fill => {
                tracing::info!(target: LOG_TARGET, path = "fill", "startup path");
                None
            },
        }
    }

    /// The candidate `ST(E')` with its peers: the seeded genesis table for `E' <= first + 1`,
    /// else the match in the L1 events.
    fn candidate(
        &self,
        lc_contract: &LcContractVotingStakeTable,
        events: &[(EventKey, StakeTableEvent)],
    ) -> Result<CandidateWithPeers, StartupCatchupError> {
        if lc_contract.voting_epoch <= self.first_epoch + 1 {
            let seeded = self.seeded_peers(lc_contract)?;
            return Ok((None, seeded));
        }
        let candidate = LcContractCandidateStakeTable::find(
            lc_contract,
            events,
            &self.params.candidate_versions,
            self.params.stake_table_capacity,
        )?;
        tracing::info!(
            target: LOG_TARGET,
            epoch = %candidate.epoch,
            l1_block = candidate.l1_block,
            version = %candidate.version,
            "lc contract candidate stake table"
        );
        let peers = candidate.peer_configs();
        Ok((Some(candidate), peers))
    }

    /// The seeded table of `E'`, checked against `votingStakeTableState`.
    fn seeded_peers(
        &self,
        lc_contract: &LcContractVotingStakeTable,
    ) -> Result<IndexMap<PubKey, PeerConfig<SeqTypes>>, StartupCatchupError> {
        let epoch = lc_contract.voting_epoch;
        let no_match = || LcContractStakeTableMatchError::NoLcContractCommitmentMatch {
            epoch,
            searched_l1_blocks: lc_contract.l1_block..=lc_contract.l1_block,
        };
        let snapshot = self
            .membership
            .membership()
            .snapshot(epoch)
            .ok_or_else(no_match)?;
        let table = HSStakeTable::<SeqTypes>::from_iter(snapshot.stake_table());
        let commitment = table
            .commitment(self.params.stake_table_capacity)
            .map_err(|source| LcContractStakeTableMatchError::Commitment { source })?;
        if commitment != lc_contract.commitment {
            return Err(no_match().into());
        }
        Ok(snapshot
            .stake_table()
            .map(|peer| (*peer.stake_table_entry.key(), peer.clone()))
            .collect())
    }
}

/// The epochs of `range` whose stake tables are in memory, read without starting catchup.
fn held_epochs(
    membership: &EpochMembershipCoordinator<SeqTypes>,
    range: RangeInclusive<EpochNumber>,
) -> BTreeSet<EpochNumber> {
    (**range.start()..=**range.end())
        .map(EpochNumber::new)
        .filter(|epoch| membership.membership().snapshot(*epoch).is_some())
        .collect()
}

/// The fill: observes certificates and fetches until the window and the DRBs are held.
struct Fill<'a, N> {
    net: &'a mut N,
    membership: &'a EpochMembershipCoordinator<SeqTypes>,
    params: &'a StartupCatchupParams,
    lc_contract: LcContractVotingStakeTable,
    candidate: Option<LcContractCandidateStakeTable>,
    progress: StartupProgress,
    anchor: Option<VerifiedAnchorLeaf>,
    /// Certificates no held quorum verified yet, re-checked after each installed table.
    pending: Vec<ObservedCertificate>,
    /// Epochs installed from a header proof by this fill.
    installed: BTreeSet<EpochNumber>,
    upgrade_lock: UpgradeLock<SeqTypes>,
    warned_no_inbound: bool,
    stats: &'a mut StartupStats,
}

impl<'a, N: StartupCliquenet> Fill<'a, N> {
    fn new(
        net: &'a mut N,
        membership: &'a EpochMembershipCoordinator<SeqTypes>,
        params: &'a StartupCatchupParams,
        plan: Box<FillPlan>,
        stats: &'a mut StartupStats,
    ) -> Self {
        let progress = StartupProgress {
            epoch_height: *membership.epoch_height(),
            first_epoch: membership
                .membership()
                .first_epoch()
                .expect("seed_membership sets the first epoch"),
            lc_contract_voting_epoch: plan.lc_contract.voting_epoch,
            held_tables: BTreeSet::new(),
            held_drbs: BTreeSet::new(),
            anchor_height: None,
            cliquenet_current_epoch: None,
            cliquenet_current_height: None,
            highest_unverified_claim: None,
        };
        Self {
            net,
            membership,
            params,
            lc_contract: plan.lc_contract,
            candidate: plan.candidate,
            progress,
            anchor: None,
            pending: Vec::new(),
            installed: BTreeSet::new(),
            upgrade_lock: UpgradeLock::new(Upgrade::trivial(NEW_PROTOCOL_VERSION)),
            warned_no_inbound: false,
            stats,
        }
    }

    /// Runs steps until `Done`; a step that makes no progress backs off, and the deadline ends the
    /// fill.
    async fn run(mut self) -> Result<EpochNumber, StartupCatchupError> {
        let mut retry = Retry::new(self.params.deadline);
        let mut last_error = anyhow!("no step ran");
        loop {
            self.refresh_held();
            let step = self.progress.next_step();
            let stage = match &step {
                StartupStep::Done {
                    cliquenet_current_epoch,
                } => {
                    self.warn_if_no_inbound();
                    self.log_installed();
                    return Ok(*cliquenet_current_epoch);
                },
                StartupStep::ObserveCertificates => StartupStage::ObserveCertificates,
                StartupStep::Fetch(batch) => batch.first().stage(),
            };
            retry.check(stage, anyhow!("{last_error:#}"))?;
            let outcome = match &step {
                StartupStep::Fetch(batch) => self.fetch(batch).await,
                _ => self.observe().await,
            };
            match outcome {
                Ok(()) => retry.reset(),
                Err(err) => {
                    let err = match err.downcast::<StartupCatchupError>() {
                        Ok(fatal) => return Err(fatal),
                        Err(err) => err,
                    };
                    self.warn_if_no_inbound();
                    last_error = anyhow!("{err:#}");
                    retry.failed(stage, err).await?;
                },
            }
        }
    }

    /// Refreshes the held tables and DRBs from the membership, which is the source of truth.
    fn refresh_held(&mut self) {
        let progress = &mut self.progress;
        let highest = [
            progress.lc_contract_voting_epoch + 1,
            progress
                .cliquenet_current_epoch
                .map_or(progress.first_epoch, |c| c + 2),
            progress
                .highest_unverified_claim
                .map_or(progress.first_epoch, |c| c + 1),
            progress
                .anchor_height
                .map_or(progress.first_epoch, |height| {
                    EpochNumber::new(epoch_from_block_number(height, progress.epoch_height) + 2)
                }),
        ]
        .into_iter()
        .max()
        .expect("non-empty");
        progress.held_tables = held_epochs(self.membership, progress.first_epoch..=highest);
        progress.held_drbs = progress
            .held_tables
            .iter()
            .copied()
            .filter(|epoch| {
                self.membership
                    .membership()
                    .snapshot(*epoch)
                    .is_some_and(|snapshot| snapshot.has_drb())
            })
            .collect();
    }

    /// The table verifying the certificates of `epoch`: installed, else the candidate.
    fn quorum(&self, epoch: EpochNumber) -> Option<HSStakeTable<SeqTypes>> {
        match &self.candidate {
            Some(candidate) => candidate.quorum_for(epoch, self.membership),
            None => installed_stake_table(self.membership, epoch),
        }
    }

    /// Waits for one certificate, verifies it or keeps it as a claim.
    async fn observe(&mut self) -> anyhow::Result<()> {
        let started = Instant::now();
        let certificate = self
            .net
            .next_observed_certificate(self.params.step_timeout)
            .await;
        self.stats.observe.duration += started.elapsed();
        let Some(certificate) = certificate else {
            self.stats.observe.unanswered += 1;
            return Err(anyhow!(
                "no certificate within {:?}",
                self.params.step_timeout
            ));
        };
        self.stats.observe.answered += 1;
        self.absorb(certificate);
        Ok(())
    }

    /// Verifies `certificate` under the table of its epoch, or keeps it as a claim when no table
    /// is held for that epoch.
    fn absorb(&mut self, certificate: ObservedCertificate) {
        let Some(epoch) = certificate.epoch() else {
            return;
        };
        let Some(table) = self.quorum(epoch) else {
            self.park(epoch, certificate);
            return;
        };
        if let Err(err) = certificate.verify(&table, self.progress.epoch_height, &self.upgrade_lock)
        {
            tracing::debug!(%epoch, "certificate failed verification: {err:#}");
            return;
        }
        let progress = &mut self.progress;
        if progress.cliquenet_current_epoch < Some(epoch) {
            tracing::info!(
                target: LOG_TARGET,
                %epoch,
                cert_view = %certificate.view(),
                "current epoch from cliquenet"
            );
            progress.cliquenet_current_epoch = Some(epoch);
        }
        progress.cliquenet_current_height = progress
            .cliquenet_current_height
            .max(certificate.block_number());
    }

    fn park(&mut self, epoch: EpochNumber, certificate: ObservedCertificate) {
        self.pending.push(certificate);
        if self.pending.len() > PENDING_CERTIFICATES_LIMIT {
            self.pending.sort_by_key(|cert| cert.epoch());
            self.pending.remove(0);
        }
        let claim = &mut self.progress.highest_unverified_claim;
        *claim = (*claim).max(Some(epoch));
    }

    /// Re-checks the parked certificates after tables were installed.
    fn reverify_pending(&mut self) {
        let pending = std::mem::take(&mut self.pending);
        self.progress.highest_unverified_claim = None;
        for certificate in pending {
            self.absorb(certificate);
        }
    }

    /// Sends the batch, applies the verified answers, and fails if nothing advanced.
    async fn fetch(&mut self, batch: &Vec1<StartupRequest>) -> anyhow::Result<()> {
        let started = Instant::now();
        let responses = self
            .net
            .fetch_concurrently(batch, self.params.step_timeout)
            .await;
        let elapsed = started.elapsed();
        let before = self.fingerprint();

        let mut results = self.apply_in_order(batch, responses).await?;
        self.refresh_held();
        self.reverify_pending();
        self.drain_buffered_certificates().await;

        let mut first_error = None;
        for (request, result) in batch.iter().zip(results.drain(..)) {
            let stats = self.stats.of_request(request);
            stats.duration += elapsed;
            match result {
                Ok(()) => stats.answered += 1,
                Err(err) => {
                    stats.unanswered += 1;
                    first_error.get_or_insert(err);
                },
            }
        }
        if self.fingerprint() == before {
            return Err(first_error.unwrap_or_else(|| anyhow!("batch made no progress")));
        }
        Ok(())
    }

    /// Certificates the adapter buffered during the batch.
    async fn drain_buffered_certificates(&mut self) {
        while let Some(certificate) = self.net.next_observed_certificate(Duration::ZERO).await {
            self.absorb(certificate);
        }
    }

    /// What advanced: tables, DRBs, the anchor and the current epoch.
    fn fingerprint(&self) -> (usize, usize, Option<u64>, Option<EpochNumber>) {
        let progress = &self.progress;
        (
            progress.held_tables.len(),
            progress.held_drbs.len(),
            progress.anchor_height,
            progress.cliquenet_current_epoch,
        )
    }

    /// Applies the answers of `batch`: anchor leaves, then tables in ascending epoch order, then
    /// DRB leaves. Tables stop at the first gap, since a later table needs its predecessors.
    async fn apply_in_order(
        &mut self,
        batch: &Vec1<StartupRequest>,
        responses: Vec<anyhow::Result<StartupResponse>>,
    ) -> anyhow::Result<Vec<anyhow::Result<()>>> {
        ensure!(
            responses.len() == batch.len(),
            "{} answers for {} requests",
            responses.len(),
            batch.len()
        );
        let mut items: Vec<_> = batch.iter().cloned().zip(responses).enumerate().collect();
        items.sort_by_key(|(_, (request, _))| apply_rank(request));
        let mut results: Vec<Option<anyhow::Result<()>>> = batch.iter().map(|_| None).collect();
        let mut gap = false;
        for (index, (request, response)) in items {
            let result = match (&request, response) {
                (_, Err(err)) => Err(err),
                (StartupRequest::RootHeaderProof { .. }, _) if gap => {
                    Err(anyhow!("an earlier table is missing"))
                },
                (_, Ok(response)) => self.apply(&request, response).await,
            };
            if let Err(err) = &result {
                if err.downcast_ref::<StartupCatchupError>().is_some() {
                    return Err(result.expect_err("matched above"));
                }
                tracing::debug!(?request, "answer not applied: {err:#}");
                gap |= matches!(request, StartupRequest::RootHeaderProof { .. });
            }
            results[index] = Some(result);
        }
        Ok(results
            .into_iter()
            .map(|result| result.expect("every request is applied"))
            .collect())
    }

    async fn apply(
        &mut self,
        request: &StartupRequest,
        response: StartupResponse,
    ) -> anyhow::Result<()> {
        match (request, response) {
            (StartupRequest::VerifiedLeaf { epoch, height }, StartupResponse::LeafChain(chain)) => {
                self.apply_verified_leaf(*epoch, *height, *chain)
            },
            (
                StartupRequest::RootHeaderProof {
                    epoch,
                    anchor_height,
                    root_height,
                },
                StartupResponse::HeaderProof(proof),
            ) => {
                self.install_epoch(*epoch, *anchor_height, *root_height, *proof)
                    .await
            },
            (StartupRequest::DrbLeaf { epoch, height }, StartupResponse::LeafChain(chain)) => {
                self.apply_drb_leaf(*epoch, *height, *chain)
            },
            (request, _) => Err(anyhow!("answer of the wrong kind for {request:?}")),
        }
    }

    /// Verifies the leaf chain under `ST(epoch)` and makes the leaf the anchor if it is the newest.
    fn apply_verified_leaf(
        &mut self,
        epoch: EpochNumber,
        height: u64,
        chain: CertifiedLeafChain<SeqTypes>,
    ) -> anyhow::Result<()> {
        let leaf = chain.verify(
            height,
            self.progress.epoch_height,
            &self.upgrade_lock,
            |epoch| self.quorum(epoch),
        )?;
        let anchor = VerifiedAnchorLeaf::from_verified(leaf);
        ensure!(
            anchor.epoch(self.progress.epoch_height) == Some(epoch),
            "the verified leaf is not in epoch {epoch}"
        );
        if self
            .anchor
            .as_ref()
            .is_none_or(|current| anchor.height() > current.height())
        {
            tracing::info!(target: LOG_TARGET, %epoch, height, "verified leaf");
            self.progress.anchor_height = Some(anchor.height());
            self.anchor = Some(anchor);
        }
        Ok(())
    }

    /// Proves `root(epoch - 2)` against the anchor and installs it with `add_epoch_root`.
    async fn install_epoch(
        &mut self,
        epoch: EpochNumber,
        anchor_height: u64,
        root_height: u64,
        proof: HeaderProof,
    ) -> anyhow::Result<()> {
        let anchor = self
            .anchor
            .as_ref()
            .filter(|anchor| anchor.height() == anchor_height)
            .context("the anchor of the request is gone")?;
        let header = anchor.verify_header_proof(proof, root_height)?;
        self.membership
            .add_epoch_root(header)
            .await
            .map_err(|err| anyhow!("add_epoch_root for epoch {epoch}: {err}"))?;
        self.installed.insert(epoch);
        self.progress.held_tables.insert(epoch);
        if epoch == self.lc_contract.voting_epoch {
            self.check_installed_commitment(epoch)?;
        }
        Ok(())
    }

    /// The installed committee of `E'` against the LC contract's commitment; exits on a mismatch.
    fn check_installed_commitment(&self, epoch: EpochNumber) -> Result<(), anyhow::Error> {
        let table = installed_stake_table(self.membership, epoch)
            .context("the installed table is missing")?;
        let installed = table.commitment(self.params.stake_table_capacity)?;
        if installed != self.lc_contract.commitment {
            tracing::error!(
                target: LOG_TARGET,
                %epoch,
                installed = ?installed,
                lc_contract = ?self.lc_contract.commitment,
                "installed committee differs from the LC contract candidate"
            );
            return Err(anyhow::Error::new(
                StartupCatchupError::InstalledCommitmentMismatch { epoch },
            ));
        }
        Ok(())
    }

    /// Verifies the transition leaf under the installed `ST(epoch - 1)` and stores its DRB.
    fn apply_drb_leaf(
        &mut self,
        epoch: EpochNumber,
        height: u64,
        chain: CertifiedLeafChain<SeqTypes>,
    ) -> anyhow::Result<()> {
        let leaf = chain.verify(
            height,
            self.progress.epoch_height,
            &self.upgrade_lock,
            |epoch| self.quorum(epoch),
        )?;
        let drb = leaf
            .next_drb_result
            .context("the transition leaf carries no DRB")?;
        self.membership.membership().add_drb_result(epoch, drb);
        self.progress.held_drbs.insert(epoch);
        Ok(())
    }

    /// The firewall hint: peers answered, so outbound connections work, but nothing was accepted.
    fn warn_if_no_inbound(&mut self) {
        let answered = self.stats.observe.answered
            + self.stats.verified_leaf.answered
            + self.stats.header_proofs.answered
            + self.stats.drb_leaves.answered;
        if answered > 0 && !self.warned_no_inbound && self.net.accepted_inbound_connections() == 0 {
            self.warned_no_inbound = true;
            tracing::warn!(
                target: LOG_TARGET,
                "no inbound cliquenet connections; check firewall and registered p2p_addr"
            );
        }
    }

    fn log_installed(&self) {
        let Some(window) = self.progress.epoch_window() else {
            return;
        };
        for epoch in (**window.start()..=**window.end()).map(EpochNumber::new) {
            let source = if self.installed.contains(&epoch) {
                "header_proof"
            } else {
                "stored"
            };
            tracing::info!(target: LOG_TARGET, %epoch, source, "installed epoch");
        }
    }
}

/// Order of application within a batch: anchors, then tables by epoch, then DRBs by epoch.
fn apply_rank(request: &StartupRequest) -> (u8, u64) {
    match request {
        StartupRequest::VerifiedLeaf { epoch, .. } => (0, **epoch),
        StartupRequest::RootHeaderProof { epoch, .. } => (1, **epoch),
        StartupRequest::DrbLeaf { epoch, .. } => (2, **epoch),
    }
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
    use std::{
        collections::VecDeque,
        net::Ipv4Addr,
        sync::{Arc, Mutex},
    };

    use alloy::primitives::{Address, U256};
    use committable::Commitment;
    use espresso_types::{
        EpochCommittees, StakeTableCommitment,
        testing::TestValidator,
        v0_3::{AuthenticatedValidator, Fetcher, RegisteredValidator},
    };
    use hotshot_contract_adapter::sol_types::StakeTableV3::{
        Delegated, ValidatorExit, ValidatorRegisteredV3,
    };
    use hotshot_example_types::storage_types::TestStorage;
    use hotshot_new_protocol::testing::{sign_certificate, sign_leaf_chain};
    use hotshot_query_service::node::BlockId;
    use hotshot_types::{
        ValidatorConfig, addr::NetAddr, signature_key::BLSPrivKey, simple_vote::QuorumData2,
        traits::signature_key::SignatureKey, x25519,
    };
    use light_client::{client::Client, testing::TestClient};
    use rstest::rstest;
    use tracing_subscriber::fmt::MakeWriter;

    use super::*;

    const EPOCH_HEIGHT: u64 = 10;
    const CAPACITY: usize = 10;

    fn epochs(range: impl IntoIterator<Item = u64>) -> BTreeSet<EpochNumber> {
        range.into_iter().map(EpochNumber::new).collect()
    }

    mod pure {
        use super::*;

        fn progress(voting_epoch: u64) -> StartupProgress {
            StartupProgress {
                epoch_height: 100,
                first_epoch: EpochNumber::new(1),
                lc_contract_voting_epoch: EpochNumber::new(voting_epoch),
                held_tables: epochs([1, 2]),
                held_drbs: epochs([1, 2]),
                anchor_height: None,
                cliquenet_current_epoch: None,
                cliquenet_current_height: None,
                highest_unverified_claim: None,
            }
        }

        fn fetched(step: StartupStep) -> Vec<StartupRequest> {
            match step {
                StartupStep::Fetch(requests) => requests.into(),
                other => panic!("expected a fetch, got {other:?}"),
            }
        }

        #[rstest]
        #[case::unknown(None, None)]
        #[case::first_epochs(Some(1), Some((3, 2)))]
        #[case::at_first_plus_two(Some(3), Some((3, 4)))]
        #[case::below_first_plus_two(Some(5), Some((3, 6)))]
        #[case::large(Some(100), Some((97, 101)))]
        fn progress_epoch_window_bounds_ok(
            #[case] current: Option<u64>,
            #[case] expected: Option<(u64, u64)>,
        ) {
            let progress = StartupProgress {
                cliquenet_current_epoch: current.map(EpochNumber::new),
                ..progress(1)
            };
            let window = progress
                .epoch_window()
                .map(|window| (**window.start(), **window.end()));
            assert_eq!(window, expected);
        }

        #[test]
        fn progress_next_step_batches_independent_ok() {
            let progress = StartupProgress {
                anchor_height: Some(901),
                cliquenet_current_epoch: Some(EpochNumber::new(10)),
                cliquenet_current_height: Some(1000),
                ..progress(10)
            };

            let requests = fetched(progress.next_step());

            let proofs = |epoch: u64| StartupRequest::RootHeaderProof {
                epoch: EpochNumber::new(epoch),
                anchor_height: 901,
                root_height: root_block_in_epoch(epoch - 2, 100),
            };
            let drb = |epoch: u64| StartupRequest::DrbLeaf {
                epoch: EpochNumber::new(epoch),
                height: transition_block_for_epoch(epoch - 1, 100),
            };
            assert_eq!(
                requests,
                [7, 8, 9, 10, 11]
                    .map(proofs)
                    .into_iter()
                    .chain([drb(10), drb(11)])
                    .collect::<Vec<_>>()
            );
        }

        #[test]
        fn next_drb_waits_for_the_transition_leaf() {
            let progress = StartupProgress {
                anchor_height: Some(901),
                held_tables: epochs([1, 2, 7, 8, 9, 10, 11]),
                cliquenet_current_epoch: Some(EpochNumber::new(10)),
                cliquenet_current_height: Some(950),
                ..progress(10)
            };

            let requests = fetched(progress.next_step());

            assert_eq!(
                requests,
                [StartupRequest::DrbLeaf {
                    epoch: EpochNumber::new(10),
                    height: 897
                }]
            );
        }

        #[test]
        fn done_when_window_and_drbs_are_held() {
            let progress = StartupProgress {
                anchor_height: Some(901),
                held_tables: epochs([1, 2, 7, 8, 9, 10, 11]),
                held_drbs: epochs([1, 2, 10]),
                cliquenet_current_epoch: Some(EpochNumber::new(10)),
                cliquenet_current_height: Some(950),
                ..progress(10)
            };

            assert!(matches!(
                progress.next_step(),
                StartupStep::Done {
                    cliquenet_current_epoch
                } if *cliquenet_current_epoch == 10
            ));
        }

        #[test]
        fn wait_for_voting_epoch_ok() {
            // The LC contract is at the root of epoch 9, so only claims of epoch 9 exist.
            let waiting = StartupProgress {
                cliquenet_current_epoch: Some(EpochNumber::new(9)),
                ..progress(10)
            };
            assert!(matches!(
                waiting.next_step(),
                StartupStep::ObserveCertificates
            ));

            let entered = StartupProgress {
                cliquenet_current_epoch: Some(EpochNumber::new(10)),
                ..progress(10)
            };
            assert_eq!(
                fetched(entered.next_step()),
                [StartupRequest::VerifiedLeaf {
                    epoch: EpochNumber::new(10),
                    height: 901
                }]
            );
        }

        #[test]
        fn forged_high_claim_ok() {
            let mut progress = StartupProgress {
                anchor_height: Some(901),
                cliquenet_current_epoch: Some(EpochNumber::new(10)),
                highest_unverified_claim: Some(EpochNumber::new(1000)),
                held_tables: epochs([1, 2, 7, 8, 9, 10]),
                ..progress(10)
            };

            // Only walk steps, never a window batch: first the table of X+1, then its leaf.
            assert_eq!(
                fetched(progress.next_step()),
                [StartupRequest::RootHeaderProof {
                    epoch: EpochNumber::new(11),
                    anchor_height: 901,
                    root_height: root_block_in_epoch(9, 100)
                }]
            );
            progress.held_tables.insert(EpochNumber::new(11));
            assert_eq!(
                fetched(progress.next_step()),
                [StartupRequest::VerifiedLeaf {
                    epoch: EpochNumber::new(11),
                    height: 1001
                }]
            );
        }

        #[test]
        fn prover_lag_walks_epoch_by_epoch() {
            // `E' = 3`, certificates of epoch 7: the walk alternates a table and a leaf.
            let mut progress = StartupProgress {
                anchor_height: Some(201),
                highest_unverified_claim: Some(EpochNumber::new(7)),
                ..progress(3)
            };
            for epoch in 4..=7u64 {
                let [StartupRequest::RootHeaderProof { epoch: e, .. }] =
                    fetched(progress.next_step())[..]
                else {
                    panic!("expected a table of epoch {epoch}");
                };
                assert_eq!(*e, epoch);
                progress.held_tables.insert(e);
                let [StartupRequest::VerifiedLeaf { height, .. }] =
                    fetched(progress.next_step())[..]
                else {
                    panic!("expected the leaf of epoch {epoch}");
                };
                progress.anchor_height = Some(height);
            }
            // The certificate of epoch 7 verifies under the table of epoch 7.
            progress.cliquenet_current_epoch = Some(EpochNumber::new(7));
            progress.highest_unverified_claim = None;
            assert!(matches!(progress.next_step(), StartupStep::Fetch(_)));
        }

        #[rstest]
        // genesis: first epoch tables only
        #[case::genesis(1, 1, 1, &[1, 2], true)]
        #[case::equal_to_voting(10, 10, 1, &[9, 10, 11], true)]
        #[case::one_behind(9, 10, 1, &[8, 9, 10], true)]
        #[case::two_behind(8, 10, 1, &[7, 8, 9], false)]
        #[case::missing_next(9, 10, 1, &[8, 9], false)]
        #[case::below_first_is_not_needed(2, 2, 2, &[2, 3], true)]
        fn choose_path(
            #[case] anchor: u64,
            #[case] voting: u64,
            #[case] first: u64,
            #[case] held: &[u64],
            #[case] from_storage: bool,
        ) {
            let path = StartupPath::choose(
                EpochNumber::new(anchor),
                EpochNumber::new(voting),
                EpochNumber::new(first),
                &epochs(held.iter().copied()),
            );
            let expected = if from_storage {
                StartupPath::FromStorage {
                    anchor_epoch: EpochNumber::new(anchor),
                }
            } else {
                StartupPath::Fill
            };
            assert_eq!(path, expected);
        }

        fn x25519_info(port: u16) -> PeerConnectInfo {
            PeerConnectInfo {
                x25519_key: x25519::Keypair::generate().unwrap().public_key(),
                p2p_addr: NetAddr::from((Ipv4Addr::LOCALHOST, port)),
            }
        }

        fn peer(index: u64, info: Option<PeerConnectInfo>) -> (PubKey, PeerConfig<SeqTypes>) {
            let mut config = ValidatorConfig::<SeqTypes>::generated_from_seed_indexed(
                [7; 32],
                index,
                U256::from(10),
                true,
            )
            .public_config();
            config.connect_info = info;
            (config.stake_table_entry.stake_key, config)
        }

        fn validator(
            key: PubKey,
            info: &PeerConnectInfo,
        ) -> (Address, AuthenticatedValidator<PubKey>) {
            let mut registered = RegisteredValidator::mock();
            registered.stake_table_key = Some(key);
            registered.x25519_key = Some(info.x25519_key);
            registered.p2p_addr = Some(info.p2p_addr.clone());
            (
                registered.account,
                registered.try_into().expect("authenticated"),
            )
        }

        #[test]
        fn bootstrap_peers_union_ok() {
            let (key_now, _) = peer(0, None);
            let (key_both, config_both) = peer(1, Some(x25519_info(1001)));
            let (key_voting, config_voting) = peer(2, Some(x25519_info(1002)));
            let (key_unreachable, config_unreachable) = peer(3, None);
            let now_info = x25519_info(2000);
            let both_now_info = x25519_info(2001);
            let now: AuthenticatedValidatorMap = [
                validator(key_now, &now_info),
                validator(key_both, &both_now_info),
            ]
            .into_iter()
            .collect();
            let voting: IndexMap<_, _> = [
                (key_both, config_both),
                (key_voting, config_voting.clone()),
                (key_unreachable, config_unreachable),
            ]
            .into_iter()
            .collect();

            let peers = BootstrapPeers::new(&now, &voting);

            let by_key: BTreeMap<_, _> = peers.iter().map(|(k, v)| (*k, v.clone())).collect();
            // Union of `ST(now)` and `ST(E')`; keys without connect info are left out.
            assert_eq!(
                by_key.keys().copied().collect::<BTreeSet<_>>(),
                BTreeSet::from([key_now, key_both, key_voting])
            );
            // Connect info comes from `ST(now)` first, else from `ST(E')`.
            assert_eq!(by_key[&key_both], both_now_info);
            assert_eq!(
                Some(&by_key[&key_voting]),
                config_voting.connect_info.as_ref()
            );
        }

        fn registry_of(key: PubKey) -> RegisteredValidatorMap {
            let mut registered = RegisteredValidator::mock();
            registered.stake_table_key = Some(key);
            [(registered.account, registered)].into_iter().collect()
        }

        #[rstest]
        #[case::in_peers(true, true, OwnKeyStatus::InBootstrapPeers)]
        #[case::exited_key_in_candidate_window_fills_ok(
            true,
            false,
            OwnKeyStatus::InBootstrapPeers
        )]
        #[case::inactive_key_waits_ok(false, true, OwnKeyStatus::RegisteredNotInPeers)]
        #[case::unregistered_key_fails(false, false, OwnKeyStatus::NotRegistered)]
        fn own_key_status(
            #[case] in_peers: bool,
            #[case] registered: bool,
            #[case] expected: OwnKeyStatus,
        ) {
            let (own, config) = peer(0, Some(x25519_info(1000)));
            let (other, _) = peer(1, None);
            let voting: IndexMap<_, _> = if in_peers {
                [(own, config)].into_iter().collect()
            } else {
                IndexMap::new()
            };
            let peers = BootstrapPeers::new(&AuthenticatedValidatorMap::default(), &voting);
            let registry = if registered {
                registry_of(own)
            } else {
                registry_of(other)
            };

            assert_eq!(peers.own_key_status(&own, &registry), expected);
        }

        /// A chain of `leaves` blocks with the header proofs of its block Merkle tree.
        async fn chain(leaves: usize) -> TestClient {
            let client = TestClient::default();
            client.leaf(leaves).await;
            client
        }

        #[tokio::test]
        async fn header_proof_verifies_against_the_anchor() {
            let client = chain(10).await;
            let anchor = VerifiedAnchorLeaf::from_verified(client.leaf(10).await.leaf().clone());
            let proof = client.header_proof(10, BlockId::Number(3)).await.unwrap();

            let header = anchor.verify_header_proof(proof, 3).unwrap();

            assert_eq!(header.height(), 3);
        }

        #[tokio::test]
        async fn header_proof_wrong_height_fails() {
            let client = chain(10).await;
            let anchor = VerifiedAnchorLeaf::from_verified(client.leaf(10).await.leaf().clone());
            // A valid proof of another height than the requested one.
            let proof = client.header_proof(10, BlockId::Number(4)).await.unwrap();

            let err = anchor.verify_header_proof(proof, 3).unwrap_err();

            assert!(err.to_string().contains("expected 3"), "{err:#}");
        }

        #[tokio::test]
        async fn header_proof_wrong_root_fails() {
            let client = chain(10).await;
            let anchor = VerifiedAnchorLeaf::from_verified(client.leaf(10).await.leaf().clone());
            // A proof against the snapshot of another root.
            let proof = client.header_proof(9, BlockId::Number(3)).await.unwrap();

            anchor.verify_header_proof(proof, 3).unwrap_err();
        }
    }

    mod fakes {
        use super::*;

        type Responder = Box<dyn FnMut(&StartupRequest) -> anyhow::Result<StartupResponse> + Send>;

        /// L1 with a fixed LC contract read; each stake table read pops the next event list and
        /// keeps the last one, each finality tick the next block.
        struct FakeL1 {
            lc_contract: LcContractVotingStakeTable,
            events: Mutex<VecDeque<Vec<(EventKey, StakeTableEvent)>>>,
            ticks: Mutex<u64>,
        }

        impl FakeL1 {
            fn new(
                lc_contract: LcContractVotingStakeTable,
                events: Vec<Vec<(EventKey, StakeTableEvent)>>,
            ) -> Self {
                Self {
                    lc_contract,
                    events: Mutex::new(events.into()),
                    ticks: Mutex::new(0),
                }
            }
        }

        #[async_trait]
        impl StartupL1Reader for FakeL1 {
            async fn l1_finalized_block(&self) -> anyhow::Result<u64> {
                Ok(self.lc_contract.l1_block)
            }

            async fn lc_contract_voting_stake_table(
                &self,
                l1_block: u64,
            ) -> anyhow::Result<LcContractVotingStakeTable> {
                Ok(LcContractVotingStakeTable {
                    l1_block,
                    ..self.lc_contract.clone()
                })
            }

            async fn stake_table_events(
                &self,
                _to_l1_block: u64,
            ) -> anyhow::Result<Vec<(EventKey, StakeTableEvent)>> {
                let mut events = self.events.lock().unwrap();
                Ok(if events.len() > 1 {
                    events.pop_front().unwrap()
                } else {
                    events.front().cloned().unwrap_or_default()
                })
            }

            async fn next_l1_finalized_block(&self, after: u64) -> anyhow::Result<u64> {
                *self.ticks.lock().unwrap() += 1;
                Ok(after + 1)
            }
        }

        /// A cliquenet answering requests with `responder` and replaying `certificates`.
        struct FakeCliquenet {
            dialed: Option<Vec<PubKey>>,
            inbound: usize,
            certificates: VecDeque<ObservedCertificate>,
            responder: Responder,
            batches: Vec<usize>,
        }

        impl FakeCliquenet {
            fn new(responder: Responder) -> Self {
                Self {
                    dialed: None,
                    inbound: 0,
                    certificates: VecDeque::new(),
                    responder,
                    batches: Vec::new(),
                }
            }

            fn silent() -> Self {
                Self::new(Box::new(|_| Err(anyhow!("no peer answers"))))
            }
        }

        #[async_trait]
        impl StartupCliquenet for FakeCliquenet {
            fn dial(&mut self, peers: &BootstrapPeers) -> anyhow::Result<()> {
                self.dialed = Some(peers.iter().map(|(key, _)| *key).collect());
                Ok(())
            }

            fn accepted_inbound_connections(&self) -> usize {
                self.inbound
            }

            async fn fetch_concurrently(
                &mut self,
                requests: &Vec1<StartupRequest>,
                _wait: Duration,
            ) -> Vec<anyhow::Result<StartupResponse>> {
                self.batches.push(requests.len());
                requests
                    .iter()
                    .map(|request| (self.responder)(request))
                    .collect()
            }

            async fn next_observed_certificate(
                &mut self,
                _wait: Duration,
            ) -> Option<ObservedCertificate> {
                self.certificates.pop_front()
            }
        }

        /// The genesis committee seeded in the membership, with its private keys.
        struct Committee {
            configs: Vec<ValidatorConfig<SeqTypes>>,
        }

        impl Committee {
            fn new(size: u64) -> Self {
                let configs = (0..size)
                    .map(|index| {
                        let mut config = ValidatorConfig::<SeqTypes>::generated_from_seed_indexed(
                            [42; 32],
                            index,
                            U256::from(100),
                            true,
                        );
                        config.x25519_keypair = Some(x25519::Keypair::generate().unwrap());
                        config.p2p_addr =
                            Some(NetAddr::from((Ipv4Addr::LOCALHOST, 9000 + index as u16)));
                        config
                    })
                    .collect();
                Self { configs }
            }

            fn peers(&self) -> Vec<PeerConfig<SeqTypes>> {
                self.configs.iter().map(|c| c.public_config()).collect()
            }

            fn signers(&self) -> Vec<(PubKey, BLSPrivKey)> {
                self.configs
                    .iter()
                    .map(|c| (c.public_key, c.private_key.clone()))
                    .collect()
            }

            fn own_key(&self) -> PubKey {
                self.configs[0].public_key
            }

            fn commitment(&self) -> StakeTableCommitment {
                HSStakeTable::<SeqTypes>::from(self.peers())
                    .commitment(CAPACITY)
                    .unwrap()
            }

            /// A membership seeded with this committee at `first_epoch` and `first_epoch + 1`.
            fn membership(&self, first_epoch: u64) -> EpochMembershipCoordinator<SeqTypes> {
                let peers = self.peers();
                let committees = EpochCommittees::new_stake(
                    peers.clone(),
                    peers,
                    None,
                    Fetcher::mock(),
                    EPOCH_HEIGHT,
                );
                let coordinator = EpochMembershipCoordinator::new(
                    committees,
                    EPOCH_HEIGHT,
                    &TestStorage::<SeqTypes>::default(),
                );
                coordinator
                    .membership()
                    .set_first_epoch(EpochNumber::new(first_epoch), [0; 32]);
                coordinator
            }

            fn cert1(&self, block: u64, lock: &UpgradeLock<SeqTypes>) -> ObservedCertificate {
                let epoch = epoch_from_block_number(block, EPOCH_HEIGHT);
                let data = QuorumData2 {
                    leaf_commit: Commitment::from_raw([5; 32]),
                    epoch: Some(EpochNumber::new(epoch)),
                    block_number: Some(block),
                };
                ObservedCertificate::Cert1(sign_certificate::<SeqTypes, _, Certificate1<SeqTypes>>(
                    data,
                    ViewNumber::new(block),
                    &self.signers(),
                    lock,
                ))
            }

            /// Leaves `from..=to` of a generated chain, signed by this committee.
            async fn leaf_chain(
                &self,
                from: usize,
                to: usize,
                lock: &UpgradeLock<SeqTypes>,
            ) -> StartupResponse {
                let client = TestClient::with_epoch_height(EPOCH_HEIGHT);
                client.leaf(to).await;
                let mut leaves = vec![];
                for height in from..=to {
                    leaves.push(client.leaf(height).await.leaf().clone());
                }
                StartupResponse::LeafChain(Box::new(sign_leaf_chain(
                    leaves,
                    EPOCH_HEIGHT,
                    &self.signers(),
                    lock,
                )))
            }
        }

        fn lock() -> UpgradeLock<SeqTypes> {
            UpgradeLock::new(Upgrade::trivial(NEW_PROTOCOL_VERSION))
        }

        fn params(own_key: PubKey, anchor_epoch: u64) -> StartupCatchupParams {
            StartupCatchupParams {
                own_key,
                stake_table_capacity: CAPACITY,
                candidate_versions: vec![NEW_PROTOCOL_VERSION],
                step_timeout: Duration::from_secs(1),
                anchor_epoch: EpochNumber::new(anchor_epoch),
                deadline: Duration::from_millis(200),
            }
        }

        fn voting(epoch: u64, commitment: StakeTableCommitment) -> LcContractVotingStakeTable {
            LcContractVotingStakeTable {
                l1_block: 100,
                finalized_block_height: 0,
                voting_epoch: EpochNumber::new(epoch),
                commitment,
            }
        }

        #[tokio::test(start_paused = true)]
        async fn recent_storage_starts_without_fill_ok() {
            let committee = Committee::new(4);
            let coordinator = committee.membership(1);
            let l1 = FakeL1::new(voting(1, committee.commitment()), vec![vec![]]);
            let mut net = FakeCliquenet::silent();

            let epoch = catch_up_over_cliquenet(
                &l1,
                &mut net,
                &coordinator,
                &params(committee.own_key(), 1),
            )
            .await
            .unwrap();

            assert_eq!(epoch, EpochNumber::new(1));
            assert!(net.dialed.is_none());
            assert!(net.batches.is_empty());
        }

        #[tokio::test(start_paused = true)]
        async fn stale_storage_runs_fill_ok() {
            // Anchor epoch 1 is below `E' - 1 = 2`: the fill runs and dials the committee.
            let committee = Committee::new(4);
            let coordinator = committee.membership(3);
            let l1 = FakeL1::new(voting(3, committee.commitment()), vec![vec![]]);
            let mut net = FakeCliquenet::silent();

            let err = catch_up_over_cliquenet(
                &l1,
                &mut net,
                &coordinator,
                &params(committee.own_key(), 1),
            )
            .await
            .unwrap_err();

            assert_eq!(net.dialed.unwrap().len(), 4);
            assert!(matches!(
                err,
                StartupCatchupError::DeadlineExceeded {
                    stage: StartupStage::ObserveCertificates,
                    ..
                }
            ));
        }

        /// The fill of the genesis epoch: the seeded table is the candidate, a certificate of
        /// epoch 3 fixes `C`, one leaf chain anchors, and the window is empty.
        async fn genesis_fill(
            committee: &Committee,
            net: &mut FakeCliquenet,
        ) -> Result<EpochNumber, StartupCatchupError> {
            let coordinator = committee.membership(3);
            let l1 = FakeL1::new(voting(3, committee.commitment()), vec![vec![]]);
            let chain = committee.leaf_chain(21, 23, &lock()).await;
            net.responder = Box::new(move |request| match request {
                StartupRequest::VerifiedLeaf { height: 21, .. } => Ok(chain.clone()),
                other => Err(anyhow!("unexpected request {other:?}")),
            });
            net.certificates.push_back(committee.cert1(25, &lock()));
            catch_up_over_cliquenet(&l1, net, &coordinator, &params(committee.own_key(), 1)).await
        }

        #[tokio::test(start_paused = true)]
        async fn candidate_table_verifies_leaf_ok() {
            let committee = Committee::new(4);
            let mut net = FakeCliquenet::silent();

            let epoch = genesis_fill(&committee, &mut net).await.unwrap();

            // C is the epoch of the verified certificate; one leaf request, no tables to fetch.
            assert_eq!(epoch, EpochNumber::new(3));
            assert_eq!(net.batches, [1]);
            assert_eq!(net.dialed.unwrap().len(), 4);
        }

        #[tokio::test(start_paused = true)]
        async fn old_peer_no_answer_ok() {
            // The first peers do not answer, a retry succeeds.
            let committee = Committee::new(4);
            let mut net = FakeCliquenet::silent();
            let coordinator = committee.membership(3);
            let l1 = FakeL1::new(voting(3, committee.commitment()), vec![vec![]]);
            let chain = committee.leaf_chain(21, 23, &lock()).await;
            let mut calls = 0;
            net.responder = Box::new(move |_| {
                calls += 1;
                if calls < 3 {
                    Err(anyhow!("peer does not decode the request"))
                } else {
                    Ok(chain.clone())
                }
            });
            net.certificates.push_back(committee.cert1(25, &lock()));

            let mut params = params(committee.own_key(), 1);
            params.deadline = Duration::from_secs(60);
            let epoch = catch_up_over_cliquenet(&l1, &mut net, &coordinator, &params)
                .await
                .unwrap();

            assert_eq!(epoch, EpochNumber::new(3));
            assert_eq!(net.batches, [1, 1, 1]);
        }

        #[tokio::test(start_paused = true)]
        async fn leaf_chain_of_another_committee_is_not_accepted() {
            let committee = Committee::new(4);
            let impostor = Committee {
                configs: Committee::new(4)
                    .configs
                    .into_iter()
                    .enumerate()
                    .map(|(index, _)| {
                        ValidatorConfig::<SeqTypes>::generated_from_seed_indexed(
                            [43; 32],
                            index as u64,
                            U256::from(100),
                            true,
                        )
                    })
                    .collect(),
            };
            let coordinator = committee.membership(3);
            let l1 = FakeL1::new(voting(3, committee.commitment()), vec![vec![]]);
            let forged = impostor.leaf_chain(21, 23, &lock()).await;
            let mut net = FakeCliquenet::new(Box::new(move |_| Ok(forged.clone())));
            net.certificates.push_back(committee.cert1(25, &lock()));

            let err = catch_up_over_cliquenet(
                &l1,
                &mut net,
                &coordinator,
                &params(committee.own_key(), 1),
            )
            .await
            .unwrap_err();

            assert!(matches!(
                err,
                StartupCatchupError::DeadlineExceeded {
                    stage: StartupStage::FetchVerifiedLeaf { .. },
                    ..
                }
            ));
        }

        /// Captures the log output of the test's thread.
        #[derive(Clone, Default)]
        struct LogBuffer(Arc<Mutex<Vec<u8>>>);

        impl std::io::Write for LogBuffer {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().write(buf)
            }

            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        impl<'a> MakeWriter<'a> for LogBuffer {
            type Writer = LogBuffer;

            fn make_writer(&'a self) -> Self::Writer {
                self.clone()
            }
        }

        #[tokio::test(start_paused = true)]
        async fn no_inbound_warns_ok() {
            let logs = LogBuffer::default();
            let subscriber = tracing_subscriber::fmt()
                .with_writer(logs.clone())
                .with_ansi(false)
                .finish();
            let _guard = tracing::subscriber::set_default(subscriber);
            let committee = Committee::new(4);
            let mut net = FakeCliquenet::silent();

            genesis_fill(&committee, &mut net).await.unwrap();

            let output = String::from_utf8(logs.0.lock().unwrap().clone()).unwrap();
            assert!(
                output.contains("no inbound cliquenet connections; check firewall"),
                "{output}"
            );
        }

        #[tokio::test(start_paused = true)]
        async fn inbound_connection_silences_the_warning() {
            let logs = LogBuffer::default();
            let subscriber = tracing_subscriber::fmt()
                .with_writer(logs.clone())
                .with_ansi(false)
                .finish();
            let _guard = tracing::subscriber::set_default(subscriber);
            let committee = Committee::new(4);
            let mut net = FakeCliquenet::silent();
            net.inbound = 2;

            genesis_fill(&committee, &mut net).await.unwrap();

            let output = String::from_utf8(logs.0.lock().unwrap().clone()).unwrap();
            assert!(!output.contains("no inbound cliquenet"), "{output}");
        }

        #[tokio::test(start_paused = true)]
        async fn deadline_names_the_stage() {
            // No certificate ever arrives.
            let committee = Committee::new(4);
            let coordinator = committee.membership(3);
            let l1 = FakeL1::new(voting(3, committee.commitment()), vec![vec![]]);
            let mut net = FakeCliquenet::silent();

            let err = catch_up_over_cliquenet(
                &l1,
                &mut net,
                &coordinator,
                &params(committee.own_key(), 1),
            )
            .await
            .unwrap_err();

            let StartupCatchupError::DeadlineExceeded {
                stage, attempts, ..
            } = err
            else {
                panic!("unexpected error {err:#}");
            };
            assert_eq!(stage, StartupStage::ObserveCertificates);
            assert!(attempts >= 1);
        }

        /// Registers `account` with the keys of `peer` (an eligible validator) and gives it stake.
        fn registration_events(validator: &TestValidator, stake: u64) -> Vec<StakeTableEvent> {
            vec![
                ValidatorRegisteredV3::from(validator).into(),
                Delegated {
                    delegator: Address::random(),
                    validator: validator.account,
                    amount: U256::from(stake),
                }
                .into(),
            ]
        }

        fn keyed(events: Vec<StakeTableEvent>, l1_block: u64) -> Vec<(EventKey, StakeTableEvent)> {
            events
                .into_iter()
                .enumerate()
                .map(|(index, event)| ((l1_block, index as u64), event))
                .collect()
        }

        fn commitment_of_events(events: &[(EventKey, StakeTableEvent)]) -> StakeTableCommitment {
            let set = ValidatorSet::from_l1_events(
                events.iter().map(|(_, event)| event.clone()),
                NEW_PROTOCOL_VERSION,
            )
            .unwrap();
            let configs: Vec<PeerConfig<SeqTypes>> = set
                .active_validators()
                .values()
                .map(|v| PeerConfig {
                    stake_table_entry: PubKey::stake_table_entry(v.stake_table_key(), v.stake),
                    state_ver_key: v.state_ver_key().clone(),
                    connect_info: None,
                })
                .collect();
            HSStakeTable::<SeqTypes>::from(configs)
                .commitment(CAPACITY)
                .unwrap()
        }

        fn validator_key(validator: &TestValidator) -> PubKey {
            PubKey::try_from(validator.bls_vk).unwrap()
        }

        /// Events of three eligible validators and `E' = 5` whose candidate they are.
        fn candidate_fixture() -> (Vec<TestValidator>, Vec<(EventKey, StakeTableEvent)>) {
            let validators: Vec<_> = (0..3).map(|_| TestValidator::random()).collect();
            let events = keyed(
                validators
                    .iter()
                    .flat_map(|validator| registration_events(validator, 10))
                    .collect(),
                50,
            );
            (validators, events)
        }

        #[tokio::test(start_paused = true)]
        async fn no_match_fails() {
            let committee = Committee::new(4);
            let (validators, events) = candidate_fixture();
            let coordinator = committee.membership(1);
            // The LC contract commits to another table than any the events give.
            let l1 = FakeL1::new(voting(5, committee.commitment()), vec![events]);
            let mut net = FakeCliquenet::silent();

            let err = catch_up_over_cliquenet(
                &l1,
                &mut net,
                &coordinator,
                &params(validator_key(&validators[0]), 1),
            )
            .await
            .unwrap_err();

            assert!(
                matches!(
                    err,
                    StartupCatchupError::LcContractStakeTableMatch(
                        LcContractStakeTableMatchError::NoLcContractCommitmentMatch { .. }
                    )
                ),
                "{err:#}"
            );
            assert!(net.dialed.is_none());
        }

        #[tokio::test(start_paused = true)]
        async fn unregistered_key_fails() {
            let committee = Committee::new(4);
            let (_, events) = candidate_fixture();
            let stranger = TestValidator::random();
            let coordinator = committee.membership(1);
            let l1 = FakeL1::new(voting(5, commitment_of_events(&events)), vec![events]);
            let mut net = FakeCliquenet::silent();

            let err = catch_up_over_cliquenet(
                &l1,
                &mut net,
                &coordinator,
                &params(validator_key(&stranger), 1),
            )
            .await
            .unwrap_err();

            assert!(
                matches!(err, StartupCatchupError::OwnKeyNotRegistered { .. }),
                "{err:#}"
            );
            assert!(net.dialed.is_none());
        }

        #[tokio::test(start_paused = true)]
        async fn inactive_key_waits_ok() {
            // The own key is registered without stake, so it is not in `P` until it is delegated to.
            let committee = Committee::new(4);
            let (validators, events) = candidate_fixture();
            let own = TestValidator::random();
            let mut waiting = events.clone();
            waiting.extend(keyed(vec![ValidatorRegisteredV3::from(&own).into()], 60));
            let mut active = waiting.clone();
            active.extend(keyed(
                vec![
                    Delegated {
                        delegator: Address::random(),
                        validator: own.account,
                        amount: U256::from(10),
                    }
                    .into(),
                ],
                70,
            ));
            let coordinator = committee.membership(1);
            let l1 = FakeL1::new(
                voting(5, commitment_of_events(&events)),
                vec![waiting, active],
            );
            let mut net = FakeCliquenet::silent();
            let _ = validators;

            let err = catch_up_over_cliquenet(
                &l1,
                &mut net,
                &coordinator,
                &params(validator_key(&own), 1),
            )
            .await
            .unwrap_err();

            // One finality tick later the key is in `P`: the fill dials and runs out of time.
            assert_eq!(*l1.ticks.lock().unwrap(), 1);
            assert!(net.dialed.is_some());
            assert!(matches!(
                err,
                StartupCatchupError::DeadlineExceeded {
                    stage: StartupStage::ObserveCertificates,
                    ..
                }
            ));
        }

        #[tokio::test(start_paused = true)]
        async fn exited_key_in_candidate_window_fills_ok() {
            // The own validator is in `ST(E')` but exited before `b_fin`: no longer registered, yet
            // a bootstrap peer through the candidate.
            let committee = Committee::new(4);
            let (validators, events) = candidate_fixture();
            let mut all = events.clone();
            all.extend(keyed(vec![ValidatorExit::from(&validators[0]).into()], 90));
            let coordinator = committee.membership(1);
            let l1 = FakeL1::new(voting(5, commitment_of_events(&events)), vec![all]);
            let mut net = FakeCliquenet::silent();

            let err = catch_up_over_cliquenet(
                &l1,
                &mut net,
                &coordinator,
                &params(validator_key(&validators[0]), 1),
            )
            .await
            .unwrap_err();

            assert!(net.dialed.is_some());
            assert!(
                matches!(err, StartupCatchupError::DeadlineExceeded { .. }),
                "{err:#}"
            );
        }
    }
}
