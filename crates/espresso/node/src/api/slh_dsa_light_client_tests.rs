//! The light client stake table endpoint and SLH-DSA key events.
//!
//! Light clients on the 0.7.x releases have no `SlhDsaKeyUpdate` variant, and serde rejects a whole
//! reply that holds one unknown variant. The stake table hash does not depend on key events, so the
//! endpoint can leave them out.

use std::time::Duration;

use espresso_api::proto;
use espresso_types::{
    Header, SeqTypes, TEST_UPGRADE,
    traits::{EventsPersistenceRead, MembershipPersistence},
    v0_3::{EventKey, StakeTableEvent},
};
use futures::StreamExt;
use hotshot_contract_adapter::sol_types::{
    StakeTableV3::{
        CommissionUpdated, ConsensusKeysUpdated, ConsensusKeysUpdatedV2, Delegated, P2pAddrUpdated,
        Undelegated, UndelegatedV2, ValidatorExit, ValidatorExitV2, ValidatorRegistered,
        ValidatorRegisteredV2, ValidatorRegisteredV3, X25519KeyUpdated,
    },
    StakeTableV4::SlhDsaKeyUpdated,
};
use hotshot_query_service::{availability::LeafQueryData, types::HeightIndexed};
use hotshot_types::{
    data::EpochNumber,
    slh_dsa::{PUBLIC_KEY_LEN, SIGNATURE_LEN},
    traits::metrics::NoMetrics,
    utils::root_block_in_epoch,
};
use http_client::{Client, ContentType, error::ClientErr};
use serde::{Deserialize, Serialize};
use test_utils::reserve_tcp_port;
use url::Url;
use vbs::version::StaticVersion;

use crate::{
    SequencerApiVersion,
    api::{
        data_source::{SequencerDataSource, testing::TestableSequencerDataSource},
        options::Options,
        sql::sqlite::DataSource as Sqlite,
        test_helpers::{TestNetwork, TestNetworkConfigBuilder, node_storage},
    },
    catchup::StatePeers,
    testing::TestConfigBuilder,
};

const NUM_NODES: usize = 1;
const EPOCH_HEIGHT: u64 = 50;

