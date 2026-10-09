//! Per-view vote collection and certificate formation.
//!
//! Votes for views arrive concurrently, and each view is tallied on its own
//! until it crosses a threshold and forms a certificate. [`VoteCollector`]
//! owns the machinery common to every kind of vote, inspecting each vote only
//! through the [`Ballot`] trait. It creates a task per view and voted epoch
//! and routes votes to the appropriate task, drops duplicate and stale votes,
//! buffers votes whose epoch is not yet resolved, and GCs decided views. How
//! a task actually combines votes into an output is delegated to a pluggable
//! [`Tally`] strategy.

mod accumulate;

use std::{
    any::{type_name, type_name_of_val},
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    marker::PhantomData,
    mem,
    panic::resume_unwind,
};

pub(crate) use accumulate::{Cert, CheckedAccumulator};
use alloy::primitives::U256;
use committable::Committable;
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    epoch_membership::{EpochMembership, EpochMembershipCoordinator},
    message::UpgradeLock,
    simple_certificate::{
        LightClientStateUpdateCertificateV2, QuorumCertificate2, SuccessThreshold, Threshold,
        TimeoutCertificate3, UpgradeThreshold,
    },
    simple_vote::{HasEpoch, QuorumVote2, SimpleVote, UpgradeVote2, VersionedVoteData, Voteable},
    stake_table::StakeTableEntries,
    traits::{
        node_implementation::NodeType,
        signature_key::{SignatureKey, StakeTableEntryType},
    },
    vote::{Certificate, HasViewNumber, LightClientStateUpdateVoteAccumulator, Vote},
};
use hotshot_utils::anytrace;
use tokio::{sync::mpsc, task::spawn_blocking};
use tokio_util::task::JoinMap;
use tracing::{error, info, warn};

use crate::{
    cert_verifier::{ValidCert, verify_signatures, verify_timeout3},
    message::{TimeoutBallot, Vote1},
};

/// Information about a vote.
///
/// Used by [`VoteCollector`] to handle incoming votes, i.e. reject
/// duplicate votes by the same signer, or stale votes for GCed views,
/// before even tallying the vote.
pub trait Ballot {
    type Signer;

    fn view(&self) -> ViewNumber;
    fn epoch(&self) -> Option<EpochNumber>;
    fn signer(&self) -> Self::Signer;
}

/// How to count votes to form a certificate.
pub trait Tally<T: NodeType>: Send + 'static {
    type Vote: Send + 'static;
    type Output: Send + 'static;

    fn new(m: EpochMembership<T>, l: UpgradeLock<T>) -> Self;

    fn add(&mut self, vote: Self::Vote) -> Option<Self::Output>;

    fn min_votes(&self) -> usize;

    fn has_signer(m: &EpochMembership<T>, signer: &T::SignatureKey) -> bool;
}

impl<T: NodeType, D> Ballot for SimpleVote<T, D>
where
    D: Voteable<T> + HasEpoch + 'static,
{
    type Signer = T::SignatureKey;

    fn view(&self) -> ViewNumber {
        self.view_number()
    }

    fn epoch(&self) -> Option<EpochNumber> {
        HasEpoch::epoch(self)
    }

    fn signer(&self) -> Self::Signer {
        self.signing_key()
    }
}

impl<T: NodeType> Ballot for Vote1<T> {
    type Signer = T::SignatureKey;

    fn view(&self) -> ViewNumber {
        self.vote.view_number()
    }

    fn epoch(&self) -> Option<EpochNumber> {
        HasEpoch::epoch(&self.vote)
    }

    fn signer(&self) -> Self::Signer {
        self.vote.signing_key()
    }
}

/// Accumulates votes into a single [`Certificate`] via a [`CheckedAccumulator`].
pub struct SimpleTally<T, V, Th>
where
    T: NodeType,
    V: Vote<T>,
    Th: Threshold<T>,
    Cert<T, V, Th>: Certificate<T, V::Commitment, Voteable = V::Commitment> + HasEpoch,
{
    accumulator: CheckedAccumulator<T, V, Th>,
}

impl<T, V, Th> Tally<T> for SimpleTally<T, V, Th>
where
    T: NodeType,
    V: Vote<T> + Send + 'static,
    V::Commitment: 'static,
    Th: Threshold<T> + Send + 'static,
    Cert<T, V, Th>:
        Certificate<T, V::Commitment, Voteable = V::Commitment> + HasEpoch + Send + 'static,
{
    type Vote = V;
    type Output = ValidCert<Cert<T, V, Th>>;

    fn new(m: EpochMembership<T>, l: UpgradeLock<T>) -> Self {
        Self {
            accumulator: CheckedAccumulator::new(m, l),
        }
    }

    fn min_votes(&self) -> usize {
        self.accumulator.min_votes()
    }

    fn has_signer(m: &EpochMembership<T>, signer: &T::SignatureKey) -> bool {
        Cert::<T, V, Th>::stake_table_entry(m, signer).is_some()
    }

    fn add(&mut self, vote: V) -> Option<Self::Output> {
        let cert = self.accumulator.add(vote)?;
        let Some(epoch) = cert.epoch() else {
            warn!(
                cert = type_name::<Cert<T, V, Th>>(),
                "certificate has no epoch number"
            );
            return None;
        };
        Some(ValidCert::new(cert, epoch))
    }
}

/// Accumulates [`UpgradeVote2`]s into an `UpgradeCertificate2`, under the
/// epoch the votes bind.
pub type UpgradeTally<T> = SimpleTally<T, UpgradeVote2<T>, UpgradeThreshold>;

/// The quorum and light-client state certificates formed at an epoch-root view.
pub type EpochRootCerts<T> = (
    ValidCert<QuorumCertificate2<T>>,
    LightClientStateUpdateCertificateV2<T>,
);

/// Accumulates epoch-root [`Vote1`]s into the (quorum, state) certificate pair.
pub struct EpochRootTally<T: NodeType> {
    membership: EpochMembership<T>,
    quorum: CheckedAccumulator<T, QuorumVote2<T>, SuccessThreshold>,
    state: LightClientStateUpdateVoteAccumulator<T>,
    quorum_cert: Option<QuorumCertificate2<T>>,
    state_cert: Option<LightClientStateUpdateCertificateV2<T>>,
}

impl<T: NodeType> Tally<T> for EpochRootTally<T> {
    type Vote = Vote1<T>;
    type Output = EpochRootCerts<T>;

    fn new(m: EpochMembership<T>, l: UpgradeLock<T>) -> Self {
        Self {
            quorum: CheckedAccumulator::new(m.clone(), l.clone()),
            state: LightClientStateUpdateVoteAccumulator {
                vote_outcomes: HashMap::new(),
                upgrade_lock: l,
            },
            membership: m,
            quorum_cert: None,
            state_cert: None,
        }
    }

    fn min_votes(&self) -> usize {
        self.quorum.min_votes()
    }

    fn has_signer(m: &EpochMembership<T>, signer: &T::SignatureKey) -> bool {
        QuorumCertificate2::<T>::stake_table_entry(m, signer).is_some()
    }

    fn add(&mut self, vote1: Vote1<T>) -> Option<Self::Output> {
        let Some(state_vote) = vote1.state_vote else {
            error!(view = %vote1.vote.view_number(), "epoch-root vote1 without state vote");
            return None;
        };
        let bls_key = vote1.vote.signing_key();

        if self.quorum_cert.is_none() {
            self.quorum_cert = self.quorum.add(vote1.vote);
        }

        // Unlike quorum votes, state votes are fully checked, including
        // their signatures, by the accumulator, so the certificate does
        // not need to be validated again.
        if self.state_cert.is_none() {
            self.state_cert = self
                .state
                .accumulate(&bls_key, &state_vote, &self.membership);
        }

        let (Some(q), Some(s)) = (&self.quorum_cert, &self.state_cert) else {
            return None;
        };
        info!(view = %q.view_number(), epoch = %s.epoch, "epoch-root certificates formed");
        let Some(e) = q.epoch() else {
            warn!(
                cert = type_name_of_val(q),
                "certificate has no epoch number"
            );
            return None;
        };
        Some((ValidCert::new(q.clone(), e), s.clone()))
    }
}

impl<T: NodeType> Ballot for TimeoutBallot<T> {
    type Signer = T::SignatureKey;

    fn view(&self) -> ViewNumber {
        self.vote().view_number()
    }

    fn epoch(&self) -> Option<EpochNumber> {
        Some(self.vote().data.epoch)
    }

    fn signer(&self) -> Self::Signer {
        self.vote().signing_key()
    }
}

/// Accumulates [`TimeoutBallot`]s into a [`TimeoutCertificate3`] once their
/// signers' combined stake reaches the threshold `Th`.
///
/// Signers with different locks sign different data, so unlike a
/// [`SimpleTally`] the stake is counted across locks. As in a
/// [`CheckedAccumulator`], votes are taken unchecked until a certificate
/// fails to verify, and are checked one by one from then on. A ballot's lock
/// certificate is checked along with its signature: a vote naming a lock no
/// quorum certified is not counted, so it cannot raise the certificate's lock.
pub struct TimeoutTally<T: NodeType, Th> {
    membership: EpochMembership<T>,
    upgrade_lock: UpgradeLock<T>,
    ballots: Vec<TimeoutBallot<T>>,
    weight: U256,
    verify_votes: bool,
    _threshold: PhantomData<fn() -> Th>,
}

