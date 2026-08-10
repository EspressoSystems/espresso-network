//! V1 submit API.

use async_trait::async_trait;
use serde::Serialize;

#[async_trait]
pub trait SubmitApi {
    type Transaction: serde::de::DeserializeOwned + Send + Sync + 'static;
    type TxHash: Serialize + Send + Sync + 'static;

    async fn submit(&self, tx: Self::Transaction) -> anyhow::Result<Self::TxHash>;
}
