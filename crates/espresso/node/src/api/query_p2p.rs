//! Query service backfill from staked peers over the cliquenet query lane.
//!
//! A node with `--query-p2p` serves leaves and payload proofs to its staked peers, and fetches the
//! ranges its proactive scanner is missing from them before asking its HTTP peers. Query traffic
//! has its own cliquenet connection to each peer, so it never queues ahead of consensus messages.

mod client;
mod message;
mod provider;
mod server;

pub(crate) use client::PeerClient;
use espresso_api::v1::{self, HotShotAvailabilityApi};
use espresso_types::SeqTypes;
use hotshot_new_protocol::network::{QueryReceiver, QuerySender};
use hotshot_query_service::availability::LeafQueryData;
pub(crate) use provider::PeerProvider;
use tokio::{select, task::JoinSet};
use tracing::{debug, info};

use self::{message::QueryMessage, server::QueryServer};
use super::ApiState;

/// Both ends of a node's query lane.
pub(crate) type QueryLane = (QuerySender<SeqTypes>, QueryReceiver<SeqTypes>);

/// Serve peers' requests and route their replies to `client`, until the network shuts down.
///
/// Requests are answered from `api`, the handlers behind the HTTP API.
pub(in crate::api) async fn run<A>(state: ApiState, api: A, client: PeerClient)
where
    A: v1::LightClientApi<
            LeafProof = ::light_client::consensus::leaf::LeafProof,
            PayloadProof = ::light_client::consensus::payload::PayloadProof,
        > + HotShotAvailabilityApi<Leaf = LeafQueryData<SeqTypes>>
        + Clone
        + Send
        + Sync
        + 'static,
{
    let Some((sender, mut receiver)) = state.context().await.take_query_lane() else {
        info!("node is not in consensus, query backfill from peers is off");
        return;
    };
    client.connect(sender.clone());
    let server = QueryServer::new(api, sender);
    let mut requests = JoinSet::new();
    loop {
        select! {
            message = receiver.receive() => {
                let Some((from, bytes)) = message else {
                    return;
                };
                match QueryMessage::decode(&bytes) {
                    Ok(QueryMessage::Request { id, request }) => {
                        if let Some(answer) = server.admit(from, id, request) {
                            requests.spawn(answer);
                        }
                    },
                    Ok(QueryMessage::Reply { id, reply }) => client.complete(from, id, reply),
                    Err(err) => {
                        debug!(%from, err = %format_args!("{err:#}"), "undecodable query message")
                    },
                }
            },
            Some(_) = requests.join_next() => {},
        }
    }
}
