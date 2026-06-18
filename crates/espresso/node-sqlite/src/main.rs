pub fn main() -> anyhow::Result<()> {
    let migrated_envs = espresso_utils::env_compat::migrate_legacy_env_vars();
    tokio::runtime::Runtime::new()?.block_on(espresso_node::main(migrated_envs))
}
