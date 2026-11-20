//! The catchup service: leaves from the store, and state from the heights the API service has
//! applied, for consensus nodes that are behind. They check every answer themselves.

use std::sync::Arc;

use chain_store::ChainStore;
use committable::Committable;
use consensus_node::proto::{
    AccountsRequest, ChainConfigRequest, Encoded, EpochRequest, HeightRequest, LeafChainResponse,
    catchup_service_server::CatchupService,
};
use espresso_api::error::to_status;
use espresso_types::{FeeAccount, FeeAccountProof, NodeState, ValidatedState};
use hotshot_types::{data::EpochNumber, utils::root_block_in_epoch};
use jf_merkle_tree_compat::MerkleTreeScheme;
use serde::Serialize;
use tonic::{Request, Response, Status};

use crate::chain_state::ChainStates;

/// How many leaves after the one asked for a chain may take to reach a `Certificate2`.
const MAX_CHAIN: u64 = 64;

/// What the catchup service answers from.
pub struct Catchup {
    store: ChainStore,
    chain: ChainStates,
    node: NodeState,
}

impl Catchup {
    pub fn new(store: ChainStore, chain: ChainStates, node: NodeState) -> Self {
        Self { store, chain, node }
    }

    /// The state after the block at `height`, which must be among those kept.
    fn state(&self, height: u64) -> Result<Arc<ValidatedState>, Status> {
        self.chain.at(height).ok_or_else(|| {
            Status::not_found(format!(
                "no state kept for height {height}; the newest is {:?}",
                self.chain.newest()
            ))
        })
    }
}

fn encoded<T: Serialize>(value: &T) -> Result<Response<Encoded>, Status> {
    let data = bincode::serialize(value).map_err(|err| Status::internal(err.to_string()))?;
    Ok(Response::new(Encoded { data }))
}

#[tonic::async_trait]
impl CatchupService for Catchup {
    async fn get_leaf_chain(
        &self,
        request: Request<HeightRequest>,
    ) -> Result<Response<LeafChainResponse>, Status> {
        let height = request.into_inner().height;
        let mut leaves = Vec::new();
        // The height comes from a peer, and can be any number.
        for at in height..height.saturating_add(MAX_CHAIN) {
            let item = self
                .store
                .item(at)
                .await
                .map_err(to_status)?
                .ok_or_else(|| Status::not_found(format!("no leaf at height {at}")))?;
            leaves
                .push(bincode::serialize(&item.leaf).map_err(|e| Status::internal(e.to_string()))?);
            if let Some(cert2) = self.store.cert2(at).await.map_err(to_status)? {
                let cert2 =
                    bincode::serialize(&cert2).map_err(|e| Status::internal(e.to_string()))?;
                return Ok(Response::new(LeafChainResponse { leaves, cert2 }));
            }
        }
        Err(Status::not_found(format!(
            "no cert2 within {MAX_CHAIN} heights of {height}"
        )))
    }

    async fn get_fee_accounts(
        &self,
        request: Request<AccountsRequest>,
    ) -> Result<Response<Encoded>, Status> {
        let r = request.into_inner();
        let state = self.state(r.height)?;
        let proofs = r
            .accounts
            .iter()
            .map(|bytes| {
                let account: FeeAccount = bincode::deserialize(bytes)
                    .map_err(|_| Status::invalid_argument("an undecodable account"))?;
                FeeAccountProof::prove(&state.fee_merkle_tree, account.0)
                    .map(|(proof, _)| proof)
                    .ok_or_else(|| Status::internal(format!("no proof of account {account}")))
            })
            .collect::<Result<Vec<_>, _>>()?;
        encoded(&proofs)
    }

    async fn get_blocks_frontier(
        &self,
        request: Request<HeightRequest>,
    ) -> Result<Response<Encoded>, Status> {
        let height = request.into_inner().height;
        let state = self.state(height)?;
        let tree = &state.block_merkle_tree;
        let last = tree
            .num_leaves()
            .checked_sub(1)
            .ok_or_else(|| Status::not_found("the block tree is empty"))?;
        let (_, frontier) = tree
            .lookup(last)
            .expect_ok()
            .map_err(|_| Status::internal("the block tree forgot its frontier"))?;
        encoded(&frontier)
    }

    async fn get_reward_balances_v2(
        &self,
        request: Request<HeightRequest>,
    ) -> Result<Response<Encoded>, Status> {
        let state = self.state(request.into_inner().height)?;
        let tree = &state.reward_merkle_tree_v2;
        let balances: Vec<_> = tree
            .iter()
            .map(|(account, amount)| (*account, *amount))
            .collect();
        if balances.len() as u64 != tree.num_leaves() {
            return Err(Status::internal("the reward tree is missing accounts"));
        }
        encoded(&balances)
    }

    async fn get_chain_config(
        &self,
        request: Request<ChainConfigRequest>,
    ) -> Result<Response<Encoded>, Status> {
        let commitment = request.into_inner().commitment;
        let latest = self
            .chain
            .latest()
            .and_then(|(_, state)| state.chain_config.resolve());
        let known = [
            Some(self.node.chain_config),
            Some(self.node.genesis_chain_config),
            latest,
        ]
        .into_iter()
        .flatten()
        .chain(
            self.node
                .upgrades
                .values()
                .filter_map(|upgrade| upgrade.upgrade_type.chain_config()),
        );
        for config in known {
            if config.commit().to_string() == commitment {
                return encoded(&config);
            }
        }
        Err(Status::not_found(format!("no chain config {commitment}")))
    }

    async fn get_state_cert(
        &self,
        request: Request<EpochRequest>,
    ) -> Result<Response<Encoded>, Status> {
        let epoch = request.into_inner().epoch;
        let epoch_height = self.node.epoch_height.unwrap_or_default();
        let root = root_block_in_epoch(epoch, epoch_height);
        match self.store.state_cert(root).await.map_err(to_status)? {
            Some(cert) => encoded(&cert),
            None => Err(Status::not_found(format!(
                "no state certificate of epoch {}",
                EpochNumber::new(epoch)
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use chain_store::mem::MemBackend;

    use super::*;
    use crate::chain_state;

    /// A leaf chain asked for at any height is answered, by not found if nothing is there.
    #[tokio::test]
    async fn a_leaf_chain_at_the_last_height_is_not_found() {
        let store = ChainStore::new(MemBackend::default());
        let node = NodeState::mock();
        let (_, states) = chain_state::follower(store.clone(), node.clone());
        let catchup = Catchup::new(store, states, node);

        let status = catchup
            .get_leaf_chain(Request::new(HeightRequest { height: u64::MAX }))
            .await
            .expect_err("nothing is stored");
        assert_eq!(status.code(), tonic::Code::NotFound);
    }
}
