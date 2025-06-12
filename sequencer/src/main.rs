#[cfg(feature = "embedded-db")]
compile_error!("The sequencer binary is not compatible with the 'embedded-db' feature, compile the sequencer-sqlite crate instead");

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    sequencer::main().await
}
