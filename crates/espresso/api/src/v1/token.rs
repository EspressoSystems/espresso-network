//! V1 token API.
//!
//! Mirrors the tide-disco endpoints defined in `crates/espresso/node/api/token.toml`.

use async_trait::async_trait;

#[async_trait]
pub trait TokenApi {
    async fn total_minted_supply(&self) -> anyhow::Result<String>;
    async fn circulating_supply(&self) -> anyhow::Result<String>;
    async fn circulating_supply_ethereum(&self) -> anyhow::Result<String>;
    async fn total_issued_supply(&self) -> anyhow::Result<String>;
    async fn total_reward_distributed(&self) -> anyhow::Result<String>;
}
