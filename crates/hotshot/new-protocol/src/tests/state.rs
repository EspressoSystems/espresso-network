use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use hotshot::traits::BlockPayload;
use hotshot_example_types::{
    block_types::{TestBlockPayload, TestMetadata},
    node_types::{TEST_VERSIONS, TestTypes},
    state_types::{TestInstanceState, TestValidatedState},
    testable_delay::{DelayConfig, DelayOptions, DelaySettings, SupportedTraitTypesForAsyncDelay},
};
use hotshot_types::{
    data::{Leaf2, ViewNumber, vid_commitment},
    traits::{
        EncodeBytes,
        block_contents::{BlockHeader, BuilderFee},
        signature_key::BuilderSignatureKey,
    },
    vote::{Certificate, HasViewNumber},
};

use crate::{
    helpers::{proposal_commitment, test_upgrade_lock},
    message::Proposal,
    state::{HeaderRequest, StateManager, StateManagerOutput, StateRequest},
    tests::common::utils::{TestData, TestView},
};

/// Build a StateRequest from a TestView.
fn make_state_request(view: &TestView) -> StateRequest<TestTypes> {
    let proposal: Proposal<TestTypes> = view.proposal.data.clone();
    StateRequest {
        view: view.view_number,
        parent_view: proposal.justify_qc.view_number(),
        epoch: view.epoch_number,
        block: BlockHeader::<TestTypes>::block_number(&proposal.block_header).into(),
        proposal: proposal.clone(),
        parent_commitment: proposal.justify_qc.data().leaf_commit,
        payload_size: 0,
        received_at: SystemTime::now(),
    }
}

/// Build a HeaderRequest from a TestView (as the parent).
fn make_header_request(
    parent_view: &TestView,
    target_view: ViewNumber,
) -> HeaderRequest<TestTypes> {
    let parent_proposal: Proposal<TestTypes> = parent_view.proposal.data.clone();
    let block = TestBlockPayload::genesis();
    let metadata = TestMetadata {
        num_transactions: 0,
    };
    let payload_commitment = vid_commitment(
        &block.encode(),
        &metadata.encode(),
        10,
        TEST_VERSIONS.test.base,
    );
    let builder_commitment =
        <TestBlockPayload as BlockPayload<TestTypes>>::builder_commitment(&block, &metadata);
    let (builder_key, builder_private_key) =
        <hotshot_types::signature_key::BuilderKey as BuilderSignatureKey>::generated_from_seed_indexed([0; 32], 0);
    let builder_signature =
        <hotshot_types::signature_key::BuilderKey as BuilderSignatureKey>::sign_builder_message(
            &builder_private_key,
            &[0u8],
        )
        .unwrap();
    HeaderRequest {
        view: target_view,
        epoch: parent_view.epoch_number,
        parent_proposal: parent_proposal.clone(),
        payload_commitment,
        builder_commitment,
        metadata,
        builder_fee: BuilderFee {
            fee_amount: 0,
            fee_account: builder_key,
            fee_signature: builder_signature,
        },
    }
}

async fn new_manager() -> StateManager<TestTypes> {
    new_manager_with_instance(TestInstanceState::default()).await
}

async fn new_manager_with_instance(instance: TestInstanceState) -> StateManager<TestTypes> {
    let mut manager = StateManager::new(Arc::new(instance.clone()), test_upgrade_lock());
    let genesis_state = TestValidatedState::default();
    // Must match the version used by `TestViewGenerator` (which produces the
    // proposals fed to the manager), otherwise the genesis leaf commitment
    // won't match the `parent_commitment` carried by the first proposal's
    // justify_qc.
    let genesis_leaf =
        Leaf2::<TestTypes>::genesis(&genesis_state, &instance, TEST_VERSIONS.vid2.base).await;
    manager.seed_state(ViewNumber::genesis(), Arc::new(genesis_state), genesis_leaf);
    manager
}

