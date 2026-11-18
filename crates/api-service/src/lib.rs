//! The API service: serves the v2 API, over HTTP and gRPC on one port, from the chain store.
//!
//! It reads only the store, which the ingester fills from consensus and the [`GapFiller`] from
//! other API services. Consensus is reached only to submit transactions and to ask for the
//! validators' discovery records. A read of a height the
//! store lacks waits a moment for it, and a stream waits at a missing height until the store has
//! it, so a client following the chain sees every height in order.

use std::{net::SocketAddr, sync::Arc};

use chain_store::ChainStore;
use espresso_api::{
    proto::{
        availability_service_server::AvailabilityServiceServer,
        chain_service_server::ChainServiceServer, node_service_server::NodeServiceServer,
        status_service_server::StatusServiceServer, submit_service_server::SubmitServiceServer,
    },
    rest,
};
use tokio::net::TcpListener;

mod availability;
mod chain;
mod gaps;
mod node;
mod reader;
mod request;
mod status;
mod submit;

pub use gaps::GapFiller;
use reader::Reader;

/// The services of the API, over one store and one consensus node.
#[derive(Clone, Debug)]
pub struct ApiService {
    reader: Reader,

    /// The consensus node's API, for submissions and discovery records.
    consensus_api: SocketAddr,
}

impl ApiService {
    pub fn new(store: ChainStore, consensus_api: SocketAddr) -> Self {
        Self {
            reader: Reader::new(store),
            consensus_api,
        }
    }

    /// Serves the API on `listener` until it fails.
    pub async fn serve(self, listener: TcpListener) -> anyhow::Result<()> {
        let service = Arc::new(self);
        let rest = rest::availability_service_rest_router(service.clone())
            .merge(rest::status_service_rest_router(service.clone()))
            .merge(rest::node_service_rest_router(service.clone()))
            .merge(rest::submit_service_rest_router(service.clone()));
        let mut grpc = tonic::service::Routes::builder();
        grpc.add_service(AvailabilityServiceServer::from_arc(service.clone()))
            .add_service(StatusServiceServer::from_arc(service.clone()))
            .add_service(NodeServiceServer::from_arc(service.clone()))
            // gRPC only: it serves other API services, in the store's own encoding.
            .add_service(ChainServiceServer::from_arc(service.clone()))
            .add_service(SubmitServiceServer::from_arc(service));
        espresso_api::serve_v2(listener, rest, grpc, None).await
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chain_store::{Item, mem::MemBackend};
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
            cert2: None,
            state_cert: None,
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
        store.put(&item, Some(payload.into()), None).await.unwrap();

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
            store.put(&item, Some(payload.into()), None).await.unwrap();
        });
        let (status, _) = get(&format!("{url}/v2/availability/leaf?height=0")).await;
        assert_eq!(status, 200);

        let (status, _) = get(&format!("{url}/v2/availability/leaf?height=1")).await;
        assert_eq!(status, 404);
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
