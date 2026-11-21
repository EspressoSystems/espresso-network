//! An API instance: serves the v2 API, over HTTP and gRPC on one port, from the chain store.
//!
//! It holds no state but the newest heights. It reads only the store, which the storage instances
//! of its deployment fill from consensus: with the blocks, and with the stake tables, discovery
//! records and recent state consensus derives (`chain_api::Tables`). Consensus is reached only to
//! submit transactions. An instance asked to [`repair`] also fills what the store lacks from other
//! API services. A read of a height the store lacks waits a moment for
//! it, and a stream waits at a missing height until the store has it, so a client following the
//! chain sees every height in order.
//!
//! Besides the v2 services of `espresso-api`, it serves services of its own over gRPC only: the
//! chain service, which other API services repair their stores from, and the discovery service.

use std::{net::SocketAddr, sync::Arc};

use catchup_api::proto::catchup_service_server::CatchupServiceServer;
use chain_api::{
    Tables,
    proto::{
        chain_service_server::ChainServiceServer, discovery_service_server::DiscoveryServiceServer,
    },
};
use chain_store::ChainStore;
use espresso_api::{
    proto::{
        availability_service_server::AvailabilityServiceServer,
        node_service_server::NodeServiceServer, status_service_server::StatusServiceServer,
        submit_service_server::SubmitServiceServer,
    },
    rest,
};
use tokio::net::TcpListener;
use tonic::transport::Channel;

mod availability;
pub mod catchup;
mod chain;
pub mod compat;
mod discovery;
mod node;
mod reader;
pub mod repair;
mod request;
mod server;
mod status;
mod submit;
mod tip;

use reader::Reader;

/// The services of the API, over one store and one consensus node.
pub struct ApiService {
    reader: Reader,

    /// The follower of the store's tip, which `serve` runs.
    follower: Option<tip::Follower>,

    /// What the store holds besides blocks.
    tables: Tables,

    /// The channel to the consensus node's API, for submissions.
    consensus: Channel,

    /// What consensus nodes that are behind catch up from, if served.
    catchup: Option<Arc<catchup::Catchup>>,

    /// The v1 paths the state relay server and the prover read, if served.
    compat: Option<Arc<compat::Compat>>,
}

impl ApiService {
    pub fn new(store: ChainStore, consensus_api: SocketAddr) -> Self {
        let (follower, tip) = tip::follower(store.clone());
        Self {
            tables: Tables::new(store.clone()),
            reader: Reader::new(store, tip),
            follower: Some(follower),
            consensus: consensus_api::channel(consensus_api),
            catchup: None,
            compat: None,
        }
    }

    /// Serves the v1 paths of `compat` too.
    pub fn with_compat(self, compat: compat::Compat) -> Self {
        Self {
            compat: Some(Arc::new(compat)),
            ..self
        }
    }

    /// Serves catchup from `catchup` too.
    pub fn with_catchup(self, catchup: catchup::Catchup) -> Self {
        Self {
            catchup: Some(Arc::new(catchup)),
            ..self
        }
    }

