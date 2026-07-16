//! Unit tests for the cutover bridging API.

use std::{collections::BTreeMap, num::NonZeroUsize, sync::Arc, time::Duration};

use committable::Committable;
use hotshot::types::{BLSPubKey, SignatureKey};
use hotshot_example_types::{node_types::TestTypes, state_types::TestValidatedState};
use hotshot_types::{
    data::{EpochNumber, Leaf2, ViewNumber},
    event::{Event, EventType},
    message::UpgradeLock,
    simple_certificate::UpgradeCertificate,
    simple_vote::UpgradeProposalData,
    stake_table::StakeTableEntries,
    vote::{Certificate, HasViewNumber},
};
use versions::{NEW_PROTOCOL_VERSION, version};

use crate::{
    client::{ClientApi, ClientRequest, CoordinatorClient},
    consensus::PreCutoverSeed,
    cutover::forward_legacy_high_qc,
    helpers::test_upgrade_lock,
    tests::common::utils::{ConsensusHarness, TestData, TestView},
};

/// Build a `PreCutoverSeed` from leaves, using `TestValidatedState::default()`
/// for every seeded view. Mirrors what production seed extraction does,
/// just with a trivial state.
fn test_seed(
    decided_anchor: Leaf2<TestTypes>,
    undecided: Vec<Leaf2<TestTypes>>,
    high_qc: Option<crate::message::Certificate1<TestTypes>>,
    cutover_view: ViewNumber,
) -> PreCutoverSeed<TestTypes> {
    let default_state = Arc::new(TestValidatedState::default());
    let mut validated_states = BTreeMap::new();
    validated_states.insert(decided_anchor.view_number(), default_state.clone());
    for leaf in &undecided {
        validated_states.insert(leaf.view_number(), default_state.clone());
    }
    PreCutoverSeed {
        decided_anchor,
        undecided,
        high_qc,
        validated_states,
        cutover_view,
    }
}

#[tokio::test]
async fn apply_pre_cutover_seed_populates_leaves_and_qcs() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(3).await;

    // anchor is decided (a marker, not a proposal); undecided leaves
    // must end up in `proposals`, and each leaf's justify_qc must end
    // up in `certs`.
    let anchor = test_data.views[0].leaf.clone();
    let undecided = vec![
        test_data.views[1].leaf.clone(),
        test_data.views[2].leaf.clone(),
    ];
    let seed = test_seed(anchor, undecided, None, ViewNumber::genesis());
    harness.consensus.apply_pre_cutover_seed(seed);

    for view_idx in [1, 2] {
        let view = test_data.views[view_idx].view_number;
        assert!(
            harness.consensus.proposal_at(view).is_some(),
            "undecided leaf at view {view} should have a proposal installed",
        );
        let parent_view = test_data.views[view_idx].leaf.justify_qc().view_number();
        assert!(
            harness.consensus.cert1_at(parent_view).is_some(),
            "Cert1 for parent of undecided leaf at view {view} should be registered",
        );
    }
}

#[tokio::test]
async fn apply_pre_cutover_seed_high_qc_is_idempotent() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(2).await;

    let qc1 = test_data.views[0].cert1.clone();
    let qc1_view = qc1.view_number();
    let anchor = test_data.views[0].leaf.clone();
    let seed = || {
        test_seed(
            anchor.clone(),
            Vec::new(),
            Some(qc1.clone()),
            ViewNumber::genesis(),
        )
    };

    harness.consensus.apply_pre_cutover_seed(seed());
    let after_first = harness
        .consensus
        .cert1_at(qc1_view)
        .cloned()
        .expect("Cert1 should be registered");

    harness.consensus.apply_pre_cutover_seed(seed());
    let after_second = harness
        .consensus
        .cert1_at(qc1_view)
        .cloned()
        .expect("Cert1 should still be registered");

    assert_eq!(
        after_first.signatures, after_second.signatures,
        "Cert1 entry should not be replaced by a second seed application",
    );
}

