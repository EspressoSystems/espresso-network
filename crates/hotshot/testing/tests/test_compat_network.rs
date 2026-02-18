use std::{collections::HashSet, net::Ipv4Addr, sync::Arc};

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
    x25519, ValidatorConfig,
};

#[test_log::test(tokio::test)]
async fn cliquenet_on_epoch_change_adds_new_peer() {
    let v1 = make_validator(0);
    let v2 = make_validator(1);
    let v3 = make_validator(2);

    let cliq = make_cliquenet(&v1, &[&v1, &v2], &[&v3]).await;
    let peers = cliq.peers();
    let non_peers = cliq.non_peers().await;

    assert!(peers.contains(&v1.public_key));
    assert!(peers.contains(&v2.public_key));
    assert!(!peers.contains(&v3.public_key));
    assert_eq!(HashSet::from_iter([v3.public_key]), non_peers);

    // Add v3:
    let coord = make_coordinator(&[&v1, &v2, &v3], v1.public_key).await;

    cliq.update_view::<TestTypes>(ViewNumber::new(1), Some(EpochNumber::new(2)), coord)
        .await;

    let peers = cliq.peers();
    let non_peers = cliq.non_peers().await;

    assert!(peers.contains(&v1.public_key));
    assert!(peers.contains(&v2.public_key));
    assert!(peers.contains(&v3.public_key));
    assert!(non_peers.is_empty());
}

#[test_log::test(tokio::test)]
async fn cliquenet_on_epoch_change_removes_departed_peer() {
    let v1 = make_validator(0);
    let v2 = make_validator(1);
    let v3 = make_validator(2);

    let cliq = make_cliquenet(&v1, &[&v1, &v2, &v3], &[]).await;
    let peers = cliq.peers();
    let non_peers = cliq.non_peers().await;

    assert!(peers.contains(&v1.public_key));
    assert!(peers.contains(&v2.public_key));
    assert!(peers.contains(&v3.public_key));
    assert!(non_peers.is_empty());

    // Remove v3:
    let coord = make_coordinator(&[&v1, &v2], v1.public_key).await;

    cliq.update_view::<TestTypes>(ViewNumber::new(1), Some(EpochNumber::new(2)), coord)
        .await;

    let peers = cliq.peers();
    let non_peers = cliq.non_peers().await;

    assert!(peers.contains(&v1.public_key));
    assert!(peers.contains(&v2.public_key));
    assert!(!peers.contains(&v3.public_key));
    assert!(non_peers.is_empty());
}

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
async fn compat_network_update_view_refreshes_peer_routing() {
    let v1 = make_validator(0);
    let v2 = make_validator(1); // starts as a Cliquenet peer, departs after epoch change

    let master = MasterMap::<BLSPubKey>::new();
    let net1 = MemoryNetwork::new(&v1.public_key, &master, &[Topic::Global], None);
    let net2 = MemoryNetwork::new(&v2.public_key, &master, &[Topic::Global], None);
    let cliq1 = make_cliquenet(&v1, &[&v1, &v2], &[]).await;
    let cliq2 = make_cliquenet(&v2, &[&v1, &v2], &[]).await;
    let compat1 = CompatNetwork::new(cliq1, net1).await;
    let compat2 = CompatNetwork::new(cliq2, net2).await;

    let sent_msg = b"hello via cliquenet";
    compat1
        .direct_message(ViewNumber::new(1), sent_msg.to_vec(), v2.public_key)
        .await
        .unwrap();
    let recv_msg = compat2.recv_message().await.unwrap();
    assert_eq!(recv_msg, sent_msg);

    let coord = make_coordinator(&[&v1], v1.public_key).await;
    compat1
        .update_view::<TestTypes>(ViewNumber::new(1), Some(EpochNumber::new(2)), coord)
        .await;
    assert!(!compat1.cliquenet().peers().contains(&v2.public_key));

    let sent_msg = b"hello via fallback";
    compat1
        .direct_message(ViewNumber::new(2), sent_msg.to_vec(), v2.public_key)
        .await
        .unwrap();
    let received = compat2.recv_message().await.unwrap();
    assert_eq!(received, sent_msg);
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

async fn make_cliquenet(
    owner: &ValidatorConfig<TestTypes>,
    peers: &[&ValidatorConfig<TestTypes>],
    other: &[&ValidatorConfig<TestTypes>],
) -> Cliquenet<BLSPubKey> {
    let parties = peers.iter().map(|peer| {
        let p = peer.x25519_keypair.as_ref().unwrap().public_key();
        let a = peer.p2p_addr.clone().unwrap();
        (peer.public_key, p, a)
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

async fn make_coordinator(
    members: &[&ValidatorConfig<TestTypes>],
    owner: BLSPubKey,
) -> EpochMembershipCoordinator<TestTypes> {
    let peer_configs: Vec<_> = members.iter().map(|vc| vc.public_config()).collect();

    let storage = TestStorage::<TestTypes>::default();
    let master_map = MasterMap::<BLSPubKey>::new();
    let network = Arc::new(MemoryNetwork::new(
        &owner,
        &master_map,
        &[Topic::Da, Topic::Global],
        None,
    ));

    let membership = Arc::new(RwLock::new(<TestTypes as NodeType>::Membership::new::<
        MemoryImpl,
    >(
        peer_configs.clone(),
        peer_configs,
        storage.clone(),
        network,
        owner,
        10,
    )));

    {
        let mut m = membership.write().await;
        m.set_first_epoch(EpochNumber::new(1), [0; 32]);
        m.set_first_epoch(EpochNumber::new(2), [0; 32]);
        m.set_first_epoch(EpochNumber::new(3), [0; 32]);
    }

    EpochMembershipCoordinator::new(membership, 10, &storage)
}
