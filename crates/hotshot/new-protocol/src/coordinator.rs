use std::sync::mpsc::Receiver;

use hotshot_types::{
    data::{EpochNumber, QuorumProposal2, ViewNumber},
    simple_vote::HasEpoch,
    traits::{block_contents::BlockHeader, node_implementation::NodeType},
    vote::HasViewNumber,
};

use crate::{events::*, message::ConsensusMessage};

pub struct Coordinator<TYPES: NodeType> {
    event_rx: Receiver<Event<TYPES>>,
    cpu_tx: std::sync::mpsc::Sender<CpuEvent<TYPES>>,
    state_tx: tokio::sync::mpsc::Sender<StateEvent<TYPES>>,
    io_tx: tokio::sync::mpsc::Sender<IOEvent<TYPES>>,
    consensus_tx: tokio::sync::mpsc::Sender<ConsensusEvent<TYPES>>,
}

#[derive(Clone)]
pub(crate) struct CoordinatorHandle<TYPES: NodeType> {
    event_tx: tokio::sync::mpsc::Sender<Event<TYPES>>,
}

impl<TYPES: NodeType> CoordinatorHandle<TYPES> {
    pub fn new(event_tx: tokio::sync::mpsc::Sender<Event<TYPES>>) -> Self {
        Self { event_tx }
    }

    pub fn send_message(&self, message: ConsensusMessage<TYPES>) {
        self.event_tx
            .send(Event::Action(Action::SendMessage(message)));
    }

    pub fn request_state(&self, proposal: QuorumProposal2<TYPES>) {
        self.event_tx
            .send(Event::Action(Action::RequestState(StateRequest {
                view: proposal.view_number(),
                parent_view: proposal.view_number() - 1,
                epoch: proposal.epoch().unwrap(),
                block_number: proposal.block_header.block_number(),
                proposal,
            })));
    }
    pub fn request_header(
        &self,
        parent: QuorumProposal2<TYPES>,
        view: ViewNumber,
        epoch: EpochNumber,
    ) {
        self.event_tx
            .send(Event::Action(Action::RequestHeader(HeaderRequest {
                view,
                parent_view: parent.view_number(),
                epoch,
                block_number: parent.block_header.block_number() + 1,
            })));
    }
}
