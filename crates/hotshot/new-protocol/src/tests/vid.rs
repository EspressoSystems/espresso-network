use hotshot::types::BLSPubKey;
use hotshot_example_types::node_types::TestTypes;
use hotshot_types::{
    data::VidDisperseShare2, traits::signature_key::SignatureKey, vid::avidm_gf2::AvidmGf2Scheme,
};

use super::common::utils::{TestData, TestView};
use crate::vid::{VidReconstructErrorKind, VidReconstructor};

/// Threshold for SuccessThreshold with 10 nodes of stake 1: (10*2)/3 + 1 = 7.
const THRESHOLD: u64 = 7;

/// Fetch the honest share for the voter with key index `i`.
fn honest_share(view: &TestView, i: u64) -> VidDisperseShare2<TestTypes> {
    view.vid_share_for(&BLSPubKey::generated_from_seed_indexed([0u8; 32], i).0)
}

/// The VID recovery threshold (in weight units), not the consensus vote
/// threshold.
fn recovery_threshold(view: &TestView) -> u64 {
    view.vid_shares[0].common.param.recovery_threshold as u64
}

/// A voter (key index 9) whose honest share the tests never feed: stands in as
/// the attacker / poison / squatter / replayer key.
fn attacker_key() -> BLSPubKey {
    BLSPubKey::generated_from_seed_indexed([0u8; 32], 9).0
}

/// A share claiming the view's commitment and common but carrying a different
/// payload's content, so it fails merkle verification. Occupies voter `slot`'s
/// shard range, addressed to `recipient_key` (forge it to model squatting).
fn garbage_share(
    view: &TestView,
    recipient_key: BLSPubKey,
    slot: usize,
) -> VidDisperseShare2<TestTypes> {
    let template = &view.vid_shares[0];
    let common = template.common.clone();
    let weights: Vec<u32> = view
        .vid_shares
        .iter()
        .map(|s| s.share.weight() as u32)
        .collect();
    let other_payload = vec![0xb7u8; 64];
    let (_, _, other_shares) = AvidmGf2Scheme::ns_disperse(
        &common.param,
        &weights,
        &other_payload,
        std::iter::once(0..other_payload.len()),
    )
    .unwrap();
    VidDisperseShare2::<TestTypes> {
        view_number: template.view_number,
        epoch: template.epoch,
        target_epoch: template.target_epoch,
        payload_commitment: template.payload_commitment,
        share: other_shares[slot].clone(),
        recipient_key,
        common,
    }
}

/// Feed a share as if it arrived from the voter it is addressed to (the
/// honest case: the authenticated sender equals the share's `recipient_key`).
fn feed(reconstructor: &mut VidReconstructor<TestTypes>, share: VidDisperseShare2<TestTypes>) {
    let sender = share.recipient_key;
    reconstructor.handle_vid_share(sender, share);
}

/// Pin the reconstructor to the view's proposal, as the coordinator does
/// when the validated proposal arrives.
fn handle_proposal(reconstructor: &mut VidReconstructor<TestTypes>, view: &TestView) {
    reconstructor.handle_proposal(
        view.view_number,
        view.vid_shares[0].payload_commitment,
        view.proposal.data.block_header.metadata,
        view.proposal.data.epoch,
        // The committee-fixed param the coordinator derives; equals the honest
        // shares' `common.param`.
        Some(view.vid_shares[0].common.param.clone()),
    );
}

/// Assert that no reconstruction attempt runs in a short window — neither a
/// successful payload nor an errored attempt. Used where the offending input
/// must be rejected at intake so nothing is ever spawned. `context` names that
/// input for the failure message.
async fn assert_no_reconstruction(reconstructor: &mut VidReconstructor<TestTypes>, context: &str) {
    let result =
        tokio::time::timeout(std::time::Duration::from_millis(500), reconstructor.next()).await;
    match result {
        Err(_) | Ok(None) => { /* timed out or no tasks — no attempt ran, good */ },
        Ok(Some(Ok(out))) => {
            panic!("BUG: {context} produced a payload for view {:?}", out.view)
        },
        Ok(Some(Err(err))) => {
            panic!(
                "BUG: {context} triggered a reconstruction attempt for view {:?}",
                err.view
            )
        },
    }
}

/// Assert that no *duplicate* successful reconstruction is produced in a short
/// window. An errored attempt is tolerated; a second `Ok` is the failure.
async fn assert_no_duplicate_success(
    reconstructor: &mut VidReconstructor<TestTypes>,
    context: &str,
) {
    let result =
        tokio::time::timeout(std::time::Duration::from_millis(500), reconstructor.next()).await;
    match result {
        Err(_) | Ok(None) | Ok(Some(Err(_))) => { /* no duplicate success — good */ },
        Ok(Some(Ok(out))) => {
            panic!(
                "BUG: got a duplicate BlockReconstructed for view {:?} — {context}",
                out.view
            )
        },
    }
}

