//! A client of the store service gets and puts as the store it serves.

use std::{sync::Arc, time::Duration};

use bytes::Bytes;
use chain_api::Tables;
use chain_store::{
    ChainStore, Item, Lookup, Part, Parts, TipUpdate, mem::MemBackend, payloads::PayloadStore,
};
use committable::Committable;
use espresso_types::{
    Leaf2, NamespaceId, NodeState, Payload, PubKey, SeqTypes, Transaction, ValidatedState,
    v0_3::ChainConfig,
};
use futures::StreamExt;
use hotshot_types::{
    PeerConfig,
    data::EpochNumber,
    drb::DrbInput,
    light_client::{StateKeyPair, StateVerKey},
    simple_certificate::{LightClientStateUpdateCertificateV2, QuorumCertificate2},
    traits::{EncodeBytes, signature_key::SignatureKey},
};
use membership::EpochMembership;
use tokio::net::TcpListener;
use tonic::transport::{Server, server::TcpIncoming};
use versions::{NEW_PROTOCOL_VERSION, Upgrade};

use crate::{INLINE_LIMIT, LocalStore, PayloadAt, PayloadData, RemoteStore, Store, server::server};

/// The genesis block, with a payload of one transaction of `size` bytes, and that payload.
async fn genesis(size: usize) -> (Item, Bytes) {
    let instance = NodeState::mock().with_current_version(NEW_PROTOCOL_VERSION);
    let state = ValidatedState {
        chain_config: instance.chain_config.into(),
        ..Default::default()
    };
    let chain_config = ChainConfig {
        max_block_size: (1u64 << 20).into(),
        ..instance.chain_config
    };
    let transaction = Transaction::new(NamespaceId::from(7u32), vec![1; size]);
    let (payload, ns_table) =
        Payload::from_transactions_sync(&[transaction], chain_config).unwrap();
    let mut leaf = Leaf2::genesis(&state, &instance, NEW_PROTOCOL_VERSION).await;
    *leaf.block_header_mut().ns_table_mut() = ns_table;
    let cert1 =
        QuorumCertificate2::genesis(&state, &instance, Upgrade::trivial(NEW_PROTOCOL_VERSION))
            .await;
    (Item { leaf, cert1 }, payload.encode().to_vec().into())
}

fn committee() -> Vec<PeerConfig<SeqTypes>> {
    (0..2)
        .map(|i| PeerConfig {
            stake_table_entry: PubKey::generated_from_seed_indexed([0; 32], i)
                .0
                .stake_table_entry(alloy::primitives::U256::from(100)),
            state_ver_key: StateKeyPair::generate_from_seed_indexed([0; 32], i).ver_key(),
            connect_info: None,
        })
        .collect()
}

fn state_cert() -> LightClientStateUpdateCertificateV2<SeqTypes> {
    LightClientStateUpdateCertificateV2 {
        epoch: EpochNumber::new(2),
        light_client_state: Default::default(),
        next_stake_table_state: Default::default(),
        signatures: Vec::<(StateVerKey, _, _)>::new(),
        auth_root: Default::default(),
    }
}

fn objects(public_url: Option<&str>) -> PayloadStore {
    PayloadStore::new(
        Arc::new(object_store::memory::InMemory::new()),
        "p",
        public_url.map(Into::into),
    )
}

/// Serves `store` and returns a client of it.
async fn client(store: ChainStore) -> RemoteStore {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(
        Server::builder()
            .add_service(server(LocalStore::spawn(store)))
            .serve_with_incoming(TcpIncoming::from(listener)),
    );
    RemoteStore::connect(&url).await.unwrap()
}

