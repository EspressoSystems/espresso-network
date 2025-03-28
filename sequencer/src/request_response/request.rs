use std::num::NonZeroU64;

use anyhow::{Context, Result};
use async_trait::async_trait;
use committable::Commitment;
use committable::Committable;
use espresso_types::v0_1::RewardAccount;
use espresso_types::v0_1::RewardAccountProof;
use espresso_types::v0_1::RewardMerkleCommitment;
use espresso_types::v0_1::RewardMerkleTree;
use espresso_types::BlockMerkleTree;
use espresso_types::{
    v0_99::ChainConfig, EpochVersion, FeeAccount, FeeAccountProof, FeeAmount, FeeMerkleCommitment,
    Leaf2, PubKey, SeqTypes, SequencerVersions,
};
use hotshot_types::{message::UpgradeLock, utils::verify_epoch_root_chain, PeerConfig};
use jf_merkle_tree::prelude::{Sha3Digest, Sha3Node, UniversalMerkleTree};
use jf_merkle_tree::ForgetableMerkleTreeScheme;
use jf_merkle_tree::MerkleTreeScheme;
use request_response::{
    request::{Request as RequestTrait, Response as ResponseTrait},
    Serializable,
};
use serde::{Deserialize, Serialize};

use crate::api::BlocksFrontier;

// Some type aliases for readability
type Height = u64;
type ViewNumber = u64;
type EpochHeight = u64;
type StakeTable = Vec<PeerConfig<PubKey>>;
type SuccessThreshold = NonZeroU64;

/// The outermost request type. This an enum that contains all the possible requests that the
/// sequencer can make.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    /// A request for the accounts at a given height and view
    Accounts(Height, ViewNumber, Vec<FeeAccount>),
    /// A request for the leaf chain at a given height
    Leaf(Height),
    /// A request for a chain config with a particular commitment
    ChainConfig(Commitment<ChainConfig>),
    /// A request for the blocks frontier
    BlocksFrontier(Height, ViewNumber),
    /// A request for the reward accounts at a given height and view
    RewardAccounts(Height, ViewNumber, Vec<RewardAccount>),
}

/// The outermost response type. This an enum that contains all the possible responses that the
/// sequencer can make.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    /// A response for the accounts at a given height and view
    Accounts(UniversalMerkleTree<FeeAmount, Sha3Digest, FeeAccount, 256, Sha3Node>),
    /// A request for the leaf chain at a given height
    Leaf(Vec<Leaf2>),
    /// A response for a chain config with a particular commitment
    ChainConfig(ChainConfig),
    /// A response for the blocks frontier
    BlocksFrontier(BlocksFrontier),
    /// A response for the reward accounts at a given height and view
    RewardAccounts(RewardMerkleTree),
}

/// The context for validating the request. This is used to pass in any additional information
/// needed to validate the request.
#[derive(Clone)]
pub enum ValidationContext {
    None,
    /// The context for validating the accounts at a given height and view
    Accounts(FeeMerkleCommitment),
    /// The context for validating the leaf chain at a given height
    Leaf(EpochHeight, StakeTable, SuccessThreshold),
    /// The context for validating the blocks frontier
    BlocksFrontier(BlockMerkleTree),
    /// The context for validating the reward accounts at a given height and view
    RewardAccounts(RewardMerkleCommitment),
}

/// The result of the request. This is the type that will be returned to the caller of the request.
#[derive(Clone)]
pub enum Output {
    Accounts(Vec<FeeAccountProof>),
    Leaf(Box<Leaf2>),
    ChainConfig(ChainConfig),
    BlocksFrontier(BlockMerkleTree),
    RewardAccounts(Vec<RewardAccountProof>),
}

/// Implement the `RequestTrait` trait for the `Request` type. This tells the request response
/// protocol how to validate the request and what the response type is.
#[async_trait]
impl RequestTrait for Request {
    type Response = Response;

    async fn validate(&self) -> Result<()> {
        // Right now, all requests are valid
        Ok(())
    }
}

/// Implement the `ResponseTrait` trait for the `Response` type. This tells the request response
/// protocol how to validate the response and what the request type is.
#[async_trait]
impl ResponseTrait<Request> for Response {
    type ValidationContext = ValidationContext;
    type Output = Output;

