use std::collections::{BTreeMap, BTreeSet};

use committable::{Commitment, Committable};
use hotshot::types::SignatureKey;
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    epoch_membership::{EpochMembership, EpochMembershipCoordinator},
    message::UpgradeLock,
    simple_vote::{HasEpoch, VersionedVoteData},
    stake_table::StakeTableEntries,
    traits::node_implementation::NodeType,
    vote::{Certificate, Vote, VoteAccumulator},
};
use tokio::{
    sync::mpsc::{self},
    task::{AbortHandle, JoinSet},
};
use tracing::{instrument, warn};

use crate::helpers::upgrade_lock;

pub struct VoteCollector<T: NodeType, V, C> {
    accumulators: BTreeMap<ViewNumber, (mpsc::Sender<V>, AbortHandle)>,
    completed_certificates: BTreeSet<ViewNumber>,
    epoch_membership_coordinator: EpochMembershipCoordinator<T>,
    membership_cache: BTreeMap<EpochNumber, EpochMembership<T>>,
    upgrade_lock: UpgradeLock<T>,
    tasks: JoinSet<C>,
}

impl<T, V, C> VoteCollector<T, V, C>
where
    T: NodeType,
    V: Vote<T> + HasEpoch + Send + Sync + 'static,
    C: Certificate<T, V::Commitment, Voteable = V::Commitment> + Send + Sync + 'static,
{
    #[instrument(level = "debug", skip_all)]
    pub fn new(
        epoch_membership_coordinator: EpochMembershipCoordinator<T>,
        upgrade_lock: UpgradeLock<T>,
    ) -> Self {
        Self {
            accumulators: BTreeMap::new(),
            completed_certificates: BTreeSet::new(),
            epoch_membership_coordinator,
            membership_cache: BTreeMap::new(),
            upgrade_lock,
            tasks: JoinSet::new(),
        }
    }

    pub async fn next(&mut self) -> Option<C> {
        loop {
            match self.tasks.join_next().await {
                Some(Ok(cert)) => {
                    if self.completed_certificates.contains(&cert.view_number()) {
                        continue;
                    }
                    self.completed_certificates.insert(cert.view_number());
                    return Some(cert);
                },
                Some(Err(e)) => {
                    warn!("Error in vote collection task: {e}");
                },
                None => return None,
            }
        }
    }

    pub async fn accumulate_vote(&mut self, vote: V) {
        let view = vote.view_number();
        if self.completed_certificates.contains(&view) {
            return;
        }
        if !self.accumulators.contains_key(&view) {
            let Some(epoch) = vote.epoch() else {
                return;
            };
            let membership = if let Some(m) = self.membership_cache.get(&epoch) {
                m.clone()
            } else {
                let Ok(m) = self
                    .epoch_membership_coordinator
                    .membership_for_epoch(Some(epoch))
                    .await
                else {
                    return;
                };
                self.membership_cache.insert(epoch, m.clone());
                m
            };
            let (tx, rx) = mpsc::channel(100);
            let accumulator = VoteAccumulator::new(self.upgrade_lock.clone());
            let abort_handle =
                self.tasks
                    .spawn(Self::run_per_view(view, rx, accumulator, membership));
            self.accumulators.insert(view, (tx, abort_handle));
        }
        if let Some((tx, _)) = self.accumulators.get(&view) {
            let _ = tx.send(vote).await;
        }
    }

    #[instrument(level = "debug", skip_all)]
    async fn run_per_view(
        _view: ViewNumber,
        mut rx: mpsc::Receiver<V>,
        mut accumulator: VoteAccumulator<T, V, C>,
        membership: EpochMembership<T>,
    ) -> C {
        let mut votes = Vec::new();

        while let Some(vote) = rx.recv().await {
            if let Some(cert) = accumulator.accumulate(&vote, membership.clone()).await {
                let stake_table = C::stake_table(&membership).await;
                let threshold = C::threshold(&membership).await;
                match cert.is_valid_cert(
                    &StakeTableEntries::<T>::from(stake_table).0,
                    threshold,
                    &upgrade_lock(),
                ) {
                    Ok(()) => {
                        return cert;
                    },
                    Err(e) => {
                        warn!("Invalid certificate formed: {e}");
                        votes.push(vote);
                        // Recover the good votes, this takes a long time
                        // TODO make this more efficient by parallelizing the validation
                        votes.retain(|v: &V| {
                            let vote_commitment = generate_vote_commitment(v, &upgrade_lock());

                            vote_commitment.is_some_and(|commitment| {
                                v.signing_key()
                                    .validate(&v.signature(), commitment.as_ref())
                            })
                        });
                        accumulator = VoteAccumulator::new(upgrade_lock());
                        for vote in &votes {
                            // after recovering the good votes, try to accumulate them again, but this time
                            // we know the cert if good if we can form it
                            if let Some(cert) =
                                accumulator.accumulate(vote, membership.clone()).await
                            {
                                return cert;
                            }
                        }
                    },
                }
            } else {
                votes.push(vote);
            }
        }
        unreachable!()
    }
    pub fn gc(&mut self, view: ViewNumber, epoch: EpochNumber) {
        let keep = self.accumulators.split_off(&view);
        self.completed_certificates = self.completed_certificates.split_off(&view);
        for (_, handle) in self.accumulators.values_mut() {
            handle.abort();
        }
        self.accumulators = keep;
        self.membership_cache = self.membership_cache.split_off(&epoch);
    }
}

