use hotshot::{traits::NodeImplementation, types::SystemContextHandle};
use hotshot_types::traits::{consensus_api::ConsensusApi, node_implementation::NodeType};
use tokio::sync::mpsc::{self};

use crate::{consensus::Consensus, coordinator::handle::CoordinatorHandle, events::*};

pub mod handle;
pub(crate) mod mock;

const CHANNEL_BUFFER_SIZE: usize = 256;

pub(crate) struct Coordinator<TYPES: NodeType> {
    event_rx: tokio::sync::mpsc::Receiver<Event<TYPES>>,
    cpu_tx: std::sync::mpsc::Sender<CpuEvent<TYPES>>,
    state_tx: tokio::sync::mpsc::Sender<StateEvent<TYPES>>,
    io_tx: tokio::sync::mpsc::Sender<IOEvent<TYPES>>,
    consensus_tx: tokio::sync::mpsc::Sender<ConsensusEvent<TYPES>>,
    external_tx: async_broadcast::Sender<hotshot_types::event::Event<TYPES>>,
}

impl<TYPES: NodeType> Coordinator<TYPES> {
    pub async fn new<I: NodeImplementation<TYPES>>(
        external_tx: async_broadcast::Sender<hotshot_types::event::Event<TYPES>>,
        system_context: SystemContextHandle<TYPES, I>,
    ) -> (Self, CoordinatorHandle<TYPES>) {
        let (event_tx, event_rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);
        let (cpu_tx, cpu_rx) = std::sync::mpsc::channel();
        let (state_tx, state_rx) = tokio::sync::mpsc::channel(CHANNEL_BUFFER_SIZE);
        let (io_tx, io_rx) = tokio::sync::mpsc::channel(CHANNEL_BUFFER_SIZE);
        let (consensus_tx, consensus_rx) = tokio::sync::mpsc::channel(CHANNEL_BUFFER_SIZE);
        let coordinator = Self {
            event_rx,
            cpu_tx,
            state_tx,
            io_tx,
            consensus_tx,
            external_tx,
        };
        let coordinator_handle = CoordinatorHandle::new(event_tx);
        let consensus = Consensus::new(
            consensus_rx,
            coordinator_handle.clone(),
            system_context.membership_coordinator.clone(),
            system_context.public_key(),
            system_context.private_key().clone(),
        );
        (coordinator, coordinator_handle)
    }
}
