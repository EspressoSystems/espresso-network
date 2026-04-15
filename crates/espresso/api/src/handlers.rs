//! Shared handler functions for reward API endpoints
//!
//! These functions implement the core logic for processing API requests:
//! 1. Parse proto request fields into internal types
//! 2. Call trait methods with parsed types
//! 3. Convert internal responses back to proto types
//!
//! Both gRPC and Axum handlers use these functions, ensuring consistent behavior.

use serialization_api::v2::*;

use crate::{error::ApiError, v2::RewardApi};

/// Handle get_reward_claim_input request
///
/// Parses the proto request, calls the trait method, and converts the response.
pub async fn get_reward_claim_input<S>(
    state: &S,
    request: GetRewardClaimInputRequest,
) -> Result<RewardClaimInput, ApiError>
where
    S: RewardApi,
{
    // Keep the original address string for proto conversion
    let address_string = request.address.clone();

    // Deserialize proto request fields
    let address = state
        .deserialize_address(&request.address)
        .map_err(ApiError::BadRequest)?;

    // Call trait method with parsed types
    let result = state
        .get_reward_claim_input(address)
        .await
        .map_err(ApiError::Internal)?;

    // Serialize response to proto
    state
        .serialize_reward_claim_input(&address_string, &result)
        .map_err(ApiError::Internal)
}

/// Handle get_reward_balance request
pub async fn get_reward_balance<S>(
    state: &S,
    request: GetRewardBalanceRequest,
) -> Result<RewardBalance, ApiError>
where
    S: RewardApi,
{
    // Deserialize proto request fields
    let address = state
        .deserialize_address(&request.address)
        .map_err(ApiError::BadRequest)?;

    // Call trait method with parsed types
    let result = state
        .get_reward_balance(address)
        .await
        .map_err(ApiError::Internal)?;

    // Serialize response to proto
    state
        .serialize_reward_balance(&result)
        .map_err(ApiError::Internal)
}

/// Handle get_reward_account_proof request
pub async fn get_reward_account_proof<S>(
    state: &S,
    request: GetRewardAccountProofRequest,
) -> Result<RewardAccountQueryDataV2, ApiError>
where
    S: RewardApi,
{
    // Deserialize proto request fields
    let address = state
        .deserialize_address(&request.address)
        .map_err(ApiError::BadRequest)?;

    // Call trait method with parsed types
    let result = state
        .get_reward_account_proof(address)
        .await
        .map_err(ApiError::Internal)?;

    // Serialize response to proto
    state
        .serialize_reward_account_query_data(&result)
        .map_err(ApiError::Internal)
}

/// Handle get_reward_balances request
pub async fn get_reward_balances<S>(
    state: &S,
    request: GetRewardBalancesRequest,
) -> Result<RewardBalances, ApiError>
where
    S: RewardApi,
{
    // No address deserialization needed for this endpoint

    // Call trait method (wrap as Internal error)
    let result = state
        .get_reward_balances(request.height, request.offset, request.limit)
        .await
        .map_err(ApiError::Internal)?;

    // Serialize response to proto
    state
        .serialize_reward_balances(&result)
        .map_err(ApiError::Internal)
}

/// Handle get_reward_merkle_tree_v2 request
pub async fn get_reward_merkle_tree_v2<S>(
    state: &S,
    request: GetRewardMerkleTreeRequest,
) -> Result<RewardMerkleTreeV2Data, ApiError>
where
    S: RewardApi,
{
    // No address deserialization needed for this endpoint

    // Call trait method (wrap as Internal error)
    let result = state
        .get_reward_merkle_tree_v2(request.height)
        .await
        .map_err(ApiError::Internal)?;

    // Serialize response to proto
    state
        .serialize_reward_merkle_tree_data(&result)
        .map_err(ApiError::Internal)
}