fn generate_vote_commitment<T: NodeType, V: Vote<T>>(
    vote: &V,
    upgrade_lock: &UpgradeLock<T>,
) -> Option<Commitment<VersionedVoteData<T, V::Commitment>>> {
    match VersionedVoteData::new(vote.date().clone(), vote.view_number(), upgrade_lock) {
        Ok(data) => Some(data.commit()),
        Err(e) => {
            tracing::warn!("Failed to generate versioned vote data: {e}");
            None
        },
    }
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
        helpers::upgrade_lock,
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
        SimpleVote::create_signed_vote(data, view, &pub_key, &priv_key, &upgrade_lock())
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
        SimpleVote::create_signed_vote(data, view, &pub_key, &priv_key, &upgrade_lock())
            .expect("Failed to sign vote")
    }

    /// Create a Vote2 with an invalid signature (signed by a different key than claimed).
    fn make_invalid_vote2(node_index: u64, view: ViewNumber) -> Vote2<TestTypes> {
        let (pub_key, _) = BLSPubKey::generated_from_seed_indexed([0u8; 32], node_index);
        // Sign with a completely different key
        let (_, wrong_priv_key) = BLSPubKey::generated_from_seed_indexed([1u8; 32], node_index);
        let data = vote_2_data();
        let commit = VersionedVoteData::<TestTypes, _>::new(data.clone(), view, &upgrade_lock())
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
    async fn setup_cert1_task()
    -> VoteCollector<TestTypes, QuorumVote2<TestTypes>, Certificate1<TestTypes>> {
        setup_task::<QuorumVote2<TestTypes>, Certificate1<TestTypes>>().await
    }
    async fn setup_cert2_task()
    -> VoteCollector<TestTypes, Vote2<TestTypes>, Certificate2<TestTypes>> {
        setup_task::<Vote2<TestTypes>, Certificate2<TestTypes>>().await
    }

    /// Spawn a VoteCollectionTask for Certificate2.
    async fn setup_task<
        V: Vote<TestTypes> + HasEpoch + Send + Sync + 'static,
        C: Certificate<TestTypes, V::Commitment, Voteable = V::Commitment> + Send + Sync + 'static,
    >() -> VoteCollector<TestTypes, V, C> {
        let membership = mock_membership().await;
        VoteCollector::<TestTypes, V, C>::new(membership, upgrade_lock())
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
    async fn verify_cert<C, D>(cert: &C, expected_data: &D, membership: &EpochMembership<TestTypes>)
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
        let stake_table = C::stake_table(membership).await;
        let stake_table_entries = StakeTableEntries::<TestTypes>::from(stake_table).0;
        let threshold = C::threshold(membership).await;
        cert.is_valid_cert(&stake_table_entries, threshold, &upgrade_lock())
            .expect("Certificate signature validation failed");
    }

    // ==================== Certificate1 (QuorumVote2) happy path ====================

    /// Sending enough QuorumVote2s for a single view produces a valid Certificate1
    /// whose data commitment matches the votes.
    #[tokio::test]
    async fn test_cert1_single_view_happy_path() {
        let mut task = setup_cert1_task().await;
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
            task.accumulate_vote(make_quorum_vote(i, view, epoch)).await;
        }

        let cert = timeout(CERT_TIMEOUT, task.next()).await.unwrap().unwrap();
        assert_eq!(cert.view_number(), view);

        let membership = mock_membership().await;
        let epoch_membership = membership.membership_for_epoch(Some(epoch)).await.unwrap();
        verify_cert(&cert, &expected_data, &epoch_membership).await;
    }

    /// Sending votes for multiple views produces a valid certificate for each view,
    /// each with data commitment matching the votes.
    #[tokio::test]
    async fn test_cert1_multiple_views_parallel() {
        let mut task = setup_cert1_task().await;
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
                task.accumulate_vote(make_quorum_vote(i, view, epoch)).await;
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

        let membership = mock_membership().await;
        let epoch_membership = membership.membership_for_epoch(Some(epoch)).await.unwrap();
        for cert in &certs {
            verify_cert(cert, &expected_data, &epoch_membership).await;
        }
    }

    // ==================== Certificate2 (Vote2) happy path ====================

    /// Sending enough Vote2s for a single view produces a valid Certificate2
    /// whose data commitment matches the votes.
    #[tokio::test]
    async fn test_cert2_single_view_happy_path() {
        let mut task = setup_cert2_task().await;
        let view = ViewNumber::new(1);
        let epoch = EpochNumber::genesis();
        let expected_data = vote_2_data();

        for i in 0..THRESHOLD {
            task.accumulate_vote(make_vote2(i, view)).await;
        }

        let cert = timeout(CERT_TIMEOUT, task.next()).await.unwrap().unwrap();
        assert_eq!(cert.view_number(), view);

        let membership = mock_membership().await;
        let epoch_membership = membership.membership_for_epoch(Some(epoch)).await.unwrap();
        verify_cert(&cert, &expected_data, &epoch_membership).await;
    }

    /// Sending votes for multiple views in parallel produces valid certificates for each,
    /// each with data commitment matching the votes.
    #[tokio::test]
    async fn test_cert2_multiple_views_parallel() {
        let mut task = setup_cert2_task().await;
        let epoch = EpochNumber::genesis();
        let expected_data = vote_2_data();

        let views = [ViewNumber::new(5), ViewNumber::new(6), ViewNumber::new(7)];

        for i in 0..THRESHOLD {
            for &view in &views {
                task.accumulate_vote(make_vote2(i, view)).await;
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

        let membership = mock_membership().await;
        let epoch_membership = membership.membership_for_epoch(Some(epoch)).await.unwrap();
        for cert in &certs {
            verify_cert(cert, &expected_data, &epoch_membership).await;
        }
    }

    // ==================== Certificate1 failure cases ====================

    /// Fewer than threshold votes do not produce a certificate.
    #[tokio::test]
    async fn test_cert1_below_threshold_no_certificate() {
        let mut task = setup_cert1_task().await;
        let view = ViewNumber::new(1);
        let epoch = EpochNumber::genesis();

        for i in 0..(THRESHOLD - 1) {
            task.accumulate_vote(make_quorum_vote(i, view, epoch)).await;
        }

        assert_no_certs(&mut task).await;
    }

    /// Duplicate votes from the same signer do not count toward threshold.
    #[tokio::test]
    async fn test_cert1_duplicate_votes_ignored() {
        let mut task = setup_cert1_task().await;
        let view = ViewNumber::new(1);
        let epoch = EpochNumber::genesis();

        // Send 6 unique votes (below threshold of 7)
        for i in 0..6 {
            task.accumulate_vote(make_quorum_vote(i, view, epoch)).await;
        }
        // Send duplicates of node 0 — should not push us over threshold
        for _ in 0..5 {
            task.accumulate_vote(make_quorum_vote(0, view, epoch)).await;
        }

        assert_no_certs(&mut task).await;
    }

    // ==================== Certificate2 failure cases ====================

    /// Fewer than threshold Vote2s do not produce a Certificate2.
    #[tokio::test]
    async fn test_cert2_below_threshold_no_certificate() {
        let mut task = setup_cert2_task().await;
        let view = ViewNumber::new(1);

        for i in 0..(THRESHOLD - 1) {
            task.accumulate_vote(make_vote2(i, view)).await;
        }

        assert_no_certs(&mut task).await;
    }

    /// Duplicate Vote2s from the same signer do not count toward threshold.
    #[tokio::test]
    async fn test_cert2_duplicate_votes_ignored() {
        let mut task = setup_cert2_task().await;
        let view = ViewNumber::new(1);

        // Send 6 unique votes (below threshold of 7)
        for i in 0..6 {
            task.accumulate_vote(make_vote2(i, view)).await;
        }
        // Repeat node 0 votes — should not reach threshold
        for _ in 0..5 {
            task.accumulate_vote(make_vote2(0, view)).await;
        }

        assert_no_certs(&mut task).await;
    }

    /// Votes with invalid signatures are rejected and do not count.
    #[tokio::test]
    async fn test_cert2_invalid_signature_rejected() {
        let mut task = setup_cert2_task().await;
        let view = ViewNumber::new(1);

        // Send 6 valid votes (below threshold)
        for i in 0..6 {
            task.accumulate_vote(make_vote2(i, view)).await;
        }
        // Send invalid-signature votes — should be rejected, not reaching threshold
        for i in 6..NUM_NODES {
            task.accumulate_vote(make_invalid_vote2(i, view)).await;
        }

        assert_no_certs(&mut task).await;
    }

    /// Votes with invalid signatures are rejected and do not count.
    #[tokio::test]
    async fn test_cert2_invalid_signature_recovery() {
        let mut task = setup_cert2_task().await;
        let view = ViewNumber::new(1);
        let epoch = EpochNumber::genesis();

        // Send 6 valid votes (below threshold)
        for i in 0..6 {
            task.accumulate_vote(make_vote2(i, view)).await;
        }
        // Send invalid-signature votes — should be rejected, not reaching threshold
        for i in 6..8 {
            task.accumulate_vote(make_invalid_vote2(i, view)).await;
        }
        assert_no_certs(&mut task).await;

        task.accumulate_vote(make_vote2(9, view)).await;

        let cert = timeout(CERT_TIMEOUT, task.next()).await.unwrap().unwrap();
        assert_no_certs(&mut task).await;
        let membership = mock_membership().await;
        let epoch_membership = membership.membership_for_epoch(Some(epoch)).await.unwrap();
        verify_cert(&cert, &vote_2_data(), &epoch_membership).await;
    }

    /// Channel closed before threshold means no certificate is produced.
    #[tokio::test]
    async fn test_cert2_channel_closed_early() {
        let mut task = setup_cert2_task().await;
        let view = ViewNumber::new(1);

        for i in 0..3 {
            task.accumulate_vote(make_vote2(i, view)).await;
        }
        assert_no_certs(&mut task).await;
    }

    // ==================== Mixed / advanced scenarios ====================

    /// Only the view that reaches threshold gets a certificate; others don't.
    #[tokio::test]
    async fn test_cert2_partial_views_only_complete_one_certifies() {
        let mut task = setup_cert2_task().await;

        let complete_view = ViewNumber::new(1);
        let partial_view = ViewNumber::new(2);

        // Send threshold votes for the complete view
        for i in 0..THRESHOLD {
            task.accumulate_vote(make_vote2(i, complete_view)).await;
        }

        // Send fewer than threshold for the partial view
        for i in 0..3 {
            task.accumulate_vote(make_vote2(i, partial_view)).await;
        }

        // Wait for the one expected certificate
        let cert = timeout(CERT_TIMEOUT, task.next()).await.unwrap().unwrap();
        assert_no_certs(&mut task).await;
        assert_eq!(cert.view_number(), complete_view);
    }

    /// Extra votes beyond threshold for the same view do not produce a second certificate.
    #[tokio::test]
    async fn test_cert2_extra_votes_after_threshold_no_duplicate_cert() {
        let mut task = setup_cert2_task().await;
        let view = ViewNumber::new(1);

        // Send all 10 votes (more than threshold of 7)
        for i in 0..NUM_NODES {
            task.accumulate_vote(make_vote2(i, view)).await;
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
        let mut task = setup_cert2_task().await;
        let view = ViewNumber::new(1);

        // Send 6 votes for one leaf commitment
        for i in 0..6 {
            task.accumulate_vote(make_vote2(i, view)).await;
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
            let vote =
                SimpleVote::create_signed_vote(data, view, &pub_key, &priv_key, &upgrade_lock())
                    .expect("Failed to sign vote");
            task.accumulate_vote(vote).await;
        }
        assert_no_certs(&mut task).await;
    }
}
