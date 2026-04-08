use async_trait::async_trait;

/// Simplified data types for the API layer, free of domain-specific dependencies.
pub struct RewardClaimData {
    pub lifetime_rewards: String,
    /// ABI-encoded auth data as hex string (0x-prefixed)
    pub auth_data_hex: String,
}

pub enum DataSourceError {
    BadRequest(String),
    NotFound(String),
    Internal(String),
}

/// Minimal trait for API handlers. Keeps the api crate free of alloy, espresso-types, etc.
/// The conversion from domain types to these simple types happens in the node crate's trait impl.
#[async_trait]
pub trait DataSource: Clone + Send + Sync + 'static {
    async fn get_reward_claim_input(
        &self,
        block_height: u64,
        address: &str,
    ) -> Result<RewardClaimData, DataSourceError>;
}
