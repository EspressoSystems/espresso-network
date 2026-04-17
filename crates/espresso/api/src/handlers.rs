//! Shared handler functions for reward API endpoints, 
//! used by both Axum and Tonic APIs.

use serialization_api::v2::*;

use crate::{error::ApiError, v2::RewardApi};

pub async fn get_reward_claim_input<S>(
    state: &S,
    request: GetRewardClaimInputRequest,
) -> Result<RewardClaimInput, ApiError>
where
    S: RewardApi,
{
    let address_string = request.address.clone();

    let address = state
        .deserialize_address(&request.address)
        .map_err(ApiError::BadRequest)?;

    let result = state
        .get_reward_claim_input(address)
        .await
        .map_err(ApiError::Internal)?;

    state
        .serialize_reward_claim_input(&address_string, &result)
        .map_err(ApiError::Internal)
}

pub async fn get_reward_balance<S>(
    state: &S,
    request: GetRewardBalanceRequest,
) -> Result<RewardBalance, ApiError>
where
    S: RewardApi,
{
    let address = state
        .deserialize_address(&request.address)
        .map_err(ApiError::BadRequest)?;

    let result = state
        .get_reward_balance(address)
        .await
        .map_err(ApiError::Internal)?;

    state
        .serialize_reward_balance(&result)
        .map_err(ApiError::Internal)
}

pub async fn get_reward_account_proof<S>(
    state: &S,
    request: GetRewardAccountProofRequest,
) -> Result<RewardAccountQueryDataV2, ApiError>
where
    S: RewardApi,
{
    let address = state
        .deserialize_address(&request.address)
        .map_err(ApiError::BadRequest)?;

    let result = state
        .get_reward_account_proof(address)
        .await
        .map_err(ApiError::Internal)?;

    state
        .serialize_reward_account_query_data(&result)
        .map_err(ApiError::Internal)
}

pub async fn get_reward_balances<S>(
    state: &S,
    request: GetRewardBalancesRequest,
) -> Result<RewardBalances, ApiError>
where
    S: RewardApi,
{
    let result = state
        .get_reward_balances(request.height, request.offset, request.limit)
        .await
        .map_err(ApiError::Internal)?;

    state
        .serialize_reward_balances(&result)
        .map_err(ApiError::Internal)
}

pub async fn get_reward_merkle_tree_v2<S>(
    state: &S,
    request: GetRewardMerkleTreeRequest,
) -> Result<RewardMerkleTreeV2Data, ApiError>
where
    S: RewardApi,
{
    let result = state
        .get_reward_merkle_tree_v2(request.height)
        .await
        .map_err(ApiError::Internal)?;

    state
        .serialize_reward_merkle_tree_data(&result)
        .map_err(ApiError::Internal)
}
