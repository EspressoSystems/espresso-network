use std::sync::Arc;

use hotshot_types::{
    data::{VidCommitment2, ViewNumber},
    traits::{block_contents::EncodeBytes, node_implementation::NodeType},
    vote::HasViewNumber,
};
use tracing::{debug, warn};

use crate::{
    coordinator::error::CoordinatorError,
    message::{
        Message, MessageType,
        payload::{
            PayloadFetchMessage, PayloadFetchRequest, PayloadFetchResponse, PayloadResponseBody,
        },
    },
    network::{NetworkError, Sender},
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
    /// The last view the network timed out on, which is what makes us willing
    /// to serve at all.
    timed_out: Option<ViewNumber>,
}

/// How long after a timeout this node keeps answering requests.
const SERVE_AFTER_TIMEOUT: u64 = 3;

impl<T: NodeType> Server<T> {
    pub fn new(public_key: T::SignatureKey) -> Self {
        Self {
            public_key,
            locked: None,
            latest: None,
            timed_out: None,
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

    /// Note that the network timed out on `view`.
    pub fn view_timed_out(&mut self, view: ViewNumber) {
        self.timed_out = self.timed_out.max(Some(view));
    }

    /// The block we can serve for `view`, if we hold it.
    ///
    /// `latest` is not what the lock rule promises to keep, but while it is
    /// still held it answers just as well.
    fn retained(&self, view: ViewNumber) -> Option<&RetainedBlock> {
        [self.locked.as_ref(), self.latest.as_ref()]
            .into_iter()
            .flatten()
            .find(|block| block.view == view)
    }

    /// Unicast the retained block to `sender` if that is what `request` asks for.
    pub fn handle_request(
        &self,
        request: &PayloadFetchRequest,
        sender: &T::SignatureKey,
        slot: ViewNumber,
        network: &Sender<T>,
    ) -> Result<(), CoordinatorError> {
        let view = request.view_number();

        // Silence rather than a refusal: a refusal sends the requester to
        // another peer at once, which is the last thing to do when the reason
        // we will not answer has nothing to do with which peer was asked.
        if !self.serving(slot) {
            debug!(%view, %sender, "payload request outside a timeout window");
            return Ok(());
        }

        let Some(block) = self.retained(view) else {
            debug!(%view, %sender, "payload request for a block we do not retain");
            return self.respond(
                view,
                PayloadResponseBody::NotAvailable,
                sender,
                slot,
                network,
            );
        };

        if block.payload.len() >= network.max_message_size().get() {
            warn!(
                %view,
                %sender,
                bytes = %block.payload.len(),
                limit = %network.max_message_size(),
                "retained payload does not fit a message"
            );
            return self.respond(view, PayloadResponseBody::TooLarge, sender, slot, network);
        }

        let message = Message {
            sender: self.public_key.clone(),
            message_type: {
                let body = PayloadResponseBody::Payload {
                    commitment: block.payload_commitment,
                    data: block.payload.to_vec(),
                };
                let res = PayloadFetchResponse::new(view, body);
                MessageType::PayloadFetch(PayloadFetchMessage::Res(res))
            },
        };

        match network.unicast(slot, sender, &message) {
            Ok(()) => Ok(()),
            Err(NetworkError::Cliquenet(cliquenet::NetworkError::MessageTooLarge)) => {
                warn!(%view, %sender, "payload response rejected as too large");
                self.respond(view, PayloadResponseBody::TooLarge, sender, slot, network)
            },
            Err(err) => Err(CoordinatorError::from(err).context("payload response")),
        }
    }

    fn respond(
        &self,
        view: ViewNumber,
        body: PayloadResponseBody,
        sender: &T::SignatureKey,
        slot: ViewNumber,
        network: &Sender<T>,
    ) -> Result<(), CoordinatorError> {
        let message = Message {
            sender: self.public_key.clone(),
            message_type: {
                let res = PayloadFetchResponse::new(view, body);
                MessageType::PayloadFetch(PayloadFetchMessage::Res(res))
            },
        };
        network
            .unicast(slot, sender, &message)
            .map_err(|err| CoordinatorError::from(err).context("payload response"))
    }

    /// Whether a payload fetch is plausible right now.
    fn serving(&self, current_view: ViewNumber) -> bool {
        self.timed_out
            .is_some_and(|view| current_view <= view + SERVE_AFTER_TIMEOUT)
    }
}

#[cfg(test)]
mod tests {
    use hotshot::types::BLSPubKey;
    use hotshot_example_types::{
        block_types::{TestBlockPayload, TestTransaction},
        node_types::TestTypes,
    };
    use hotshot_types::{
        data::{VidCommitment2, ViewNumber},
        traits::signature_key::SignatureKey,
    };

    use super::Server;

    fn server() -> Server<TestTypes> {
        Server::new(BLSPubKey::generated_from_seed_indexed([0; 32], 0).0)
    }

    fn payload() -> TestBlockPayload {
        TestBlockPayload {
            transactions: vec![TestTransaction::new(vec![1, 2, 3])],
        }
    }

    fn view(n: u64) -> ViewNumber {
        ViewNumber::new(n)
    }

    /// A freshly obtained block is not what we serve yet: the lock has not
    /// reached it, and a peer asking is asking for the view we are locked at.
    #[test]
    fn retaining_a_block_does_not_make_it_the_locked_one() {
        let mut server = server();
        server.retain(view(1), VidCommitment2::default(), &payload());

        assert!(server.locked.is_none());
        assert_eq!(server.latest.as_ref().map(|b| b.view), Some(view(1)));
    }

    /// Locking the view a candidate belongs to is what promotes it.
    #[test]
    fn locking_the_candidates_view_promotes_it() {
        let mut server = server();
        server.retain(view(1), VidCommitment2::default(), &payload());
        server.lock_moved(view(1));

        assert_eq!(server.locked.as_ref().map(|b| b.view), Some(view(1)));
        assert!(server.latest.is_none());
    }

    /// Both slots answer while both are held: the block we are locked at, and
    /// the one waiting for its lock.
    #[test]
    fn either_slot_serves_its_view() {
        let mut server = server();
        server.retain(view(1), VidCommitment2::default(), &payload());
        server.lock_moved(view(1));
        server.retain(view(2), VidCommitment2::default(), &payload());

        assert_eq!(server.retained(view(1)).map(|b| b.view), Some(view(1)));
        assert_eq!(server.retained(view(2)).map(|b| b.view), Some(view(2)));
        assert!(server.retained(view(3)).is_none());
    }

    /// The documented gap: two reconstructions before the certificate for the
    /// first one arrives leave nothing to promote, so the lock moves on while
    /// the served block stays where it was.
    #[test]
    fn a_lock_that_skips_a_candidate_promotes_nothing() {
        let mut server = server();
        server.retain(view(1), VidCommitment2::default(), &payload());
        server.lock_moved(view(1));
        server.retain(view(2), VidCommitment2::default(), &payload());
        server.retain(view(3), VidCommitment2::default(), &payload());
        server.lock_moved(view(2));

        assert_eq!(server.locked.as_ref().map(|b| b.view), Some(view(1)));
        assert_eq!(server.latest.as_ref().map(|b| b.view), Some(view(3)));
        assert!(server.retained(view(2)).is_none());
    }
}
