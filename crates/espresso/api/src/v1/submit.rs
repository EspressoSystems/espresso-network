//! V1 submit API.

use async_trait::async_trait;
use committable::Commitment;
use espresso_types::Transaction;

#[async_trait]
pub trait SubmitApi {
    async fn submit(&self, tx: Transaction) -> anyhow::Result<Commitment<Transaction>>;
}
