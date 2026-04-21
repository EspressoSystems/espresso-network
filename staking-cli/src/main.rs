use anyhow::Result;

fn main() -> Result<()> {
    let migrated_envs = espresso_utils::env_compat::migrate_legacy_env_vars();
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async_main(migrated_envs))
}

async fn async_main(migrated_envs: Vec<(&str, &str)>) -> Result<()> {
    staking_cli::run(migrated_envs).await
}
