//! Tests for VID shares that become available after their view was decided without one. A late
//! share is validated, persisted, and surfaced as [`ConsensusOutput::VidShareValidated`] so the
//! query service can back-fill the missing VID data.

use std::time::Duration;

use hotshot::types::BLSPubKey;
use hotshot_types::{traits::signature_key::SignatureKey, vid::avidm_gf2::AvidmGf2Commitment};

use super::common::{
    harness::TestHarness,
    utils::{TestData, TestView},
};
use crate::{
    consensus::ConsensusOutput,
    tests::common::assertions::{any, is_vid_share_validated},
};

/// Build a `LeafDecided` output for `view_idx` with no VID share attached,
/// mimicking a view decided before this node's share became available.
fn decide_without_share(
    test_data: &TestData,
    view_idx: usize,
) -> ConsensusOutput<hotshot_example_types::node_types::TestTypes> {
    let view = &test_data.views[view_idx];
    ConsensusOutput::LeafDecided {
        leaves: vec![view.leaf.clone()],
        cert1: view.cert1.clone(),
        cert2: Some(view.cert2.clone()),
        vid_shares: vec![None],
    }
}

/// Assert that the harness emitted `VidShareValidated` carrying the decided
/// header and this node's share for `view`.
fn assert_share_delivered(harness: &TestHarness, view: &TestView, our_key: &BLSPubKey) {
    let expected_share = view.vid_share_for(our_key);
    let expected_header = &view.proposal.data.block_header;
    assert!(
        harness.outputs().iter().any(|out| matches!(
            out,
            ConsensusOutput::VidShareValidated { view: v, header, share }
                if v == &view.view_number
                    && header == expected_header
                    && share == &expected_share
        )),
        "expected VidShareValidated for view {} with the decided header and this node's share",
        view.view_number,
    );
}

/// Drive the coordinator until the late share has been delivered (or give
/// up after a bounded number of inputs). The timer provides the consensus
/// inputs that flush the outbox after the share-validator arm runs.
async fn process_until_share_delivered(harness: &mut TestHarness) {
    for _ in 0..20 {
        harness.process_until(|inputs| !inputs.is_empty()).await;
        if any(harness.outputs(), is_vid_share_validated) {
            return;
        }
    }
}

/// A VID share arriving *after* its view was decided without one is
/// validated, then delivered as `VidShareValidated` with the decided header,
/// so the query service can back-fill the missing VID data.
#[tokio::test]
async fn test_late_vid_share_delivered_after_decide() {
    let test_data = TestData::new(1).await;
    let view = &test_data.views[0];
    let (our_key, _) = BLSPubKey::generated_from_seed_indexed([0u8; 32], 0);
    // Short timer: the timeout input flushes the outbox after the late share
    // is processed inside `next_consensus_input`.
    let mut harness = TestHarness::new_with_timer(0, Duration::from_millis(500)).await;

    // Decide view 1 without its VID share.
    harness.process_output(decide_without_share(&test_data, 0));
    assert!(
        !any(harness.outputs(), is_vid_share_validated),
        "nothing to deliver yet: the share has not arrived"
    );

    // The share arrives over the network after the decide.
    harness.message(view.vid_share_input(&our_key)).await;
    process_until_share_delivered(&mut harness).await;

    assert_share_delivered(&harness, view, &our_key);
}

/// A share that was validated *before* the decide but never paired with its
/// proposal (it sat in the unpaired-share cache) is swept and delivered as
/// soon as its view is decided without a share.
#[tokio::test]
async fn test_cached_vid_share_swept_at_decide() {
    let test_data = TestData::new(1).await;
    let view = &test_data.views[0];
    let (our_key, _) = BLSPubKey::generated_from_seed_indexed([0u8; 32], 0);
    let mut harness = TestHarness::new(0).await;

    // The share arrived (and was validated) before the decide, but its
    // proposal never did, so it sat unpaired in the cache.
    harness.cache_vid_share(view.vid_share_for(&our_key));

    // Deciding view 1 without a VID share delivers the cached one.
    harness.process_output(decide_without_share(&test_data, 0));

    assert_share_delivered(&harness, view, &our_key);
}

