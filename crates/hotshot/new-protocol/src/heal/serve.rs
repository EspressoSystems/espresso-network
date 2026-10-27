use std::{num::NonZeroUsize, sync::Arc};

use hotshot_types::{
    data::{VidDisperseShare, ViewNumber},
    traits::{
        metrics::{Counter, CounterFamily, Metrics},
        node_implementation::NodeType,
    },
    vote::HasViewNumber,
};
use tokio::sync::{Semaphore, watch};
use tracing::warn;

use crate::{
    heal::HealConfig,
    message::{
        Message, MessageType,
        fetch::Response,
        share::{ShareFetchMessage, ShareFetchRequest, ShareFetchResponse, ShareResponseBody},
    },
    network::Sender,
    storage::NewProtocolStorage,
};

/// Headroom the network reserves above a block for message framing.
const FRAMING_BYTES: usize = 64 << 10;

/// Sends a response to a peer, unicast in the given slot.
pub type Reply<T> =
    Arc<dyn Fn(&<T as NodeType>::SignatureKey, ViewNumber, ShareFetchResponse<T>) + Send + Sync>;

/// Serves this node's stored VID share to a peer that lost its own.
///
/// `handle_request` only spawns: storage reads never run on the coordinator task, and at most
/// `serve_inflight` reads run at once, further requests are answered `Busy`.
pub struct ShareServer<T: NodeType, S> {
    storage: S,
    reply: Reply<T>,
    /// The node's current view, read when a reply is sent: the network drops sends below its
    /// garbage-collection bound, which a slot captured at request time can fall under.
    view: watch::Receiver<ViewNumber>,
    inflight: Arc<Semaphore>,
    max_response_bytes: usize,
    metrics: ServeMetrics,
}

#[derive(Clone)]
struct ServeMetrics {
    results: Arc<dyn CounterFamily>,
    bytes: Arc<dyn Counter>,
}

impl ServeMetrics {
    fn new(metrics: &dyn Metrics) -> Self {
        Self {
            results: metrics
                .counter_family("share_serve_total".into(), vec!["result".into()])
                .into(),
            bytes: metrics
                .create_counter("share_serve_bytes_total".into(), None)
                .into(),
        }
    }

    fn result(&self, result: &str) {
        self.results.create(vec![result.into()]).add(1);
    }
}

impl<T: NodeType, S: NewProtocolStorage<T>> ShareServer<T, S> {
    pub fn new(
        storage: S,
        reply: Reply<T>,
        view: watch::Receiver<ViewNumber>,
        serve_inflight: NonZeroUsize,
        max_response_bytes: usize,
        metrics: &dyn Metrics,
    ) -> Self {
        Self {
            storage,
            reply,
            view,
            inflight: Arc::new(Semaphore::new(serve_inflight.get())),
            max_response_bytes,
            metrics: ServeMetrics::new(metrics),
        }
    }

    /// A server answering over `network`, with responses capped to what one message can carry.
    pub fn over_network(
        storage: S,
        network: Sender<T>,
        public_key: T::SignatureKey,
        view: watch::Receiver<ViewNumber>,
        config: &HealConfig,
        metrics: &dyn Metrics,
    ) -> Self {
        let max_bytes = config.max_response_bytes.min(
            network
                .max_message_size()
                .get()
                .saturating_sub(FRAMING_BYTES),
        );
        let reply: Reply<T> = Arc::new(move |to, slot, response| {
            let view = response.view_number();
            let message = Message {
                sender: public_key.clone(),
                message_type: MessageType::ShareFetch(ShareFetchMessage::Res(response)),
            };
            if let Err(err) = network.unicast(slot, to, &message) {
                warn!(%view, %err, "failed to send share response");
            }
        });
        Self::new(
            storage,
            reply,
            view,
            config.serve_inflight,
            max_bytes,
            metrics,
        )
    }

    /// Answer `request` from `to`. Returns without waiting for storage.
    pub fn handle_request(&self, request: ShareFetchRequest, to: T::SignatureKey) {
        let view = request.view_number();
        let Ok(permit) = self.inflight.clone().try_acquire_owned() else {
            (self.reply)(
                &to,
                *self.view.borrow(),
                Response::new(view, ShareResponseBody::Busy),
            );
            self.metrics.result("busy");
            return;
        };
        let storage = self.storage.clone();
        let reply = self.reply.clone();
        let current = self.view.clone();
        let max_bytes = self.max_response_bytes;
        let metrics = self.metrics.clone();
        tokio::spawn(async move {
            let (body, result) = load_response(&storage, &request, max_bytes).await;
            metrics.result(result);
            if let ShareResponseBody::Share(share) = &body {
                metrics.bytes.add(serialized_len(share));
            }
            let slot = *current.borrow();
            reply(&to, slot, Response::new(view, body));
            drop(permit);
        });
    }
}

pub(super) fn serialized_len<V: serde::Serialize>(value: &V) -> usize {
    bincode::serialized_size(value).map_or(usize::MAX, |len| len as usize)
}

