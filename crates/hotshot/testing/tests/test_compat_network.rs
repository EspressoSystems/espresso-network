use std::net::Ipv4Addr;

use alloy::primitives::U256;
use hotshot::traits::implementations::{Cliquenet, CompatNetwork, MasterMap, MemoryNetwork};
use hotshot_example_types::node_types::TestTypes;
use hotshot_types::{
    addr::NetAddr,
    data::ViewNumber,
    signature_key::BLSPubKey,
    traits::{
        metrics::NoMetrics,
        network::{ConnectedNetwork, Topic},
        node_implementation::ConsensusTime,
    },
    x25519, PeerConnectInfo, ValidatorConfig,
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
