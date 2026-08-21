use committable::Commitment;
use hotshot_example_types::node_types::TestTypes;
use hotshot_types::data::{Leaf2, ViewNumber};

use crate::coordinator::{MissingProposalRequests, PROPOSAL_FETCH_RETRY_VIEWS};

fn leaf_commit(byte: u8) -> Commitment<Leaf2<TestTypes>> {
    Commitment::from_raw([byte; 32])
}

/// A first request broadcasts; repeating it within the retry margin does not.
#[test]
fn test_missing_proposal_request_dedup() {
    let mut requests = MissingProposalRequests::<TestTypes>::default();
    let view = ViewNumber::new(10);
    let current = ViewNumber::new(12);

    assert!(requests.try_begin(view, leaf_commit(1), current));
    assert!(!requests.try_begin(view, leaf_commit(1), current));
    assert!(!requests.try_begin(view, leaf_commit(1), current + 1));

    // A different leaf commitment for the same view is a separate request.
    assert!(requests.try_begin(view, leaf_commit(2), current));
}

/// A request whose responses were lost re-broadcasts once the current view
/// has advanced past the retry margin, and then dedups again.
#[test]
fn test_missing_proposal_request_retries() {
    let mut requests = MissingProposalRequests::<TestTypes>::default();
    let view = ViewNumber::new(10);
    let current = ViewNumber::new(12);

    assert!(requests.try_begin(view, leaf_commit(1), current));
    let retry_view = current + PROPOSAL_FETCH_RETRY_VIEWS;
    assert!(!requests.try_begin(view, leaf_commit(1), retry_view - 1));
    assert!(requests.try_begin(view, leaf_commit(1), retry_view));
    assert!(!requests.try_begin(view, leaf_commit(1), retry_view));
}

/// Resolving forgets the request; a later request for the same proposal
/// broadcasts again immediately.
#[test]
fn test_missing_proposal_request_resolve() {
    let mut requests = MissingProposalRequests::<TestTypes>::default();
    let view = ViewNumber::new(10);
    let current = ViewNumber::new(12);

    assert!(!requests.resolve(view, leaf_commit(1)));
    assert!(requests.try_begin(view, leaf_commit(1), current));
    assert!(requests.resolve(view, leaf_commit(1)));
    assert!(!requests.resolve(view, leaf_commit(1)));
    assert!(requests.try_begin(view, leaf_commit(1), current));
}

/// GC drops requests at or below the decided view and keeps the rest.
#[test]
fn test_missing_proposal_request_gc() {
    let mut requests = MissingProposalRequests::<TestTypes>::default();
    let current = ViewNumber::new(12);

    assert!(requests.try_begin(ViewNumber::new(9), leaf_commit(1), current));
    assert!(requests.try_begin(ViewNumber::new(10), leaf_commit(2), current));
    assert!(requests.try_begin(ViewNumber::new(11), leaf_commit(3), current));

    requests.gc(ViewNumber::new(10));

    assert!(requests.try_begin(ViewNumber::new(9), leaf_commit(1), current));
    assert!(requests.try_begin(ViewNumber::new(10), leaf_commit(2), current));
    assert!(!requests.try_begin(ViewNumber::new(11), leaf_commit(3), current));
}
