//! This file contains the [`DataSource`] trait. This trait allows the [`RequestResponseProtocol`]
//! to calculate/derive a response for a specific request. In the confirmation layer the implementer
//! would be something like a [`FeeMerkleTree`] for fee catchup

use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use espresso_types::NodeState;
use hotshot_query_service::data_source::storage::SqlStorage;
use hotshot_types::{data::ViewNumber, traits::node_implementation::ConsensusTime};
use request_response::data_source::DataSource as DataSourceTrait;

use crate::catchup::CatchupStorage;

use super::request::{Request, Response};

/// A type alias for SQL storage
type Storage = Arc<SqlStorage>;

#[derive(Clone, Debug)]
pub struct DataSource {
    /// The node's state
    pub node_state: NodeState,
    /// The storage
    pub storage: Option<Storage>,
}

/// Implement the trait that allows the [`RequestResponseProtocol`] to calculate/derive a response for a specific request
#[async_trait]
impl DataSourceTrait<Request> for DataSource {
    async fn derive_response_for(&self, request: &Request) -> Result<Response> {
        match request {
            Request::Accounts(height, view, accounts) => Ok(Response::Accounts(
                self.storage
                    .as_ref()
                    .with_context(|| "storage was not initialized")?
                    .get_accounts(&self.node_state, *height, ViewNumber::new(*view), accounts)
                    .await
                    .with_context(|| "failed to get accounts from sql storage")?
                    .0,
            )),
            Request::Leaf(height) => Ok(Response::Leaf(
                self.storage
                    .as_ref()
                    .with_context(|| "storage was not initialized")?
                    .get_leaf_chain(*height)
                    .await
                    .with_context(|| "failed to get leaf from sql storage")?,
            )),
            Request::ChainConfig(commitment) => Ok(Response::ChainConfig(
                self.storage
                    .as_ref()
                    .with_context(|| "storage was not initialized")?
                    .get_chain_config(*commitment)
                    .await
                    .with_context(|| "failed to get chain config from sql storage")?,
            )),
            Request::BlocksFrontier(height, view) => Ok(Response::BlocksFrontier(
                self.storage
                    .as_ref()
                    .with_context(|| "storage was not initialized")?
                    .get_frontier(&self.node_state, *height, ViewNumber::new(*view))
                    .await
                    .with_context(|| "failed to get blocks frontier from sql storage")?,
            )),
            Request::RewardAccounts(height, view, accounts) => Ok(Response::RewardAccounts(
                self.storage
                    .as_ref()
                    .with_context(|| "storage was not initialized")?
                    .get_reward_accounts(
                        &self.node_state,
                        *height,
                        ViewNumber::new(*view),
                        accounts,
                    )
                    .await
                    .with_context(|| "failed to get reward accounts from sql storage")?
                    .0,
            )),
        }
    }
}
