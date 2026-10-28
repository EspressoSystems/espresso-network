use std::{
    collections::HashMap,
    future::Future,
    ops::Range,
    sync::Arc,
    time::{Duration, Instant},
};

use ::light_client::consensus::{leaf::LeafProof, payload::PayloadProof};
use espresso_api::v1::{self, HotShotAvailabilityApi, LeafQuery};
use espresso_types::{PubKey, SeqTypes};
use hotshot_new_protocol::network::QuerySender;
use hotshot_query_service::availability::LeafQueryData;
use parking_lot::Mutex;
use tokio::time::{sleep, timeout};
use tracing::debug;

use super::message::{QueryMessage, QueryReply, QueryRequest};

/// Requests answered at once, across all peers.
const MAX_REQUESTS: usize = 4;

/// Requests answered at once for a single peer, so one peer cannot take every slot.
const MAX_REQUESTS_PER_PEER: usize = 2;

/// How long a request may take before the peer gets `Unavailable`.
const ANSWER_TIMEOUT: Duration = Duration::from_secs(5);

/// A payload page stops growing past this many bytes. One proof may exceed it alone.
const PAGE_BYTES: usize = 4 * 1024 * 1024;

/// A payload page holds at most this many proofs, however small.
const PAGE_HEIGHTS: usize = 100;

/// Replies are paced to this many bytes per second, so serving peers cannot saturate the uplink
/// that consensus shares.
const SEND_RATE: f64 = 25.0 * 1024.0 * 1024.0;

/// Answers query lane requests from this node's query service.
///
/// Requests are served by the same handlers as the HTTP API.
pub(super) struct QueryServer<A> {
    api: A,
    sender: QuerySender<SeqTypes>,
    in_flight: Arc<Mutex<HashMap<PubKey, usize>>>,
    pacer: Arc<Pacer>,
}

impl<A> QueryServer<A>
where
    A: v1::LightClientApi<LeafProof = LeafProof, PayloadProof = PayloadProof>
        + HotShotAvailabilityApi<Leaf = LeafQueryData<SeqTypes>>
        + Clone
        + Send
        + Sync
        + 'static,
{
    pub(super) fn new(api: A, sender: QuerySender<SeqTypes>) -> QueryServer<A> {
        QueryServer {
            api,
            sender,
            in_flight: Default::default(),
            pacer: Arc::new(Pacer::new(SEND_RATE)),
        }
    }

    /// Admit a request, returning the task that answers it.
    ///
    /// A request over the concurrency limits is answered `Unavailable` right away.
    pub(super) fn admit(
        &self,
        from: PubKey,
        id: u64,
        request: QueryRequest,
    ) -> Option<impl Future<Output = ()> + Send + 'static> {
        let Some(slot) = Slot::take(&self.in_flight, from) else {
            debug!(%from, id, "query server busy");
            self.reply(from, id, QueryReply::Unavailable);
            return None;
        };
        let api = self.api.clone();
        let sender = self.sender.clone();
        let pacer = Arc::clone(&self.pacer);
        Some(async move {
            let reply = match timeout(ANSWER_TIMEOUT, answer(&api, request)).await {
                Ok(Ok(reply)) => reply,
                Ok(Err(err)) => {
                    debug!(%from, id, err = %format_args!("{err:#}"), "cannot answer query");
                    QueryReply::Unavailable
                },
                Err(_) => {
                    debug!(%from, id, "query timed out");
                    QueryReply::Unavailable
                },
            };
            drop(slot);
            let bytes = match (QueryMessage::Reply { id, reply }).encode() {
                Ok(bytes) => bytes,
                Err(err) => {
                    debug!(%from, id, err = %format_args!("{err:#}"), "cannot encode query reply");
                    return;
                },
            };
            pacer.wait(bytes.len()).await;
            send(&sender, from, bytes);
        })
    }

    fn reply(&self, to: PubKey, id: u64, reply: QueryReply) {
        match (QueryMessage::Reply { id, reply }).encode() {
            Ok(bytes) => send(&self.sender, to, bytes),
            Err(err) => {
                debug!(%to, id, err = %format_args!("{err:#}"), "cannot encode query reply")
            },
        }
    }
}

async fn answer<A>(api: &A, request: QueryRequest) -> anyhow::Result<QueryReply>
where
    A: v1::LightClientApi<LeafProof = LeafProof, PayloadProof = PayloadProof>
        + HotShotAvailabilityApi<Leaf = LeafQueryData<SeqTypes>>
        + Sync,
{
    match request {
        QueryRequest::LeafProof { height, finalized } => {
            let proof = api
                .get_leaf_proof(LeafQuery::Height(height), finalized)
                .await?;
            Ok(QueryReply::LeafProof(proof))
        },
        QueryRequest::Leaves { ranges } => {
            Ok(QueryReply::Leaves(api.get_leaf_ranges(ranges).await?))
        },
        QueryRequest::PayloadProofs { ranges } => payload_page(api, ranges).await,
    }
}