impl<T: NodeType, Th: Threshold<T> + Send + 'static> Tally<T> for TimeoutTally<T, Th> {
    type Vote = TimeoutBallot<T>;
    type Output = ValidCert<TimeoutCertificate3<T>>;

    fn new(m: EpochMembership<T>, l: UpgradeLock<T>) -> Self {
        Self {
            membership: m,
            upgrade_lock: l,
            ballots: Vec::new(),
            weight: U256::ZERO,
            verify_votes: false,
            _threshold: PhantomData,
        }
    }

    fn min_votes(&self) -> usize {
        let table: Vec<_> = self.membership.stake_table().collect();
        let largest = table
            .iter()
            .map(|peer| peer.stake_table_entry.stake())
            .max()
            .unwrap_or_default();
        if largest.is_zero() {
            return 1;
        }
        let nodes = table.len().max(1);
        let votes = Th::threshold(&self.membership).div_ceil(largest);
        usize::try_from(votes).unwrap_or(nodes).clamp(1, nodes)
    }

    fn has_signer(m: &EpochMembership<T>, signer: &T::SignatureKey) -> bool {
        m.stake(signer).is_some()
    }

    fn add(&mut self, ballot: TimeoutBallot<T>) -> Option<Self::Output> {
        if self.verify_votes && !self.is_valid(&ballot) {
            return None;
        }
        self.weight += self.stake(&ballot);
        self.ballots.push(ballot);
        if self.weight < Th::threshold(&self.membership) {
            return None;
        }
        match self.assemble() {
            Some(Ok(cert)) => Some(cert),
            Some(Err(err)) if !self.verify_votes => {
                warn!(%err, "invalid timeout certificate formed");
                self.recover()
            },
            Some(Err(err)) => {
                error!(%err, "timeout certificate of checked votes is invalid");
                None
            },
            None => None,
        }
    }
}

impl<T: NodeType, Th: Threshold<T>> TimeoutTally<T, Th> {
    fn stake(&self, ballot: &TimeoutBallot<T>) -> U256 {
        self.membership
            .stake(&ballot.vote().signing_key())
            .map(|peer| peer.stake_table_entry.stake())
            .unwrap_or_default()
    }

    /// The certificate of the ballots so far, checked.
    fn assemble(&self) -> Option<anytrace::Result<ValidCert<TimeoutCertificate3<T>>>> {
        let first = self.ballots.first()?;
        let view = first.vote().view_number();
        let epoch = first.vote().data.epoch;
        let latest = self
            .ballots
            .iter()
            .max_by_key(|ballot| ballot.vote().data.lock)
            .expect("there is a first ballot");
        let entries = StakeTableEntries::from_iter(self.membership.stake_table()).0;
        let threshold = Th::threshold(&self.membership);
        let votes = self.ballots.iter().map(|ballot| {
            (
                ballot.vote().data.lock,
                ballot.vote().signing_key(),
                ballot.vote().signature(),
            )
        });
        let result =
            TimeoutCertificate3::assemble(view, epoch, &entries, votes, latest.lock().cloned())
                .and_then(|cert| {
                    verify_timeout3(
                        &cert,
                        &entries,
                        threshold,
                        *self.membership.coordinator.epoch_height(),
                        &self.upgrade_lock,
                        &self.membership.coordinator,
                    )?;
                    Ok(cert)
                })
                .map(|cert| {
                    info!(%view, %epoch, lock = ?cert.lock(), "timeout certificate formed");
                    ValidCert::new(cert, epoch)
                });
        Some(result)
    }

    /// Drop the ballots that fail their checks and assemble the rest again.
    fn recover(&mut self) -> Option<ValidCert<TimeoutCertificate3<T>>> {
        self.verify_votes = true;
        let ballots = mem::take(&mut self.ballots);
        self.weight = U256::ZERO;
        for ballot in ballots {
            if self.is_valid(&ballot) {
                self.weight += self.stake(&ballot);
                self.ballots.push(ballot);
            }
        }
        if self.weight < Th::threshold(&self.membership) {
            return None;
        }
        match self.assemble()? {
            Ok(cert) => Some(cert),
            Err(err) => {
                error!(%err, "timeout certificate of checked votes is invalid");
                None
            },
        }
    }

    /// Check a ballot's signature and its lock certificate.
    fn is_valid(&self, ballot: &TimeoutBallot<T>) -> bool {
        let vote = ballot.vote();
        let Ok(data) =
            VersionedVoteData::new(vote.data.clone(), vote.view_number(), &self.upgrade_lock)
        else {
            return false;
        };
        if !vote
            .signing_key()
            .validate(&vote.signature(), data.commit().as_ref())
        {
            warn!(view = %vote.view_number(), signer = %vote.signing_key(), "invalid timeout vote");
            return false;
        }
        let Some(cert) = ballot.lock() else {
            return true;
        };
        if !cert
            .data
            .is_well_formed(*self.membership.coordinator.epoch_height())
        {
            return false;
        }
        let Ok(membership) = self
            .membership
            .coordinator
            .membership_for_epoch(cert.data.epoch)
        else {
            return false;
        };
        let entries = StakeTableEntries::from_iter(membership.stake_table()).0;
        let valid = verify_signatures(
            cert,
            &entries,
            membership.success_threshold(),
            &self.upgrade_lock,
        )
        .is_ok();
        if !valid {
            warn!(view = %vote.view_number(), signer = %vote.signing_key(), "timeout vote with an invalid lock certificate");
        }
        valid
    }
}

async fn run_tally<T, S>(
    mut rx: mpsc::UnboundedReceiver<S::Vote>,
    mut tally: S,
) -> Option<S::Output>
where
    T: NodeType,
    S: Tally<T>,
{
    let cap = tally.min_votes().max(1);
    let mut buf = Vec::with_capacity(cap);
    let mut need = cap;

    loop {
        while buf.len() < need {
            if rx.recv_many(&mut buf, cap).await == 0 {
                return None;
            }
        }
        need = 1;

        let hop = spawn_blocking(move || {
            let out = buf.drain(..).find_map(|v| tally.add(v));
            (tally, buf, out)
        });

        match hop.await {
            Ok((_, _, Some(o))) => return Some(o),
            Ok((t, b, None)) => (tally, buf) = (t, b),
            Err(e) if e.is_panic() => resume_unwind(e.into_panic()),
            Err(_) => return None,
        }
    }
}

/// What a tally is keyed by: the view and the epoch.
type Key = (ViewNumber, EpochNumber);

/// Collects votes per view and epoch and forms certificate(s) using the strategy `S`.
pub struct VoteCollector<T: NodeType, S: Tally<T>> {
    /// Tasks collecting votes and forming certificates.
    accumulators: JoinMap<Key, Option<S::Output>>,

    /// Where callers submit their votes.
    ballot_boxes: BTreeMap<Key, mpsc::UnboundedSender<S::Vote>>,

    /// Votes for epochs we have yet to resolve, deduplicated by signer.
    pending: BTreeMap<Key, HashMap<T::SignatureKey, S::Vote>>,

    /// The view and epoch pairs that formed a valid certificate already.
    completed: BTreeSet<Key>,

    /// The signers per view and epoch.
    signers: BTreeMap<Key, HashSet<T::SignatureKey>>,

    /// The GC threshold.
    lower_bound: ViewNumber,

    membership: EpochMembershipCoordinator<T>,

    upgrade_lock: UpgradeLock<T>,
}

