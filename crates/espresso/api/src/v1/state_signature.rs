//! V1 state-signature API.
//!
//! Mirrors the tide-disco endpoints defined in `crates/espresso/node/api/state_signature.toml`.

use async_trait::async_trait;
use serde::Serialize;

#[async_trait]
pub trait StateSignatureApi {
    type Signature: Serialize + Send + Sync + 'static;

    async fn get_state_signature(&self, height: u64) -> anyhow::Result<Self::Signature>;
}