#[tokio::test]
async fn apply_pre_cutover_seed_anchor_only_advances() {
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(3).await;

    let starting_view = harness.consensus.last_decided_view();
    let advanced_leaf = test_data.views[1].leaf.clone();
    let advanced_view = advanced_leaf.view_number();
    assert!(advanced_view > starting_view);

    harness.consensus.apply_pre_cutover_seed(test_seed(
        advanced_leaf,
        Vec::new(),
        None,
        ViewNumber::genesis(),
    ));
    assert_eq!(harness.consensus.last_decided_view(), advanced_view);

    let earlier_leaf = test_data.views[0].leaf.clone();
    harness.consensus.apply_pre_cutover_seed(test_seed(
        earlier_leaf,
        Vec::new(),
        None,
        ViewNumber::genesis(),
    ));
    assert_eq!(
        harness.consensus.last_decided_view(),
        advanced_view,
        "anchor should not regress to earlier view",
    );
}

/// Multi-node E2E cutover over real Cliquenet.
#[tokio::test(flavor = "multi_thread")]
async fn five_nodes_decide_after_pre_cutover_seed() {
    use crate::tests::common::runner::TestRunner;

    // epoch_height=100 keeps seeded leaves' `with_epoch` consistent with
    // the synthesized proposals' round-trip.
    let test_data =
        crate::tests::common::utils::TestData::new_with_epoch_height_and_num_nodes(2, 100, 5).await;

    let anchor = test_data.views[0].leaf.clone();
    let undecided = vec![test_data.views[1].leaf.clone()];
    let high_qc = test_data.views[1].cert1.clone();
    let seed = test_seed(anchor, undecided, Some(high_qc), ViewNumber::new(3));

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
async fn upgrade_certificate_cutover() {
    use crate::tests::common::runner::TestRunner;

    let num_nodes = 5;
    let num_views = 2;
    let pre_cliquenet = version(NEW_PROTOCOL_VERSION.major, NEW_PROTOCOL_VERSION.minor - 1);
    let upgrade_data = UpgradeProposalData {
        old_version: pre_cliquenet,
        new_version: NEW_PROTOCOL_VERSION,
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
    assert_eq!(cert.data.new_version, NEW_PROTOCOL_VERSION);
    assert_eq!(cert.data.old_version, pre_cliquenet);
    assert_eq!(
        cert.data.new_version_first_view,
        ViewNumber::new(num_views as u64 + 1)
    );

    let public_key = BLSPubKey::generated_from_seed_indexed([0u8; 32], 0).0;
    let (membership, ..) = crate::tests::common::utils::mock_membership_with_client(
        num_nodes,
        100,
        public_key,
        Default::default(),
    );
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
    let seed = test_seed(
        anchor,
        undecided,
        Some(high_qc),
        upgrade_data.new_version_first_view,
    );

    TestRunner::builder()
        .num_nodes(num_nodes)
        .pre_cutover_seed(seed)
        .target_decisions(10)
        .build()
        .run()
        .await
        .expect("new protocol should decide past the upgrade boundary");
}

fn high_qc_event(view: &TestView) -> Event<TestTypes> {
    Event {
        view_number: view.view_number,
        event: EventType::LegacyHighQcFormed {
            qc: view.cert1.clone(),
        },
    }
}

/// Certificate deciding the upgrade to `NEW_PROTOCOL_VERSION`.
fn new_protocol_upgrade_cert() -> UpgradeCertificate<TestTypes> {
    let upgrade_data = UpgradeProposalData {
        old_version: version(0, 1),
        new_version: NEW_PROTOCOL_VERSION,
        decide_by: ViewNumber::genesis(),
        new_version_hash: Default::default(),
        old_version_last_view: ViewNumber::genesis(),
        new_version_first_view: ViewNumber::genesis(),
    };
    UpgradeCertificate::new(
        upgrade_data.clone(),
        upgrade_data.commit(),
        ViewNumber::genesis(),
        Default::default(),
        Default::default(),
    )
}

/// With the coordinator parked (never polling `next_request`), the forwarder
/// must keep draining the legacy event stream instead of blocking on a reply.
#[tokio::test]
async fn forward_legacy_high_qc_never_blocks_on_parked_coordinator() {
    let test_data = TestData::new(6).await;

    let client = CoordinatorClient::<TestTypes>::new(NonZeroUsize::new(4).unwrap());
    let client_api = client.handle().clone();

    let (legacy_tx, legacy_rx) = async_broadcast::broadcast::<Event<TestTypes>>(8);

    let upgrade_lock = test_upgrade_lock::<TestTypes>();
    upgrade_lock.set_decided_upgrade_cert(Some(new_protocol_upgrade_cert()));

    let forwarder = tokio::spawn(forward_legacy_high_qc(
        legacy_rx.deactivate(),
        client_api,
        upgrade_lock,
    ));

    for view in &test_data.views {
        legacy_tx
            .broadcast_direct(high_qc_event(view))
            .await
            .expect("legacy event channel should accept the send");
    }
    drop(legacy_tx);

    // Pre-fix: blocked awaiting the parked coordinator's reply, never exited.
    tokio::time::timeout(Duration::from_secs(5), forwarder)
        .await
        .expect(
            "forward_legacy_high_qc stalled: broadcast channel did not drain (parked coordinator \
             blocked submit_legacy_high_qc)",
        )
        .expect("forwarder task should not panic");

    drop(client);
}

/// Feed `views` as `LegacyHighQcFormed` events and wait for the forwarder to exit.
async fn run_high_qc_forwarder(
    api: ClientApi<TestTypes>,
    views: &[TestView],
    lock: UpgradeLock<TestTypes>,
) {
    let (legacy_tx, legacy_rx) = async_broadcast::broadcast::<Event<TestTypes>>(8);
    let forwarder = tokio::spawn(forward_legacy_high_qc(legacy_rx.deactivate(), api, lock));
    for view in views {
        legacy_tx
            .broadcast_direct(high_qc_event(view))
            .await
            .expect("legacy event channel should accept the send");
    }
    drop(legacy_tx);
    tokio::time::timeout(Duration::from_secs(5), forwarder)
        .await
        .expect("forwarder should drain and exit")
        .expect("forwarder task should not panic");
}

/// No request queues before the V0_6 upgrade is decided; one per event after.
#[tokio::test]
async fn forward_legacy_high_qc_gates_on_decided_upgrade() {
    let test_data = TestData::new(3).await;
    let mut client = CoordinatorClient::<TestTypes>::new(NonZeroUsize::new(8).unwrap());
    let upgrade_lock = test_upgrade_lock::<TestTypes>();

    run_high_qc_forwarder(
        client.handle().clone(),
        &test_data.views[..2],
        upgrade_lock.clone(),
    )
    .await;
    assert!(
        tokio::time::timeout(Duration::from_millis(50), client.next_request())
            .await
            .is_err(),
        "no request should be forwarded while the upgrade is undecided",
    );

    upgrade_lock.set_decided_upgrade_cert(Some(new_protocol_upgrade_cert()));
    let boundary_view = &test_data.views[2];
    run_high_qc_forwarder(
        client.handle().clone(),
        std::slice::from_ref(boundary_view),
        upgrade_lock,
    )
    .await;

    let request = tokio::time::timeout(Duration::from_millis(50), client.next_request())
        .await
        .expect("a request should be forwarded once the upgrade is decided")
        .expect("request channel should be open");
    assert!(
        matches!(
            &request,
            ClientRequest::SubmitLegacyHighQc { qc } if qc.view_number() == boundary_view.view_number
        ),
        "expected SubmitLegacyHighQc for view {}",
        boundary_view.view_number,
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(50), client.next_request())
            .await
            .is_err(),
        "exactly one request should be forwarded per event",
    );
}
