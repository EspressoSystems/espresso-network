//! V1 submit API.
//!
//! Mirrors the tide-disco endpoints defined in `crates/espresso/node/api/submit.toml`.

use async_trait::async_trait;
use serde::Serialize;

#[async_trait]
pub trait SubmitApi {
    type Transaction: serde::de::DeserializeOwned + Send + Sync + 'static;
    type TxHash: Serialize + Send + Sync + 'static;

    async fn submit(&self, tx: Self::Transaction) -> anyhow::Result<Self::TxHash>;
}
