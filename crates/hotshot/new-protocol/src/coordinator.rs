use hotshot::{traits::NodeImplementation, types::SystemContextHandle};
use hotshot_types::traits::{consensus_api::ConsensusApi, node_implementation::NodeType};
use tokio::sync::mpsc::{self};

use crate::{consensus::Consensus, coordinator::handle::CoordinatorHandle, events::*};

pub mod handle;

#[cfg(test)]
pub(crate) mod mock;

const CHANNEL_BUFFER_SIZE: usize = 256;

pub(crate) struct Coordinator<TYPES: NodeType> {
    event_rx: tokio::sync::mpsc::Receiver<ConsensusOutput<TYPES>>,
    state_tx: tokio::sync::mpsc::Sender<StateEvent<TYPES>>,
    io_tx: tokio::sync::mpsc::Sender<IoEvent<TYPES>>,
    external_tx: async_broadcast::Sender<hotshot_types::event::Event<TYPES>>,
}

impl<TYPES: NodeType> Coordinator<TYPES> {
    pub async fn new<I: NodeImplementation<TYPES>>(
        external_tx: async_broadcast::Sender<hotshot_types::event::Event<TYPES>>,
        system_context: SystemContextHandle<TYPES, I>,
    ) -> (Self, CoordinatorHandle<TYPES>) {
        let (event_tx, event_rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);
        let (state_tx, state_rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);
        let (io_tx, io_rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);
        let coordinator = Self {
            event_rx,
            state_tx,
            io_tx,
            external_tx,
        };
        let coordinator_handle = CoordinatorHandle::new(event_tx);
        let consensus = Consensus::new(
            system_context.membership_coordinator.clone(),
            system_context.public_key(),
            system_context.private_key().clone(),
        );
        (coordinator, coordinator_handle)
    }

    pub async fn run(&mut self) {
        while let Some(event) = self.event_rx.recv().await {
            match event {
                ConsensusOutput::Event(update) => self.handle_update(update).await,
                ConsensusOutput::Action(action) => self.handle_action(action).await,
            }
        }
    }

    async fn handle_update(&mut self, update: Event<TYPES>) {
        todo!()
    }

    async fn handle_action(&mut self, action: Action<TYPES>) {
        todo!()
    }
}
