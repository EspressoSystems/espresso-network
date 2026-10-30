//! Startup stake-table catchup for new-protocol nodes.
//!
//! [`catch_up_over_cliquenet`] is the validator boot. [`bootstrap_epoch_window`] is the follower
//! startup: cliquenet only connects to validators in the current epoch's window (`N-1`, `N`,
//! `N+1`) and a follower holds no stake, so it drives the existing catchup machinery synchronously,
//! one epoch at a time from the highest epoch loaded by `reload_stake`, until peers can no longer
//! serve the next epoch root leaf.

use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    mem,
    ops::RangeInclusive,
    time::Duration,
};

use anyhow::{Context, anyhow, ensure};
use async_trait::async_trait;
use espresso_types::{
    AuthenticatedValidatorMap, Header, LcContractCandidateStakeTable,
    LcContractStakeTableMatchError, LcContractVotingStakeTable, NodeState, PubKey,
    RegisteredValidatorMap, SeqTypes, ValidatorSet, stake_table_state_at,
    v0_3::{EventKey, StakeTableError, StakeTableEvent},
};
use hotshot_new_protocol::{cert_verifier::Verifiable, utils::CertifiedLeafChain};
use hotshot_types::{
    PeerConfig, PeerConnectInfo,
    data::{EpochNumber, Leaf2, ViewNumber},
    drb::DrbResult,
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

/// All startup steps together; the node exits at expiry. The wait for the own key to enter the LC
/// contract's stake table does not count.
pub(crate) const STARTUP_CATCHUP_DEADLINE: Duration = Duration::from_secs(15 * 60);

/// Bootstrap peers asked per fill request.
pub(crate) const STARTUP_FAN_OUT: usize = 5;

/// First wait after a failed attempt of a stage; doubles up to `RETRY_BACKOFF_MAX`.
const RETRY_BACKOFF_INITIAL: Duration = Duration::from_secs(1);

const RETRY_BACKOFF_MAX: Duration = Duration::from_secs(60);

/// Unverifiable certificates kept to re-check after the next table is installed.
const PENDING_CERTIFICATES_LIMIT: usize = 64;

/// Interval of the warning while the own key waits to enter the LC contract's stake table.
const OWN_KEY_WAIT_HEARTBEAT: Duration = Duration::from_secs(30 * 60);

/// Target of the startup log lines.
const LOG_TARGET: &str = "announce::startup_catchup";

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

/// Bounds and identity of one startup catchup.
#[derive(Clone, Debug)]
pub struct StartupCatchupParams {
    /// The node's own BLS key, checked against the LC contract's stake table and the L1 registry.
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
    /// The node's restored upgrade lock, which versions the votes of the certificates checked.
    pub upgrade_lock: UpgradeLock<SeqTypes>,
    /// L1 phase and fill together, [`STARTUP_CATCHUP_DEADLINE`] in production; exit at expiry.
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

/// Validator boot: LC contract read, then `StartupPath::choose`: from storage
/// (`apply_epoch(anchor_epoch)` by the caller, returns `anchor_epoch`) or the fill (dial `P`, fill
/// the window over cliquenet, returns `C`).
///
/// - Candidates: `ST(now)` and the candidate `ST(E')` from L1 events only dial and verify the anchor
///   leaf; exact tables come from `add_epoch_root` on proven root headers. For `E' <= first + 1` the
///   seeded genesis table is the candidate.
/// - Own key: it must be in `ST(E')`. Registered but not in it, the L1 phase re-runs on each L1
///   finality tick without deadline and with WARN, heartbeat and INFO logs; not registered exits.
/// - Network: `net` borrows the node's instance before the coordinator owns it. The fill never
///   calls `client_api()`, `apply_epoch` or sets `shared.epoch`, so the caller's `apply_epoch(C)`
///   installs the window and drops candidate peers. Consumed messages never reach the coordinator;
///   inbound requests are ignored.
/// - Membership reads never start catchup (`snapshot`, never `stake_table_for_epoch`).
/// - Recent storage skips the fill. Gap: prover lag plus long downtime with `anchor_epoch >= E' - 1`
///   but `C > anchor_epoch + 1` starts at `anchor_epoch` and relies on post-start catchup.
///
/// Exits: see [`StartupCatchupError`]. There is no HTTP fallback.
pub(crate) async fn catch_up_over_cliquenet(
    l1: &impl StartupL1Reader,
    net: &mut impl StartupCliquenet,
    membership: &impl StartupMembership,
    params: &StartupCatchupParams,
) -> Result<EpochNumber, StartupCatchupError> {
    let started = Instant::now();
    let mut stats = StartupStats::default();
    let mut retry = Retry::new(params.deadline);
    let first_epoch = membership.first_epoch();
    let phase = L1Phase {
        l1,
        membership,
        params,
        first_epoch,
    };
    let Some(plan) = phase.run(&mut retry, &mut stats).await? else {
        return Ok(params.anchor_epoch);
    };

    tracing::info!(
        target: LOG_TARGET,
        count = plan.peers.iter().count(),
        own_key_included = plan.peers.contains(&params.own_key),
        "bootstrap peers"
    );
    net.dial(&plan.peers).map_err(StartupCatchupError::Dial)?;
    Fill::new(net, membership, params, plan, &mut stats)
        .run(retry, started)
        .await
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
    /// The next L1 finalized block above `after`; the wait while the own key is not in the LC
    /// contract's stake table.
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

/// A stake table held in memory, and whether its DRB is.
#[derive(Clone, Debug)]
pub(crate) struct TableSnapshot {
    pub table: HSStakeTable<SeqTypes>,
    pub has_drb: bool,
}

/// The membership the startup fills. Reads never start catchup.
#[async_trait]
pub(crate) trait StartupMembership: Send + Sync {
    /// Blocks per epoch.
    fn epoch_height(&self) -> u64;
    /// First epoch with a stake table of its own.
    fn first_epoch(&self) -> EpochNumber;
    /// Highest epoch with a table in memory.
    fn highest_known_epoch(&self) -> Option<EpochNumber>;
    /// The table of `epoch`, if in memory.
    fn snapshot(&self, epoch: EpochNumber) -> Option<TableSnapshot>;
    /// Installs the table of the epoch whose root `header` is.
    async fn add_epoch_root(&self, header: Header) -> anyhow::Result<()>;
    /// Stores the DRB of `epoch`, whose table is installed.
    fn supply_drb(&self, epoch: EpochNumber, drb: DrbResult);
}

#[async_trait]
impl StartupMembership for EpochMembershipCoordinator<SeqTypes> {
    fn epoch_height(&self) -> u64 {
        *EpochMembershipCoordinator::epoch_height(self)
    }

    fn first_epoch(&self) -> EpochNumber {
        self.membership()
            .first_epoch()
            .expect("seed_membership sets the first epoch")
    }

    fn highest_known_epoch(&self) -> Option<EpochNumber> {
        self.membership().highest_known_epoch()
    }

    fn snapshot(&self, epoch: EpochNumber) -> Option<TableSnapshot> {
        let snapshot = self.membership().snapshot(epoch)?;
        Some(TableSnapshot {
            table: HSStakeTable::from_iter(snapshot.stake_table()),
            has_drb: snapshot.has_drb(),
        })
    }

    async fn add_epoch_root(&self, header: Header) -> anyhow::Result<()> {
        EpochMembershipCoordinator::add_epoch_root(self, header)
            .await
            .map_err(|err| anyhow!("add_epoch_root: {err}"))
    }

    fn supply_drb(&self, epoch: EpochNumber, drb: DrbResult) {
        EpochMembershipCoordinator::supply_drb(self, epoch, drb)
    }
}

/// The node's cliquenet during the fill, before the coordinator owns it. `Send` so the
/// `&mut self` futures `async_trait` boxes as `Send` hold `&mut Self` across awaits.
#[async_trait]
pub(crate) trait StartupCliquenet: Send {
    /// Add `peers` as parties and dial them.
    fn dial(&mut self, peers: &BootstrapPeers) -> anyhow::Result<()>;

    /// Each request fans out to `STARTUP_FAN_OUT` random bootstrap peers, first answer of the right
    /// shape wins, all in flight at once, within `wait` for the whole batch; results in request
    /// order, an unanswered entry is `Err`. `VerifiedLeaf` and `DrbLeaf` fetch `Leaf(height)` then
    /// `Cert2(cert2 height)`; `RootHeaderProof` fetches `HeaderProof`.
    ///
    /// One `receive()` loop routes each response by `RequestHash`, the blake3 of
    /// `Request::to_bytes()`; requests with identical bytes share one entry. `receive()` errors
    /// `InvalidSender` and `Serialize` are skipped; `NetworkError::is_critical()` ends the batch.
    /// Payloads are framed by `frame_external_message` and sent as `MessageType::External` through
    /// `Sender::unicast`.
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
    /// well-formedness check, and rejects the genesis view, whose certificate may be unsigned.
    /// Entries come from `StakeTableEntries::from(table).0` and the threshold from
    /// `supermajority_threshold(table.total_stakes())`.
    pub fn verify(
        &self,
        table: &HSStakeTable<SeqTypes>,
        epoch_height: u64,
        upgrade_lock: &UpgradeLock<SeqTypes>,
    ) -> anyhow::Result<()> {
        ensure!(
            self.view() > ViewNumber::genesis(),
            "certificate of the genesis view"
        );
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

/// Candidate keys dialed during the fill, and the keys of `ST(E')`; `apply_epoch(C)` removes the
/// candidates outside the window.
#[derive(Clone, Debug)]
pub(crate) struct BootstrapPeers {
    peers: BTreeMap<PubKey, PeerConnectInfo>,
    voting_epoch_keys: BTreeSet<PubKey>,
}

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
        Self {
            peers,
            voting_epoch_keys: voting_epoch_peers.keys().copied().collect(),
        }
    }

    /// Where the own key stands: in `ST(E')`, registered at `b_fin` but not in `ST(E')`, or not
    /// registered. Pure.
    ///
    /// Check order: `ST(E')` first, then the L1 validator registry. Peers accept the key only once
    /// it is in their window, so the fill needs it in `ST(E')`. Not registered (never, or exited,
    /// which removes the validator from the registry) exits with `OwnKeyNotRegistered`; registered
    /// but not in `ST(E')` waits for the next L1 finality tick and re-runs the L1 phase.
    pub fn own_key_status(
        &self,
        own_key: &PubKey,
        l1_finalized_registry: &RegisteredValidatorMap,
    ) -> OwnKeyStatus {
        if self.voting_epoch_keys.contains(own_key) {
            OwnKeyStatus::InVotingStakeTable
        } else if l1_finalized_registry
            .values()
            .any(|validator| validator.stake_table_key.as_ref() == Some(own_key))
        {
            OwnKeyStatus::RegisteredNotInVotingStakeTable
        } else {
            OwnKeyStatus::NotRegistered
        }
    }

    /// Whether `key` is one of the bootstrap peers.
    pub fn contains(&self, key: &PubKey) -> bool {
        self.peers.contains_key(key)
    }

    /// Keys with their connect info, for `StartupCliquenet::dial`.
    pub fn iter(&self) -> impl Iterator<Item = (&PubKey, &PeerConnectInfo)> {
        self.peers.iter()
    }
}

/// Own key against `ST(E')` first, then the L1 validator registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OwnKeyStatus {
    /// The key is in `ST(E')`: the fill runs.
    InVotingStakeTable,
    /// The L1 phase waits and re-runs on each L1 finality tick.
    RegisteredNotInVotingStakeTable,
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
    ///   go into one `Fetch` batch once the anchor is held, ascending by epoch.
    /// - Prover lag: while the anchor is below the highest claim and below the verified `C`, the
    ///   batch is `RootHeaderProof` of `X+1` (from `root(X-1)` against the anchor of `X`), then
    ///   `VerifiedLeaf` of `X+1`. Once `C` is verified and the anchor is at it, a higher claim
    ///   no longer walks. No epoch cap; the deadline bounds it.
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
        let behind_current = self
            .cliquenet_current_epoch
            .is_none_or(|current| anchor_epoch < current);
        if anchor_epoch < claim && behind_current {
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

/// Why the startup catchup exits. Every variant exits the node; none is recovered from, and there
/// is no HTTP fallback. The operator fixes the cause and restarts.
#[derive(Debug, Error)]
pub(crate) enum StartupCatchupError {
    /// No L1 block and version reproduce `votingStakeTableState`; our L1 view differs from the LC
    /// contract's. Wraps the matcher's error, whose `NoLcContractCommitmentMatch` variant is the
    /// one operators see.
    #[error(transparent)]
    LcContractStakeTableMatch(#[from] LcContractStakeTableMatchError),
    /// The own BLS key is not in the L1 validator registry at `b_fin` (never registered or exited).
    #[error("own key {key} is not registered at the L1 finalized block")]
    OwnKeyNotRegistered { key: Box<PubKey> },
    /// A held or installed table of `E'` commits to another table than the LC contract. Both
    /// commitments are logged at ERROR first. The walk of a lagging prover does not install `E'`,
    /// so it does not run this check.
    #[error("held committee of epoch {epoch} differs from the LC contract commitment")]
    InstalledCommitmentMismatch { epoch: EpochNumber },
    /// The cliquenet instance refused the bootstrap peers: its network task is down.
    #[error("failed to dial the bootstrap peers")]
    Dial(#[source] anyhow::Error),
    /// A step ran out of time. `stage` names it; a lagging prover with no serving peer and a
    /// committee turnover with no reachable window peer both end here. With no connection
    /// established the source says to check the firewall and `p2p_addr`.
    #[error("startup deadline exceeded at {stage:?} after {attempts} attempts: {source:#}")]
    DeadlineExceeded {
        stage: StartupStage,
        attempts: usize,
        source: anyhow::Error,
    },
}

/// A retried stage, named in `DeadlineExceeded`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StartupStage {
    /// L1 RPC unreachable or a getter failed.
    L1Read,
    /// No certificate from a window peer verified: no window peer reachable, or the network is
    /// stalled.
    ObserveCertificates,
    /// No bootstrap peer served a verifying leaf chain for `epoch`.
    FetchVerifiedLeaf { epoch: EpochNumber },
    /// No bootstrap peer served a header proof of `root(epoch - 2)` that holds against the anchor.
    InstallEpoch { epoch: EpochNumber },
    /// No bootstrap peer served the transition leaf carrying the DRB of `epoch`.
    InstallDrb { epoch: EpochNumber },
}

/// Time, answered and unanswered requests of one kind of stage.
#[derive(Clone, Copy, Debug, Default)]
struct StageStats {
    duration_ms: u64,
    answered: usize,
    unanswered: usize,
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

/// Backoff and the one deadline of the L1 phase and the fill.
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
        tracing::debug!(target: LOG_TARGET, ?stage, attempts = self.attempts, "startup stage failed, retrying");
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
                Ok(value) => {
                    self.reset();
                    return Ok(value);
                },
                Err(err) => self.failed(stage, err).await?,
            }
        }
    }
}

/// What the L1 phase hands to the fill.
struct FillPlan {
    lc_contract: LcContractVotingStakeTable,
    /// The candidate `ST(E')`; `None` when `E' <= first + 1`, where the seeded table is installed.
    candidate: Option<LcContractCandidateStakeTable>,
    peers: BootstrapPeers,
    first_epoch: EpochNumber,
}

/// The own key waiting to enter the LC contract's stake table, and its log lines.
struct OwnKeyWait {
    since: Instant,
    last_heartbeat: Instant,
    voting_epoch: EpochNumber,
}

impl OwnKeyWait {
    /// Starts the wait with the WARN that says why.
    fn enter(key: &PubKey, voting_epoch: EpochNumber, b_fin: u64) -> Self {
        tracing::warn!(
            target: LOG_TARGET,
            %key,
            lc_contract_voting_epoch = %voting_epoch,
            b_fin,
            "own key is not in the LC contract stake table for epoch {voting_epoch}; peers accept \
             this key only once it is in their window; waiting"
        );
        let now = Instant::now();
        Self {
            since: now,
            last_heartbeat: now,
            voting_epoch,
        }
    }

    /// One more L1 finality tick without the key.
    fn tick(&mut self, voting_epoch: EpochNumber, b_fin: u64) {
        tracing::debug!(target: LOG_TARGET, b_fin, "own key still not in the LC contract stake table");
        if voting_epoch != self.voting_epoch {
            tracing::info!(
                target: LOG_TARGET,
                lc_contract_voting_epoch = %voting_epoch,
                "the LC contract stake table advanced without the own key"
            );
            self.voting_epoch = voting_epoch;
        }
        if self.last_heartbeat.elapsed() >= OWN_KEY_WAIT_HEARTBEAT {
            self.last_heartbeat = Instant::now();
            tracing::warn!(
                target: LOG_TARGET,
                waited_ms = self.since.elapsed().as_millis() as u64,
                lc_contract_voting_epoch = %voting_epoch,
                "own key still waits to enter the LC contract stake table"
            );
        }
    }

    /// The key is in the table: the wait is over. Returns its duration.
    fn finish(self) -> Duration {
        let waited = self.since.elapsed();
        tracing::info!(
            target: LOG_TARGET,
            waited_ms = waited.as_millis() as u64,
            "own key is in the LC contract stake table"
        );
        waited
    }
}

/// The L1 reads of the startup: the LC contract table, the path decision, the candidates and the
/// own-key check.
struct L1Phase<'a, L, M> {
    l1: &'a L,
    membership: &'a M,
    params: &'a StartupCatchupParams,
    first_epoch: EpochNumber,
}

impl<L: StartupL1Reader, M: StartupMembership> L1Phase<'_, L, M> {
    /// Reads L1 until the path is decided: `None` for recent storage, else the fill's inputs. While
    /// the own key is registered but not in `ST(E')`, re-runs on each L1 finality tick, without
    /// deadline.
    async fn run(
        &self,
        retry: &mut Retry,
        stats: &mut StartupStats,
    ) -> Result<Option<FillPlan>, StartupCatchupError> {
        let started = Instant::now();
        let step_timeout = self.params.step_timeout;
        let mut b_fin = retry
            .run(StartupStage::L1Read, step_timeout, || {
                self.l1.l1_finalized_block()
            })
            .await?;
        let mut wait: Option<OwnKeyWait> = None;
        let mut waited = Duration::ZERO;
        let plan = loop {
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
            if self.starts_from_storage(&lc_contract) {
                return Ok(None);
            }
            let events = retry
                .run(StartupStage::L1Read, step_timeout, || {
                    self.l1.stake_table_events(b_fin)
                })
                .await?;
            let plan = self.plan(lc_contract, &events)?;
            match plan
                .fill
                .peers
                .own_key_status(&self.params.own_key, &plan.registry)
            {
                OwnKeyStatus::InVotingStakeTable => {
                    waited += wait.take().map_or(Duration::ZERO, OwnKeyWait::finish);
                    break plan.fill;
                },
                OwnKeyStatus::NotRegistered => {
                    return Err(StartupCatchupError::OwnKeyNotRegistered {
                        key: Box::new(self.params.own_key),
                    });
                },
                OwnKeyStatus::RegisteredNotInVotingStakeTable => {
                    let voting_epoch = plan.fill.lc_contract.voting_epoch;
                    match wait.as_mut() {
                        Some(wait) => wait.tick(voting_epoch, b_fin),
                        None => {
                            wait =
                                Some(OwnKeyWait::enter(&self.params.own_key, voting_epoch, b_fin))
                        },
                    }
                    let tick_started = Instant::now();
                    b_fin = self.next_tick(retry, b_fin).await?;
                    // The wait does not count against the deadline.
                    retry.deadline += tick_started.elapsed();
                },
            }
        };
        stats.l1_read.duration_ms = started.elapsed().saturating_sub(waited).as_millis() as u64;
        Ok(Some(plan))
    }

    /// The next L1 finalized block, awaited without timeout; a failed read counts as an attempt.
    async fn next_tick(&self, retry: &mut Retry, b_fin: u64) -> Result<u64, StartupCatchupError> {
        loop {
            match self.l1.next_l1_finalized_block(b_fin).await {
                Ok(next) => {
                    retry.reset();
                    return Ok(next);
                },
                Err(err) => retry.failed(StartupStage::L1Read, err).await?,
            }
        }
    }

    /// Whether storage is recent enough to skip the fill, logging the decision.
    fn starts_from_storage(&self, lc_contract: &LcContractVotingStakeTable) -> bool {
        let anchor_epoch = self.params.anchor_epoch;
        let lowest = self
            .first_epoch
            .max(EpochNumber::new(anchor_epoch.saturating_sub(1)));
        let held = held_epochs(self.membership, lowest..=anchor_epoch + 1);
        let path = StartupPath::choose(
            anchor_epoch,
            lc_contract.voting_epoch,
            self.first_epoch,
            &held,
        );
        match path {
            StartupPath::FromStorage { .. } => tracing::info!(
                target: LOG_TARGET,
                path = "start from storage",
                %anchor_epoch,
                lc_contract_voting_epoch = %lc_contract.voting_epoch,
                "startup path"
            ),
            StartupPath::Fill => tracing::info!(target: LOG_TARGET, path = "fill", "startup path"),
        }
        matches!(path, StartupPath::FromStorage { .. })
    }

    /// The fill's inputs from the L1 events: the candidate `ST(E')` (the seeded genesis table for
    /// `E' <= first + 1`), `ST(now)` and the registry, both from the state after the last event.
    fn plan(
        &self,
        lc_contract: LcContractVotingStakeTable,
        events: &[(EventKey, StakeTableEvent)],
    ) -> Result<L1Plan, StartupCatchupError> {
        let (candidate, voting_epoch_peers, state) =
            if lc_contract.voting_epoch <= self.first_epoch + 1 {
                let state = stake_table_state_at(events, lc_contract.l1_block)
                    .map_err(LcContractStakeTableMatchError::from)?;
                (None, self.seeded_peers(&lc_contract)?, state)
            } else {
                let (candidate, state) = LcContractCandidateStakeTable::find(
                    &lc_contract,
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
                (Some(candidate), peers, state)
            };
        // No eligible validator yet: the seeded table can be the only peer set.
        let now = match ValidatorSet::from_state(&state, self.params.running_version()) {
            Ok(set) => Some(set),
            Err(StakeTableError::NoValidValidators) => None,
            Err(err) => return Err(LcContractStakeTableMatchError::from(err).into()),
        };
        let active = now
            .as_ref()
            .map(|set| set.active_validators().clone())
            .unwrap_or_default();
        Ok(L1Plan {
            registry: state.validators().clone(),
            fill: FillPlan {
                peers: BootstrapPeers::new(&active, &voting_epoch_peers),
                lc_contract,
                candidate,
                first_epoch: self.first_epoch,
            },
        })
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
        let snapshot = self.membership.snapshot(epoch).ok_or_else(no_match)?;
        let commits = lc_contract
            .commits_to(&snapshot.table, self.params.stake_table_capacity)
            .map_err(|source| LcContractStakeTableMatchError::Commitment { source })?;
        if !commits {
            return Err(no_match().into());
        }
        Ok(snapshot
            .table
            .iter()
            .map(|peer| (*peer.stake_table_entry.key(), peer.clone()))
            .collect())
    }
}

/// The fill's inputs and the registry at `b_fin`.
struct L1Plan {
    fill: FillPlan,
    registry: RegisteredValidatorMap,
}

/// The epochs of `range` whose stake tables are in memory, read without starting catchup.
fn held_epochs(
    membership: &impl StartupMembership,
    range: RangeInclusive<EpochNumber>,
) -> BTreeSet<EpochNumber> {
    (**range.start()..=**range.end())
        .map(EpochNumber::new)
        .filter(|epoch| membership.snapshot(*epoch).is_some())
        .collect()
}

/// The fill: observes certificates and fetches until the window and the DRBs are held.
struct Fill<'a, N, M> {
    net: &'a mut N,
    membership: &'a M,
    params: &'a StartupCatchupParams,
    lc_contract: LcContractVotingStakeTable,
    candidate: Option<LcContractCandidateStakeTable>,
    progress: StartupProgress,
    anchor: Option<VerifiedAnchorLeaf>,
    /// Certificates no held quorum verified yet, re-checked after each installed table.
    pending: Vec<ObservedCertificate>,
    /// Epochs installed from a header proof by this fill.
    installed: BTreeSet<EpochNumber>,
    /// Whether the table of `E'` was checked against the LC contract.
    commitment_checked: bool,
    upgrade_lock: UpgradeLock<SeqTypes>,
    stats: &'a mut StartupStats,
}