/// The response for `request` and its label for the `result` metric.
pub(crate) async fn load_response<T: NodeType, S: NewProtocolStorage<T>>(
    storage: &S,
    request: &ShareFetchRequest,
    max_bytes: usize,
) -> (ShareResponseBody<T>, &'static str) {
    let view = request.view_number();
    let loaded = match storage.load_vid_share(view).await {
        Ok(loaded) => loaded,
        Err(err) => {
            warn!(%view, %err, "failed to load VID share for a peer");
            return (ShareResponseBody::NotHeld, "error");
        },
    };
    let share = match loaded.map(|proposal| proposal.data) {
        Some(VidDisperseShare::V2(share))
            if share.payload_commitment == request.body().commitment =>
        {
            share
        },
        _ => return (ShareResponseBody::NotHeld, "not_held"),
    };
    if serialized_len(&share) > max_bytes {
        return (ShareResponseBody::TooLarge, "too_large");
    }
    (ShareResponseBody::Share(Box::new(share)), "served")
}

#[cfg(test)]
mod tests {
    use hotshot_example_types::{node_types::TestTypes, storage_types::TestStorage};
    use hotshot_types::{
        data::VidDisperseShare,
        signature_key::BLSPubKey,
        traits::{metrics::NoMetrics, signature_key::SignatureKey},
    };
    use tokio::sync::mpsc;

    use super::*;
    use crate::{message::share::ShareRequestBody, tests::common::utils::TestData};

    type Key = <TestTypes as NodeType>::SignatureKey;

    fn peer() -> Key {
        BLSPubKey::generated_from_seed_indexed([0u8; 32], 2).0
    }

    /// A server over a storage holding node 0's share of view 1, and the channel its replies land
    /// in.
    async fn server(
        max_bytes: usize,
    ) -> (
        ShareServer<TestTypes, TestStorage<TestTypes>>,
        mpsc::UnboundedReceiver<(ViewNumber, ShareFetchResponse<TestTypes>)>,
        ShareFetchRequest,
        watch::Sender<ViewNumber>,
    ) {
        let data = TestData::new(1).await;
        let view = &data.views[0];
        let own = BLSPubKey::generated_from_seed_indexed([0u8; 32], 0);
        let share = VidDisperseShare::V2(view.vid_share_for(&own.0))
            .to_proposal(&own.1)
            .unwrap();
        let storage = TestStorage::<TestTypes>::default();
        hotshot_types::traits::storage::Storage::append_vid(&storage, &share)
            .await
            .unwrap();
        let (tx, rx) = mpsc::unbounded_channel();
        let reply: Reply<TestTypes> = Arc::new(move |_, slot, response| {
            tx.send((slot, response)).unwrap();
        });
        let (view_tx, view_rx) = watch::channel(ViewNumber::new(5));
        let server = ShareServer::new(
            storage,
            reply,
            view_rx,
            NonZeroUsize::MIN,
            max_bytes,
            &NoMetrics,
        );
        let request = ShareFetchRequest::new(
            view.view_number,
            ShareRequestBody {
                commitment: view.vid_commitment(),
            },
        );
        (server, rx, request, view_tx)
    }

    /// The view moves on between the request and the reply, and the reply goes out in the new one.
    #[tokio::test]
    async fn heal_serve_replies_in_current_view_ok() {
        let (server, mut replies, request, view) = server(usize::MAX).await;
        server.handle_request(request, peer());
        view.send_replace(ViewNumber::new(9));
        let (slot, response) = replies.recv().await.unwrap();
        assert!(matches!(response.body(), ShareResponseBody::Share(_)));
        assert_eq!(slot, ViewNumber::new(9));
    }

    #[tokio::test]
    async fn heal_serve_busy_ok() {
        let (server, mut replies, request, _view) = server(usize::MAX).await;
        let _held = server.inflight.clone().try_acquire_owned().unwrap();
        server.handle_request(request, peer());
        let (_, response) = replies.recv().await.unwrap();
        assert_eq!(*response.body(), ShareResponseBody::Busy);
    }

    #[tokio::test]
    async fn heal_serve_too_large_ok() {
        let (server, mut replies, request, _view) = server(1).await;
        server.handle_request(request, peer());
        let (_, response) = replies.recv().await.unwrap();
        assert_eq!(*response.body(), ShareResponseBody::TooLarge);
    }

    #[tokio::test]
    async fn heal_serve_share_and_not_held_ok() {
        let (server, mut replies, request, _view) = server(usize::MAX).await;
        server.handle_request(request.clone(), peer());
        let (_, response) = replies.recv().await.unwrap();
        assert!(matches!(response.body(), ShareResponseBody::Share(_)));

        let wrong = ShareFetchRequest::new(
            request.view_number(),
            ShareRequestBody {
                commitment: Default::default(),
            },
        );
        server.handle_request(wrong, peer());
        let (_, response) = replies.recv().await.unwrap();
        assert_eq!(*response.body(), ShareResponseBody::NotHeld);
    }
}
