//! Vote collection and certificate formation, per view and epoch.
//!
//! Votes arrive concurrently and are tallied on their own until they cross a
//! threshold and form a certificate. [`VoteCollector`] owns the machinery
//! common to every kind of vote, inspecting each vote only through the
//! [`Ballot`] trait. It creates a task per [`Round`] and routes votes to the
//! appropriate task, drops duplicate and stale votes, buffers votes whose
//! epoch is not yet resolved, and GCs decided views. How a round's task
//! actually combines votes into an output is delegated to a pluggable
//! [`Tally`] strategy.

mod accumulate;

use std::{
    any::{type_name, type_name_of_val},
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    marker::PhantomData,
    mem,
    sync::mpsc::{self, Receiver},
};

pub(crate) use accumulate::CheckedAccumulator;
use alloy::primitives::U256;
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    epoch_membership::{EpochMembership, EpochMembershipCoordinator},
    message::UpgradeLock,
    simple_certificate::{LightClientStateUpdateCertificateV2, QuorumCertificate2},
    simple_vote::{HasEpoch, QuorumVote2, SimpleVote, Voteable},
    traits::{node_implementation::NodeType, signature_key::StakeTableEntryType},
    vote::{Certificate, HasViewNumber, LightClientStateUpdateVoteAccumulator, Vote},
};
use tokio_util::task::JoinMap;
use tracing::{error, info, warn};

use crate::{cert_verifier::ValidCert, message::Vote1};

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
pub trait Tally<T: NodeType> {
    type Vote: Send + 'static;
    type Output: Send + 'static;

    fn tally(
        r: Receiver<Self::Vote>,
        m: EpochMembership<T>,
        l: UpgradeLock<T>,
    ) -> Option<Self::Output>;
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
#[allow(clippy::type_complexity)]
pub struct SimpleTally<T, V, C>(PhantomData<fn() -> (T, V, C)>);

impl<T, V, C> Tally<T> for SimpleTally<T, V, C>
where
    T: NodeType,
    V: Vote<T> + Send + 'static,
    C: Certificate<T, V::Commitment, Voteable = V::Commitment> + HasEpoch + Send + 'static,
{
    type Vote = V;
    type Output = ValidCert<C>;

    fn tally(r: Receiver<V>, m: EpochMembership<T>, l: UpgradeLock<T>) -> Option<Self::Output> {
        let mut a = CheckedAccumulator::<T, V, C>::new(m, l);
        while let Ok(v) = r.recv() {
            if let Some(c) = a.add(v) {
                if let Some(e) = c.epoch() {
                    return Some(ValidCert::new(c, e));
                } else {
                    warn!(cert = type_name::<C>(), "certificate has no epoch number");
                    break;
                }
            }
        }
        None
    }
}

/// The quorum and light-client state certificates formed at an epoch-root view.
pub type EpochRootCerts<T> = (
    ValidCert<QuorumCertificate2<T>>,
    LightClientStateUpdateCertificateV2<T>,
);

/// Accumulates epoch-root [`Vote1`]s into the (quorum, state) certificate pair.
pub struct EpochRootTally<T>(PhantomData<fn() -> T>);

impl<T: NodeType> Tally<T> for EpochRootTally<T> {
    type Vote = Vote1<T>;
    type Output = EpochRootCerts<T>;

    fn tally(
        rx: mpsc::Receiver<Vote1<T>>,
        mem: EpochMembership<T>,
        lock: UpgradeLock<T>,
    ) -> Option<Self::Output> {
        let mut quorum_accu = CheckedAccumulator::<T, QuorumVote2<T>, QuorumCertificate2<T>>::new(
            mem.clone(),
            lock.clone(),
        );

        let mut state_accu = LightClientStateUpdateVoteAccumulator::<T> {
            vote_outcomes: HashMap::new(),
            upgrade_lock: lock,
        };

        let mut quorum_cert = None;
        let mut state_cert = None;

        while let Ok(vote1) = rx.recv() {
            let Some(state_vote) = vote1.state_vote else {
                error!(view = %vote1.vote.view_number(), "epoch-root vote1 without state vote");
                continue;
            };
            let bls_key = vote1.vote.signing_key();

            if quorum_cert.is_none() {
                quorum_cert = quorum_accu.add(vote1.vote);
            }

            // Unlike quorum votes, state votes are fully checked, including
            // their signatures, by the accumulator, so the certificate does
            // not need to be validated again.
            if state_cert.is_none() {
                state_cert = state_accu.accumulate(&bls_key, &state_vote, &mem);
            }

            if let (Some(q), Some(s)) = (&quorum_cert, &state_cert) {
                info!(view = %q.view_number(), epoch = %s.epoch, "epoch-root certificates formed");
                if let Some(e) = q.epoch() {
                    return Some((ValidCert::new(q.clone(), e), s.clone()));
                } else {
                    warn!(
                        cert = type_name_of_val(q),
                        "certificate has no epoch number"
                    );
                    break;
                }
            }
        }

        None
    }
}

/// The view a vote is for and the epoch whose committee it asks to certify it.
type Round = (ViewNumber, EpochNumber);

/// Collects votes per round and forms certificate(s) using the strategy `S`.
pub struct VoteCollector<T: NodeType, S: Tally<T>> {
    /// Tasks collecting votes and forming certificates.
    accumulators: JoinMap<Round, Option<S::Output>>,

