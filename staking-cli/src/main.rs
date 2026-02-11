use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    staking_cli::run().await
}