/// Drive the reconstructor to a single successful reconstruction and assert it
/// matches `view`'s commitment.
async fn expect_reconstruction(reconstructor: &mut VidReconstructor<TestTypes>, view: &TestView) {
    let out = tokio::time::timeout(std::time::Duration::from_secs(5), reconstructor.next())
        .await
        .expect("reconstruction should complete in time")
        .expect("should produce a result")
        .expect("reconstruction should succeed");
    assert_eq!(out.view, view.view_number);
    assert_eq!(
        out.payload_commitment,
        view.vid_shares[0].payload_commitment
    );
}

/// Feeding shares beyond the threshold produces exactly one reconstruction
/// result, not one per extra share.
#[tokio::test]
async fn test_no_duplicate_reconstruction_after_threshold() {
    let test_data = TestData::new(1).await;
    let view = &test_data.views[0];
    let mut reconstructor = VidReconstructor::<TestTypes>::new();

    handle_proposal(&mut reconstructor, view);
    // Feed every node's share — more than the threshold.
    for i in 0..view.vid_shares.len() as u64 {
        feed(&mut reconstructor, honest_share(view, i));
    }

    expect_reconstruction(&mut reconstructor, view).await;

    // A second call must NOT produce another result for the same view.
    assert_no_duplicate_success(
        &mut reconstructor,
        "the reconstructor spawned multiple tasks for the same view",
    )
    .await;
}

/// `retire_view` should suppress reconstruction for the retired view
/// even when the proposal and threshold-plus shares are fed in afterwards.
#[tokio::test]
async fn test_retire_view_skips_reconstruction() {
    let test_data = TestData::new(1).await;
    let view = &test_data.views[0];
    let mut reconstructor = VidReconstructor::<TestTypes>::new();

    reconstructor.retire_view(view.view_number);

    handle_proposal(&mut reconstructor, view);
    for i in 0..view.vid_shares.len() as u64 {
        feed(&mut reconstructor, honest_share(view, i));
    }

    assert_no_duplicate_success(
        &mut reconstructor,
        "retire_view should have suppressed reconstruction",
    )
    .await;
}

/// Shares arriving after reconstruction has already completed for a view
/// should be silently dropped (the `reconstructed` set guards this path).
#[tokio::test]
async fn test_shares_after_reconstruction_are_ignored() {
    let test_data = TestData::new(1).await;
    let view = &test_data.views[0];
    let mut reconstructor = VidReconstructor::<TestTypes>::new();

    handle_proposal(&mut reconstructor, view);
    for i in 0..THRESHOLD {
        feed(&mut reconstructor, honest_share(view, i));
    }

    // Drain the one expected result.
    expect_reconstruction(&mut reconstructor, view).await;

    // More shares for the same view should be ignored.
    for i in THRESHOLD..view.vid_shares.len() as u64 {
        feed(&mut reconstructor, honest_share(view, i));
    }

    assert_no_duplicate_success(&mut reconstructor, "reconstruction was already completed").await;
}

/// Shares arriving before the view's proposal are held pending and admitted as
/// soon as the proposal pins the view's commitment.
#[tokio::test]
async fn test_shares_before_proposal_reconstruct_on_proposal() {
    let test_data = TestData::new(1).await;
    let view = &test_data.views[0];
    let mut reconstructor = VidReconstructor::<TestTypes>::new();

    for i in 0..THRESHOLD {
        feed(&mut reconstructor, honest_share(view, i));
    }

    // No proposal yet: nothing to reconstruct against.
    assert_no_reconstruction(&mut reconstructor, "a share before the proposal").await;

    // The proposal admits the pending shares; no further input is needed.
    handle_proposal(&mut reconstructor, view);
    expect_reconstruction(&mut reconstructor, view).await;
}

/// Shares whose common is not hash-bound to the proposal's commitment are
/// rejected at intake and don't block honest reconstruction of the view.
#[tokio::test]
async fn test_inconsistent_common_shares_ignored() {
    let test_data = TestData::new(1).await;
    let view = &test_data.views[0];
    let mut reconstructor = VidReconstructor::<TestTypes>::new();
    handle_proposal(&mut reconstructor, view);

    // A valid common over different bytes: consistent with some commitment,
    // but not the proposal's.
    let param = &view.vid_shares[0].common.param;
    let other_payload = vec![0xa5u8; 64];
    let (_, wrong_common) = AvidmGf2Scheme::commit(
        param,
        &other_payload,
        std::iter::once(0..other_payload.len()),
    )
    .unwrap();

    // Threshold-plus shares claiming the proposal's commitment but carrying the
    // wrong common; all dropped at intake, so no reconstruction is attempted.
    for i in 0..THRESHOLD {
        let mut share = honest_share(view, i);
        share.common = wrong_common.clone();
        feed(&mut reconstructor, share);
    }
    assert_no_reconstruction(
        &mut reconstructor,
        "a share with a common inconsistent with the commitment",
    )
    .await;

    // The same voters can still contribute their honest shares.
    for i in 0..THRESHOLD {
        feed(&mut reconstructor, honest_share(view, i));
    }
    expect_reconstruction(&mut reconstructor, view).await;
}

