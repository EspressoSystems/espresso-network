use std::{
    cmp::max,
    collections::{BTreeMap, BTreeSet},
};

use anyhow::Context;
use committable::Commitment;
use hotshot_types::{
    data::{
        EpochNumber, Leaf2, QuorumProposal2, QuorumProposalWrapper, VidCommitment, VidCommitment2,
        VidDisperse2, VidDisperseShare2, ViewChangeEvidence2, ViewNumber,
        vid_disperse::vid_total_weight,
    },
    epoch_membership::EpochMembershipCoordinator,
    simple_certificate::{TimeoutCertificate2, ViewSyncFinalizeCertificate2},
    simple_vote::{HasEpoch, QuorumData2, SimpleVote},
    stake_table::StakeTableEntries,
    traits::{
        block_contents::BlockHeader, node_implementation::NodeType, signature_key::SignatureKey,
    },
    vote::{self, Certificate, HasViewNumber},
};
use tokio::sync::mpsc::Receiver;

use crate::{
    coordinator::handle::CoordinatorHandle,
    events::{ConsensusEvent, RequestMessageSender},
    helpers::{proposal_commitment, upgrade_lock},
    message::{Certificate1, Certificate2, ProposalMessage, Vote1, Vote2Data},
};

pub(crate) struct Consensus<TYPES: NodeType> {
    proposals: BTreeMap<ViewNumber, QuorumProposal2<TYPES>>,
    vid_shares: BTreeMap<ViewNumber, VidDisperseShare2<TYPES>>,
    states_verified: BTreeMap<ViewNumber, Commitment<Leaf2<TYPES>>>,
    blocks_reconstructed: BTreeMap<ViewNumber, VidCommitment2>,
    blocks: BTreeMap<ViewNumber, TYPES::BlockPayload>,
    vid_disperses: BTreeMap<ViewNumber, VidDisperse2<TYPES>>,
    certs: BTreeMap<ViewNumber, Certificate1<TYPES>>,
    certs2: BTreeMap<ViewNumber, Certificate2<TYPES>>,
    timeout_certs: BTreeMap<ViewNumber, TimeoutCertificate2<TYPES>>,
    view_sync_certs: BTreeMap<ViewNumber, ViewSyncFinalizeCertificate2<TYPES>>,
    locked_qc: Option<Certificate1<TYPES>>,
    headers: BTreeMap<ViewNumber, TYPES::BlockHeader>,
    last_decided_view: ViewNumber,

    voted_1_views: BTreeSet<ViewNumber>,
    voted_2_views: BTreeSet<ViewNumber>,

    timeout_view: ViewNumber,

    event_rx: Receiver<ConsensusEvent<TYPES>>,
    coordinator_handle: CoordinatorHandle<TYPES>,

    // TODO: We need a next epoch stake table to handle the transition
    // And a way to set these stake tables, probably an event from coordinator
    stake_table_coordinator: EpochMembershipCoordinator<TYPES>,

    public_key: TYPES::SignatureKey,
    private_key: <TYPES::SignatureKey as SignatureKey>::PrivateKey,
}

impl<TYPES: NodeType> Consensus<TYPES> {
    pub fn new(
        event_rx: Receiver<ConsensusEvent<TYPES>>,
        coordinator_handle: CoordinatorHandle<TYPES>,
        membership_coordinator: EpochMembershipCoordinator<TYPES>,
        public_key: TYPES::SignatureKey,
        private_key: <TYPES::SignatureKey as SignatureKey>::PrivateKey,
    ) -> Self {
        Self {
            proposals: BTreeMap::new(),
            vid_disperses: BTreeMap::new(),
            blocks: BTreeMap::new(),
            states_verified: BTreeMap::new(),
            blocks_reconstructed: BTreeMap::new(),
            certs: BTreeMap::new(),
            certs2: BTreeMap::new(),
            timeout_certs: BTreeMap::new(),
            view_sync_certs: BTreeMap::new(),
            locked_qc: None,
            last_decided_view: ViewNumber::genesis(),
            event_rx,
            coordinator_handle,
            headers: BTreeMap::new(),
            public_key,
            timeout_view: ViewNumber::genesis(),
            stake_table_coordinator: membership_coordinator,
            voted_1_views: BTreeSet::new(),
            voted_2_views: BTreeSet::new(),
            private_key,
            vid_shares: BTreeMap::new(),
        }
    }

    async fn is_leader(&self, view: ViewNumber, epoch: EpochNumber) -> bool {
        let Ok(stake_table) = self
            .stake_table_coordinator
            .membership_for_epoch(Some(epoch))
            .await
        else {
            return false;
        };
        let Ok(leader) = stake_table.leader(view).await else {
            return false;
        };
        leader == self.public_key
    }

