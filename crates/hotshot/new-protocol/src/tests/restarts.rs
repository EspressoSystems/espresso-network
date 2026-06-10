use std::time::Duration;

use hotshot::types::BLSPubKey;
use hotshot_example_types::{node_types::TestTypes, storage_types::TestStorage};
use hotshot_types::{
    data::ViewNumber,
    event::HotShotAction,
    traits::{metrics::NoMetrics, signature_key::SignatureKey, storage::Storage as _},
};

use crate::{
    consensus::ConsensusOutput,
    helpers::test_upgrade_lock,
    message::{ConsensusMessage, Message, MessageType},
    network::cliquenet::Cliquenet,
    tests::common::{
        assertions::node_index_for_key,
        coordinator_builder::build_test_coordinator,
        runner::{NodeAction, NodeChange, TestRunner},
        utils::{TestData, mock_membership_with_client_and_storage},
        views,
    },
};

/// A cliquenet instance with no peers, for coordinator tests that never
/// expect a message to be delivered.
async fn lone_network(node_index: u64) -> Cliquenet<TestTypes> {
    let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], node_index);
    let keypair = hotshot_types::x25519::Keypair::derive_from::<BLSPubKey>(&private_key)
        .expect("keypair derivation should succeed");
    let port = test_utils::reserve_tcp_port().expect("OS should have ephemeral ports available");
    let addr = hotshot_types::addr::NetAddr::Inet(std::net::Ipv4Addr::LOCALHOST.into(), port);
    Cliquenet::create(
        "test-restarts",
        public_key,
        keypair,
        addr,
        vec![],
        test_upgrade_lock(),
        Box::new(NoMetrics),
    )
    .await
    .expect("cliquenet creation should succeed")
}

/// A node restarting past its anchor has no parent proposal for the resumed
/// view. `start()` must not panic — even when the node is the leader of the
/// entered view — and the node must resume at the persisted restart view.
#[tokio::test]
async fn restart_past_anchor_as_leader_does_not_panic() {
    // Pick the leader of view 5 so `start()` takes the leader path, where
    // the missing parent proposal used to be an `expect`.
    let test_data = TestData::new(5).await;
    let leader_of_view_5 = test_data.views[4].leader_public_key;
    let node_index = node_index_for_key(&leader_of_view_5);

    let storage = TestStorage::<TestTypes>::default();
    // Simulate a previous run that voted through view 4 → restart view 5.
    storage
        .record_action(ViewNumber::new(4), None, HotShotAction::Vote)
        .await
        .unwrap();

    let (membership, storage, client, _events) =
        mock_membership_with_client_and_storage(10, 100, leader_of_view_5, storage);
    let coordinator = build_test_coordinator(
        node_index,
        lone_network(node_index).await,
        membership,
        storage,
        client,
        100,
        Duration::from_secs(30),
        None,
    )
    .await;

    assert_eq!(
        coordinator.current_view(),
        ViewNumber::new(5),
        "node should resume at the persisted restart view"
    );
}

/// Sending vote1/vote2/proposal records the action to storage, so the next
/// restart resumes past those views.
#[tokio::test]
async fn actions_recorded_on_send() {
    let node_index = 0u64;
    let (public_key, _) = BLSPubKey::generated_from_seed_indexed([0; 32], node_index);
    let (membership, storage, client, _events) =
        mock_membership_with_client_and_storage(10, 100, public_key, TestStorage::default());
    let mut coordinator = build_test_coordinator(
        node_index,
        lone_network(node_index).await,
        membership,
        storage.clone(),
        client,
        100,
        Duration::from_secs(30),
        None,
    )
    .await;

    let test_data = TestData::new(3).await;

    // Vote1 for view 1: advances both watermarks.
    let Message {
        message_type: MessageType::Consensus(ConsensusMessage::Vote1(vote1)),
        ..
    } = test_data.views[0].vote1_input(node_index)
    else {
        panic!("expected a vote1 message");
    };
    coordinator
        .process_consensus_output(ConsensusOutput::SendVote1(vote1))
        .unwrap();
    wait_for_recorded_views(&storage, ViewNumber::new(1), ViewNumber::new(2)).await;

    // Proposal for view 2: advances the actioned view but not the restart view.
    coordinator
        .process_consensus_output(ConsensusOutput::SendProposal(
            test_data.views[1].proposal.clone(),
        ))
        .unwrap();
    wait_for_recorded_views(&storage, ViewNumber::new(2), ViewNumber::new(2)).await;

    // Vote2 for view 3: advances both watermarks again.
    let Message {
        message_type: MessageType::Consensus(ConsensusMessage::Vote2(vote2)),
        ..
    } = test_data.views[2].vote2_input(node_index)
    else {
        panic!("expected a vote2 message");
    };
    coordinator
        .process_consensus_output(ConsensusOutput::SendVote2(vote2))
        .unwrap();
    wait_for_recorded_views(&storage, ViewNumber::new(3), ViewNumber::new(4)).await;
}