fn count_state_verified(events: &[StateManagerOutput<TestTypes>]) -> usize {
    events
        .iter()
        .filter(|e| {
            matches!(
                e,
                StateManagerOutput::State {
                    validated: true,
                    ..
                }
            )
        })
        .count()
}

fn count_header_created(events: &[StateManagerOutput<TestTypes>]) -> usize {
    events
        .iter()
        .filter(|e| {
            matches!(
                e,
                StateManagerOutput::Header {
                    header: Some(_),
                    ..
                }
            )
        })
        .count()
}

/// State request with missing parent inserts empty state (no output produced).
#[tokio::test]
async fn test_state_request_missing_parent_inserts_empty() {
    let mut manager =
        StateManager::new(Arc::new(TestInstanceState::default()), test_upgrade_lock());
    let test_data = TestData::new(2).await;

    // View 1's parent is genesis (view 0), which isn't seeded.
    manager.request_state(make_state_request(&test_data.views[0]));

    // No task was spawned, so next() should return None.
    assert!(
        manager.next().await.is_none(),
        "No output when parent is missing"
    );

    // But the empty state should be stored for the view.
    assert!(
        manager.validated_contains_view(test_data.views[0].view_number),
        "Empty state should be inserted for the view"
    );
}

/// A state request whose parent is entirely unknown is retried once the
/// parent is seeded from its header — the ordering where a first-of-epoch
/// proposal arrives before the boundary leaf's `EpochChangeMessage`.
#[tokio::test]
async fn test_state_request_missing_parent_retried_after_seed() {
    let mut manager =
        StateManager::new(Arc::new(TestInstanceState::default()), test_upgrade_lock());
    let test_data = TestData::new(3).await;

    // View 2 arrives while view 1 (its parent) is entirely unknown.
    manager.request_state(make_state_request(&test_data.views[1]));

    let view_1_commit = proposal_commitment(&test_data.views[0].proposal.data.clone());
    assert!(
        manager.pending_contains_commitment(&view_1_commit),
        "View 2 should be queued on its missing parent's commitment"
    );
    assert!(
        manager.validated_contains_view(test_data.views[1].view_number),
        "A from_header stub should be inserted for view 2"
    );

    // The parent becomes final (e.g. via a verified EpochChangeMessage):
    // seeding it must restart the queued request.
    manager.seed_from_header(test_data.views[0].proposal.data.clone());

    assert!(
        !manager.pending_contains_commitment(&view_1_commit),
        "the queued request should be consumed, not re-queued"
    );

    let output = manager.next().await.expect("view 2 should complete");
    assert!(
        matches!(
            output,
            StateManagerOutput::State {
                validated: true,
                ..
            }
        ),
        "View 2 should validate against the seeded parent state"
    );
}

/// A failed validation seeds a stub, so a queued child is validated instead of dropped.
#[tokio::test]
async fn test_failed_validation_seeds_stub_for_children() {
    let test_data = TestData::new(3).await;
    let failing_block =
        BlockHeader::<TestTypes>::block_number(&test_data.views[0].proposal.data.block_header);
    let mut manager = new_manager_with_instance(TestInstanceState {
        failing_block: Some(failing_block),
        ..Default::default()
    })
    .await;

    manager.request_state(make_state_request(&test_data.views[0]));
    manager.request_state(make_state_request(&test_data.views[1]));

    let first = manager.next().await.expect("view 1 should complete");
    assert!(
        matches!(
            &first,
            StateManagerOutput::State {
                response,
                validated: false,
            } if response.view == ViewNumber::new(1)
        ),
        "view 1 must fail validation"
    );
    assert!(
        manager.validated_contains_view(ViewNumber::new(1)),
        "a from_header stub must be seeded for the failed leaf"
    );

    let second = manager.next().await.expect("view 2 should complete");
    assert!(
        matches!(
            &second,
            StateManagerOutput::State {
                response,
                validated: true,
            } if response.view == ViewNumber::new(2)
        ),
        "view 2 must validate against the stub"
    );
}

