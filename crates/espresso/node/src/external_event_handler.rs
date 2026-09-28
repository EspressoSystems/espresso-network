//! Should probably rename this to "external" or something

use anyhow::{Context, Result, bail};
use espresso_types::{PubKey, SeqTypes};
use hotshot::types::Message;
use hotshot_new_protocol::client::ClientApi;
use hotshot_types::message::MessageKind;
use request_response::network::Bytes;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{Receiver, Sender, error::TrySendError};
use vbs::{BinarySerializer, bincode_serializer::BincodeSerializer, version::StaticVersion};

use crate::context::TaskList;

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
    /// Creates a new `ExternalEventHandler` that sends outbound messages through the coordinator
    pub async fn new(
        tasks: &mut TaskList,
        request_response_sender: Sender<Bytes>,
        outbound_message_receiver: Receiver<OutboundMessage>,
        client_api: ClientApi<SeqTypes>,
        public_key: PubKey,
    ) -> Result<Self> {
        tasks.spawn(
            "ExternalEventHandler",
            Self::outbound_message_loop(outbound_message_receiver, client_api, public_key),
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
    async fn outbound_message_loop(
        mut receiver: Receiver<OutboundMessage>,
        client_api: ClientApi<SeqTypes>,
        public_key: PubKey,
    ) {
        while let Some(message) = receiver.recv().await {
            Self::send_via_coordinator(&client_api, message, public_key).await;
        }
    }

    /// Send an outbound message through the coordinator's network.
    ///
    /// The coordinator's network sends external payloads over the wire
    /// verbatim, so they must be self-framing: wrap the payload in the
    /// versioned `Message` envelope, which the receiving side's fallback
    /// decoder recognizes and unwraps.
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
                tracing::warn!(message = ?other, "dropping unsupported external message");
            },
        }
    }
}
