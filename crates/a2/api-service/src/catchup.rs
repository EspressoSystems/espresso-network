//! The catchup service: leaves from the store, and the state of the newest heights as the
//! ingester stored it, for consensus nodes that are behind. They check every answer themselves.

use catchup_api::proto::{
    AccountsRequest, ChainConfigRequest, Encoded, EpochRequest, HeightRequest, LeafChainResponse,
    catchup_service_server::CatchupService,
};
use chain_api::{StateRow, Tables};
use chain_store::ChainStore;
use espresso_api::error::to_status;
use espresso_types::{FEE_MERKLE_TREE_HEIGHT, FeeAccount, FeeAccountProof, FeeMerkleTree};
use hotshot_types::{data::EpochNumber, utils::root_block_in_epoch};
use serde::Serialize;
use tokio::sync::OnceCell;
use tonic::{Request, Response, Status};

/// How many leaves after the one asked for a chain may take to reach a `Certificate2`.
const MAX_CHAIN: u64 = 64;

/// What the catchup service answers from.
pub struct Catchup {
    store: ChainStore,
    tables: Tables,
    epoch_height: u64,

    /// The fee tree, which does not change under the new protocol, built on first use from the
    /// accounts the ingester stored.
    fees: OnceCell<FeeMerkleTree>,
}

impl Catchup {
    pub fn new(store: ChainStore, epoch_height: u64) -> Self {
        Self {
            tables: Tables::new(store.clone()),
            store,
            epoch_height,
            fees: OnceCell::new(),
        }
    }

    /// What the state after `height` holds besides its header's roots, which the ingester keeps
    /// for the newest heights only.
    async fn state(&self, height: u64) -> Result<StateRow, Status> {
        self.tables
            .state(height)
            .await
            .map_err(to_status)?
            .ok_or_else(|| Status::not_found(format!("no state kept for height {height}")))
    }

    async fn fee_tree(&self) -> Result<&FeeMerkleTree, Status> {
        self.fees
            .get_or_try_init(|| async {
                let accounts = self
                    .tables
                    .fee_accounts()
                    .await
                    .map_err(to_status)?
                    .ok_or_else(|| Status::unavailable("the fee accounts are not stored yet"))?;
                FeeMerkleTree::from_kv_set(FEE_MERKLE_TREE_HEIGHT, accounts)
                    .map_err(|err| Status::internal(format!("building the fee tree: {err}")))
            })
            .await
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
        // The fee tree is the same at every height; the height must still be one kept.
        self.state(r.height).await?;
        let tree = self.fee_tree().await?;
        let proofs = r
            .accounts
            .iter()
            .map(|bytes| {
                let account: FeeAccount = bincode::deserialize(bytes)
                    .map_err(|_| Status::invalid_argument("an undecodable account"))?;
                FeeAccountProof::prove(tree, account.0)
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
        let frontier = self
            .state(height)
            .await?
            .frontier
            .ok_or_else(|| Status::not_found("the block tree is empty"))?;
        encoded(&frontier)
    }

    async fn get_reward_balances_v2(
        &self,
        request: Request<HeightRequest>,
    ) -> Result<Response<Encoded>, Status> {
        let height = request.into_inner().height;
        let at = self.state(height).await?.rewards_at;
        let balances = self
            .tables
            .rewards(at)
            .await
            .map_err(to_status)?
            // Pruned since the row was read: the height is no longer kept.
            .ok_or_else(|| Status::not_found(format!("no state kept for height {height}")))?;
        encoded(&balances)
    }

    async fn get_chain_config(
        &self,
        request: Request<ChainConfigRequest>,
    ) -> Result<Response<Encoded>, Status> {
        let commitment = request.into_inner().commitment;
        match self
            .tables
            .chain_config(&commitment)
            .await
            .map_err(to_status)?
        {
            Some(config) => encoded(&config),
            None => Err(Status::not_found(format!("no chain config {commitment}"))),
        }
    }

    async fn get_state_cert(
        &self,
        request: Request<EpochRequest>,
    ) -> Result<Response<Encoded>, Status> {
        let epoch = request.into_inner().epoch;
        let root = root_block_in_epoch(epoch, self.epoch_height);
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

    /// A leaf chain asked for at any height is answered, by not found if nothing is there.
    #[tokio::test]
    async fn a_leaf_chain_at_the_last_height_is_not_found() {
        let store = ChainStore::new(MemBackend::default());
        let catchup = Catchup::new(store, 10);

        let status = catchup
            .get_leaf_chain(Request::new(HeightRequest { height: u64::MAX }))
            .await
            .expect_err("nothing is stored");
        assert_eq!(status.code(), tonic::Code::NotFound);
    }

    /// State is answered from the rows the ingester stored, for the heights kept only.
    #[tokio::test]
    async fn answers_from_the_stored_state() {
        let store = ChainStore::new(MemBackend::default());
        let tables = Tables::new(store.clone());
        let config = espresso_types::v0_3::ChainConfig::default();
        let row = StateRow {
            frontier: None,
            chain_config: config,
            rewards_at: 4,
        };
        tables
            .put_state(4, &row, Some(&Vec::new()), 4, 4)
            .await
            .unwrap();
        tables.put_state(5, &row, None, 4, 4).await.unwrap();
        tables.put_chain_config(&config).await.unwrap();
        tables.put_fee_accounts(&[]).await.unwrap();
        let catchup = Catchup::new(store, 10);
        let height = |height| Request::new(HeightRequest { height });

        let balances = catchup.get_reward_balances_v2(height(5)).await.unwrap();
        let balances: chain_api::RewardSet =
            bincode::deserialize(&balances.into_inner().data).unwrap();
        assert!(balances.is_empty());
        let status = catchup
            .get_reward_balances_v2(height(6))
            .await
            .expect_err("no state kept for height 6");
        assert_eq!(status.code(), tonic::Code::NotFound);
        let status = catchup
            .get_blocks_frontier(height(5))
            .await
            .expect_err("the block tree is empty");
        assert_eq!(status.code(), tonic::Code::NotFound);
        catchup
            .get_fee_accounts(Request::new(AccountsRequest {
                height: 5,
                accounts: Vec::new(),
            }))
            .await
            .unwrap();
        let commitment = committable::Committable::commit(&config).to_string();
        catchup
            .get_chain_config(Request::new(ChainConfigRequest { commitment }))
            .await
            .unwrap();
    }
}
