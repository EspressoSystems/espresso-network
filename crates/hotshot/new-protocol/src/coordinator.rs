use hotshot::{traits::NodeImplementation, types::SystemContextHandle};
use hotshot_types::traits::node_implementation::NodeType;

use crate::events::*;

const CHANNEL_BUFFER_SIZE: usize = 256;

pub(crate) struct Coordinator<TYPES: NodeType> {
    external_tx: async_broadcast::Sender<hotshot_types::event::Event<TYPES>>,
}

impl<TYPES: NodeType> Coordinator<TYPES> {
    pub async fn new<I: NodeImplementation<TYPES>>(
        external_tx: async_broadcast::Sender<hotshot_types::event::Event<TYPES>>,
        _system_context: SystemContextHandle<TYPES, I>,
    ) -> Self {
        Self { external_tx }
    }

    pub async fn run(&mut self) {
        todo!()
    }

    async fn handle_update(&mut self, update: Event<TYPES>) {
        todo!()
    }

    async fn handle_action(&mut self, action: Action<TYPES>) {
        todo!()
    }
}
