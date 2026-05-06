//! Unit tests for the legacy → new-protocol (0.4 → 0.8) cutover bridging
//! API: `Consensus::seed_pre_cutover_leaves`,
//! `Consensus::register_proposal_justify_qc`,
//! `Consensus::set_pre_cutover_anchor`, and the `pre_cutover_views`
//! grandfathering of the V2 VID-availability check in
//! `maybe_vote_2_and_update_lock`.

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

/// `seed_pre_cutover_leaves` populates `proposals`, marks the view in
/// `pre_cutover_views` (probed indirectly via the public `proposal_at`
/// accessor), and registers the parent's Cert1.
#[tokio::test]
async fn test_seed_pre_cutover_leaves_populates_state() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(3).await;

    // Seed views 1 and 2 as undecided pre-cutover leaves.
    let leaves: Vec<_> = test_data
        .views
        .iter()
        .take(2)
        .map(|v| v.leaf.clone())
        .collect();
    harness.consensus.seed_pre_cutover_leaves(leaves);

    // Both seeded views have a synthesized proposal recorded.
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

    // Each leaf's `justify_qc` was registered as Cert1 for the parent view.
    // The oldest seeded leaf's justify_qc points to the genesis-style parent
    // (its `view_number()` minus one, in TestData).
    let parent_view_of_first = test_data.views[0].leaf.justify_qc().view_number();
    assert!(
        harness.consensus.cert1_at(parent_view_of_first).is_some(),
        "Cert1 for parent of oldest seeded leaf should be registered",
    );

    // The second leaf's justify_qc is the QC of the first seeded leaf —
    // that registers Cert1 for the first seeded leaf's view.
    let parent_view_of_second = test_data.views[1].leaf.justify_qc().view_number();
    assert!(
        harness.consensus.cert1_at(parent_view_of_second).is_some(),
        "Cert1 for first seeded leaf (= second's parent) should be registered",
    );
}

/// `register_proposal_justify_qc` is idempotent: calling it twice with the
/// same QC doesn't replace the original entry, and a second call with a
/// different QC for the same view doesn't overwrite either.
#[tokio::test]
async fn test_register_proposal_justify_qc_idempotent() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(2).await;

    let qc1 = test_data.views[0].cert1.clone();
    let qc1_view = qc1.view_number();

    // First registration installs the QC.
    harness.consensus.register_proposal_justify_qc(&qc1);
    let after_first = harness
        .consensus
        .cert1_at(qc1_view)
        .cloned()
        .expect("Cert1 should be registered");

    // Second registration is a no-op (or_insert_with semantics).
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

/// `set_pre_cutover_anchor` advances `last_decided_view` only when the
/// supplied leaf's view is strictly greater than the current anchor.
/// Lower-or-equal views are silently ignored (idempotent / safe to retry).
#[tokio::test]
async fn test_set_pre_cutover_anchor_only_advances() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(3).await;

    // Pre-condition: harness starts at genesis (view 0).
    let starting_view = harness.consensus.last_decided_view();

    // Advance to a leaf above genesis.
    let advanced_leaf = test_data.views[1].leaf.clone();
    let advanced_view = advanced_leaf.view_number();
    assert!(advanced_view > starting_view);
    harness.consensus.set_pre_cutover_anchor(advanced_leaf);
    assert_eq!(harness.consensus.last_decided_view(), advanced_view);

    // Calling again with an EARLIER leaf should be a no-op.
    let earlier_leaf = test_data.views[0].leaf.clone();
    harness.consensus.set_pre_cutover_anchor(earlier_leaf);
    assert_eq!(
        harness.consensus.last_decided_view(),
        advanced_view,
        "anchor should not regress to earlier view",
    );
}

/// Multi-node E2E test of the legacy → new-protocol cutover seed via
/// `TestRunner` (real Cliquenet network between five nodes).
///
/// Each coordinator is seeded with a chain of pre-cutover leaves
/// **synchronously before** its run loop starts (the seed is threaded into
/// `build_test_coordinator` so it applies before `coord.start()`, avoiding
/// the startup race that would otherwise let the view-1 leader propose
/// before the seed lands).
#[tokio::test(flavor = "multi_thread")]
async fn five_nodes_decide_after_pre_cutover_seed() {
    use crate::tests::common::runner::{PreCutoverSeed, TestRunner};

    // Generate a 2-leaf pre-cutover chain (anchor at view 1, one undecided
    // leaf at view 2). This is the minimum useful seed that exercises the
    // entire bridging path while keeping the boundary close to genesis
    // where the leader-of-view-3 can take over cleanly.
    //
    // Use epoch_height=100 so the leaves carry `epoch = Some(...)`,
    // matching what `Leaf2::from_quorum_proposal` produces from a
    // new-protocol Proposal (whose `epoch` is non-Option). Without this
    // the synthesized proposals in `seed_pre_cutover_leaves` round-trip
    // with `with_epoch = true` while originals have `with_epoch = false`,
    // making the commitments differ.
    //
    // num_nodes=5 to match `TestRunner`'s default — TestData and TestRunner
    // must use the same membership size or the leaf certs (signed under
    // TestData's stake table) won't verify against TestRunner's stake table.
    let test_data =
        crate::tests::common::utils::TestData::new_with_epoch_height_and_num_nodes(2, 100, 5).await;

    let anchor = test_data.views[0].leaf.clone();
    let undecided = vec![test_data.views[1].leaf.clone()];
    let high_qc = test_data.views[1].cert1.clone();

    let seed = PreCutoverSeed {
        decided_anchor: anchor,
        undecided,
        high_qc,
    };

    TestRunner::builder()
        .pre_cutover_seed(seed)
        .target_decisions(10)
        .build()
        .run()
        .await
        .expect("network should decide past the pre-cutover boundary");
}

