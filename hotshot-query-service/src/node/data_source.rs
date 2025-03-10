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

//! Data for the [`node`](super) API.
//!
//! This module is just an alternative view of the same data provided by the
//! [`availability`](crate::availability) API. It provides more insight into what data the node
//! actually has at present, as opposed to trying to present a perfect view of an abstract chain,
//! fetching data from other sources as needed. It is also more liberal with provided aggregate
//! counts and statistics which may be inaccurate if data is missing.
//!
//! Due to this relationship with the availability module, this module has its own [data source
//! trait](`NodeDataSource`) but not its own update trait. The node data source is expected to read
//! its data from the same underlying database as the availability API, and as such the data is
//! updated implicitly via the [availability API update
//! trait](crate::availability::UpdateAvailabilityData).

use std::ops::RangeBounds;

use async_trait::async_trait;
use derivative::Derivative;
use derive_more::From;
use hotshot_types::{data::VidShare, traits::node_implementation::NodeType};

use super::query_data::{BlockHash, BlockId, SyncStatus, TimeWindowQueryData};
use crate::{Header, QueryResult};

#[derive(Derivative, From)]
#[derivative(Copy(bound = ""), Debug(bound = ""))]
pub enum WindowStart<Types: NodeType> {
    #[from(ignore)]
    Time(u64),
    #[from(ignore)]
    Height(u64),
    Hash(BlockHash<Types>),
}

impl<Types: NodeType> Clone for WindowStart<Types> {
    fn clone(&self) -> Self {
        *self
    }
}

#[async_trait]
pub trait NodeDataSource<Types: NodeType> {
    async fn block_height(&self) -> QueryResult<usize>;
    async fn count_transactions_in_range(
        &self,
        range: impl RangeBounds<usize> + Send,
    ) -> QueryResult<usize>;
    async fn payload_size_in_range(
        &self,
        range: impl RangeBounds<usize> + Send,
    ) -> QueryResult<usize>;
    async fn vid_share<ID>(&self, id: ID) -> QueryResult<VidShare>
    where
        ID: Into<BlockId<Types>> + Send + Sync;
    async fn get_header_window(
        &self,
        start: impl Into<WindowStart<Types>> + Send + Sync,
        end: u64,
        limit: usize,
    ) -> QueryResult<TimeWindowQueryData<Header<Types>>>;

    /// Search the database for missing objects and generate a report.
    async fn sync_status(&self) -> QueryResult<SyncStatus>;

    async fn count_transactions(&self) -> QueryResult<usize> {
        self.count_transactions_in_range(0..).await
    }

    async fn payload_size(&self) -> QueryResult<usize> {
        self.payload_size_in_range(0..).await
    }
}
