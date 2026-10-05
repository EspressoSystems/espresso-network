//! V1 state-signature API.

use async_trait::async_trait;
use serde::Serialize;

#[async_trait]
pub trait StateSignatureApi {
    type Signature: Serialize + Send + Sync + 'static;

    async fn get_state_signature(&self, height: u64) -> anyhow::Result<Self::Signature>;
}
