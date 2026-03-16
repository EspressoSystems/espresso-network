use std::collections::{BTreeMap, BTreeSet};

use committable::{Commitment, Committable};
use hotshot_types::{
    data::{
        Leaf2, QuorumProposal2, QuorumProposalWrapper, VidCommitment, VidCommitment2,
        VidDisperseShare2, ViewNumber, vid_disperse::vid_total_weight,
    },
    message::UpgradeLock,
    simple_certificate::{TimeoutCertificate2, ViewSyncCommitCertificate2},
    simple_vote::SimpleVote,
    traits::{
        block_contents::BlockHeader, election::Membership, node_implementation::NodeType,
        signature_key::SignatureKey,
    },
    vote::{Certificate, HasViewNumber},
};
use tokio::sync::mpsc::{Receiver, Sender};
use versions::{DRB_AND_HEADER_UPGRADE_VERSION, Upgrade};

use crate::{
    events::{ConsensusEvent, Event, StateRequest, StateResponse},
    message::{
        Certificate1, Certificate2, ConsensusMessage, Proposal, ProposalMessage, Vote1, Vote1Data,
    },
};

fn upgrade_lock<TYPES: NodeType>() -> UpgradeLock<TYPES> {
    UpgradeLock::new(Upgrade::trivial(DRB_AND_HEADER_UPGRADE_VERSION))
}

pub(crate) struct Consensus<TYPES: NodeType> {
    proposals: BTreeMap<ViewNumber, Proposal<TYPES>>,
    vid_shares: BTreeMap<ViewNumber, VidDisperseShare2<TYPES>>,
    states_verified: BTreeMap<ViewNumber, Commitment<Proposal<TYPES>>>,
    blocks_reconstructed: BTreeMap<ViewNumber, VidCommitment2>,
    certs: BTreeMap<ViewNumber, Certificate1<TYPES>>,
    timeout_certs: BTreeMap<ViewNumber, TimeoutCertificate2<TYPES>>,
    view_sync_certs: BTreeMap<ViewNumber, ViewSyncCommitCertificate2<TYPES>>,
    locked_qc: Option<Certificate1<TYPES>>,
    headers: BTreeMap<ViewNumber, TYPES::BlockHeader>,

    voted_1_views: BTreeSet<ViewNumber>,
    voted_2_views: BTreeSet<ViewNumber>,

    timeout_view: ViewNumber,

    event_rx: Receiver<ConsensusEvent<TYPES>>,
    event_tx: Sender<Event<TYPES>>,

    // TODO: We need a next epoch stake table to handle the transition
    // And a way to set these stake tables, probably an event from coordinator
    stake_table: TYPES::Membership,

    public_key: TYPES::SignatureKey,
    private_key: <TYPES::SignatureKey as SignatureKey>::PrivateKey,
}

impl<TYPES: NodeType> Consensus<TYPES> {
    pub fn new(
        event_rx: Receiver<ConsensusEvent<TYPES>>,
        event_tx: Sender<Event<TYPES>>,
        initial_stake_table: TYPES::Membership,
        public_key: TYPES::SignatureKey,
        private_key: <TYPES::SignatureKey as SignatureKey>::PrivateKey,
    ) -> Self {
        Self {
            proposals: BTreeMap::new(),
            states_verified: BTreeMap::new(),
            blocks_reconstructed: BTreeMap::new(),
            certs: BTreeMap::new(),
            timeout_certs: BTreeMap::new(),
            view_sync_certs: BTreeMap::new(),
            locked_qc: None,
            event_rx,
            event_tx,
            headers: BTreeMap::new(),
            public_key,
            timeout_view: ViewNumber::genesis(),
            stake_table: initial_stake_table,
            voted_1_views: BTreeSet::new(),
            voted_2_views: BTreeSet::new(),
            private_key,
            vid_shares: BTreeMap::new(),
        }
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
                self.handle_timeout_certificate(certificate)
            },
            ConsensusEvent::ViewSyncCertificate(certificate) => {
                self.handle_view_sync_certificate(certificate)
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
            ConsensusEvent::Timeout(view) => self.handle_timeout(view),
        }
        self.maybe_vote_1(view).await;
        self.maybe_vote_2(view).await;
        self.maybe_propose(view).await;
        Some(())
    }

    pub async fn run(&mut self) {
        while let Some(event) = self.event_rx.recv().await {
            self.handle_event(event).await;
        }
    }

    async fn maybe_propose(&mut self, view: ViewNumber) {}
    async fn maybe_vote_2(&mut self, view: ViewNumber) {}

    async fn maybe_vote_1(&mut self, view: ViewNumber) -> Option<()> {
        if self.voted_1_views.contains(&view) {
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

        // Verify the state commitment matches the proposal
        if state_commitment != &proposal.commit() {
            return None;
        }

        // TODO make the vote data be over a different commitment
        let inner_vote = SimpleVote::create_signed_vote(
            Vote1Data {
                leaf_commit: proposal.commit(),
                epoch: proposal.epoch,
                block_number: proposal.block_header.block_number(),
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
        let vote_event = Event::SendMessage(ConsensusMessage::Vote1(vote));
        self.event_tx.send(vote_event).await.unwrap();
        self.voted_1_views.insert(view);
        Some(())
    }
    async fn validate_safety(&mut self, proposal: &Proposal<TYPES>) -> Option<()> {
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
    async fn handle_proposal(&mut self, proposal: ProposalMessage<TYPES>) -> Option<()> {
        let view = proposal.view_number();
        let epoch = proposal.proposal.data.epoch;
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
        let total_weight =
            vid_total_weight(&self.stake_table.stake_table(Some(epoch)), Some(epoch));
        if !proposal.vid_share.verify(total_weight) {
            return None;
        }
        // Verify the proposal is valid
        self.validate_safety(&proposal.proposal.data).await?;
        self.proposals.insert(view, proposal.proposal.data);
        self.vid_shares.insert(view, proposal.vid_share);
        Some(())
    }

    fn handle_certificate1(&mut self, certificate: Certificate1<TYPES>) {
        self.certs.insert(certificate.view_number(), certificate);
    }

    fn handle_certificate2(&mut self, certificate: Certificate2<TYPES>) {
        // TODO: Handle certificate2 by deciding views
        todo!()
    }

    fn handle_timeout_certificate(&mut self, certificate: TimeoutCertificate2<TYPES>) {
        self.timeout_certs
            .insert(certificate.view_number(), certificate);
    }

    fn handle_view_sync_certificate(&mut self, certificate: ViewSyncCommitCertificate2<TYPES>) {
        self.view_sync_certs
            .insert(certificate.view_number(), certificate);
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