    async fn handle_event(&mut self, event: ConsensusEvent<TYPES>) -> Option<()> {
        let view = event.view_number();
        if view <= self.timeout_view {
            return None;
        }
        match event {
            ConsensusEvent::Proposal(proposal) => self.handle_proposal(proposal).await?,
            ConsensusEvent::Certificate1(certificate) => {
                self.handle_certificate1(certificate).await?
            },
            ConsensusEvent::Certificate2(certificate) => {
                self.handle_certificate2(certificate).await?
            },
            ConsensusEvent::TimeoutCertificate(certificate) => {
                self.handle_timeout_certificate(certificate).await?;
            },
            ConsensusEvent::ViewSyncCertificate(certificate) => {
                self.handle_view_sync_certificate(certificate).await?;
            },
            ConsensusEvent::BlockReconstructed(view, vid_commitment) => {
                self.blocks_reconstructed.insert(view, vid_commitment);
            },
            ConsensusEvent::StateVerified(state_response) => {
                self.states_verified
                    .insert(state_response.view, state_response.commitment);
            },
            ConsensusEvent::HeaderCreated(view, header) => {
                self.headers.insert(view, header);
            },
            ConsensusEvent::StateVerificationFailed(state_response) => {
                if let Some(proposal) = self.proposals.remove(&state_response.view)
                    && proposal_commitment(&proposal) != state_response.commitment
                {
                    // Proposal we stored didn't match the failed state verification, put it back
                    self.proposals.insert(state_response.view, proposal);
                    return None;
                }
                self.vid_shares.remove(&state_response.view);
                return None;
            },
            ConsensusEvent::Timeout(view) => {
                self.handle_timeout(view);
                // we are done after timeout, don't try to vote, decide, or propose
                return None;
            },
            ConsensusEvent::BlockBuilt(view, epoch, block, metadata) => {
                self.handle_block_built(view, epoch, block, metadata)
                    .await?;
            },
            ConsensusEvent::VidDisperseCreated(view, vid_disperse) => {
                self.vid_disperses.insert(view, vid_disperse);
            },
            ConsensusEvent::Shutdown => {
                unreachable!();
            },
        }
        self.maybe_vote_1(view).await;
        self.maybe_vote_2_and_update_lock(view).await;
        self.maybe_decide(view).await;
        self.maybe_propose(view).await;
        // An event from the current view or the previous view can trigger a propose
        self.maybe_propose(view + 1).await;
        Some(())
    }

    pub async fn run(&mut self) {
        while let Some(event) = self.event_rx.recv().await {
            if matches!(event, ConsensusEvent::Shutdown) {
                break;
            }
            self.handle_event(event).await;
        }
    }

    async fn handle_block_built(
        &mut self,
        view: ViewNumber,
        epoch: EpochNumber,
        block: TYPES::BlockPayload,
        metadata: <TYPES::BlockPayload as hotshot::traits::BlockPayload<TYPES>>::Metadata,
    ) -> Option<()> {
        self.coordinator_handle
            .request_vid_disperse(view, epoch, block.clone(), metadata)
            .await
            .ok()?;
        self.blocks.insert(view, block);
        Some(())
    }

    async fn maybe_propose(&mut self, view: ViewNumber) -> Option<()> {
        let is_after_timeout =
            self.view_sync_certs.contains_key(&view) || self.timeout_certs.contains_key(&view);
        let qc = if is_after_timeout {
            self.locked_qc.as_ref()?
        } else {
            self.certs.get(&ViewNumber::from(view.saturating_sub(1)))?
        };
        let parent_view = qc.view_number();
        let proposal = self.proposals.get(&parent_view)?;
        if !self.is_leader(view, proposal.epoch?).await {
            return None;
        }

        let header = self.headers.get(&view)?;
        let block = self.blocks.get(&view)?;
        let vid_disperse = self.vid_disperses.get(&view)?;

        // TODO: Handle epoch change and properly set next epoch qc drb result and state cert
        let mut proposal = QuorumProposal2::<TYPES> {
            block_header: header.clone(),
            view_number: view,
            epoch: proposal.epoch,
            justify_qc: qc.clone(),
            next_epoch_justify_qc: None,
            upgrade_certificate: None,
            view_change_evidence: None,
            next_drb_result: None,
            state_cert: None,
        };

        // add View Change Evidence if we are after timeout
        if is_after_timeout {
            if let Some(view_sync_cert) = self.view_sync_certs.get(&view) {
                proposal.view_change_evidence =
                    Some(ViewChangeEvidence2::ViewSync(view_sync_cert.clone()));
            } else {
                proposal.view_change_evidence = Some(ViewChangeEvidence2::Timeout(
                    self.timeout_certs.get(&view)?.clone(),
                ));
            }
        }

        // TODO: Handle epoch change here

        self.coordinator_handle
            .send_message(RequestMessageSender::Proposal(
                proposal,
                vid_disperse.clone(),
            ))
            .await
            .ok()
    }

    async fn maybe_decide(&mut self, view: ViewNumber) -> Option<()> {
        if view <= self.last_decided_view {
            return None;
        }
        let cert2 = self.certs2.get(&view)?;
        let proposal = self.proposals.get(&view)?;
        let proposal_commit = proposal_commitment(proposal);
        if cert2.data.leaf_commit != proposal_commit {
            return None;
        }
        // we have a second certificate, and matching proposal, it is decided.
        let leaf = Leaf2::from_quorum_proposal(&QuorumProposalWrapper::from(proposal.clone()));
        self.last_decided_view = max(self.last_decided_view, leaf.view_number());
        let mut decided = vec![leaf];

        let mut parent_view = proposal.justify_qc.view_number();
        let mut parent_commit = proposal.justify_qc.data.leaf_commit;

        while let Some(proposal) = self.proposals.get(&parent_view) {
            let proposal_commit = proposal_commitment(proposal);
            if proposal_commit != parent_commit {
                break;
            }
            let leaf = Leaf2::from_quorum_proposal(&QuorumProposalWrapper::from(proposal.clone()));
            decided.push(leaf);
            parent_view = proposal.justify_qc.view_number();
            parent_commit = proposal.justify_qc.data.leaf_commit;
        }
        self.coordinator_handle.send_decided(decided).await.ok()?;
        Some(())
    }