/// A share claiming the right commitment but carrying a different payload's
/// content poisons the first decode. The reconstructor weeds it out (reporting
/// the offending voter) and recovers from the verified remainder.
#[tokio::test]
async fn test_weeds_bad_shares_and_recovers() {
    let test_data = TestData::new(1).await;
    let view = &test_data.views[0];
    let mut reconstructor = VidReconstructor::<TestTypes>::new();

    let payload_commitment = view.vid_shares[0].payload_commitment;
    let recovery_threshold = recovery_threshold(view);

    // A share claiming the real commitment but carrying a different payload's
    // content: structurally valid but fails verification. Its voter is one
    // whose honest share we never feed.
    let poison_voter = attacker_key();
    let poison = garbage_share(view, poison_voter, view.vid_shares.len() - 1);

    // One short of the threshold honestly, then the poison share to trigger a
    // poisoned attempt.
    handle_proposal(&mut reconstructor, view);
    for i in 0..recovery_threshold - 1 {
        feed(&mut reconstructor, honest_share(view, i));
    }
    feed(&mut reconstructor, poison);
    // More honest shares arrive while the poisoned attempt is in flight.
    feed(
        &mut reconstructor,
        honest_share(view, recovery_threshold - 1),
    );
    feed(&mut reconstructor, honest_share(view, recovery_threshold));

    // The poisoned attempt fails, identifying exactly the poison voter.
    let result = tokio::time::timeout(std::time::Duration::from_secs(5), reconstructor.next())
        .await
        .expect("poisoned attempt should complete in time")
        .expect("should produce a result");
    let err = match result {
        Err(err) => err,
        Ok(out) => panic!(
            "BUG: poisoned reconstruction for view {:?} produced a payload",
            out.view
        ),
    };
    assert_eq!(err.view, view.view_number);
    assert_eq!(err.payload_commitment, payload_commitment);
    assert_eq!(err.kind, VidReconstructErrorKind::AwaitingShares);
    assert_eq!(err.bad_share_keys, vec![poison_voter]);

    // Weeding plus the shares that arrived in flight put the coverage back
    // over the threshold: the retry happens without further input.
    expect_reconstruction(&mut reconstructor, view).await;
}

/// Replaying another voter's valid share under your own key adds no coverage:
/// it collides with the genuine share's range and loses the conflict, so no
/// attempt runs until genuinely new shares cover the threshold.
#[tokio::test]
async fn test_replayed_share_does_not_fake_coverage() {
    let test_data = TestData::new(1).await;
    let view = &test_data.views[0];
    let mut reconstructor = VidReconstructor::<TestTypes>::new();

    let recovery_threshold = recovery_threshold(view);

    // The replayer is a voter whose honest share we never feed.
    let replayer = attacker_key();
    let mut replay = honest_share(view, 0);
    replay.recipient_key = replayer;

    handle_proposal(&mut reconstructor, view);
    for i in 0..recovery_threshold - 1 {
        feed(&mut reconstructor, honest_share(view, i));
    }
    // The replay covers an already-covered range: it loses the conflict to the
    // genuine verified share, so coverage stays below the threshold.
    feed(&mut reconstructor, replay);

    assert_no_reconstruction(&mut reconstructor, "a replayed share").await;

    // One more honest share provides the missing distinct range.
    feed(
        &mut reconstructor,
        honest_share(view, recovery_threshold - 1),
    );
    expect_reconstruction(&mut reconstructor, view).await;
}

/// A share delivered by a sender other than the voter it is addressed to is
/// rejected at intake (cheap key-equality): a node can only contribute its own
/// share, so impersonation cannot fake coverage.
#[tokio::test]
async fn test_forged_recipient_key_rejected() {
    let test_data = TestData::new(1).await;
    let view = &test_data.views[0];
    let mut reconstructor = VidReconstructor::<TestTypes>::new();

    let recovery_threshold = recovery_threshold(view);
    let attacker = attacker_key();

    handle_proposal(&mut reconstructor, view);
    // One short of the recovery threshold, honestly.
    for i in 0..recovery_threshold - 1 {
        feed(&mut reconstructor, honest_share(view, i));
    }
    // The missing share, but delivered by an attacker forging the victim's
    // recipient_key. Rejected at intake, so coverage stays short.
    let victim_share = honest_share(view, recovery_threshold - 1);
    reconstructor.handle_vid_share(attacker, victim_share);

    assert_no_reconstruction(&mut reconstructor, "a forged-sender share").await;

    // Delivered honestly by its true owner, the same share completes the set.
    feed(
        &mut reconstructor,
        honest_share(view, recovery_threshold - 1),
    );
    expect_reconstruction(&mut reconstructor, view).await;
}