impl<T: NodeType, S: Tally<T>> VoteCollector<T, S>
where
    <S as Tally<T>>::Vote: Ballot<Signer = T::SignatureKey>,
{
    pub fn new(mc: EpochMembershipCoordinator<T>, lock: UpgradeLock<T>) -> Self {
        Self {
            accumulators: JoinMap::new(),
            ballot_boxes: BTreeMap::new(),
            pending: BTreeMap::new(),
            signers: BTreeMap::new(),
            completed: BTreeSet::new(),
            membership: mc,
            upgrade_lock: lock,
            lower_bound: ViewNumber::genesis(),
        }
    }

    pub async fn next(&mut self) -> Option<S::Output> {
        loop {
            let (key, result) = self.accumulators.join_next().await?;
            let (view, epoch) = key;
            // However the task ended, it is gone from the `JoinMap` and will
            // not be joined again. A sender left behind in `ballot_boxes`
            // would keep taking votes for `key` that nothing ever reads.
            self.ballot_boxes.remove(&key);
            let cert = match result {
                Ok(cert) => cert,
                Err(err) => {
                    if err.is_panic() {
                        error!(%view, %epoch, %err, "vote collection task panic");
                    }
                    None
                },
            };
            match cert {
                Some(cert) if view >= self.lower_bound && self.completed.insert(key) => {
                    return Some(cert);
                },
                // `key` was retired by `mark_completed` before its tally was
                // joined. Nothing reopens it, so its signers stay for the
                // timeout diagnostics.
                _ if self.completed.contains(&key) => {},
                // No certificate came out of `key`, so a later vote may open a
                // new ballot box for it. Its accumulator starts empty, hence the
                // signers this one saw have to go too, or every one of them is
                // deduplicated away from a tally that never counted them.
                _ => {
                    self.signers.remove(&key);
                },
            }
        }
    }

    pub fn accumulate_vote(&mut self, vote: S::Vote) {
        let view = vote.view();

        let Some(epoch) = vote.epoch() else {
            // A vote without an epoch names no committee, so it can never be tallied.
            return;
        };

        let key = (view, epoch);

        if view < self.lower_bound || self.completed.contains(&key) {
            return;
        }

        let Ok(membership) = self.membership.membership_for_epoch(Some(epoch)) else {
            // The epoch's stake table may still arrive by catchup, so keep
            // the vote for `retry_pending_votes`.
            self.pending
                .entry(key)
                .or_default()
                .insert(vote.signer(), vote);
            return;
        };

        if !S::has_signer(&membership, &vote.signer()) {
            return;
        }

        // Check that we have not received a vote from this signer already.
        if !self.signers.entry(key).or_default().insert(vote.signer()) {
            return;
        }

        if let Some(tx) = self.ballot_boxes.get(&key) {
            let _ = tx.send(vote);
            return;
        }

        let (tx, rx) = mpsc::unbounded_channel();

        let _ = tx.send(vote);
        self.ballot_boxes.insert(key, tx);

        let tally = S::new(membership, self.upgrade_lock.clone());
        self.accumulators.spawn(key, run_tally::<T, S>(rx, tally));
    }

    /// Record that `view`'s certificate in `epoch` was obtained by other means,
    /// e.g. from the network, so the tally for it stops.
    ///
    /// Drops the ballot box and buffered votes and aborts the tally. A tally
    /// mid-way through a blocking hop finishes that hop and has its
    /// certificate discarded by [`Self::next`]. `signers` is left in place so
    /// the timeout diagnostics keep what the tally saw.
    pub fn mark_completed(&mut self, view: ViewNumber, epoch: EpochNumber) {
        if view < self.lower_bound {
            return;
        }
        let key = (view, epoch);
        self.completed.insert(key);
        self.ballot_boxes.remove(&key);
        self.pending.remove(&key);
        self.accumulators.abort(&key);
    }

    pub fn retry_pending_votes(&mut self) {
        for vote in mem::take(&mut self.pending)
            .into_values()
            .flat_map(|votes| votes.into_values())
        {
            self.accumulate_vote(vote)
        }
    }

    pub fn gc(&mut self, view: ViewNumber) {
        let floor = (view, EpochNumber::new(0));
        self.ballot_boxes = self.ballot_boxes.split_off(&floor);
        self.completed = self.completed.split_off(&floor);
        self.pending = self.pending.split_off(&floor);
        self.signers = self.signers.split_off(&floor);
        self.lower_bound = view;
    }
}

impl<T, V, Th> VoteCollector<T, SimpleTally<T, V, Th>>
where
    T: NodeType,
    V: Vote<T> + Send + 'static,
    V::Commitment: 'static,
    Th: Threshold<T> + Send + 'static,
    Cert<T, V, Th>:
        Certificate<T, V::Commitment, Voteable = V::Commitment> + HasEpoch + Send + 'static,
{
    /// What every epoch that voted in `view` has collected.
    ///
    /// Each entry counts the unique signers routed to `view`'s tally under
    /// that epoch and, where the epoch still resolves, sums their stake
    /// against its cert threshold. Votes name their own epoch, so a view can
    /// hold more than one entry; that means its votes are split across
    /// committees and neither tally need reach its threshold. Looks up each
    /// signer's stake on demand, only intended for rare paths like timeout
    /// diagnostics.
    ///
    /// An epoch that has signers always yields an entry, so an empty result
    /// means no vote was received for `view` and nothing weaker.
    pub fn stats(&self, view: ViewNumber) -> Vec<(EpochNumber, VoteStats)> {
        let epochs = (view, EpochNumber::new(0))..(view + 1, EpochNumber::new(0));
        self.signers
            .range(epochs)
            .filter(|(_, signers)| !signers.is_empty())
            .map(|(&(_, epoch), signers)| {
                let stats = VoteStats {
                    signers: signers.len(),
                    stake: self.membership.membership_for_epoch(Some(epoch)).ok().map(
                        |membership| {
                            let stake = signers
                                .iter()
                                .filter_map(|signer| {
                                    Cert::<T, V, Th>::stake_table_entry(&membership, signer)
                                })
                                .map(|peer| peer.stake_table_entry.stake())
                                .sum();
                            (stake, Cert::<T, V, Th>::threshold(&membership))
                        },
                    ),
                };
                (epoch, stats)
            })
            .collect()
    }
}

/// What a single view and epoch has collected.
///
/// Used by diagnostics (e.g. timeout logging) to show how close a view came
/// to forming a cert.
#[derive(Clone, Copy, Debug)]
pub struct VoteStats {
    /// Unique signers routed to this view and epoch.
    pub signers: usize,

    /// Their combined stake and the epoch's cert threshold, in that order.
    ///
    /// Absent once the epoch's stake table is no longer available: the
    /// signers were counted while it resolved, the stake cannot be.
    pub stake: Option<(U256, U256)>,
}

#[cfg(test)]
mod tests {
    use std::{fmt::Debug, marker::PhantomData, time::Duration};

    use committable::Committable;
    use hotshot::types::BLSPubKey;
    use hotshot_example_types::node_types::TestTypes;
    use hotshot_testing::{node_stake::TestNodeStakes, test_builder::gen_node_lists};
    use hotshot_types::{
        data::{EpochNumber, ViewNumber},
        epoch_membership::EpochMembership,
        message::UpgradeLock,
        simple_certificate::{
            Certificate1, SuccessThreshold, Threshold, TimeoutCertificate2, TimeoutCertificate3,
            TimeoutEvidence, UpgradeCertificate,
        },
        simple_vote::{
            HasEpoch, LockView, QuorumData2, QuorumVote2, SimpleVote, TimeoutData2, TimeoutData3,
            TimeoutVote2, UpgradeProposalData, VersionedVoteData, Vote2Data,
        },
        stake_table::StakeTableEntries,
        traits::{node_implementation::NodeType, signature_key::SignatureKey},
        vote::{Certificate, HasViewNumber, Vote, VoteAccumulator},
    };
    use tokio::{sync::mpsc, time::timeout};
    use versions::{NEW_PROTOCOL_VERSION, TIMEOUT_EPOCH_VERSION, Upgrade};

    use super::{Ballot, Cert, SimpleTally, TimeoutTally, VoteCollector};
    use crate::{
        helpers::{test_timeout_epoch_lock, test_upgrade_lock},
        message::{TimeoutBallot, TimeoutVoteMessage3, UpgradeVoteMessage, Vote2},
        tests::common::utils::mock_membership,
    };

    /// Number of test validators.
    const NUM_NODES: u64 = 10;
    /// Threshold for SuccessThreshold with 10 nodes of stake 1: (10*2)/3 + 1 = 7.
    const THRESHOLD: u64 = 7;

    /// How long to wait for expected certificates before failing.
    const CERT_TIMEOUT: Duration = Duration::from_millis(100);
    /// How long to wait for a timeout certificate that checks its votes one
    /// by one, lock certificates included, which is slow on a loaded machine.
    const RECOVERY_TIMEOUT: Duration = Duration::from_secs(10);
    /// How long to wait to confirm no certificate is produced (failure tests).
    const NO_CERT_TIMEOUT: Duration = Duration::from_millis(500);

    /// Create a signed QuorumVote2 (used for Certificate1 accumulation).
    fn make_quorum_vote(
        node_index: u64,
        view: ViewNumber,
        epoch: EpochNumber,
    ) -> QuorumVote2<TestTypes> {
        let (pub_key, priv_key) = BLSPubKey::generated_from_seed_indexed([0u8; 32], node_index);
        let data = QuorumData2 {
            leaf_commit: committable::RawCommitmentBuilder::new("FakeLeaf")
                .u64(42)
                .finalize(),
            epoch: Some(epoch),
            block_number: Some(1),
        };
        SimpleVote::create_signed_vote(data, view, &pub_key, &priv_key, &test_upgrade_lock())
            .expect("Failed to sign vote")
    }

    fn vote_2_data() -> Vote2Data<TestTypes> {
        Vote2Data {
            leaf_commit: committable::RawCommitmentBuilder::new("FakeLeaf")
                .u64(42)
                .finalize(),
            epoch: EpochNumber::genesis(),
            block_number: 1,
        }
    }

    /// Create a signed Vote2 (used for Certificate2 accumulation).
    fn make_vote2(node_index: u64, view: ViewNumber) -> Vote2<TestTypes> {
        let (pub_key, priv_key) = BLSPubKey::generated_from_seed_indexed([0u8; 32], node_index);
        let data = vote_2_data();
        SimpleVote::create_signed_vote(data, view, &pub_key, &priv_key, &test_upgrade_lock())
            .expect("Failed to sign vote")
    }