impl<'a, N: StartupCliquenet, M: StartupMembership> Fill<'a, N, M> {
    fn new(
        net: &'a mut N,
        membership: &'a M,
        params: &'a StartupCatchupParams,
        plan: FillPlan,
        stats: &'a mut StartupStats,
    ) -> Self {
        let progress = StartupProgress {
            epoch_height: membership.epoch_height(),
            first_epoch: plan.first_epoch,
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
            commitment_checked: false,
            upgrade_lock: params.upgrade_lock.clone(),
            stats,
        }
    }

    /// Runs steps until `Done`; a step that makes no progress backs off, and the deadline ends the
    /// fill.
    async fn run(
        mut self,
        mut retry: Retry,
        started: Instant,
    ) -> Result<EpochNumber, StartupCatchupError> {
        let mut last_error = anyhow!("no step ran");
        loop {
            self.refresh_held();
            self.check_voting_epoch_commitment()?;
            let step = self.progress.next_step();
            let stage = match &step {
                StartupStep::Done {
                    cliquenet_current_epoch,
                } => {
                    self.log_done(*cliquenet_current_epoch, started);
                    return Ok(*cliquenet_current_epoch);
                },
                StartupStep::ObserveCertificates => StartupStage::ObserveCertificates,
                StartupStep::Fetch(batch) => batch.first().stage(),
            };
            retry.check(stage, self.deadline_source(stage, &last_error))?;
            let outcome = match &step {
                StartupStep::Fetch(batch) => self.fetch(batch).await,
                _ => self.observe().await,
            };
            match outcome {
                Ok(()) => retry.reset(),
                Err(err) => {
                    last_error = anyhow!("{err:#}");
                    retry.failed(stage, err).await?;
                },
            }
        }
    }

