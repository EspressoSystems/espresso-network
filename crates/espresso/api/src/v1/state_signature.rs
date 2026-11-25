//! V1 state-signature API.

use async_trait::async_trait;
use hotshot_types::light_client::LCV3StateSignatureRequestBody;

#[async_trait]
pub trait StateSignatureApi {
    async fn get_state_signature(
        &self,
        height: u64,
    ) -> anyhow::Result<LCV3StateSignatureRequestBody>;
}