/// Past the parent deadline a queued child validates against the parent's stub,
/// in parallel with it; the real parent state still lands.
#[tokio::test(start_paused = true)]
async fn test_child_proceeds_on_stub_after_parent_deadline() {
    let test_data = TestData::new(3).await;
    let validation_delay = Duration::from_millis(500);
    let mut delay_config = DelayConfig::default();
    delay_config.add_setting(
        SupportedTraitTypesForAsyncDelay::ValidatedState,
        &DelaySettings {
            delay_option: DelayOptions::Fixed,
            fixed_time_in_milliseconds: validation_delay.as_millis() as u64,
            ..Default::default()
        },
    );
    let mut manager = new_manager_with_instance(TestInstanceState::new(delay_config))
        .await
        .with_parent_deadline(Duration::from_millis(50));

    let started = tokio::time::Instant::now();
    manager.request_state(make_state_request(&test_data.views[0]));
    manager.request_state(make_state_request(&test_data.views[1]));

    let mut validated = Vec::new();
    for _ in 0..2 {
        match manager.next().await.expect("both views complete") {
            StateManagerOutput::State {
                response,
                validated: true,
            } => validated.push(response.view),
            other => panic!("unexpected output: {other:?}"),
        }
    }
    assert_eq!(
        validated,
        vec![ViewNumber::new(1), ViewNumber::new(2)],
        "both views validate, the parent first"
    );
    assert!(
        started.elapsed() < validation_delay * 2,
        "view 2 must not wait for view 1's full validation"
    );
    assert!(
        manager
            .get_state(ViewNumber::new(1))
            .is_some_and(|entry| entry.delta.is_some()),
        "the real parent state replaces the stub"
    );
}

/// Without the deadline elapsing, a child still waits for its parent.
#[tokio::test(start_paused = true)]
async fn test_child_waits_for_parent_within_deadline() {
    let test_data = TestData::new(3).await;
    let validation_delay = Duration::from_millis(500);
    let mut delay_config = DelayConfig::default();
    delay_config.add_setting(
        SupportedTraitTypesForAsyncDelay::ValidatedState,
        &DelaySettings {
            delay_option: DelayOptions::Fixed,
            fixed_time_in_milliseconds: validation_delay.as_millis() as u64,
            ..Default::default()
        },
    );
    let mut manager = new_manager_with_instance(TestInstanceState::new(delay_config))
        .await
        .with_parent_deadline(Duration::from_secs(5));

    let started = tokio::time::Instant::now();
    manager.request_state(make_state_request(&test_data.views[0]));
    manager.request_state(make_state_request(&test_data.views[1]));
    manager.next().await.expect("view 1 completes");
    manager.next().await.expect("view 2 completes");
    assert!(
        started.elapsed() >= validation_delay * 2,
        "view 2 must validate against the real parent state, after it"
    );
}

/// State request with seeded genesis parent spawns validation and produces output.
#[tokio::test]
async fn test_state_request_with_genesis_parent() {
    let mut manager = new_manager().await;
    let test_data = TestData::new(2).await;

    manager.request_state(make_state_request(&test_data.views[0]));

    let output = manager.next().await.expect("should produce output");
    assert!(
        matches!(
            output,
            StateManagerOutput::State {
                validated: true,
                ..
            }
        ),
        "Should receive validated state output after validation completes"
    );
}

/// Sequential state requests: view 1 completes, then view 2 uses its result.
#[tokio::test]
async fn test_sequential_state_requests() {
    let mut manager = new_manager().await;
    let test_data = TestData::new(3).await;

    // Request view 1 and let it complete.
    manager.request_state(make_state_request(&test_data.views[0]));
    manager.next().await.expect("view 1 should complete");

    // Request view 2 — parent (view 1) should now exist.
    manager.request_state(make_state_request(&test_data.views[1]));
    let output = manager.next().await.expect("should produce output");
    assert!(
        matches!(
            output,
            StateManagerOutput::State {
                validated: true,
                ..
            }
        ),
        "View 2 should produce StateVerified"
    );
}