    /// Where callers submit their votes.
    ballot_boxes: BTreeMap<Round, mpsc::Sender<S::Vote>>,

    /// Votes for epochs we have yet to resolve, deduplicated by signer.
    pending: BTreeMap<Round, HashMap<T::SignatureKey, S::Vote>>,

    /// Rounds that had a valid certificate already.
    completed: BTreeSet<Round>,

    /// The signers per round.
    signers: BTreeMap<Round, HashSet<T::SignatureKey>>,

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
            match self.accumulators.join_next().await {
                Some((round, Ok(Some(cert)))) => {
                    self.ballot_boxes.remove(&round);
                    if round.0 >= self.lower_bound {
                        self.completed.insert(round);
                        return Some(cert);
                    }
                },
                Some((_, Ok(None))) => {},
                Some((round, Err(err))) => {
                    if err.is_panic() {
                        let (view, epoch) = round;
                        error!(%view, %epoch, %err, "vote collection task panic");
                    }
                    self.ballot_boxes.remove(&round);
                },
                None => return None,
            }
        }
    }

    pub fn accumulate_vote(&mut self, vote: S::Vote) {
        let view = vote.view();

        if view < self.lower_bound {
            return;
        }

        let Some(epoch) = vote.epoch() else {
            return;
        };

        let round = (view, epoch);

        if self.completed.contains(&round) {
            return;
        }

        let Ok(membership) = self.membership.membership_for_epoch(Some(epoch)) else {
            // The stake table for a named epoch becomes available eventually,
            // so the vote waits rather than being dropped.
            self.pending
                .entry(round)
                .or_default()
                .insert(vote.signer(), vote);
            return;
        };

        // Check that we have not received a vote from this signer already.
        if !self.signers.entry(round).or_default().insert(vote.signer()) {
            return;
        }

        if let Some(tx) = self.ballot_boxes.get(&round) {
            let _ = tx.send(vote);
            return;
        }

        let (tx, rx) = mpsc::channel();

        let _ = tx.send(vote);
        self.ballot_boxes.insert(round, tx);

        let lock = self.upgrade_lock.clone();
        self.accumulators
            .spawn_blocking(round, move || S::tally(rx, membership, lock));
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

impl<T, V, C> VoteCollector<T, SimpleTally<T, V, C>>
where
    T: NodeType,
    V: Vote<T> + Send + 'static,
    C: Certificate<T, V::Commitment, Voteable = V::Commitment> + HasEpoch + Send + 'static,
{
    /// Compute the accumulated stake.
    ///
    /// This is the sum across unique signers we've routed to the accumulator
    /// for `view` in `epoch`, and that epoch's cert threshold. Looks up each
    /// signer's stake on demand — only intended for rare paths like timeout
    /// diagnostics. Returns `None` if no votes have been seen for the round or
    /// `epoch`'s stake table is unavailable.
    pub fn stats(&self, view: ViewNumber, epoch: EpochNumber) -> Option<VoteStats> {
        let signers = self.signers.get(&(view, epoch))?;
        if signers.is_empty() {
            return None;
        }
        let membership = self.membership.membership_for_epoch(Some(epoch)).ok()?;
        let threshold = C::threshold(&membership);
        let mut stake = U256::ZERO;
        for signer in signers {
            if let Some(peer) = C::stake_table_entry(&membership, signer) {
                stake += peer.stake_table_entry.stake();
            }
        }
        Some(VoteStats { stake, threshold })
    }
}

/// Accumulated stake / threshold for a single view.
///
/// Used by diagnostics (e.g. timeout logging) to show how close a view came
/// to forming a cert.
#[derive(Clone, Copy, Debug)]
pub struct VoteStats {
    pub stake: U256,
    pub threshold: U256,
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
            TimeoutCertificate2, TimeoutCertificate3, TimeoutEvidence, UpgradeCertificate,
        },
        simple_vote::{
            HasEpoch, QuorumData2, QuorumVote2, SimpleVote, TimeoutData2, TimeoutData3,
            TimeoutVote2, TimeoutVote3, UpgradeProposalData, VersionedVoteData, Vote2Data,
        },
        stake_table::StakeTableEntries,
        traits::{node_implementation::NodeType, signature_key::SignatureKey},
        vote::{Certificate, HasViewNumber, Vote, VoteAccumulator},
    };
    use tokio::{sync::mpsc, time::timeout};
    use versions::{NEW_PROTOCOL_VERSION, TIMEOUT_EPOCH_VERSION, Upgrade};

    use super::{Ballot, SimpleTally, VoteCollector};
    use crate::{
        helpers::test_upgrade_lock,
        message::{Certificate1, Certificate2, Vote2},
        tests::common::utils::mock_membership,
    };

    /// Number of test validators.
    const NUM_NODES: u64 = 10;
    /// Threshold for SuccessThreshold with 10 nodes of stake 1: (10*2)/3 + 1 = 7.
    const THRESHOLD: u64 = 7;

    /// How long to wait for expected certificates before failing.
    const CERT_TIMEOUT: Duration = Duration::from_millis(100);
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
    fn setup_cert1_task() -> VoteCollector<
        TestTypes,
        SimpleTally<TestTypes, QuorumVote2<TestTypes>, Certificate1<TestTypes>>,
    > {
        setup_task::<QuorumVote2<TestTypes>, Certificate1<TestTypes>>()
    }

    fn setup_cert2_task()
    -> VoteCollector<TestTypes, SimpleTally<TestTypes, Vote2<TestTypes>, Certificate2<TestTypes>>>
    {
        setup_task::<Vote2<TestTypes>, Certificate2<TestTypes>>()
    }

    /// Spawn a VoteCollectionTask for Certificate2.
    fn setup_task<
        V: Ballot<Signer = <TestTypes as NodeType>::SignatureKey>
            + Vote<TestTypes>
            + HasEpoch
            + Send
            + Sync
            + 'static,
        C: Certificate<TestTypes, V::Commitment, Voteable = V::Commitment>
            + HasEpoch
            + Send
            + Sync
            + 'static,
    >() -> VoteCollector<TestTypes, SimpleTally<TestTypes, V, C>> {
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

    /// Confirm no certificates are produced within the timeout, then abort the task.
    async fn assert_no_certs<
        V: Ballot<Signer = <TestTypes as NodeType>::SignatureKey>
            + Vote<TestTypes>
            + HasEpoch
            + Send
            + Sync
            + 'static,
        C: Certificate<TestTypes, V::Commitment, Voteable = V::Commitment>
            + HasEpoch
            + Debug
            + Send
            + Sync
            + 'static,
    >(
        task: &mut VoteCollector<TestTypes, SimpleTally<TestTypes, V, C>>,
    ) {
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

    // ==================== One view, two epochs ====================

    /// Collect both certificates a view can produce when its voters disagree
    /// about the epoch, sorted by epoch.
    async fn certs_by_epoch(
        task: &mut VoteCollector<
            TestTypes,
            SimpleTally<TestTypes, QuorumVote2<TestTypes>, Certificate1<TestTypes>>,
        >,
    ) -> Vec<EpochNumber> {
        let mut epochs = Vec::new();
        for _ in 0..2 {
            let cert = timeout(CERT_TIMEOUT, task.next())
                .await
                .expect("a certificate for each epoch")
                .expect("the collector still has a tally running");
            epochs.push(cert.epoch());
        }
        epochs.sort();
        epochs
    }

    /// Votes naming different epochs are tallied against their own committees,
    /// and each reaching the threshold forms its own certificate. Sharing one
    /// tally would weigh one committee's signers against the other's stake
    /// table.
    #[tokio::test]
    async fn votes_of_two_epochs_in_one_view_tally_separately() {
        let mut task = setup_cert1_task();
        let view = ViewNumber::new(1);
        let (old, new) = (EpochNumber::genesis(), EpochNumber::genesis() + 1);

        for i in 0..THRESHOLD {
            task.accumulate_vote(make_quorum_vote(i, view, old));
            task.accumulate_vote(make_quorum_vote(i, view, new));
        }

        assert_eq!(certs_by_epoch(&mut task).await, vec![old, new]);
    }

    /// A signer whose epoch advances may vote again in the same view, and the
    /// second vote counts.
    ///
    /// This is what an epoch boundary needs. A node that has not yet seen the
    /// boundary block decided votes under the outgoing committee; when the
    /// decision reaches it, it votes again under the incoming one. Deduplicating
    /// by view alone would drop that second vote as a repeat from a signer
    /// already seen, so neither committee would ever reach a threshold and the
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

        assert_eq!(certs_by_epoch(&mut task).await, vec![old, new]);
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
    fn setup_cert1_task_with_removed_node() -> VoteCollector<
        TestTypes,
        SimpleTally<TestTypes, QuorumVote2<TestTypes>, Certificate1<TestTypes>>,
    > {
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
        let mut accumulator = VoteAccumulator::<
            TestTypes,
            TimeoutVote3<TestTypes>,
            TimeoutCertificate3<TestTypes>,
        >::new(lock.clone());

        for i in 0..NUM_NODES {
            let (pub_key, priv_key) = BLSPubKey::generated_from_seed_indexed([0u8; 32], i);
            let data = TimeoutData3 { view, epoch };
            let vote = SimpleVote::create_signed_vote(data, view, &pub_key, &priv_key, lock)
                .expect("failed to sign timeout vote");
            if let Some(cert) = accumulator.accumulate(&vote, membership.clone()) {
                return cert;
            }
        }
        panic!("threshold reached without forming a certificate");
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

        cert.data.epoch = epoch + 1;
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
}
