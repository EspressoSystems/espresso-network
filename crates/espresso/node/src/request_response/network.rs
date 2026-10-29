use anyhow::{Context, Result};
use async_trait::async_trait;
use espresso_types::PubKey;
use request_response::network::{Bytes, Sender as SenderTrait};
use tokio::sync::mpsc;

use crate::external_event_handler::OutboundMessage;

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
    async fn send_direct_message(&self, message: &Bytes, recipient: PubKey) -> Result<()> {
        self.0
            .send(OutboundMessage::Direct(message.to_vec(), recipient))
            .await
            .with_context(|| "failed to send message over channel")?;
        Ok(())
    }
}