    async fn maybe_vote_1(&mut self, view: ViewNumber) -> Option<()> {
        if self.voted_1_views.contains(&view) {
            return None;
        }
        let state_commitment = self.states_verified.get(&view)?;
        let proposal = self.proposals.get(&view)?;
        let vid_share = self.vid_shares.get(&view)?;

        // Verify parent chain unless justify_qc is the genesis QC
        let parent_view = proposal.justify_qc.view_number();

        // We don't need the genesis block to be reconstructed or verified
        // or the genesis qc to be verified
        if parent_view != ViewNumber::genesis() {
            // Verify we have the block for the QC on this commitment
            let block_commitment = self.blocks_reconstructed.get(&parent_view)?;
            let prev_proposal = self.proposals.get(&parent_view)?;
            let VidCommitment::V2(prev_block_commitment) =
                prev_proposal.block_header.payload_commitment()
            else {
                return None;
            };
            if block_commitment != &prev_block_commitment {
                return None;
            }

            if proposal.justify_qc.data().leaf_commit != proposal_commitment(prev_proposal) {
                return None;
            }
        }

        let proposal_commit = proposal_commitment(proposal);

        // Verify the state commitment matches the proposal
        if state_commitment != &proposal_commit {
            return None;
        }

        let inner_vote = SimpleVote::create_signed_vote(
            QuorumData2 {
                leaf_commit: proposal_commit,
                epoch: proposal.epoch,
                block_number: Some(proposal.block_header.block_number()),
            },
            view,
            &self.public_key,
            &self.private_key,
            &upgrade_lock::<TYPES>(),
        )
        .unwrap();
        let vote = Vote1 {
            vote: inner_vote,
            vid_share: vid_share.clone(),
        };
        self.coordinator_handle
            .send_message(RequestMessageSender::Vote1(vote))
            .await
            .ok()?;
        self.voted_1_views.insert(view);
        Some(())
    }

    async fn maybe_vote_2_and_update_lock(&mut self, view: ViewNumber) -> Option<()> {
        if self.voted_2_views.contains(&view) {
            return None;
        }

        // we have a proposal, reconstructed block, and first certificate for this view
        let reconstructed_block_commitment = self.blocks_reconstructed.get(&view)?;
        let cert1 = self.certs.get(&view)?;
        let proposal = self.proposals.get(&view)?;

        let proposal_commit = proposal_commitment(proposal);

        // The certificate must match the proposal
        if cert1.data.leaf_commit != proposal_commit {
            return None;
        }
        // The proposal block commitment must match the reconstructed block commitment
        let VidCommitment::V2(proposal_block_commitment) =
            proposal.block_header.payload_commitment()
        else {
            return None;
        };
        if &proposal_block_commitment != reconstructed_block_commitment {
            return None;
        }

        // We have a valid certificate, proposal, and reconstructed block
        // We can now update the lock and vote
        if self
            .locked_qc
            .as_mut()
            .is_none_or(|locked_qc| locked_qc.view_number() < cert1.view_number())
        {
            self.locked_qc = Some(cert1.clone());
        }

        let vote = SimpleVote::create_signed_vote(
            Vote2Data {
                leaf_commit: proposal_commit,
                epoch: proposal.epoch?,
                block_number: proposal.block_header.block_number(),
            },
            view,
            &self.public_key,
            &self.private_key,
            &upgrade_lock::<TYPES>(),
        )
        .unwrap();
        self.coordinator_handle
            .send_message(RequestMessageSender::Vote2(vote))
            .await
            .ok()?;
        self.voted_2_views.insert(view);
        Some(())
    }

    async fn handle_proposal(&mut self, proposal: ProposalMessage<TYPES>) -> Option<()> {
        let view = proposal.view_number();
        // Verify the proposal is signed by the leader
        proposal
            .proposal
            .validate_signature(&self.stake_table_coordinator)
            .await
            .ok()?;

        let vid_share = proposal.vid_share;
        let proposal = proposal.proposal.data;
        let epoch = proposal.epoch?;

        Self::vid_matches_proposal(&vid_share, &proposal)?;

        //Verify the VID share
        if !vid_share.is_consistent() {
            return None;
        }

        self.verify_vid_share(&vid_share, epoch).await?;

        // Verify the proposal is valid
        self.validate_safety(&proposal)?;
        self.proposals.insert(view, proposal.clone());
        let payload_size = vid_share.payload_byte_len();
        self.vid_shares.insert(view, vid_share);

        // Now ask for the state to verify the header of the proposal
        self.coordinator_handle
            .request_state(proposal.clone(), payload_size)
            .await
            .ok()?;
        // And if we are leader next, ask for a header
        if self.is_leader(view + 1, epoch).await {
            self.coordinator_handle
                .request_block_and_header(proposal.clone(), view + 1, epoch)
                .await
                .ok()?;
        }
        Some(())
    }

    fn vid_matches_proposal(
        vid_share: &VidDisperseShare2<TYPES>,
        proposal: &QuorumProposal2<TYPES>,
    ) -> Option<()> {
        let VidCommitment::V2(vid_comm) = proposal.block_header.payload_commitment() else {
            return None;
        };
        if vid_comm != vid_share.payload_commitment {
            return None;
        }
        Some(())
    }