    /// The error a deadline at `stage` reports; with nothing received from cliquenet at all, it
    /// points at the firewall.
    fn deadline_source(&self, stage: StartupStage, last_error: &anyhow::Error) -> anyhow::Error {
        let stats = &self.stats;
        let received = stats.observe.answered
            + stats.verified_leaf.answered
            + stats.header_proofs.answered
            + stats.drb_leaves.answered;
        if stage == StartupStage::ObserveCertificates && received == 0 {
            anyhow!("no cliquenet connection established; check firewall and p2p_addr")
        } else {
            anyhow!("{last_error:#}")
        }
    }

    /// Refreshes the held tables and DRBs from the membership, the source of truth.
    fn refresh_held(&mut self) {
        let progress = &mut self.progress;
        let highest = self
            .membership
            .highest_known_epoch()
            .unwrap_or(progress.first_epoch);
        progress.held_tables = held_epochs(self.membership, progress.first_epoch..=highest);
        progress.held_drbs = progress
            .held_tables
            .iter()
            .copied()
            .filter(|epoch| {
                self.membership
                    .snapshot(*epoch)
                    .is_some_and(|snapshot| snapshot.has_drb)
            })
            .collect();
    }

    /// Checks the table of `E'` against the LC contract once it is held, whether stored, seeded or
    /// installed from a proof; exits on a mismatch. The walk of a lagging prover does not hold
    /// `E'`, so it does not run this check.
    fn check_voting_epoch_commitment(&mut self) -> Result<(), StartupCatchupError> {
        let epoch = self.lc_contract.voting_epoch;
        if self.commitment_checked || !self.progress.held_tables.contains(&epoch) {
            return Ok(());
        }
        let table = self
            .membership
            .snapshot(epoch)
            .expect("held tables are in memory")
            .table;
        let capacity = self.params.stake_table_capacity;
        if !self
            .lc_contract
            .commits_to(&table, capacity)
            .unwrap_or(false)
        {
            tracing::error!(
                target: LOG_TARGET,
                %epoch,
                held = ?table.commitment(capacity).ok(),
                lc_contract = ?self.lc_contract.commitment,
                "the held committee differs from the LC contract commitment"
            );
            return Err(StartupCatchupError::InstalledCommitmentMismatch { epoch });
        }
        self.commitment_checked = true;
        Ok(())
    }