    async fn validate(
        self,
        request: &Request,
        context: &Self::ValidationContext,
    ) -> Result<Self::Output> {
        // Match the type of the response and request
        match (request, self, context) {
            // An accounts request
            (
                Request::Accounts(_height, _view, accounts),
                Response::Accounts(fee_merkle_tree),
                ValidationContext::Accounts(fee_merkle_tree_commitment),
            ) => {
                // Verify the merkle proofs
                let mut proofs = Vec::new();
                for account in accounts {
                    let (proof, _) = FeeAccountProof::prove(&fee_merkle_tree, (*account).into())
                        .with_context(|| format!("response was missing account {account}"))?;
                    proof
                        .verify(fee_merkle_tree_commitment)
                        .with_context(|| format!("invalid proof for account {account}"))?;
                    proofs.push(proof);
                }

                Ok(Output::Accounts(proofs))
            },

            // A leaf chain request
            (
                Request::Leaf(_height),
                Response::Leaf(leaf_chain),
                ValidationContext::Leaf(epoch_height, stake_table, success_threshold),
            ) => {
                // Sort the leaf chain by view number and reverse it
                let mut leaf_chain = leaf_chain.clone();
                leaf_chain.sort_by_key(|l| l.view_number());
                leaf_chain.reverse();

                // Verify the leaf chain
                let leaf = verify_epoch_root_chain(
                    leaf_chain,
                    stake_table.clone(),
                    *success_threshold,
                    *epoch_height,
                    &UpgradeLock::<SeqTypes, SequencerVersions<EpochVersion, EpochVersion>>::new(),
                )
                .await
                .with_context(|| "leaf chain verification failed")?;

                Ok(Output::Leaf(Box::new(leaf)))
            },

            // A chain config request
            (
                Request::ChainConfig(commitment),
                Response::ChainConfig(chain_config),
                ValidationContext::None,
            ) => {
                // Make sure the commitments match
                if *commitment != chain_config.commit() {
                    return Err(anyhow::anyhow!("chain config commitment mismatch"));
                }

                Ok(Output::ChainConfig(chain_config))
            },

            // A blocks frontier request
            (
                Request::BlocksFrontier(_height, _view),
                Response::BlocksFrontier(blocks_frontier),
                ValidationContext::BlocksFrontier(block_merkle_tree),
            ) => {
                // Clone the merkle tree
                let mut block_merkle_tree = block_merkle_tree.clone();

                // Get the leaf element associated with the proof
                let leaf_elem = blocks_frontier
                    .elem()
                    .with_context(|| "provided frontier is missing leaf element")?;

                // Verify the block proof
                block_merkle_tree
                    .remember(
                        block_merkle_tree.num_leaves() - 1,
                        *leaf_elem,
                        blocks_frontier,
                    )
                    .with_context(|| "merkle tree verification failed")?;

                // Return the verified merkle tree
                Ok(Output::BlocksFrontier(block_merkle_tree))
            },

            // A reward accounts request
            (
                Request::RewardAccounts(_height, _view, accounts),
                Response::RewardAccounts(reward_merkle_tree),
                ValidationContext::RewardAccounts(reward_merkle_commitment),
            ) => {
                // Verify the merkle proofs
                let mut proofs = Vec::new();
                for account in accounts {
                    let (proof, _) =
                        RewardAccountProof::prove(&reward_merkle_tree, (*account).into())
                            .with_context(|| format!("response was missing account {account}"))?;
                    proof
                        .verify(reward_merkle_commitment)
                        .with_context(|| format!("invalid proof for account {account}"))?;
                    proofs.push(proof);
                }

                Ok(Output::RewardAccounts(proofs))
            },

            _ => Err(anyhow::anyhow!(
                "request, response, or validation context types mismatched"
            )),
        }
    }
}

/// Implement the `Serializable` trait for the `Request` type. This tells the request response
/// protocol how to serialize and deserialize the request
impl Serializable for Request {
    fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(&self).with_context(|| "failed to serialize")
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        bincode::deserialize(bytes).with_context(|| "failed to deserialize")
    }
}

/// Implement the `Serializable` trait for the `Response` type. This tells the request response
/// protocol how to serialize and deserialize the response.
impl Serializable for Response {
    fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).with_context(|| "failed to serialize")
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        bincode::deserialize(bytes).with_context(|| "failed to deserialize")
    }
}
