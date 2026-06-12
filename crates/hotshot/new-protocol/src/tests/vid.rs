use hotshot::types::BLSPubKey;
use hotshot_example_types::node_types::TestTypes;
use hotshot_types::{
    data::VidDisperseShare2, traits::signature_key::SignatureKey, vid::avidm_gf2::AvidmGf2Scheme,
};

use super::common::utils::{TestData, TestView};
use crate::vid::{VidReconstructErrorKind, VidReconstructor};

/// Threshold for SuccessThreshold with 10 nodes of stake 1: (10*2)/3 + 1 = 7.
const THRESHOLD: u64 = 7;

/// Feeding shares beyond the reconstruction threshold for the same view
/// should produce exactly one BlockReconstructed result from `next()`,
/// not one per extra share.
#[tokio::test]
async fn test_no_duplicate_reconstruction_after_threshold() {
    let test_data = TestData::new(1).await;
    let view = &test_data.views[0];
    let mut reconstructor = VidReconstructor::<TestTypes>::new();

    // Feed the proposal share first (carries metadata required for reconstruction).
    let proposal_key = BLSPubKey::generated_from_seed_indexed([0u8; 32], 0).0;
    let proposal_share = view
        .vid_shares
        .iter()
        .find(|s| s.recipient_key == proposal_key)
        .unwrap()
        .clone();
    reconstructor.handle_vid_share(proposal_share, view.proposal.data.block_header.metadata);

    // Feed remaining shares from other nodes — enough to exceed the threshold.
    for i in 1..view.vid_shares.len() as u64 {
        let key = BLSPubKey::generated_from_seed_indexed([0u8; 32], i).0;
        let share = view
            .vid_shares
            .iter()
            .find(|s| s.recipient_key == key)
            .unwrap()
            .clone();
        reconstructor.handle_vid_share(share, None);
    }

    // First call to `next()` should succeed.
    let first = tokio::time::timeout(std::time::Duration::from_secs(5), reconstructor.next())
        .await
        .expect("reconstruction should complete in time");
    assert!(first.is_some(), "should produce a reconstruction result");
    assert!(first.unwrap().is_ok(), "reconstruction should succeed");

    // A second call must NOT produce another result for the same view.
    let second =
        tokio::time::timeout(std::time::Duration::from_millis(500), reconstructor.next()).await;

    // Either the future times out (Ok(None) from join_next returning None
    // meaning no tasks left) or it returns None. Both are acceptable.
    // What is NOT acceptable is getting Ok(Some(Ok(..))) — a duplicate result.
    match second {
        Err(_elapsed) => { /* timed out — no duplicate, good */ },
        Ok(None) => { /* no more tasks — no duplicate, good */ },
        Ok(Some(Err(_))) => { /* error, not a duplicate success */ },
        Ok(Some(Ok(out))) => {
            panic!(
                "BUG: got a duplicate BlockReconstructed for view {:?} — the reconstructor \
                 spawned multiple tasks for the same view",
                out.view
            );
        },
    }
}

/// `retire_view` should suppress reconstruction for the retired view
/// even when threshold-plus shares are fed in afterwards.
#[tokio::test]
async fn test_retire_view_skips_reconstruction() {
    let test_data = TestData::new(1).await;
    let view = &test_data.views[0];
    let mut reconstructor = VidReconstructor::<TestTypes>::new();

    reconstructor.retire_view(view.view_number);

    // Feed threshold-plus shares; none should trigger reconstruction.
    let proposal_key = BLSPubKey::generated_from_seed_indexed([0u8; 32], 0).0;
    let proposal_share = view
        .vid_shares
        .iter()
        .find(|s| s.recipient_key == proposal_key)
        .unwrap()
        .clone();
    reconstructor.handle_vid_share(proposal_share, view.proposal.data.block_header.metadata);
    for i in 1..view.vid_shares.len() as u64 {
        let key = BLSPubKey::generated_from_seed_indexed([0u8; 32], i).0;
        let share = view
            .vid_shares
            .iter()
            .find(|s| s.recipient_key == key)
            .unwrap()
            .clone();
        reconstructor.handle_vid_share(share, None);
    }

    let result =
        tokio::time::timeout(std::time::Duration::from_millis(500), reconstructor.next()).await;

    match result {
        Err(_elapsed) => { /* no task ever spawned — good */ },
        Ok(None) => { /* no tasks — good */ },
        Ok(Some(Err(_))) => { /* error, not a duplicate success */ },
        Ok(Some(Ok(out))) => {
            panic!(
                "BUG: retire_view should have suppressed reconstruction, but got a result for \
                 view {:?}",
                out.view
            );
        },
    }
}

