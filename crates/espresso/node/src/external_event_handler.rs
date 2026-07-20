//! Should probably rename this to "external" or something

use std::sync::Arc;

use anyhow::{Context, Result, bail};
use espresso_types::{PubKey, SeqTypes, v0::traits::SequencerPersistence};
use hotshot::types::Message;
use hotshot_new_protocol::client::ClientApi;
use hotshot_types::{
    message::MessageKind,
    traits::network::{BroadcastDelay, ConnectedNetwork, Topic, ViewMessage},
};
use request_response::network::Bytes;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{Receiver, Sender, error::TrySendError};
use vbs::{BinarySerializer, bincode_serializer::BincodeSerializer, version::StaticVersion};

use crate::{
    consensus_handle::ConsensusHandle,
    context::{ConsensusNode, TaskList},
};

/// An external message that can be sent to or received from a node
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ExternalMessage {
    RequestResponse(#[serde(with = "serde_bytes")] Vec<u8>),
}

/// The external event handler
#[derive(Clone)]
pub struct ExternalEventHandler {
    /// The sender to the request-response protocol
    request_response_sender: Sender<Bytes>,
}

// The different types of outbound messages (broadcast or direct)
#[derive(Debug)]
#[allow(dead_code)]
pub enum OutboundMessage {
    Direct(MessageKind<SeqTypes>, PubKey),
    Broadcast(MessageKind<SeqTypes>),
}

impl ExternalEventHandler {
    /// Creates a new `ExternalEventHandler` with the given network
    pub async fn new<N, P>(
        tasks: &mut TaskList,
        request_response_sender: Sender<Bytes>,
        outbound_message_receiver: Receiver<OutboundMessage>,
        consensus_handle: Arc<ConsensusHandle<SeqTypes, ConsensusNode<N, P>>>,
        network: Arc<N>,
        public_key: PubKey,
    ) -> Result<Self>
    where
        N: ConnectedNetwork<PubKey>,
        P: SequencerPersistence,
    {
        // Spawn the outbound message handling loop
        tasks.spawn(
            "ExternalEventHandler",
            Self::outbound_message_loop(
                outbound_message_receiver,
                consensus_handle,
                network,
                public_key,
            ),
        );

        Ok(Self {
            request_response_sender,
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
                match self
                    .request_response_sender
                    .try_send(request_response.into())
                {
                    Ok(()) => Ok(()),
                    Err(TrySendError::Full(..)) => bail!("request-response channel full"),
                    Err(TrySendError::Closed(..)) => bail!("request-response channel closed"),
                }
            },
        }
    }

    /// The main loop for sending outbound messages.
    async fn outbound_message_loop<N, P>(
        mut receiver: Receiver<OutboundMessage>,
        consensus_handle: Arc<ConsensusHandle<SeqTypes, ConsensusNode<N, P>>>,
        network: Arc<N>,
        public_key: PubKey,
    ) where
        N: ConnectedNetwork<PubKey>,
        P: SequencerPersistence,
    {
        let mut network = Some(network);

        while let Some(message) = receiver.recv().await {
            // Once the coordinator is running it owns the only live network;
            // route external messages through it. The coordinator never
            // stops once started, so drop the legacy network for good.
            if let Some(client_api) = consensus_handle.client_api().await {
                network = None;
                Self::send_via_coordinator(&client_api, message, public_key).await;
                continue;
            }
            let Some(network) = &network else {
                continue;
            };

            // Match the message type
            match message {
                OutboundMessage::Direct(message, recipient) => {
                    let view = message.view_number();
                    // Wrap it in the real message type
                    let message_inner = Message {
                        sender: public_key,
                        kind: message,
                    };

                    // Serialize it
                    let message_bytes =
                        match BincodeSerializer::<StaticVersion<0, 0>>::serialize(&message_inner) {
                            Ok(message_bytes) => message_bytes,
                            Err(err) => {
                                tracing::warn!("Failed to serialize direct message: {}", err);
                                continue;
                            },
                        };

                    // Send the message to the recipient
                    let network = Arc::clone(network);
                    tokio::spawn(async move {
                        if let Err(err) =
                            network.direct_message(view, message_bytes, recipient).await
                        {
                            tracing::warn!("Failed to send message: {:?}", err);
                        }
                    });
                },

                OutboundMessage::Broadcast(message) => {
                    let view = message.view_number();
                    // Wrap it in the real message type
                    let message_inner = Message {
                        sender: public_key,
                        kind: message,
                    };

                    // Serialize it
                    let message_bytes =
                        match BincodeSerializer::<StaticVersion<0, 0>>::serialize(&message_inner) {
                            Ok(message_bytes) => message_bytes,
                            Err(err) => {
                                tracing::warn!("Failed to serialize broadcast message: {}", err);
                                continue;
                            },
                        };

                    // Broadcast the message to the global topic
                    if let Err(err) = network
                        .broadcast_message(view, message_bytes, Topic::Global, BroadcastDelay::None)
                        .await
                    {
                        tracing::error!("Failed to broadcast message: {:?}", err);
                    };
                },
            }
        }
    }

    /// Send an outbound message through the coordinator's network.
    ///
    /// The coordinator's network sends external payloads over the wire
    /// verbatim, so they must be self-framing: wrap the payload in the same
    /// versioned `Message` envelope the legacy path uses, which the receiving
    /// side's fallback decoder recognizes and unwraps.
    async fn send_via_coordinator(
        client_api: &ClientApi<SeqTypes>,
        message: OutboundMessage,
        public_key: PubKey,
    ) {
        match message {
            OutboundMessage::Direct(kind @ MessageKind::External(_), recipient) => {
                let message = Message {
                    sender: public_key,
                    kind,
                };
                let message_bytes =
                    match BincodeSerializer::<StaticVersion<0, 0>>::serialize(&message) {
                        Ok(message_bytes) => message_bytes,
                        Err(err) => {
                            tracing::warn!("Failed to serialize direct message: {}", err);
                            return;
                        },
                    };
                if let Err(err) = client_api
                    .send_external_message(message_bytes, recipient)
                    .await
                {
                    tracing::warn!(%err, "failed to send external message via coordinator");
                }
            },
            // All request-response traffic uses batched direct messages; the
            // coordinator's network has no broadcast topic for external
            // messages.
            other => {
                tracing::warn!(
                    message = ?other,
                    "dropping unsupported external message after cutover"
                );
            },
        }
    }
}
