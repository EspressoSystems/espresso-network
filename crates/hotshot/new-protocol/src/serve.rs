use std::sync::Arc;

use hotshot_types::{
    data::{VidCommitment2, ViewNumber},
    traits::{block_contents::EncodeBytes, node_implementation::NodeType},
    vote::HasViewNumber,
};
use tracing::{debug, warn};

use crate::{
    coordinator::error::CoordinatorError,
    message::{FetchRequest, Message, MessageType, PayloadFetchMessage, PayloadFetchResponse},
    network::Sender,
};

/// The block this node hands to a peer whose payload fetch names it.
///
/// The bytes are encoded once, when the block is retained, so answering a
/// request costs no more than a copy into the response.
struct RetainedBlock {
    view: ViewNumber,
    payload_commitment: VidCommitment2,
    payload: Arc<[u8]>,
}

/// The serving half of payload recovery.
///
/// What a stalled peer asks for is the payload of the view the quorum is
/// locked at, so that is what this keeps: `locked`, replaced only when our own
/// lock moves. Reconstruction runs ahead of the lock and would otherwise
/// overwrite it, which is what happens in a stall, where later views keep
/// reconstructing while every lock stays put. The newest reconstruction
/// therefore waits in `latest` until the lock reaches it.
pub struct Server<T: NodeType> {
    public_key: T::SignatureKey,
    /// The payload of the view we are locked at.
    locked: Option<RetainedBlock>,
    /// The most recently reconstructed payload, promoted by
    /// [`Self::lock_moved`] once we lock the view it belongs to.
    latest: Option<RetainedBlock>,
}

impl<T: NodeType> Server<T> {
    pub fn new(public_key: T::SignatureKey) -> Self {
        Self {
            public_key,
            locked: None,
            latest: None,
        }
    }

    /// Hold on to a freshly obtained block, in place of the one held before.
    pub fn retain(
        &mut self,
        view: ViewNumber,
        payload_commitment: VidCommitment2,
        payload: &T::BlockPayload,
    ) {
        self.latest = Some(RetainedBlock {
            view,
            payload_commitment,
            payload: payload.encode(),
        });
    }

    /// Follow our lock to `view`, keeping that view's payload to serve.
    ///
    /// A lock only ever moves to a view we reconstructed, so the payload is
    /// normally the one [`Self::retain`] just took. It is not when two
    /// reconstructions ran before the certificate arrived: the older block is
    /// gone by then, and this node serves nothing for its lock until the lock
    /// moves again.
    pub fn lock_moved(&mut self, view: ViewNumber) {
        if self.latest.as_ref().is_some_and(|block| block.view == view) {
            self.locked = self.latest.take();
        }
    }

    /// Unicast the retained block to `sender` if that is what `request` asks for.
    pub fn handle_request(
        &self,
        request: &FetchRequest<T>,
        sender: &T::SignatureKey,
        slot: ViewNumber,
        network: &Sender<T>,
    ) -> Result<(), CoordinatorError> {
        let view = request.view_number();

        if !request.validate_sender(sender) {
            warn!(%view, %sender, "ignoring payload request with an invalid signature");
            return Ok(());
        }

        // `latest` is not what the lock rule promises to keep, but while it is
        // still held it answers just as well.
        let Some(block) = [self.locked.as_ref(), self.latest.as_ref()]
            .into_iter()
            .flatten()
            .find(|block| block.view == view)
        else {
            debug!(%view, %sender, "payload request for a block we do not retain");
            return Ok(());
        };

        let message = Message {
            sender: self.public_key.clone(),
            message_type: MessageType::PayloadFetch(PayloadFetchMessage::Response(
                PayloadFetchResponse {
                    view,
                    payload_commitment: block.payload_commitment,
                    payload: block.payload.to_vec(),
                },
            )),
        };

        network
            .unicast(slot, sender, &message)
            .map_err(|err| CoordinatorError::from(err).context("unicast payload response"))
    }
}
