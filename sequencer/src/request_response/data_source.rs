//! This file contains the [`DataSource`] trait. This trait allows the [`RequestResponseProtocol`]
//! to calculate/derive a response for a specific request. In the confirmation layer the implementer
//! would be something like a [`FeeMerkleTree`] for fee catchup

use std::{marker::PhantomData, sync::Arc};

use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use espresso_types::{
    retain_accounts,
    traits::SequencerPersistence,
    v0_1::{RewardAccount, RewardMerkleTree},
    NodeState, PubKey, SeqTypes,
};
use hotshot::{traits::NodeImplementation, SystemContext};
use hotshot_query_service::data_source::storage::SqlStorage;
use hotshot_types::{
    data::ViewNumber,
    traits::{
        network::ConnectedNetwork,
        node_implementation::{ConsensusTime, Versions},
    },
    vote::HasViewNumber,
};
use itertools::Itertools;
use jf_merkle_tree::{
    ForgetableMerkleTreeScheme, ForgetableUniversalMerkleTreeScheme, LookupResult,
    MerkleTreeScheme, UniversalMerkleTreeScheme,
};
use request_response::data_source::DataSource as DataSourceTrait;

use super::request::{Request, Response};
use crate::{
    api::BlocksFrontier,
    catchup::{add_fee_accounts_to_state, add_reward_accounts_to_state, CatchupStorage},
};

/// A type alias for SQL storage
type Storage = Arc<SqlStorage>;

/// A type alias for the consensus handle
type Consensus<I, V> = Arc<SystemContext<SeqTypes, I, V>>;

#[derive(Clone)]
pub struct DataSource<
    I: NodeImplementation<SeqTypes>,
    V: Versions,
    N: ConnectedNetwork<PubKey>,
    P: SequencerPersistence,
> {
    /// The consensus handle
    pub consensus: Consensus<I, V>,
    /// The node's state
    pub node_state: NodeState,
    /// The storage
    pub storage: Option<Storage>,
    /// Phantom data
    pub phantom: PhantomData<(N, P)>,
}

