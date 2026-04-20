fn main() -> anyhow::Result<()> {
    let migrated_envs = espresso_utils::env_compat::migrate_legacy_env_vars();
    tokio::runtime::Runtime::new()?.block_on(async { deployment_info::run(migrated_envs).await })
}