    /// Create a Vote2 with an invalid signature (signed by a different key than claimed).
    fn make_invalid_vote2(node_index: u64, view: ViewNumber) -> Vote2<TestTypes> {
        let (pub_key, _) = BLSPubKey::generated_from_seed_indexed([0u8; 32], node_index);
        // Sign with a completely different key
        let (_, wrong_priv_key) = BLSPubKey::generated_from_seed_indexed([1u8; 32], node_index);
        let data = vote_2_data();
        let commit =
            VersionedVoteData::<TestTypes, _>::new(data.clone(), view, &test_upgrade_lock())
                .unwrap()
                .commit();
        let bad_sig = BLSPubKey::sign(&wrong_priv_key, commit.as_ref()).unwrap();
        SimpleVote {
            signature: (pub_key, bad_sig),
            data,
            view_number: view,
        }
    }

    /// Spawn a VoteCollectionTask and return:
    /// - vote sender
    /// - cert notification channel (receives (view, cert) when a certificate is formed)
    /// - task JoinHandle (abort this to clean up)
    fn setup_cert1_task()
    -> VoteCollector<TestTypes, SimpleTally<TestTypes, QuorumVote2<TestTypes>, SuccessThreshold>>
    {
        setup_task::<QuorumVote2<TestTypes>, SuccessThreshold>()
    }

    fn setup_cert2_task()
    -> VoteCollector<TestTypes, SimpleTally<TestTypes, Vote2<TestTypes>, SuccessThreshold>> {
        setup_task::<Vote2<TestTypes>, SuccessThreshold>()
    }

    /// Spawn a VoteCollectionTask for Certificate2.
    fn setup_task<
        V: Ballot<Signer = <TestTypes as NodeType>::SignatureKey>
            + Vote<TestTypes>
            + HasEpoch
            + Send
            + Sync
            + 'static,
        Th: Threshold<TestTypes> + Send + 'static,
    >() -> VoteCollector<TestTypes, SimpleTally<TestTypes, V, Th>>
    where
        Cert<TestTypes, V, Th>: Certificate<TestTypes, V::Commitment, Voteable = V::Commitment>
            + HasEpoch
            + Send
            + 'static,
    {
        let membership = mock_membership();
        VoteCollector::new(membership, test_upgrade_lock())
    }

    /// Wait for exactly `expected` certificates, then abort the task.
    async fn _collect_certs<T: std::fmt::Debug>(
        cert_rx: &mut mpsc::Receiver<T>,
        expected: usize,
    ) -> Vec<T> {
        let mut results = Vec::new();
        for _ in 0..expected {
            let cert = tokio::time::timeout(CERT_TIMEOUT, cert_rx.recv())
                .await
                .expect("Timed out waiting for certificate")
                .expect("Cert channel closed unexpectedly");
            results.push(cert);
        }
        results
    }

    /// Confirm no certificates are produced within the timeout.
    ///
    /// Requires the collector to have a tally running or one already
    /// certified: `next` yields `None` the moment there is no task at all, so
    /// without this a test that opened no tally would pass without waiting for
    /// anything.
    async fn assert_no_certs<
        V: Ballot<Signer = <TestTypes as NodeType>::SignatureKey>
            + Vote<TestTypes>
            + HasEpoch
            + Send
            + Sync
            + 'static,
        Th: Threshold<TestTypes> + Send + 'static,
    >(
        task: &mut VoteCollector<TestTypes, SimpleTally<TestTypes, V, Th>>,
    ) where
        Cert<TestTypes, V, Th>: Certificate<TestTypes, V::Commitment, Voteable = V::Commitment>
            + HasEpoch
            + Debug
            + Send
            + 'static,
    {
        assert!(
            !task.ballot_boxes.is_empty() || !task.completed.is_empty(),
            "no tally has run, so there is nothing that could produce a certificate"
        );
        let result = tokio::time::timeout(NO_CERT_TIMEOUT, task.next()).await;
        match result {
            Err(_) => { /* timeout — good, no cert produced */ },
            Ok(None) => { /* good, no cert produced */ },
            Ok(Some(cert)) => panic!("Expected no certificate but got one: {cert:?}"),
        }
    }

    /// Verify that a certificate's data commitment matches `expected_data` and that
    /// the aggregate signature is valid against the stake table.
    fn verify_cert<C, D>(cert: &C, expected_data: &D, membership: &EpochMembership<TestTypes>)
    where
        D: Committable,
        C: Certificate<TestTypes, D, Voteable = D>,
    {
        // Data commitment must match the vote data that produced the cert.
        assert_eq!(
            cert.data().commit(),
            expected_data.commit(),
            "Certificate data commitment does not match expected vote data"
        );

        // Aggregate signature must be valid against the stake table.
        let stake_table = C::stake_table(membership);
        let stake_table_entries = StakeTableEntries::<TestTypes>::from(stake_table).0;
        let threshold = C::threshold(membership);
        cert.is_valid_cert(&stake_table_entries, threshold, &test_upgrade_lock())
            .expect("Certificate signature validation failed");
    }

    // ==================== Certificate1 (QuorumVote2) happy path ====================

    /// Sending enough QuorumVote2s for a single view produces a valid Certificate1
    /// whose data commitment matches the votes.
    #[tokio::test]
    async fn test_cert1_single_view_happy_path() {
        let mut task = setup_cert1_task();
        let view = ViewNumber::new(1);
        let epoch = EpochNumber::genesis();
        let expected_data = QuorumData2 {
            leaf_commit: committable::RawCommitmentBuilder::new("FakeLeaf")
                .u64(42)
                .finalize(),
            epoch: Some(epoch),
            block_number: Some(1),
        };

        for i in 0..THRESHOLD {
            task.accumulate_vote(make_quorum_vote(i, view, epoch));
        }

        let cert = timeout(CERT_TIMEOUT, task.next()).await.unwrap().unwrap();
        assert_eq!(cert.view_number(), view);

        let membership = mock_membership();
        let epoch_membership = membership.membership_for_epoch(Some(epoch)).unwrap();
        verify_cert(cert.cert(), &expected_data, &epoch_membership);
    }

    /// Sending votes for multiple views produces a valid certificate for each view,
    /// each with data commitment matching the votes.
    #[tokio::test]
    async fn test_cert1_multiple_views_parallel() {
        let mut task = setup_cert1_task();
        let epoch = EpochNumber::genesis();
        let expected_data = QuorumData2 {
            leaf_commit: committable::RawCommitmentBuilder::new("FakeLeaf")
                .u64(42)
                .finalize(),
            epoch: Some(epoch),
            block_number: Some(1),
        };

        let views = [ViewNumber::new(1), ViewNumber::new(2), ViewNumber::new(3)];

        // Interleave votes across views
        for i in 0..THRESHOLD {
            for &view in &views {
                task.accumulate_vote(make_quorum_vote(i, view, epoch));
            }
        }
        let mut certs = Vec::new();
        for _ in 0..views.len() {
            certs.push(timeout(CERT_TIMEOUT, task.next()).await.unwrap().unwrap());
        }
        assert_eq!(
            certs.len(),
            views.len(),
            "Expected one Certificate1 per view"
        );
        let mut cert_views: Vec<_> = certs.iter().map(|c| c.view_number()).collect();
        cert_views.sort();
        assert_eq!(cert_views, views.to_vec());

        let membership = mock_membership();
        let epoch_membership = membership.membership_for_epoch(Some(epoch)).unwrap();
        for cert in &certs {
            verify_cert(cert.cert(), &expected_data, &epoch_membership);
        }
    }

    // ==================== Certificate2 (Vote2) happy path ====================

    /// Sending enough Vote2s for a single view produces a valid Certificate2
    /// whose data commitment matches the votes.
    #[tokio::test]
    async fn test_cert2_single_view_happy_path() {
        let mut task = setup_cert2_task();
        let view = ViewNumber::new(1);
        let epoch = EpochNumber::genesis();
        let expected_data = vote_2_data();

        for i in 0..THRESHOLD {
            task.accumulate_vote(make_vote2(i, view));
        }

        let cert = timeout(CERT_TIMEOUT, task.next()).await.unwrap().unwrap();
        assert_eq!(cert.view_number(), view);

        let membership = mock_membership();
        let epoch_membership = membership.membership_for_epoch(Some(epoch)).unwrap();
        verify_cert(cert.cert(), &expected_data, &epoch_membership);
    }

    /// Sending votes for multiple views in parallel produces valid certificates for each,
    /// each with data commitment matching the votes.
    #[tokio::test]
    async fn test_cert2_multiple_views_parallel() {
        let mut task = setup_cert2_task();
        let epoch = EpochNumber::genesis();
        let expected_data = vote_2_data();

        let views = [ViewNumber::new(5), ViewNumber::new(6), ViewNumber::new(7)];

        for i in 0..THRESHOLD {
            for &view in &views {
                task.accumulate_vote(make_vote2(i, view));
            }
        }

        let mut certs = Vec::new();
        for _ in 0..views.len() {
            certs.push(timeout(CERT_TIMEOUT, task.next()).await.unwrap().unwrap());
        }
        assert_eq!(
            certs.len(),
            views.len(),
            "Expected one Certificate2 per view"
        );
        let mut cert_views: Vec<_> = certs.iter().map(|c| c.view_number()).collect();
        cert_views.sort();
        assert_eq!(cert_views, views.to_vec());

        let membership = mock_membership();
        let epoch_membership = membership.membership_for_epoch(Some(epoch)).unwrap();
        for cert in &certs {
            verify_cert(cert.cert(), &expected_data, &epoch_membership);
        }
    }

