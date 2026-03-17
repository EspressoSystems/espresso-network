use hotshot_types::{
    data::{EpochNumber, Leaf2, QuorumProposal2, ViewNumber},
    simple_vote::HasEpoch,
    traits::{block_contents::BlockHeader, node_implementation::NodeType},
    vote::HasViewNumber,
};
use tokio::sync::mpsc::error::SendError;

use crate::{events::*, message::ConsensusMessage};

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
        message: ConsensusMessage<TYPES>,
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
    pub async fn request_header(
        &self,
        parent: QuorumProposal2<TYPES>,
        view: ViewNumber,
        epoch: EpochNumber,
    ) -> Result<(), SendError<Event<TYPES>>> {
        self.event_tx
            .send(Event::Action(Action::RequestHeader(HeaderRequest {
                view,
                parent_view: parent.view_number(),
                epoch,
                block_number: parent.block_header.block_number() + 1,
            })))
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
}
