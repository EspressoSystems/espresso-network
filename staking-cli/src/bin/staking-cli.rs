#[tokio::main]
async fn main() -> anyhow::Result<()> {
    staking_cli::main().await
}