    // ==================== Certificate1 failure cases ====================

    /// Fewer than threshold votes do not produce a certificate.
    #[tokio::test]
    async fn test_cert1_below_threshold_no_certificate() {
        let mut task = setup_cert1_task();
        let view = ViewNumber::new(1);
        let epoch = EpochNumber::genesis();

        for i in 0..(THRESHOLD - 1) {
            task.accumulate_vote(make_quorum_vote(i, view, epoch));
        }

        assert_no_certs(&mut task).await;
    }

    /// Duplicate votes from the same signer do not count toward threshold.
    #[tokio::test]
    async fn test_cert1_duplicate_votes_ignored() {
        let mut task = setup_cert1_task();
        let view = ViewNumber::new(1);
        let epoch = EpochNumber::genesis();

        // Send 6 unique votes (below threshold of 7)
        for i in 0..6 {
            task.accumulate_vote(make_quorum_vote(i, view, epoch));
        }
        // Send duplicates of node 0 — should not push us over threshold
        for _ in 0..5 {
            task.accumulate_vote(make_quorum_vote(0, view, epoch));
        }

        assert_no_certs(&mut task).await;
    }

    // ==================== Certificate2 failure cases ====================

    /// Fewer than threshold Vote2s do not produce a Certificate2.
    #[tokio::test]
    async fn test_cert2_below_threshold_no_certificate() {
        let mut task = setup_cert2_task();
        let view = ViewNumber::new(1);

        for i in 0..(THRESHOLD - 1) {
            task.accumulate_vote(make_vote2(i, view));
        }

        assert_no_certs(&mut task).await;
    }

    /// Duplicate Vote2s from the same signer do not count toward threshold.
    #[tokio::test]
    async fn test_cert2_duplicate_votes_ignored() {
        let mut task = setup_cert2_task();
        let view = ViewNumber::new(1);

        // Send 6 unique votes (below threshold of 7)
        for i in 0..6 {
            task.accumulate_vote(make_vote2(i, view));
        }
        // Repeat node 0 votes — should not reach threshold
        for _ in 0..5 {
            task.accumulate_vote(make_vote2(0, view));
        }

        assert_no_certs(&mut task).await;
    }

    /// Votes with invalid signatures are rejected and do not count.
    #[tokio::test]
    async fn test_cert2_invalid_signature_rejected() {
        let mut task = setup_cert2_task();
        let view = ViewNumber::new(1);

        // Send 6 valid votes (below threshold)
        for i in 0..6 {
            task.accumulate_vote(make_vote2(i, view));
        }
        // Send invalid-signature votes — should be rejected, not reaching threshold
        for i in 6..NUM_NODES {
            task.accumulate_vote(make_invalid_vote2(i, view));
        }

        assert_no_certs(&mut task).await;
    }

    /// Votes with invalid signatures are rejected and do not count.
    #[tokio::test]
    async fn test_cert2_invalid_signature_recovery() {
        let mut task = setup_cert2_task();
        let view = ViewNumber::new(1);
        let epoch = EpochNumber::genesis();

        // Send 6 valid votes (below threshold)
        for i in 0..6 {
            task.accumulate_vote(make_vote2(i, view));
        }
        // Send invalid-signature votes — should be rejected, not reaching threshold
        for i in 6..8 {
            task.accumulate_vote(make_invalid_vote2(i, view));
        }
        assert_no_certs(&mut task).await;

        task.accumulate_vote(make_vote2(9, view));

        let cert = timeout(CERT_TIMEOUT, task.next()).await.unwrap().unwrap();
        assert_no_certs(&mut task).await;
        let membership = mock_membership();
        let epoch_membership = membership.membership_for_epoch(Some(epoch)).unwrap();
        verify_cert(cert.cert(), &vote_2_data(), &epoch_membership);
    }

    /// A timeout certificate for the genesis view is signature-checked like any
    /// other.
    ///
    /// `is_valid_cert` passes every certificate at the genesis view, so a vote
    /// with a bad signature would end up in the certificate there, and the
    /// recovery that drops such votes would never run.
    #[tokio::test]
    async fn test_genesis_timeout_invalid_signature_recovery() {
        let lock = test_timeout_epoch_lock();
        let mut task = VoteCollector::<TestTypes, TimeoutTally<TestTypes, SuccessThreshold>>::new(
            mock_membership(),
            lock.clone(),
        );
        let view = ViewNumber::genesis();
        let epoch = EpochNumber::genesis();
        let vote = |node_index, signer_seed| {
            let (pub_key, _) = BLSPubKey::generated_from_seed_indexed([0u8; 32], node_index);
            let (_, priv_key) = BLSPubKey::generated_from_seed_indexed(signer_seed, node_index);
            let data = TimeoutData3 {
                view,
                epoch,
                lock: None,
            };
            let commit = VersionedVoteData::<TestTypes, _>::new(data.clone(), view, &lock)
                .unwrap()
                .commit();
            TimeoutVoteMessage3 {
                signer: pub_key,
                signature: BLSPubKey::sign(&priv_key, commit.as_ref()).unwrap(),
                view,
                epoch,
                lock: None,
                evidence: None,
            }
            .ballot()
            .unwrap()
        };

        for i in 0..6 {
            task.accumulate_vote(vote(i, [0u8; 32]));
        }
        for i in 6..8 {
            task.accumulate_vote(vote(i, [1u8; 32]));
        }
        // Six valid votes are one short; the two invalid ones are dropped.
        assert!(timeout(NO_CERT_TIMEOUT, task.next()).await.is_err());

        task.accumulate_vote(vote(9, [0u8; 32]));
        let cert = timeout(RECOVERY_TIMEOUT, task.next())
            .await
            .unwrap()
            .unwrap();
        let membership = mock_membership().membership_for_epoch(Some(epoch)).unwrap();
        let entries = StakeTableEntries::<TestTypes>::from_iter(membership.stake_table()).0;
        cert.check_signatures(&entries, membership.success_threshold(), &lock)
            .expect("the certificate carries only valid signatures");
        assert!(cert.earlier.is_empty());
    }

    /// Timeout votes naming different locks are counted together, and the
    /// certificate carries the latest of their locks.
    #[tokio::test]
    async fn timeout_votes_with_different_locks_form_one_certificate() {
        let lock = test_timeout_epoch_lock();
        let coordinator = mock_membership();
        let mut task = VoteCollector::<TestTypes, TimeoutTally<TestTypes, SuccessThreshold>>::new(
            coordinator.clone(),
            lock.clone(),
        );
        let epoch = EpochNumber::genesis();
        let membership = coordinator.membership_for_epoch(Some(epoch)).unwrap();
        let view = ViewNumber::new(5);
        let older = lock_cert(ViewNumber::new(2), epoch, &membership, &lock);
        let newer = lock_cert(ViewNumber::new(3), epoch, &membership, &lock);
        let ballot = |node_index, cert: &Option<Certificate1<TestTypes>>| {
            let (pub_key, priv_key) = BLSPubKey::generated_from_seed_indexed([0u8; 32], node_index);
            TimeoutBallot::sign(view, epoch, cert.clone(), &pub_key, &priv_key, &lock).unwrap()
        };
        for i in 0..3 {
            task.accumulate_vote(ballot(i, &None));
        }
        for i in 3..5 {
            task.accumulate_vote(ballot(i, &Some(older.clone())));
        }
        for i in 5..7 {
            task.accumulate_vote(ballot(i, &Some(newer.clone())));
        }
        let cert = timeout(RECOVERY_TIMEOUT, task.next())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(cert.earlier.len(), 2);
        assert_eq!(cert.lock(), Some(LockView::of(&newer)));
        let entries = StakeTableEntries::<TestTypes>::from_iter(membership.stake_table()).0;
        cert.check_signatures(&entries, membership.success_threshold(), &lock)
            .expect("valid grouped certificate");
    }

    /// A vote naming a lock its certificate does not show is not counted, so
    /// it cannot raise the certificate's lock.
    #[tokio::test]
    async fn timeout_vote_with_a_forged_lock_is_dropped() {
        let lock = test_timeout_epoch_lock();
        let coordinator = mock_membership();
        let mut task = VoteCollector::<TestTypes, TimeoutTally<TestTypes, SuccessThreshold>>::new(
            coordinator.clone(),
            lock.clone(),
        );
        let epoch = EpochNumber::genesis();
        let membership = coordinator.membership_for_epoch(Some(epoch)).unwrap();
        let view = ViewNumber::new(5);
        let honest = lock_cert(ViewNumber::new(2), epoch, &membership, &lock);
        // A "certificate" at a later view with no signatures at all.
        let mut forged = honest.clone();
        forged.view_number = ViewNumber::new(4);
        forged.signatures = None;
        let ballot = |node_index, cert: &Certificate1<TestTypes>| {
            let (pub_key, priv_key) = BLSPubKey::generated_from_seed_indexed([0u8; 32], node_index);
            TimeoutBallot::sign(view, epoch, Some(cert.clone()), &pub_key, &priv_key, &lock)
                .unwrap()
        };
        task.accumulate_vote(ballot(0, &forged));
        for i in 1..8 {
            task.accumulate_vote(ballot(i, &honest));
        }
        let cert = timeout(RECOVERY_TIMEOUT, task.next())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(cert.lock(), Some(LockView::of(&honest)));
    }