    /// The table verifying the certificates of `epoch`: installed, else the candidate.
    fn quorum(&self, epoch: EpochNumber) -> Option<HSStakeTable<SeqTypes>> {
        let installed = self
            .membership
            .snapshot(epoch)
            .map(|snapshot| snapshot.table);
        match &self.candidate {
            Some(candidate) => candidate.quorum_for(epoch, installed),
            None => installed,
        }
    }

    /// Waits for one certificate, verifies it or keeps it as a claim.
    async fn observe(&mut self) -> anyhow::Result<()> {
        let started = Instant::now();
        let certificate = self
            .net
            .next_observed_certificate(self.params.step_timeout)
            .await;
        self.stats.observe.duration_ms += started.elapsed().as_millis() as u64;
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
        let pending = mem::take(&mut self.pending);
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
        let elapsed_ms = started.elapsed().as_millis() as u64;
        let before = self.fingerprint();

        let results = self.apply_in_order(batch, responses).await;
        self.refresh_held();
        self.reverify_pending();
        while let Some(certificate) = self.net.next_observed_certificate(Duration::ZERO).await {
            self.absorb(certificate);
        }

        let mut timed = HashSet::new();
        let mut first_error = None;
        for (request, result) in batch.iter().zip(results) {
            let stats = self.stats.of_request(request);
            if timed.insert(mem::discriminant(request)) {
                stats.duration_ms += elapsed_ms;
            }
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

    /// Applies the answers of `batch` in its order, which is ascending by epoch: a later table
    /// needs its predecessors, so tables stop at the first gap.
    async fn apply_in_order(
        &mut self,
        batch: &Vec1<StartupRequest>,
        responses: Vec<anyhow::Result<StartupResponse>>,
    ) -> Vec<anyhow::Result<()>> {
        let mut results = Vec::with_capacity(batch.len());
        let mut gap = false;
        for (request, response) in batch.iter().zip(responses) {
            let is_table = matches!(request, StartupRequest::RootHeaderProof { .. });
            let result = match response {
                Err(err) => Err(err),
                Ok(_) if is_table && gap => Err(anyhow!("an earlier table is missing")),
                Ok(response) => self.apply(request, response).await,
            };
            if let Err(err) = &result {
                tracing::debug!(?request, "answer not applied: {err:#}");
                gap |= is_table;
            }
            results.push(result);
        }
        results
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
        self.membership.add_epoch_root(header).await?;
        self.installed.insert(epoch);
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
        self.membership.supply_drb(epoch, drb);
        Ok(())
    }

    fn log_done(&self, current_epoch: EpochNumber, started: Instant) {
        let stats = &self.stats;
        tracing::info!(
            target: LOG_TARGET,
            %current_epoch,
            duration_ms = started.elapsed().as_millis() as u64,
            installed = ?self.installed,
            l1_read = ?stats.l1_read,
            observe = ?stats.observe,
            verified_leaf = ?stats.verified_leaf,
            header_proofs = ?stats.header_proofs,
            drb_leaves = ?stats.drb_leaves,
            "startup catchup done"
        );
    }
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
    use hotshot_new_protocol::testing::{sign_certificate, sign_leaf_chain, with_next_drb_result};
    use hotshot_query_service::node::BlockId;
    use hotshot_types::{
        ValidatorConfig, addr::NetAddr, signature_key::BLSPrivKey, simple_vote::QuorumData2,
        traits::signature_key::SignatureKey, x25519,
    };
    use light_client::{client::Client, testing::TestClient};
    use rstest::rstest;
    use tracing_subscriber::fmt::MakeWriter;
    use versions::{EPOCH_VERSION, NEW_PROTOCOL_VERSION, Upgrade};

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
            // `C` is verified and the anchor is at it: a claim far above does not walk on.
            let held = StartupProgress {
                anchor_height: Some(901),
                cliquenet_current_epoch: Some(EpochNumber::new(10)),
                cliquenet_current_height: Some(950),
                highest_unverified_claim: Some(EpochNumber::new(1000)),
                held_tables: epochs([1, 2, 7, 8, 9, 10, 11]),
                held_drbs: epochs([1, 2, 10]),
                ..progress(10)
            };
            assert!(matches!(held.next_step(), StartupStep::Done { .. }));

            // The window is still fetched in one batch, not walked.
            let missing = StartupProgress {
                held_tables: epochs([1, 2, 7, 8, 9, 10]),
                ..held
            };
            assert_eq!(
                fetched(missing.next_step()),
                [StartupRequest::RootHeaderProof {
                    epoch: EpochNumber::new(11),
                    anchor_height: 901,
                    root_height: root_block_in_epoch(9, 100)
                }]
            );
        }

        #[test]
        fn claim_without_verified_current_epoch_walks() {
            let progress = StartupProgress {
                anchor_height: Some(901),
                highest_unverified_claim: Some(EpochNumber::new(1000)),
                held_tables: epochs([1, 2, 10, 11]),
                ..progress(10)
            };

            assert_eq!(
                fetched(progress.next_step()),
                [StartupRequest::VerifiedLeaf {
                    epoch: EpochNumber::new(11),
                    height: 1001
                }]
            );
        }

        #[tokio::test]
        async fn genesis_view_certificate_is_rejected() {
            // An unsigned certificate of the genesis view passes `Verifiable::check`.
            let committee = fakes::Committee::new(4);
            let data = QuorumData2 {
                leaf_commit: Commitment::from_raw([5; 32]),
                epoch: Some(EpochNumber::new(3)),
                block_number: Some(25),
            };
            let cert = Certificate1::<SeqTypes>::new(
                data,
                Commitment::from_raw([6; 32]),
                ViewNumber::genesis(),
                None,
                std::marker::PhantomData,
            );
            let table = HSStakeTable::<SeqTypes>::from(committee.peers());
            let lock = UpgradeLock::new(Upgrade::trivial(NEW_PROTOCOL_VERSION));

            let err = ObservedCertificate::Cert1(cert)
                .verify(&table, EPOCH_HEIGHT, &lock)
                .unwrap_err();

            assert!(err.to_string().contains("genesis view"), "{err:#}");
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
        #[case::in_peers(true, true, OwnKeyStatus::InVotingStakeTable)]
        #[case::exited_key_in_candidate_window_fills_ok(
            true,
            false,
            OwnKeyStatus::InVotingStakeTable
        )]
        #[case::inactive_key_waits_ok(false, true, OwnKeyStatus::RegisteredNotInVotingStakeTable)]
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

