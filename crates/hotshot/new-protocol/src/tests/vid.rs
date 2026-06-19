use hotshot::types::BLSPubKey;
use hotshot_example_types::node_types::TestTypes;
use hotshot_types::{
    data::{VidCommitment2, VidDisperseShare2},
    traits::signature_key::SignatureKey,
    vid::avidm_gf2::AvidmGf2Scheme,
};

use super::common::utils::{TestData, TestView};
use crate::{
    tests::common::utils::vid_fragments,
    vid::{VidFragmentAccumulator, VidFragmentError, VidReconstructErrorKind, VidReconstructor},
};

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

/// A dispersal that commits to a *non-codeword*: every returned share verifies
/// against the returned common, yet no threshold-covering subset decodes to a
/// payload that re-commits to the commitment. Built with the view's real VID
/// param and weights so the committee's `expected_param` check admits the
/// shares, and addressed to the same voter keys `honest_share` uses.
fn non_codeword_shares(view: &TestView) -> (VidCommitment2, Vec<VidDisperseShare2<TestTypes>>) {
    let template = &view.vid_shares[0];
    let param = template.common.param.clone();
    let weights: Vec<u32> = view
        .vid_shares
        .iter()
        .map(|s| s.share.weight() as u32)
        .collect();
    // Content is irrelevant — the dispersal is poisoned regardless — but it
    // must be long enough to produce recovery shards to corrupt.
    let payload = vec![0x5au8; 64];
    let (payload_commitment, common, shares) = AvidmGf2Scheme::ns_disperse_non_codeword(
        &param,
        &weights,
        &payload,
        std::iter::once(0..payload.len()),
    )
    .expect("non-codeword dispersal");
    let shares = shares
        .into_iter()
        .enumerate()
        .map(|(i, share)| VidDisperseShare2::<TestTypes> {
            view_number: template.view_number,
            epoch: template.epoch,
            target_epoch: template.target_epoch,
            payload_commitment,
            share,
            recipient_key: BLSPubKey::generated_from_seed_indexed([0u8; 32], i as u64).0,
            common: common.clone(),
        })
        .collect();
    (payload_commitment, shares)
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

/// A disperser that commits to a non-codeword produces shares that each verify
/// against the commitment yet cannot decode to a matching payload. The
/// reconstructor blames no voter (the shares are all individually valid),
/// reports the view as unrecoverable, and — having proven the disperser
/// faulty — spawns no further attempt even as the remaining shares arrive.
#[tokio::test]
async fn test_non_codeword_payload_is_unrecoverable() {
    let test_data = TestData::new(1).await;
    let view = &test_data.views[0];
    let mut reconstructor = VidReconstructor::<TestTypes>::new();

    let (payload_commitment, shares) = non_codeword_shares(view);
    let recovery_threshold = recovery_threshold(view);

    // Pin the view to the poisoned commitment and the committee's real VID
    // param (so the param check admits the shares).
    reconstructor.handle_proposal(
        view.view_number,
        payload_commitment,
        view.proposal.data.block_header.metadata,
        view.proposal.data.epoch,
        Some(shares[0].common.param.clone()),
    );

    // Feed just enough verifying shares to cover the recovery threshold.
    let mut coverage = 0u64;
    for share in &shares {
        if coverage >= recovery_threshold {
            break;
        }
        coverage += share.share.weight() as u64;
        feed(&mut reconstructor, share.clone());
    }

    // Every share verifies, so no voter is blamed; but the shares decode to a
    // payload that does not re-commit, so the view is unrecoverable.
    let result = tokio::time::timeout(std::time::Duration::from_secs(5), reconstructor.next())
        .await
        .expect("attempt should complete in time")
        .expect("should produce a result");
    let err = match result {
        Err(err) => err,
        Ok(out) => panic!(
            "BUG: non-codeword reconstruction for view {:?} produced a payload",
            out.view
        ),
    };
    assert_eq!(err.view, view.view_number);
    assert_eq!(err.payload_commitment, payload_commitment);
    assert_eq!(err.kind, VidReconstructErrorKind::Unrecoverable);
    assert!(
        err.bad_share_keys.is_empty(),
        "no share is individually unverifiable: {:?}",
        err.bad_share_keys
    );

    // The view is now exhausted: the remaining verified shares trigger no
    // further attempt.
    for share in &shares {
        feed(&mut reconstructor, share.clone());
    }
    assert_no_reconstruction(
        &mut reconstructor,
        "shares for a view already proven unrecoverable",
    )
    .await;
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

/// Feeding a view's namespace fragments — in any order — reassembles exactly
/// the combined share the leader would otherwise have sent, and only the final
/// fragment yields it.
#[tokio::test]
async fn fragment_accumulator_reassembles_share() {
    let test_data = TestData::new(1).await;
    let view = &test_data.views[0];
    let original = honest_share(view, 0);

    // Reverse the fragments to prove order independence.
    let mut fragments = vid_fragments(&original).collect::<Vec<_>>();
    fragments.reverse();
    let last = fragments.len() - 1;

    let mut accumulator = VidFragmentAccumulator::<TestTypes>::default();
    let mut reassembled = None;
    for (i, fragment) in fragments.into_iter().enumerate() {
        let out = accumulator.accept(fragment).expect("fragment accepted");
        if i == last {
            reassembled = out;
        } else {
            assert!(out.is_none(), "share completed before its last fragment");
        }
    }
    assert_eq!(reassembled, Some(original));
}

/// A fragment whose namespace index is outside `0..num_namespaces` is rejected,
/// a second fragment for an already-buffered index is a duplicate, and a
/// fragment disagreeing with the pinned metadata is inconsistent.
#[tokio::test]
async fn fragment_accumulator_rejects_malformed_fragments() {
    let test_data = TestData::new(1).await;
    let view = &test_data.views[0];
    // A real fragment to use as a template; its content is irrelevant since the
    // accumulator's intake checks are purely structural.
    let template = vid_fragments(&honest_share(view, 0))
        .collect::<Vec<_>>()
        .remove(0);

    let mut accumulator = VidFragmentAccumulator::<TestTypes>::default();

    // Out-of-range index.
    let mut out_of_range = template.clone();
    out_of_range.num_namespaces = 2;
    out_of_range.namespaces[0].ns_index = 2;
    assert!(matches!(
        accumulator.accept(out_of_range),
        Err(VidFragmentError::IndexOutOfRange {
            index: 2,
            num_namespaces: 2
        })
    ));

    // First fragment of a two-namespace share: buffered, incomplete.
    let mut first = template.clone();
    first.num_namespaces = 2;
    first.namespaces[0].ns_index = 0;
    assert!(
        accumulator
            .accept(first.clone())
            .expect("accepted")
            .is_none()
    );

    // A second fragment for the same index is a duplicate.
    assert!(matches!(
        accumulator.accept(first),
        Err(VidFragmentError::DuplicateIndex(0))
    ));

    // A fragment that disagrees with the pinned commitment is inconsistent.
    let mut wrong_commitment = template;
    wrong_commitment.num_namespaces = 2;
    wrong_commitment.namespaces[0].ns_index = 1;
    wrong_commitment.payload_commitment = VidCommitment2::default();
    assert!(matches!(
        accumulator.accept(wrong_commitment),
        Err(VidFragmentError::Inconsistent)
    ));
}

/// Build storage node `slot`'s share of a payload split into `ns_count`
/// namespaces, using the view's VID params and weights so the structure matches
/// a real dispersal. `TestView`'s blocks are single-namespace, so this is how
/// the multi-piece fragment paths get exercised.
fn multi_namespace_share(
    view: &TestView,
    slot: usize,
    ns_count: usize,
) -> VidDisperseShare2<TestTypes> {
    let template = &view.vid_shares[0];
    let param = template.common.param.clone();
    let weights: Vec<u32> = view
        .vid_shares
        .iter()
        .map(|s| s.share.weight() as u32)
        .collect();
    // A few bytes per namespace; the contents are irrelevant to the accumulator.
    let ns_len = 16usize;
    let payload: Vec<u8> = (0..ns_count * ns_len).map(|i| i as u8).collect();
    let ns_table = (0..ns_count).map(|i| i * ns_len..(i + 1) * ns_len);
    let (payload_commitment, common, shares) =
        AvidmGf2Scheme::ns_disperse(&param, &weights, &payload, ns_table).unwrap();
    VidDisperseShare2 {
        view_number: template.view_number,
        epoch: template.epoch,
        target_epoch: template.target_epoch,
        payload_commitment,
        share: shares[slot].clone(),
        recipient_key: BLSPubKey::generated_from_seed_indexed([0u8; 32], slot as u64).0,
        common,
    }
}

/// A fragment carrying several pieces (a bucket) reassembles them, whether the
/// whole share arrives in one fragment or is split across multi-piece fragments
/// delivered out of order.
#[tokio::test]
async fn fragment_accumulator_reassembles_multi_piece_fragments() {
    let test_data = TestData::new(1).await;
    let view = &test_data.views[0];
    let original = multi_namespace_share(view, 0, 4);

    // All four namespaces in one bucket → completes on the single fragment.
    let mut whole = vid_fragments(&original).next().expect("at least one piece");
    whole.namespaces = vid_fragments(&original)
        .flat_map(|f| f.namespaces)
        .collect();
    let mut accumulator = VidFragmentAccumulator::<TestTypes>::default();
    assert_eq!(
        accumulator.accept(whole).expect("accepted"),
        Some(original.clone())
    );

    // Split into two multi-piece fragments — pieces [0, 1] and [2, 3] — and feed
    // them out of order; the share completes on the second.
    let pieces: Vec<_> = vid_fragments(&original)
        .flat_map(|f| f.namespaces)
        .collect();
    let template = vid_fragments(&original).next().unwrap();
    let mut first = template.clone();
    first.namespaces = pieces[..2].to_vec();
    let mut second = template;
    second.namespaces = pieces[2..].to_vec();

    let mut accumulator = VidFragmentAccumulator::<TestTypes>::default();
    assert!(accumulator.accept(second).expect("accepted").is_none());
    assert_eq!(accumulator.accept(first).expect("accepted"), Some(original));
}

/// Two pieces for the same namespace index within a single fragment are
/// rejected as a duplicate (the existing case splits the duplicate across two
/// fragments; this exercises the intra-fragment path).
#[tokio::test]
async fn fragment_accumulator_rejects_intra_fragment_duplicate() {
    let test_data = TestData::new(1).await;
    let view = &test_data.views[0];
    let original = multi_namespace_share(view, 0, 2);
    let pieces: Vec<_> = vid_fragments(&original)
        .flat_map(|f| f.namespaces)
        .collect();

    let mut fragment = vid_fragments(&original).next().unwrap();
    fragment.namespaces = vec![pieces[0].clone(), pieces[0].clone()];

    let mut accumulator = VidFragmentAccumulator::<TestTypes>::default();
    assert!(matches!(
        accumulator.accept(fragment),
        Err(VidFragmentError::DuplicateIndex(0))
    ));
}