/// What a client puts, it and the store read back: a block with its payload, whose info and index
/// the store computes, a certificate of a height without an item, the state and membership
/// entities, and a lease.
#[tokio::test]
async fn a_client_gets_what_it_puts() {
    let store = ChainStore::new(MemBackend::default()).with_payloads(objects(None));
    let remote = client(store.clone()).await;
    let (item, payload) = genesis(10).await;
    let parts = Parts {
        payload: Some(payload.clone()),
        ..Default::default()
    };
    remote.put_block(&item, &parts).await.unwrap();
    let root = 5;
    let cert = state_cert();
    let cert_parts = Parts {
        state_cert: Some(cert.clone()),
        ..Default::default()
    };
    remote.put_parts(root, &cert_parts).await.unwrap();

    assert_eq!(remote.newest().await.unwrap(), Some(0));
    assert_eq!(remote.floor().await.unwrap(), 0);
    assert_eq!(remote.item(0).await.unwrap(), Some(item.clone()));
    assert_eq!(remote.item(1).await.unwrap(), None);
    let summary = remote.record(0, false).await.unwrap().unwrap();
    assert_eq!(summary.item().unwrap(), item);
    let summarized = summary.parts().unwrap();
    assert_eq!(summarized.payload, None);
    assert_eq!(
        summarized.payload_info.map(|info| info.size),
        Some(payload.len() as u64),
        "the store describes the payload"
    );
    let record = remote.record(0, true).await.unwrap().unwrap();
    assert_eq!(record.parts().unwrap().payload, Some(payload.clone()));
    for lookup in [
        Lookup::Leaf(item.leaf.commit()),
        Lookup::Block(item.leaf.block_header().commit()),
        Lookup::Payload(item.leaf.payload_commitment()),
    ] {
        assert_eq!(remote.locate(&lookup).await.unwrap(), Some(0), "{lookup:?}");
    }
    assert_eq!(remote.totals(0..=0, None).await.unwrap().transactions, 1);

    let commitment = item.leaf.payload_commitment().to_string();
    for at in [
        PayloadAt::Height(0),
        PayloadAt::Commitment(commitment.clone()),
    ] {
        assert_eq!(
            remote.payload(&at, None).await.unwrap(),
            Some(PayloadData::Inline(payload.clone())),
            "{at:?}"
        );
        assert_eq!(
            remote.payload(&at, Some(2..6)).await.unwrap(),
            Some(PayloadData::Inline(payload.slice(2..6))),
            "{at:?}"
        );
    }
    assert_eq!(
        remote.payload(&PayloadAt::Height(1), None).await.unwrap(),
        None
    );
    let status = remote.payload_status(&commitment).await.unwrap();
    assert!(status.has && status.held);
    let status = remote.payload_status("C~none").await.unwrap();
    assert!(!status.has && !status.held);

    assert_eq!(remote.state_cert(root).await.unwrap(), Some(cert));
    assert_eq!(remote.cert2(0).await.unwrap(), None);
    assert_eq!(
        remote.missing_parts(Part::Cert2, 0, 10).await.unwrap(),
        vec![0]
    );
    assert!(
        remote
            .missing_parts(Part::Payload, 0, 10)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(remote.missing_items(0, 10).await.unwrap().is_empty());
    assert_eq!(remote.newest_gap().await.unwrap(), None);

    // Tables the storage instances write, read back.
    let epoch = EpochNumber::new(2);
    Tables::new(store.clone())
        .put_stake_table(epoch, &committee())
        .await
        .unwrap();
    assert_eq!(remote.stake_table(epoch).await.unwrap(), Some(committee()));
    assert_eq!(remote.stake_table(epoch + 1).await.unwrap(), None);
    assert_eq!(remote.service_records(None).await.unwrap(), None);

    // The state entities repair writes.
    let config = ChainConfig::default();
    remote.put_fee_accounts(&[]).await.unwrap();
    assert_eq!(remote.fee_accounts().await.unwrap(), Some(Vec::new()));
    remote.put_chain_config(&config).await.unwrap();
    let key = config.commit().to_string();
    assert_eq!(remote.chain_config(&key).await.unwrap(), Some(config));
    assert!(
        remote
            .put_state(4, None, config, Some(Vec::new()))
            .await
            .unwrap()
    );
    assert!(remote.put_state(5, None, config, None).await.unwrap());
    let row = remote.state(5).await.unwrap().unwrap();
    assert_eq!(row.rewards_at, 4);
    assert_eq!(remote.rewards(4).await.unwrap(), Some(Vec::new()));
    assert_eq!(
        remote
            .states()
            .await
            .unwrap()
            .iter()
            .map(|(height, _)| *height)
            .collect::<Vec<_>>(),
        vec![4, 5]
    );
    assert_eq!(remote.sent_state(5).await.unwrap(), None);

    // The membership entities.
    assert_eq!(
        remote.epoch(epoch).await.unwrap(),
        EpochMembership::default()
    );
    let update = EpochMembership {
        stake: Some(Default::default()),
        drb_result: Some([3; 32]),
        ..Default::default()
    };
    remote.put_epoch(epoch, update).await.unwrap();
    let root_update = EpochMembership {
        root: Some(item.leaf.block_header().clone()),
        ..Default::default()
    };
    remote.put_epoch(epoch, root_update).await.unwrap();
    let kept = remote.epoch(epoch).await.unwrap();
    assert_eq!(
        kept.drb_result,
        Some([3; 32]),
        "a later update keeps the parts"
    );
    assert!(kept.stake.is_some() && kept.root.is_some() && kept.validators.is_none());
    let epochs = remote.epochs(10).await.unwrap();
    assert_eq!(
        epochs.iter().map(|(e, _)| *e).collect::<Vec<_>>(),
        vec![epoch]
    );
    let stakes = remote.latest_stakes(10).await.unwrap();
    assert_eq!(
        stakes.iter().map(|(e, _)| *e).collect::<Vec<_>>(),
        vec![epoch]
    );
    assert_eq!(remote.events(0, 10).await.unwrap().until, None);
    remote.put_events(10, Vec::new()).await.unwrap();
    assert_eq!(remote.events(0, 20).await.unwrap().until, Some(10));
    remote.clear_stake_tables().await.unwrap();
    assert_eq!(remote.events(0, 20).await.unwrap().until, None);
    let input = |iteration| DrbInput {
        epoch: 3,
        iteration,
        value: [0; 32],
        difficulty_level: 10,
    };
    remote.put_drb_input(input(5)).await.unwrap();
    remote
        .put_drb_input(input(4))
        .await
        .expect_err("older DRB progress");
    assert_eq!(
        remote.drb_input(3).await.unwrap().map(|i| i.iteration),
        Some(5)
    );

    // A lease.
    let ttl = Duration::from_secs(60);
    assert!(remote.lease("work", "a", ttl).await.unwrap());
    assert!(!remote.lease("work", "b", ttl).await.unwrap());

    // The tip.
    let mut updates = remote.tip_updates(0).await.unwrap();
    let mut heights = Vec::new();
    loop {
        match updates.next().await.unwrap().unwrap() {
            TipUpdate::Height { height, parts, .. } => {
                assert!(parts.iter().all(|(part, _)| *part != Part::Payload));
                heights.push(height);
            },
            TipUpdate::Bounds { newest, floor } => {
                assert_eq!((newest, floor), (Some(0), 0));
                break;
            },
        }
    }
    assert_eq!(heights, vec![0]);
}

/// A store whose payloads clients reach answers a large payload, or a large range, with where it
/// is, and a small one, or a small range, with its bytes.
#[tokio::test]
async fn a_large_payload_is_answered_with_its_url() {
    let store = ChainStore::new(MemBackend::default())
        .with_payloads(objects(Some("http://objects.invalid")));
    let remote = client(store).await;
    let (item, payload) = genesis(INLINE_LIMIT as usize + 1).await;
    let parts = Parts {
        payload: Some(payload.clone()),
        ..Default::default()
    };
    remote.put_block(&item, &parts).await.unwrap();
    let commitment = item.leaf.payload_commitment().to_string();

    for at in [
        PayloadAt::Height(0),
        PayloadAt::Commitment(commitment.clone()),
    ] {
        let Some(PayloadData::Url(url)) = remote.payload(&at, None).await.unwrap() else {
            panic!("a large payload by URL, {at:?}");
        };
        assert_eq!(
            url,
            format!("http://objects.invalid/p/{}", commitment.replace('~', "."))
        );
        assert!(matches!(
            remote
                .payload(&at, Some(0..INLINE_LIMIT + 1))
                .await
                .unwrap(),
            Some(PayloadData::Url(_))
        ));
        assert_eq!(
            remote.payload(&at, Some(2..6)).await.unwrap(),
            Some(PayloadData::Inline(payload.slice(2..6))),
            "a small range inline, {at:?}"
        );
    }
}

/// A storage instance stages the payloads of the decide log, but a block an API instance puts,
/// as repair fills it, is uploaded at once, so it is not taken for missing and fetched again.
#[tokio::test]
async fn a_put_payload_is_uploaded_at_once() {
    let dir = tempfile::tempdir().unwrap();
    let (staged, _uploader) = ChainStore::new(MemBackend::default())
        .with_staged_payloads(objects(None), dir.path().to_path_buf())
        .await
        .unwrap();
    let (local, _follower) = LocalStore::new(staged.clone());
    let local = local.writing_through(staged.unstaged());
    let (item, payload) = genesis(10).await;
    let parts = Parts {
        payload: Some(payload),
        ..Default::default()
    };
    local.put_block(&item, &parts).await.unwrap();

    let commitment = item.leaf.payload_commitment().to_string();
    assert!(local.payload_status(&commitment).await.unwrap().has);
    assert!(
        local
            .missing_parts(Part::Payload, 0, 10)
            .await
            .unwrap()
            .is_empty()
    );
}
