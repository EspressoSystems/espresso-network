//! V1 status API.
//!
//! Mirrors the tide-disco endpoints defined in `hotshot-query-service/api/status.toml`.

use async_trait::async_trait;

#[async_trait]
pub trait StatusApi {
    async fn block_height(&self) -> anyhow::Result<u64>;
    async fn success_rate(&self) -> anyhow::Result<f64>;
    async fn time_since_last_decide(&self) -> anyhow::Result<u64>;

    async fn metrics(&self) -> anyhow::Result<String>;
}
