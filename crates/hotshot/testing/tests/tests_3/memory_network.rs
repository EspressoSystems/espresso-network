// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

#![allow(clippy::panic)]
use std::{sync::Arc, time::Duration};

use hotshot::{
    traits::{
        election::static_committee::StaticCommittee,
        implementations::{MasterMap, MemoryNetwork},
        NodeImplementation,
    },
    types::SignatureKey,
};
use hotshot_example_types::{
    auction_results_provider_types::{TestAuctionResult, TestAuctionResultsProvider},
    block_types::{TestBlockHeader, TestBlockPayload, TestTransaction},
    node_types::TestVersions,
    state_types::{TestInstanceState, TestValidatedState},
    storage_types::TestStorage,
};
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    message::{DataMessage, Message, MessageKind, UpgradeLock},
    signature_key::{BLSPubKey, BuilderKey, SchnorrPubKey},
    traits::{
        network::{BroadcastDelay, ConnectedNetwork, TestableNetworkingImplementation, Topic},
        node_implementation::{ConsensusTime, NodeType},
    },
};
use rand::{rngs::StdRng, RngCore, SeedableRng};
use serde::{Deserialize, Serialize};
use tokio::time::timeout;
use tracing::{instrument, trace};

#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct Test;

impl NodeType for Test {
    type AuctionResult = TestAuctionResult;
    type View = ViewNumber;
    type Epoch = EpochNumber;
    type BlockHeader = TestBlockHeader;
    type BlockPayload = TestBlockPayload;
    type SignatureKey = BLSPubKey;
    type Transaction = TestTransaction;
    type ValidatedState = TestValidatedState;
    type InstanceState = TestInstanceState;
    type Membership = StaticCommittee<Test>;
    type BuilderSignatureKey = BuilderKey;
    type StateSignatureKey = SchnorrPubKey;
}

#[derive(Clone, Debug, Deserialize, Serialize, Hash, PartialEq, Eq)]
pub struct TestImpl {}

impl NodeImplementation<Test> for TestImpl {
    type Network = MemoryNetwork<<Test as NodeType>::SignatureKey>;
    type Storage = TestStorage<Test>;
    type AuctionResultsProvider = TestAuctionResultsProvider<Test>;
}

/// fake Eq
/// we can't compare the votetokentype for equality, so we can't
/// derive EQ on `VoteType<TYPES>` and thereby message
/// we are only sending data messages, though so we compare key and
/// data message
fn fake_message_eq(message_1: Message<Test>, message_2: Message<Test>) {
    assert_eq!(message_1.sender, message_2.sender);
    if let MessageKind::Data(DataMessage::SubmitTransaction(d_1, _)) = message_1.kind {
        if let MessageKind::Data(DataMessage::SubmitTransaction(d_2, _)) = message_2.kind {
            assert_eq!(d_1, d_2);
        }
    } else {
        panic!("Got unexpected message type in memory test!");
    }
}

