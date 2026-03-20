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
use hotshot_utils::anytrace;

use crate::{
    events::{Action, BlockAndHeaderRequest, ConsensusInput, ConsensusOutput, Event},
    helpers::{Outbox, proposal_commitment, upgrade_lock},
    message::{Certificate1, Certificate2, ProposalMessage, Vote1, Vote2Data},
};

pub struct Consensus<T: NodeType> {
    proposals: BTreeMap<ViewNumber, QuorumProposal2<T>>,
    vid_shares: BTreeMap<ViewNumber, VidDisperseShare2<T>>,
    states_verified: BTreeMap<ViewNumber, Commitment<Leaf2<T>>>,
    blocks_reconstructed: BTreeMap<ViewNumber, VidCommitment2>,
    blocks: BTreeMap<ViewNumber, T::BlockPayload>,
    vid_disperses: BTreeMap<ViewNumber, VidDisperse2<T>>,
    certs: BTreeMap<ViewNumber, Certificate1<T>>,
    certs2: BTreeMap<ViewNumber, Certificate2<T>>,
    timeout_certs: BTreeMap<ViewNumber, TimeoutCertificate2<T>>,
    view_sync_certs: BTreeMap<ViewNumber, ViewSyncFinalizeCertificate2<T>>,
    locked_qc: Option<Certificate1<T>>,
    headers: BTreeMap<ViewNumber, T::BlockHeader>,
    last_decided_view: ViewNumber,

    voted_1_views: BTreeSet<ViewNumber>,
    voted_2_views: BTreeSet<ViewNumber>,

    timeout_view: ViewNumber,

    // TODO: We need a next epoch stake table to handle the transition
    // And a way to set these stake tables, probably an event from coordinator
    stake_table_coordinator: EpochMembershipCoordinator<T>,

    public_key: T::SignatureKey,
    private_key: <T::SignatureKey as SignatureKey>::PrivateKey,
}

impl<T: NodeType> Consensus<T> {
    pub fn new(
        membership_coordinator: EpochMembershipCoordinator<T>,
        public_key: T::SignatureKey,
        private_key: <T::SignatureKey as SignatureKey>::PrivateKey,
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

    pub async fn apply(
        &mut self,
        input: ConsensusInput<T>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) {
        let view = input.view_number();
        if view <= self.timeout_view {
            return;
        }
        match input {
            ConsensusInput::Proposal(proposal) => self.handle_proposal(proposal, outbox).await,
            ConsensusInput::Certificate1(certificate) => {
                self.handle_certificate1(certificate).await
            },
            ConsensusInput::Certificate2(certificate) => {
                self.handle_certificate2(certificate).await
            },
            ConsensusInput::TimeoutCertificate(certificate) => {
                self.handle_timeout_certificate(certificate, outbox).await
            },
            ConsensusInput::ViewSyncCertificate(certificate) => {
                self.handle_view_sync_certificate(certificate, outbox).await
            },
            ConsensusInput::BlockReconstructed(view, vid_commitment) => {
                self.blocks_reconstructed.insert(view, vid_commitment);
            },
            ConsensusInput::StateVerified(state_response) => {
                self.states_verified
                    .insert(state_response.view, state_response.commitment);
            },
            ConsensusInput::HeaderCreated(view, header) => {
                self.headers.insert(view, header);
            },
            ConsensusInput::StateVerificationFailed(state_response) => {
                if let Some(proposal) = self.proposals.remove(&state_response.view)
                    && proposal_commitment(&proposal) != state_response.commitment
                {
                    // Proposal we stored didn't match the failed state verification, put it back
                    self.proposals.insert(state_response.view, proposal);
                    return;
                }
                self.vid_shares.remove(&state_response.view);
                return;
            },
            ConsensusInput::Timeout(view) => {
                self.handle_timeout(view);
                // we are done after timeout, don't try to vote, decide, or propose
                return;
            },
            ConsensusInput::BlockBuilt(view, epoch, block, metadata) => {
                self.handle_block_built(view, epoch, block, metdata, outbox);
            },
            ConsensusInput::VidDisperseCreated(view, vid_disperse) => {
                self.vid_disperses.insert(view, vid_disperse);
            },
        }
        self.maybe_vote_1(view, outbox);
        self.maybe_vote_2_and_update_lock(view, outbox);
        self.maybe_decide(view, outbox);
        self.maybe_propose(view, outbox).await;
        // An event from the current view or the previous view can trigger a propose
        self.maybe_propose(view + 1, outbox).await;
    }

    fn handle_block_built(
        &mut self,
        view: ViewNumber,
        epoch: EpochNumber,
        block: TYPES::BlockPayload,
        metadata: <TYPES::BlockPayload as hotshot::traits::BlockPayload<TYPES>>::Metadata,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) {
        outbox.push_back(Action::RequestVidDisperse {
            view,
            epoch,
            block: block.clone(),
            metadata,
        });
        self.blocks.insert(view, block);
    }

    async fn maybe_propose(&mut self, view: ViewNumber, outbox: &mut Outbox<ConsensusOutput<T>>) {
        let is_after_timeout =
            self.view_sync_certs.contains_key(&view) || self.timeout_certs.contains_key(&view);
        let qc = if is_after_timeout {
            self.locked_qc.as_ref()
        } else {
            self.certs.get(&ViewNumber::from(view.saturating_sub(1)))
        };
        let Some(qc) = qc else { return };
        let parent_view = qc.view_number();
        let Some(proposal) = self.proposals.get(&parent_view) else {
            return;
        };
        let Some(epoch) = proposal.epoch else { return };
        if !self.is_leader(view, epoch).await {
            return;
        }
        let Some(header) = self.headers.get(&parent_view) else {
            return;
        };
        let Some(block) = self.blocks.get(&view) else {
            return;
        };
        let Some(vid_disperse) = self.vid_disperses.get(&view) else {
            return;
        };

        // TODO: Handle epoch change and properly set next epoch qc drb result and state cert
        let mut proposal = QuorumProposal2::<T> {
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
            } else if let Some(timeout_cert) = self.timeout_certs.get(&view).cloned() {
                proposal.view_change_evidence = Some(ViewChangeEvidence2::Timeout(timeout_cert));
            } else {
                // TODO: logging?
            }
        }

        // TODO: Handle epoch change here

        outbox.push_back(Action::SendProposal(proposal, vid_disperse.clone()));
    }