/// Implement the trait that allows the [`RequestResponseProtocol`] to calculate/derive a response for a specific request
#[async_trait]
impl<
        I: NodeImplementation<SeqTypes>,
        V: Versions,
        N: ConnectedNetwork<PubKey>,
        P: SequencerPersistence,
    > DataSourceTrait<Request> for DataSource<I, V, N, P>
{
    async fn derive_response_for(&self, request: &Request) -> Result<Response> {
        match request {
            Request::Accounts(height, view, accounts) => {
                // Try to get accounts from memory first, then fall back to storage
                if let Some(state) = self.consensus.state(ViewNumber::new(*view)).await {
                    if let Ok(accounts) =
                        retain_accounts(&state.fee_merkle_tree, accounts.iter().copied())
                    {
                        return Ok(Response::Accounts(accounts));
                    }
                }

                // Fall back to storage
                let (merkle_tree, leaf) = self
                    .storage
                    .as_ref()
                    .with_context(|| "storage was not initialized")?
                    .get_accounts(&self.node_state, *height, ViewNumber::new(*view), accounts)
                    .await
                    .with_context(|| "failed to get accounts from sql storage")?;

                // If we successfully fetched accounts from storage, try to add them back into the in-memory
                // state.
                if let Err(err) = add_fee_accounts_to_state::<N, V, P>(
                    &self.consensus.consensus(),
                    &ViewNumber::new(*view),
                    accounts,
                    &merkle_tree,
                    leaf,
                )
                .await
                {
                    tracing::warn!(?view, "Cannot update fetched account state: {err:#}");
                }

                Ok(Response::Accounts(merkle_tree))
            },

            Request::Leaf(height) => {
                // Try to get the leaves from memory first, then fall back to storage
                let mut leaves = self.consensus.consensus().read().await.undecided_leaves();
                leaves.sort_by_key(|l| l.view_number());

                if let Some((position, mut last_leaf)) =
                    leaves.iter().find_position(|l| l.height() == *height)
                {
                    let mut leaf_chain = vec![last_leaf.clone()];
                    for leaf in leaves.iter().skip(position + 1) {
                        if leaf.justify_qc().view_number() == last_leaf.view_number() {
                            leaf_chain.push(leaf.clone());
                        } else {
                            continue;
                        }
                        if leaf.view_number() == last_leaf.view_number() + 1 {
                            // one away from decide
                            last_leaf = leaf;
                            break;
                        }
                        last_leaf = leaf;
                    }

                    // Make sure we got one more leaf to confirm the decide
                    for leaf in leaves
                        .iter()
                        .skip_while(|l| l.view_number() <= last_leaf.view_number())
                    {
                        if leaf.justify_qc().view_number() == last_leaf.view_number() {
                            leaf_chain.push(leaf.clone());
                            return Ok(Response::Leaf(leaf_chain));
                        }
                    }
                }

                // Fall back to storage
                let leaf_chain = self
                    .storage
                    .as_ref()
                    .with_context(|| "storage was not initialized")?
                    .get_leaf_chain(*height)
                    .await
                    .with_context(|| "failed to get leaf from sql storage")?;

                Ok(Response::Leaf(leaf_chain))
            },
            Request::ChainConfig(commitment) => {
                // Try to get the chain config from memory first, then fall back to storage
                let chain_config_from_memory = self.consensus.decided_state().await.chain_config;
                if chain_config_from_memory.commit() == *commitment {
                    if let Some(chain_config) = chain_config_from_memory.resolve() {
                        return Ok(Response::ChainConfig(chain_config));
                    }
                }

                // Fall back to storage
                Ok(Response::ChainConfig(
                    self.storage
                        .as_ref()
                        .with_context(|| "storage was not initialized")?
                        .get_chain_config(*commitment)
                        .await
                        .with_context(|| "failed to get chain config from sql storage")?,
                ))
            },
            Request::BlocksFrontier(height, view) => {
                // First try to respond from memory
                let blocks_frontier_from_memory: Option<Result<BlocksFrontier>> = self
                    .consensus
                    .state(ViewNumber::new(*view))
                    .await
                    .map(|state| {
                        let tree = &state.block_merkle_tree;
                        let frontier = tree.lookup(tree.num_leaves() - 1).expect_ok()?.1;
                        Ok(frontier)
                    });

                if let Some(Ok(blocks_frontier_from_memory)) = blocks_frontier_from_memory {
                    return Ok(Response::BlocksFrontier(blocks_frontier_from_memory));
                } else {
                    // If we can't get the blocks frontier from memory, fall through to storage
                    let blocks_frontier_from_storage = self
                        .storage
                        .as_ref()
                        .with_context(|| "storage was not initialized")?
                        .get_frontier(&self.node_state, *height, ViewNumber::new(*view))
                        .await
                        .with_context(|| "failed to get blocks frontier from sql storage")?;

                    Ok(Response::BlocksFrontier(blocks_frontier_from_storage))
                }
            },
            Request::RewardAccounts(height, view, accounts) => {
                // Try to get the reward accounts from memory first, then fall back to storage
                if let Some(state) = self.consensus.state(ViewNumber::new(*view)).await {
                    if let Ok(reward_accounts) =
                        retain_reward_accounts(&state.reward_merkle_tree, accounts.iter().copied())
                    {
                        return Ok(Response::RewardAccounts(reward_accounts));
                    }
                }

                // Fall back to storage
                let (merkle_tree, leaf) = self
                    .storage
                    .as_ref()
                    .with_context(|| "storage was not initialized")?
                    .get_reward_accounts(
                        &self.node_state,
                        *height,
                        ViewNumber::new(*view),
                        accounts,
                    )
                    .await
                    .with_context(|| "failed to get accounts from sql storage")?;

                // If we successfully fetched accounts from storage, try to add them back into the in-memory
                // state.
                if let Err(err) = add_reward_accounts_to_state::<N, V, P>(
                    &self.consensus.consensus(),
                    &ViewNumber::new(*view),
                    accounts,
                    &merkle_tree,
                    leaf,
                )
                .await
                {
                    tracing::warn!(?view, "Cannot update fetched account state: {err:#}");
                }

                Ok(Response::RewardAccounts(merkle_tree))
            },
        }
    }
}

/// Get a partial snapshot of the given reward state, which contains only the specified accounts.
///
/// Fails if one of the requested accounts is not represented in the original `state`.
pub fn retain_reward_accounts(
    state: &RewardMerkleTree,
    accounts: impl IntoIterator<Item = RewardAccount>,
) -> anyhow::Result<RewardMerkleTree> {
    let mut snapshot = RewardMerkleTree::from_commitment(state.commitment());
    for account in accounts {
        match state.universal_lookup(account) {
            LookupResult::Ok(elem, proof) => {
                // This remember cannot fail, since we just constructed a valid proof, and are
                // remembering into a tree with the same commitment.
                snapshot.remember(account, *elem, proof).unwrap();
            },
            LookupResult::NotFound(proof) => {
                // Likewise this cannot fail.
                snapshot.non_membership_remember(account, proof).unwrap()
            },
            LookupResult::NotInMemory => {
                bail!("missing account {account}");
            },
        }
    }

    Ok(snapshot)
}