    /// A `Certificate1` at `view` signed by every member.
    fn lock_cert(
        view: ViewNumber,
        epoch: EpochNumber,
        membership: &EpochMembership<TestTypes>,
        lock: &UpgradeLock<TestTypes>,
    ) -> Certificate1<TestTypes> {
        let mut accumulator =
            VoteAccumulator::<TestTypes, QuorumVote2<TestTypes>, Certificate1<TestTypes>>::new(
                lock.clone(),
            );
        for i in 0..NUM_NODES {
            let (pub_key, priv_key) = BLSPubKey::generated_from_seed_indexed([0u8; 32], i);
            let data = QuorumData2 {
                leaf_commit: committable::RawCommitmentBuilder::new("FakeLeaf")
                    .u64(*view)
                    .finalize(),
                epoch: Some(epoch),
                block_number: Some(*view),
            };
            let vote = SimpleVote::create_signed_vote(data, view, &pub_key, &priv_key, lock)
                .expect("failed to sign quorum vote");
            if let Some(cert) = accumulator.accumulate(&vote, membership.clone()) {
                return cert;
            }
        }
        panic!("threshold reached without forming a certificate");
    }

    /// Channel closed before threshold means no certificate is produced.
    #[tokio::test]
    async fn test_cert2_channel_closed_early() {
        let mut task = setup_cert2_task();
        let view = ViewNumber::new(1);

        for i in 0..3 {
            task.accumulate_vote(make_vote2(i, view));
        }
        assert_no_certs(&mut task).await;
    }

    // ==================== Mixed / advanced scenarios ====================

    /// Only the view that reaches threshold gets a certificate; others don't.
    #[tokio::test]
    async fn test_cert2_partial_views_only_complete_one_certifies() {
        let mut task = setup_cert2_task();

        let complete_view = ViewNumber::new(1);
        let partial_view = ViewNumber::new(2);

        // Send threshold votes for the complete view
        for i in 0..THRESHOLD {
            task.accumulate_vote(make_vote2(i, complete_view));
        }

        // Send fewer than threshold for the partial view
        for i in 0..3 {
            task.accumulate_vote(make_vote2(i, partial_view));
        }

        // Wait for the one expected certificate
        let cert = timeout(CERT_TIMEOUT, task.next()).await.unwrap().unwrap();
        assert_no_certs(&mut task).await;
        assert_eq!(cert.view_number(), complete_view);
    }

    /// Extra votes beyond threshold for the same view do not produce a second certificate.
    #[tokio::test]
    async fn test_cert2_extra_votes_after_threshold_no_duplicate_cert() {
        let mut task = setup_cert2_task();
        let view = ViewNumber::new(1);

        // Send all 10 votes (more than threshold of 7)
        for i in 0..NUM_NODES {
            task.accumulate_vote(make_vote2(i, view));
        }

        // Should get exactly one cert, then confirm no more arrive
        let cert = timeout(CERT_TIMEOUT, task.next()).await.unwrap().unwrap();
        assert_eq!(cert.view_number(), view);

        // Confirm no second certificate
        assert_no_certs(&mut task).await;
    }

    /// Votes for different data commitments on the same view do not combine.
    #[tokio::test]
    async fn test_cert2_conflicting_data_same_view_no_certificate() {
        let mut task = setup_cert2_task();
        let view = ViewNumber::new(1);

        // Send 6 votes for one leaf commitment
        for i in 0..6 {
            task.accumulate_vote(make_vote2(i, view));
        }

        // Send 4 votes for a different leaf commitment
        for i in 6..NUM_NODES {
            let (pub_key, priv_key) = BLSPubKey::generated_from_seed_indexed([0u8; 32], i);
            let data = Vote2Data {
                leaf_commit: committable::RawCommitmentBuilder::new("FakeLeaf")
                    //different leaf commitment
                    .u64(1000)
                    .finalize(),
                epoch: EpochNumber::genesis(),
                block_number: 1,
            };
            let vote = SimpleVote::create_signed_vote(
                data,
                view,
                &pub_key,
                &priv_key,
                &test_upgrade_lock(),
            )
            .expect("Failed to sign vote");
            task.accumulate_vote(vote);
        }
        assert_no_certs(&mut task).await;
    }

    /// Collector over a membership where node 9 is removed from the quorum
    /// committee starting at epoch 3 (9 members of stake 1, threshold 7).
    fn setup_cert1_task_with_removed_node()
    -> VoteCollector<TestTypes, SimpleTally<TestTypes, QuorumVote2<TestTypes>, SuccessThreshold>>
    {
        let membership = mock_membership();
        let committee = gen_node_lists::<TestTypes>(9, 9, &TestNodeStakes::default()).0;
        membership
            .membership()
            .add_quorum_committee(EpochNumber::new(3), committee);
        membership
            .membership()
            .register_epoch(EpochNumber::new(3), [0u8; 32]);
        VoteCollector::new(membership, test_upgrade_lock())
    }

    /// A vote from a validator that was removed from the quorum committee in
    /// a later epoch does not count toward that epoch's certificate.
    #[tokio::test]
    async fn test_vote_from_removed_validator_ignored() {
        let mut task = setup_cert1_task_with_removed_node();
        let view = ViewNumber::new(21);
        let epoch = EpochNumber::new(3);

        for i in 0..6 {
            task.accumulate_vote(make_quorum_vote(i, view, epoch));
        }
        task.accumulate_vote(make_quorum_vote(9, view, epoch));
        assert_no_certs(&mut task).await;

        task.accumulate_vote(make_quorum_vote(6, view, epoch));
        let cert = timeout(CERT_TIMEOUT, task.next()).await.unwrap().unwrap();
        assert_eq!(cert.view_number(), view);
    }

    /// A vote's epoch is chosen by its sender and bound to nothing else in
    /// the vote, so the first vote of a view must not decide which committee
    /// the rest of it is counted against.
    ///
    /// Node 9 left the committee in epoch 3, so a view opened by a vote
    /// naming that epoch would drop node 9's epoch-2 vote and leave the
    /// remaining six short of the threshold.
    #[tokio::test]
    async fn test_forged_epoch_vote_does_not_pin_the_committee() {
        let mut task = setup_cert1_task_with_removed_node();
        let view = ViewNumber::new(11);
        let voted = EpochNumber::new(2);
        let forged = EpochNumber::new(3);

        // Node 0 opens the view naming an epoch nobody voted in.
        task.accumulate_vote(make_quorum_vote(0, view, forged));

        // Exactly the threshold votes in the epoch of the view, node 9 among
        // them.
        for i in 3..NUM_NODES {
            task.accumulate_vote(make_quorum_vote(i, view, voted));
        }

        let cert = timeout(CERT_TIMEOUT, task.next()).await.unwrap().unwrap();
        assert_eq!(cert.view_number(), view);
        assert_eq!(cert.epoch(), voted);
        assert_eq!(cert.data.epoch, Some(voted));
        // The forged epoch holds a single vote and certifies nothing.
        assert_no_certs(&mut task).await;
    }

    /// Two epochs voting in one view are tallied apart, each against its own
    /// committee, and both reach a certificate.
    #[tokio::test]
    async fn test_votes_in_two_epochs_certify_separately() {
        let mut task = setup_cert1_task_with_removed_node();
        let view = ViewNumber::new(11);
        let epochs = [EpochNumber::new(2), EpochNumber::new(3)];

        for &epoch in &epochs {
            for i in 0..THRESHOLD {
                task.accumulate_vote(make_quorum_vote(i, view, epoch));
            }
        }

        let mut certified = Vec::new();
        for _ in 0..epochs.len() {
            let cert = timeout(CERT_TIMEOUT, task.next()).await.unwrap().unwrap();
            assert_eq!(cert.view_number(), view);
            certified.push(cert.epoch());
        }
        certified.sort();
        assert_eq!(certified, epochs.to_vec());
    }

    /// GC drops every epoch of the views it collects, including epoch 0,
    /// which sorts below the epoch numbers a stake table can have, and keeps
    /// every epoch of the views it does not.
    #[tokio::test]
    async fn test_gc_clears_all_epochs_of_a_view() {
        let mut task = setup_cert1_task();
        let view = ViewNumber::new(1);

        // A vote whose epoch resolves opens a ballot box; one whose epoch
        // never will is buffered instead.
        for v in [view, view + 1] {
            task.accumulate_vote(make_quorum_vote(0, v, EpochNumber::genesis()));
            task.accumulate_vote(make_quorum_vote(1, v, EpochNumber::new(0)));
        }
        assert_eq!(task.signers.len(), 2);
        assert_eq!(task.pending.len(), 2);

        task.gc(view + 1);

        let kept = [(view + 1, EpochNumber::genesis())];
        assert_eq!(task.ballot_boxes.keys().copied().collect::<Vec<_>>(), kept);
        assert_eq!(task.signers.keys().copied().collect::<Vec<_>>(), kept);
        assert_eq!(
            task.pending.keys().copied().collect::<Vec<_>>(),
            [(view + 1, EpochNumber::new(0))]
        );
        assert!(task.completed.is_empty());
    }

