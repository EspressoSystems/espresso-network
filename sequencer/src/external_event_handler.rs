//! Should probably rename this to "external" or something

use std::{marker::PhantomData, sync::Arc};

use anyhow::{Context, Result};
use espresso_types::{PubKey, SeqTypes};
use hotshot::types::Message;
use hotshot_types::{
    message::{MessageKind, UpgradeLock},
    traits::{
        network::{BroadcastDelay, ConnectedNetwork, Topic},
        node_implementation::Versions,
    },
};
use request_response::network::Bytes;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::context::TaskList;

/// An external message that can be sent to or received from a node
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ExternalMessage {
    RequestResponse(Vec<u8>),
}

/// The external event handler
#[derive(Clone)]
pub struct ExternalEventHandler<V: Versions> {
    /// The sender to the request-response protocol
    request_response_sender: Sender<Bytes>,

    /// The type phantom
    phantom: PhantomData<V>,
}

// The different types of outbound messages (broadcast or direct)
#[derive(Debug)]
#[allow(dead_code)]
pub enum OutboundMessage {
    Direct(MessageKind<SeqTypes>, PubKey),
    Broadcast(MessageKind<SeqTypes>),
}

impl<V: Versions> ExternalEventHandler<V> {
    /// Creates a new `ExternalEventHandler` with the given network
    pub async fn new<N: ConnectedNetwork<PubKey>>(
        tasks: &mut TaskList,
        request_response_sender: Sender<Bytes>,
        outbound_message_receiver: Receiver<OutboundMessage>,
        network: Arc<N>,
        public_key: PubKey,
        hotshot_upgrade_lock: UpgradeLock<SeqTypes, V>,
    ) -> Result<Self> {
        // Spawn the outbound message handling loop
        tasks.spawn(
            "ExternalEventHandler",
            Self::outbound_message_loop(
                outbound_message_receiver,
                network,
                public_key,
                hotshot_upgrade_lock,
            ),
        );

        Ok(Self {
            request_response_sender,
            phantom: PhantomData,
        })
    }

    /// Handles an event
    ///
    /// # Errors
    /// If the message type is unknown or if there is an error serializing or deserializing the message
    pub async fn handle_event(&self, external_message_bytes: &[u8]) -> Result<()> {
        // Deserialize the external message
        let external_message = bincode::deserialize(external_message_bytes)
            .with_context(|| "Failed to deserialize external message")?;

        // Match the type
        match external_message {
            ExternalMessage::RequestResponse(request_response) => {
                // Send the inner message to the request-response protocol
                self.request_response_sender
                    .send(request_response.into())
                    .await?;
            },
        }
        Ok(())
    }

    /// The main loop for sending outbound messages.
    async fn outbound_message_loop<N: ConnectedNetwork<PubKey>>(
        mut receiver: Receiver<OutboundMessage>,
        network: Arc<N>,
        public_key: PubKey,
        hotshot_upgrade_lock: UpgradeLock<SeqTypes, V>,
    ) {
        while let Some(message) = receiver.recv().await {
            // Match the message type
            match message {
                OutboundMessage::Direct(message, recipient) => {
                    // Wrap it in the real message type
                    let message_inner = Message {
                        sender: public_key,
                        kind: message,
                    };

                    // Serialize it
                    let message_bytes = match hotshot_upgrade_lock.serialize(&message_inner).await {
                        Ok(message_bytes) => message_bytes,
                        Err(err) => {
                            tracing::warn!("Failed to serialize direct message: {}", err);
                            continue;
                        },
                    };

                    // Send the message to the recipient
                    if let Err(err) = network.direct_message(message_bytes, recipient).await {
                        tracing::warn!("Failed to send message: {:?}", err);
                    };
                },

                OutboundMessage::Broadcast(message) => {
                    // Wrap it in the real message type
                    let message_inner = Message {
                        sender: public_key,
                        kind: message,
                    };

                    // Serialize it
                    let message_bytes = match hotshot_upgrade_lock.serialize(&message_inner).await {
                        Ok(message_bytes) => message_bytes,
                        Err(err) => {
                            tracing::warn!("Failed to serialize broadcast message: {}", err);
                            continue;
                        },
                    };

                    // Broadcast the message to the global topic
                    if let Err(err) = network
                        .broadcast_message(message_bytes, Topic::Global, BroadcastDelay::None)
                        .await
                    {
                        tracing::error!("Failed to broadcast message: {:?}", err);
                    };
                },
            }
        }
    }
}
