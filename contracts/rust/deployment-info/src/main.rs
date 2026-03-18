#[tokio::main]
async fn main() -> anyhow::Result<()> {
    deployment_info::run().await
}
