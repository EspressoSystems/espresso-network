// NOTE: due to nextest eagerly compiling binaries we allow the build if we're
// not building with --release (without debug_assertions). There is
// unfortunately no good way to detect if a build is performed by nextest
// because nextest doesn't expose any build time env vars.
#[cfg(all(feature = "embedded-db", not(debug_assertions), not(clippy)))]
compile_error!(
    r#"
The `embedded-db` feature is enabled, but the sequencer binary is not compatible
with the `embedded-db` feature. Aborting build.

If the intention is to build the sequencer-sqlite binary run `cargo build -p
sequencer-sqlite` instead.

To build the (postgres) sequencer binary make sure the embedded-db feature is
disabled and the sequencer-sqlite crate is **not** part of the build. Including
the sequencer-sqlite crate in the build will enable the `embedded-db` feature
globally.

By default the sequencer-sqlite workspace crate is excluded from cargo
invocations because it's not a default workspace member. Avoid using the
`--workspace` cargo flag unless enabling the `embedded-db` feature (via
inclusion of the sequencer-sqlite crate in the build) is intended.

Similarly, avoid enabling the `embedded-db` feature by using passing the cargo
flag `--all-features` when building the sequencer binary target.
"#
);

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    // If we compiled with the embedded-db feature **and** are running it now
    // something is wrong.
    #[cfg(feature = "embedded-db")]
    {
        panic!(
            r#"The sequencer binary is not compatible with the embedded-db feature.
     Please build the sequencer-sqlite binary instead."#
        );
    }

    #[cfg(not(feature = "embedded-db"))]
    {
        sequencer::main().await
    }
}
