#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    espresso_node::main().await
}
