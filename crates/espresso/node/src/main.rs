// NOTE: due to nextest eagerly compiling binaries we allow the build if we're
// not building with --release (without debug_assertions). There is
// unfortunately no good way to detect if a build is performed by nextest
// because nextest doesn't expose any build time env vars.
#[cfg(all(feature = "testing", not(debug_assertions), not(clippy)))]
compile_error!(
    "testing feature must not be enabled in release builds. If this is intentional, comment out \
     this check."
);

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    espresso_node::main().await
}
