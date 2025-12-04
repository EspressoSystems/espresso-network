use sha2::{Sha256, Digest};
use hotshot_types::drb::compute_drb_result;
use hotshot_types::drb::DrbInput;
use hotshot_example_types::storage_types::TestStorage;
use hotshot_example_types::node_types::TestTypes;
use std::sync::Arc;

#[cfg(test)]
#[tokio::test(flavor = "multi_thread")]
async fn test_compute_drb_result() {
    let difficulty_level = 10;

    let drb_input = DrbInput {
        epoch: 0,
        iteration: 0,
        value: [0u8; 32],
        difficulty_level,
    };

    let mut expected_result = [0u8; 32];
    {
    let mut hash = drb_input.value.to_vec().clone();
        for _ in 0..difficulty_level {
            hash = Sha256::digest(hash).to_vec();
        }
    expected_result.copy_from_slice(&hash);
    }

    let storage = Arc::new(TestStorage::<TestTypes>::default());
    let actual_result = compute_drb_result(drb_input, storage).await;

    assert_eq!(expected_result, actual_result);
}

#[cfg(test)]
#[tokio::test(flavor = "multi_thread")]
async fn test_compute_drb_result_2() {
    let difficulty_level = 10;
    let drb_input = DrbInput {
        epoch: 0,
        iteration: 2,
        value: [0u8; 32],
        difficulty_level,
    };

    let mut expected_result = [0u8; 32];
    {
    let mut hash = drb_input.value.to_vec().clone();
        for _ in 2..difficulty_level {
            hash = Sha256::digest(hash).to_vec();
        }
    expected_result.copy_from_slice(&hash);
    }

    let storage = Arc::new(TestStorage::<TestTypes>::default());
    let actual_result = compute_drb_result(drb_input, storage).await;

    assert_eq!(expected_result, actual_result);
}
