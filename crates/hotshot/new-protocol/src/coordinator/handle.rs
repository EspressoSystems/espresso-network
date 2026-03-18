use hotshot::traits::BlockPayload;
use hotshot_types::{
    data::{EpochNumber, Leaf2, QuorumProposal2, ViewNumber},
    simple_vote::HasEpoch,
    traits::{block_contents::BlockHeader, node_implementation::NodeType},
    vote::HasViewNumber,
};
use tokio::sync::mpsc::error::SendError;

use crate::events::*;

#[derive(Clone)]
pub(crate) struct CoordinatorHandle<TYPES: NodeType> {
    event_tx: tokio::sync::mpsc::Sender<Event<TYPES>>,
}

impl<TYPES: NodeType> CoordinatorHandle<TYPES> {
    pub fn new(event_tx: tokio::sync::mpsc::Sender<Event<TYPES>>) -> Self {
        Self { event_tx }
    }

    pub async fn send_message(
        &self,
        message: RequestMessageSender<TYPES>,
    ) -> Result<(), SendError<Event<TYPES>>> {
        self.event_tx
            .send(Event::Action(Action::SendMessage(message)))
            .await
    }

    pub async fn request_state(
        &self,
        proposal: QuorumProposal2<TYPES>,
    ) -> Result<(), SendError<Event<TYPES>>> {
        self.event_tx
            .send(Event::Action(Action::RequestState(StateRequest {
                view: proposal.view_number(),
                parent_view: proposal.view_number() - 1,
                epoch: proposal.epoch().unwrap(),
                block_number: proposal.block_header.block_number(),
                proposal,
            })))
            .await
    }
    pub async fn request_block_and_header(
        &self,
        parent: QuorumProposal2<TYPES>,
        view: ViewNumber,
        epoch: EpochNumber,
    ) -> Result<(), SendError<Event<TYPES>>> {
        self.event_tx
            .send(Event::Action(Action::RequestBlockAndHeader(
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
    ) -> Result<(), SendError<Event<TYPES>>> {
        self.event_tx
            .send(Event::Update(Update::LeafDecided(decided)))
            .await
    }
    pub async fn request_vid_disperse(
        &self,
        view: ViewNumber,
        epoch: EpochNumber,
        block: TYPES::BlockPayload,
        metadata: <TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
    ) -> Result<(), SendError<Event<TYPES>>> {
        self.event_tx
            .send(Event::Action(Action::RequestVidDisperse(
                view, epoch, block, metadata,
            )))
            .await
    }
}

mod test {
    use super::*;
    impl<TYPES: NodeType> CoordinatorHandle<TYPES> {
        pub async fn send_event(&self, event: Event<TYPES>) -> Result<(), SendError<Event<TYPES>>> {
            self.event_tx.send(event).await
        }
    }
}
