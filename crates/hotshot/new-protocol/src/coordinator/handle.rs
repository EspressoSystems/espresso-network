use hotshot::traits::BlockPayload;
use hotshot_types::{
    data::{EpochNumber, Leaf2, QuorumProposal2, VidCommitment2, VidDisperse2, ViewNumber},
    drb::DrbResult,
    message::Proposal,
    simple_vote::HasEpoch,
    traits::{block_contents::BlockHeader, node_implementation::NodeType},
    vote::{Certificate, HasViewNumber},
};
use tokio::sync::mpsc::error::SendError;

use crate::{
    events::*,
    message::{Vote1, Vote2},
};

#[derive(Clone)]
pub(crate) struct CoordinatorHandle<TYPES: NodeType> {
    event_tx: tokio::sync::mpsc::Sender<ConsensusOutput<TYPES>>,
}

impl<TYPES: NodeType> CoordinatorHandle<TYPES> {
    pub fn new(event_tx: tokio::sync::mpsc::Sender<ConsensusOutput<TYPES>>) -> Self {
        Self { event_tx }
    }

    pub async fn send_proposal(
        &self,
        message: Proposal<TYPES, QuorumProposal2<TYPES>>,
        share: VidDisperse2<TYPES>,
    ) -> Result<(), SendError<ConsensusOutput<TYPES>>> {
        self.event_tx
            .send(ConsensusOutput::Action(Action::SendProposal(
                message, share,
            )))
            .await
    }

    pub async fn send_vote1(
        &self,
        message: Vote1<TYPES>,
    ) -> Result<(), SendError<ConsensusOutput<TYPES>>> {
        self.event_tx
            .send(ConsensusOutput::Action(Action::SendVote1(message)))
            .await
    }

    pub async fn send_vote2(
        &self,
        message: Vote2<TYPES>,
    ) -> Result<(), SendError<ConsensusOutput<TYPES>>> {
        self.event_tx
            .send(ConsensusOutput::Action(Action::SendVote2(message)))
            .await
    }

    pub async fn request_state(
        &self,
        proposal: QuorumProposal2<TYPES>,
        payload_size: u32,
    ) -> Result<(), SendError<ConsensusOutput<TYPES>>> {
        let parent_commitment = proposal.justify_qc.data().leaf_commit;
        self.event_tx
            .send(ConsensusOutput::Action(Action::RequestState(
                StateRequest {
                    view: proposal.view_number(),
                    parent_view: proposal.view_number() - 1,
                    epoch: proposal.epoch().unwrap(),
                    block_number: proposal.block_header.block_number(),
                    proposal,
                    parent_commitment,
                    payload_size,
                },
            )))
            .await
    }
    pub async fn request_block_and_header(
        &self,
        parent: QuorumProposal2<TYPES>,
        view: ViewNumber,
        epoch: EpochNumber,
    ) -> Result<(), SendError<ConsensusOutput<TYPES>>> {
        self.event_tx
            .send(ConsensusOutput::Action(Action::RequestBlockAndHeader(
                BlockAndHeaderRequest {
                    view,
                    parent_proposal: parent,
                    epoch,
                },
            )))
            .await
    }
    pub async fn send_decided(
        &self,
        decided: Vec<Leaf2<TYPES>>,
    ) -> Result<(), SendError<ConsensusOutput<TYPES>>> {
        self.event_tx
            .send(ConsensusOutput::Event(Event::LeafDecided(decided)))
            .await
    }
    pub async fn request_vid_disperse(
        &self,
        view: ViewNumber,
        epoch: EpochNumber,
        block: TYPES::BlockPayload,
        metadata: <TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
    ) -> Result<(), SendError<ConsensusOutput<TYPES>>> {
        self.event_tx
            .send(ConsensusOutput::Action(Action::RequestVidDisperse(
                view, epoch, block, metadata,
            )))
            .await
    }
    pub async fn respond_state(
        &self,
        request: StateRequest<TYPES>,
    ) -> Result<(), SendError<ConsensusOutput<TYPES>>> {
        self.event_tx
            .send(ConsensusOutput::Event(Event::StateVerified(request)))
            .await
    }
    pub async fn respond_header(
        &self,
        view: ViewNumber,
        header: TYPES::BlockHeader,
    ) -> Result<(), SendError<ConsensusOutput<TYPES>>> {
        self.event_tx
            .send(ConsensusOutput::Event(Event::HeaderCreated(view, header)))
            .await
    }
    pub async fn respond_block_reconstructed(
        &self,
        view: ViewNumber,
        payload: TYPES::BlockPayload,
        vid_commitment: VidCommitment2,
    ) -> Result<(), SendError<ConsensusOutput<TYPES>>> {
        self.event_tx
            .send(ConsensusOutput::Event(Event::BlockReconstructed(
                view,
                payload,
                vid_commitment,
            )))
            .await
    }

    pub async fn respond_certificate1(
        &self,
        cert: crate::message::Certificate1<TYPES>,
    ) -> Result<(), SendError<ConsensusOutput<TYPES>>> {
        self.event_tx
            .send(ConsensusOutput::Event(Event::Certificate1Formed(cert)))
            .await
    }

    pub async fn respond_certificate2(
        &self,
        cert: crate::message::Certificate2<TYPES>,
    ) -> Result<(), SendError<ConsensusOutput<TYPES>>> {
        self.event_tx
            .send(ConsensusOutput::Event(Event::Certificate2Formed(cert)))
            .await
    }

    pub async fn respond_drb(
        &self,
        result: DrbResult,
    ) -> Result<(), SendError<ConsensusOutput<TYPES>>> {
        self.event_tx
            .send(ConsensusOutput::Event(Event::DrbCalculated(result)))
            .await
    }

    pub async fn respond_vid_disperse(
        &self,
        payload_commitment: VidCommitment2,
        disperse: VidDisperse2<TYPES>,
    ) -> Result<(), SendError<ConsensusOutput<TYPES>>> {
        self.event_tx
            .send(ConsensusOutput::Event(Event::VidDisperseCreated(
                payload_commitment,
                disperse,
            )))
            .await
    }
}

mod test {
    use super::*;
    impl<TYPES: NodeType> CoordinatorHandle<TYPES> {
        pub async fn send_event(
            &self,
            event: ConsensusOutput<TYPES>,
        ) -> Result<(), SendError<ConsensusOutput<TYPES>>> {
            self.event_tx.send(event).await
        }
    }
}
