use std::{
    cmp::max,
    collections::{BTreeMap, BTreeSet},
};

use committable::{Commitment, Committable};
use hotshot_types::{
    data::{
        EpochNumber, Leaf2, QuorumProposal2, QuorumProposalWrapper, VidCommitment, VidCommitment2,
        VidDisperseShare2, ViewNumber, vid_disperse::vid_total_weight,
    },
    epoch_membership::EpochMembershipCoordinator,
    simple_certificate::{TimeoutCertificate2, ViewSyncCommitCertificate2},
    simple_vote::{HasEpoch, QuorumData2, SimpleVote},
    traits::{
        block_contents::BlockHeader, election::Membership, node_implementation::NodeType,
        signature_key::SignatureKey,
    },
    vote::{Certificate, HasViewNumber},
};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::{
    coordinator::handle::CoordinatorHandle,
    events::{ConsensusEvent, StateRequest, StateResponse},
    helpers::{proposal_commitment, upgrade_lock},
    message::{Certificate1, Certificate2, ConsensusMessage, ProposalMessage, Vote1, Vote2Data},
};

pub(crate) struct Consensus<TYPES: NodeType> {
    proposals: BTreeMap<ViewNumber, QuorumProposal2<TYPES>>,
    vid_shares: BTreeMap<ViewNumber, VidDisperseShare2<TYPES>>,
    states_verified: BTreeMap<ViewNumber, Commitment<Leaf2<TYPES>>>,
    blocks_reconstructed: BTreeMap<ViewNumber, VidCommitment2>,
    certs: BTreeMap<ViewNumber, Certificate1<TYPES>>,
    certs2: BTreeMap<ViewNumber, Certificate2<TYPES>>,
    timeout_certs: BTreeMap<ViewNumber, TimeoutCertificate2<TYPES>>,
    view_sync_certs: BTreeMap<ViewNumber, ViewSyncCommitCertificate2<TYPES>>,
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
            ConsensusEvent::Certificate1(certificate) => self.handle_certificate1(certificate),
            ConsensusEvent::Certificate2(certificate) => self.handle_certificate2(certificate),
            ConsensusEvent::TimeoutCertificate(certificate) => {
                self.handle_timeout_certificate(certificate).await?;
            },
            ConsensusEvent::ViewSyncCertificate(certificate) => {
                self.handle_view_sync_certificate(certificate).await?;
            },
            ConsensusEvent::BlockReconstructed(view, vid_commitment) => {
                self.handle_block_reconstructed(view, vid_commitment)
            },
            ConsensusEvent::StateVerified(state_response) => {
                self.handle_state_verified(state_response)
            },
            ConsensusEvent::HeaderCreated(view, header) => self.handle_header_created(view, header),
            ConsensusEvent::StateVerificationFailed(state_request) => {
                self.handle_state_verification_failed(state_request)
            },
            ConsensusEvent::HeaderCreationFailed(header_request) => {
                self.handle_header_creation_failed(header_request.view)
            },
            ConsensusEvent::Timeout(view) => {
                self.handle_timeout(view);
                return None;
            },
        }
        self.maybe_vote_1(view).await;
        self.maybe_vote_2_and_update_lock(view).await;
        self.maybe_decide(view).await;
        self.maybe_propose_next_view(view).await;
        Some(())
    }

    pub async fn run(&mut self) {
        while let Some(event) = self.event_rx.recv().await {
            self.handle_event(event).await;
        }
    }

    async fn maybe_propose_after_timeout(&mut self, view: ViewNumber) -> Option<()> {
        Some(())
    }

    async fn maybe_propose_next_view(&mut self, view: ViewNumber) -> Option<()> {
        let is_after_timeout =
            self.view_sync_certs.get(&view).is_some() || self.timeout_certs.get(&view).is_some();
        if is_after_timeout {
            return self.maybe_propose_after_timeout(view).await;
        }
        let proposal = self.proposals.get(&view)?;
        let next_view = view + 1;
        if !self.is_leader(next_view, proposal.epoch?).await {
            return None;
        }

        let header = self.headers.get(&view)?;
        Some(())
    }

    async fn maybe_decide(&mut self, view: ViewNumber) -> Option<()> {
        if view <= self.last_decided_view {
            return None;
        }
        let cert2 = self.certs2.get(&view)?;
        let proposal = self.proposals.get(&view)?;
        let proposal_commit = proposal_commitment(&proposal);
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
            let proposal_commit = proposal_commitment(&proposal);
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

        let proposal_commit = proposal_commitment(&proposal);

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
            .send_message(ConsensusMessage::Vote1(vote))
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

        let proposal_commit = proposal_commitment(&proposal);

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
            .send_message(ConsensusMessage::Vote2(vote))
            .await
            .ok()?;
        self.voted_2_views.insert(view);
        Some(())
    }

    async fn handle_proposal(&mut self, proposal: ProposalMessage<TYPES>) -> Option<()> {
        let view = proposal.view_number();
        let epoch = proposal.proposal.data.epoch?;
        //Verify the VID share matches the proposal
        let VidCommitment::V2(vid_comm) = proposal.proposal.data.block_header.payload_commitment()
        else {
            return None;
        };
        if vid_comm != proposal.vid_share.payload_commitment {
            return None;
        }

        //Verify the VID share
        if !proposal.vid_share.is_consistent() {
            return None;
        }
        let stake_table = self
            .stake_table_coordinator
            .membership_for_epoch(Some(epoch))
            .await
            .ok()?;
        let total_weight = vid_total_weight(&stake_table.stake_table().await, Some(epoch));
        if !proposal.vid_share.verify(total_weight) {
            return None;
        }
        // Verify the proposal is valid
        self.validate_safety(&proposal.proposal.data).await?;
        self.proposals.insert(view, proposal.proposal.data.clone());
        self.vid_shares.insert(view, proposal.vid_share);

        // Now ask for the state
        self.coordinator_handle
            .request_state(proposal.proposal.data.clone())
            .await
            .ok()?;
        // And if we are leader next, ask for a header
        if self.is_leader(view + 1, epoch).await {
            self.coordinator_handle
                .request_header(proposal.proposal.data.clone(), view + 1, epoch)
                .await
                .ok()?;
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

    fn handle_certificate1(&mut self, certificate: Certificate1<TYPES>) {
        self.certs.insert(certificate.view_number(), certificate);
    }

    fn handle_certificate2(&mut self, certificate: Certificate2<TYPES>) {
        self.certs2.insert(certificate.view_number(), certificate);
    }

    async fn handle_timeout_certificate(
        &mut self,
        certificate: TimeoutCertificate2<TYPES>,
    ) -> Option<()> {
        let view = certificate.view_number();
        let epoch = certificate.epoch()?;
        self.timeout_certs
            .insert(certificate.view_number(), certificate);
        if self.is_leader(view + 1, epoch).await {
            let locked_view = self.locked_qc.as_ref().map(|qc| qc.view_number())?;
            let proposal = self.proposals.get(&locked_view)?;
            self.coordinator_handle
                .request_header(proposal.clone(), view, epoch)
                .await
                .ok()?;
            Some(())
        } else {
            None
        }
    }

    async fn handle_view_sync_certificate(
        &mut self,
        certificate: ViewSyncCommitCertificate2<TYPES>,
    ) -> Option<()> {
        let view = certificate.view_number();
        let epoch = certificate.epoch()?;
        self.view_sync_certs
            .insert(certificate.view_number(), certificate);
        if self.is_leader(view, epoch).await {
            let locked_view = self.locked_qc.as_ref().map(|qc| qc.view_number())?;
            let proposal = self.proposals.get(&locked_view)?;
            self.coordinator_handle
                .request_header(proposal.clone(), view, epoch)
                .await
                .ok()?;
            Some(())
        } else {
            None
        }
    }

    fn handle_block_reconstructed(&mut self, view: ViewNumber, vid_commitment: VidCommitment2) {
        self.blocks_reconstructed.insert(view, vid_commitment);
    }

    fn handle_state_verified(&mut self, state_request: StateResponse<TYPES>) {
        self.states_verified
            .insert(state_request.view, state_request.commitment);
    }

    fn handle_header_created(&mut self, view: ViewNumber, header: TYPES::BlockHeader) {
        self.headers.insert(view, header);
    }

    fn handle_state_verification_failed(&mut self, state_request: StateRequest<TYPES>) {
        self.states_verified.remove(&state_request.view);
    }

    fn handle_header_creation_failed(&mut self, view: ViewNumber) {
        self.headers.remove(&view);
    }

    fn handle_timeout(&mut self, view: ViewNumber) {
        self.timeout_view = view;
        // TODO: clear_view(view);
    }
}
