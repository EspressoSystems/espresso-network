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
                return None;
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
            ConsensusEvent::BlockBuilt(view, block) => {
                self.handle_block_built(view, block).await?;
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
        block: TYPES::BlockPayload,
    ) -> Option<()> {
        let proposal = self.proposals.get(&view)?;
        let epoch = proposal.epoch?;

        // TODO: This implicitly relies on the ordering of events,
        // We are assuming the header is received before the block is built
        let header = self.headers.get(&view)?;
        let metadata = header.metadata();
        self.coordinator_handle
            .request_vid_disperse(view, epoch, block.clone(), metadata.clone())
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

        let header = self.headers.get(&parent_view)?;
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
            let is_after_certs = self
                .certs
                .get(&view)
                .is_some_and(|cert| cert.view_number() < view);
            if is_after_certs {
                return None;
            }
            let is_after_certs2 = self
                .certs2
                .get(&view)
                .is_some_and(|cert| cert.view_number() < view);
            if is_after_certs2 {
                return None;
            }
            return None;
        }
        let state_commitment = self.states_verified.get(&view)?;
        let proposal = self.proposals.get(&view)?;
        let vid_share = self.vid_shares.get(&view)?;

        // Verify we have the block for the QC on this commitment
        let block_commitment = self
            .blocks_reconstructed
            .get(&proposal.justify_qc.view_number())?;
        let prev_proposal = self.proposals.get(&proposal.justify_qc.view_number())?;
        let VidCommitment::V2(prev_block_commitment) =
            prev_proposal.block_header.payload_commitment()
        else {
            return None;
        };
        if block_commitment != &prev_block_commitment {
            return None;
        }

        if proposal.justify_qc.data().leaf_commit != prev_proposal.justify_qc.data().leaf_commit {
            return None;
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
        .await
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
        .await
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
        self.validate_safety(&proposal).await?;
        self.proposals.insert(view, proposal.clone());
        self.vid_shares.insert(view, vid_share);

        // Now ask for the state to verify the header of the proposal
        self.coordinator_handle
            .request_state(proposal.clone())
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

    async fn validate_safety(&mut self, proposal: &QuorumProposal2<TYPES>) -> Option<()> {
        let Some(locked_qc) = self.locked_qc.as_ref() else {
            // Locked QC is not set which means it is at genesis
            return Some(());
        };
        let liveness_check = proposal.justify_qc.view_number() > locked_qc.view_number();
        let safety_check = proposal
            .justify_qc
            .data_commitment(&upgrade_lock::<TYPES>())
            .await
            .ok()?
            == locked_qc
                .data_commitment(&upgrade_lock::<TYPES>())
                .await
                .ok()?;
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
            .await
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
        self.timeout_certs
            .insert(certificate.view_number(), certificate);
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
    use hotshot::types::BLSPubKey;
    use hotshot_example_types::node_types::TestTypes;
    use hotshot_types::traits::signature_key::SignatureKey;
    use tokio::task::JoinHandle;

    use super::*;
    use crate::{
        coordinator::mock::testing::MockCoordinator,
        events::Event,
        test_utils::{TestData, mock_membership},
    };

    async fn mock_consensus_and_coordinator() -> (MockCoordinator, Consensus<TestTypes>) {
        let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], 0);
        let membership = mock_membership().await;
        let (event_tx, event_rx) = tokio::sync::mpsc::channel(10);
        let (consensus_tx, consensus_rx) = tokio::sync::mpsc::channel(10);

        let mock_coordinator = MockCoordinator {
            event_rx,
            consensus_tx,
            membership_coordinator: membership.clone(),
            received_events: Vec::new(),
        };
        let coordinator_handle = CoordinatorHandle::new(event_tx);
        let consensus = Consensus::new(
            consensus_rx,
            coordinator_handle,
            membership,
            public_key,
            private_key,
        );
        (mock_coordinator, consensus)
    }

    async fn run_consensus() -> (
        CoordinatorHandle<TestTypes>,
        JoinHandle<Vec<Event<TestTypes>>>,
    ) {
        let (mock_coordinator, mut consensus) = mock_consensus_and_coordinator().await;
        let handle = consensus.coordinator_handle.clone();
        tokio::spawn(async move {
            consensus.run().await;
        });
        let join_handle = tokio::spawn(async move { mock_coordinator.run().await });
        (handle, join_handle)
    }

    async fn run_test(input_events: Vec<Event<TestTypes>>, output_events: Vec<Event<TestTypes>>) {
        let (coordinator_handle, join_handle) = run_consensus().await;
        for event in input_events {
            coordinator_handle.send_event(event).await.unwrap();
        }
        let events = join_handle.await.unwrap();
        assert_eq!(events, output_events);
    }

    #[tokio::test]
    async fn test_consensus() {
        let _test_data = TestData::new(5).await;
    }
}
