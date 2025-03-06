use crate::external_event_handler::ExternalMessage;
use crate::external_event_handler::OutboundMessage;
use anyhow::{Context, Result};
use async_trait::async_trait;
use espresso_types::PubKey;
use espresso_types::SeqTypes;
use hotshot_types::message::MessageKind;
use request_response::network::Bytes;
use request_response::network::Sender as SenderTrait;
use tokio::sync::mpsc;

/// A wrapper type that we will implement the `Sender` trait for
#[derive(Clone)]
pub struct Sender(mpsc::Sender<OutboundMessage>);

impl Sender {
    pub fn new(sender: mpsc::Sender<OutboundMessage>) -> Self {
        Self(sender)
    }
}

/// Implement the `Sender` trait for the `RequestResponseSender` type. This tells
/// the request response protocol how to send messages to other nodes.
#[async_trait]
impl SenderTrait<PubKey> for Sender {
    async fn send_message(&self, message: &Bytes, recipient: PubKey) -> Result<()> {
        // Serialize the inner message
        let message_bytes = bincode::serialize(&ExternalMessage::RequestResponse(message.to_vec()))
            .with_context(|| "failed to serialize message")?;

        // Send the message
        self.0
            .send(OutboundMessage::Direct(
                MessageKind::External::<SeqTypes>(message_bytes),
                recipient,
            ))
            .await
            .with_context(|| "failed to send message over channel")?;
        Ok(())
    }
}
