use std::{
    any::type_name,
    collections::{BTreeMap, BTreeSet, HashMap},
    mem,
    sync::mpsc,
};

use alloy::primitives::U256;
use committable::Committable;
use hotshot::types::SignatureKey;
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    epoch_membership::{EpochMembership, EpochMembershipCoordinator},
    message::UpgradeLock,
    simple_vote::{HasEpoch, VersionedVoteData},
    stake_table::StakeTableEntries,
    traits::{node_implementation::NodeType, signature_key::StakeTableEntryType},
    vote::{Certificate, Vote, VoteAccumulator},
};
use tokio_util::task::JoinMap;
use tracing::{error, info, warn};

#[allow(type_alias_bounds)]
type VoteSig<T: NodeType> = <T::SignatureKey as SignatureKey>::PureAssembledSignatureType;

pub struct VoteCollector<T: NodeType, V, C> {
    /// Tasks collecting votes and verifying certificates.
    accumulators: JoinMap<ViewNumber, Option<C>>,

    /// Where callers submit their votes.
    ballot_boxes: BTreeMap<ViewNumber, mpsc::Sender<V>>,

    /// Votes for epochs we have yet to resolve.
    pending: BTreeMap<ViewNumber, Vec<V>>,

    /// Views that had a valid certificate already.
    completed: BTreeSet<ViewNumber>,

    /// The signers and their vote signatures per view.
    signers: BTreeMap<ViewNumber, HashMap<T::SignatureKey, VoteSig<T>>>,

    lower_bound: ViewNumber,

    membership: EpochMembershipCoordinator<T>,
    upgrade_lock: UpgradeLock<T>,
}

impl<T, V, C> VoteCollector<T, V, C>
where
    T: NodeType,
    V: Vote<T> + HasEpoch + Send + 'static,
    C: Certificate<T, V::Commitment, Voteable = V::Commitment> + Send + 'static,
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

    pub async fn next(&mut self) -> Option<C> {
        loop {
            match self.accumulators.join_next().await {
                Some((view, Ok(Some(cert)))) => {
                    self.ballot_boxes.remove(&view);
                    if view >= self.lower_bound {
                        self.completed.insert(view);
                        return Some(cert);
                    }
                },
                Some((_, Ok(None))) => {},
                Some((view, Err(err))) => {
                    if !err.is_cancelled() {
                        error!(%view, %err, "vote collection task panic");
                    }
                },
                None => return None,
            }
        }
    }

    pub fn accumulate_vote(&mut self, vote: V) {
        let view = vote.view_number();

        if view < self.lower_bound || self.completed.contains(&view) {
            return;
        }

        // Check that we have not received a vote from this signer already.
        {
            let key = vote.signing_key();
            let sig = vote.signature();

            let signers = self.signers.entry(view).or_default();

            if let Some(s) = signers.get(&key)
                && *s != sig
            {
                warn!(%view, cert = type_name::<C>(), signer = %key, "multiple votes in one view");
                return;
            } else {
                signers.insert(vote.signing_key(), vote.signature());
            }
        }

        let Some(membership) = self.resolve_membership(&vote) else {
            self.pending.entry(view).or_default().push(vote);
            return;
        };

        if let Some(tx) = self.ballot_boxes.get(&view) {
            let _ = tx.send(vote);
            return;
        }

        let (tx, rx) = mpsc::channel();

        let _ = tx.send(vote);
        self.ballot_boxes.insert(view, tx);

        let lock = self.upgrade_lock.clone();
        self.accumulators
            .spawn_blocking(view, move || accumulate_votes(view, rx, membership, lock));
    }

    pub fn retry_pending_votes(&mut self) {
        for vote in mem::take(&mut self.pending).into_values().flatten() {
            self.accumulate_vote(vote)
        }
    }

    /// Compute the accumulated stake.
    ///
    /// This is the sum across unique signers we've routed to the accumulator
    /// for `view` and the cert threshold in `epoch`. Looks up each signer's
    /// stake on demand — only intended for rare paths like timeout
    /// diagnostics. Returns `None` if no votes have been seen for `view` or
    /// `epoch`'s stake table is unavailable.
    pub fn stats(&self, view: ViewNumber, epoch: EpochNumber) -> Option<VoteStats> {
        let signers = self.signers.get(&view)?;
        if signers.is_empty() {
            return None;
        }
        let membership = self.membership.membership_for_epoch(Some(epoch)).ok()?;
        let threshold = C::threshold(&membership);
        let mut stake = U256::ZERO;
        for signer in signers.keys() {
            if let Some(peer) = C::stake_table_entry(&membership, signer) {
                stake += peer.stake_table_entry.stake();
            }
        }
        Some(VoteStats { stake, threshold })
    }

    pub fn gc(&mut self, view: ViewNumber) {
        self.ballot_boxes = self.ballot_boxes.split_off(&view);
        self.completed = self.completed.split_off(&view);
        self.pending = self.pending.split_off(&view);
        self.signers = self.signers.split_off(&view);
        self.lower_bound = view;
    }

    fn resolve_membership(&mut self, vote: &V) -> Option<EpochMembership<T>> {
        let epoch = vote.epoch()?;
        self.membership.membership_for_epoch(Some(epoch)).ok()
    }
}

