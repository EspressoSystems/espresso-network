use std::sync::Arc;

use hotshot::traits::BlockPayload;
use hotshot_example_types::{
    block_types::{TestBlockPayload, TestMetadata},
    node_types::{TEST_VERSIONS, TestTypes},
    state_types::{TestInstanceState, TestValidatedState},
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
    helpers::proposal_commitment,
    message::Proposal,
    state::{HeaderRequest, StateManager, StateManagerOutput, StateRequest},
    tests::common::utils::{TestData, TestView},
};

/// Build a StateRequest from a TestView.
fn make_state_request(view: &TestView) -> StateRequest<TestTypes> {
    let proposal: Proposal<TestTypes> = view.proposal.data.clone().into();
    StateRequest {
        view: view.view_number,
        parent_view: proposal.justify_qc.view_number(),
        epoch: view.epoch_number,
        block: BlockHeader::<TestTypes>::block_number(&proposal.block_header).into(),
        proposal: proposal.clone(),
        parent_commitment: proposal.justify_qc.data().leaf_commit,
        payload_size: 0,
    }
}

/// Build a HeaderRequest from a TestView (as the parent).
fn make_header_request(
    parent_view: &TestView,
    target_view: ViewNumber,
) -> HeaderRequest<TestTypes> {
    let parent_proposal: Proposal<TestTypes> = parent_view.proposal.data.clone().into();
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
    let mut manager = StateManager::new(Arc::new(TestInstanceState::default()));
    let genesis_state = TestValidatedState::default();
    let genesis_leaf = Leaf2::<TestTypes>::genesis(
        &genesis_state,
        &TestInstanceState::default(),
        TEST_VERSIONS.test.base,
    )
    .await;
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
    let mut manager = StateManager::new(Arc::new(TestInstanceState::default()));
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
    let view_1_commit = proposal_commitment(&test_data.views[0].proposal.data.clone().into());
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
    let view_1_commit = proposal_commitment(&test_data.views[0].proposal.data.clone().into());
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