        /// L1 whose LC contract read and stake table events advance with each finality tick; the
        /// last entry stays. Each tick takes `tick_delay`.
        struct FakeL1 {
            lc_contracts: Vec<LcContractVotingStakeTable>,
            events: Vec<Vec<(EventKey, StakeTableEvent)>>,
            tick: Mutex<usize>,
            tick_delay: Duration,
            read_delay: Duration,
        }

        impl FakeL1 {
            fn new(
                lc_contract: LcContractVotingStakeTable,
                events: Vec<Vec<(EventKey, StakeTableEvent)>>,
            ) -> Self {
                Self::per_tick(vec![lc_contract], events)
            }

            fn per_tick(
                lc_contracts: Vec<LcContractVotingStakeTable>,
                events: Vec<Vec<(EventKey, StakeTableEvent)>>,
            ) -> Self {
                Self {
                    lc_contracts,
                    events,
                    tick: Mutex::new(0),
                    tick_delay: Duration::ZERO,
                    read_delay: Duration::ZERO,
                }
            }

            fn ticks(&self) -> usize {
                *self.tick.lock().unwrap()
            }
        }

        #[async_trait]
        impl StartupL1Reader for FakeL1 {
            async fn l1_finalized_block(&self) -> anyhow::Result<u64> {
                Ok(self.lc_contracts[0].l1_block)
            }

            async fn lc_contract_voting_stake_table(
                &self,
                l1_block: u64,
            ) -> anyhow::Result<LcContractVotingStakeTable> {
                sleep(self.read_delay).await;
                let tick = self.ticks().min(self.lc_contracts.len() - 1);
                Ok(LcContractVotingStakeTable {
                    l1_block,
                    ..self.lc_contracts[tick].clone()
                })
            }