/// Poll the storage until the recorded action watermarks match; recording is
/// fire-and-forget so the write lands shortly after the send.
async fn wait_for_recorded_views(
    storage: &TestStorage<TestTypes>,
    last_actioned: ViewNumber,
    restart: ViewNumber,
) {
    for _ in 0..100 {
        if storage.last_actioned_view().await == last_actioned
            && storage.restart_view().await == restart
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!(
        "recorded views not observed: expected last_actioned={last_actioned} restart={restart}, \
         got last_actioned={} restart={}",
        storage.last_actioned_view().await,
        storage.restart_view().await,
    );
}

/// 5 nodes, node 2 restarts at view 8 keeping its storage. The runner
/// verifies — for every test — that no node sends vote1 twice for the same
/// view across restarts (`verify_no_revotes`); this test exercises that
/// check with a restart early in the run, away from epoch boundaries.
#[tokio::test(flavor = "multi_thread")]
async fn restart_after_vote_does_not_revote() {
    TestRunner::builder()
        .num_nodes(5)
        .target_decisions(20)
        .epoch_height(100)
        .node_changes(vec![(
            8,
            vec![NodeChange {
                idx: 2,
                action: NodeAction::Restart,
            }],
        )])
        .build()
        .run()
        .await
        .unwrap();
}

// ---------------------------------------------------------------------------
// Restart with persisted actions (with epochs)
// ---------------------------------------------------------------------------

/// 10 nodes, 1 restarts at view 11, epoch_height=10.
///
/// The restarted node keeps its action storage (so it resumes at its
/// persisted restart view) but no chain state. Verifies that it catches up
/// and participates across epoch boundaries while the rest of the network
/// continues.
#[tokio::test(flavor = "multi_thread")]
async fn restart_one_node_with_epochs() {
    TestRunner::builder()
        .num_nodes(10)
        .target_decisions(30)
        .epoch_height(10)
        .node_changes(vec![(
            11,
            vec![NodeChange {
                idx: 5,
                action: NodeAction::Restart,
            }],
        )])
        .build()
        .run()
        .await
        .unwrap();
}

/// 10 nodes, f=3 restart simultaneously at view 15, epoch_height=10.
///
/// Verifies the network recovers when the maximum tolerable number of
/// nodes restart at once.  Views where a restarting node is leader during
/// catchup are expected to fail.
#[tokio::test(flavor = "multi_thread")]
async fn restart_f_nodes_with_epochs() {
    // Nodes 7, 8, 9 restart at view 13.  Their first leader views after
    // restart are 17(7), 18(8), 19(9) — they should propose while catching
    // up.  Views 27(7), 28(8), 29(9) are in the epoch transition zone and
    // may also fail if the DRB hasn't arrived yet, but this tests that it does.
    TestRunner::builder()
        .num_nodes(10)
        .target_decisions(30)
        .epoch_height(10)
        .node_changes(vec![(
            11,
            vec![
                NodeChange {
                    idx: 7,
                    action: NodeAction::Restart,
                },
                NodeChange {
                    idx: 8,
                    action: NodeAction::Restart,
                },
                NodeChange {
                    idx: 9,
                    action: NodeAction::Restart,
                },
            ],
        )])
        .build()
        .run()
        .await
        .unwrap();
}

/// 5 nodes all restart simultaneously at view 15, epoch_height=10.
///
/// Every node keeps its action storage, so each resumes past the views it
/// acted in and the runner verifies nothing is vote1'd twice. The test
/// runner does not persist chain state (anchor leaf, certificates), so the
/// network converges via timeouts and then rebuilds a fresh chain — anchored
/// at genesis via the locked certificate — at the resumed view numbers.
/// Views decided before the restart stay decided; the runner detects the
/// restart-all and tolerates the undecided convergence window.
///
/// TODO: Once the test runner persists chain state, the restarted nodes
/// should resume from their last decided state instead of re-anchoring the
/// chain at genesis.
#[tokio::test(flavor = "multi_thread")]
async fn restart_all_nodes_with_epochs() {
    TestRunner::builder()
        .num_nodes(5)
        .target_decisions(30)
        .epoch_height(10)
        .node_changes(vec![(
            15,
            (0..5)
                .map(|i| NodeChange {
                    idx: i,
                    action: NodeAction::Restart,
                })
                .collect(),
        )])
        .build()
        .run()
        .await
        .unwrap();
}

// ---------------------------------------------------------------------------
// Late start (with epochs)
// ---------------------------------------------------------------------------

/// 10 nodes, 1 starts late at view 20, epoch_height=10.
///
/// Node 9 is initially offline.  At view 20 (after 2 epoch transitions)
/// it joins the network from genesis and must catch up.  Views where
/// node 9 was leader while offline are expected to fail.
#[tokio::test(flavor = "multi_thread")]
async fn late_start_one_node_with_epochs() {
    // Node 9 is leader for 9, 19, 29, 39.  It rejoins right after
    // it would have led for view 39 and should propose at view 49 in epoch 3
    TestRunner::builder()
        .num_nodes(10)
        .target_decisions(50)
        .max_runtime(Duration::from_secs(500))
        .epoch_height(15)
        .expected_failed_views(views([9, 19, 29, 39]))
        .node_changes(vec![(
            39,
            vec![NodeChange {
                idx: 9,
                action: NodeAction::Start,
            }],
        )])
        .build()
        .run()
        .await
        .unwrap();
}

/// 10 nodes, f=3 start late at view 20, epoch_height=10.
///
/// Nodes 7-9 are initially offline (the network runs with 7 nodes, the
/// minimum quorum for n=10).  At view 23 they all join simultaneously.
#[tokio::test(flavor = "multi_thread")]
async fn late_start_f_nodes_with_epochs() {
    TestRunner::builder()
        .num_nodes(10)
        .target_decisions(50)
        .max_runtime(Duration::from_secs(500))
        .epoch_height(15)
        .expected_failed_views(views([7, 8, 9, 17, 18, 19, 27, 28, 29, 37, 38, 39]))
        .node_changes(vec![
            (
                37,
                vec![
                    NodeChange {
                        idx: 7,
                        action: NodeAction::Start,
                    },
                    NodeChange {
                        idx: 8,
                        action: NodeAction::Start,
                    },
                ],
            ),
            (
                39,
                vec![NodeChange {
                    idx: 9,
                    action: NodeAction::Start,
                }],
            ),
        ])
        .build()
        .run()
        .await
        .unwrap();
}
