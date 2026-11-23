//! The decide log served over the consensus API, end to end.

use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use committable::Committable;
use consensus_api::{
    DecideLogClient,
    proto::{SubscribeRequest, SubscribeResponse, subscribe_response::Event},
    server,
};
use futures::StreamExt;
use hotshot_example_types::node_types::TestTypes;
use hotshot_types::{
    data::{EpochNumber, Leaf2},
    signature_key::BLSPubKey,
    simple_certificate::Certificate1,
    traits::signature_key::SignatureKey,
};
use tokio::time::timeout;
use tonic::transport::Server;

use super::common::utils::TestData;
use crate::{
    ingest::{IngestSource, StakeTables},
    storage::{ActionKind, Storage, StorageOutput},
    store::{ConsensusStore, MemStore},
};

const EPOCH_HEIGHT: u64 = 10;

/// A node that has built no stake tables.
struct NoStakeTables;

#[async_trait::async_trait]
impl StakeTables for NoStakeTables {
    async fn stake_table(&self, _: EpochNumber) -> anyhow::Result<Option<consensus_store::Bytes>> {
        Ok(None)
    }
}

/// Decided entries reach the subscriber, without payloads, and an acknowledged height is gone.
#[tokio::test]
async fn decided_entries_reach_the_subscriber() {
    let data = TestData::new(3).await;
    let [v0, v1, v2] = [&data.views[0], &data.views[1], &data.views[2]];
    let (_, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], 0);
    let store = ConsensusStore::<TestTypes, MemStore>::new(Arc::new(MemStore::new()));
    let (queue, acks) = consensus_api::payloads::PayloadQueue::new();
    let mut storage = Storage::new(
        store.clone(),
        &private_key,
        &store.load().unwrap(),
        queue,
        acks,
    );
    let mut client = serve(IngestSource::new(
        store.clone(),
        EPOCH_HEIGHT,
        Arc::new(NoStakeTables),
    ))
    .await;

    let mut decided = client
        .subscribe(SubscribeRequest {})
        .await
        .unwrap()
        .into_inner();
    assert_eq!(
        next_event(&mut decided).await,
        Some(Event::Start(consensus_api::proto::Start {
            earliest_height: None
        }))
    );

    // v1 decides v0 and v1; v2 decides itself later.
    storage.append_decided(
        &[v1.leaf.clone(), v0.leaf.clone()],
        &v1.cert1,
        Some(&v1.cert2),
        &[None, None],
        &[],
    );
    flush(&mut storage).await;
    storage.append_decided(
        std::slice::from_ref(&v2.leaf),
        &v2.cert1,
        Some(&v2.cert2),
        &[None],
        &[],
    );
    flush(&mut storage).await;

    for v in [v0, v1, v2] {
        let Some(Event::Decided(entry)) = next_event(&mut decided).await else {
            panic!("expected a decided entry");
        };
        assert_eq!(entry.height, v.leaf.height());
        assert_eq!(entry.view, *v.view_number);
        assert_eq!(entry.leaf_commitment, v.leaf.commit().to_string());
        assert_eq!(
            entry.parent_commitment,
            v.leaf.parent_commitment().to_string()
        );
        let body = entry.body.unwrap();
        let leaf: Leaf2<TestTypes> = bincode::deserialize(&body.leaf).unwrap();
        assert_eq!(leaf.commit(), v.leaf.commit());
        let newest = v.view_number != v0.view_number;
        let cert1: Certificate1<TestTypes> = bincode::deserialize(&body.cert1).unwrap();
        assert_eq!(
            cert1.data.leaf_commit,
            v.leaf.commit(),
            "every leaf is certified"
        );
        assert_eq!(
            !body.cert2.is_empty(),
            newest,
            "certificates on the newest leaf only"
        );
    }

    // An acknowledged height is gone, and a new subscriber learns where the log starts.
    // The acknowledgement goes through the writer, which the test drives meanwhile.
    {
        let ack = client.ack(consensus_api::proto::AckRequest {
            ranges: vec![consensus_api::proto::HeightRange {
                start: 0,
                end: v2.leaf.height(),
            }],
        });
        let mut ack = std::pin::pin!(ack);
        loop {
            tokio::select! {
                acked = &mut ack => {
                    acked.unwrap();
                    break;
                },
                output = storage.next() => {
                    output.unwrap();
                },
            }
        }
    }
    let mut late = client
        .subscribe(SubscribeRequest {})
        .await
        .unwrap()
        .into_inner();
    assert_eq!(
        next_event(&mut late).await,
        Some(Event::Start(consensus_api::proto::Start {
            earliest_height: Some(v2.leaf.height())
        }))
    );
}

/// The next event other than progress.
async fn next_event(stream: &mut tonic::Streaming<SubscribeResponse>) -> Option<Event> {
    loop {
        let event = timeout(Duration::from_secs(5), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap()
            .event;
        if !matches!(event, Some(Event::Progress(_))) {
            return event;
        }
    }
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
    let (service, dispatcher) = server(source, consensus_api::payloads::PayloadQueue::new().0);
    tokio::spawn(dispatcher);
    tokio::spawn(Server::builder().add_service(service).serve(addr));
    for _ in 0..100 {
        if let Ok(client) = DecideLogClient::connect(format!("http://{addr}")).await {
            return client;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("server did not start");
}
