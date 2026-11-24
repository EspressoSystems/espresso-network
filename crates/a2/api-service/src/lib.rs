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
use membership::MembershipStore;
use tokio::net::TcpListener;
use tonic::transport::Channel;

mod availability;
pub mod catchup;
mod chain;
pub mod compat;
mod discovery;
mod node;
mod payload_route;
mod reader;
pub mod repair;
mod request;
mod server;
mod status;
mod submit;

use reader::Reader;

/// The services of the API, over one store and one consensus node.
pub struct ApiService {
    reader: Reader,

    /// The follower of the store's tip, which `serve` runs.
    follower: Option<chain_store::tip::Follower>,

    /// What the store holds besides blocks.
    tables: Tables,

    /// The validators and rewards of recent epochs, which repair keeps in the store.
    membership: MembershipStore<ChainStore>,

    /// How many blocks an epoch has, from the chain's genesis file.
    epoch_height: u64,

    /// The channel to the consensus node's API, for submissions.
    consensus: Channel,

    /// What consensus nodes that are behind catch up from, if served.
    catchup: Option<Arc<catchup::Catchup>>,

    /// The v1 paths the state relay server and the prover read, if served.
    compat: Option<Arc<compat::Compat>>,
}

impl ApiService {
    /// The API over `store`, for a chain whose epochs have `epoch_height` blocks, submitting
    /// transactions to the consensus API at `consensus_api`.
    pub fn new(store: ChainStore, consensus_api: SocketAddr, epoch_height: u64) -> Self {
        let (follower, tip) = chain_store::tip::follower(store.clone());
        Self {
            tables: Tables::new(store.clone()),
            membership: MembershipStore::new(store.clone()),
            epoch_height,
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
            .merge(payload_route::router(service.clone()))
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
        serve_store(ChainStore::new(MemBackend::default())).await
    }

    /// Serves an API over `store`, returning the store and the base URL.
    async fn serve_store(store: ChainStore) -> (ChainStore, String) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        // Nothing here reaches consensus.
        let service = ApiService::new(store.clone(), "127.0.0.1:1".parse().unwrap(), 10);
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

    /// A streamed block comes without its payload, whose raw bytes the payload route serves under
    /// the payload commitment, for good, matching the block's BLAKE3 hash and size.
    #[tokio::test]
    async fn streams_blocks_without_their_payloads() {
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
        let commitment = item.leaf.block_header().payload_commitment().to_string();

        let fetched = reqwest::get(format!(
            "{url}{}",
            espresso_api::payload_data::path(&commitment)
        ))
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
            block["payloadBlake3"].as_str().unwrap(),
            blake3::hash(&bytes).to_hex().as_str()
        );
        assert_eq!(block["size"], Value::from(bytes.len().to_string()));
    }