fn accumulate_votes<T, V, C>(
    view: ViewNumber,
    rx: mpsc::Receiver<V>,
    membership: EpochMembership<T>,
    lock: UpgradeLock<T>,
) -> Option<C>
where
    T: NodeType,
    V: Vote<T>,
    C: Certificate<T, V::Commitment, Voteable = V::Commitment> + Send + 'static,
{
    let generate_vote_commitment = |vote: &V, lock| match VersionedVoteData::new(
        vote.date().clone(),
        vote.view_number(),
        lock,
    ) {
        Ok(data) => Some(data.commit()),
        Err(err) => {
            warn!(%err, "failed to generate versioned vote data");
            None
        },
    };

    let mut accu = VoteAccumulator::<T, V, C>::new(lock.clone());
    let mut votes = Vec::new();

    // Collect all votes until a certificate can be formed:
    let cert = loop {
        let Ok(vote) = rx.recv() else { return None };
        if let Some(cert) = accu.accumulate(&vote, membership.clone()) {
            votes.push(vote);
            break cert;
        }
        votes.push(vote);
    };

    let table = StakeTableEntries::from(C::stake_table(&membership));
    let thresh = C::threshold(&membership);

    match cert.is_valid_cert(&table.0, thresh, &lock) {
        Ok(()) => {
            info!(%view, cert = type_name::<C>(), "certificate formed");
            Some(cert)
        },
        Err(err) => {
            warn!(%view, %err, "invalid certificate formed");

            // Remove all invalid votes and reset the accumulator:
            votes.retain(|v| {
                if let Some(c) = generate_vote_commitment(v, &lock) {
                    v.signing_key().validate(&v.signature(), c.as_ref())
                } else {
                    warn!(%view, cert = type_name::<C>(), signer = %v.signing_key(), "invalid vote");
                    false
                }
            });

            accu.clear();

            for vote in votes {
                if let Some(cert) = accu.accumulate(&vote, membership.clone()) {
                    debug_assert!(cert.is_valid_cert(&table.0, thresh, &lock).is_ok());
                    return Some(cert);
                }
            }

            // Continue to collect votes, but check them before accumulating:
            while let Ok(vote) = rx.recv() {
                if let Some(c) = generate_vote_commitment(&vote, &lock)
                    && vote.signing_key().validate(&vote.signature(), c.as_ref())
                {
                    if let Some(cert) = accu.accumulate(&vote, membership.clone()) {
                        debug_assert!(cert.is_valid_cert(&table.0, thresh, &lock).is_ok());
                        return Some(cert);
                    }
                } else {
                    warn!(%view, cert = type_name::<C>(), signer = %vote.signing_key(), "invalid vote");
                }
            }

            None
        },
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
    use std::{fmt::Debug, time::Duration};

    use committable::Committable;
    use hotshot::types::BLSPubKey;
    use hotshot_example_types::node_types::TestTypes;
    use hotshot_types::{
        data::{EpochNumber, ViewNumber},
        epoch_membership::EpochMembership,
        simple_vote::{
            HasEpoch, QuorumData2, QuorumVote2, SimpleVote, VersionedVoteData, Vote2Data,
        },
        stake_table::StakeTableEntries,
        traits::signature_key::SignatureKey,
        vote::{Certificate, HasViewNumber, Vote},
    };
    use tokio::{sync::mpsc, time::timeout};

    use super::VoteCollector;
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
    fn setup_cert1_task()
    -> VoteCollector<TestTypes, QuorumVote2<TestTypes>, Certificate1<TestTypes>> {
        setup_task::<QuorumVote2<TestTypes>, Certificate1<TestTypes>>()
    }

    fn setup_cert2_task() -> VoteCollector<TestTypes, Vote2<TestTypes>, Certificate2<TestTypes>> {
        setup_task::<Vote2<TestTypes>, Certificate2<TestTypes>>()
    }

    /// Spawn a VoteCollectionTask for Certificate2.
    fn setup_task<
        V: Vote<TestTypes> + HasEpoch + Send + Sync + 'static,
        C: Certificate<TestTypes, V::Commitment, Voteable = V::Commitment> + Send + Sync + 'static,
    >() -> VoteCollector<TestTypes, V, C> {
        let membership = mock_membership();
        VoteCollector::<TestTypes, V, C>::new(membership, test_upgrade_lock())
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
        V: Vote<TestTypes> + HasEpoch + Send + Sync + 'static,
        C: Certificate<TestTypes, V::Commitment, Voteable = V::Commitment>
            + Debug
            + Send
            + Sync
            + 'static,
    >(
        task: &mut VoteCollector<TestTypes, V, C>,
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
        verify_cert(&cert, &expected_data, &epoch_membership);
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
            verify_cert(cert, &expected_data, &epoch_membership);
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
        verify_cert(&cert, &expected_data, &epoch_membership);
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
            verify_cert(cert, &expected_data, &epoch_membership);
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
        verify_cert(&cert, &vote_2_data(), &epoch_membership);
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
}