            async fn stake_table_events(
                &self,
                _to_l1_block: u64,
            ) -> anyhow::Result<Vec<(EventKey, StakeTableEvent)>> {
                let tick = self.ticks().min(self.events.len() - 1);
                Ok(self.events[tick].clone())
            }

            async fn next_l1_finalized_block(&self, after: u64) -> anyhow::Result<u64> {
                sleep(self.tick_delay).await;
                *self.tick.lock().unwrap() += 1;
                Ok(after + 1)
            }
        }

        /// A cliquenet answering requests with `responder` and replaying `certificates`.
        struct FakeCliquenet {
            dialed: Option<Vec<PubKey>>,
            certificates: VecDeque<ObservedCertificate>,
            responder: Responder,
            batches: Vec<usize>,
            delay: Duration,
        }

        impl FakeCliquenet {
            fn new(responder: Responder) -> Self {
                Self {
                    dialed: None,
                    certificates: VecDeque::new(),
                    responder,
                    batches: Vec::new(),
                    delay: Duration::ZERO,
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

            async fn fetch_concurrently(
                &mut self,
                requests: &Vec1<StartupRequest>,
                _wait: Duration,
            ) -> Vec<anyhow::Result<StartupResponse>> {
                self.batches.push(requests.len());
                sleep(self.delay).await;
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
        pub(super) struct Committee {
            configs: Vec<ValidatorConfig<SeqTypes>>,
        }

        impl Committee {
            pub(super) fn new(size: u64) -> Self {
                Self::with_seed(42, size)
            }

            fn with_seed(seed: u8, size: u64) -> Self {
                let configs = (0..size)
                    .map(|index| {
                        let mut config = ValidatorConfig::<SeqTypes>::generated_from_seed_indexed(
                            [seed; 32],
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

            pub(super) fn peers(&self) -> Vec<PeerConfig<SeqTypes>> {
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
                upgrade_lock: lock(),
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
        async fn the_fill_uses_the_configured_upgrade() {
            let membership = FakeMembership::seeded(6);
            let mut net = FakeCliquenet::silent();
            let mut params = params(committee(5).own_key(), 0);
            let configured = Upgrade::new(EPOCH_VERSION, NEW_PROTOCOL_VERSION);
            params.upgrade_lock = UpgradeLock::new(configured);
            let mut stats = StartupStats::default();

            let fill = fill_of(&mut net, &membership, &params, &committee(5), &mut stats);

            assert_eq!(fill.upgrade_lock.upgrade(), configured);
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

            let text = err.to_string();
            assert!(
                text.contains("no cliquenet connection established; check firewall and p2p_addr"),
                "{text}"
            );

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
            // The own key is registered but not in `ST(E')` for 40 ticks of 31 minutes each, far
            // beyond the step timeout and the deadline; then the LC contract commits to a table
            // with it. One WARN enters the wait, a heartbeat WARN follows per 30 minutes, and
            // nothing else at WARN.
            let logs = LogBuffer::default();
            let subscriber = tracing_subscriber::fmt()
                .with_writer(logs.clone())
                .with_ansi(false)
                .finish();
            let _guard = tracing::subscriber::set_default(subscriber);
            let committee = Committee::new(4);
            let (_, events) = candidate_fixture();
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
            let before = voting(5, commitment_of_events(&events));
            let after = voting(5, commitment_of_events(&active));
            let mut lcs = vec![before; 40];
            lcs.push(after);
            let mut all_events = vec![waiting; 40];
            all_events.push(active);
            let mut l1 = FakeL1::per_tick(lcs, all_events);
            l1.tick_delay = Duration::from_secs(31 * 60);
            let coordinator = committee.membership(1);
            let mut net = FakeCliquenet::silent();

            let err = catch_up_over_cliquenet(
                &l1,
                &mut net,
                &coordinator,
                &params(validator_key(&own), 1),
            )
            .await
            .unwrap_err();

            // The key entered `ST(E')` on tick 40: the fill dialed and ran out of time.
            assert_eq!(l1.ticks(), 40);
            assert!(net.dialed.is_some());
            assert!(matches!(
                err,
                StartupCatchupError::DeadlineExceeded {
                    stage: StartupStage::ObserveCertificates,
                    ..
                }
            ));
            let output = String::from_utf8(logs.0.lock().unwrap().clone()).unwrap();
            let warns: Vec<_> = output
                .lines()
                .filter(|line| line.contains(" WARN "))
                .collect();
            let entering = warns
                .iter()
                .filter(|line| line.contains("own key is not in the LC contract stake table"))
                .count();
            let heartbeats = warns
                .iter()
                .filter(|line| line.contains("still waits to enter"))
                .count();
            assert_eq!(entering, 1, "{output}");
            assert_eq!(heartbeats, 39, "{output}");
            assert_eq!(warns.len(), 1 + heartbeats, "{output}");
            assert!(
                output.contains("own key is in the LC contract stake table"),
                "{output}"
            );
        }

        #[tokio::test(start_paused = true)]
        async fn the_deadline_is_shared_by_the_l1_phase_and_the_fill() {
            // The L1 phase takes 150 ms of the 200 ms deadline; the fill gets the rest.
            let committee = Committee::new(4);
            let coordinator = committee.membership(3);
            let mut l1 = FakeL1::new(voting(3, committee.commitment()), vec![vec![]]);
            l1.read_delay = Duration::from_millis(150);
            let mut net = FakeCliquenet::silent();
            let started = Instant::now();

            catch_up_over_cliquenet(&l1, &mut net, &coordinator, &params(committee.own_key(), 1))
                .await
                .unwrap_err();

            assert!(
                started.elapsed() < Duration::from_millis(300),
                "{:?}",
                started.elapsed()
            );
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

        /// The committee of `epoch`: one for the seeded epochs, a distinct one for each later epoch.
        fn committee(epoch: u64) -> Committee {
            Committee::with_seed(100 + epoch.saturating_sub(2) as u8, 4)
        }

        /// An in-memory membership holding the seeded epochs 1 and 2, which installs the table
        /// scripted for the root height of each header it is given.
        struct FakeMembership {
            tables: Mutex<BTreeMap<EpochNumber, TableSnapshot>>,
            scripted: BTreeMap<u64, HSStakeTable<SeqTypes>>,
            installs: Mutex<Vec<EpochNumber>>,
            drbs: Mutex<Vec<EpochNumber>>,
        }

        impl FakeMembership {
            /// Holds the table of `held` epochs, `scripted` by the height of the root header that
            /// installs them.
            fn new(
                held: impl IntoIterator<Item = (u64, HSStakeTable<SeqTypes>)>,
                scripted: BTreeMap<u64, HSStakeTable<SeqTypes>>,
            ) -> Self {
                let tables = held
                    .into_iter()
                    .map(|(epoch, table)| {
                        (
                            EpochNumber::new(epoch),
                            TableSnapshot {
                                table,
                                has_drb: epoch <= 2,
                            },
                        )
                    })
                    .collect();
                Self {
                    tables: Mutex::new(tables),
                    scripted,
                    installs: Mutex::default(),
                    drbs: Mutex::default(),
                }
            }

            /// The seeded epochs 1 and 2 plus the scripted installs of epochs `3..=last`.
            fn seeded(last: u64) -> Self {
                let scripted = (3..=last)
                    .map(|epoch| {
                        (
                            root_block_in_epoch(epoch - 2, EPOCH_HEIGHT),
                            HSStakeTable::from(committee(epoch).peers()),
                        )
                    })
                    .collect();
                let seeded = HSStakeTable::from(committee(2).peers());
                Self::new([(1, seeded.clone()), (2, seeded)], scripted)
            }

            fn installs(&self) -> Vec<u64> {
                self.installs.lock().unwrap().iter().map(|e| **e).collect()
            }
        }

        #[async_trait]
        impl StartupMembership for FakeMembership {
            fn epoch_height(&self) -> u64 {
                EPOCH_HEIGHT
            }

            fn first_epoch(&self) -> EpochNumber {
                EpochNumber::new(1)
            }

            fn highest_known_epoch(&self) -> Option<EpochNumber> {
                self.tables.lock().unwrap().keys().next_back().copied()
            }

            fn snapshot(&self, epoch: EpochNumber) -> Option<TableSnapshot> {
                self.tables.lock().unwrap().get(&epoch).cloned()
            }

            async fn add_epoch_root(&self, header: Header) -> anyhow::Result<()> {
                let table = self
                    .scripted
                    .get(&header.height())
                    .with_context(|| format!("no table scripted for root {}", header.height()))?;
                let epoch =
                    EpochNumber::new(epoch_from_block_number(header.height(), EPOCH_HEIGHT) + 2);
                self.tables.lock().unwrap().insert(
                    epoch,
                    TableSnapshot {
                        table: table.clone(),
                        has_drb: false,
                    },
                );
                self.installs.lock().unwrap().push(epoch);
                Ok(())
            }

            fn supply_drb(&self, epoch: EpochNumber, _drb: DrbResult) {
                self.tables
                    .lock()
                    .unwrap()
                    .get_mut(&epoch)
                    .expect("the table of a DRB is installed first")
                    .has_drb = true;
                self.drbs.lock().unwrap().push(epoch);
            }
        }

        /// One chain of blocks shared by the install tests.
        async fn blocks() -> &'static TestClient {
            static BLOCKS: tokio::sync::OnceCell<TestClient> = tokio::sync::OnceCell::const_new();
            BLOCKS
                .get_or_init(|| async {
                    let client = TestClient::with_epoch_height(EPOCH_HEIGHT);
                    client.leaf(60).await;
                    client
                })
                .await
        }

        fn first_block(epoch: u64) -> u64 {
            (epoch - 1) * EPOCH_HEIGHT + 1
        }

        /// The answers of the peers: a leaf chain of three blocks from `from`, signed by
        /// `committee`, optionally carrying a DRB in its first leaf.
        async fn leaf_chain_answer(
            committee: &Committee,
            from: u64,
            drb: Option<DrbResult>,
        ) -> StartupResponse {
            let client = blocks().await;
            let mut leaves = Vec::new();
            for height in from..from + 3 {
                leaves.push(client.leaf(height as usize).await.leaf().clone());
            }
            if let Some(drb) = drb {
                leaves[0] = with_next_drb_result(&leaves[0], drb);
            }
            StartupResponse::LeafChain(Box::new(sign_leaf_chain(
                leaves,
                EPOCH_HEIGHT,
                &committee.signers(),
                &lock(),
            )))
        }

        async fn header_proof_answer(anchor_height: u64, root_height: u64) -> StartupResponse {
            let proof = blocks()
                .await
                .header_proof(anchor_height, BlockId::Number(root_height as usize))
                .await
                .unwrap();
            StartupResponse::HeaderProof(Box::new(proof))
        }

        /// A responder that answers exactly the scripted requests.
        fn scripted(answers: Vec<(StartupRequest, StartupResponse)>) -> Responder {
            Box::new(move |request| {
                answers
                    .iter()
                    .find(|(asked, _)| asked == request)
                    .map(|(_, answer)| answer.clone())
                    .ok_or_else(|| anyhow!("no answer scripted for {request:?}"))
            })
        }

        fn table_request(epoch: u64, anchor_epoch: u64) -> StartupRequest {
            StartupRequest::RootHeaderProof {
                epoch: EpochNumber::new(epoch),
                anchor_height: first_block(anchor_epoch),
                root_height: root_block_in_epoch(epoch - 2, EPOCH_HEIGHT),
            }
        }

        async fn table_answer(epoch: u64, anchor_epoch: u64) -> (StartupRequest, StartupResponse) {
            let request = table_request(epoch, anchor_epoch);
            let StartupRequest::RootHeaderProof {
                anchor_height,
                root_height,
                ..
            } = request
            else {
                unreachable!()
            };
            (
                request,
                header_proof_answer(anchor_height, root_height).await,
            )
        }

        async fn leaf_answer(epoch: u64) -> (StartupRequest, StartupResponse) {
            (
                StartupRequest::VerifiedLeaf {
                    epoch: EpochNumber::new(epoch),
                    height: first_block(epoch),
                },
                leaf_chain_answer(&committee(epoch), first_block(epoch), None).await,
            )
        }

        async fn drb_answer(epoch: u64) -> (StartupRequest, StartupResponse) {
            let height = transition_block_for_epoch(epoch - 1, EPOCH_HEIGHT);
            (
                StartupRequest::DrbLeaf {
                    epoch: EpochNumber::new(epoch),
                    height,
                },
                leaf_chain_answer(&committee(epoch - 1), height, Some([7; 32])).await,
            )
        }

        /// A certificate of `block`, signed by the committee of its epoch.
        fn certificate(block: u64) -> ObservedCertificate {
            let epoch = epoch_from_block_number(block, EPOCH_HEIGHT);
            committee(epoch).cert1(block, &lock())
        }

        #[tokio::test(start_paused = true)]
        async fn prover_lag_walk_ok() {
            // `E' = 2` (seeded) while the network is at epoch 6: the walk installs 3 to 6, each
            // proven against the verified leaf of the epoch before, then the window.
            let membership = FakeMembership::seeded(7);
            let mut answers = vec![];
            for epoch in 2..=6 {
                answers.push(leaf_answer(epoch).await);
            }
            for epoch in 3..=6 {
                answers.push(table_answer(epoch, epoch - 1).await);
            }
            answers.push(table_answer(7, 6).await);
            answers.push(drb_answer(6).await);
            let mut net = FakeCliquenet::new(scripted(answers));
            net.certificates.push_back(certificate(55));
            let own_key = committee(2).own_key();
            let l1 = FakeL1::new(voting(2, committee(2).commitment()), vec![vec![]]);

            let current = catch_up_over_cliquenet(&l1, &mut net, &membership, &params(own_key, 0))
                .await
                .unwrap();

            assert_eq!(current, EpochNumber::new(6));
            assert_eq!(membership.installs(), [3, 4, 5, 6, 7]);
            assert_eq!(*membership.drbs.lock().unwrap(), [EpochNumber::new(6)]);
            // The leaf of `E'`, a table and a leaf per walked epoch, then the window in one batch.
            assert_eq!(net.batches, [1, 1, 1, 1, 1, 1, 1, 1, 1, 2]);
        }

        /// A fill whose candidate `ST(5)` is the committee of epoch 5 and whose LC contract
        /// commits to `lc_committee`, on top of `membership`.
        fn fill_of<'a>(
            net: &'a mut FakeCliquenet,
            membership: &'a FakeMembership,
            params: &'a StartupCatchupParams,
            lc_committee: &Committee,
            stats: &'a mut StartupStats,
        ) -> Fill<'a, FakeCliquenet, FakeMembership> {
            let validators: AuthenticatedValidatorMap = committee(5)
                .peers()
                .into_iter()
                .map(|peer| {
                    let mut registered = RegisteredValidator::mock();
                    registered.stake_table_key = Some(*peer.stake_table_entry.key());
                    registered.state_ver_key = Some(peer.state_ver_key.clone());
                    (registered.account, registered.try_into().unwrap())
                })
                .collect();
            let candidate = LcContractCandidateStakeTable {
                epoch: EpochNumber::new(5),
                l1_block: 1,
                version: NEW_PROTOCOL_VERSION,
                validators,
            };
            let voting_peers = committee(5)
                .peers()
                .into_iter()
                .map(|peer| (*peer.stake_table_entry.key(), peer))
                .collect();
            let plan = FillPlan {
                lc_contract: voting(5, lc_committee.commitment()),
                candidate: Some(candidate),
                peers: BootstrapPeers::new(&AuthenticatedValidatorMap::default(), &voting_peers),
                first_epoch: EpochNumber::new(1),
            };
            Fill::new(net, membership, params, plan, stats)
        }

        /// The answers of a fill at `E' = 5` with `C = 5`: the leaf of `E'`, tables 3 to 6, a DRB.
        async fn window_answers() -> Vec<(StartupRequest, StartupResponse)> {
            let mut answers = vec![leaf_answer(5).await, drb_answer(5).await];
            for epoch in 3..=6 {
                answers.push(table_answer(epoch, 5).await);
            }
            answers
        }

        #[tokio::test(start_paused = true)]
        async fn window_is_installed_in_ascending_order_ok() {
            let membership = FakeMembership::seeded(6);
            let mut net = FakeCliquenet::new(scripted(window_answers().await));
            net.certificates.push_back(certificate(45));
            let params = params(committee(5).own_key(), 0);
            let mut stats = StartupStats::default();
            // The LC contract commits to the table installed for epoch 5.
            let fill = fill_of(&mut net, &membership, &params, &committee(5), &mut stats);

            let current = fill
                .run(Retry::new(Duration::from_secs(60)), Instant::now())
                .await
                .unwrap();

            assert_eq!(current, EpochNumber::new(5));
            assert_eq!(membership.installs(), [3, 4, 5, 6]);
            assert_eq!(*membership.drbs.lock().unwrap(), [EpochNumber::new(5)]);
            assert_eq!(net.batches, [1, 5]);
        }

        #[tokio::test(start_paused = true)]
        async fn installed_commitment_mismatch_fails() {
            let membership = FakeMembership::seeded(6);
            let mut net = FakeCliquenet::new(scripted(window_answers().await));
            net.certificates.push_back(certificate(45));
            let params = params(committee(5).own_key(), 0);
            let mut stats = StartupStats::default();
            // The LC contract commits to another table than the one installed for epoch 5.
            let fill = fill_of(&mut net, &membership, &params, &committee(9), &mut stats);

            let err = fill
                .run(Retry::new(Duration::from_secs(60)), Instant::now())
                .await
                .unwrap_err();

            assert!(
                matches!(
                    err,
                    StartupCatchupError::InstalledCommitmentMismatch { epoch }
                        if epoch == EpochNumber::new(5)
                ),
                "{err:#}"
            );
        }

        #[tokio::test(start_paused = true)]
        async fn stored_commitment_mismatch_fails() {
            // The table of `E'` is already held, from a previous boot, and is not the LC contract's.
            let stored = HSStakeTable::from(committee(9).peers());
            let membership = FakeMembership::new(
                [(1, HSStakeTable::from(committee(2).peers())), (5, stored)],
                BTreeMap::new(),
            );
            let mut net = FakeCliquenet::silent();
            let params = params(committee(5).own_key(), 0);
            let mut stats = StartupStats::default();
            let fill = fill_of(&mut net, &membership, &params, &committee(5), &mut stats);

            let err = fill
                .run(Retry::new(Duration::from_secs(60)), Instant::now())
                .await
                .unwrap_err();

            assert!(matches!(
                err,
                StartupCatchupError::InstalledCommitmentMismatch { .. }
            ));
            assert!(net.batches.is_empty());
        }

        #[tokio::test(start_paused = true)]
        async fn stored_commitment_match_is_accepted() {
            let stored = HSStakeTable::from(committee(5).peers());
            let membership = FakeMembership::new([(5, stored)], BTreeMap::new());
            let mut net = FakeCliquenet::silent();
            let params = params(committee(5).own_key(), 0);
            let mut stats = StartupStats::default();
            let mut fill = fill_of(&mut net, &membership, &params, &committee(5), &mut stats);

            fill.refresh_held();

            fill.check_voting_epoch_commitment().unwrap();
            assert!(fill.commitment_checked);
        }

        /// A fill with the leaf of epoch 5 as anchor, to apply batches of header proofs.
        async fn anchored<'a>(
            net: &'a mut FakeCliquenet,
            membership: &'a FakeMembership,
            params: &'a StartupCatchupParams,
            stats: &'a mut StartupStats,
        ) -> Fill<'a, FakeCliquenet, FakeMembership> {
            let mut fill = fill_of(net, membership, params, &committee(5), stats);
            let leaf = blocks().await.leaf(first_block(5) as usize).await;
            fill.anchor = Some(VerifiedAnchorLeaf::from_verified(leaf.leaf().clone()));
            fill
        }

        #[tokio::test(start_paused = true)]
        async fn a_missing_table_stops_the_batch() {
            let membership = FakeMembership::seeded(6);
            let mut net = FakeCliquenet::silent();
            let params = params(committee(5).own_key(), 0);
            let mut stats = StartupStats::default();
            let mut fill = anchored(&mut net, &membership, &params, &mut stats).await;
            let batch = Vec1::try_from_vec(vec![
                table_request(3, 5),
                table_request(4, 5),
                table_request(5, 5),
            ])
            .unwrap();
            // The proof of epoch 4 is not served.
            let responses = vec![
                Ok(table_answer(3, 5).await.1),
                Err(anyhow!("no peer answers")),
                Ok(table_answer(5, 5).await.1),
            ];

            let results = fill.apply_in_order(&batch, responses).await;

            assert!(results[0].is_ok());
            assert!(results[1].is_err());
            let err = results[2].as_ref().unwrap_err();
            assert!(
                err.to_string().contains("earlier table is missing"),
                "{err:#}"
            );
            assert_eq!(membership.installs(), [3]);
        }

        #[tokio::test(start_paused = true)]
        async fn stage_time_is_added_once_per_kind_and_batch() {
            let membership = FakeMembership::seeded(6);
            let mut net = FakeCliquenet::new(scripted(vec![
                table_answer(3, 5).await,
                table_answer(4, 5).await,
            ]));
            net.delay = Duration::from_secs(1);
            let params = params(committee(5).own_key(), 0);
            let mut stats = StartupStats::default();
            let mut fill = anchored(&mut net, &membership, &params, &mut stats).await;
            let batch = Vec1::try_from_vec(vec![table_request(3, 5), table_request(4, 5)]).unwrap();

            fill.fetch(&batch).await.unwrap();

            assert_eq!(fill.stats.header_proofs.duration_ms, 1000);
            assert_eq!(fill.stats.header_proofs.answered, 2);
        }
    }
}
