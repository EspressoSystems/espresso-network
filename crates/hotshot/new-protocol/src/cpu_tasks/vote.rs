use std::collections::BTreeMap;

use committable::{Commitment, Committable};
use hotshot::types::SignatureKey;
use hotshot_types::{
    data::ViewNumber,
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    simple_vote::{HasEpoch, VersionedVoteData},
    stake_table::StakeTableEntries,
    traits::node_implementation::NodeType,
    vote::{Certificate, Vote, VoteAccumulator},
};
use tokio::{
    spawn,
    sync::mpsc::{self},
};
use tracing::{instrument, warn};

use crate::helpers::upgrade_lock;

pub(super) type CertFut = std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>;

pub(super) struct VoteCollectionTask<TYPES: NodeType, V, C> {
    per_view: BTreeMap<ViewNumber, mpsc::Sender<V>>,
    rx: mpsc::Receiver<V>,
    epoch_membership_coordinator: EpochMembershipCoordinator<TYPES>,
    upgrade_lock: UpgradeLock<TYPES>,
    internal_tx: mpsc::Sender<C>,
    internal_rx: mpsc::Receiver<C>,
}

impl<TYPES, V, C> VoteCollectionTask<TYPES, V, C>
where
    TYPES: NodeType,
    V: Vote<TYPES> + HasEpoch + Send + Sync + 'static,
    C: Certificate<TYPES, V::Commitment, Voteable = V::Commitment> + Send + Sync + 'static,
{
    #[instrument(level = "debug", skip_all)]
    pub fn new(
        rx: mpsc::Receiver<V>,
        epoch_membership_coordinator: EpochMembershipCoordinator<TYPES>,
        upgrade_lock: UpgradeLock<TYPES>,
    ) -> Self {
        let (internal_tx, internal_rx) = mpsc::channel(100);
        Self {
            per_view: BTreeMap::new(),
            rx,
            epoch_membership_coordinator,
            upgrade_lock,
            internal_tx,
            internal_rx,
        }
    }

    #[instrument(level = "debug", skip_all)]
    pub async fn run(mut self, on_cert: impl Fn(C) -> CertFut + Send + 'static) {
        loop {
            tokio::select! {
                Some(vote) = self.rx.recv() => {
                    let view = vote.view_number();
                    let tx = self.per_view.entry(view).or_insert_with(|| {
                        let (tx, rx) = mpsc::channel(100);
                        let accumulator = VoteAccumulator::new(self.upgrade_lock.clone());
                        let membership_coordinator = self.epoch_membership_coordinator.clone();
                        let internal_tx = self.internal_tx.clone();
                        spawn(Self::run_per_view(view, rx, accumulator, membership_coordinator, internal_tx));
                        tx
                    });
                    let _ = tx.send(vote).await;
                },
                Some(cert) = self.internal_rx.recv() => {
                    self.per_view.remove(&cert.view_number());
                    on_cert(cert).await;
                },
                else => break,
            }
        }
    }

    #[instrument(level = "debug", skip_all)]
    async fn run_per_view(
        view: ViewNumber,
        mut rx: mpsc::Receiver<V>,
        mut accumulator: VoteAccumulator<TYPES, V, C>,
        membership_coordinator: EpochMembershipCoordinator<TYPES>,
        internal_tx: mpsc::Sender<C>,
    ) {
        let mut votes = Vec::new();
        while let Some(vote) = rx.recv().await {
            let epoch = vote.epoch();
            let Ok(membership) = membership_coordinator.membership_for_epoch(epoch).await else {
                continue;
            };
            if let Some(cert) = accumulator.accumulate(&vote, membership.clone()).await {
                let stake_table = membership.stake_table().await;
                let threshold = membership.success_threshold().await;
                match cert.is_valid_cert(
                    &StakeTableEntries::<TYPES>::from(stake_table).0,
                    threshold,
                    &upgrade_lock(),
                ) {
                    Ok(()) => {
                        let _ = internal_tx.send(cert).await;
                        return;
                    },
                    Err(e) => {
                        warn!("Invalid certificate formed: {e}");
                        votes.push(vote);
                        // Recover the good votes, this takes a long time
                        votes.retain(|v: &V| {
                            let vote_commitment =
                                Self::generate_vote_commitment(v, &upgrade_lock());

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
                                let _ = internal_tx.send(cert).await;
                                return;
                            }
                        }
                    },
                }
            } else {
                votes.push(vote);
            }
        }
    }
    fn generate_vote_commitment(
        vote: &V,
        upgrade_lock: &UpgradeLock<TYPES>,
    ) -> Option<Commitment<VersionedVoteData<TYPES, V::Commitment>>> {
        match VersionedVoteData::new(vote.date().clone(), vote.view_number(), upgrade_lock) {
            Ok(data) => Some(data.commit()),
            Err(e) => {
                tracing::warn!("Failed to generate versioned vote data: {e}");
                None
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use committable::Committable;
    use hotshot::types::BLSPubKey;
    use hotshot_example_types::node_types::TestTypes;
    use hotshot_types::{
        data::{EpochNumber, ViewNumber},
        epoch_membership::EpochMembership,
        simple_vote::{HasEpoch, QuorumData2, QuorumVote2, SimpleVote, VersionedVoteData},
        stake_table::StakeTableEntries,
        traits::signature_key::SignatureKey,
        vote::{Certificate, Vote},
    };
    use tokio::sync::mpsc;

    use super::VoteCollectionTask;
    use crate::{
        helpers::upgrade_lock,
        message::{Certificate1, Certificate2, Vote2, Vote2Data},
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
    async fn setup_cert1_task() -> (
        mpsc::Sender<QuorumVote2<TestTypes>>,
        mpsc::Receiver<(ViewNumber, Certificate1<TestTypes>)>,
        tokio::task::JoinHandle<()>,
    ) {
        setup_task::<QuorumVote2<TestTypes>, Certificate1<TestTypes>>().await
    }
    async fn setup_cert2_task() -> (
        mpsc::Sender<Vote2<TestTypes>>,
        mpsc::Receiver<(ViewNumber, Certificate2<TestTypes>)>,
        tokio::task::JoinHandle<()>,
    ) {
        setup_task::<Vote2<TestTypes>, Certificate2<TestTypes>>().await
    }

    /// Spawn a VoteCollectionTask for Certificate2.
    async fn setup_task<
        V: Vote<TestTypes> + HasEpoch + Send + Sync + 'static,
        C: Certificate<TestTypes, V::Commitment, Voteable = V::Commitment> + Send + Sync + 'static,
    >() -> (
        mpsc::Sender<V>,
        mpsc::Receiver<(ViewNumber, C)>,
        tokio::task::JoinHandle<()>,
    ) {
        let membership = mock_membership().await;
        let (vote_tx, vote_rx) = mpsc::channel(100);
        let task = VoteCollectionTask::<TestTypes, V, C>::new(vote_rx, membership, upgrade_lock());
        let (cert_tx, cert_rx) = mpsc::channel(100);
        let handle = tokio::spawn(async move {
            task.run(move |cert| {
                let cert_tx = cert_tx.clone();
                let view = cert.view_number();
                Box::pin(async move {
                    let _ = cert_tx.send((view, cert)).await;
                })
            })
            .await;
        });
        (vote_tx, cert_rx, handle)
    }

    /// Wait for exactly `expected` certificates, then abort the task.
    async fn collect_certs<T: std::fmt::Debug>(
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
    async fn assert_no_certs<T: std::fmt::Debug>(cert_rx: &mut mpsc::Receiver<T>) {
        let result = tokio::time::timeout(NO_CERT_TIMEOUT, cert_rx.recv()).await;
        match result {
            Err(_) => { /* timeout — good, no cert produced */ },
            Ok(None) => { /* channel closed — also fine */ },
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
        let (vote_tx, mut cert_rx, handle) = setup_cert1_task().await;
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
            vote_tx
                .send(make_quorum_vote(i, view, epoch))
                .await
                .unwrap();
        }

        let certs = collect_certs(&mut cert_rx, 1).await;
        assert_eq!(certs.len(), 1, "Expected exactly one Certificate1");
        assert_eq!(certs[0].0, view);

        let membership = mock_membership().await;
        let epoch_membership = membership.membership_for_epoch(Some(epoch)).await.unwrap();
        verify_cert(&certs[0].1, &expected_data, &epoch_membership).await;
        handle.abort();
    }

    /// Sending votes for multiple views produces a valid certificate for each view,
    /// each with data commitment matching the votes.
    #[tokio::test]
    async fn test_cert1_multiple_views_parallel() {
        let (vote_tx, mut cert_rx, handle) = setup_cert1_task().await;
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
                vote_tx
                    .send(make_quorum_vote(i, view, epoch))
                    .await
                    .unwrap();
            }
        }
        let certs = collect_certs(&mut cert_rx, 3).await;
        handle.abort();
        assert_eq!(certs.len(), 3, "Expected one Certificate1 per view");
        let mut cert_views: Vec<_> = certs.iter().map(|(v, _)| *v).collect();
        cert_views.sort();
        assert_eq!(cert_views, views);

        let membership = mock_membership().await;
        let epoch_membership = membership.membership_for_epoch(Some(epoch)).await.unwrap();
        for (_, cert) in &certs {
            verify_cert(cert, &expected_data, &epoch_membership).await;
        }
    }

    // ==================== Certificate2 (Vote2) happy path ====================

    /// Sending enough Vote2s for a single view produces a valid Certificate2
    /// whose data commitment matches the votes.
    #[tokio::test]
    async fn test_cert2_single_view_happy_path() {
        let (vote_tx, mut cert_rx, handle) = setup_cert2_task().await;
        let view = ViewNumber::new(1);
        let epoch = EpochNumber::genesis();
        let expected_data = vote_2_data();

        for i in 0..THRESHOLD {
            vote_tx.send(make_vote2(i, view)).await.unwrap();
        }

        let certs = collect_certs(&mut cert_rx, 1).await;
        handle.abort();
        assert_eq!(certs.len(), 1, "Expected exactly one Certificate2");
        assert_eq!(certs[0].0, view);

        let membership = mock_membership().await;
        let epoch_membership = membership.membership_for_epoch(Some(epoch)).await.unwrap();
        verify_cert(&certs[0].1, &expected_data, &epoch_membership).await;
    }

    /// Sending votes for multiple views in parallel produces valid certificates for each,
    /// each with data commitment matching the votes.
    #[tokio::test]
    async fn test_cert2_multiple_views_parallel() {
        let (vote_tx, mut cert_rx, handle) = setup_cert2_task().await;
        let epoch = EpochNumber::genesis();
        let expected_data = vote_2_data();

        let views = [ViewNumber::new(5), ViewNumber::new(6), ViewNumber::new(7)];

        for i in 0..THRESHOLD {
            for &view in &views {
                vote_tx.send(make_vote2(i, view)).await.unwrap();
            }
        }

        let certs = collect_certs(&mut cert_rx, 3).await;
        handle.abort();
        assert_eq!(certs.len(), 3, "Expected one Certificate2 per view");
        let mut cert_views: Vec<_> = certs.iter().map(|(v, _)| *v).collect();
        cert_views.sort();
        assert_eq!(cert_views, views);

        let membership = mock_membership().await;
        let epoch_membership = membership.membership_for_epoch(Some(epoch)).await.unwrap();
        for (_, cert) in &certs {
            verify_cert(cert, &expected_data, &epoch_membership).await;
        }
    }

    // ==================== Certificate1 failure cases ====================

    /// Fewer than threshold votes do not produce a certificate.
    #[tokio::test]
    async fn test_cert1_below_threshold_no_certificate() {
        let (vote_tx, mut cert_rx, handle) = setup_cert1_task().await;
        let view = ViewNumber::new(1);
        let epoch = EpochNumber::genesis();

        for i in 0..(THRESHOLD - 1) {
            vote_tx
                .send(make_quorum_vote(i, view, epoch))
                .await
                .unwrap();
        }

        drop(vote_tx);
        assert_no_certs(&mut cert_rx).await;
        handle.abort();
    }

    /// Duplicate votes from the same signer do not count toward threshold.
    #[tokio::test]
    async fn test_cert1_duplicate_votes_ignored() {
        let (vote_tx, mut cert_rx, handle) = setup_cert1_task().await;
        let view = ViewNumber::new(1);
        let epoch = EpochNumber::genesis();

        // Send 6 unique votes (below threshold of 7)
        for i in 0..6 {
            vote_tx
                .send(make_quorum_vote(i, view, epoch))
                .await
                .unwrap();
        }
        // Send duplicates of node 0 — should not push us over threshold
        for _ in 0..5 {
            vote_tx
                .send(make_quorum_vote(0, view, epoch))
                .await
                .unwrap();
        }

        drop(vote_tx);
        assert_no_certs(&mut cert_rx).await;
        handle.abort();
    }

    // ==================== Certificate2 failure cases ====================

    /// Fewer than threshold Vote2s do not produce a Certificate2.
    #[tokio::test]
    async fn test_cert2_below_threshold_no_certificate() {
        let (vote_tx, mut cert_rx, handle) = setup_cert2_task().await;
        let view = ViewNumber::new(1);
        let epoch = EpochNumber::genesis();

        for i in 0..(THRESHOLD - 1) {
            vote_tx.send(make_vote2(i, view)).await.unwrap();
        }

        drop(vote_tx);
        assert_no_certs(&mut cert_rx).await;
        handle.abort();
    }

    /// Duplicate Vote2s from the same signer do not count toward threshold.
    #[tokio::test]
    async fn test_cert2_duplicate_votes_ignored() {
        let (vote_tx, mut cert_rx, handle) = setup_cert2_task().await;
        let view = ViewNumber::new(1);
        let epoch = EpochNumber::genesis();

        // Send 6 unique votes (below threshold of 7)
        for i in 0..6 {
            vote_tx.send(make_vote2(i, view)).await.unwrap();
        }
        // Repeat node 0 votes — should not reach threshold
        for _ in 0..5 {
            vote_tx.send(make_vote2(0, view)).await.unwrap();
        }

        drop(vote_tx);
        assert_no_certs(&mut cert_rx).await;
        handle.abort();
    }

    /// Votes with invalid signatures are rejected and do not count.
    #[tokio::test]
    async fn test_cert2_invalid_signature_rejected() {
        let (vote_tx, mut cert_rx, handle) = setup_cert2_task().await;
        let view = ViewNumber::new(1);
        let epoch = EpochNumber::genesis();

        // Send 6 valid votes (below threshold)
        for i in 0..6 {
            vote_tx.send(make_vote2(i, view)).await.unwrap();
        }
        // Send invalid-signature votes — should be rejected, not reaching threshold
        for i in 6..NUM_NODES {
            vote_tx.send(make_invalid_vote2(i, view)).await.unwrap();
        }

        drop(vote_tx);
        assert_no_certs(&mut cert_rx).await;
        handle.abort();
    }

    /// Votes with invalid signatures are rejected and do not count.
    #[tokio::test]
    async fn test_cert2_invalid_signature_recovery() {
        let (vote_tx, mut cert_rx, handle) = setup_cert2_task().await;
        let view = ViewNumber::new(1);
        let epoch = EpochNumber::genesis();

        // Send 6 valid votes (below threshold)
        for i in 0..6 {
            vote_tx.send(make_vote2(i, view)).await.unwrap();
        }
        // Send invalid-signature votes — should be rejected, not reaching threshold
        for i in 6..8 {
            vote_tx.send(make_invalid_vote2(i, view)).await.unwrap();
        }
        assert_no_certs(&mut cert_rx).await;

        vote_tx.send(make_vote2(9, view)).await.unwrap();

        let certs = collect_certs(&mut cert_rx, 1).await;
        assert_eq!(certs.len(), 1, "Expected exactly one Certificate2");
        let membership = mock_membership().await;
        let epoch_membership = membership.membership_for_epoch(Some(epoch)).await.unwrap();
        let expected_data = vote_2_data();
        verify_cert(&certs[0].1, &expected_data, &epoch_membership).await;

        drop(vote_tx);
        handle.abort();
    }

    /// Channel closed before threshold means no certificate is produced.
    #[tokio::test]
    async fn test_cert2_channel_closed_early() {
        let (vote_tx, mut cert_rx, handle) = setup_cert2_task().await;
        let view = ViewNumber::new(1);
        let epoch = EpochNumber::genesis();

        for i in 0..3 {
            vote_tx.send(make_vote2(i, view)).await.unwrap();
        }
        drop(vote_tx);
        assert_no_certs(&mut cert_rx).await;
        handle.abort();
    }

    // ==================== Mixed / advanced scenarios ====================

    /// Only the view that reaches threshold gets a certificate; others don't.
    #[tokio::test]
    async fn test_cert2_partial_views_only_complete_one_certifies() {
        let (vote_tx, mut cert_rx, handle) = setup_cert2_task().await;
        let epoch = EpochNumber::genesis();

        let complete_view = ViewNumber::new(1);
        let partial_view = ViewNumber::new(2);

        // Send threshold votes for the complete view
        for i in 0..THRESHOLD {
            vote_tx.send(make_vote2(i, complete_view)).await.unwrap();
        }

        // Send fewer than threshold for the partial view
        for i in 0..3 {
            vote_tx.send(make_vote2(i, partial_view)).await.unwrap();
        }

        // Wait for the one expected certificate
        let certs = collect_certs(&mut cert_rx, 1).await;
        handle.abort();
        assert_eq!(certs.len(), 1, "Only one view should produce a certificate");
        assert_eq!(certs[0].0, complete_view);
    }

    /// Extra votes beyond threshold for the same view do not produce a second certificate.
    #[tokio::test]
    async fn test_cert2_extra_votes_after_threshold_no_duplicate_cert() {
        let (vote_tx, mut cert_rx, handle) = setup_cert2_task().await;
        let view = ViewNumber::new(1);
        let epoch = EpochNumber::genesis();

        // Send all 10 votes (more than threshold of 7)
        for i in 0..NUM_NODES {
            vote_tx.send(make_vote2(i, view)).await.unwrap();
        }

        // Should get exactly one cert, then confirm no more arrive
        let cert = tokio::time::timeout(CERT_TIMEOUT, cert_rx.recv())
            .await
            .expect("Timed out waiting for certificate")
            .expect("Channel closed");
        assert_eq!(cert.0, view);

        // Confirm no second certificate
        let extra = tokio::time::timeout(NO_CERT_TIMEOUT, cert_rx.recv()).await;
        assert!(
            extra.is_err() || extra.unwrap().is_none(),
            "Should not produce a second certificate"
        );
        handle.abort();
    }

    /// Votes for different data commitments on the same view do not combine.
    #[tokio::test]
    async fn test_cert2_conflicting_data_same_view_no_certificate() {
        let (vote_tx, mut cert_rx, handle) = setup_cert2_task().await;
        let view = ViewNumber::new(1);
        let epoch = EpochNumber::genesis();

        // Send 6 votes for one leaf commitment
        for i in 0..6 {
            vote_tx.send(make_vote2(i, view)).await.unwrap();
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
            vote_tx.send(vote).await.unwrap();
        }

        drop(vote_tx);
        assert_no_certs(&mut cert_rx).await;
        handle.abort();
    }
}