/// Pending shares are keyed by authenticated sender, so a forged-recipient
/// share (garbage addressed to a victim) is rejected at intake and cannot squat
/// the victim's pending slot; the victim's real share lands and reconstructs.
#[tokio::test]
async fn test_forged_recipient_does_not_squat_pending() {
    let test_data = TestData::new(1).await;
    let view = &test_data.views[0];
    let mut reconstructor = VidReconstructor::<TestTypes>::new();

    let recovery_threshold = recovery_threshold(view);
    let attacker = attacker_key();
    let victim_slot = 0usize;
    let victim_key = honest_share(view, victim_slot as u64).recipient_key;

    // Before the proposal: the attacker forges the victim's recipient_key on a
    // garbage share. The sender check rejects it, so the slot is never squatted.
    let squat = garbage_share(view, victim_key, victim_slot);
    reconstructor.handle_vid_share(attacker, squat);

    // The honest shares (including the victim's) land in their own pending
    // slots; the proposal then admits them and the view reconstructs cleanly.
    for i in 0..recovery_threshold {
        feed(&mut reconstructor, honest_share(view, i));
    }
    handle_proposal(&mut reconstructor, view);

    expect_reconstruction(&mut reconstructor, view).await;
}

/// A Byzantine voter pins the view's `common` to one with the proposal's real
/// `ns_commits` (so `is_consistent` passes) but a forged `param` (an inflated
/// `recovery_threshold`). If admitted first it becomes the verification oracle,
/// and since later shares must carry the identical common, every honest share
/// is rejected for common-mismatch. A secure reconstructor rejects the forged
/// `param` and reconstructs from the honest shares.
#[tokio::test]
async fn test_poisoned_common_param_does_not_block_reconstruction() {
    let test_data = TestData::new(1).await;
    let view = &test_data.views[0];
    let mut reconstructor = VidReconstructor::<TestTypes>::new();

    // Attacker (slot 9, whose honest share we never feed) keeps its real share
    // content — so the common stays `is_consistent` — but forges an unreachable
    // `recovery_threshold`. Sent as voter 9's own share, so the sender check passes.
    let mut poison = honest_share(view, 9);
    poison.common.param.recovery_threshold = usize::MAX;

    handle_proposal(&mut reconstructor, view);
    // The attacker's poisoned share is the first admitted, so it pins the
    // poisoned common before any honest share arrives.
    feed(&mut reconstructor, poison);
    // Every honest voter's share — far more than the real recovery threshold.
    for i in 0..9u64 {
        feed(&mut reconstructor, honest_share(view, i));
    }

    // A secure reconstructor rejects the poisoned common and reconstructs from
    // the honest shares; vulnerable code pins it and `next()` yields nothing.
    expect_reconstruction(&mut reconstructor, view).await;
}

/// A squatter occupies an honest voter's shard range with garbage under its own
/// key (admitted on the crypto-free fast path). When the genuine owner's share
/// arrives, the collision triggers verification: the garbage is evicted and the
/// honest share admitted, so the first attempt is all-honest and succeeds.
#[tokio::test]
async fn test_overlapping_garbage_loses_conflict_to_honest_share() {
    let test_data = TestData::new(1).await;
    let view = &test_data.views[0];
    let mut reconstructor = VidReconstructor::<TestTypes>::new();

    let recovery_threshold = recovery_threshold(view);
    let squatter = attacker_key();
    let victim_slot = 0usize;

    handle_proposal(&mut reconstructor, view);
    // The squatter takes the victim's range with garbage (fast-path admit).
    feed(
        &mut reconstructor,
        garbage_share(view, squatter, victim_slot),
    );
    // Honest shares for other slots — still below the threshold, so no attempt
    // runs while the garbage is admitted.
    for i in 1..recovery_threshold - 1 {
        feed(&mut reconstructor, honest_share(view, i));
    }
    // The victim's genuine share collides with the squatter's range: the garbage
    // is verified, fails, and is evicted; the honest share is admitted.
    feed(&mut reconstructor, honest_share(view, victim_slot as u64));
    // One more honest share reaches the threshold.
    feed(
        &mut reconstructor,
        honest_share(view, recovery_threshold - 1),
    );

    expect_reconstruction(&mut reconstructor, view).await;
}
