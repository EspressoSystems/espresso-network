use hotshot::{traits::NodeImplementation, types::SystemContextHandle};
use hotshot_types::traits::node_implementation::NodeType;

use crate::events::*;

const CHANNEL_BUFFER_SIZE: usize = 256;

pub(crate) struct Coordinator<T: NodeType, I: NodeImplementation<T>> {
    external_tx: async_broadcast::Sender<hotshot_types::event::Event<T>>,
    system_context: SystemContextHandle<T, I>,
}

impl<T: NodeType, I: NodeImplementation<T>> Coordinator<T, I> {
    pub async fn new(
        external_tx: async_broadcast::Sender<hotshot_types::event::Event<T>>,
        system_context: SystemContextHandle<T, I>,
    ) -> Self {
        Self {
            external_tx,
            system_context,
        }
    }

    pub async fn run(&mut self) {
        todo!()
    }

    async fn handle_update(&mut self, update: Event<T>) {
        todo!()
    }

    async fn handle_action(&mut self, action: Action<T>) {
        todo!()
    }
}
