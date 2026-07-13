#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

pub fn main() -> anyhow::Result<()> {
    let migrated_envs = espresso_utils::env_compat::migrate_legacy_env_vars();
    let rt = tokio::runtime::Runtime::new()?;
    let result = rt.block_on(espresso_node::main(migrated_envs));
    // Bound teardown so a stuck blocking-pool task cannot hang exit indefinitely.
    rt.shutdown_timeout(std::time::Duration::from_secs(5));
    result
}
