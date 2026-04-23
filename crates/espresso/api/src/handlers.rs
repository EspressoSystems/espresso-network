//! Shared handler functions for API endpoints,
//! used by both Axum and Tonic APIs.

use serialization_api::v2::*;

use crate::{
    error::ApiError,
    v2::{ConsensusApi, DataApi, RewardApi},
};

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

// Data API handlers

pub async fn get_namespace_proof<S>(
    state: &S,
    request: GetNamespaceProofRequest,
) -> Result<NamespaceProofResponse, ApiError>
where
    S: DataApi,
{
    let result = state
        .get_namespace_proof(request.namespace_id, request.block_height)
        .await
        .map_err(ApiError::Internal)?;

    state
        .serialize_namespace_proof(&result)
        .map_err(ApiError::Internal)
}

pub async fn get_namespace_proof_range<S>(
    state: &S,
    request: GetNamespaceProofRangeRequest,
) -> Result<NamespaceProofRangeResponse, ApiError>
where
    S: DataApi,
{
    let proofs = state
        .get_namespace_proof_range(request.namespace_id, request.from, request.until)
        .await
        .map_err(ApiError::Internal)?;

    let serialized_proofs = proofs
        .iter()
        .map(|proof| state.serialize_namespace_proof(proof))
        .collect::<Result<Vec<_>, _>>()
        .map_err(ApiError::Internal)?;

    Ok(NamespaceProofRangeResponse {
        proofs: serialized_proofs,
    })
}

pub async fn get_incorrect_encoding_proof<S>(
    state: &S,
    request: GetIncorrectEncodingProofRequest,
) -> Result<IncorrectEncodingProofResponse, ApiError>
where
    S: DataApi,
{
    let result = state
        .get_incorrect_encoding_proof(request.namespace_id, request.block_height)
        .await
        .map_err(ApiError::Internal)?;

    state
        .serialize_incorrect_encoding_proof(&result)
        .map_err(ApiError::Internal)
}

// Consensus API handlers

pub async fn get_state_certificate<S>(
    state: &S,
    request: GetStateCertificateRequest,
) -> Result<StateCertificateResponse, ApiError>
where
    S: ConsensusApi,
{
    let result = state
        .get_state_certificate(request.epoch)
        .await
        .map_err(ApiError::Internal)?;

    state
        .serialize_state_certificate(&result)
        .map_err(ApiError::Internal)
}

pub async fn get_stake_table<S>(
    state: &S,
    request: GetStakeTableRequest,
) -> Result<StakeTableResponse, ApiError>
where
    S: ConsensusApi,
{
    let result = state
        .get_stake_table(request.epoch)
        .await
        .map_err(ApiError::Internal)?;

    state
        .serialize_stake_table(&result)
        .map_err(ApiError::Internal)
}
