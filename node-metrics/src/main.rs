use clap::Parser;
use hotshot::helpers::initialize_logging;
use node_metrics::{Options, run_standalone_service};

fn main() {
    let migrated_envs = espresso_utils::env_compat::migrate_legacy_env_vars();
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async_main(migrated_envs))
}

async fn async_main(migrated_envs: Vec<(&str, &str)>) {
    initialize_logging();
    espresso_utils::env_compat::log_migrated_env_vars(&migrated_envs);

    run_standalone_service(Options::parse()).await;
}