/// Shares arriving after reconstruction has already completed for a view
/// should be silently dropped (the `reconstructed` set guards this path).
#[tokio::test]
async fn test_shares_after_reconstruction_are_ignored() {
    let test_data = TestData::new(1).await;
    let view = &test_data.views[0];
    let mut reconstructor = VidReconstructor::<TestTypes>::new();

    // Feed exactly threshold shares (with metadata on the first).
    let first_key = BLSPubKey::generated_from_seed_indexed([0u8; 32], 0).0;
    let first_share = view
        .vid_shares
        .iter()
        .find(|s| s.recipient_key == first_key)
        .unwrap()
        .clone();
    reconstructor.handle_vid_share(first_share, view.proposal.data.block_header.metadata);

    for i in 1..THRESHOLD {
        let key = BLSPubKey::generated_from_seed_indexed([0u8; 32], i).0;
        let share = view
            .vid_shares
            .iter()
            .find(|s| s.recipient_key == key)
            .unwrap()
            .clone();
        reconstructor.handle_vid_share(share, None);
    }

    // Drain the one expected result.
    let _result = tokio::time::timeout(std::time::Duration::from_secs(5), reconstructor.next())
        .await
        .expect("reconstruction should complete")
        .expect("should have a result")
        .expect("reconstruction should succeed");

    // Now feed more shares for the same view — they should be ignored.
    for i in THRESHOLD..view.vid_shares.len() as u64 {
        let key = BLSPubKey::generated_from_seed_indexed([0u8; 32], i).0;
        let share = view
            .vid_shares
            .iter()
            .find(|s| s.recipient_key == key)
            .unwrap()
            .clone();
        reconstructor.handle_vid_share(share, None);
    }

    // No additional reconstruction should be spawned.
    let extra =
        tokio::time::timeout(std::time::Duration::from_millis(500), reconstructor.next()).await;

    match extra {
        Err(_elapsed) => { /* timed out — good, no extra task */ },
        Ok(None) => { /* no tasks — good */ },
        Ok(Some(Err(_))) => { /* error, not a duplicate success */ },
        Ok(Some(Ok(out))) => {
            panic!(
                "BUG: got a duplicate BlockReconstructed for view {:?} after reconstruction was \
                 already completed",
                out.view
            );
        },
    }
}

/// Fetch the honest share for the voter with key index `i`.
fn honest_share(view: &TestView, i: u64) -> VidDisperseShare2<TestTypes> {
    let key = BLSPubKey::generated_from_seed_indexed([0u8; 32], i).0;
    view.vid_shares
        .iter()
        .find(|s| s.recipient_key == key)
        .unwrap()
        .clone()
}

/// Shares whose claimed commitment is inconsistent with their common data
/// must be rejected at intake (the common is the verification oracle for
/// weeding, so it has to be hash-bound to the commitment first) and must not
/// interfere with honest reconstruction of the same view.
#[tokio::test]
async fn test_inconsistent_commitment_shares_ignored() {
    let test_data = TestData::new(1).await;
    let view = &test_data.views[0];
    let mut reconstructor = VidReconstructor::<TestTypes>::new();

    // A valid commitment over different payload bytes: the honest common is
    // inconsistent with it.
    let param = &view.vid_shares[0].common.param;
    let other_payload = vec![0xa5u8; 64];
    let (wrong_commitment, _) = AvidmGf2Scheme::commit(
        param,
        &other_payload,
        std::iter::once(0..other_payload.len()),
    )
    .unwrap();

    // Feed threshold-plus shares claiming the wrong commitment; all must be
    // dropped at intake, so no reconstruction is ever attempted.
    for i in 0..THRESHOLD {
        let mut share = honest_share(view, i);
        share.payload_commitment = wrong_commitment;
        reconstructor.handle_vid_share(share, view.proposal.data.block_header.metadata);
    }
    let result =
        tokio::time::timeout(std::time::Duration::from_millis(500), reconstructor.next()).await;
    match result {
        Err(_) | Ok(None) => { /* timed out or no tasks — no attempt ran, good */ },
        Ok(Some(Ok(out))) => {
            panic!(
                "BUG: shares with a commitment inconsistent with their common produced a payload \
                 for view {:?}",
                out.view
            );
        },
        Ok(Some(Err(err))) => {
            panic!(
                "BUG: shares with a commitment inconsistent with their common triggered a \
                 reconstruction attempt for view {:?}",
                err.view
            );
        },
    }

    // The same voters can still contribute their honest shares.
    reconstructor.handle_vid_share(
        honest_share(view, 0),
        view.proposal.data.block_header.metadata,
    );
    for i in 1..THRESHOLD {
        reconstructor.handle_vid_share(honest_share(view, i), None);
    }
    let out = tokio::time::timeout(std::time::Duration::from_secs(5), reconstructor.next())
        .await
        .expect("reconstruction should complete in time")
        .expect("should produce a result")
        .expect("reconstruction from honest shares should succeed");
    assert_eq!(out.view, view.view_number);
    assert_eq!(
        out.payload_commitment,
        view.vid_shares[0].payload_commitment
    );
}

