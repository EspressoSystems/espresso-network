//! V1 fee-state API extension.
//!
//! The merklized-state base routes (`get_path` by height and by commitment, and the snapshot
//! height) come from `hotshot-query-service`'s `merklized_state` router, mounted once per tree.
//! Only the fee balance lookup is Espresso's own.

use async_trait::async_trait;
use serde::Serialize;

#[async_trait]
pub trait FeeStateApiExtension {
    type FeeAmount: Serialize + Send + Sync + 'static;

    async fn get_fee_balance_latest(
        &self,
        address: String,
    ) -> anyhow::Result<Option<Self::FeeAmount>>;
}
