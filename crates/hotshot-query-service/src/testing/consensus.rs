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

use async_trait::async_trait;
use hotshot_types::new_protocol::CoordinatorEvent;

use super::mocks::MockTypes;
use crate::{
    availability::{AvailabilityDataSource, UpdateAvailabilityData},
    data_source::{FileSystemDataSource, VersionedDataSource, fetching::Builder, sql},
    fetching::provider::NoFetching,
    node::NodeDataSource,
    status::StatusDataSource,
};

pub type MockDataSource = FileSystemDataSource<MockTypes, NoFetching>;
pub type MockPostgresDataSource = sql::postgres::SqlDataSource<MockTypes, NoFetching>;
pub type MockSqliteDataSource = sql::sqlite::SqlDataSource<MockTypes, NoFetching>;

#[cfg(test)]
#[rstest_reuse::template]
#[rstest::rstest]
#[case::postgres(std::marker::PhantomData::<crate::testing::consensus::MockPostgresDataSource>)]
#[case::sqlite(std::marker::PhantomData::<crate::testing::consensus::MockSqliteDataSource>)]
#[test_log::test(tokio::test(flavor = "multi_thread"))]
pub fn sql_data_sources<D: TestableDataSource>(#[case] _d: std::marker::PhantomData<D>) {}

#[async_trait]
pub trait DataSourceLifeCycle: Clone + Send + Sync + Sized + 'static {
    /// Backing storage for the data source.
    ///
    /// This can be used to connect to data sources to the same underlying data. It must be kept
    /// alive as long as the related data sources are open.
    type Storage: Send + Sync;

    /// Type parameter for builder.
    type S;

    /// Type parameter for builder.
    type P;

    async fn create(node_id: usize) -> Self::Storage;
    async fn build(
        storage: &Self::Storage,
        opt: impl Send
        + FnOnce(Builder<MockTypes, Self::S, Self::P>) -> Builder<MockTypes, Self::S, Self::P>,
    ) -> Self;
    async fn reset(storage: &Self::Storage) -> Self;
    async fn handle_event(&self, event: &CoordinatorEvent<MockTypes>);

    async fn connect(storage: &Self::Storage) -> Self {
        Self::build(storage, |builder| builder).await
    }

    async fn leaf_only_ds(storage: &Self::Storage) -> Self {
        Self::build(storage, |builder| builder.leaf_only()).await
    }
}

pub trait TestableDataSource:
    DataSourceLifeCycle
    + AvailabilityDataSource<MockTypes>
    + UpdateAvailabilityData<MockTypes>
    + NodeDataSource<MockTypes>
    + StatusDataSource
    + VersionedDataSource
{
}

impl<T> TestableDataSource for T where
    T: DataSourceLifeCycle
        + AvailabilityDataSource<MockTypes>
        + UpdateAvailabilityData<MockTypes>
        + NodeDataSource<MockTypes>
        + StatusDataSource
        + VersionedDataSource
{
}

/// Write access a test needs that no storage trait offers.
#[cfg(test)]
#[async_trait]
pub trait SavePrunedHeight {
    /// Mark everything up to and including `height` as pruned.
    async fn save_pruned_height(&mut self, height: u64) -> anyhow::Result<()>;
}
