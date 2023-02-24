use async_compatibility_layer::logging::{setup_backtrace, setup_logging};
use escargot::CargoBuild;
use hermez_adaptor::DemoZkEvmNode;
use std::path::Path;

#[async_std::test]
async fn test_permissionless_sequencer_example() {
    setup_logging();
    setup_backtrace();

    let _node = DemoZkEvmNode::start().await;

    let exit_status = CargoBuild::new()
        .manifest_path(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("Cargo.toml"),
        )
        .example("permissionless-sequencer")
        .current_release()
        .features("testing")
        .run()
        .expect("Failed to run cargo build")
        .command()
        .spawn()
        .expect("Failed to spawn permissionless-sequencer example")
        .wait()
        .expect("permissionless-sequencer example exited with error");

    assert!(exit_status.success());
}
