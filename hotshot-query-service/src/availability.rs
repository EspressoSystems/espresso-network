// Copyright (c) 2022 Espresso Systems (espressosys.com)
// This file is part of the HotShot Query Service library.
//
// This program is free software: you can redistribute it and/or modify it under the terms of the GNU
// General Public License as published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without
// even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// General Public License for more details.
// You should have received a copy of the GNU General Public License along with this program. If not,
// see <https://www.gnu.org/licenses/>.

//! Queries for HotShot chain state.
//!
//! The availability API provides an objective view of the HotShot blockchain. It provides access
//! only to normative data: that is, data which is agreed upon by all honest consensus nodes and
//! which is immutable. This means access to core consensus data structures including leaves,
//! blocks, and headers, where each query is pure and idempotent. This also means that it is
//! possible for a client to verify all of the information provided by this API, by running a
//! HotShot light client and downloading the appropriate evidence with each query.
//!
//! This API does not provide any queries which represent only the _current_ state of the chain or
//! may change over time, and it does not provide information for which there is not (yet) agreement
//! of a supermajority of consensus nodes. For information about the current dynamic state of
//! consensus and uncommitted state, try the [status](crate::status) API. For information about the
//! chain which is tabulated by this specific node and not subject to full consensus agreement, try
//! the [node](crate::node) API.

use std::time::Duration;

use hotshot_types::{
    data::Leaf, simple_certificate::QuorumCertificate, traits::node_implementation::NodeType,
};
use serde::{Deserialize, Serialize};

pub(crate) mod data_source;
mod fetch;
pub(crate) mod query_data;
pub mod router;
pub use data_source::*;
pub use fetch::Fetch;
pub use hotshot_query_service_types::availability::Error;
pub use query_data::*;

#[derive(Debug)]
pub struct Options {
    /// Timeout for failing requests due to missing data.
    ///
    /// If data needed to respond to a request is missing, it can (in some cases) be fetched from an
    /// external provider. This parameter controls how long the request handler will wait for
    /// missing data to be fetched before giving up and failing the request.
    pub fetch_timeout: Duration,

    /// The maximum number of small objects which can be loaded in a single range query.
    ///
    /// Currently small objects include leaves only. In the future this limit will also apply to
    /// headers, block summaries, and VID common, however
    /// * loading of headers and block summaries is currently implemented by loading the entire
    ///   block
    /// * imperfect VID parameter tuning means that VID common can be much larger than it should
    pub small_object_range_limit: usize,

    /// The maximum number of large objects which can be loaded in a single range query.
    ///
    /// Large objects include anything that _might_ contain a full payload or an object proportional
    /// in size to a payload. Note that this limit applies to the entire class of objects: we do not
    /// check the size of objects while loading to determine which limit to apply. If an object
    /// belongs to a class which might contain a large payload, the large object limit always
    /// applies.
    pub large_object_range_limit: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            fetch_timeout: Duration::from_millis(500),
            large_object_range_limit: 100,
            small_object_range_limit: 500,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(bound = "")]
pub struct Leaf1QueryData<Types: NodeType> {
    pub(crate) leaf: Leaf<Types>,
    pub(crate) qc: QuorumCertificate<Types>,
}

impl<Types: NodeType> Leaf1QueryData<Types> {
    pub fn new(leaf: Leaf<Types>, qc: QuorumCertificate<Types>) -> Self {
        Self { leaf, qc }
    }

    pub fn leaf(&self) -> &Leaf<Types> {
        &self.leaf
    }

    pub fn qc(&self) -> &QuorumCertificate<Types> {
        &self.qc
    }
}

#[cfg(test)]
mod test {
    use futures::StreamExt;

    use super::*;
    use crate::{
        data_source::{VersionedDataSource, storage::AvailabilityStorage},
        status::StatusDataSource,
        testing::{
            consensus::{MockNetwork, MockSqlDataSource},
            mocks::MockTypes,
        },
        types::HeightIndexed,
    };

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_header_endpoint() {
        // Create the consensus network.
        let mut network = MockNetwork::<MockSqlDataSource>::init().await;
        network.start().await;

        let ds = network.data_source();

        // Get the current block height and fetch header for some later block height
        // This fetch will only resolve when we receive a leaf or block for that block height
        let block_height = ds.block_height().await.unwrap();
        let fetch = ds
            .get_header(BlockId::<MockTypes>::Number(block_height + 25))
            .await;

        assert!(fetch.is_pending());
        let header = fetch.await;
        assert_eq!(header.height() as usize, block_height + 25);

        network.shut_down().await;
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_leaf_only_ds() {
        // Create the consensus network.
        let mut network = MockNetwork::<MockSqlDataSource>::init_with_leaf_ds().await;
        network.start().await;

        let ds = network.data_source();

        // Wait for some headers and leaves to be produced.
        ds.subscribe_headers(0)
            .await
            .take(5)
            .collect::<Vec<_>>()
            .await;
        ds.subscribe_leaves(5)
            .await
            .take(5)
            .collect::<Vec<_>>()
            .await;

        // Get the current block height and fetch a block at some later block height
        // This fetch will only resolve if we get a block notification
        // However, this block will never be stored
        let block_height = ds.block_height().await.unwrap();
        let target_block_height = block_height + 20;
        let fetch = ds
            .get_block(BlockId::<MockTypes>::Number(target_block_height))
            .await;

        assert!(fetch.is_pending());
        let block = fetch.await;
        assert_eq!(block.height() as usize, target_block_height);

        let mut tx = ds.read().await.unwrap();
        tx.get_block(BlockId::<MockTypes>::Number(target_block_height))
            .await
            .unwrap_err();
        drop(tx);

        network.shut_down().await;
    }
}