/// Proofs for the first heights in `ranges` that fit in a page.
async fn payload_page<A>(api: &A, ranges: Vec<Range<u64>>) -> anyhow::Result<QueryReply>
where
    A: v1::LightClientApi<PayloadProof = PayloadProof> + Sync,
{
    let mut proofs = Vec::new();
    let mut bytes = 0;
    for height in ranges.into_iter().flatten() {
        if bytes >= PAGE_BYTES || proofs.len() >= PAGE_HEIGHTS {
            return Ok(QueryReply::PayloadProofs {
                proofs,
                next: Some(height),
            });
        }
        let proof = match api.get_payload_proof(height).await {
            Ok(proof) => proof,
            Err(err) if proofs.is_empty() => return Err(err),
            // Serve what we have, and the requester asks again from `height`.
            Err(_) => {
                return Ok(QueryReply::PayloadProofs {
                    proofs,
                    next: Some(height),
                });
            },
        };
        bytes += bincode::serialized_size(&proof)? as usize;
        proofs.push(proof);
    }
    Ok(QueryReply::PayloadProofs { proofs, next: None })
}

fn send(sender: &QuerySender<SeqTypes>, to: PubKey, bytes: Vec<u8>) {
    if let Err(err) = sender.send(&to, bytes) {
        debug!(%to, err = %format_args!("{err:#}"), "cannot send query reply");
    }
}

/// A request's claim on the server's capacity, released on drop.
struct Slot {
    in_flight: Arc<Mutex<HashMap<PubKey, usize>>>,
    peer: PubKey,
}

impl Slot {
    fn take(in_flight: &Arc<Mutex<HashMap<PubKey, usize>>>, peer: PubKey) -> Option<Slot> {
        let mut counts = in_flight.lock();
        let total = counts.values().sum::<usize>();
        let ours = counts.entry(peer).or_default();
        if total >= MAX_REQUESTS || *ours >= MAX_REQUESTS_PER_PEER {
            return None;
        }
        *ours += 1;
        Some(Slot {
            in_flight: Arc::clone(in_flight),
            peer,
        })
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        let mut counts = self.in_flight.lock();
        if let Some(count) = counts.get_mut(&self.peer) {
            *count -= 1;
            if *count == 0 {
                counts.remove(&self.peer);
            }
        }
    }
}

/// A token bucket that lets every send through but delays it until its bytes are paid for.
struct Pacer {
    bytes_per_sec: f64,
    state: Mutex<PacerState>,
}

struct PacerState {
    /// Bytes available to send now. Negative while sends wait on bytes not yet earned.
    budget: f64,
    updated: Instant,
}

impl Pacer {
    fn new(bytes_per_sec: f64) -> Pacer {
        Pacer {
            bytes_per_sec,
            state: Mutex::new(PacerState {
                budget: bytes_per_sec,
                updated: Instant::now(),
            }),
        }
    }

    async fn wait(&self, bytes: usize) {
        let delay = {
            let mut state = self.state.lock();
            let now = Instant::now();
            let earned = now.duration_since(state.updated).as_secs_f64() * self.bytes_per_sec;
            state.budget = (state.budget + earned).min(self.bytes_per_sec) - bytes as f64;
            state.updated = now;
            Duration::from_secs_f64((-state.budget).max(0.0) / self.bytes_per_sec)
        };
        sleep(delay).await;
    }
}

#[cfg(test)]
mod tests {
    use hotshot_types::traits::signature_key::SignatureKey as _;

    use super::*;

    #[test]
    fn slots_cap_total_and_per_peer() {
        let in_flight = Arc::default();
        let a = PubKey::generated_from_seed_indexed([0; 32], 0).0;
        let b = PubKey::generated_from_seed_indexed([0; 32], 1).0;
        let c = PubKey::generated_from_seed_indexed([0; 32], 2).0;

        let a1 = Slot::take(&in_flight, a).unwrap();
        let _a2 = Slot::take(&in_flight, a).unwrap();
        assert!(Slot::take(&in_flight, a).is_none());

        let _b1 = Slot::take(&in_flight, b).unwrap();
        let _b2 = Slot::take(&in_flight, b).unwrap();
        assert!(Slot::take(&in_flight, c).is_none());

        drop(a1);
        assert!(Slot::take(&in_flight, c).is_some());
    }
}
