use std::{net::Ipv4Addr, sync::Arc};

use alloy::primitives::U256;
use async_lock::RwLock;
use hotshot::traits::implementations::{Cliquenet, CompatNetwork, MasterMap, MemoryNetwork};
use hotshot_example_types::{
    node_types::{MemoryImpl, TestTypes},
    storage_types::TestStorage,
};
use hotshot_types::{
    addr::NetAddr,
    data::{EpochNumber, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    signature_key::BLSPubKey,
    traits::{
        election::Membership,
        metrics::NoMetrics,
        network::{ConnectedNetwork, Topic},
        node_implementation::{ConsensusTime, NodeType},
    },
    x25519, PeerConfig, PeerConnectInfo, ValidatorConfig,
};

#[test_log::test(tokio::test)]
async fn compat_network_routes_unknown_peer_via_fallback() {
    let v1 = make_validator(0);
    let v2 = make_validator(1);

    let master = MasterMap::<BLSPubKey>::new();
    let net1 = MemoryNetwork::new(&v1.public_key, &master, &[Topic::Global], None);
    let net2 = MemoryNetwork::new(&v2.public_key, &master, &[Topic::Global], None);
    let cliq1 = make_cliquenet(&v1, &[&v1], &[&v2]).await;
    let cliq2 = make_cliquenet(&v2, &[&v2], &[&v1]).await;
    let compat1 = CompatNetwork::new(cliq1, net1).await;
    let compat2 = CompatNetwork::new(cliq2, net2).await;

    let sent_msg = b"hello via fallback";
    compat1
        .direct_message(ViewNumber::new(1), sent_msg.to_vec(), v2.public_key)
        .await
        .unwrap();
    let recv_msg = compat2.recv_message().await.unwrap();
    assert_eq!(recv_msg, sent_msg);
}

#[test_log::test(tokio::test)]
async fn cliquenet_epoch_change_adds_peers_from_stake_table() {
    let v0 = make_validator(0);
    let v1 = make_validator(1);
    let v2 = make_validator(2);

    let coord = make_coordinator(&[&v0, &v1, &v2], 3).await;
    let cliq = make_cliquenet(&v0, &[&v0], &[]).await;

    cliq.update_view(ViewNumber::new(1), Some(EpochNumber::new(2)), coord)
        .await;

    let peers = cliq.peers();
    assert!(peers.contains(&v0.public_key), "v0 should be a peer");
    assert!(peers.contains(&v1.public_key), "v1 should be a peer");
    assert!(peers.contains(&v2.public_key), "v2 should be a peer");
}

#[test_log::test(tokio::test)]
async fn cliquenet_epoch_change_removes_stale_peers() {
    let v0 = make_validator(0);
    let v1 = make_validator(1);
    let v2 = make_validator(2);
    let v3 = make_validator(3);

    // Stake table only has v0 and v1; v2 and v3 are initial peers but not in any epoch.
    let coord = make_coordinator(&[&v0, &v1], 3).await;
    let cliq = make_cliquenet(&v0, &[&v0, &v1, &v2, &v3], &[]).await;

    cliq.update_view(ViewNumber::new(1), Some(EpochNumber::new(2)), coord)
        .await;

    let peers = cliq.peers();
    assert!(peers.contains(&v0.public_key), "v0 should remain");
    assert!(peers.contains(&v1.public_key), "v1 should remain");
    assert!(!peers.contains(&v2.public_key), "v2 should be removed");
    assert!(!peers.contains(&v3.public_key), "v3 should be removed");
}

#[test_log::test(tokio::test)]
async fn cliquenet_epoch_change_no_addr_validator_goes_to_non_peers() {
    let v0 = make_validator(0);
    let v1_no_addr = ValidatorConfig::<TestTypes>::generated_from_seed_indexed(
        [0u8; 32],
        99,
        U256::from(1),
        true,
    );

    let coord = make_coordinator(&[&v0, &v1_no_addr], 3).await;
    let cliq = make_cliquenet(&v0, &[&v0], &[]).await;

    cliq.update_view(ViewNumber::new(1), Some(EpochNumber::new(2)), coord)
        .await;

    assert!(!cliq.peers().contains(&v1_no_addr.public_key));

    let non_peers = cliq.non_peers().await;
    assert!(non_peers.contains(&v1_no_addr.public_key));
}

/// A validator that leaves the stake table is retained as a peer for one extra
/// epoch, then removed on the following epoch change.
///
/// The test uses the DA stake table to make v2 present in epochs 1-2 and absent
/// from epoch 3 onwards, while v0/v1 are always in the quorum stake table.
#[test_log::test(tokio::test)]
async fn cliquenet_epoch_change_retains_prev_epoch_validator_then_removes() {
    let v0 = make_validator(0);
    let v1 = make_validator(1);
    let v2 = make_validator(2);

    let master = MasterMap::<BLSPubKey>::new();
    let storage = TestStorage::<TestTypes>::default();
    let network = Arc::new(MemoryNetwork::new(
        &v0.public_key,
        &master,
        &[Topic::Global],
        None,
    ));

    let quorum = vec![v0.public_config(), v1.public_config()];

    let mut membership = <TestTypes as NodeType>::Membership::new::<MemoryImpl>(
        quorum.clone(),
        quorum.clone(),
        storage.clone(),
        network,
        v0.public_key,
        10,
    );

    // Mark epochs 1 through 5 as having stake tables available.
    for e in 1..5 {
        membership.set_first_epoch(EpochNumber::new(e), [0u8; 32]);
    }

    membership.add_da_committee(
        1,
        vec![v0.public_config(), v1.public_config(), v2.public_config()],
    );

    // Remove v2 in epoch 3:
    membership.add_da_committee(3, vec![v0.public_config(), v1.public_config()]);

    let coord = EpochMembershipCoordinator::new(Arc::new(RwLock::new(membership)), 10, &storage);

    let cliq = make_cliquenet(&v0, &[&v0], &[]).await;

    cliq.update_view(ViewNumber::new(1), Some(EpochNumber::new(2)), coord.clone())
        .await;
    assert!(cliq.peers().contains(&v2.public_key));

    // v2 is absent from epoch 3 and 4, but was in epoch 2, so we still expect to find it in peers.
    cliq.update_view(ViewNumber::new(2), Some(EpochNumber::new(3)), coord.clone())
        .await;
    assert!(cliq.peers().contains(&v2.public_key));

    // v2 is absent going forward, so it should be removed.
    cliq.update_view(ViewNumber::new(3), Some(EpochNumber::new(4)), coord)
        .await;
    assert!(!cliq.peers().contains(&v2.public_key));
    assert!(!cliq.non_peers().await.contains(&v2.public_key));
}

#[test_log::test(tokio::test)]
async fn compat_network_updates_routing_after_epoch_change() {
    let v0 = make_validator(0);
    let v1 = make_validator(1);

    let master = MasterMap::<BLSPubKey>::new();
    let net0 = MemoryNetwork::new(&v0.public_key, &master, &[Topic::Global], None);
    let net1 = MemoryNetwork::new(&v1.public_key, &master, &[Topic::Global], None);

    // Initially v0's cliquenet has only itself; v1 is in neither peers nor non_peers.
    let cliq0 = make_cliquenet(&v0, &[&v0], &[]).await;
    let cliq1 = make_cliquenet(&v1, &[&v1], &[&v0]).await;
    let compat0 = CompatNetwork::new(cliq0, net0).await;
    let compat1 = CompatNetwork::new(cliq1, net1).await;

    // Before epoch change: v1 is not a cliquenet peer, so a direct message
    // must travel through the fallback (MemoryNetwork).
    let msg = b"via fallback before epoch change";
    compat0
        .direct_message(ViewNumber::new(1), msg.to_vec(), v1.public_key)
        .await
        .unwrap();
    let received = compat1.recv_message().await.unwrap();
    assert_eq!(received, msg);

    // Epoch change: both v0 and v1 now appear in the stake table.
    let coord = make_coordinator(&[&v0, &v1], 3).await;
    compat0
        .update_view(ViewNumber::new(2), Some(EpochNumber::new(2)), coord)
        .await;

    // After the epoch change, v1 must be in cliquenet peer set.
    assert!(compat0.cliquenet().peers().contains(&v1.public_key));
}

fn make_validator(index: u64) -> ValidatorConfig<TestTypes> {
    let mut v = ValidatorConfig::<TestTypes>::generated_from_seed_indexed(
        [0u8; 32],
        index,
        U256::from(1),
        true,
    );
    let k = x25519::Keypair::derive_from::<BLSPubKey>(&v.private_key);
    let p = test_utils::reserve_tcp_port().unwrap();
    v.x25519_keypair = Some(k);
    v.p2p_addr = Some(NetAddr::Inet(Ipv4Addr::LOCALHOST.into(), p));
    v
}

async fn make_coordinator(
    validators: &[&ValidatorConfig<TestTypes>],
    epochs: u64,
) -> EpochMembershipCoordinator<TestTypes> {
    let master = MasterMap::<BLSPubKey>::new();
    let public_key = validators[0].public_key;
    let network = Arc::new(MemoryNetwork::new(
        &public_key,
        &master,
        &[Topic::Global],
        None,
    ));
    let storage = TestStorage::<TestTypes>::default();

    let peer_configs: Vec<PeerConfig<TestTypes>> =
        validators.iter().map(|v| v.public_config()).collect();

    let mut membership = <TestTypes as NodeType>::Membership::new::<MemoryImpl>(
        peer_configs.clone(),
        peer_configs,
        storage.clone(),
        network,
        public_key,
        10,
    );

    for e in 1..epochs {
        membership.set_first_epoch(EpochNumber::new(e), [0u8; 32]);
    }

    EpochMembershipCoordinator::new(Arc::new(RwLock::new(membership)), 10, &storage)
}

async fn make_cliquenet(
    owner: &ValidatorConfig<TestTypes>,
    peers: &[&ValidatorConfig<TestTypes>],
    other: &[&ValidatorConfig<TestTypes>],
) -> Cliquenet<BLSPubKey> {
    let parties = peers.iter().map(|peer| {
        let p = peer.x25519_keypair.as_ref().unwrap().public_key();
        let a = peer.p2p_addr.clone().unwrap();
        (
            peer.public_key,
            PeerConnectInfo {
                x25519_key: p,
                p2p_addr: a,
            },
        )
    });

    let non_parties = other.iter().map(|val| val.public_key);

    Cliquenet::create(
        "test",
        owner.public_key,
        owner.x25519_keypair.clone().unwrap(),
        owner.p2p_addr.clone().unwrap(),
        parties,
        non_parties,
        Box::new(NoMetrics),
    )
    .await
    .unwrap()
}
