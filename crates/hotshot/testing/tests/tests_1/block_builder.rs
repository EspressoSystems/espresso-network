// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use hotshot_builder_api::v0_1::block_info::AvailableBlockData;
use hotshot_example_types::{
    block_types::{TestBlockPayload, TestMetadata, TestTransaction},
    node_types::TestTypes,
};
use hotshot_task_impls::builder::{BuilderClient, BuilderClientError};
use hotshot_testing::block_builder::{
    BuilderTask, RandomBuilderImplementation, TestBuilderImplementation,
};
use hotshot_types::{
    data::vid_commitment,
    network::RandomBuilderConfig,
    traits::{node_implementation::NodeType, signature_key::SignatureKey, BlockPayload},
};
use tide_disco::Url;
use tokio::time::sleep;
use vbs::version::StaticVersion;

#[cfg(test)]
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn test_random_block_builder() {
    use hotshot_example_types::node_types::TestVersions;
    use vbs::version::Version;

    let port = portpicker::pick_unused_port().expect("No free ports");
    let api_url = Url::parse(&format!("http://localhost:{port}")).expect("Valid URL");
    let task: Box<dyn BuilderTask<TestTypes>> = RandomBuilderImplementation::start(
        1,
        api_url.clone(),
        RandomBuilderConfig {
            blocks_per_second: u32::MAX,
            ..Default::default()
        },
        HashMap::new(),
    )
    .await;
    task.start(Box::new(futures::stream::empty()));

    let builder_started = Instant::now();

    let client: BuilderClient<TestTypes, StaticVersion<0, 1>> = BuilderClient::new(api_url);
    assert!(client.connect(Duration::from_millis(100)).await);

    let (pub_key, private_key) =
        <TestTypes as NodeType>::SignatureKey::generated_from_seed_indexed([0_u8; 32], 0);
    let signature = <TestTypes as NodeType>::SignatureKey::sign(&private_key, &[0_u8; 32])
        .expect("Failed to create dummy signature");
    let dummy_view_number = 0u64;

    let mut blocks = loop {
        // Test getting blocks
        let blocks = client
            .available_blocks(
                vid_commitment::<TestVersions>(&[], &[], 1, Version { major: 0, minor: 0 }),
                dummy_view_number,
                pub_key,
                &signature,
            )
            .await
            .expect("Failed to get available blocks");

        if !blocks.is_empty() {
            break blocks;
        };

        // Wait for at least one block to be built
        sleep(Duration::from_millis(20)).await;

        if builder_started.elapsed() > Duration::from_secs(2) {
            panic!("Builder failed to provide blocks in two seconds");
        }
    };

    let _: AvailableBlockData<TestTypes> = client
        .claim_block(
            blocks.pop().unwrap().block_hash,
            dummy_view_number,
            pub_key,
            &signature,
        )
        .await
        .expect("Failed to claim block");

    // Test claiming non-existent block
    let commitment_for_non_existent_block =
        <TestBlockPayload as BlockPayload<TestTypes>>::builder_commitment(
            &TestBlockPayload {
                transactions: vec![TestTransaction::new(vec![0; 1])],
            },
            &TestMetadata {
                num_transactions: 1,
            },
        );

    let result = client
        .claim_block(
            commitment_for_non_existent_block,
            dummy_view_number,
            pub_key,
            &signature,
        )
        .await;
    assert!(matches!(result, Err(BuilderClientError::BlockNotFound)));
}