    async fn verify_vid_share(
        &self,
        vid_share: &VidDisperseShare2<TYPES>,
        epoch: EpochNumber,
    ) -> Option<()> {
        let stake_table = self
            .stake_table_coordinator
            .membership_for_epoch(Some(epoch))
            .await
            .ok()?;
        let total_weight = vid_total_weight(&stake_table.stake_table().await, Some(epoch));
        if !vid_share.verify(total_weight) {
            return None;
        }
        Some(())
    }

    fn validate_safety(&self, proposal: &QuorumProposal2<TYPES>) -> Option<()> {
        let Some(locked_qc) = self.locked_qc.as_ref() else {
            // Locked QC is not set which means it is at genesis
            return Some(());
        };
        let liveness_check = proposal.justify_qc.view_number() > locked_qc.view_number();
        let safety_check = proposal
            .justify_qc
            .data_commitment(&upgrade_lock::<TYPES>())
            .ok()?
            == locked_qc.data_commitment(&upgrade_lock::<TYPES>()).ok()?;
        if !safety_check && !liveness_check {
            return None;
        }
        Some(())
    }

    async fn verify_cert<T>(
        &self,
        cert: &impl vote::Certificate<TYPES, T>,
        epoch: EpochNumber,
    ) -> anyhow::Result<()> {
        let stake_table = self
            .stake_table_coordinator
            .membership_for_epoch(Some(epoch))
            .await?;
        let entries = StakeTableEntries::<TYPES>::from(stake_table.stake_table().await).0;
        let threshold = stake_table.success_threshold().await;
        cert.is_valid_cert(&entries, threshold, &upgrade_lock::<TYPES>())
            .context("invalid threshold signature")
    }

    async fn handle_certificate1(&mut self, certificate: Certificate1<TYPES>) -> Option<()> {
        self.verify_cert(&certificate, certificate.epoch()?)
            .await
            .ok()?;
        self.certs.insert(certificate.view_number(), certificate);
        Some(())
    }

    async fn handle_certificate2(&mut self, certificate: Certificate2<TYPES>) -> Option<()> {
        self.verify_cert(&certificate, certificate.epoch()?)
            .await
            .ok()?;
        self.certs2.insert(certificate.view_number(), certificate);
        Some(())
    }

    async fn handle_timeout_certificate(
        &mut self,
        certificate: TimeoutCertificate2<TYPES>,
    ) -> Option<()> {
        let view = certificate.view_number() + 1;
        let epoch = certificate.epoch()?;
        self.timeout_certs.insert(view, certificate);
        if self.is_leader(view, epoch).await {
            let locked_view = self.locked_qc.as_ref().map(|qc| qc.view_number())?;
            let proposal = self.proposals.get(&locked_view)?;
            self.coordinator_handle
                .request_block_and_header(proposal.clone(), view, epoch)
                .await
                .ok()?;
            Some(())
        } else {
            None
        }
    }

    async fn handle_view_sync_certificate(
        &mut self,
        certificate: ViewSyncFinalizeCertificate2<TYPES>,
    ) -> Option<()> {
        let view = certificate.view_number();
        let epoch = certificate.epoch()?;
        self.view_sync_certs.insert(view, certificate);
        if self.is_leader(view, epoch).await {
            let locked_view = self.locked_qc.as_ref().map(|qc| qc.view_number())?;
            let proposal = self.proposals.get(&locked_view)?;
            self.coordinator_handle
                .request_block_and_header(proposal.clone(), view + 1, epoch)
                .await
                .ok()?;
            Some(())
        } else {
            None
        }
    }

