use std::{
    collections::{BTreeMap, BTreeSet},
    net::Ipv4Addr,
    time::Duration,
};

use hotshot_example_types::{
    membership::{
        TestableMembership, static_committee::StaticStakeTable, strict_membership::StrictMembership,
    },
    node_stake::{TestNodeStakes, gen_node_lists},
    node_types::TestTypes,
    storage_types::TestStorage,
};
use hotshot_metrics::NoMetrics;
use hotshot_types::{
    PeerConfig, PeerConnectInfo,
    addr::NetAddr,
    data::{EpochNumber, ViewNumber},
    drb::INITIAL_DRB_RESULT,
    epoch_membership::EpochMembershipCoordinator,
    signature_key::{BLSPubKey, SchnorrPubKey},
    traits::election::Membership,
    x25519::Keypair,
};
use tokio::time::timeout;

use crate::{
    helpers::test_upgrade_lock,
    message::{
        Message, MessageType, Validated,
        fetch::Request,
        payload::{PayloadFetchMessage, PayloadRequestBody},
    },
    network::Cliquenet,
    tests::common::{runner::TestRunner, utils::epoch_length},
};

#[tokio::test(flavor = "multi_thread")]
async fn five_nodes_decide_same_chain_over_cliquenet() {
    TestRunner::builder().build().run().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn three_nodes_decide_over_cliquenet() {
    TestRunner::builder()
        .num_nodes(3)
        .target_decisions(50)
        .build()
        .run()
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn epoch_changes_over_cliquenet() {
    TestRunner::builder()
        .epoch_height(10)
        .target_decisions(50)
        .build()
        .run()
        .await
        .unwrap();
}

/// Nodes 1 and 4 cannot connect to each other; cliquenet does not relay.
/// When one of them leads the other never receives the proposal and cannot
/// vote, leaving 6 of 7 votes against the quorum threshold of 5
/// (`2n/3 + 1`) — one vote of slack, so every view is still required to
/// decide. The pair sits 3 apart in the round-robin leader order, so a
/// connected leader always follows a blocked one and re-propagates its
/// certificates.
#[tokio::test(flavor = "multi_thread")]
async fn blocked_pair_still_decides_over_cliquenet() {
    TestRunner::builder()
        .num_nodes(7)
        .target_decisions(30)
        .epoch_height(10)
        .blocked_pairs(BTreeSet::from([(1, 4)]))
        .build()
        .run()
        .await
        .unwrap();
}

/// As in [`blocked_pair_still_decides_over_cliquenet`], but nodes 1, 4 and
/// 7 of 10 are mutually unreachable: a blocked leader still collects 8 of
/// 10 votes against the quorum threshold of 7 (`2n/3 + 1`) — one vote of
/// slack.
#[tokio::test(flavor = "multi_thread")]
async fn mutually_blocked_nodes_still_decide_over_cliquenet() {
    TestRunner::builder()
        .num_nodes(10)
        .target_decisions(30)
        .epoch_height(10)
        .blocked_pairs(BTreeSet::from([(1, 4), (1, 7), (4, 7)]))
        .build()
        .run()
        .await
        .unwrap();
}

/// A node that misses one view's VID share broadcasts recovers by fetching
/// the payload, and the network certifies again.
///
/// Five of seven nodes are up, which is exactly the quorum threshold, so no
/// view can certify without every one of them. Node 1 receives view 2's
/// proposal, its own share fragments and every vote, and so takes full part
/// in that view — but never the other voters' share broadcasts, so it cannot
/// reconstruct the block. From then on it cannot vote on any proposal
/// parented at view 2, which is every later proposal, and the run only
/// completes if it recovers the payload from a peer.
///
/// The views around the deficit are tolerated rather than pinned: how many
/// time out before the fetch lands depends on scheduling. Completing at all
/// is the assertion — without the fetch this run never reaches its target.
#[tokio::test(flavor = "multi_thread")]
async fn share_starved_node_recovers_over_cliquenet() {
    TestRunner::builder()
        .num_nodes(7)
        .down_nodes(BTreeSet::from([5, 6]))
        .starved_of_shares(BTreeMap::from([(1, BTreeSet::from([ViewNumber::new(2)]))]))
        .tolerated_failed_views((2..=8).map(ViewNumber::new).collect())
        .target_decisions(12)
        .view_timeout(std::time::Duration::from_secs(2))
        .build()
        .run()
        .await
        .unwrap();
}

/// A validator that rotates its BLS key keeps its x25519 key, so the old and
/// the new key name one cliquenet party. When the old key leaves the peer
/// window, the party must stay: the new key still uses it.
///
/// The validator holds its old key in epochs 1-2 and its new key from epoch
/// 3, so the observer drops the old key when it applies epoch 4.
#[tokio::test(flavor = "multi_thread")]
async fn bls_key_rotation_keeps_the_shared_x25519_party() {
    let observer_identity = Identity::generate();
    let validator_identity = Identity::generate();
    let members = members([
        &observer_identity.info,
        &validator_identity.info,
        &validator_identity.info,
    ]);
    let [observer, old, new] = [0, 1, 2].map(|i| members[i].stake_table_entry.stake_key);
    let membership = rotating_membership(&members);
    let observer_party = (observer, observer_identity.info.clone());
    let mut validator_net = cliquenet(old, validator_identity, vec![observer_party]).await;
    let mut observer_net = cliquenet(observer, observer_identity, vec![]).await;

    observer_net
        .apply_epoch(EpochNumber::new(1), &membership)
        .unwrap();
    assert_delivers(&observer_net, observer, &mut validator_net, old, 1).await;

    for epoch in 2..=4 {
        observer_net
            .apply_epoch(EpochNumber::new(epoch), &membership)
            .unwrap();
    }
    assert_delivers(&observer_net, observer, &mut validator_net, new, 2).await;
}

/// A validator that replaces its x25519 key keeps its BLS key, so the peer
/// stays and only its cliquenet party changes. The old party must go, or the
/// observer keeps a connection that nothing addresses any more.
///
/// The validator holds its old x25519 key in epochs 1-2 and its new one from
/// epoch 3, so the observer switches when it applies epoch 2.
#[tokio::test(flavor = "multi_thread")]
async fn x25519_key_replacement_removes_the_old_party() {
    let observer_identity = Identity::generate();
    let old_identity = Identity::generate();
    let new_identity = Identity::generate();
    let mut members = members([
        &observer_identity.info,
        &old_identity.info,
        &new_identity.info,
    ]);
    members[2].stake_table_entry = members[1].stake_table_entry.clone();
    let [observer, validator] = [0, 1].map(|i| members[i].stake_table_entry.stake_key);
    let membership = rotating_membership(&members);
    let observer_party = (observer, observer_identity.info.clone());
    let mut old_net = cliquenet(validator, old_identity, vec![observer_party.clone()]).await;
    let mut new_net = cliquenet(validator, new_identity, vec![observer_party]).await;
    let mut observer_net = cliquenet(observer, observer_identity, vec![]).await;

    observer_net
        .apply_epoch(EpochNumber::new(1), &membership)
        .unwrap();
    assert_delivers(&observer_net, observer, &mut old_net, validator, 1).await;

    observer_net
        .apply_epoch(EpochNumber::new(2), &membership)
        .unwrap();
    assert_delivers(&observer_net, observer, &mut new_net, validator, 2).await;

    // The old party sends first, so a connection the observer still held
    // would deliver its message before or right after the new party's.
    send(&old_net, validator, observer, 3);
    assert_delivers(&new_net, validator, &mut observer_net, observer, 4).await;
    let stray = timeout(Duration::from_millis(250), observer_net.receive()).await;
    assert!(stray.is_err(), "the old party still delivers: {stray:?}");
}

/// An x25519 keypair and the connect info that reaches it.
struct Identity {
    keypair: Keypair,
    info: PeerConnectInfo,
}

impl Identity {
    fn generate() -> Self {
        let keypair = Keypair::generate().expect("keypair generation should succeed");
        let port =
            test_utils::reserve_tcp_port().expect("OS should have ephemeral ports available");
        let info = PeerConnectInfo {
            x25519_key: keypair.public_key(),
            p2p_addr: NetAddr::Inet(Ipv4Addr::LOCALHOST.into(), port),
        };
        Self { keypair, info }
    }
}

/// Three members, registered with `infos` in order.
fn members(infos: [&PeerConnectInfo; 3]) -> Vec<PeerConfig<TestTypes>> {
    let mut members = gen_node_lists::<TestTypes>(3, 3, &TestNodeStakes::default()).0;
    for (member, info) in members.iter_mut().zip(infos) {
        member.connect_info = Some(info.clone());
    }
    members
}

/// Epochs 1-2 hold members 0 and 1, epochs 3-5 hold members 0 and 2.
fn rotating_membership(members: &[PeerConfig<TestTypes>]) -> EpochMembershipCoordinator<TestTypes> {
    const EPOCH_HEIGHT: u64 = 10;
    let before = vec![members[0].clone(), members[1].clone()];
    let after = vec![members[0].clone(), members[2].clone()];
    let membership = StrictMembership::<TestTypes, StaticStakeTable<BLSPubKey, SchnorrPubKey>>::new(
        before.clone(),
        before,
        members[0].stake_table_entry.stake_key,
        EPOCH_HEIGHT,
        EpochNumber::genesis(),
    );
    membership.add_quorum_committee(EpochNumber::new(3), after.clone());
    membership.add_da_committee(EpochNumber::new(3), after);
    for epoch in 3..=5 {
        membership.register_epoch(EpochNumber::new(epoch), INITIAL_DRB_RESULT);
    }
    EpochMembershipCoordinator::new(
        membership,
        epoch_length(EPOCH_HEIGHT),
        &TestStorage::default(),
    )
}

/// Networks handshake only with networks of the same name.
async fn cliquenet(
    key: BLSPubKey,
    identity: Identity,
    parties: Vec<(BLSPubKey, PeerConnectInfo)>,
) -> Cliquenet<TestTypes> {
    Cliquenet::create(
        "test-key-rotation",
        key,
        identity.keypair,
        identity.info.p2p_addr,
        parties,
        None,
        test_upgrade_lock(),
        Box::new(NoMetrics),
    )
    .await
    .expect("cliquenet creation should succeed")
}

/// `from`, signing as `from_key`, unicasts a message to `to_key` in `view` and
/// `to` receives it.
async fn assert_delivers(
    from: &Cliquenet<TestTypes>,
    from_key: BLSPubKey,
    to: &mut Cliquenet<TestTypes>,
    to_key: BLSPubKey,
    view: u64,
) {
    let message = send(from, from_key, to_key, view);

    let received = timeout(Duration::from_secs(10), to.receive())
        .await
        .unwrap_or_else(|_| panic!("no message arrived for {to_key} in view {view}"))
        .unwrap();
    assert_eq!(received, message.into_unchecked());
}

/// `from`, signing as `from_key`, unicasts a message to `to_key` in `view`.
fn send(
    from: &Cliquenet<TestTypes>,
    from_key: BLSPubKey,
    to_key: BLSPubKey,
    view: u64,
) -> Message<TestTypes, Validated> {
    let view = ViewNumber::new(view);
    let request = PayloadFetchMessage::Req(Request::new(view, PayloadRequestBody));
    let message = Message {
        sender: from_key,
        message_type: MessageType::PayloadFetch(request),
    };
    from.sender().unicast(view, &to_key, &message).unwrap();
    message
}
