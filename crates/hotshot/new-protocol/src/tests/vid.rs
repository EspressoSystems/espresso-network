use hotshot::types::BLSPubKey;
use hotshot_example_types::node_types::TestTypes;
use hotshot_types::traits::signature_key::SignatureKey;

use super::common::utils::TestData;
use crate::vid::VidReconstructor;

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
        Ok(Some(Err(()))) => { /* error, not a duplicate success */ },
        Ok(Some(Ok(out))) => {
            panic!(
                "BUG: got a duplicate BlockReconstructed for view {:?} — the reconstructor \
                 spawned multiple tasks for the same view",
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
        Ok(Some(Err(()))) => { /* error, not a duplicate success */ },
        Ok(Some(Ok(out))) => {
            panic!(
                "BUG: got a duplicate BlockReconstructed for view {:?} after reconstruction was \
                 already completed",
                out.view
            );
        },
    }
}