    fn handle_timeout(&mut self, view: ViewNumber) {
        self.timeout_view = view;
        // TODO: clear_view(view);
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use hotshot::{traits::ValidatedState, types::BLSPubKey};
    use hotshot_example_types::{node_types::TestTypes, state_types::TestValidatedState};
    use hotshot_types::traits::signature_key::SignatureKey;
    use tokio::{sync::mpsc::Sender, task::JoinHandle};

    use super::*;
    use crate::{
        coordinator::mock::testing::MockCoordinator,
        events::{Action, ConsensusEvent, Event, RequestMessageSender, Update},
        test_utils::{TestData, mock_membership},
    };

    /// Test harness that spawns consensus + mock coordinator and provides
    /// helpers to send events and collect results.
    struct TestHarness {
        /// Send ConsensusEvents directly to consensus
        consensus_tx: Sender<ConsensusEvent<TestTypes>>,
        /// Send Events to the mock coordinator (for shutdown etc.)
        coordinator_handle: CoordinatorHandle<TestTypes>,
        /// Join handle for mock coordinator (collects received events)
        mock_join: JoinHandle<Vec<Event<TestTypes>>>,
    }

    impl TestHarness {
        /// Create a new test harness with the given node index (0-9).
        async fn new(node_index: u64) -> Self {
            let (public_key, private_key) =
                BLSPubKey::generated_from_seed_indexed([0; 32], node_index);
            let membership = mock_membership().await;
            let (event_tx, event_rx) = tokio::sync::mpsc::channel(100);
            let (consensus_tx, consensus_rx) = tokio::sync::mpsc::channel(100);

            let mock_coordinator = MockCoordinator {
                event_rx,
                consensus_tx: consensus_tx.clone(),
                state_tx: None,
                membership_coordinator: membership.clone(),
                received_events: Vec::new(),
            };
            let coordinator_handle = CoordinatorHandle::new(event_tx);
            let mut consensus = Consensus::new(
                consensus_rx,
                coordinator_handle.clone(),
                membership,
                public_key,
                private_key,
            );

            tokio::spawn(async move {
                consensus.run().await;
            });
            let mock_join = tokio::spawn(async move { mock_coordinator.run().await });

            Self {
                consensus_tx,
                coordinator_handle,
                mock_join,
            }
        }

        /// Send a ConsensusEvent directly to the consensus state machine.
        async fn send(&self, event: ConsensusEvent<TestTypes>) {
            self.consensus_tx.send(event).await.unwrap();
        }

        /// Send multiple ConsensusEvents in order.
        async fn send_all(&self, events: Vec<ConsensusEvent<TestTypes>>) {
            for event in events {
                self.send(event).await;
            }
        }

        /// Shut down consensus and the mock coordinator, returning all
        /// events the mock coordinator received from consensus.
        async fn shutdown(self) -> Vec<Event<TestTypes>> {
            // Send shutdown to consensus
            self.consensus_tx
                .send(ConsensusEvent::Shutdown)
                .await
                .unwrap();
            // Small delay to let consensus process shutdown and close its handle
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            // Send shutdown to mock coordinator
            self.coordinator_handle
                .send_event(Event::Action(Action::Shutdown))
                .await
                .unwrap();
            self.mock_join.await.unwrap()
        }
    }

    /// Check if received events contain a Vote1 action.
    fn has_vote1(events: &[Event<TestTypes>]) -> bool {
        events.iter().any(|e| {
            matches!(
                e,
                Event::Action(Action::SendMessage(RequestMessageSender::Vote1(_)))
            )
        })
    }

    /// Check if received events contain a Vote2 action.
    fn has_vote2(events: &[Event<TestTypes>]) -> bool {
        events.iter().any(|e| {
            matches!(
                e,
                Event::Action(Action::SendMessage(RequestMessageSender::Vote2(_)))
            )
        })
    }

    /// Check if received events contain a LeafDecided update.
    fn has_leaf_decided(events: &[Event<TestTypes>]) -> bool {
        events
            .iter()
            .any(|e| matches!(e, Event::Update(Update::LeafDecided(_))))
    }

    /// Check if received events contain a RequestState action.
    fn has_request_state(events: &[Event<TestTypes>]) -> bool {
        events
            .iter()
            .any(|e| matches!(e, Event::Action(Action::RequestState(_))))
    }

    /// Count how many Vote1 actions are in the events.
    fn count_vote1(events: &[Event<TestTypes>]) -> usize {
        events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    Event::Action(Action::SendMessage(RequestMessageSender::Vote1(_)))
                )
            })
            .count()
    }

    /// Count how many Vote2 actions are in the events.
    fn count_vote2(events: &[Event<TestTypes>]) -> usize {
        events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    Event::Action(Action::SendMessage(RequestMessageSender::Vote2(_)))
                )
            })
            .count()
    }

    /// Fresh consensus with no locked_qc accepts any proposal (genesis safety).
    #[tokio::test]
    async fn test_safety_genesis_no_lock() {
        let harness = TestHarness::new(0).await;
        let test_data = TestData::new(2).await;
        let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

        // Send proposal for view 1 — locked_qc is None, so safety passes
        harness
            .send(test_data.views[0].proposal_event(&node_key))
            .await;

        // Allow async processing
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let events = harness.shutdown().await;

        // Should have requested state verification (proposal accepted)
        assert!(
            has_request_state(&events),
            "Proposal should be accepted with no locked QC"
        );
    }

    /// Events with view <= timeout_view are silently dropped.
    #[tokio::test]
    async fn test_timeout_filters_stale_events() {
        let harness = TestHarness::new(0).await;
        let test_data = TestData::new(6).await;
        let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

        // Set timeout at view 3
        harness
            .send(ConsensusEvent::Timeout(ViewNumber::new(3)))
            .await;

        // Send stale proposal (view 2, which is <= timeout_view 3)
        harness
            .send(test_data.views[1].proposal_event(&node_key))
            .await;

        // Send fresh proposal (view 4, which is > timeout_view 3)
        harness
            .send(test_data.views[3].proposal_event(&node_key))
            .await;

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let events = harness.shutdown().await;

        // Only the fresh proposal (view 4) should generate a RequestState
        let request_states: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, Event::Action(Action::RequestState(_))))
            .collect();
        assert_eq!(
            request_states.len(),
            1,
            "Only one RequestState expected (fresh view), got {}",
            request_states.len()
        );
    }

    /// Vote1 fires for sequential views when all preconditions are met.
    #[tokio::test]
    async fn test_vote1_for_sequential_views() {
        let harness = TestHarness::new(0).await;
        let test_data = TestData::new(3).await;
        let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

        // Send proposal for view 1 (parent data setup)
        harness
            .send(test_data.views[0].proposal_event(&node_key))
            .await;
        harness
            .send(test_data.views[0].block_reconstructed_event())
            .await;

        // Send proposal for view 2 — mock auto-responds with StateVerified
        harness
            .send(test_data.views[1].proposal_event(&node_key))
            .await;

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let events = harness.shutdown().await;

        assert!(
            count_vote1(&events) == 2,
            "Vote1 should fire for sequential views"
        );
    }

    /// Vote1 fires for view 1 (genesis parent) — parent checks are skipped.
    #[tokio::test]
    async fn test_vote1_genesis_parent() {
        let harness = TestHarness::new(0).await;
        let test_data = TestData::new(2).await;
        let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

        // Send proposal for view 1 — justify_qc references genesis
        harness
            .send(test_data.views[0].proposal_event(&node_key))
            .await;

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let events = harness.shutdown().await;

        assert!(
            has_vote1(&events),
            "Vote1 should fire for view 1 with genesis parent"
        );
    }

    /// Vote2 requires Certificate1 + BlockReconstructed + Proposal.
    /// Without Certificate1, no Vote2 is sent.
    #[tokio::test]
    async fn test_vote2_missing_cert1() {
        let harness = TestHarness::new(0).await;
        let test_data = TestData::new(3).await;
        let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

        // Set up view 1 as parent
        harness
            .send(test_data.views[0].proposal_event(&node_key))
            .await;
        harness
            .send(test_data.views[0].block_reconstructed_event())
            .await;

        // Send proposal for view 2 + BlockReconstructed but NO cert1
        harness
            .send(test_data.views[1].proposal_event(&node_key))
            .await;
        harness
            .send(test_data.views[1].block_reconstructed_event())
            .await;

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let events = harness.shutdown().await;

        // Vote1 may succeed, but Vote2 should NOT be sent (no cert1)
        assert!(
            !has_vote2(&events),
            "Vote2 should not be sent without Certificate1"
        );
    }

    /// Vote2 is sent when Certificate1 arrives after proposal.
    #[tokio::test]
    async fn test_vote2_with_cert1() {
        let harness = TestHarness::new(0).await;
        let test_data = TestData::new(3).await;
        let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

        // Set up view 1 as parent
        harness
            .send(test_data.views[0].proposal_event(&node_key))
            .await;
        harness
            .send(test_data.views[0].block_reconstructed_event())
            .await;

        // View 2: proposal + block reconstructed + cert1
        harness
            .send(test_data.views[1].proposal_event(&node_key))
            .await;
        harness
            .send(test_data.views[1].block_reconstructed_event())
            .await;
        harness.send(test_data.views[1].cert1_event()).await;

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let events = harness.shutdown().await;

        assert!(
            has_vote2(&events),
            "Vote2 should be sent when cert1 is present"
        );
    }

    /// Full single-view decision: proposal → vote1, cert1 → vote2, cert2 → decide.
    #[tokio::test]
    async fn test_single_view_decide() {
        let harness = TestHarness::new(0).await;
        let test_data = TestData::new(3).await;
        let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

        // Set up view 1 as parent
        harness
            .send(test_data.views[0].proposal_event(&node_key))
            .await;
        harness
            .send(test_data.views[0].block_reconstructed_event())
            .await;

        // View 2: full consensus round
        harness
            .send(test_data.views[1].proposal_event(&node_key))
            .await;
        harness
            .send(test_data.views[1].block_reconstructed_event())
            .await;
        harness.send(test_data.views[1].cert1_event()).await;
        harness.send(test_data.views[1].cert2_event()).await;

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let events = harness.shutdown().await;

        assert!(has_vote1(&events), "Vote1 should be sent");
        assert!(has_vote2(&events), "Vote2 should be sent");
        assert!(
            has_leaf_decided(&events),
            "Leaf should be decided after cert2"
        );
    }

    /// Duplicate votes are prevented — only one Vote1 per view.
    #[tokio::test]
    async fn test_no_duplicate_vote1() {
        let harness = TestHarness::new(0).await;
        let test_data = TestData::new(2).await;
        let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

        // View 1: trigger vote1 via proposal (genesis parent, mock responds with StateVerified)
        harness
            .send(test_data.views[0].proposal_event(&node_key))
            .await;

        // Send block_reconstructed + cert1 which re-trigger maybe_vote_1 for same view
        harness
            .send(test_data.views[0].block_reconstructed_event())
            .await;
        harness.send(test_data.views[0].cert1_event()).await;

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let events = harness.shutdown().await;

        assert_eq!(
            count_vote1(&events),
            1,
            "Should only send one Vote1 per view"
        );
    }

    /// Duplicate votes are prevented — only one Vote2 per view.
    #[tokio::test]
    async fn test_no_duplicate_vote2() {
        let harness = TestHarness::new(0).await;
        let test_data = TestData::new(3).await;
        let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

        // Set up parent
        harness
            .send(test_data.views[0].proposal_event(&node_key))
            .await;
        harness
            .send(test_data.views[0].block_reconstructed_event())
            .await;

        // View 2: trigger vote2
        harness
            .send(test_data.views[1].proposal_event(&node_key))
            .await;
        harness
            .send(test_data.views[1].block_reconstructed_event())
            .await;
        harness.send(test_data.views[1].cert1_event()).await;
        // Sending cert2 triggers maybe_vote_2 again (via handle_event post-calls)
        harness.send(test_data.views[1].cert2_event()).await;

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let events = harness.shutdown().await;

        assert_eq!(
            count_vote2(&events),
            1,
            "Should only send one Vote2 per view"
        );
    }

    /// StateVerificationFailed with matching commitment removes proposal and vid_share.
    #[tokio::test]
    async fn test_state_verification_failed_removes_proposal() {
        let harness = TestHarness::new(0).await;
        let test_data = TestData::new(3).await;
        let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

        // Set up parent
        harness
            .send(test_data.views[0].proposal_event(&node_key))
            .await;
        harness
            .send(test_data.views[0].block_reconstructed_event())
            .await;

        // Send proposal for view 2 (stores proposal and vid_share)
        harness
            .send(test_data.views[1].proposal_event(&node_key))
            .await;

        // Send StateVerificationFailed with matching commitment — removes proposal
        let proposal_commit = proposal_commitment(&test_data.views[1].proposal.data.proposal);
        let state = <TestValidatedState as ValidatedState<TestTypes>>::from_header(
            &test_data.views[1].proposal.data.proposal.block_header,
        );
        harness
            .send(ConsensusEvent::StateVerificationFailed(
                crate::events::StateResponse {
                    view: test_data.views[1].view_number,
                    commitment: proposal_commit,
                    state: Arc::new(state),
                },
            ))
            .await;

        // Now send cert1 + block_reconstructed — vote2 should NOT fire
        // because the proposal was removed
        harness
            .send(test_data.views[1].block_reconstructed_event())
            .await;
        harness.send(test_data.views[1].cert1_event()).await;

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let events = harness.shutdown().await;

        assert!(
            !has_vote2(&events),
            "Vote2 should not fire after proposal removed by StateVerificationFailed"
        );
    }

    /// Without Certificate2, no decision is made even with all other data.
    #[tokio::test]
    async fn test_decide_requires_cert2() {
        let harness = TestHarness::new(0).await;
        let test_data = TestData::new(3).await;
        let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

        // Set up view 1 as parent
        harness
            .send(test_data.views[0].proposal_event(&node_key))
            .await;
        harness
            .send(test_data.views[0].block_reconstructed_event())
            .await;

        // View 2: everything except cert2
        harness
            .send(test_data.views[1].proposal_event(&node_key))
            .await;
        harness
            .send(test_data.views[1].block_reconstructed_event())
            .await;
        harness.send(test_data.views[1].cert1_event()).await;
        // No cert2 sent

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let events = harness.shutdown().await;

        assert!(has_vote2(&events), "Vote2 should still fire");
        assert!(
            !has_leaf_decided(&events),
            "No decision without Certificate2"
        );
    }

    /// Vote2 requires BlockReconstructed for the current view.
    #[tokio::test]
    async fn test_vote2_missing_block_reconstructed() {
        let harness = TestHarness::new(0).await;
        let test_data = TestData::new(3).await;
        let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

        // Set up view 1 as parent
        harness
            .send(test_data.views[0].proposal_event(&node_key))
            .await;
        harness
            .send(test_data.views[0].block_reconstructed_event())
            .await;

        // View 2: proposal + cert1, but NO block_reconstructed for view 2
        harness
            .send(test_data.views[1].proposal_event(&node_key))
            .await;
        harness.send(test_data.views[1].cert1_event()).await;

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let events = harness.shutdown().await;

        assert!(
            !has_vote2(&events),
            "Vote2 should not fire without BlockReconstructed"
        );
    }

    /// BlockReconstructed arriving after cert1 triggers vote2.
    #[tokio::test]
    async fn test_vote2_block_reconstructed_arrives_late() {
        let harness = TestHarness::new(0).await;
        let test_data = TestData::new(3).await;
        let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

        // Set up parent
        harness
            .send(test_data.views[0].proposal_event(&node_key))
            .await;
        harness
            .send(test_data.views[0].block_reconstructed_event())
            .await;

        // View 2: proposal + cert1 first (no block_reconstructed yet)
        harness
            .send(test_data.views[1].proposal_event(&node_key))
            .await;
        harness.send(test_data.views[1].cert1_event()).await;

        // Now send block_reconstructed — should trigger vote2
        harness
            .send(test_data.views[1].block_reconstructed_event())
            .await;

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let events = harness.shutdown().await;

        assert!(
            has_vote2(&events),
            "Vote2 should fire when BlockReconstructed arrives late"
        );
    }

    /// Multi-view chain: consecutive views each get decided when cert2 arrives.
    #[tokio::test]
    async fn test_multi_view_chain_decide() {
        let harness = TestHarness::new(0).await;
        let test_data = TestData::new(5).await;
        let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

        // Process each view: proposal + block_reconstructed + cert1 + cert2
        for view in &test_data.views {
            harness.send(view.proposal_event(&node_key)).await;
            harness.send(view.block_reconstructed_event()).await;
            harness.send(view.cert1_event()).await;
            harness.send(view.cert2_event()).await;
        }

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let events = harness.shutdown().await;

        // Each view should produce a LeafDecided
        let decide_count = events
            .iter()
            .filter(|e| matches!(e, Event::Update(Update::LeafDecided(_))))
            .count();
        assert!(
            decide_count >= 2,
            "Multiple views should produce decisions, got {decide_count}"
        );
    }

    /// Timeout event sets timeout_view and prevents processing of that view.
    #[tokio::test]
    async fn test_timeout_prevents_voting() {
        let harness = TestHarness::new(0).await;
        let test_data = TestData::new(3).await;
        let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

        // Set up view 1 as parent
        harness
            .send(test_data.views[0].proposal_event(&node_key))
            .await;
        harness
            .send(test_data.views[0].block_reconstructed_event())
            .await;

        // Send proposal for view 2 (this gets stored)
        harness
            .send(test_data.views[1].proposal_event(&node_key))
            .await;
        harness
            .send(test_data.views[1].block_reconstructed_event())
            .await;

        // Timeout view 2 — now cert1 for view 2 should be dropped
        harness
            .send(ConsensusEvent::Timeout(test_data.views[1].view_number))
            .await;

        // Send cert1 for view 2 — should be stale
        harness.send(test_data.views[1].cert1_event()).await;

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let events = harness.shutdown().await;

        assert!(
            !has_vote2(&events),
            "Vote2 should not fire after timeout for that view"
        );
    }

    /// Helper: find the node index (0..10) for a given public key.
    fn node_index_for_key(key: &BLSPubKey) -> u64 {
        for i in 0..10 {
            let (pk, _) = BLSPubKey::generated_from_seed_indexed([0; 32], i);
            if pk == *key {
                return i;
            }
        }
        panic!("Key not found in test keys (indices 0..10)");
    }

    /// Check if received events contain a Proposal action.
    fn has_proposal(events: &[Event<TestTypes>]) -> bool {
        events.iter().any(|e| {
            matches!(
                e,
                Event::Action(Action::SendMessage(RequestMessageSender::Proposal(_, _)))
            )
        })
    }

    /// Check if received events contain a RequestBlockAndHeader action.
    fn has_request_block_and_header(events: &[Event<TestTypes>]) -> bool {
        events
            .iter()
            .any(|e| matches!(e, Event::Action(Action::RequestBlockAndHeader(_))))
    }

    /// Leader sends a proposal for view N+1 after receiving proposal for view N
    /// and cert1 for view N.
    #[tokio::test]
    async fn test_leader_sends_proposal() {
        let test_data = TestData::new(4).await;

        // Find who is leader for view 2 (test_data.views[1])
        let leader_for_view_2 = test_data.views[1].leader_public_key;
        let leader_index = node_index_for_key(&leader_for_view_2);
        let harness = TestHarness::new(leader_index).await;

        // Send proposal for view 1 — since we're leader for view 2,
        // this triggers request_block_and_header for view 2.
        // The mock coordinator responds with HeaderCreated(2, ...) and BlockBuilt(2, ...).
        harness
            .send(test_data.views[0].proposal_event(&leader_for_view_2))
            .await;

        // Send cert1 for view 1 — triggers maybe_propose(2)
        harness.send(test_data.views[0].cert1_event()).await;

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let events = harness.shutdown().await;

        // The leader should have requested a block and header for view 2
        assert!(
            has_request_block_and_header(&events),
            "Leader should request block and header for the next view"
        );

        // The leader should have sent a proposal for view 2
        assert!(
            has_proposal(&events),
            "Leader should send a proposal when it has cert1, header, block, and vid_disperse"
        );
    }

    /// Leader sends a proposal after a timeout using the locked QC and
    /// view change evidence.
    #[tokio::test]
    async fn test_leader_proposes_after_timeout() {
        let test_data = TestData::new(5).await;

        // We need a leader for view 3 (after timeout at view 2).
        // The timeout cert is for view 2, so the next view is 3.
        let leader_for_view_3 = test_data.views[2].leader_public_key;
        let leader_index = node_index_for_key(&leader_for_view_3);
        let harness = TestHarness::new(leader_index).await;

        // Build up locked_qc: process view 1 fully so cert1 for view 1 sets locked_qc
        harness
            .send(test_data.views[0].proposal_event(&leader_for_view_3))
            .await;
        harness
            .send(test_data.views[0].block_reconstructed_event())
            .await;
        harness.send(test_data.views[0].cert1_event()).await;

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Now send timeout cert for view 2 — this triggers request_block_and_header
        // for view 3 if we are leader
        harness.send(test_data.views[1].timeout_event()).await;

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let events = harness.shutdown().await;

        assert!(
            has_request_block_and_header(&events),
            "Leader should request block and header after timeout"
        );
        assert!(
            has_proposal(&events),
            "Leader should send proposal with timeout view change evidence"
        );
    }

    /// Non-leader node does NOT send a proposal.
    #[tokio::test]
    async fn test_non_leader_does_not_propose() {
        let test_data = TestData::new(4).await;

        // Find who is leader for view 2 and pick a DIFFERENT node
        let leader_for_view_2 = test_data.views[1].leader_public_key;
        let leader_index = node_index_for_key(&leader_for_view_2);
        let non_leader_index = if leader_index == 0 { 1 } else { 0 };
        let non_leader_key = BLSPubKey::generated_from_seed_indexed([0; 32], non_leader_index).0;
        let harness = TestHarness::new(non_leader_index).await;

        // Send proposal for view 1
        harness
            .send(test_data.views[0].proposal_event(&non_leader_key))
            .await;

        // Send cert1 for view 1
        harness.send(test_data.views[0].cert1_event()).await;

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let events = harness.shutdown().await;

        assert!(
            !has_proposal(&events),
            "Non-leader should NOT send a proposal"
        );
    }

    /// Cert2 for a view that is already decided is ignored.
    #[tokio::test]
    async fn test_decide_not_repeated_for_same_view() {
        let harness = TestHarness::new(0).await;
        let test_data = TestData::new(3).await;
        let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

        // Full round for view 2
        harness
            .send(test_data.views[0].proposal_event(&node_key))
            .await;
        harness
            .send(test_data.views[0].block_reconstructed_event())
            .await;
        harness
            .send(test_data.views[1].proposal_event(&node_key))
            .await;
        harness
            .send(test_data.views[1].block_reconstructed_event())
            .await;
        harness.send(test_data.views[1].cert1_event()).await;
        harness.send(test_data.views[1].cert2_event()).await;

        // Send cert2 again for same view — should not produce another decide
        harness.send(test_data.views[1].cert2_event()).await;

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let events = harness.shutdown().await;

        let decide_count = events
            .iter()
            .filter(|e| matches!(e, Event::Update(Update::LeafDecided(_))))
            .count();
        assert_eq!(decide_count, 1, "Should only decide once per view");
    }
}
