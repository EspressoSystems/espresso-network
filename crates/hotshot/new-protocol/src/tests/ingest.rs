//! The decide log served over the ingester API, end to end.

use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use committable::Committable;
use consensus_api::{
    DecideLogClient,
    proto::{
        GetPayloadRequest, PayloadsReadyRequest, SubscribeRequest, get_payload_request::Key,
        subscribe_response::Event,
    },
    server,
};
use futures::StreamExt;
use hotshot_example_types::node_types::TestTypes;
use hotshot_types::{
    data::{EpochNumber, Leaf2, VidCommitment},
    signature_key::BLSPubKey,
    traits::{BlockPayload, EncodeBytes, signature_key::SignatureKey},
};
use tokio::time::timeout;
use tonic::transport::Server;

use super::common::utils::{TestData, TestView};
use crate::{
    ingest::IngestSource,
    storage::{ActionKind, Storage, StorageOutput},
    store::{ConsensusStore, MemStore},
};

const EPOCH_HEIGHT: u64 = 10;

#[tokio::test]
async fn decided_entries_and_payloads_reach_the_ingester() {
    let data = TestData::new(3).await;
    let [v0, v1, v2] = [&data.views[0], &data.views[1], &data.views[2]];
    let (_, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], 0);
    let store = ConsensusStore::<TestTypes, MemStore>::new(Arc::new(MemStore::new()));
    let mut storage = Storage::new(store.clone(), &private_key);
    let mut client = serve(IngestSource::new(store.clone(), EPOCH_HEIGHT)).await;

    let mut ready = client
        .payloads_ready(PayloadsReadyRequest {})
        .await
        .unwrap()
        .into_inner();
    let mut decided = client
        .subscribe(SubscribeRequest { from_height: 0 })
        .await
        .unwrap()
        .into_inner();

    // Payloads are announced when they are durable, before their block is decided.
    for v in [v0, v1, v2] {
        store_payload(&mut storage, v);
    }
    flush(&mut storage).await;
    for v in [v0, v1, v2] {
        let announced = timeout(Duration::from_secs(5), ready.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(announced.view, *v.view_number);
        assert_eq!(
            announced.payload_commitment,
            v.leaf.payload_commitment().to_string()
        );
    }

    // v1 decides v0 and v1; v2 decides itself later.
    storage.append_decided(
        &[v1.leaf.clone(), v0.leaf.clone()],
        &v1.cert1,
        Some(&v1.cert2),
        &[None, None],
    );
    flush(&mut storage).await;
    storage.append_decided(
        std::slice::from_ref(&v2.leaf),
        &v2.cert1,
        Some(&v2.cert2),
        &[None],
    );
    flush(&mut storage).await;

    for v in [v0, v1, v2] {
        let Some(Event::Decided(entry)) = timeout(Duration::from_secs(5), decided.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap()
            .event
        else {
            panic!("expected a decided entry");
        };
        assert_eq!(entry.height, v.leaf.height());
        assert_eq!(entry.view, *v.view_number);
        assert_eq!(entry.leaf_commitment, v.leaf.commit().to_string());
        assert_eq!(
            entry.parent_commitment,
            v.leaf.parent_commitment().to_string()
        );
        let payload = v.payload.encode();
        assert_eq!(entry.payload_size, Some(payload.len() as u64));

        let body = entry.body.unwrap();
        let leaf: Leaf2<TestTypes> = bincode::deserialize(&body.leaf).unwrap();
        assert_eq!(leaf.commit(), v.leaf.commit());
        let newest = v.view_number != v0.view_number;
        assert_eq!(
            !body.cert2.is_empty(),
            newest,
            "certificates on the newest leaf only"
        );

        let chunks: Vec<_> = client
            .get_payload(GetPayloadRequest {
                key: Some(Key::Height(entry.height)),
            })
            .await
            .unwrap()
            .into_inner()
            .collect()
            .await;
        let bytes: Vec<u8> = chunks.into_iter().flat_map(|c| c.unwrap().data).collect();
        assert_eq!(bytes, payload.to_vec());
    }

    // An acknowledged height is gone, and a new subscriber learns where the log starts.
    client
        .ack(consensus_api::proto::AckRequest {
            height: v2.leaf.height(),
        })
        .await
        .unwrap();
    let mut late = client
        .subscribe(SubscribeRequest { from_height: 0 })
        .await
        .unwrap()
        .into_inner();
    let first = timeout(Duration::from_secs(5), late.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap()
        .event;
    assert_eq!(
        first,
        Some(Event::Start(consensus_api::proto::Start {
            earliest_height: v2.leaf.height()
        }))
    );
}

fn store_payload(storage: &mut Storage<TestTypes, MemStore>, v: &TestView) {
    let (_, metadata) =
        <hotshot_example_types::block_types::TestBlockPayload as BlockPayload<TestTypes>>::empty();
    let VidCommitment::V2(commitment) = v.leaf.payload_commitment() else {
        panic!("test blocks have V2 commitments");
    };
    storage.append_da(
        v.view_number,
        EpochNumber::genesis(),
        v.payload.clone(),
        metadata,
        VidCommitment::V2(commitment),
    );
}

/// Applies everything queued, by queueing one more write and waiting for it.
async fn flush(storage: &mut Storage<TestTypes, MemStore>) {
    let view = hotshot_types::data::ViewNumber::new(u64::MAX - 1);
    storage.record_action(view, ActionKind::Propose);
    while storage.next().await.unwrap() != StorageOutput::Action(view, ActionKind::Propose) {}
}

async fn serve(
    source: Arc<IngestSource<TestTypes, MemStore>>,
) -> DecideLogClient<tonic::transport::Channel> {
    let port = test_utils::reserve_tcp_port().unwrap();
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    tokio::spawn(Server::builder().add_service(server(source)).serve(addr));
    for _ in 0..100 {
        if let Ok(client) = DecideLogClient::connect(format!("http://{addr}")).await {
            return client;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("server did not start");
}