/// State request queued behind in-progress parent auto-starts when parent completes.
#[tokio::test]
async fn test_state_request_queued_behind_parent() {
    let mut manager = new_manager().await;
    let test_data = TestData::new(3).await;

    // Send both requests before either completes.
    manager.request_state(make_state_request(&test_data.views[0]));
    manager.request_state(make_state_request(&test_data.views[1]));

    // View 2 should be queued as pending (parent view 1 is in progress).
    let view_1_commit = proposal_commitment(&test_data.views[0].proposal.data.clone());
    assert!(
        manager.pending_contains_commitment(&view_1_commit),
        "View 2 should be pending on view 1's commitment"
    );

    // next() should process view 1, then eagerly chain view 2.
    let output1 = manager.next().await.expect("view 1 should complete");
    let output2 = manager.next().await.expect("view 2 should complete");
    assert_eq!(
        count_state_verified(&[output1, output2]),
        2,
        "Both views should complete after pending resolution"
    );
}

/// Header request with existing parent state produces header output.
#[tokio::test]
async fn test_header_request_with_parent() {
    let mut manager = new_manager().await;
    let test_data = TestData::new(3).await;

    // Complete state for view 1 so it can be used as parent for header.
    manager.request_state(make_state_request(&test_data.views[0]));
    manager.next().await.expect("view 1 should complete");

    // Now request a header with view 1 as parent.
    let header_req = make_header_request(&test_data.views[0], test_data.views[1].view_number);
    manager.request_header(header_req);

    let output = manager.next().await.expect("should produce output");
    assert!(
        matches!(
            output,
            StateManagerOutput::Header {
                header: Some(_),
                ..
            }
        ),
        "Should receive HeaderCreated after header creation completes"
    );
}

/// Header request queued behind in-progress state starts when state completes.
#[tokio::test]
async fn test_header_request_queued_behind_state() {
    let mut manager = new_manager().await;
    let test_data = TestData::new(3).await;

    // Send state request for view 1 (starts validation).
    manager.request_state(make_state_request(&test_data.views[0]));

    // Send header request with view 1 as parent BEFORE view 1 completes.
    let header_req = make_header_request(&test_data.views[0], test_data.views[1].view_number);
    manager.request_header(header_req);

    // Header should be pending on view 1's commitment.
    let view_1_commit = proposal_commitment(&test_data.views[0].proposal.data.clone());
    assert!(
        manager.pending_contains_commitment(&view_1_commit),
        "Header should be pending on view 1's commitment"
    );

    // next() processes state completion, which chains the header request.
    let output1 = manager.next().await.expect("state should complete");
    assert!(
        matches!(
            output1,
            StateManagerOutput::State {
                validated: true,
                ..
            }
        ),
        "State should be verified first"
    );

    let output2 = manager.next().await.expect("header should complete");
    assert!(
        matches!(
            output2,
            StateManagerOutput::Header {
                header: Some(_),
                ..
            }
        ),
        "Header should be created after pending state resolves"
    );
}

/// Duplicate state request for the same view is ignored.
#[tokio::test]
async fn test_duplicate_state_request_ignored() {
    let mut manager = new_manager().await;
    let test_data = TestData::new(2).await;

    // Send same state request twice.
    manager.request_state(make_state_request(&test_data.views[0]));
    manager.request_state(make_state_request(&test_data.views[0]));

    let output = manager.next().await.expect("should produce output");
    assert!(matches!(
        output,
        StateManagerOutput::State {
            validated: true,
            ..
        }
    ));

    // No second output — duplicate was ignored.
    assert!(
        manager.next().await.is_none(),
        "Duplicate request should be ignored — only one response"
    );
}

