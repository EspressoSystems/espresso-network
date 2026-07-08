//! V1 config API.
//!
//! Mirrors the tide-disco endpoints defined in `crates/espresso/node/api/config.toml`.

use async_trait::async_trait;
use serde::Serialize;

#[async_trait]
pub trait ConfigApi {
    type HotShotConfig: Serialize + Send + Sync + 'static;
    type RuntimeConfig: Serialize + Send + Sync + 'static;

    async fn hotshot_config(&self) -> anyhow::Result<Self::HotShotConfig>;

    async fn env(&self) -> anyhow::Result<Vec<String>>;

    async fn runtime_config(&self) -> anyhow::Result<Self::RuntimeConfig>;
}
