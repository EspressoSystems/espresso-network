use sha2::{Sha256, Digest};
use hotshot_types::traits::storage::null_store_drb_progress_fn;
use hotshot_types::drb::compute_drb_result;
use hotshot_types::drb::DIFFICULTY_LEVEL;
use hotshot_types::drb::DrbInput;

#[cfg(test)]
#[tokio::test(flavor = "multi_thread")]
async fn test_compute_drb_result() {
    let drb_input = DrbInput {
        epoch: 0,
        iteration: 0,
        initial: [0u8; 32],
    };

    let mut expected_result = [0u8; 32];
    {
    let mut hash = drb_input.initial.to_vec().clone();
        for _ in 0..DIFFICULTY_LEVEL {
            hash = Sha256::digest(hash).to_vec();
        }
    expected_result.copy_from_slice(&hash);
    }

    let actual_result = compute_drb_result(drb_input, null_store_drb_progress_fn());

    assert_eq!(expected_result, actual_result);
}
