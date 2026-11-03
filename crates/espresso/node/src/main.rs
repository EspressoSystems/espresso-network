// NOTE: due to nextest eagerly compiling binaries we allow the build if we're
// not building with --release (without debug_assertions). There is
// unfortunately no good way to detect if a build is performed by nextest
// because nextest doesn't expose any build time env vars.
#[cfg(all(feature = "testing", not(debug_assertions), not(clippy)))]
compile_error!(
    "testing feature must not be enabled in release builds. If this is intentional, comment out \
     this check."
);

pub fn main() -> anyhow::Result<()> {
    let migrated_envs = espresso_utils::env_compat::migrate_legacy_env_vars();
    let rt = tokio::runtime::Runtime::new()?;
    let result = rt.block_on(espresso_node::main(migrated_envs));
    // Bound teardown so a stuck blocking-pool task cannot hang exit indefinitely.
    rt.shutdown_timeout(std::time::Duration::from_secs(5));
    result
}
