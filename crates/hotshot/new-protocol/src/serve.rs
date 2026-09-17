use std::{collections::BTreeMap, sync::Arc};

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
    payload_commitment: VidCommitment2,
    payload: Arc<[u8]>,
}

/// The serving half of payload recovery.
///
/// What a stalled peer asks for is the payload of the view the quorum is
/// locked at, so that is what this keeps, and `locked` names it. Payloads
/// arrive before the certificate that moves the lock onto them, and in a
/// stall later views keep reconstructing while every lock stays put, so a few
/// of the most recent are held as candidates until one of them is locked.
pub struct Server<T: NodeType> {
    public_key: T::SignatureKey,
    /// The blocks we can serve, newest last.
    blocks: BTreeMap<ViewNumber, RetainedBlock>,
    /// The view we are locked at, whose block is never evicted.
    locked: Option<ViewNumber>,
    /// The last view the network timed out on, which is what makes us willing
    /// to serve at all.
    timed_out: Option<ViewNumber>,
}

/// How long after a timeout this node keeps answering requests.
const SERVE_AFTER_TIMEOUT: u64 = 3;

/// How many blocks to keep.
///
/// One is the payload of the view we are locked at, which is what peers ask
/// for. The rest are candidates waiting for a certificate to move the lock
/// onto them, so this bounds how late a certificate may be — in payloads
/// obtained meanwhile, which is roughly one per view — before the block it
/// certifies has been evicted and the node can no longer serve it.
///
/// A count is the wrong shape if blocks ever approach the message size limit;
/// it would then want to be a byte budget that always keeps the locked block
/// and the newest.
const RETAINED_BLOCKS: usize = 4;

impl<T: NodeType> Server<T> {
    pub fn new(public_key: T::SignatureKey) -> Self {
        Self {
            public_key,
            blocks: BTreeMap::new(),
            locked: None,
            timed_out: None,
        }
    }

    /// Hold on to a freshly obtained block, evicting the earliest one we no
    /// longer need.
    ///
    /// Which of the retained blocks the lock will land on is not known yet:
    /// payloads arrive before the certificates that move the lock onto them,
    /// and decoding several views can finish out of order. Keeping the last
    /// few means the one the lock reaches is still here when it does.
    pub fn retain(
        &mut self,
        view: ViewNumber,
        payload_commitment: VidCommitment2,
        payload: &T::BlockPayload,
    ) {
        self.blocks.insert(
            view,
            RetainedBlock {
                payload_commitment,
                payload: payload.encode(),
            },
        );

        while self.blocks.len() > RETAINED_BLOCKS
            && let Some(&v) = self.blocks.keys().find(|v| Some(**v) != self.locked)
        {
            self.blocks.remove(&v);
        }
    }

    /// Follow our lock to `view`, so that view's block is the one kept.
    ///
    /// A lock only ever moves to a view we reconstructed, so we normally hold
    /// its block. We do not when the certificate arrived so late that the
    /// block was evicted, and then this node serves nothing for its lock until
    /// the lock moves again.
    pub fn lock_moved(&mut self, view: ViewNumber) {
        if self.blocks.contains_key(&view) {
            self.locked = Some(view);
        }
    }

    /// Note that the network timed out on `view`.
    pub fn view_timed_out(&mut self, view: ViewNumber) {
        self.timed_out = self.timed_out.max(Some(view));
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

        let Some(block) = self.blocks.get(&view) else {
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

    use super::{RETAINED_BLOCKS, Server};

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

    /// A freshly obtained block is servable at once, but the lock has not
    /// reached it: a peer asking is asking for the view we are locked at.
    #[test]
    fn retaining_a_block_does_not_move_the_lock() {
        let mut server = server();
        server.retain(view(1), VidCommitment2::default(), &payload());

        assert!(server.locked.is_none());
        assert!(server.blocks.contains_key(&view(1)));
    }

    /// Locking a view we hold the block for makes it the one we keep.
    #[test]
    fn locking_a_retained_view_keeps_its_block() {
        let mut server = server();
        server.retain(view(1), VidCommitment2::default(), &payload());
        server.lock_moved(view(1));

        assert_eq!(server.locked, Some(view(1)));
        assert!(server.blocks.contains_key(&view(1)));
    }

    /// Every retained block answers for its own view, whether or not the lock
    /// has reached it.
    #[test]
    fn any_retained_block_serves_its_view() {
        let mut server = server();
        server.retain(view(1), VidCommitment2::default(), &payload());
        server.lock_moved(view(1));
        server.retain(view(2), VidCommitment2::default(), &payload());

        assert!(server.blocks.contains_key(&view(1)));
        assert!(server.blocks.contains_key(&view(2)));
        assert!(!server.blocks.contains_key(&view(3)));
    }

    /// A certificate arriving after later views were reconstructed still
    /// finds its block, which one slot could not survive.
    #[test]
    fn a_late_certificate_still_finds_its_block() {
        let mut server = server();
        server.retain(view(1), VidCommitment2::default(), &payload());
        server.retain(view(2), VidCommitment2::default(), &payload());
        server.retain(view(3), VidCommitment2::default(), &payload());
        server.lock_moved(view(1));

        assert_eq!(server.locked, Some(view(1)));
        assert!(server.blocks.contains_key(&view(1)));
    }

    /// Blocks obtained out of order each take their own place.
    #[test]
    fn an_earlier_block_does_not_displace_a_later_one() {
        let mut server = server();
        server.retain(view(2), VidCommitment2::default(), &payload());
        server.retain(view(1), VidCommitment2::default(), &payload());

        assert!(server.blocks.contains_key(&view(1)));
        assert!(server.blocks.contains_key(&view(2)));
    }

    /// Eviction takes the earliest block, and never the locked one.
    #[test]
    fn eviction_spares_the_locked_block() {
        let mut server = server();
        server.retain(view(1), VidCommitment2::default(), &payload());
        server.lock_moved(view(1));
        for v in 2..=(RETAINED_BLOCKS as u64 + 2) {
            server.retain(view(v), VidCommitment2::default(), &payload());
        }

        assert_eq!(server.blocks.len(), RETAINED_BLOCKS);
        assert!(
            server.blocks.contains_key(&view(1)),
            "the locked block is never evicted"
        );
        assert!(
            !server.blocks.contains_key(&view(2)),
            "the earliest unlocked block goes first"
        );
    }
}