    fn maybe_decide(&mut self, view: ViewNumber, outbox: &mut Outbox<ConsensusOutput<T>>) {
        if view <= self.last_decided_view {
            return;
        }
        let Some(cert2) = self.certs2.get(&view) else {
            return;
        };
        let Some(proposal) = self.proposals.get(&view) else {
            return;
        };
        let proposal_commit = proposal_commitment(proposal);
        if cert2.data.leaf_commit != proposal_commit {
            return;
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
        outbox.push_back(Event::LeafDecided(decided));
    }

    fn maybe_vote_1(&mut self, view: ViewNumber, outbox: &mut Outbox<ConsensusOutput<T>>) {
        if self.voted_1_views.contains(&view) {
            return;
        }
        let Some(state_commitment) = self.states_verified.get(&view) else {
            return;
        };
        let Some(proposal) = self.proposals.get(&view) else {
            return;
        };
        let Some(vid_share) = self.vid_shares.get(&view) else {
            return;
        };

        // Verify parent chain unless justify_qc is the genesis QC
        let parent_view = proposal.justify_qc.view_number();

        // We don't need the genesis block to be reconstructed or verified
        // or the genesis qc to be verified
        if parent_view != ViewNumber::genesis() {
            // Verify we have the block for the QC on this commitment
            let Some(block_commitment) = self.blocks_reconstructed.get(&parent_view) else {
                return;
            };
            let Some(prev_proposal) = self.proposals.get(&parent_view) else {
                return;
            };
            let VidCommitment::V2(prev_block_commitment) =
                prev_proposal.block_header.payload_commitment()
            else {
                return;
            };
            if block_commitment != &prev_block_commitment {
                return;
            }

            if proposal.justify_qc.data().leaf_commit != proposal_commitment(prev_proposal) {
                return;
            }
        }

        let proposal_commit = proposal_commitment(proposal);

        // Verify the state commitment matches the proposal
        if state_commitment != &proposal_commit {
            return;
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
            &upgrade_lock::<T>(),
        )
        .unwrap();
        let vote = Vote1 {
            vote: inner_vote,
            vid_share: vid_share.clone(),
        };
        outbox.push_back(Action::SendVote1(vote));
        self.voted_1_views.insert(view);
    }

    fn maybe_vote_2_and_update_lock(
        &mut self,
        view: ViewNumber,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) {
        if self.voted_2_views.contains(&view) {
            return;
        }

        // we have a proposal, reconstructed block, and first certificate for this view
        let Some(reconstructed_block_commitment) = self.blocks_reconstructed.get(&view) else {
            return;
        };
        let Some(cert1) = self.certs.get(&view) else {
            return;
        };
        let Some(proposal) = self.proposals.get(&view) else {
            return;
        };

        let proposal_commit = proposal_commitment(proposal);

        // The certificate must match the proposal
        if cert1.data.leaf_commit != proposal_commit {
            return;
        }
        // The proposal block commitment must match the reconstructed block commitment
        let VidCommitment::V2(proposal_block_commitment) =
            proposal.block_header.payload_commitment()
        else {
            return;
        };
        if &proposal_block_commitment != reconstructed_block_commitment {
            return;
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

        let Some(epoch) = proposal.epoch else { return };

        let vote = SimpleVote::create_signed_vote(
            Vote2Data {
                leaf_commit: proposal_commit,
                epoch,
                block: proposal.block_header.block_number().into(),
            },
            view,
            &self.public_key,
            &self.private_key,
            &upgrade_lock::<T>(),
        )
        .unwrap();
        outbox.push_back(Action::SendVote2(vote));
        self.voted_2_views.insert(view);
    }

    async fn handle_proposal(
        &mut self,
        proposal: ProposalMessage<T>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) {
        let view = proposal.view_number();

        if let Err(err) = proposal
            .proposal
            .validate_signature(&self.stake_table_coordinator)
            .await
        {
            return;
        }

        let vid_share = proposal.vid_share;
        let proposal = proposal.proposal.data;
        let Some(epoch) = proposal.epoch else { return };

        if !vid_matches_proposal(&vid_share, &proposal) {
            return;
        }

        if !vid_share.is_consistent() {
            return;
        }

        if self.verify_vid_share(&vid_share, epoch).await.is_err() {
            return;
        }

        if let Err(err) = self.validate_proposal(&proposal) {
            return;
        }

        self.proposals.insert(view, proposal.clone());
        let payload_size = vid_share.payload_byte_len();
        self.vid_shares.insert(view, vid_share);

        outbox.push_back(Action::RequestState(proposal.clone().into()));

        if self.is_leader(view + 1, epoch).await {
            outbox.push_back(Action::RequestBlockAndHeader(BlockAndHeaderRequest {
                view: view + 1,
                epoch,
                parent_proposal: proposal,
            }))
        }
    }

    async fn verify_vid_share(
        &self,
        vid_share: &VidDisperseShare2<T>,
        epoch: EpochNumber,
    ) -> Result<(), ()> {
        let Ok(stake_table) = self
            .stake_table_coordinator
            .membership_for_epoch(Some(epoch))
            .await
        else {
            return Err(());
        };
        let total_weight = vid_total_weight(&stake_table.stake_table().await, Some(epoch));
        if !vid_share.verify(total_weight) {
            return Err(());
        }
        Ok(())
    }

    fn validate_proposal(&mut self, proposal: &QuorumProposal2<T>) -> Result<(), ConsensusError> {
        let Some(locked_qc) = self.locked_qc.as_ref() else {
            // Locked QC is not set which means it is at genesis
            return Ok(())
        };
        let Ok(qc_commit) = proposal.justify_qc.data_commitment(&upgrade_lock::<T>()) else {
            return Err(ConsensusError::QcDataCommitment);
        };
        let Ok(locked_qc_commit) = locked_qc.data_commitment(&upgrade_lock::<T>()) else {
            return Err(ConsensusError::QcDataCommitment);
        };

        let safety_check = qc_commit == locked_qc_commit;
        let liveness_check = proposal.justify_qc.view_number() > locked_qc.view_number();

        if !safety_check && !liveness_check {
            return Err(ConsensusError::InvalidProposal);
        }

        Ok(())
    }

    async fn verify_cert<C, A>(&self, cert: &C, epoch: EpochNumber) -> Result<(), ConsensusError>
    where
        C: vote::Certificate<T, A>,
    {
        let stake_table = self
            .stake_table_coordinator
            .membership_for_epoch(Some(epoch))
            .await
            .map_err(ConsensusError::EpochMembership)?;
        let entries = StakeTableEntries::<T>::from(stake_table.stake_table().await).0;
        let threshold = stake_table.success_threshold().await;
        cert.is_valid_cert(&entries, threshold, &upgrade_lock::<T>())
            .context("invalid threshold signature")
            .map_err(ConsensusError::InvalidVoteCert)
    }

    async fn handle_certificate1(&mut self, cert: Certificate1<T>) {
        let Some(epoch) = cert.epoch() else { return };
        if let Err(err) = self.verify_cert(&cert, epoch).await {
            return;
        };
        self.certs.insert(cert.view_number(), cert);
    }

    async fn handle_certificate2(&mut self, cert: Certificate2<T>) {
        let Some(epoch) = cert.epoch() else { return };
        if let Err(err) = self.verify_cert(&cert, epoch).await {
            return;
        }
        self.certs2.insert(cert.view_number(), cert);
    }

    async fn handle_timeout_certificate(
        &mut self,
        cert: TimeoutCertificate2<T>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) {
        let Some(epoch) = cert.epoch() else { return };
        let view = cert.view_number() + 1;
        self.timeout_certs.insert(cert.view_number(), cert);
        if !self.is_leader(view, epoch).await {
            return;
        }
        let Some(locked_view) = self.locked_qc.as_ref().map(|qc| qc.view_number()) else {
            return;
        };

        let Some(proposal) = self.proposals.get(&locked_view) else {
            return;
        };
        outbox.push_back(Action::RequestBlockAndHeader(BlockAndHeaderRequest {
            view,
            epoch,
            parent_proposal: proposal.clone(),
        }));
    }

    async fn handle_view_sync_certificate(
        &mut self,
        cert: ViewSyncFinalizeCertificate2<T>,
        outbox: &mut Outbox<ConsensusOutput<T>>,
    ) {
        let Some(epoch) = cert.epoch() else { return };
        let view = cert.view_number();
        self.view_sync_certs.insert(view, cert);
        if !self.is_leader(view, epoch).await {
            return;
        }
        let Some(locked_view) = self.locked_qc.as_ref().map(|qc| qc.view_number()) else {
            return;
        };
        let Some(proposal) = self.proposals.get(&locked_view) else {
            return;
        };
        outbox.push_back(Action::RequestBlockAndHeader(BlockAndHeaderRequest {
            view: view + 1,
            epoch,
            parent_proposal: proposal.clone(),
        }));
    }

    fn handle_timeout(&mut self, view: ViewNumber) {
        self.timeout_view = view;
        // TODO: clear_view(view);
    }
}

fn vid_matches_proposal<T>(share: &VidDisperseShare2<T>, proposal: &QuorumProposal2<T>) -> bool
where
    T: NodeType,
{
    let VidCommitment::V2(vid_comm) = proposal.block_header.payload_commitment() else {
        return false;
    };
    vid_comm == share.payload_commitment
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ConsensusError {
    #[error("failed to get membership for epoch: {0}")]
    EpochMembership(#[source] anytrace::Error),
    #[error("invalid vote certificate: {0}")]
    InvalidVoteCert(#[source] anyhow::Error),
    #[error("failed to comput data commitment of qc")]
    QcDataCommitment,
    #[error("proposal does not pass safety and liveness check")]
    InvalidProposal,
}

#[cfg(test)]
mod test {
    use hotshot::types::BLSPubKey;
    use hotshot_example_types::node_types::TestTypes;
    use hotshot_types::traits::signature_key::SignatureKey;

    use super::*;
    use crate::{
        events::{Action, ConsensusInput, ConsensusOutput, Event},
        test_utils::{TestData, mock_membership},
    };

    async fn make_consensus(node_index: u64) -> Consensus<TestTypes> {
        let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], node_index);
        let membership = mock_membership().await;
        Consensus::new(membership, public_key, private_key)
    }

    /// Check if received outputs contain a Vote1 action.
    fn has_vote1(outputs: &Outbox<ConsensusOutput<TestTypes>>) -> bool {
        outputs
            .iter()
            .any(|e| matches!(e, ConsensusOutput::Action(Action::SendVote1(_))))
    }

    /// Check if received outputs contain a Vote2 action.
    fn has_vote2(outputs: &Outbox<ConsensusOutput<TestTypes>>) -> bool {
        outputs
            .iter()
            .any(|e| matches!(e, ConsensusOutput::Action(Action::SendVote2(_))))
    }

    /// Check if received outputs contain a LeafDecided update.
    fn has_leaf_decided(outputs: &Outbox<ConsensusOutput<TestTypes>>) -> bool {
        outputs
            .iter()
            .any(|e| matches!(e, ConsensusOutput::Event(Event::LeafDecided(_))))
    }

    /// Check if received outputs contain a RequestState action.
    fn has_request_state(outputs: &Outbox<ConsensusOutput<TestTypes>>) -> bool {
        outputs
            .iter()
            .any(|e| matches!(e, ConsensusOutput::Action(Action::RequestState(_))))
    }

    /// Count how many Vote1 actions are in the outputs.
    fn count_vote1(outputs: &Outbox<ConsensusOutput<TestTypes>>) -> usize {
        outputs
            .iter()
            .filter(|e| matches!(e, ConsensusOutput::Action(Action::SendVote1(_))))
            .count()
    }

    /// Count how many Vote2 actions are in the outputs.
    fn count_vote2(outputs: &Outbox<ConsensusOutput<TestTypes>>) -> usize {
        outputs
            .iter()
            .filter(|e| matches!(e, ConsensusOutput::Action(Action::SendVote2(_))))
            .count()
    }

    #[tokio::test]
    async fn test_data_generation() {
        let _test_data = TestData::new(5).await;
    }

    /// Fresh consensus with no locked_qc accepts any proposal (genesis safety).
    #[tokio::test]
    async fn test_safety_genesis_no_lock() {
        let mut consensus = make_consensus(0).await;
        let mut outbox = Outbox::new();
        let test_data = TestData::new(2).await;
        let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

        // Send proposal for view 1 — locked_qc is None, so safety passes
        consensus
            .apply(test_data.views[0].proposal_event(&node_key), &mut outbox)
            .await;

        // Should have requested state verification (proposal accepted)
        assert!(
            has_request_state(&outbox),
            "Proposal should be accepted with no locked QC"
        );
    }

    /// Events with view <= timeout_view are silently dropped.
    #[tokio::test]
    async fn test_timeout_filters_stale_events() {
        let mut consensus = make_consensus(0).await;
        let mut outbox = Outbox::new();
        let test_data = TestData::new(6).await;
        let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

        // Set timeout at view 3
<<<<<<< HEAD
        consensus
            .apply(ConsensusInput::Timeout(ViewNumber::new(3)), &mut outbox)
            .await;

        // Send stale proposal (view 2, which is <= timeout_view 3)
        consensus
            .apply(test_data.views[1].proposal_event(&node_key), &mut outbox)
            .await;

        // Send fresh proposal (view 4, which is > timeout_view 3)
        consensus
            .apply(test_data.views[3].proposal_event(&node_key), &mut outbox)
            .await;

        // Only the fresh proposal (view 4) should generate a RequestState
        let request_states: Vec<_> = outbox
            .iter()
            .filter(|e| matches!(e, ConsensusOutput::Action(Action::RequestState(_))))
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
        let mut consensus = make_consensus(0).await;
        let mut outbox = Outbox::new();
        let test_data = TestData::new(3).await;
        let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

        // Send proposal for view 1 (parent data setup)
        consensus
            .apply(test_data.views[0].proposal_event(&node_key), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[0].state_verified_event(), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[0].block_reconstructed_event(), &mut outbox)
            .await;

        // Send proposal for view 2 + simulate StateVerified
        consensus
            .apply(test_data.views[1].proposal_event(&node_key), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[1].state_verified_event(), &mut outbox)
            .await;

        assert!(
            count_vote1(&outbox) == 2,
            "Vote1 should fire for sequential views"
        );
    }

    /// Vote1 fires for view 1 (genesis parent) — parent checks are skipped.
    #[tokio::test]
    async fn test_vote1_genesis_parent() {
        let mut consensus = make_consensus(0).await;
        let mut outbox = Outbox::new();
        let test_data = TestData::new(2).await;
        let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

        // Send proposal for view 1 — justify_qc references genesis
        consensus
            .apply(test_data.views[0].proposal_event(&node_key), &mut outbox)
            .await;
        // Simulate coordinator responding with StateVerified
        consensus
            .apply(test_data.views[0].state_verified_event(), &mut outbox)
            .await;

        assert!(
            has_vote1(&outbox),
            "Vote1 should fire for view 1 with genesis parent"
        );
    }

    /// Vote2 requires Certificate1 + BlockReconstructed + Proposal.
    /// Without Certificate1, no Vote2 is sent.
    #[tokio::test]
    async fn test_vote2_missing_cert1() {
        let mut consensus = make_consensus(0).await;
        let mut outbox = Outbox::new();
        let test_data = TestData::new(3).await;
        let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

        // Set up view 1 as parent
        consensus
            .apply(test_data.views[0].proposal_event(&node_key), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[0].block_reconstructed_event(), &mut outbox)
            .await;

        // Send proposal for view 2 + BlockReconstructed but NO cert1
        consensus
            .apply(test_data.views[1].proposal_event(&node_key), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[1].block_reconstructed_event(), &mut outbox)
            .await;

        assert!(
            !has_vote2(&outbox),
            "Vote2 should not be sent without Certificate1"
        );
    }

    /// Vote2 is sent when Certificate1 arrives after proposal.
    #[tokio::test]
    async fn test_vote2_with_cert1() {
        let mut consensus = make_consensus(0).await;
        let mut outbox = Outbox::new();
        let test_data = TestData::new(3).await;
        let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

        // Set up view 1 as parent
        consensus
            .apply(test_data.views[0].proposal_event(&node_key), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[0].block_reconstructed_event(), &mut outbox)
            .await;

        // View 2: proposal + block reconstructed + cert1
        consensus
            .apply(test_data.views[1].proposal_event(&node_key), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[1].block_reconstructed_event(), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[1].cert1_event(), &mut outbox)
            .await;

        assert!(
            has_vote2(&outbox),
            "Vote2 should be sent when cert1 is present"
        );
    }

    /// Full single-view decision: proposal → vote1, cert1 → vote2, cert2 → decide.
    #[tokio::test]
    async fn test_single_view_decide() {
        let mut consensus = make_consensus(0).await;
        let mut outbox = Outbox::new();
        let test_data = TestData::new(3).await;
        let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

        // Set up view 1 as parent
        consensus
            .apply(test_data.views[0].proposal_event(&node_key), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[0].state_verified_event(), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[0].block_reconstructed_event(), &mut outbox)
            .await;

        // View 2: full consensus round
        consensus
            .apply(test_data.views[1].proposal_event(&node_key), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[1].state_verified_event(), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[1].block_reconstructed_event(), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[1].cert1_event(), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[1].cert2_event(), &mut outbox)
            .await;

        assert!(has_vote1(&outbox), "Vote1 should be sent");
        assert!(has_vote2(&outbox), "Vote2 should be sent");
        assert!(
            has_leaf_decided(&outbox),
            "Leaf should be decided after cert2"
        );
    }

    /// Duplicate votes are prevented — only one Vote1 per view.
    #[tokio::test]
    async fn test_no_duplicate_vote1() {
        let mut consensus = make_consensus(0).await;
        let mut outbox = Outbox::new();
        let test_data = TestData::new(2).await;
        let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

        // View 1: trigger vote1 via proposal + StateVerified
        consensus
            .apply(test_data.views[0].proposal_event(&node_key), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[0].state_verified_event(), &mut outbox)
            .await;

        // Send block_reconstructed + cert1 which re-trigger maybe_vote_1 for same view
        consensus
            .apply(test_data.views[0].block_reconstructed_event(), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[0].cert1_event(), &mut outbox)
            .await;

        assert_eq!(
            count_vote1(&outbox),
            1,
            "Should only send one Vote1 per view"
        );
    }

    /// Duplicate votes are prevented — only one Vote2 per view.
    #[tokio::test]
    async fn test_no_duplicate_vote2() {
        let mut consensus = make_consensus(0).await;
        let mut outbox = Outbox::new();
        let test_data = TestData::new(3).await;
        let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

        // Set up parent
        consensus
            .apply(test_data.views[0].proposal_event(&node_key), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[0].block_reconstructed_event(), &mut outbox)
            .await;

        // View 2: trigger vote2
        consensus
            .apply(test_data.views[1].proposal_event(&node_key), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[1].block_reconstructed_event(), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[1].cert1_event(), &mut outbox)
            .await;
        // Sending cert2 triggers maybe_vote_2 again (via handle_event post-calls)
        consensus
            .apply(test_data.views[1].cert2_event(), &mut outbox)
            .await;

        assert_eq!(
            count_vote2(&outbox),
            1,
            "Should only send one Vote2 per view"
        );
    }

    /// StateVerificationFailed with matching commitment removes proposal and vid_share.
    #[tokio::test]
    async fn test_state_verification_failed_removes_proposal() {
        let mut consensus = make_consensus(0).await;
        let mut outbox = Outbox::new();
        let test_data = TestData::new(3).await;
        let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

        // Set up parent
        consensus
            .apply(test_data.views[0].proposal_event(&node_key), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[0].block_reconstructed_event(), &mut outbox)
            .await;

        // Send proposal for view 2 (stores proposal and vid_share)
        consensus
            .apply(test_data.views[1].proposal_event(&node_key), &mut outbox)
            .await;

        // StateVerificationFailed input with matching state request => removes proposal
        consensus.apply(ConsensusInput::StateverificationFailed(
                Event::Update(
                    Update::StateVerificationFailed(
                        StateRequest {
                            view: test_data.views[1].view_number,
                            parent_view: test_data.views[1]
                                .proposal
                                .data
                                .proposal
                                .justify_qc
                                .view_number(),
                            epoch: test_data.views[1].epoch_number,
                            block_number: hotshot_types::traits::block_contents::BlockHeader::<
                                TestTypes,
                            >::block_number(
                                &test_data.views[1].proposal.data.proposal.block_header,
                            ),
                            proposal: test_data.views[1].proposal.data.proposal.clone(),
                            parent_commitment: test_data.views[1]
                                .proposal
                                .data
                                .proposal
                                .justify_qc
                                .data()
                                .leaf_commit,
                            payload_size: 0,
                        },
                    ),
                ),
            ),
            &mut outbox
        )
        .await;

        // Now send cert1 + block_reconstructed — vote2 should NOT fire
        // because the proposal was removed
        consensus
            .apply(test_data.views[1].block_reconstructed_event(), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[1].cert1_event(), &mut outbox)
            .await;

        assert!(
            !has_vote2(&outbox),
            "Vote2 should not fire after proposal removed by StateVerificationFailed"
        );
    }

    /// Without Certificate2, no decision is made even with all other data.
    #[tokio::test]
    async fn test_decide_requires_cert2() {
        let mut consensus = make_consensus(0).await;
        let mut outbox = Outbox::new();
        let test_data = TestData::new(3).await;
        let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

        // Set up view 1 as parent
        consensus
            .apply(test_data.views[0].proposal_event(&node_key), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[0].block_reconstructed_event(), &mut outbox)
            .await;

        // View 2: everything except cert2
        consensus
            .apply(test_data.views[1].proposal_event(&node_key), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[1].block_reconstructed_event(), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[1].cert1_event(), &mut outbox)
            .await;

        // No cert2 sent

        assert!(has_vote2(&outbox), "Vote2 should still fire");
        assert!(
            !has_leaf_decided(&outbox),
            "No decision without Certificate2"
        );
    }

    /// Vote2 requires BlockReconstructed for the current view.
    #[tokio::test]
    async fn test_vote2_missing_block_reconstructed() {
        let mut consensus = make_consensus(0).await;
        let mut outbox = Outbox::new();
        let test_data = TestData::new(3).await;
        let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

        // Set up view 1 as parent
        consensus
            .apply(test_data.views[0].proposal_event(&node_key), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[0].block_reconstructed_event(), &mut outbox)
            .await;

        // View 2: proposal + cert1, but NO block_reconstructed for view 2
        consensus
            .apply(test_data.views[1].proposal_event(&node_key), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[1].cert1_event(), &mut outbox)
            .await;

        assert!(
            !has_vote2(&outbox),
            "Vote2 should not fire without BlockReconstructed"
        );
    }

    /// BlockReconstructed arriving after cert1 triggers vote2.
    #[tokio::test]
    async fn test_vote2_block_reconstructed_arrives_late() {
        let mut consensus = make_consensus(0).await;
        let mut outbox = Outbox::new();
        let test_data = TestData::new(3).await;
        let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

        // Set up parent
        consensus
            .apply(test_data.views[0].proposal_event(&node_key), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[0].block_reconstructed_event(), &mut outbox)
            .await;

        // View 2: proposal + cert1 first (no block_reconstructed yet)
        consensus
            .apply(test_data.views[1].proposal_event(&node_key), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[1].cert1_event(), &mut outbox)
            .await;

        // Now send block_reconstructed — should trigger vote2
        consensus
            .apply(test_data.views[1].block_reconstructed_event(), &mut outbox)
            .await;

        assert!(
            has_vote2(&outbox),
            "Vote2 should fire when BlockReconstructed arrives late"
        );
    }

    /// Multi-view chain: consecutive views each get decided when cert2 arrives.
    #[tokio::test]
    async fn test_multi_view_chain_decide() {
        let mut consensus = make_consensus(0).await;
        let mut outbox = Outbox::new();
        let test_data = TestData::new(5).await;
        let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

        // Process each view: proposal + block_reconstructed + cert1 + cert2
        for view in &test_data.views {
            consensus
                .apply(view.proposal_event(&node_key), &mut outbox)
                .await;
            consensus
                .apply(view.block_reconstructed_event(), &mut outbox)
                .await;
            consensus.apply(view.cert1_event(), &mut outbox).await;
            consensus.apply(view.cert2_event(), &mut outbox).await;
        }

        // Each view should produce a LeafDecided
        let decide_count = outbox
            .iter()
            .filter(|e| matches!(e, ConsensusOutput::Event(Event::LeafDecided(_))))
            .count();
        assert!(
            decide_count >= 2,
            "Multiple views should produce decisions, got {decide_count}"
        );
    }

    /// Timeout event sets timeout_view and prevents processing of that view.
    #[tokio::test]
    async fn test_timeout_prevents_voting() {
        let mut consensus = make_consensus(0).await;
        let mut outbox = Outbox::new();
        let test_data = TestData::new(3).await;
        let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

        // Set up view 1 as parent
        consensus
            .apply(test_data.views[0].proposal_event(&node_key), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[0].block_reconstructed_event(), &mut outbox)
            .await;

        // Send proposal for view 2 (this gets stored)
        consensus
            .apply(test_data.views[1].proposal_event(&node_key), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[1].block_reconstructed_event(), &mut outbox)
            .await;

        // Timeout view 2 — now cert1 for view 2 should be dropped
        consensus
            .apply(
                ConsensusInput::Timeout(test_data.views[1].view_number),
                &mut outbox,
            )
            .await;

        // Send cert1 for view 2 — should be stale
        consensus
            .apply(test_data.views[1].cert1_event(), &mut outbox)
            .await;

        assert!(
            !has_vote2(&outbox),
            "Vote2 should not fire after timeout for that view"
        );
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
            .send(test_data.views[0].proposal_update(&leader_for_view_2))
            .await;

        // Send cert1 for view 1 — triggers maybe_propose(2)
        harness.send(test_data.views[0].cert1_update()).await;

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
            .send(test_data.views[0].proposal_update(&leader_for_view_3))
            .await;
        harness
            .send(test_data.views[0].block_reconstructed_update())
            .await;
        harness.send(test_data.views[0].cert1_update()).await;

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Now send timeout cert for view 2 — this triggers request_block_and_header
        // for view 3 if we are leader
        harness.send(test_data.views[1].timeout_cert_update()).await;

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
            .send(test_data.views[0].proposal_update(&non_leader_key))
            .await;

        // Send cert1 for view 1
        harness.send(test_data.views[0].cert1_update()).await;

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
        let mut consensus = make_consensus(0).await;
        let mut outbox = Outbox::new();
        let test_data = TestData::new(3).await;
        let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;

        // Full round for view 2
        consensus
            .apply(test_data.views[0].proposal_event(&node_key), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[0].block_reconstructed_event(), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[1].proposal_event(&node_key), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[1].block_reconstructed_event(), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[1].cert1_event(), &mut outbox)
            .await;
        consensus
            .apply(test_data.views[1].cert2_event(), &mut outbox)
            .await;

        // Send cert2 again for same view — should not produce another decide
        consensus
            .apply(test_data.views[1].cert2_event(), &mut outbox)
            .await;

        let decide_count = outbox
            .iter()
            .filter(|e| matches!(e, ConsensusOutput::Event(Event::LeafDecided(_))))
            .count();
        assert_eq!(decide_count, 1, "Should only decide once per view");
    }
}
