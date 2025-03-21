//! This file contains the [`Sender`] and [`Receiver`] traits. These traits are **used** by the
//! [`RequestResponseProtocol`] to send and receive messages from a network or other source.
//!
//! For HotShot I've gone ahead and done a blanket implementation for a [`Sender`] for all
//! [`ConnectedNetwork`]s. The reason it's not done for the [`Receiver`] is because both
//! HS and the confirmation layer will receive messages from a single point and _then_ decide
//! what to do with them (as opposed to having some sort of filtering mechanism). So for
//! [`Receiver`] I've done a blanket implementation for channels that send [`Vec<u8>`]s.

use std::{ops::Deref, sync::Arc};

use anyhow::{Context, Result};
use async_trait::async_trait;
use hotshot_types::traits::{network::ConnectedNetwork, signature_key::SignatureKey};
use tokio::sync::mpsc;

/// A type alias for a shareable byte array
pub type Bytes = Arc<Vec<u8>>;

/// The [`Sender`] trait is used to allow the [`RequestResponseProtocol`] to send messages to a specific recipient
#[async_trait]
pub trait Sender<K: SignatureKey + 'static>: Send + Sync + 'static + Clone {
    /// Send a message to the specified recipient
    async fn send_message(&self, message: &Bytes, recipient: K) -> Result<()>;
}

/// The [`Receiver`] trait is used to allow the [`RequestResponseProtocol`] to receive messages from a network
/// or other source.
#[async_trait]
pub trait Receiver: Send + Sync + 'static {
    /// Receive a message. Returning an error here means the receiver will _NEVER_ receive any more messages
    async fn receive_message(&mut self) -> Result<Bytes>;
}

/// A blanket implementation of the [`Sender`] trait for all types that dereference to [`ConnectedNetwork`]
#[async_trait]
impl<T, K> Sender<K> for T
where
    T: Deref<Target: ConnectedNetwork<K>> + Send + Sync + 'static + Clone,
    K: SignatureKey + 'static,
{
    async fn send_message(&self, message: &Bytes, recipient: K) -> Result<()> {
        // Just send the message to the recipient
        self.direct_message(message.to_vec(), recipient)
            .await
            .with_context(|| "failed to send message")
    }
}

/// An implementation of the [`Receiver`] trait for the [`mpsc::Receiver`] type. Allows us to send messages
/// to a channel and have the protocol receive them.
#[async_trait]
impl Receiver for mpsc::Receiver<Bytes> {
    async fn receive_message(&mut self) -> Result<Bytes> {
        //  Just receive a message from the channel
        self.recv().await.ok_or(anyhow::anyhow!("channel closed"))
    }
}