/// `store_events` attempts before giving up: the node's L1 update task can raise the last stored
/// L1 block between our read of it and our write, and then the write is skipped.
const STORE_ATTEMPTS: usize = 5;

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn light_client_stake_table_omits_slh_dsa_key_events() {
    let fixture = KeyEventFixture::new().await;

    let path = format!("light-client/stake-table/{}", fixture.epoch);
    let v1: Vec<StakeTableEvent> = fixture.client.get(&path).send().await.unwrap();
    let key_events = v1
        .iter()
        .filter(|event| matches!(event, StakeTableEvent::SlhDsaKeyUpdate(_)))
        .count();
    assert_eq!(
        key_events, 0,
        "{path} returned {key_events} SlhDsaKeyUpdate events, which light clients on 0.7.x cannot \
         parse"
    );
    let others: Vec<StakeTableEvent> = fixture
        .stored
        .iter()
        .filter(|(key, _)| *key != fixture.key)
        .map(|(_, event)| event.clone())
        .collect();
    assert!(
        v1 == others,
        "{path} must return every other stored event in L1 order: got {} events, storage holds {} \
         besides the key event",
        v1.len(),
        others.len()
    );

    let v2_path = format!("v2/light-client/stake-table?epoch={}", fixture.epoch);
    let v2: proto::LightClientStakeTableResponse =
        fixture.client.get(&v2_path).send().await.unwrap();
    let empty = v2
        .events
        .iter()
        .filter(|event| event.event.is_none())
        .count();
    assert_eq!(
        empty, 0,
        "{v2_path} returned {empty} items with no event, which is what an SlhDsaKeyUpdate event \
         becomes in v2"
    );
    assert_eq!(
        v2.events,
        v1.iter()
            .map(proto::StakeTableEvent::from)
            .collect::<Vec<_>>(),
        "{v2_path} must return the same events as {path}"
    );
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn light_client_stake_table_parses_as_0_7_events() {
    let fixture = KeyEventFixture::new().await;

    let path = format!("light-client/stake-table/{}", fixture.epoch);
    let json_client: Client<ClientErr, StaticVersion<0, 1>> = Client::builder(fixture.url.clone())
        .content_type(ContentType::Json)
        .build();
    let body = json_client
        .get::<serde_json::Value>(&path)
        .bytes()
        .await
        .unwrap();
    let events: Vec<StakeTableEventV07> = serde_json::from_slice(&body).unwrap_or_else(|err| {
        panic!("a 0.7.x light client cannot parse the reply of {path}: {err}")
    });
    assert!(
        !events.is_empty(),
        "{path} returned no events, so the parse proves nothing"
    );
    assert_eq!(
        serde_json::to_value(&events).unwrap(),
        serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
        "a 0.7.x light client drops data from the reply of {path}"
    );
}

/// `StakeTableEvent` as light clients on the 0.7.x releases have it: without `SlhDsaKeyUpdate`.
#[derive(Serialize, Deserialize)]
enum StakeTableEventV07 {
    Register(ValidatorRegistered),
    RegisterV2(ValidatorRegisteredV2),
    Deregister(ValidatorExit),
    DeregisterV2(ValidatorExitV2),
    Delegate(Delegated),
    Undelegate(Undelegated),
    UndelegateV2(UndelegatedV2),
    KeyUpdate(ConsensusKeysUpdated),
    KeyUpdateV2(ConsensusKeysUpdatedV2),
    CommissionUpdate(CommissionUpdated),
    RegisterV3(ValidatorRegisteredV3),
    X25519KeyUpdate(X25519KeyUpdated),
    P2pAddrUpdate(P2pAddrUpdated),
}

/// A one-node network, stopped, whose storage holds an SLH-DSA key event in the L1 range that the
/// light client stake table endpoint reads for `epoch`.
struct KeyEventFixture {
    client: Client<ClientErr, StaticVersion<0, 1>>,
    url: Url,
    epoch: EpochNumber,
    /// The key of the injected `SlhDsaKeyUpdate` event.
    key: EventKey,
    /// What storage holds for the L1 range of `epoch`, the injected event included.
    stored: Vec<(EventKey, StakeTableEvent)>,
    // Declared before the storage, so it is dropped first.
    _network: TestNetwork<<Sqlite as SequencerDataSource>::Options, NUM_NODES>,
    _storage: Vec<<Sqlite as TestableSequencerDataSource>::Storage>,
}

impl KeyEventFixture {
    async fn new() -> Self {
        let port = reserve_tcp_port().expect("OS should have ephemeral ports available");
        let url: Url = format!("http://localhost:{port}").parse().unwrap();

        let test_config = TestConfigBuilder::default()
            .epoch_height(EPOCH_HEIGHT)
            .epoch_start_block(0)
            .builder_timeout(Duration::from_millis(250))
            .build();
        let (storage, persistence) = node_storage::<Sqlite, NUM_NODES>().await;
        let config = TestNetworkConfigBuilder::<NUM_NODES, _, _>::with_num_nodes()
            .api_config(Sqlite::options(&storage[0], Options::with_port(port)))
            .persistences(persistence.clone())
            .catchups(std::array::from_fn(|_| {
                StatePeers::<SequencerApiVersion>::from_urls(
                    vec![url.clone()],
                    Default::default(),
                    Duration::from_secs(2),
                    &NoMetrics,
                )
            }))
            .network_config(test_config)
            .build();
        let mut network = TestNetwork::new(config, TEST_UPGRADE).await;
        let client: Client<ClientErr, StaticVersion<0, 1>> = Client::new(url.clone());
        client.connect(None).await;

        // The first epoch the endpoint serves. It reads every event up to the L1 block that the
        // epoch root of `epoch - 2` finalizes.
        let epoch = wait_for_first_served_epoch(&client).await;
        network.stop_consensus().await;

        let root = root_block_in_epoch(*epoch - 2, EPOCH_HEIGHT);
        let root_header: Header = client
            .get(&format!("availability/header/{root}"))
            .send()
            .await
            .unwrap();
        let to_l1_block = root_header
            .l1_finalized()
            .expect("the epoch root finalizes an L1 block")
            .number();

        let persistence = network.server.persistence();
        let (key, key_event) =
            key_event_for_a_registered_validator(&*persistence, to_l1_block).await;
        let (status, stored) = inject(&*persistence, key, key_event, to_l1_block).await;
        assert_eq!(
            status,
            Some(EventsPersistenceRead::Complete),
            "precondition: storage must hold every event up to L1 block {to_l1_block}"
        );
        assert!(
            stored.iter().any(|(stored_key, event)| *stored_key == key
                && matches!(event, StakeTableEvent::SlhDsaKeyUpdate(_))),
            "precondition: storage must return the key event at {key:?} in the L1 range [0, \
             {to_l1_block}] of epoch {epoch}"
        );

        Self {
            client,
            url,
            epoch,
            key,
            stored,
            _network: network,
            _storage: storage,
        }
    }
}

/// Waits until the chain is in the first epoch that the light client stake table endpoint serves,
/// two after the first epoch, so the node has stored the events of its stake table.
async fn wait_for_first_served_epoch(
    client: &Client<ClientErr, StaticVersion<0, 1>>,
) -> EpochNumber {
    let mut leaves = client
        .socket("availability/stream/leaves/0")
        .subscribe::<LeafQueryData<SeqTypes>>()
        .await
        .unwrap();
    let first_epoch = leaves
        .next()
        .await
        .unwrap()
        .unwrap()
        .leaf()
        .epoch(EPOCH_HEIGHT)
        .unwrap();
    let epoch = first_epoch + 2;
    loop {
        let leaf = leaves.next().await.unwrap().unwrap();
        let leaf_epoch = leaf.leaf().epoch(EPOCH_HEIGHT).unwrap();
        if leaf_epoch >= epoch {
            return epoch;
        }
        tracing::info!(
            height = leaf.height(),
            ?leaf_epoch,
            ?epoch,
            "waiting for epoch"
        );
    }
}

/// An SLH-DSA key event for a validator that registered at or before `to_l1_block`, keyed in the
/// L1 block of that registration with a log index no stored event uses.
async fn key_event_for_a_registered_validator(
    persistence: &impl MembershipPersistence,
    to_l1_block: u64,
) -> (EventKey, StakeTableEvent) {
    let (_, stored) = persistence.load_events(0, to_l1_block).await.unwrap();
    let ((l1_block, _), validator) = stored
        .iter()
        .find_map(|(key, event)| {
            let account = match event {
                StakeTableEvent::Register(event) => event.account,
                StakeTableEvent::RegisterV2(event) => event.account,
                StakeTableEvent::RegisterV3(event) => event.account,
                _ => return None,
            };
            Some((*key, account))
        })
        .unwrap_or_else(|| panic!("no validator registered up to L1 block {to_l1_block}"));
    let log_index = stored
        .iter()
        .filter(|((block, _), _)| *block == l1_block)
        .map(|((_, index), _)| index + 1)
        .max()
        .unwrap();
    let event = SlhDsaKeyUpdated {
        validator,
        slhDsaVK: vec![0x11; PUBLIC_KEY_LEN].into(),
        slhDsaSig: vec![0x22; SIGNATURE_LEN].into(),
    };
    (
        (l1_block, log_index),
        StakeTableEvent::SlhDsaKeyUpdate(event),
    )
}

/// Stores `event` at `key`, and returns what storage holds up to `to_l1_block` afterwards.
async fn inject(
    persistence: &impl MembershipPersistence,
    key: EventKey,
    event: StakeTableEvent,
    to_l1_block: u64,
) -> (
    Option<EventsPersistenceRead>,
    Vec<(EventKey, StakeTableEvent)>,
) {
    for attempt in 1..=STORE_ATTEMPTS {
        // `store_events` skips a write below the last stored L1 block.
        let last_l1_block = last_stored_l1_block(persistence).await;
        persistence
            .store_events(last_l1_block, vec![(key, event.clone())])
            .await
            .unwrap();
        let (status, stored) = persistence.load_events(0, to_l1_block).await.unwrap();
        if stored.iter().any(|(stored_key, _)| *stored_key == key) {
            return (status, stored);
        }
        tracing::info!(attempt, last_l1_block, "key event not stored, retrying");
    }
    panic!("could not store the key event at {key:?} in {STORE_ATTEMPTS} attempts");
}

/// The last L1 block storage holds stake table events up to.
async fn last_stored_l1_block(persistence: &impl MembershipPersistence) -> u64 {
    let beyond_l1 = i64::MAX as u64;
    match persistence.load_events(beyond_l1, beyond_l1).await.unwrap() {
        (Some(EventsPersistenceRead::UntilL1Block(block)), _) => block,
        (status, _) => panic!("storage tracks no last L1 block: {status:?}"),
    }
}
