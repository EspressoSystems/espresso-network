use anyhow::Result;

use crate::{common::NativeDemo, smoke::assert_native_demo_works};

#[tokio::test(flavor = "multi_thread")]
async fn test_native_demo_pos_base() -> Result<()> {
    let _child = NativeDemo::run(
        None,
        Some(vec![(
            "ESPRESSO_SEQUENCER_PROCESS_COMPOSE_GENESIS_FILE".to_string(),
            "data/genesis/demo-pos-base.toml".to_string(),
        )]),
    );
    assert_native_demo_works(Default::default()).await
}
