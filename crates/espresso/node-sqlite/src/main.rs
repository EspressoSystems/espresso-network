pub fn main() -> anyhow::Result<()> {
    let migrated_envs = espresso_utils::env_compat::migrate_legacy_env_vars();
    espresso_node::main_blocking(migrated_envs)
}
