//! Integration tests for the upgrade sub-protocol.
//!
//! With `TEST_UPGRADE_CONSTANTS`, an upgrade proposed at view `v` must decide
//! by `v + 10` and activates (flips the wire format and header version) at
//! `v + 20`.

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use hotshot_types::upgrade_config::UpgradeConfig;
use versions::{NEW_PROTOCOL_VERSION, Upgrade, version};

use crate::{
    message::{ConsensusMessage, MessageType},
    tests::common::{
        runner::{NodeAction, NodeChange, TestRunner},
        views,
    },
};

fn upgrade() -> Upgrade {
    Upgrade::new(NEW_PROTOCOL_VERSION, version(0, 7))
}

/// Proposing open for views [start, stop); voting and times unrestricted.
fn window(start: u64, stop: u64) -> UpgradeConfig {
    UpgradeConfig {
        start_proposing_view: start,
        stop_proposing_view: stop,
        start_voting_view: 0,
        stop_voting_view: u64::MAX,
        start_proposing_time: 0,
        stop_proposing_time: u64::MAX,
        start_voting_time: 0,
        stop_voting_time: u64::MAX,
    }
}

async fn assert_upgraded(runner: &TestRunner) {
    for (i, lock) in runner.node_locks().iter().enumerate() {
        let lock = lock.as_ref().expect("node ran");
        let cert = lock
            .decided_upgrade_cert()
            .unwrap_or_else(|| panic!("node {i} did not decide the upgrade"));
        assert_eq!(cert.data.old_version, NEW_PROTOCOL_VERSION);
        assert_eq!(cert.data.new_version, version(0, 7));
    }
    for (i, storage) in runner.node_storages().iter().enumerate() {
        assert!(
            storage.decided_upgrade_certificate().await.is_some(),
            "node {i} did not persist the decided upgrade certificate"
        );
    }
}

fn assert_not_upgraded(runner: &TestRunner) {
    for (i, lock) in runner.node_locks().iter().enumerate() {
        let lock = lock.as_ref().expect("node ran");
        assert!(
            lock.decided_upgrade_cert().is_none(),
            "node {i} decided an upgrade that should have expired"
        );
    }
}

/// The upgrade is proposed, voted, attached, decided, and the network keeps
/// deciding leaves past the activation view (view 34 at the latest).
#[tokio::test(flavor = "multi_thread")]
async fn test_upgrade_happy_path() {
    let mut runner = TestRunner::builder()
        .num_nodes(5)
        .target_decisions(45)
        .upgrade(upgrade())
        .upgrade_config(window(5, 15))
        .build();
    runner.run().await.unwrap();
    assert_upgraded(&runner).await;
}

/// The proposals carrying the upgrade certificate are dropped network-wide a
/// couple of times (those views time out); a later leader re-attaches the
/// certificate and the upgrade still decides before its deadline.
#[tokio::test(flavor = "multi_thread")]
async fn test_upgrade_reattached_after_dropped_carrier() {
    let dropped = Arc::new(AtomicU64::new(0));
    let mut runner = TestRunner::builder()
        .num_nodes(5)
        .target_decisions(45)
        .upgrade(upgrade())
        .upgrade_config(window(5, 15))
        // Which views time out depends on which leaders held the certificate.
        .tolerated_failed_views(views(5..=20))
        .drop_inbound(Arc::new(move |_node, message| {
            let MessageType::Consensus(ConsensusMessage::Proposal(p)) = &message.message_type
            else {
                return false;
            };
            if p.proposal.data.upgrade_certificate.is_none() {
                return false;
            }
            // The first two carrying proposals, each dropped at all 5 nodes.
            dropped.fetch_add(1, Ordering::Relaxed) < 2 * 5
        }))
        .build();
    runner.run().await.unwrap();
    assert_upgraded(&runner).await;
}

/// Every proposal carrying the upgrade certificate is dropped until past the
/// certificate's decide deadline: the upgrade cleanly fails to activate and
/// consensus stays live on the base version.
#[tokio::test(flavor = "multi_thread")]
async fn test_upgrade_expires_without_activation() {
    let mut runner = TestRunner::builder()
        .num_nodes(5)
        .target_decisions(20)
        .upgrade(upgrade())
        // A single upgrade proposal at view 5, expiring at view 15.
        .upgrade_config(window(5, 6))
        .view_timeout(std::time::Duration::from_secs(2))
        .tolerated_failed_views(views(5..=18))
        .drop_inbound(Arc::new(|_node, message| {
            matches!(
                &message.message_type,
                MessageType::Consensus(ConsensusMessage::Proposal(p))
                    if p.proposal.data.upgrade_certificate.is_some()
            )
        }))
        .build();
    runner.run().await.unwrap();
    assert_not_upgraded(&runner);
}

/// A node restarted (with persistent storage) after the upgrade decided but
/// around activation restores the decided certificate from storage and keeps
/// up with the upgraded network.
#[tokio::test(flavor = "multi_thread")]
async fn test_upgrade_restart_across_activation() {
    let mut runner = TestRunner::builder()
        .num_nodes(5)
        .target_decisions(45)
        .upgrade(upgrade())
        .upgrade_config(window(5, 15))
        .persistent_storage(true)
        .node_changes(vec![(
            20,
            vec![NodeChange {
                idx: 1,
                action: NodeAction::Restart,
            }],
        )])
        .tolerated_failed_views(views(18..=30))
        .build();
    runner.run().await.unwrap();
    assert_upgraded(&runner).await;
}