/// An unpaired share cached before its view's decide survives the local GC run
/// by the view change that precedes the decide (`ViewChanged(V+1)` always
/// arrives before `LeafDecided(V)`), so the decide sweep can still deliver it.
#[tokio::test]
async fn test_cached_vid_share_survives_view_change_gc() {
    let test_data = TestData::new(1).await;
    let view = &test_data.views[0];
    let (our_key, _) = BLSPubKey::generated_from_seed_indexed([0u8; 32], 0);
    let mut harness = TestHarness::new(0).await;

    // The share arrived (and was validated) before the decide, but its
    // proposal never did, so it sat unpaired in the cache.
    harness.cache_vid_share(view.vid_share_for(&our_key));

    // The view change to V+1 garbage-collects local state before view V's
    // decide is processed; the cached share must survive it.
    harness.process_output(ConsensusOutput::ViewChanged(
        view.view_number + 1,
        view.epoch_number,
    ));

    // Deciding view 1 without a VID share still delivers the cached one.
    harness.process_output(decide_without_share(&test_data, 0));

    assert_share_delivered(&harness, view, &our_key);
}

/// A share addressed to a different node is rejected even though it carries a
/// valid leader envelope (the leader signs the payload commitment, not the
/// recipient): externally only this node's own share matters. The view keeps
/// waiting, and our own share arriving later is still delivered.
#[tokio::test]
async fn test_foreign_vid_share_rejected() {
    let test_data = TestData::new(1).await;
    let view = &test_data.views[0];
    let (our_key, _) = BLSPubKey::generated_from_seed_indexed([0u8; 32], 0);
    let (other_key, _) = BLSPubKey::generated_from_seed_indexed([0u8; 32], 1);
    let mut harness = TestHarness::new_with_timer(0, Duration::from_millis(500)).await;

    // Another node's (validly signed) share sits in the cache when the view
    // decides without ours.
    harness.cache_vid_share(view.vid_share_for(&other_key));
    harness.process_output(decide_without_share(&test_data, 0));
    assert!(
        !any(harness.outputs(), is_vid_share_validated),
        "a share addressed to another node must not be delivered as ours"
    );

    // Our own share still gets through afterwards.
    harness.message(view.vid_share_input(&our_key)).await;
    process_until_share_delivered(&mut harness).await;

    assert_share_delivered(&harness, view, &our_key);
}

/// A cached share whose payload commitment does not match the decided header
/// is rejected, and the view keeps waiting: the genuine share arriving later
/// is still delivered.
#[tokio::test]
async fn test_mismatched_cached_share_rejected() {
    let test_data = TestData::new(1).await;
    let view = &test_data.views[0];
    let (our_key, _) = BLSPubKey::generated_from_seed_indexed([0u8; 32], 0);
    let mut harness = TestHarness::new_with_timer(0, Duration::from_millis(500)).await;

    // Cache a share whose commitment does not match the header view 1 was
    // decided with.
    let mut bad_share = view.vid_share_for(&our_key);
    bad_share.payload_commitment = AvidmGf2Commitment {
        commit: Default::default(),
    };
    harness.cache_vid_share(bad_share);

    harness.process_output(decide_without_share(&test_data, 0));
    assert!(
        !any(harness.outputs(), is_vid_share_validated),
        "a share whose commitment does not match the decided header must not be delivered"
    );

    // The genuine share still gets through afterwards: the view remains
    // tracked as decided-without-share.
    harness.message(view.vid_share_input(&our_key)).await;
    process_until_share_delivered(&mut harness).await;

    assert_share_delivered(&harness, view, &our_key);
}