/// Two header requests with the same parent but different views must both
/// run.  This is the common case after consecutive timeouts where the
/// `locked_cert` doesn't change: a leader for view N+1 and view N+2 both
/// build proposals against the same parent.
#[tokio::test]
async fn test_header_requests_same_parent_different_views() {
    let mut manager = new_manager().await;
    let test_data = TestData::new(4).await;

    // Complete state for view 1 so it can be used as parent.
    manager.request_state(make_state_request(&test_data.views[0]));
    manager.next().await.expect("view 1 should complete");

    // Two header requests sharing the same parent (view 1) but targeting
    // different views.  Both must produce headers; neither may be deduped.
    let req_a = make_header_request(&test_data.views[0], test_data.views[1].view_number);
    let req_b = make_header_request(&test_data.views[0], test_data.views[2].view_number);
    manager.request_header(req_a);
    manager.request_header(req_b);

    let mut outputs = Vec::new();
    for _ in 0..2 {
        outputs.push(manager.next().await.expect("should produce output"));
    }
    assert_eq!(
        count_header_created(&outputs),
        2,
        "Both header requests should complete — same parent, different views"
    );
}

/// State and header requests for different views can be interleaved.
#[tokio::test]
async fn test_interleaved_state_and_header_requests() {
    let mut manager = new_manager().await;
    let test_data = TestData::new(4).await;

    // Start state validation for views 1 and 2, plus header request for view 2
    // (with view 1 as parent).
    manager.request_state(make_state_request(&test_data.views[0]));
    manager.request_state(make_state_request(&test_data.views[1]));
    let header_req = make_header_request(&test_data.views[0], test_data.views[1].view_number);
    manager.request_header(header_req);

    // Collect all outputs.
    let mut outputs = Vec::new();
    for _ in 0..3 {
        outputs.push(manager.next().await.expect("should produce output"));
    }

    assert_eq!(
        count_state_verified(&outputs),
        2,
        "Both state requests should complete"
    );
    assert_eq!(
        count_header_created(&outputs),
        1,
        "Header request should complete"
    );
}

/// A decide that garbage-collects a still-running validation must not orphan
/// the requests queued behind it: view 2's own validation queued on view 1,
/// and the header for view 3 (led by this node) queued on view 2.
#[tokio::test]
async fn test_gc_releases_requests_queued_on_aborted_validation() {
    let mut manager = new_manager().await;
    let test_data = TestData::new(4).await;

    // Spawned but not yet polled, so view 1 is still in flight at the gc.
    manager.request_state(make_state_request(&test_data.views[0]));
    manager.request_state(make_state_request(&test_data.views[1]));
    manager.request_header(make_header_request(
        &test_data.views[1],
        test_data.views[2].view_number,
    ));

    manager.gc(test_data.views[1].view_number);

    let mut outputs = Vec::new();
    while let Some(output) = manager.next().await {
        outputs.push(output);
    }
    assert_eq!(
        count_state_verified(&outputs),
        1,
        "view 2 should validate against a stub of the aborted view 1"
    );
    assert_eq!(
        count_header_created(&outputs),
        1,
        "the header for view 3 should be built once view 2's state lands"
    );
}

/// No stub is seeded for an aborted validation nothing is queued on.
#[tokio::test]
async fn test_gc_aborts_stale_validation_without_dependents() {
    let mut manager = new_manager().await;
    let test_data = TestData::new(3).await;

    manager.request_state(make_state_request(&test_data.views[0]));
    manager.gc(test_data.views[1].view_number);

    assert!(
        manager.next().await.is_none(),
        "the aborted validation should produce no output"
    );
    assert!(
        !manager.validated_contains_view(test_data.views[0].view_number),
        "no stub should be seeded for a view nothing is queued on"
    );
}
