use std::sync::mpsc::Receiver;

use hotshot_types::traits::node_implementation::NodeType;

use crate::events::*;

pub struct Coordinator<TYPES: NodeType> {
    event_rx: Receiver<Event<TYPES>>,
    cpu_tx: std::sync::mpsc::Sender<CpuEvent<TYPES>>,
    state_tx: tokio::sync::mpsc::Sender<StateEvent<TYPES>>,
    io_tx: tokio::sync::mpsc::Sender<IOEvent<TYPES>>,
    consensus_tx: tokio::sync::mpsc::Sender<ConsensusEvent<TYPES>>,
}