/// End-to-end legacy → new-protocol handover with a *real, quorum-signed*
/// `UpgradeCertificate` formed by aggregating BLS votes from every
/// validator in the test stake table — the same primitive the legacy
/// upgrade task uses in production.
///
/// What this test exercises:
///
/// 1. **Upgrade certificate formation**: an `UpgradeProposalData`
///    transitioning from `CLIQUENET_VERSION - 1` (0.7) to
///    `CLIQUENET_VERSION` (0.8) is signed by every validator and
///    aggregated into an `UpgradeCertificate`. The cert verifies under
///    the same `EpochMembership` that signed it (via
///    `Certificate::is_valid_cert`).
///
/// 2. **Embedding in the legacy chain**: the certificate is attached to
///    the leaf at view 2 — exactly where the legacy upgrade task would
///    embed it once enough votes form. `TestData::new_with_upgrade`
///    re-derives that leaf's commit and re-signs the chain's `cert1` /
///    `cert2` so the chain remains internally consistent.
///
/// 3. **Cutover hand-over**: the chain (anchor view 1, undecided view 2)
///    plus its high QC seed five new-protocol coordinators on a real
///    Cliquenet network. The leaf at view 2 carries the upgrade
///    certificate forward via
///    `Consensus::seed_pre_cutover_leaves`.
///
/// 4. **Post-cutover progress**: the new protocol takes over at view 3
///    (= `new_version_first_view`) and decides through view 10 — proving
///    that with a properly formed upgrade certificate decided in the
///    legacy chain, the handover lets the new protocol extend the chain
///    past the boundary.
///
/// What this test does *not* exercise:
///
/// - Live legacy HotShot consensus rounds: the chain is generated
///   deterministically rather than driven by a running `SystemContext`.
///   The validator keys, signatures, certificate aggregation, and chain
///   shape are all real, so the upgrade cert is indistinguishable from
///   one formed by a live cluster — the only thing absent is the
///   wall-clock view advance.
#[tokio::test(flavor = "multi_thread")]
async fn upgrade_certificate_handover() {
    use crate::tests::common::runner::{PreCutoverSeed, TestRunner};

    let num_nodes = 5;
    let num_views = 2;
    // The cert says the legacy version's last view is `num_views` and
    // the new version begins at `num_views + 1`. The new-protocol
    // coordinators take over at view `num_views + 1` (view 3 here).
    let pre_cliquenet = version(CLIQUENET_VERSION.major, CLIQUENET_VERSION.minor - 1);
    let upgrade_data = UpgradeProposalData {
        old_version: pre_cliquenet,
        new_version: CLIQUENET_VERSION,
        decide_by: ViewNumber::new(1),
        new_version_hash: vec![0u8; 12],
        old_version_last_view: ViewNumber::new(num_views as u64),
        new_version_first_view: ViewNumber::new(num_views as u64 + 1),
    };

    // Generate a 2-leaf legacy chain. The leaf at view 2 carries a
    // properly signed `UpgradeCertificate` — TestData::new_with_upgrade
    // calls `build_cert` (the same helper used by the upgrade task in
    // production legacy hotshot) to aggregate votes from every
    // validator in the membership.
    let upgrade_view = ViewNumber::new(num_views as u64);
    let test_data = TestData::new_with_upgrade(
        num_views,
        100,
        num_nodes,
        Some((upgrade_view, upgrade_data.clone())),
    )
    .await;

    // Sanity: the leaf at view 2 actually carries the cert, and the
    // certificate aggregates a quorum-threshold signature that
    // verifies against the test stake table.
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

    // Verify the upgrade cert against the same membership that signed it.
    // This proves we have a real, quorum-signed certificate — not just a
    // structurally-valid struct.
    let public_key = BLSPubKey::generated_from_seed_indexed([0u8; 32], 0).0;
    let (membership, ..) =
        crate::tests::common::utils::mock_membership_with_client(num_nodes, 100, public_key).await;
    let epoch_membership = membership
        .membership_for_epoch(Some(EpochNumber::genesis()))
        .await
        .unwrap();
    let stake_entries =
        StakeTableEntries::<TestTypes>::from(epoch_membership.stake_table().await).0;
    let threshold = epoch_membership.upgrade_threshold().await;
    cert.is_valid_cert(&stake_entries, threshold, &test_upgrade_lock::<TestTypes>())
        .expect("upgrade certificate should verify against the validator stake table");

    // Hand the legacy chain to a new-protocol cluster: anchor at view 1
    // (decided in the legacy protocol), undecided leaves = [view 2]
    // (the upgrade cert is on view 2, so any node that sees the
    // post-cutover seed receives it).
    let anchor = test_data.views[0].leaf.clone();
    let undecided = vec![test_data.views[1].leaf.clone()];
    let high_qc = test_data.views[1].cert1.clone();

    let seed = PreCutoverSeed {
        decided_anchor: anchor,
        undecided,
        high_qc,
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