#[instrument]
fn pubkey() -> BLSPubKey {
    // random 32 bytes
    let mut bytes = [0; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    BLSPubKey::generated_from_seed_indexed(bytes, 0).0
}

/// create a message
fn gen_messages(num_messages: u64, seed: u64, pk: BLSPubKey) -> Vec<Message<Test>> {
    let mut messages = Vec::new();
    for _ in 0..num_messages {
        // create a random transaction from seed
        let mut bytes = [0u8; 8];
        let mut rng = StdRng::seed_from_u64(seed);
        rng.fill_bytes(&mut bytes);

        let message = Message {
            sender: pk,
            kind: MessageKind::Data(DataMessage::SubmitTransaction(
                TestTransaction::new(bytes.to_vec()),
                <ViewNumber as ConsensusTime>::new(0),
            )),
        };
        messages.push(message);
    }
    messages
}

// Spawning a single MemoryNetwork should produce no errors

#[tokio::test(flavor = "multi_thread")]
#[instrument]
async fn memory_network_spawn_single() {
    hotshot::helpers::initialize_logging();
    let group: Arc<MasterMap<<Test as NodeType>::SignatureKey>> = MasterMap::new();
    trace!(?group);
    let _pub_key = pubkey();
}

// // Spawning a two MemoryNetworks and connecting them should produce no errors

#[tokio::test(flavor = "multi_thread")]
#[instrument]
async fn memory_network_spawn_double() {
    hotshot::helpers::initialize_logging();
    let group: Arc<MasterMap<<Test as NodeType>::SignatureKey>> = MasterMap::new();
    trace!(?group);
    let _pub_key_1 = pubkey();
    let _pub_key_2 = pubkey();
}

// Check to make sure direct queue works

#[tokio::test(flavor = "multi_thread")]
#[instrument]
async fn memory_network_direct_queue() {
    hotshot::helpers::initialize_logging();
    // Create some dummy messages

    // Make and connect the networking instances
    let group: Arc<MasterMap<<Test as NodeType>::SignatureKey>> = MasterMap::new();
    trace!(?group);

    let pub_key_1 = pubkey();
    let network1 = MemoryNetwork::new(&pub_key_1, &group.clone(), &[Topic::Global], Option::None);

    let pub_key_2 = pubkey();
    let network2 = MemoryNetwork::new(&pub_key_2, &group, &[Topic::Global], Option::None);

    let first_messages: Vec<Message<Test>> = gen_messages(5, 100, pub_key_1);

    let upgrade_lock = UpgradeLock::<Test, TestVersions>::new();

    // Test 1 -> 2
    // Send messages
    for sent_message in first_messages {
        let serialized_message = upgrade_lock.serialize(&sent_message).await.unwrap();
        network1
            .direct_message(serialized_message.clone(), pub_key_2)
            .await
            .expect("Failed to message node");
        let recv_message = network2
            .recv_message()
            .await
            .expect("Failed to receive message");
        let deserialized_message = upgrade_lock.deserialize(&recv_message).await.unwrap();
        assert!(timeout(Duration::from_secs(1), network2.recv_message())
            .await
            .is_err());
        fake_message_eq(sent_message, deserialized_message);
    }

    let second_messages: Vec<Message<Test>> = gen_messages(5, 200, pub_key_2);

    // Test 2 -> 1
    // Send messages
    for sent_message in second_messages {
        let serialized_message = upgrade_lock.serialize(&sent_message).await.unwrap();
        network2
            .direct_message(serialized_message.clone(), pub_key_1)
            .await
            .expect("Failed to message node");
        let recv_message = network1
            .recv_message()
            .await
            .expect("Failed to receive message");
        let deserialized_message = upgrade_lock.deserialize(&recv_message).await.unwrap();
        assert!(timeout(Duration::from_secs(1), network1.recv_message())
            .await
            .is_err());
        fake_message_eq(sent_message, deserialized_message);
    }
}

// Check to make sure direct queue works

#[tokio::test(flavor = "multi_thread")]
#[instrument]
async fn memory_network_broadcast_queue() {
    // Make and connect the networking instances
    let group: Arc<MasterMap<<Test as NodeType>::SignatureKey>> = MasterMap::new();
    let pub_key_1 = pubkey();
    let network1 = MemoryNetwork::new(&pub_key_1, &group.clone(), &[Topic::Global], Option::None);
    let pub_key_2 = pubkey();
    let network2 = MemoryNetwork::new(&pub_key_2, &group, &[Topic::Da], Option::None);

    let first_messages: Vec<Message<Test>> = gen_messages(5, 100, pub_key_1);

    let upgrade_lock = UpgradeLock::<Test, TestVersions>::new();

    // Test 1 -> 2
    // Send messages
    for sent_message in first_messages {
        let serialized_message = upgrade_lock.serialize(&sent_message).await.unwrap();
        network1
            .broadcast_message(serialized_message.clone(), Topic::Da, BroadcastDelay::None)
            .await
            .expect("Failed to message node");
        let recv_message = network2
            .recv_message()
            .await
            .expect("Failed to receive message");
        let deserialized_message = upgrade_lock.deserialize(&recv_message).await.unwrap();
        assert!(timeout(Duration::from_secs(1), network2.recv_message())
            .await
            .is_err());
        fake_message_eq(sent_message, deserialized_message);
    }

    let second_messages: Vec<Message<Test>> = gen_messages(5, 200, pub_key_2);

    // Test 2 -> 1
    // Send messages
    for sent_message in second_messages {
        let serialized_message = upgrade_lock.serialize(&sent_message).await.unwrap();
        network2
            .broadcast_message(
                serialized_message.clone(),
                Topic::Global,
                BroadcastDelay::None,
            )
            .await
            .expect("Failed to message node");
        let recv_message = network1
            .recv_message()
            .await
            .expect("Failed to receive message");
        let deserialized_message = upgrade_lock.deserialize(&recv_message).await.unwrap();
        assert!(timeout(Duration::from_secs(1), network1.recv_message())
            .await
            .is_err());
        fake_message_eq(sent_message, deserialized_message);
    }
}

#[tokio::test(flavor = "multi_thread")]
#[instrument]
#[allow(deprecated)]
async fn memory_network_test_in_flight_message_count() {
    hotshot::helpers::initialize_logging();

    let group: Arc<MasterMap<<Test as NodeType>::SignatureKey>> = MasterMap::new();
    trace!(?group);
    let pub_key_1 = pubkey();
    let network1 = MemoryNetwork::new(&pub_key_1, &group.clone(), &[Topic::Global], Option::None);
    let pub_key_2 = pubkey();
    let network2 = MemoryNetwork::new(&pub_key_2, &group, &[Topic::Global], Option::None);

    // Create some dummy messages
    let messages: Vec<Message<Test>> = gen_messages(5, 100, pub_key_1);

    assert_eq!(
        TestableNetworkingImplementation::<Test>::in_flight_message_count(&network1),
        Some(0)
    );
    assert_eq!(
        TestableNetworkingImplementation::<Test>::in_flight_message_count(&network2),
        Some(0)
    );

    let upgrade_lock = UpgradeLock::<Test, TestVersions>::new();

    for (count, message) in messages.iter().enumerate() {
        let serialized_message = upgrade_lock.serialize(message).await.unwrap();

        network1
            .direct_message(serialized_message.clone(), pub_key_2)
            .await
            .unwrap();
        // network 2 has received `count` broadcast messages and `count + 1` direct messages
        assert_eq!(
            TestableNetworkingImplementation::<Test>::in_flight_message_count(&network2),
            Some(count + count + 1)
        );

        network2
            .broadcast_message(
                serialized_message.clone(),
                Topic::Global,
                BroadcastDelay::None,
            )
            .await
            .unwrap();
        // network 1 has received `count` broadcast messages
        assert_eq!(
            TestableNetworkingImplementation::<Test>::in_flight_message_count(&network1),
            Some(count + 1)
        );

        // network 2 has received `count + 1` broadcast messages and `count + 1` direct messages
        assert_eq!(
            TestableNetworkingImplementation::<Test>::in_flight_message_count(&network2),
            Some((count + 1) * 2)
        );
    }

    while TestableNetworkingImplementation::<Test>::in_flight_message_count(&network1).unwrap() > 0
    {
        network1.recv_message().await.unwrap();
    }

    while TestableNetworkingImplementation::<Test>::in_flight_message_count(&network2).unwrap()
        > messages.len()
    {
        network2.recv_message().await.unwrap();
    }

    while TestableNetworkingImplementation::<Test>::in_flight_message_count(&network2).unwrap() > 0
    {
        network2.recv_message().await.unwrap();
    }

    assert_eq!(
        TestableNetworkingImplementation::<Test>::in_flight_message_count(&network1),
        Some(0)
    );
    assert_eq!(
        TestableNetworkingImplementation::<Test>::in_flight_message_count(&network2),
        Some(0)
    );
}