    /// With payloads in an object store that clients reach, the payload route redirects to the
    /// payload's object, whose URL follows from its commitment; without a URL for clients, the
    /// API instance serves the payload from the object store itself.
    #[tokio::test]
    async fn the_payload_route_redirects_to_the_object_store() {
        use chain_store::payloads::PayloadStore;

        let objects = std::sync::Arc::new(object_store::memory::InMemory::new());
        let reachable = PayloadStore::new(objects.clone(), "p", Some("http://objects.test".into()));
        let (store, url) =
            serve_store(ChainStore::new(MemBackend::default()).with_payloads(reachable)).await;
        let (item, _) = genesis().await;
        let payload = vec![9u8; 100];
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
        let commitment = item.leaf.block_header().payload_commitment().to_string();
        let route = format!("{url}{}", espresso_api::payload_data::path(&commitment));

        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        let response = client.get(&route).send().await.unwrap();
        assert_eq!(response.status(), 307);
        assert_eq!(
            response.headers()["location"],
            format!("http://objects.test/p/{}", commitment.replace('~', ".")).as_str()
        );

        let hidden = PayloadStore::new(objects, "p", None);
        let (store, url) =
            serve_store(ChainStore::new(MemBackend::default()).with_payloads(hidden)).await;
        let with_payload = Parts {
            payload: Some(payload.clone().into()),
            ..Default::default()
        };
        store.put(&item, &with_payload).await.unwrap();
        let served = reqwest::get(format!(
            "{url}{}",
            espresso_api::payload_data::path(&commitment)
        ))
        .await
        .unwrap();
        assert_eq!(served.status(), 200);
        assert_eq!(&served.bytes().await.unwrap()[..], &payload[..]);
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
    /// Transactions of namespaces 7 (three), 9 (one) and 11 (two), in that order.
    fn transactions() -> Vec<espresso_types::Transaction> {
        [(7u32, 3), (9, 1), (11, 2)]
            .into_iter()
            .flat_map(|(namespace, count)| {
                (0..count).map(move |i| {
                    espresso_types::Transaction::new(namespace.into(), vec![i as u8; 10 + i])
                })
            })
            .collect()
    }

    /// The block at `height` with timestamp `timestamp`, holding `transactions`, with its payload
    /// and VID common data as the store keeps them.
    async fn block_at(
        height: u64,
        timestamp: u64,
        transactions: &[espresso_types::Transaction],
    ) -> (Item, Parts) {
        use hotshot_types::{
            data::{VidCommon, ns_table::parse_ns_table},
            vid::avidm_gf2::{AvidmGf2Param, AvidmGf2Scheme},
        };

        let (mut item, _) = genesis().await;
        let chain_config = NodeState::mock().chain_config;
        let (payload, ns_table) =
            Payload::from_transactions_sync(transactions, chain_config).unwrap();
        let bytes = payload.encode().to_vec();
        let ranges = parse_ns_table(bytes.len(), &ns_table.encode());
        let param = AvidmGf2Param::new(5, 10).unwrap();
        let (_, common) = AvidmGf2Scheme::commit(&param, &bytes, ranges).unwrap();
        let header = item.leaf.block_header_mut();
        *header.ns_table_mut() = ns_table;
        *header.height_mut() = height;
        header.set_timestamp(timestamp, timestamp * 1000);
        let parts = Parts {
            payload: Some(bytes.into()),
            vid_common: Some(VidCommon::V2(common)),
            ..Default::default()
        };
        (item, parts)
    }

    /// Reads `count` events of the server-sent event stream at `url`.
    async fn events(url: &str, count: usize) -> Vec<Value> {
        let mut response = reqwest::Client::new()
            .get(url)
            .header("Accept", "text/event-stream")
            .send()
            .await
            .unwrap();
        let mut text = String::new();
        loop {
            let events: Vec<Value> = text
                .lines()
                .filter_map(|line| line.strip_prefix("data:"))
                .map(|data| serde_json::from_str(data.trim()).unwrap())
                .collect();
            if events.len() >= count {
                return events;
            }
            let chunk = response.chunk().await.unwrap().unwrap();
            text.push_str(&String::from_utf8_lossy(&chunk));
        }
    }

    /// Transactions are served from the stored payload by their position or hash, with their
    /// proofs, in block summaries and in the transaction stream, and counted, with their bytes,
    /// from the index the store keeps: per namespace as the block summary counts them.
    #[tokio::test]
    async fn serves_transactions_from_stored_blocks() {
        let (store, url) = serve().await;
        let (item, parts) = block_at(0, 100, &transactions()).await;
        store.put(&item, &parts).await.unwrap();
        let availability = format!("{url}/v2/availability");

        let (status, body) = get(&format!("{availability}/transaction?height=0&index=3")).await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["namespace"], Value::from("9"));
        assert_eq!(body["index"], Value::from("3"));
        let (status, _) = get(&format!("{availability}/transaction?height=0&index=6")).await;
        assert_eq!(status, 404);
        let hash = transactions()[3].commit().to_string();
        let (status, body) = get(&format!("{availability}/transaction?hash={hash}")).await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["index"], Value::from("3"));
        let elsewhere = espresso_types::Transaction::new(7u32.into(), vec![1, 2, 3]);
        let (status, _) = get(&format!(
            "{availability}/transaction?hash={}",
            elsewhere.commit()
        ))
        .await;
        assert_eq!(status, 404);
        let (status, body) = get(&format!("{availability}/transaction-proof?hash={hash}")).await;
        assert_eq!(status, 200, "{body}");