/// A share that claims the right commitment but carries content from a
/// different payload poisons the first decode. The reconstructor must weed
/// it out (reporting the offending voter), and recover automatically from
/// the verified remainder.
#[tokio::test]
async fn test_weeds_bad_shares_and_recovers() {
    let test_data = TestData::new(1).await;
    let view = &test_data.views[0];
    let mut reconstructor = VidReconstructor::<TestTypes>::new();

    let template = &view.vid_shares[0];
    let common = template.common.clone();
    let payload_commitment = template.payload_commitment;
    // The VID recovery threshold (in weight units), not the consensus vote
    // threshold.
    let recovery_threshold = common.param.recovery_threshold as u64;

    // Disperse a different (non-empty, so guaranteed different from the test
    // view's) payload with the same parameters and weights: the poison share
    // is structurally valid but its content fails verification against the
    // real commitment.
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

    // The poison voter is one whose honest share we never feed.
    let poison_voter = BLSPubKey::generated_from_seed_indexed([0u8; 32], 9).0;
    let poison = VidDisperseShare2::<TestTypes> {
        view_number: template.view_number,
        epoch: template.epoch,
        target_epoch: template.target_epoch,
        payload_commitment,
        share: other_shares.last().unwrap().clone(),
        recipient_key: poison_voter,
        common: common.clone(),
    };

    // Feed one less than the recovery threshold honestly, then the poison
    // share to trigger a (poisoned) reconstruction attempt.
    reconstructor.handle_vid_share(
        honest_share(view, 0),
        view.proposal.data.block_header.metadata,
    );
    for i in 1..recovery_threshold - 1 {
        reconstructor.handle_vid_share(honest_share(view, i), None);
    }
    reconstructor.handle_vid_share(poison, None);
    // More honest shares arrive while the poisoned attempt is in flight.
    reconstructor.handle_vid_share(honest_share(view, recovery_threshold - 1), None);
    reconstructor.handle_vid_share(honest_share(view, recovery_threshold), None);

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
    let out = tokio::time::timeout(std::time::Duration::from_secs(5), reconstructor.next())
        .await
        .expect("retry should complete in time")
        .expect("should produce a result")
        .expect("retry from verified shares should succeed");
    assert_eq!(out.view, view.view_number);
    assert_eq!(out.payload_commitment, payload_commitment);
}

/// A Byzantine voter replaying another voter's (valid) share contributes no
/// new shard coverage, so it must be dropped at intake: no reconstruction
/// attempt runs until genuinely new shares cover the recovery threshold.
#[tokio::test]
async fn test_replayed_share_does_not_fake_coverage() {
    let test_data = TestData::new(1).await;
    let view = &test_data.views[0];
    let mut reconstructor = VidReconstructor::<TestTypes>::new();

    let template = &view.vid_shares[0];
    let payload_commitment = template.payload_commitment;
    let recovery_threshold = template.common.param.recovery_threshold as u64;

    // The replayer is a voter whose honest share we never feed.
    let replayer = BLSPubKey::generated_from_seed_indexed([0u8; 32], 9).0;
    let mut replay = honest_share(view, 0);
    replay.recipient_key = replayer;

    reconstructor.handle_vid_share(
        honest_share(view, 0),
        view.proposal.data.block_header.metadata,
    );
    for i in 1..recovery_threshold - 1 {
        reconstructor.handle_vid_share(honest_share(view, i), None);
    }
    // The replay covers an already-covered shard range: dropped at intake,
    // coverage stays below the threshold and no attempt is triggered.
    reconstructor.handle_vid_share(replay, None);

    let result =
        tokio::time::timeout(std::time::Duration::from_millis(500), reconstructor.next()).await;
    match result {
        Err(_) | Ok(None) => { /* timed out or no tasks — no attempt ran, good */ },
        Ok(Some(Ok(out))) => panic!(
            "BUG: a replayed share produced a payload for view {:?}",
            out.view
        ),
        Ok(Some(Err(err))) => panic!(
            "BUG: a replayed share triggered a reconstruction attempt for view {:?}",
            err.view
        ),
    }

    // One more honest share provides the missing distinct range.
    reconstructor.handle_vid_share(honest_share(view, recovery_threshold - 1), None);
    let out = tokio::time::timeout(std::time::Duration::from_secs(5), reconstructor.next())
        .await
        .expect("reconstruction should complete in time")
        .expect("should produce a result")
        .expect("reconstruction should succeed once a new distinct share arrives");
    assert_eq!(out.view, view.view_number);
    assert_eq!(out.payload_commitment, payload_commitment);
}
