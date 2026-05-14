//! Unit tests for the cutover bridging API.

use hotshot::types::{BLSPubKey, SignatureKey};
use hotshot_example_types::node_types::TestTypes;
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    simple_vote::UpgradeProposalData,
    stake_table::StakeTableEntries,
    vote::{Certificate, HasViewNumber},
};
use versions::{CLIQUENET_VERSION, version};

use crate::{
    helpers::test_upgrade_lock,
    tests::common::utils::{ConsensusHarness, TestData},
};

#[tokio::test]
async fn test_seed_pre_cutover_leaves_populates_state() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(3).await;

    let leaves: Vec<_> = test_data
        .views
        .iter()
        .take(2)
        .map(|v| v.leaf.clone())
        .collect();
    harness.consensus.seed_pre_cutover_leaves(leaves);

    assert!(
        harness
            .consensus
            .proposal_at(test_data.views[0].view_number)
            .is_some(),
        "seeded view 0 should have a proposal",
    );
    assert!(
        harness
            .consensus
            .proposal_at(test_data.views[1].view_number)
            .is_some(),
        "seeded view 1 should have a proposal",
    );

    let parent_view_of_first = test_data.views[0].leaf.justify_qc().view_number();
    assert!(
        harness.consensus.cert1_at(parent_view_of_first).is_some(),
        "Cert1 for parent of oldest seeded leaf should be registered",
    );

    let parent_view_of_second = test_data.views[1].leaf.justify_qc().view_number();
    assert!(
        harness.consensus.cert1_at(parent_view_of_second).is_some(),
        "Cert1 for first seeded leaf (= second's parent) should be registered",
    );
}

#[tokio::test]
async fn test_register_proposal_justify_qc_idempotent() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(2).await;

    let qc1 = test_data.views[0].cert1.clone();
    let qc1_view = qc1.view_number();

    harness.consensus.register_proposal_justify_qc(&qc1);
    let after_first = harness
        .consensus
        .cert1_at(qc1_view)
        .cloned()
        .expect("Cert1 should be registered");

    harness.consensus.register_proposal_justify_qc(&qc1);
    let after_second = harness
        .consensus
        .cert1_at(qc1_view)
        .cloned()
        .expect("Cert1 should still be registered");

    assert_eq!(
        after_first.signatures, after_second.signatures,
        "Cert1 entry should not be replaced by a second register call",
    );
}

#[tokio::test]
async fn test_set_pre_cutover_anchor_only_advances() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(3).await;

    let starting_view = harness.consensus.last_decided_view();

    let advanced_leaf = test_data.views[1].leaf.clone();
    let advanced_view = advanced_leaf.view_number();
    assert!(advanced_view > starting_view);
    harness.consensus.set_pre_cutover_anchor(advanced_leaf);
    assert_eq!(harness.consensus.last_decided_view(), advanced_view);

    let earlier_leaf = test_data.views[0].leaf.clone();
    harness.consensus.set_pre_cutover_anchor(earlier_leaf);
    assert_eq!(
        harness.consensus.last_decided_view(),
        advanced_view,
        "anchor should not regress to earlier view",
    );
}

/// Multi-node E2E cutover over real Cliquenet.
#[tokio::test(flavor = "multi_thread")]
async fn five_nodes_decide_after_pre_cutover_seed() {
    use crate::tests::common::runner::{PreCutoverSeed, TestRunner};

    // epoch_height=100 keeps seeded leaves' `with_epoch` consistent with
    // the synthesized proposals' round-trip.
    let test_data =
        crate::tests::common::utils::TestData::new_with_epoch_height_and_num_nodes(2, 100, 5).await;

    let anchor = test_data.views[0].leaf.clone();
    let undecided = vec![test_data.views[1].leaf.clone()];
    let high_qc = test_data.views[1].cert1.clone();

    let seed = PreCutoverSeed {
        decided_anchor: anchor,
        undecided,
        high_qc,
        cutover_view: ViewNumber::new(3),
    };

    TestRunner::builder()
        .pre_cutover_seed(seed)
        .target_decisions(10)
        .build()
        .run()
        .await
        .expect("network should decide past the pre-cutover boundary");
}

/// Cutover with a real quorum-signed `UpgradeCertificate` in the seed.
#[tokio::test(flavor = "multi_thread")]
async fn upgrade_certificate_handover() {
    use crate::tests::common::runner::{PreCutoverSeed, TestRunner};

    let num_nodes = 5;
    let num_views = 2;
    let pre_cliquenet = version(CLIQUENET_VERSION.major, CLIQUENET_VERSION.minor - 1);
    let upgrade_data = UpgradeProposalData {
        old_version: pre_cliquenet,
        new_version: CLIQUENET_VERSION,
        decide_by: ViewNumber::new(1),
        new_version_hash: vec![0u8; 12],
        old_version_last_view: ViewNumber::new(num_views as u64),
        new_version_first_view: ViewNumber::new(num_views as u64 + 1),
    };

    let upgrade_view = ViewNumber::new(num_views as u64);
    let test_data = TestData::new_with_upgrade(
        num_views,
        100,
        num_nodes,
        Some((upgrade_view, upgrade_data.clone())),
    )
    .await;

    let upgraded_leaf = &test_data.views[1].leaf;
    let cert_opt = upgraded_leaf.upgrade_certificate();
    let cert = cert_opt
        .as_ref()
        .expect("upgrade certificate should be embedded in legacy chain");
    assert_eq!(cert.data.new_version, CLIQUENET_VERSION);
    assert_eq!(cert.data.old_version, pre_cliquenet);
    assert_eq!(
        cert.data.new_version_first_view,
        ViewNumber::new(num_views as u64 + 1)
    );

    let public_key = BLSPubKey::generated_from_seed_indexed([0u8; 32], 0).0;
    let (membership, ..) =
        crate::tests::common::utils::mock_membership_with_client(num_nodes, 100, public_key).await;
    let epoch_membership = membership
        .membership_for_epoch(Some(EpochNumber::genesis()))
        .unwrap();
    let entries = StakeTableEntries::from_iter(epoch_membership.stake_table()).0;
    let threshold = epoch_membership.upgrade_threshold();
    cert.is_valid_cert(&entries, threshold, &test_upgrade_lock::<TestTypes>())
        .expect("upgrade certificate should verify against the validator stake table");

    let anchor = test_data.views[0].leaf.clone();
    let undecided = vec![test_data.views[1].leaf.clone()];
    let high_qc = test_data.views[1].cert1.clone();

    let seed = PreCutoverSeed {
        decided_anchor: anchor,
        undecided,
        high_qc,
        cutover_view: upgrade_data.new_version_first_view,
    };

    TestRunner::builder()
        .num_nodes(num_nodes)
        .pre_cutover_seed(seed)
        .target_decisions(10)
        .build()
        .run()
        .await
        .expect("new protocol should decide past the upgrade boundary");
}