    /// A view waiting for votes must not hold a thread from the blocking
    /// pool, which the process shares with VID and certificate verification.
    ///
    /// This runtime has a single blocking thread, so one tally parked in a
    /// receive loop would starve every other view.
    #[test]
    fn test_waiting_tallies_do_not_hold_blocking_threads() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .max_blocking_threads(1)
            .enable_all()
            .build()
            .expect("runtime");

        rt.block_on(async {
            let mut task = setup_cert1_task();
            let epoch = EpochNumber::genesis();

            // Open a tally for each of these views and leave every one of
            // them short of the threshold.
            for view in 1..=8 {
                task.accumulate_vote(make_quorum_vote(0, ViewNumber::new(view), epoch));
            }

            let view = ViewNumber::new(9);
            for i in 0..THRESHOLD {
                task.accumulate_vote(make_quorum_vote(i, view, epoch));
            }

            let cert = timeout(Duration::from_secs(5), task.next())
                .await
                .expect("a tally runs while every other view is still waiting")
                .expect("certificate");
            assert_eq!(cert.view_number(), view);
        });
    }

    /// A vote from outside the epoch's committee can never be tallied, so it
    /// must not create collector state for its view and epoch either.
    #[tokio::test]
    async fn test_vote_from_outside_the_committee_opens_nothing() {
        let mut task = setup_cert1_task_with_removed_node();
        let view = ViewNumber::new(21);

        // Node 9 left the committee in epoch 3.
        task.accumulate_vote(make_quorum_vote(9, view, EpochNumber::new(3)));
        assert!(task.ballot_boxes.is_empty());
        assert!(task.signers.is_empty());

        // In epoch 2 it is still a member, and the same vote opens a tally.
        task.accumulate_vote(make_quorum_vote(9, view, EpochNumber::new(2)));
        assert_eq!(task.ballot_boxes.len(), 1);
    }

    /// The same membership still counts node 9's vote in an epoch where it
    /// is a member: committee resolution is per-epoch.
    #[tokio::test]
    async fn test_vote_counts_in_epoch_before_removal() {
        let mut task = setup_cert1_task_with_removed_node();
        let view = ViewNumber::new(11);
        let epoch = EpochNumber::new(2);

        for i in 0..6 {
            task.accumulate_vote(make_quorum_vote(i, view, epoch));
        }
        task.accumulate_vote(make_quorum_vote(9, view, epoch));
        let cert = timeout(CERT_TIMEOUT, task.next()).await.unwrap().unwrap();
        assert_eq!(cert.view_number(), view);
    }

    // ==================== mark_completed ====================

    /// `mark_completed` discards a certificate the accumulator already formed.
    #[tokio::test]
    async fn test_mark_completed_discards_formed_cert() {
        let mut task = setup_cert2_task();
        let view = ViewNumber::new(1);

        for i in 0..THRESHOLD {
            task.accumulate_vote(make_vote2(i, view));
        }
        task.mark_completed(view, EpochNumber::genesis());
        assert_no_certs(&mut task).await;
    }

    /// Votes arriving after `mark_completed` are ignored.
    #[tokio::test]
    async fn test_mark_completed_ignores_later_votes() {
        let mut task = setup_cert2_task();
        let view = ViewNumber::new(1);

        task.mark_completed(view, EpochNumber::genesis());
        for i in 0..THRESHOLD {
            task.accumulate_vote(make_vote2(i, view));
        }
        assert_no_certs(&mut task).await;
    }

    /// `mark_completed` only affects the marked view.
    #[tokio::test]
    async fn test_mark_completed_leaves_other_views_alone() {
        let mut task = setup_cert2_task();
        let marked = ViewNumber::new(1);
        let live = ViewNumber::new(2);

        task.mark_completed(marked, EpochNumber::genesis());
        for i in 0..THRESHOLD {
            task.accumulate_vote(make_vote2(i, marked));
            task.accumulate_vote(make_vote2(i, live));
        }
        let cert = timeout(CERT_TIMEOUT, task.next()).await.unwrap().unwrap();
        assert_eq!(cert.view_number(), live);
        assert_no_certs(&mut task).await;
    }

    /// `mark_completed` only affects the marked epoch: at a boundary the same
    /// view tallies under two committees, and a certificate from one says
    /// nothing about the other.
    #[tokio::test]
    async fn test_mark_completed_leaves_other_epochs_alone() {
        let mut task = setup_cert1_task();
        let view = ViewNumber::new(1);
        let (marked, live) = (EpochNumber::genesis(), EpochNumber::genesis() + 1);

        task.mark_completed(view, marked);
        for i in 0..THRESHOLD {
            task.accumulate_vote(make_quorum_vote(i, view, marked));
            task.accumulate_vote(make_quorum_vote(i, view, live));
        }
        let cert = timeout(CERT_TIMEOUT, task.next()).await.unwrap().unwrap();
        assert_eq!(cert.epoch(), live);
        assert_no_certs(&mut task).await;
    }

    /// A signer that has voted under one epoch may vote again under the next.
    ///
    /// At a boundary the honest nodes are split across the two committees, so
    /// a tally keyed by view alone sees the same signer twice and drops the
    /// second vote. Neither side then reaches its threshold on its own and the
    /// view would have no certificate at all.
    #[tokio::test]
    async fn a_signer_may_vote_again_once_its_epoch_advances() {
        let mut task = setup_cert1_task();
        let view = ViewNumber::new(1);
        let (old, new) = (EpochNumber::genesis(), EpochNumber::genesis() + 1);

        // Everyone votes under the outgoing committee first...
        for i in 0..THRESHOLD {
            task.accumulate_vote(make_quorum_vote(i, view, old));
        }
        // ...then the very same signers vote again under the incoming one.
        for i in 0..THRESHOLD {
            task.accumulate_vote(make_quorum_vote(i, view, new));
        }

        let mut certified = Vec::new();
        for _ in 0..2 {
            let cert = timeout(CERT_TIMEOUT, task.next())
                .await
                .expect("a certificate for each epoch")
                .expect("the collector still has a tally running");
            assert_eq!(cert.view_number(), view);
            certified.push(cert.epoch());
        }
        certified.sort();
        assert_eq!(certified, vec![old, new]);
    }

    // ==================== Timeout certificate epoch binding ====================

    /// Collect a timeout certificate for `view` in `epoch`, in the form that
    /// does not bind the epoch.
    fn timeout_cert_v2(
        view: ViewNumber,
        epoch: EpochNumber,
        lock: &UpgradeLock<TestTypes>,
        membership: &EpochMembership<TestTypes>,
    ) -> TimeoutCertificate2<TestTypes> {
        let mut accumulator = VoteAccumulator::<
            TestTypes,
            TimeoutVote2<TestTypes>,
            TimeoutCertificate2<TestTypes>,
        >::new(lock.clone());

        for i in 0..NUM_NODES {
            let (pub_key, priv_key) = BLSPubKey::generated_from_seed_indexed([0u8; 32], i);
            let data = TimeoutData2 {
                view,
                epoch: Some(epoch),
            };
            let vote = SimpleVote::create_signed_vote(data, view, &pub_key, &priv_key, lock)
                .expect("failed to sign timeout vote");
            if let Some(cert) = accumulator.accumulate(&vote, membership.clone()) {
                return cert;
            }
        }
        panic!("threshold reached without forming a certificate");
    }

    /// Collect a timeout certificate for `view` in `epoch`, in the form that
    /// binds the epoch.
    fn timeout_cert_v3(
        view: ViewNumber,
        epoch: EpochNumber,
        lock: &UpgradeLock<TestTypes>,
        membership: &EpochMembership<TestTypes>,
    ) -> TimeoutCertificate3<TestTypes> {
        let entries = StakeTableEntries::<TestTypes>::from_iter(membership.stake_table()).0;
        let votes = (0..NUM_NODES).map(|i| {
            let (pub_key, priv_key) = BLSPubKey::generated_from_seed_indexed([0u8; 32], i);
            let data = TimeoutData3 {
                view,
                epoch,
                lock: None,
            };
            let vote = SimpleVote::create_signed_vote(data, view, &pub_key, &priv_key, lock)
                .expect("failed to sign timeout vote");
            (None, pub_key, vote.signature())
        });
        TimeoutCertificate3::assemble(view, epoch, &entries, votes.collect::<Vec<_>>(), None)
            .expect("assemble timeout certificate")
    }

    fn verifies(
        cert: &TimeoutEvidence<TestTypes>,
        membership: &EpochMembership<TestTypes>,
        lock: &UpgradeLock<TestTypes>,
    ) -> bool {
        let entries = StakeTableEntries::<TestTypes>::from(
            TimeoutCertificate2::<TestTypes>::stake_table(membership),
        )
        .0;
        let threshold =
            <TimeoutCertificate2<TestTypes> as Certificate<_, _>>::threshold(membership);
        cert.is_valid_cert(&entries, threshold, lock).is_ok()
    }

    /// An upgrade lock that puts [`TIMEOUT_EPOCH_VERSION`] in effect from
    /// `first_view` on. The signatures are never checked by
    /// `UpgradeLock::version`, so the certificate carries none.
    fn upgrading_lock(first_view: ViewNumber) -> UpgradeLock<TestTypes> {
        let data = UpgradeProposalData {
            old_version: NEW_PROTOCOL_VERSION,
            new_version: TIMEOUT_EPOCH_VERSION,
            decide_by: first_view,
            new_version_hash: Vec::new(),
            old_version_last_view: first_view - 1,
            new_version_first_view: first_view,
        };
        let commitment = data.commit();
        let cert =
            UpgradeCertificate::<TestTypes>::new(data, commitment, first_view, None, PhantomData);
        UpgradeLock::from_certificate(
            Upgrade::new(NEW_PROTOCOL_VERSION, TIMEOUT_EPOCH_VERSION),
            &Some(cert),
        )
    }

    /// Relabelling the epoch of an epoch binding certificate must invalidate
    /// its signature. Every consumer picks the stake table to verify against
    /// and the committee to advance into from this field, so an unbound label
    /// lets a relaying node steer both.
    #[tokio::test]
    async fn relabelled_v3_cert_is_rejected() {
        let coordinator = mock_membership();
        let epoch = EpochNumber::genesis();
        let membership = coordinator.membership_for_epoch(Some(epoch)).unwrap();
        let lock = UpgradeLock::new(Upgrade::trivial(TIMEOUT_EPOCH_VERSION));

        let mut cert = timeout_cert_v3(ViewNumber::new(1), epoch, &lock, &membership);
        assert!(
            verifies(&TimeoutEvidence::V3(cert.clone()), &membership, &lock),
            "freshly collected certificate must verify"
        );

        cert.epoch = epoch + 1;
        assert!(
            !verifies(&TimeoutEvidence::V3(cert), &membership, &lock),
            "a certificate relabelled to another epoch must not verify"
        );
    }

    /// The hole the new form closes, pinned: the old form's signature says
    /// nothing about the epoch, so relabelling one leaves it valid. It is
    /// therefore admissibility, checked below, that has to keep the old form out
    /// of a view that must bind its epoch.
    #[tokio::test]
    async fn relabelled_v2_cert_still_verifies_where_it_is_allowed() {
        let coordinator = mock_membership();
        let epoch = EpochNumber::genesis();
        let membership = coordinator.membership_for_epoch(Some(epoch)).unwrap();
        let lock = test_upgrade_lock();

        let mut cert = timeout_cert_v2(ViewNumber::new(1), epoch, &lock, &membership);
        cert.data.epoch = Some(epoch + 1);
        assert!(
            verifies(&TimeoutEvidence::V2(cert), &membership, &lock),
            "the old form does not cover the epoch"
        );
    }

    /// Neither form may stand in for the other: past the upgrade the old form
    /// is refused however well signed, and before it the new one is.
    #[tokio::test]
    async fn the_wrong_form_is_refused() {
        let coordinator = mock_membership();
        let epoch = EpochNumber::genesis();
        let membership = coordinator.membership_for_epoch(Some(epoch)).unwrap();
        let view = ViewNumber::new(1);

        let bound = UpgradeLock::new(Upgrade::trivial(TIMEOUT_EPOCH_VERSION));
        let unbound = test_upgrade_lock();

        let v2 = TimeoutEvidence::V2(timeout_cert_v2(view, epoch, &unbound, &membership));
        let v3 = TimeoutEvidence::V3(timeout_cert_v3(view, epoch, &bound, &membership));

        assert!(!verifies(&v2, &membership, &bound), "old form past upgrade");
        assert!(
            !verifies(&v3, &membership, &unbound),
            "new form before upgrade"
        );
    }

    /// Which form a view requires is keyed on the view the certificate
    /// justifies, not the one it certifies, so the form always matches the
    /// version of the block that carries it.
    #[tokio::test]
    async fn the_required_form_flips_at_the_boundary() {
        let coordinator = mock_membership();
        let epoch = EpochNumber::genesis();
        let membership = coordinator.membership_for_epoch(Some(epoch)).unwrap();
        let boundary = ViewNumber::new(10);
        let lock = upgrading_lock(boundary);

        // Certifies the last old-version view, justifies the first new-version
        // proposal, so the epoch must be bound.
        let justifies_boundary = boundary - 1;
        assert!(
            verifies(
                &TimeoutEvidence::V3(timeout_cert_v3(
                    justifies_boundary,
                    epoch,
                    &lock,
                    &membership
                )),
                &membership,
                &lock,
            ),
            "the certificate justifying the first new-version proposal must bind its epoch"
        );

        // One view earlier the proposal it justifies is still old-version.
        assert!(
            verifies(
                &TimeoutEvidence::V2(timeout_cert_v2(
                    justifies_boundary - 1,
                    epoch,
                    &lock,
                    &membership
                )),
                &membership,
                &lock,
            ),
            "before the boundary the old form is the admissible one"
        );
    }

    // ==================== UpgradeTally ====================

    /// Upgrade threshold for 10 nodes of stake 1: max((10*9)/10, 7) = 9.
    const UPGRADE_THRESHOLD: u64 = 9;

    fn upgrade_data(view: ViewNumber) -> hotshot_types::simple_vote::UpgradeProposalData2 {
        crate::upgrade::expected_upgrade_data(
            &versions::Upgrade::new(versions::version(0, 6), versions::version(0, 7)),
            view,
            EpochNumber::genesis(),
        )
        .unwrap()
    }

    fn make_upgrade_vote(node_index: u64, view: ViewNumber) -> UpgradeVoteMessage<TestTypes> {
        let (pub_key, priv_key) = BLSPubKey::generated_from_seed_indexed([0u8; 32], node_index);
        SimpleVote::create_signed_vote(
            upgrade_data(view),
            view,
            &pub_key,
            &priv_key,
            &test_upgrade_lock(),
        )
        .expect("Failed to sign vote")
    }

    fn make_invalid_upgrade_vote(
        node_index: u64,
        view: ViewNumber,
    ) -> UpgradeVoteMessage<TestTypes> {
        let (pub_key, _) = BLSPubKey::generated_from_seed_indexed([0u8; 32], node_index);
        let (_, wrong_priv_key) = BLSPubKey::generated_from_seed_indexed([1u8; 32], node_index);
        let data = upgrade_data(view);
        let commit =
            VersionedVoteData::<TestTypes, _>::new(data.clone(), view, &test_upgrade_lock())
                .unwrap()
                .commit();
        let bad_sig = BLSPubKey::sign(&wrong_priv_key, commit.as_ref()).unwrap();
        SimpleVote {
            signature: (pub_key, bad_sig),
            data,
            view_number: view,
        }
    }

    fn setup_upgrade_task() -> VoteCollector<TestTypes, super::UpgradeTally<TestTypes>> {
        VoteCollector::new(mock_membership(), test_upgrade_lock())
    }

    async fn assert_no_upgrade_cert(
        task: &mut VoteCollector<TestTypes, super::UpgradeTally<TestTypes>>,
    ) {
        match tokio::time::timeout(NO_CERT_TIMEOUT, task.next()).await {
            Err(_) | Ok(None) => {},
            Ok(Some(cert)) => panic!("Expected no upgrade certificate but got one: {cert:?}"),
        }
    }

    /// An upgrade certificate forms at the (stricter) upgrade threshold, not
    /// at the quorum threshold.
    #[tokio::test]
    async fn test_upgrade_cert_at_upgrade_threshold() {
        let mut task = setup_upgrade_task();
        let view = ViewNumber::new(1);

        for i in 0..UPGRADE_THRESHOLD - 1 {
            task.accumulate_vote(make_upgrade_vote(i, view));
        }
        assert_no_upgrade_cert(&mut task).await;

        task.accumulate_vote(make_upgrade_vote(UPGRADE_THRESHOLD - 1, view));
        let cert = timeout(CERT_TIMEOUT, task.next()).await.unwrap().unwrap();
        assert_eq!(cert.view_number(), view);
        assert_eq!(cert.epoch(), EpochNumber::genesis());
        assert_eq!(cert.data, upgrade_data(view));

        let membership = mock_membership();
        let epoch_membership = membership
            .membership_for_epoch(Some(EpochNumber::genesis()))
            .unwrap();
        verify_cert(cert.cert(), &upgrade_data(view), &epoch_membership);
    }

    /// Duplicate upgrade votes by the same signer count once.
    #[tokio::test]
    async fn test_upgrade_cert_duplicate_votes_ignored() {
        let mut task = setup_upgrade_task();
        let view = ViewNumber::new(1);

        for _ in 0..3 {
            for i in 0..UPGRADE_THRESHOLD - 1 {
                task.accumulate_vote(make_upgrade_vote(i, view));
            }
        }
        assert_no_upgrade_cert(&mut task).await;
    }

    /// Invalid signatures do not contribute; the tally recovers.
    #[tokio::test]
    async fn test_upgrade_cert_invalid_signature_recovery() {
        let mut task = setup_upgrade_task();
        let view = ViewNumber::new(1);

        for i in 0..UPGRADE_THRESHOLD - 1 {
            task.accumulate_vote(make_upgrade_vote(i, view));
        }
        task.accumulate_vote(make_invalid_upgrade_vote(UPGRADE_THRESHOLD - 1, view));
        assert_no_upgrade_cert(&mut task).await;

        task.accumulate_vote(make_upgrade_vote(UPGRADE_THRESHOLD, view));
        let cert = timeout(CERT_TIMEOUT, task.next()).await.unwrap().unwrap();
        assert_eq!(cert.view_number(), view);
    }
}