    /// Serves the API on `listener`, and follows the store's tip, until either stops.
    pub async fn serve(mut self, listener: TcpListener) -> anyhow::Result<()> {
        let follower = self.follower.take().expect("an API service is served once");
        let catchup = self.catchup.clone();
        let compat = self.compat.clone();
        let service = Arc::new(self);
        let mut rest = rest::availability_service_rest_router(service.clone())
            .merge(espresso_api::payload_data::router(service.clone()))
            .merge(rest::status_service_rest_router(service.clone()))
            .merge(rest::node_service_rest_router(service.clone()))
            .merge(rest::submit_service_rest_router(service.clone()));
        if let Some(compat) = compat {
            rest = rest.merge(compat.router());
        }
        let mut grpc = tonic::service::Routes::builder();
        grpc.add_service(AvailabilityServiceServer::from_arc(service.clone()))
            .add_service(StatusServiceServer::from_arc(service.clone()))
            .add_service(NodeServiceServer::from_arc(service.clone()))
            .add_service(SubmitServiceServer::from_arc(service.clone()))
            .add_service(ChainServiceServer::from_arc(service.clone()))
            .add_service(DiscoveryServiceServer::from_arc(service));
        if let Some(catchup) = catchup {
            grpc.add_service(CatchupServiceServer::from_arc(catchup));
        }
        tokio::select! {
            served = server::serve(listener, rest, grpc) => served,
            () = follower.run() => Err(anyhow::anyhow!("the store's changes ended")),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chain_store::{Item, Parts, mem::MemBackend};
    use committable::Committable;
    use espresso_types::{Leaf2, NodeState, Payload, ValidatedState};
    use hotshot_types::{
        simple_certificate::QuorumCertificate2,
        traits::{BlockPayload, EncodeBytes},
    };
    use serde_json::Value;
    use versions::{NEW_PROTOCOL_VERSION, Upgrade};

    use super::*;

    async fn genesis() -> (Item, Vec<u8>) {
        let instance = NodeState::mock().with_current_version(NEW_PROTOCOL_VERSION);
        let state = ValidatedState {
            chain_config: instance.chain_config.into(),
            ..Default::default()
        };
        let upgrade = Upgrade::trivial(NEW_PROTOCOL_VERSION);
        let item = Item {
            leaf: Leaf2::genesis(&state, &instance, NEW_PROTOCOL_VERSION).await,
            cert1: QuorumCertificate2::genesis(&state, &instance, upgrade).await,
        };
        let payload = Payload::empty().0.encode().to_vec();
        (item, payload)
    }

    /// Serves an API over an empty store, returning the store and the base URL.
    async fn serve() -> (ChainStore, String) {
        let store = ChainStore::new(MemBackend::default());
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        // Nothing here reaches consensus.
        let service = ApiService::new(store.clone(), "127.0.0.1:1".parse().unwrap());
        tokio::spawn(service.serve(listener));
        (store, url)
    }

    async fn get(url: &str) -> (u16, Value) {
        let response = reqwest::get(url).await.unwrap();
        let status = response.status().as_u16();
        (status, response.json().await.unwrap_or(Value::Null))
    }

    #[tokio::test]
    async fn serves_stored_blocks_over_http() {
        let (store, url) = serve().await;
        let (item, payload) = genesis().await;
        store
            .put(
                &item,
                &Parts {
                    payload: Some(payload.into()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let (status, height) = get(&format!("{url}/v2/status/block-height")).await;
        assert_eq!((status, &height["height"]), (200, &Value::from("1")));
        for path in ["leaf", "header", "block", "payload"] {
            let (status, body) = get(&format!("{url}/v2/availability/{path}?height=0")).await;
            assert_eq!(status, 200, "{path}: {body}");
        }
        let hash = item.leaf.block_header().commit().to_string();
        let (status, _) = get(&format!("{url}/v2/availability/header?hash={hash}")).await;
        assert_eq!(status, 200);
    }

    /// A read of a height the store lacks waits a moment for it, then gives up.
    #[tokio::test]
    async fn a_read_waits_briefly_for_a_missing_height() {
        let (store, url) = serve().await;
        let (item, payload) = genesis().await;
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            store
                .put(
                    &item,
                    &Parts {
                        payload: Some(payload.into()),
                        ..Default::default()
                    },
                )
                .await
                .unwrap();
        });
        let (status, _) = get(&format!("{url}/v2/availability/leaf?height=0")).await;
        assert_eq!(status, 200);

        let (status, _) = get(&format!("{url}/v2/availability/leaf?height=1")).await;
        assert_eq!(status, 404);
    }

    /// A streamed block carries its payload by reference only, and the reference's URL serves the
    /// raw bytes it describes, for good.
    #[tokio::test]
    async fn streams_the_payload_by_reference() {
        let (store, url) = serve().await;
        let (item, _) = genesis().await;
        // Any bytes: the store does not check them against the header.
        let payload = vec![7u8; 1000];
        store
            .put(
                &item,
                &Parts {
                    payload: Some(payload.clone().into()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let mut response = reqwest::Client::new()
            .get(format!("{url}/v2/availability/stream/blocks?from=0"))
            .header("Accept", "text/event-stream")
            .send()
            .await
            .unwrap();
        let mut text = String::new();
        while !text.contains("\n\n") {
            let chunk = response.chunk().await.unwrap().unwrap();
            text.push_str(&String::from_utf8_lossy(&chunk));
        }
        let data = text
            .lines()
            .find_map(|line| line.strip_prefix("data:"))
            .unwrap();
        let block: Value = serde_json::from_str(data.trim()).unwrap();
        assert!(block.get("payload").is_none(), "no payload inline: {block}");
        let reference = &block["payloadRef"];

        let fetched = reqwest::get(format!("{url}{}", reference["url"].as_str().unwrap()))
            .await
            .unwrap();
        assert_eq!(fetched.status(), 200);
        assert!(
            fetched.headers()["cache-control"]
                .to_str()
                .unwrap()
                .contains("immutable")
        );
        let bytes = fetched.bytes().await.unwrap();
        assert_eq!(&bytes[..], &payload[..]);
        assert_eq!(
            reference["blake3"].as_str().unwrap(),
            blake3::hash(&bytes).to_hex().as_str()
        );
        assert_eq!(reference["size"], Value::from(bytes.len().to_string()));
    }

    #[tokio::test]
    async fn refuses_a_range_over_the_limit() {
        let (_, url) = serve().await;
        let (status, _) = get(&format!(
            "{url}/v2/availability/block-range?from=0&until=1000"
        ))
        .await;
        assert_eq!(status, 400);
    }
}
