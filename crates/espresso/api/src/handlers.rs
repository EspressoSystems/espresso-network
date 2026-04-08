//! Shared handler functions for reward API endpoints
//!
//! These functions implement the core logic for processing API requests:
//! 1. Parse proto request fields into internal types
//! 2. Call trait methods with parsed types
//! 3. Convert internal responses back to proto types
//!
//! Both gRPC and Axum handlers use these functions, ensuring consistent behavior.

use serialization_api::v2::*;
use crate::v2::RewardApi;

/// Handle get_reward_claim_input request
///
/// Parses the proto request, calls the trait method, and converts the response.
pub async fn get_reward_claim_input<S>(
    state: &S,
    request: GetRewardClaimInputRequest,
) -> anyhow::Result<RewardClaimInput>
where
    S: RewardApi,
{
    // Keep the original address string for proto conversion
    let address_string = request.address.clone();

    // Deserialize proto request fields
    let address = state.deserialize_address(&request.address)?;

    // Call trait method with parsed types
    let result = state
        .get_reward_claim_input(request.block_height, address)
        .await?;

    // Serialize response to proto, passing original address string
    state.serialize_reward_claim_input(&address_string, &result)
}

/// Handle get_reward_balance request
pub async fn get_reward_balance<S>(
    state: &S,
    request: GetRewardBalanceRequest,
) -> anyhow::Result<RewardBalance>
where
    S: RewardApi,
{
    // Deserialize proto request fields
    let address = state.deserialize_address(&request.address)?;

    // Call trait method with parsed types
    let result = state
        .get_reward_balance(request.height, address)
        .await?;

    // Serialize response to proto
    state.serialize_reward_balance(&result)
}

/// Handle get_latest_reward_balance request
pub async fn get_latest_reward_balance<S>(
    state: &S,
    request: GetLatestRewardBalanceRequest,
) -> anyhow::Result<RewardBalance>
where
    S: RewardApi,
{
    // Deserialize proto request fields
    let address = state.deserialize_address(&request.address)?;

    // Call trait method with parsed types
    let result = state
        .get_latest_reward_balance(address)
        .await?;

    // Serialize response to proto
    state.serialize_reward_balance(&result)
}

/// Handle get_reward_account_proof request
pub async fn get_reward_account_proof<S>(
    state: &S,
    request: GetRewardAccountProofRequest,
) -> anyhow::Result<RewardAccountQueryDataV2>
where
    S: RewardApi,
{
    // Deserialize proto request fields
    let address = state.deserialize_address(&request.address)?;

    // Call trait method with parsed types
    let result = state
        .get_reward_account_proof(request.height, address)
        .await?;

    // Serialize response to proto
    state.serialize_reward_account_query_data(&result)
}

/// Handle get_latest_reward_account_proof request
pub async fn get_latest_reward_account_proof<S>(
    state: &S,
    request: GetLatestRewardAccountProofRequest,
) -> anyhow::Result<RewardAccountQueryDataV2>
where
    S: RewardApi,
{
    // Deserialize proto request fields
    let address = state.deserialize_address(&request.address)?;

    // Call trait method with parsed types
    let result = state
        .get_latest_reward_account_proof(address)
        .await?;

    // Serialize response to proto
    state.serialize_reward_account_query_data(&result)
}

/// Handle get_reward_amounts request
pub async fn get_reward_amounts<S>(
    state: &S,
    request: GetRewardAmountsRequest,
) -> anyhow::Result<RewardAmounts>
where
    S: RewardApi,
{
    // No address deserialization needed for this endpoint

    // Call trait method
    let result = state
        .get_reward_amounts(request.height, request.offset, request.limit)
        .await?;

    // Serialize response to proto
    state.serialize_reward_amounts(&result)
}

/// Handle get_reward_merkle_tree_v2 request
pub async fn get_reward_merkle_tree_v2<S>(
    state: &S,
    request: GetRewardMerkleTreeRequest,
) -> anyhow::Result<RewardMerkleTreeV2Data>
where
    S: RewardApi,
{
    // No address deserialization needed for this endpoint

    // Call trait method
    let result = state
        .get_reward_merkle_tree_v2(request.height)
        .await?;

    // Serialize response to proto
    state.serialize_reward_merkle_tree_data(&result)
}