        let (status, body) = get(&format!(
            "{availability}/transaction-proof?height=0&index=1"
        ))
        .await;
        assert_eq!(status, 200, "{body}");

        let (status, summary) = get(&format!("{availability}/block-summary?height=0")).await;
        assert_eq!(status, 200, "{summary}");
        assert_eq!(summary["numTransactions"], Value::from("6"));
        let node = format!("{url}/v2/node");
        let (_, count) = get(&format!("{node}/transaction-count")).await;
        assert_eq!(count["count"], Value::from("6"));
        let (_, size) = get(&format!("{node}/payload-size?from=0&to=0")).await;
        assert_eq!(size["size"], summary["size"]);
        for namespace in ["7", "9", "11"] {
            let info = &summary["namespaces"][namespace];
            let query = format!("namespace={namespace}");
            let (_, count) = get(&format!("{node}/transaction-count?{query}")).await;
            assert_eq!(count["count"], info["numTransactions"], "{namespace}");
            let (_, size) = get(&format!("{node}/payload-size?{query}")).await;
            assert_eq!(size["size"], info["size"], "{namespace}");
        }
        // protoJSON leaves out a count of zero.
        let (status, count) = get(&format!("{node}/transaction-count?from=1")).await;
        assert_eq!(status, 200);
        assert!(count.get("count").is_none(), "{count}");
        let (status, body) = get(&format!(
            "{availability}/block-summary-range?from=0&until=1"
        ))
        .await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["summaries"].as_array().unwrap().len(), 1);

        let streamed = events(
            &format!("{availability}/stream/transactions?from=0&namespace=11"),
            2,
        )
        .await;
        let indexes: Vec<_> = streamed.iter().map(|tx| tx["index"].clone()).collect();
        assert_eq!(indexes, [Value::from("4"), Value::from("5")]);
    }

    /// The node service answers from the store: which heights it lacks, header windows, stake
    /// tables, validators and block rewards.
    #[tokio::test]
    async fn serves_the_node_service_from_the_store() {
        use alloy::primitives::{Address, U256};
        use espresso_types::{
            AuthenticatedValidatorMap, PubKey, RegisteredValidatorMap, SeqTypes,
            traits::MembershipPersistence,
            v0_3::{RegisteredValidator, RewardAmount},
        };
        use hotshot_types::{
            PeerConfig, data::EpochNumber, light_client::StateKeyPair,
            traits::signature_key::SignatureKey,
        };

        let (store, url) = serve().await;
        // Heights 0 to 2 in full, none at 3, and 4 without its payload and VID common data.
        for (height, timestamp) in [(0, 100), (1, 110), (2, 120)] {
            let (item, parts) = block_at(height, timestamp, &transactions()).await;
            store.put(&item, &parts).await.unwrap();
        }
        let (item, _) = block_at(4, 140, &transactions()).await;
        store.put(&item, &Parts::default()).await.unwrap();
        let node = format!("{url}/v2/node");

        let (status, body) = get(&format!("{node}/sync-status")).await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["leaves"]["missing"], Value::from("1"));
        assert_eq!(body["blocks"]["missing"], Value::from("2"));
        assert_eq!(body["vidCommon"]["missing"], Value::from("2"));

        let (status, body) = get(&format!("{node}/header-window?start_height=0&end=115")).await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["window"].as_array().unwrap().len(), 2);
        assert!(body.get("prev").is_none(), "{body}");
        assert!(body.get("next").is_some(), "{body}");
        let (status, body) = get(&format!("{node}/header-window?start_time=105&end=118")).await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["window"].as_array().unwrap().len(), 1);
        assert!(body.get("prev").is_some(), "{body}");
        assert!(body.get("next").is_some(), "{body}");

        let (status, body) = get(&format!("{node}/limits")).await;
        assert_eq!((status, &body["windowLimit"]), (200, &Value::from("500")));

        let committee: Vec<PeerConfig<SeqTypes>> = (0..4)
            .map(|i| PeerConfig {
                stake_table_entry: PubKey::generated_from_seed_indexed([0; 32], i)
                    .0
                    .stake_table_entry(U256::from(100)),
                state_ver_key: StateKeyPair::generate_from_seed_indexed([0; 32], i).ver_key(),
                connect_info: None,
            })
            .collect();
        let epoch = EpochNumber::new(1);
        Tables::new(store.clone())
            .put_stake_table(epoch, &committee)
            .await
            .unwrap();
        for query in ["?epoch=1", ""] {
            let (status, body) = get(&format!("{node}/stake-table{query}")).await;
            assert_eq!(status, 200, "{query}: {body}");
            assert_eq!(body["epoch"], Value::from("1"), "{query}");
            assert_eq!(body["stakeTable"].as_array().unwrap().len(), 4);
        }

        let validator = RegisteredValidator::<PubKey>::mock();
        let account: Address = validator.account;
        let eligible: AuthenticatedValidatorMap =
            [(account, validator.clone().try_into().unwrap())]
                .into_iter()
                .collect();
        let registered: RegisteredValidatorMap = [(account, validator)].into_iter().collect();
        let membership = MembershipStore::new(store.clone());
        membership
            .store_stake(epoch, eligible, Some(RewardAmount(U256::from(5))), None)
            .await
            .unwrap();
        membership
            .store_all_validators(epoch, registered)
            .await
            .unwrap();
        let (status, body) = get(&format!("{node}/validators?epoch=1")).await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["validators"].as_array().unwrap().len(), 1);
        let (status, _) = get(&format!("{node}/validators?epoch=2")).await;
        assert_eq!(status, 404);
        let (status, body) = get(&format!("{node}/all-validators?epoch=1&offset=0&limit=10")).await;
        assert_eq!(status, 200, "{body}");
        assert_eq!(body["validators"].as_array().unwrap().len(), 1);
        let (status, _) = get(&format!(
            "{node}/all-validators?epoch=1&offset=0&limit=1001"
        ))
        .await;
        assert_eq!(status, 400);
        let (status, body) = get(&format!("{node}/block-reward?epoch=1")).await;
        assert_eq!((status, &body["amount"]), (200, &Value::from("5")));
        let (status, body) = get(&format!("{node}/block-reward?epoch=2")).await;
        assert_eq!(status, 200);
        assert!(body.get("amount").is_none(), "{body}");
    }

    /// The state certificate of an epoch is served from the store's part at the epoch's root, in
    /// both versions.
    #[tokio::test]
    async fn serves_state_certificates_by_epoch() {
        use hotshot_types::{
            data::EpochNumber, light_client::StateVerKey,
            simple_certificate::LightClientStateUpdateCertificateV2, utils::root_block_in_epoch,
        };

        let (store, url) = serve().await;
        let epoch = 2;
        let cert = LightClientStateUpdateCertificateV2::<espresso_types::SeqTypes> {
            epoch: EpochNumber::new(epoch),
            light_client_state: Default::default(),
            next_stake_table_state: Default::default(),
            signatures: Vec::<(StateVerKey, _, _)>::new(),
            auth_root: Default::default(),
        };
        // The service is built with 10 blocks an epoch.
        let parts = Parts {
            state_cert: Some(cert),
            ..Default::default()
        };
        store
            .put_parts(root_block_in_epoch(epoch, 10), &parts)
            .await
            .unwrap();

        for path in ["state-cert", "state-cert-v2"] {
            let (status, body) = get(&format!("{url}/v2/availability/{path}?epoch={epoch}")).await;
            assert_eq!(status, 200, "{path}: {body}");
            let (status, _) = get(&format!("{url}/v2/availability/{path}?epoch=3")).await;
            assert_eq!(status, 404, "{path}");
        }
    }
}
