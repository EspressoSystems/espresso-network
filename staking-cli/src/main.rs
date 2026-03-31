use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    espresso_utils::env_compat::migrate_legacy_env_vars();
    staking_cli::run().await
}
