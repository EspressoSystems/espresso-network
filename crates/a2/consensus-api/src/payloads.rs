//! The payloads a consensus node hands its storage instances before their blocks are decided.
//!
//! Consensus [offers](PayloadQueue::offer) each payload once; the `TakePayloads` subscriptions
//! take them in turn, and a subscriber acknowledges each payload once it is durable, which
//! consensus learns from the [acknowledgements](Acks). A payload sent to a subscription that ends
//! before it is acknowledged is offered again.

use std::{collections::BTreeMap, sync::Arc};

use bytes::Bytes;
use tokio::sync::{broadcast, mpsc};

/// A payload for the storage instances.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offer {
    /// The view the block was proposed in.
    pub view: u64,

    /// The block's VID commitment, as tagged base64.
    pub commitment: String,

    /// The block's namespace table, as the header encodes it.
    pub ns_table: Bytes,

    pub payload: Bytes,
}

/// A payload a storage instance reported durable: its view and commitment.
pub type Acked = (u64, String);

/// The payloads consensus offers, shared between consensus and the service.
#[derive(Clone, Debug)]
pub struct PayloadQueue {
    offers: async_channel::Sender<Arc<Offer>>,
    taken: async_channel::Receiver<Arc<Offer>>,
    acks: mpsc::UnboundedSender<Acked>,
    acked: broadcast::Sender<Acked>,
}

/// The acknowledgements of the payloads consensus offered.
pub type Acks = mpsc::UnboundedReceiver<Acked>;

impl PayloadQueue {
    /// A queue, and the acknowledgements of what is offered on it.
    pub fn new() -> (Self, Acks) {
        let (offers, taken) = async_channel::unbounded();
        let (acks, acks_rx) = mpsc::unbounded_channel();
        let (acked, _) = broadcast::channel(1024);
        let queue = Self {
            offers,
            taken,
            acks,
            acked,
        };
        (queue, acks_rx)
    }

    /// Offers `offer` to the next subscription that asks.
    pub fn offer(&self, offer: Offer) {
        // The queue is unbounded, and holds a receiver itself.
        let _ = self.offers.try_send(Arc::new(offer));
    }

    /// Reports the payload `acked` durable, as a storage instance does.
    pub fn ack(&self, acked: Acked) {
        let _ = self.acks.send(acked.clone());
        let _ = self.acked.send(acked);
    }

    pub(crate) fn taker(&self) -> Taker {
        Taker {
            taken: self.taken.clone(),
            acked: self.acked.subscribe(),
            sent: Sent {
                offers: BTreeMap::new(),
                requeue: self.offers.clone(),
            },
        }
    }
}

/// One subscription's share of the queue.
pub(crate) struct Taker {
    taken: async_channel::Receiver<Arc<Offer>>,
    acked: broadcast::Receiver<Acked>,
    sent: Sent,
}

impl Taker {
    /// The next offer for this subscription, which it then holds until it is acknowledged.
    pub(crate) async fn next(&mut self) -> Option<Arc<Offer>> {
        loop {
            tokio::select! {
                offer = self.taken.recv() => {
                    let offer = offer.ok()?;
                    self.sent
                        .offers
                        .insert((offer.view, offer.commitment.clone()), offer.clone());
                    return Some(offer);
                },
                acked = self.acked.recv() => match acked {
                    Ok(acked) => {
                        self.sent.offers.remove(&acked);
                    },
                    // A missed acknowledgement only means the payload is offered again.
                    Err(broadcast::error::RecvError::Lagged(_)) => {},
                    Err(broadcast::error::RecvError::Closed) => return None,
                },
            }
        }
    }
}

/// What a subscription was sent and not seen acknowledged, which goes back to the queue when the
/// subscription ends.
struct Sent {
    offers: BTreeMap<Acked, Arc<Offer>>,
    requeue: async_channel::Sender<Arc<Offer>>,
}

impl Drop for Sent {
    fn drop(&mut self) {
        for (_, offer) in std::mem::take(&mut self.offers) {
            let _ = self.requeue.try_send(offer);
        }
    }
}
