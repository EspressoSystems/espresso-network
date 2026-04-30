use std::{marker::PhantomData, net::Ipv4Addr, sync::Arc, time::Duration};

use alloy::primitives::U256;
use async_lock::RwLock;
use committable::Committable;
use hotshot::{
    traits::implementations::{Cliquenet, CompatNetwork, MasterMap, MemoryNetwork},
    types::SignatureKey,
};
use hotshot_example_types::{
    node_types::{MemoryImpl, TestTypes},
    storage_types::TestStorage,
};
use hotshot_types::{
    PeerConfig, PeerConnectInfo, ValidatorConfig,
    addr::NetAddr,
    data::{EpochNumber, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    signature_key::BLSPubKey,
    simple_certificate::UpgradeCertificate,
    simple_vote::UpgradeProposalData,
    traits::{
        election::Membership,
        network::{ConnectedNetwork, Topic},
        node_implementation::NodeType,
    },
    x25519,
};
use vbs::version::Version;
use versions::{CLIQUENET_VERSION, Upgrade};

#[test_log::test(tokio::test)]
async fn cliquenet_epoch_change_adds_peers_from_stake_table() {
    let v0 = make_validator(0);
    let v1 = make_validator(1);
    let v2 = make_validator(2);

    let coord = make_coordinator(&[&v0, &v1, &v2], 3).await;
    let cliq = make_cliquenet(&v0, &[&v0]).await;

    cliq.update_view(ViewNumber::new(1), Some(EpochNumber::new(2)), coord)
        .await;

    let peers = peers(&cliq);
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
    let cliq = make_cliquenet(&v0, &[&v0, &v1, &v2, &v3]).await;

    cliq.update_view(ViewNumber::new(1), Some(EpochNumber::new(2)), coord)
        .await;

    let peers = peers(&cliq);
    assert!(peers.contains(&v0.public_key), "v0 should remain");
    assert!(peers.contains(&v1.public_key), "v1 should remain");
    assert!(!peers.contains(&v2.public_key), "v2 should be removed");
    assert!(!peers.contains(&v3.public_key), "v3 should be removed");
}

#[test_log::test(tokio::test)]
async fn cliquenet_epoch_change_skips_validator_without_addr() {
    let v0 = make_validator(0);
    let v1_no_addr = ValidatorConfig::<TestTypes>::generated_from_seed_indexed(
        [0u8; 32],
        99,
        U256::from(1),
        true,
    );

    let coord = make_coordinator(&[&v0, &v1_no_addr], 3).await;
    let cliq = make_cliquenet(&v0, &[&v0]).await;

    cliq.update_view(ViewNumber::new(1), Some(EpochNumber::new(2)), coord)
        .await;

    assert!(
        !peers(&cliq).contains(&v1_no_addr.public_key),
        "validator without connect info should not become a peer"
    );
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

    let cliq = make_cliquenet(&v0, &[&v0]).await;

    cliq.update_view(ViewNumber::new(1), Some(EpochNumber::new(2)), coord.clone())
        .await;
    assert!(peers(&cliq).contains(&v2.public_key));

    // v2 is absent from epoch 3 and 4, but was in epoch 2, so we still expect to find it in peers.
    cliq.update_view(ViewNumber::new(2), Some(EpochNumber::new(3)), coord.clone())
        .await;
    assert!(peers(&cliq).contains(&v2.public_key));

    // v2 is absent going forward, so it should be removed.
    cliq.update_view(ViewNumber::new(3), Some(EpochNumber::new(4)), coord)
        .await;
    assert!(!peers(&cliq).contains(&v2.public_key));
}

#[test_log::test(tokio::test)]
async fn compat_network_switches_via_upgrade_lock() {
    let v0 = make_validator(0);
    let master = MasterMap::<BLSPubKey>::new();
    let net0 = MemoryNetwork::new(&v0.public_key, &master, &[Topic::Global], None);
    let cliq0 = make_cliquenet(&v0, &[&v0]).await;
    let compat0: CompatNetwork<_, TestTypes> = CompatNetwork::new(cliq0, net0).await;

    assert!(!compat0.is_cliquenet());

    // Set up an upgrade lock: base is pre-cliquenet, target is CLIQUENET_VERSION.
    let pre_cliquenet = Version {
        major: CLIQUENET_VERSION.major,
        minor: CLIQUENET_VERSION.minor - 1,
    };
    let upgrade = Upgrade::new(pre_cliquenet, CLIQUENET_VERSION);
    let lock = UpgradeLock::<TestTypes>::new(upgrade);
    compat0.set_upgrade_lock(lock.clone());

    // Simulate a decided upgrade certificate that takes effect at view 10.
    let data = UpgradeProposalData {
        old_version: pre_cliquenet,
        new_version: CLIQUENET_VERSION,
        decide_by: ViewNumber::new(8),
        new_version_hash: [0u8; 12].to_vec(),
        old_version_last_view: ViewNumber::new(9),
        new_version_first_view: ViewNumber::new(10),
    };
    let cert = UpgradeCertificate::<TestTypes>::new(
        data.clone(),
        data.commit(),
        ViewNumber::new(1),
        None,
        PhantomData,
    );
    lock.set_decided_upgrade_cert(cert);

    // A view before the upgrade should not switch.
    let coord = make_coordinator(&[&v0], 3).await;
    compat0
        .update_view(ViewNumber::new(5), None, coord.clone())
        .await;
    assert!(!compat0.is_cliquenet());

    // A view at the upgrade boundary should switch.
    compat0.update_view(ViewNumber::new(10), None, coord).await;
    assert!(compat0.is_cliquenet());
}

#[test_log::test(tokio::test)]
async fn compat_network_update_view_updates_both_networks() {
    let v0 = make_validator(0);
    let v1 = make_validator(1);

    let master = MasterMap::<BLSPubKey>::new();
    let net0 = MemoryNetwork::new(&v0.public_key, &master, &[Topic::Global], None);
    let cliq0 = make_cliquenet(&v0, &[&v0]).await;
    let compat0: CompatNetwork<_, TestTypes> = CompatNetwork::new(cliq0, net0).await;

    // We are in fallback mode.
    assert!(!compat0.is_cliquenet());

    // Epoch change adds v1 to the stake table.
    let coord = make_coordinator(&[&v0, &v1], 3).await;
    compat0
        .update_view(ViewNumber::new(2), Some(EpochNumber::new(2)), coord)
        .await;

    // Despite being in fallback mode, cliquenet's peer list was updated.
    assert!(
        peers(compat0.cliquenet()).contains(&v1.public_key),
        "cliquenet peers must be updated even while fallback is active"
    );
}

#[test_log::test(tokio::test)]
async fn compat_network_receives_from_fallback() {
    let v0 = make_validator(0);
    let v1 = make_validator(1);

    let master = MasterMap::<BLSPubKey>::new();
    let net0 = MemoryNetwork::new(&v0.public_key, &master, &[Topic::Global], None);
    let net1 = MemoryNetwork::new(&v1.public_key, &master, &[Topic::Global], None);

    let cliq0 = make_cliquenet(&v0, &[&v0]).await;
    let cliq1 = make_cliquenet(&v1, &[&v1]).await;
    let compat0: CompatNetwork<_, TestTypes> = CompatNetwork::new(cliq0, net0).await;
    let compat1: CompatNetwork<_, TestTypes> = CompatNetwork::new(cliq1, net1).await;

    let msg = b"hello via fallback";
    compat0
        .direct_message(ViewNumber::new(1), msg.to_vec(), v1.public_key)
        .await
        .unwrap();

    let received = compat1.recv_message().await.unwrap();
    assert_eq!(received, msg);
}

#[test_log::test(tokio::test)]
async fn compat_network_sends_and_receives_via_cliquenet() {
    let v0 = make_validator(0);
    let v1 = make_validator(1);

    let master = MasterMap::<BLSPubKey>::new();
    let net0 = MemoryNetwork::new(&v0.public_key, &master, &[Topic::Global], None);
    let net1 = MemoryNetwork::new(&v1.public_key, &master, &[Topic::Global], None);

    // Both cliquenet instances know each other as peers.
    let cliq0 = make_cliquenet(&v0, &[&v0, &v1]).await;
    let cliq1 = make_cliquenet(&v1, &[&v0, &v1]).await;

    let compat0: CompatNetwork<_, TestTypes> = CompatNetwork::new(cliq0, net0).await;
    let compat1: CompatNetwork<_, TestTypes> = CompatNetwork::new(cliq1, net1).await;

    // Switch both to cliquenet mode.
    compat0.use_cliquenet();
    compat1.use_cliquenet();

    let msg = b"hello via cliquenet";
    compat0
        .direct_message(ViewNumber::new(1), msg.to_vec(), v1.public_key)
        .await
        .unwrap();

    let received = tokio::time::timeout(Duration::from_secs(5), compat1.recv_message())
        .await
        .expect("timed out waiting for cliquenet message")
        .unwrap();

    assert_eq!(received, msg);
}

#[test_log::test(tokio::test)]
async fn compat_network_recv_selects_from_both_networks() {
    let v0 = make_validator(0);
    let v1 = make_validator(1);

    let master = MasterMap::<BLSPubKey>::new();
    let net0 = MemoryNetwork::new(&v0.public_key, &master, &[Topic::Global], None);
    let net1 = MemoryNetwork::new(&v1.public_key, &master, &[Topic::Global], None);

    let cliq0 = make_cliquenet(&v0, &[&v0]).await;
    let cliq1 = make_cliquenet(&v1, &[&v1]).await;

    // v0 stays in fallback mode (sends via MemoryNetwork).
    let compat0: CompatNetwork<_, TestTypes> = CompatNetwork::new(cliq0, net0).await;
    // v1 is in cliquenet mode but should still receive fallback messages.
    let compat1: CompatNetwork<_, TestTypes> = CompatNetwork::new(cliq1, net1).await;
    compat1.use_cliquenet();

    let msg = b"fallback msg to cliquenet receiver";
    compat0
        .direct_message(ViewNumber::new(1), msg.to_vec(), v1.public_key)
        .await
        .unwrap();

    let received = tokio::time::timeout(Duration::from_secs(5), compat1.recv_message())
        .await
        .expect("timed out: recv_message should select from both networks")
        .unwrap();

    assert_eq!(received, msg);
}

fn make_validator(index: u64) -> ValidatorConfig<TestTypes> {
    let mut v = ValidatorConfig::<TestTypes>::generated_from_seed_indexed(
        [0u8; 32],
        index,
        U256::from(1),
        true,
    );
    let k = x25519::Keypair::derive_from::<BLSPubKey>(&v.private_key).unwrap();
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

    Cliquenet::create(
        "test",
        owner.public_key,
        owner.x25519_keypair.clone().unwrap(),
        owner.p2p_addr.clone().unwrap(),
        parties,
    )
    .await
    .unwrap()
}

fn peers<K: SignatureKey + 'static>(c: &Cliquenet<K>) -> Vec<K> {
    c.peers()
        .into_iter()
        .filter_map(|(k, _)| c.reverse_lookup(&k))
        .collect()
}
