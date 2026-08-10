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

//! The HotShot Query Service is a minimal, generic query service that can be integrated into any
//! decentralized application running on the [hotshot] consensus layer. It provides all the features
//! that HotShot itself expects of a query service (such as providing consensus-related data for
//! catchup and synchronization) as well as some application-level features that deal only with
//! consensus-related or application-agnostic data. In addition, the query service is provided as an
//! extensible library, which makes it easy to add additional, application-specific features.
//!
//! # Basic usage
//!
//! ```
//! # use hotshot::types::SystemContextHandle;
//! # use hotshot_query_service::testing::mocks::{
//! #   MockNodeImpl as AppNodeImpl, MockTypes as AppTypes, MockVersions as AppVersions,
//! # };
//! # use hotshot_example_types::node_types::TestVersions;
//! # use hotshot_types::consensus::ConsensusMetricsValue;
//! # use std::path::Path;
//! # async fn doc(storage_path: &std::path::Path) -> anyhow::Result<()> {
//! use hotshot_query_service::{
//!     data_source::{FileSystemDataSource, UpdateDataSource},
//!     fetching::provider::NoFetching,
//!     status::UpdateStatusData,
//! };
//!
//! use futures::StreamExt;
//! use hotshot::SystemContext;
//! use hotshot_types::new_protocol::CoordinatorEvent;
//!
//! // Create or open a data source.
//! let data_source = FileSystemDataSource::<AppTypes, NoFetching>::create(storage_path, NoFetching)
//!     .await?;
//!
//! // Create hotshot, giving it a handle to the status metrics.
//! let hotshot = SystemContext::<AppTypes, AppNodeImpl, AppVersions>::init(
//! #   panic!(), panic!(), panic!(), panic!(), panic!(), panic!(), panic!(),
//!     ConsensusMetricsValue::new(&*data_source.populate_metrics()), panic!(),
//!     panic!()
//!     // Other fields omitted
//! ).await?.0;
//!
//! // Update query data using HotShot events.
//! let mut events = hotshot.event_stream();
//! while let Some(event) = events.next().await {
//!     // Update the query data based on this event.
//!     let event = CoordinatorEvent::LegacyEvent(event);
//!     data_source.update(&event).await.ok();
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Persistence
//!
//! Naturally, an archival query service such as this is heavily dependent on a persistent storage
//! implementation. The APIs provided by this query service are generic over the specific type of
//! the persistence layer, which we call a _data source_. This crate provides several data source
//! implementations in the [`data_source`] module.
//!
//! # Interaction with other components
//!
//! The HotShot Query Service is designed to be used as a single component of a larger service
//! consisting of several other interacting components. This interaction has two dimensions:
//! * _extension_, adding new functionality to the data source modules provided by this crate
//! * _composition_, combining the data source modules from this crate with other,
//!   application-specific state to back a single API
//!
//! ## Extension
//!
//! It is possible to add new functionality directly to the modules provided by this create. This
//! allows you to keep semantically related functionality grouped together in a single API module,
//! for interface purposes, even while some of the functionality of that module is provided by this
//! crate and some of it is an application-specific extension.
//!
//! For example, consider an application which is a UTXO-based blockchain. Each transaction consists
//! of a handful of new _output records_, and you want your query service to provide an API for
//! looking up a specific output by its index. Semantically, this functionality belongs in the
//! _data availability_ API, however it is application-specific -- HotShot itself makes no
//! assumptions and provides no guarantees about the internal structure of a transaction. In order
//! to expose this UTXO-specific functionality as well as the generic data availability
//! functionality provided by this crate as part of the same public API, you can extend the
//! [availability] module of this crate with additional data structures and endpoints that know
//! about the internal structure of your transactions.
//!
//! There are two parts to adding additional functionality to a module in this crate: adding the
//! required additional data structures to the data source, and creating a new API endpoint to
//! expose the functionality. The mechanism for the former will depend on the specific data source
//! you are using. Check the documentation for your data source implementation to see how it can be
//! extended. The latter is the responsibility of the application's own API layer, which serves
//! endpoints backed by the data source.
//!
//! It is good practice to define a trait for accessing this custom state, so that if you want to
//! switch data sources in the future, you can easily extend the new data source, implement the
//! trait, and then transparently replace the data source that you use to set up your API. In the
//! case of adding a UTXO index, this trait might look like this:
//!
//! ```
//! # use hotshot_query_service::{
//! #   availability::{AvailabilityDataSource, TransactionIndex},
//! #   testing::mocks::MockTypes as AppTypes,
//! # };
//! use async_trait::async_trait;
//!
//! #[async_trait]
//! trait UtxoDataSource: AvailabilityDataSource<AppTypes> {
//!     // Index mapping UTXO index to (block index, transaction index, output index)
//!     async fn find_utxo(&self, utxo: u64) -> Option<(usize, TransactionIndex<AppTypes>, usize)>;
//! }
//! ```
//!
//! Implement this trait for the extended data source you're using, and then serve the new
//! endpoint from your application's API layer by calling the trait method from the handler.
//!
//! ## Composition
//!
//! Composing the modules provided by this crate with other, unrelated modules to create a unified
//! service is fairly simple: an application-level state type can aggregate the data sources
//! provided by this crate with state for other modules, and a single API can be served from that
//! aggregate state.
//!
//! The data source traits defined by this crate ([availability::AvailabilityDataSource],
//! [node::NodeDataSource], and [status::StatusDataSource]) make this possible: they can be
//! implemented for any state type. The data sources provided by this crate implement these traits,
//! but if you want to use a custom state type that includes state for other modules, you will need
//! to implement these traits for your custom type. The basic pattern looks like this:
//!
//! ```
//! # use async_trait::async_trait;
//! # use hotshot_query_service::{Header, QueryResult, VidShare};
//! # use hotshot_query_service::availability::{
//! #   AvailabilityDataSource, BlockId, BlockQueryData, Fetch, FetchStream, LeafId, LeafQueryData,
//! #   PayloadMetadata, PayloadQueryData, TransactionFromBlock, TransactionHash,
//! #   VidCommonMetadata, VidCommonQueryData,
//! # };
//! # use hotshot_query_service::metrics::PrometheusMetrics;
//! # use hotshot_query_service::node::{
//! #   NodeDataSource, SyncStatus, TimeWindowQueryData, WindowStart,
//! # };
//! # use hotshot_query_service::status::{HasMetrics, StatusDataSource};
//! # use hotshot_query_service::testing::mocks::MockTypes as AppTypes;
//! # use std::ops::{Bound, RangeBounds};
//! # type AppQueryData = ();
//! // Our AppState takes an underlying data source `D` which already implements the relevant
//! // traits, and adds some state for use with other modules.
//! struct AppState<D> {
//!     hotshot_qs: D,
//!     // additional state for other modules
//! }
//!
//! // Implement data source trait for availability API by delegating to the underlying data source.
//! #[async_trait]
//! impl<D: AvailabilityDataSource<AppTypes> + Send + Sync>
//!     AvailabilityDataSource<AppTypes> for AppState<D>
//! {
//!     async fn get_leaf<ID>(&self, id: ID) -> Fetch<LeafQueryData<AppTypes>>
//!     where
//!         ID: Into<LeafId<AppTypes>> + Send + Sync,
//!     {
//!         self.hotshot_qs.get_leaf(id).await
//!     }
//!
//!     // etc
//! #   async fn get_block<ID>(&self, id: ID) -> Fetch<BlockQueryData<AppTypes>>
//! #   where
//! #       ID: Into<BlockId<AppTypes>> + Send + Sync { todo!() }
//! #   async fn get_payload<ID>(&self, id: ID) -> Fetch<PayloadQueryData<AppTypes>>
//! #   where
//! #       ID: Into<BlockId<AppTypes>> + Send + Sync { todo!() }
//! #   async fn get_payload_metadata<ID>(&self, id: ID) -> Fetch<PayloadMetadata<AppTypes>>
//! #   where
//! #       ID: Into<BlockId<AppTypes>> + Send + Sync { todo!() }
//! #   async fn get_vid_common<ID>(&self, id: ID) -> Fetch<VidCommonQueryData<AppTypes>>
//! #   where
//! #       ID: Into<BlockId<AppTypes>> + Send + Sync { todo!() }
//! #   async fn get_vid_common_metadata<ID>(&self, id: ID) -> Fetch<VidCommonMetadata<AppTypes>>
//! #   where
//! #       ID: Into<BlockId<AppTypes>> + Send + Sync { todo!() }
//! #   async fn get_transaction<T: TransactionFromBlock<AppTypes>>(&self, hash: TransactionHash<AppTypes>) -> Fetch<T> { todo!() }
//! #   async fn get_leaf_range<R>(&self, range: R) -> FetchStream<LeafQueryData<AppTypes>>
//! #   where
//! #       R: RangeBounds<usize> + Send { todo!() }
//! #   async fn get_block_range<R>(&self, range: R) -> FetchStream<BlockQueryData<AppTypes>>
//! #   where
//! #       R: RangeBounds<usize> + Send { todo!() }
//! #   async fn get_payload_range<R>(&self, range: R) -> FetchStream<PayloadQueryData<AppTypes>>
//! #   where
//! #       R: RangeBounds<usize> + Send { todo!() }
//! #   async fn get_payload_metadata_range<R>(&self, range: R) -> FetchStream<PayloadMetadata<AppTypes>>
//! #   where
//! #       R: RangeBounds<usize> + Send { todo!() }
//! #   async fn get_vid_common_range<R>(&self, range: R) -> FetchStream<VidCommonQueryData<AppTypes>>
//! #   where
//! #       R: RangeBounds<usize> + Send { todo!() }
//! #   async fn get_vid_common_metadata_range<R>(&self, range: R) -> FetchStream<VidCommonMetadata<AppTypes>>
//! #   where
//! #       R: RangeBounds<usize> + Send { todo!() }
//! #   async fn get_leaf_range_rev(&self, start: Bound<usize>, end: usize) -> FetchStream<LeafQueryData<AppTypes>> { todo!() }
//! #   async fn get_block_range_rev(&self, start: Bound<usize>, end: usize) -> FetchStream<BlockQueryData<AppTypes>> { todo!() }
//! #   async fn get_payload_range_rev(&self, start: Bound<usize>, end: usize) -> FetchStream<PayloadQueryData<AppTypes>> { todo!() }
//! #   async fn get_payload_metadata_range_rev(&self, start: Bound<usize>, end: usize) -> FetchStream<PayloadMetadata<AppTypes>> { todo!() }
//! #   async fn get_vid_common_range_rev(&self, start: Bound<usize>, end: usize) -> FetchStream<VidCommonQueryData<AppTypes>> { todo!() }
//! #   async fn get_vid_common_metadata_range_rev(&self, start: Bound<usize>, end: usize) -> FetchStream<VidCommonMetadata<AppTypes>> { todo!() }
//! }
//!
//! // Implement data source trait for node API by delegating to the underlying data source.
//! #[async_trait]
//! impl<D: NodeDataSource<AppTypes> + Send + Sync> NodeDataSource<AppTypes> for AppState<D> {
//!     async fn block_height(&self) -> QueryResult<usize> {
//!         self.hotshot_qs.block_height().await
//!     }
//!
//!     async fn count_transactions_in_range(
//!         &self,
//!         range: impl RangeBounds<usize> + Send,
//!     ) -> QueryResult<usize> {
//!         self.hotshot_qs.count_transactions_in_range(range).await
//!     }
//!
//!     async fn payload_size_in_range(
//!         &self,
//!         range: impl RangeBounds<usize> + Send,
//!     ) -> QueryResult<usize> {
//!         self.hotshot_qs.payload_size_in_range(range).await
//!     }
//!
//!     async fn vid_share<ID>(&self, id: ID) -> QueryResult<VidShare>
//!     where
//!         ID: Into<BlockId<AppTypes>> + Send + Sync,
//!     {
//!         self.hotshot_qs.vid_share(id).await
//!     }
//!
//!     async fn sync_status(&self) -> QueryResult<SyncStatus> {
//!         self.hotshot_qs.sync_status().await
//!     }
//!
//!     async fn get_header_window(
//!         &self,
//!         start: impl Into<WindowStart<AppTypes>> + Send + Sync,
//!         end: u64,
//!         limit: usize,
//!     ) -> QueryResult<TimeWindowQueryData<Header<AppTypes>>> {
//!         self.hotshot_qs.get_header_window(start, end, limit).await
//!     }
//! }
//!
//! // Implement data source trait for status API by delegating to the underlying data source.
//! impl<D: HasMetrics> HasMetrics for AppState<D> {
//!     fn metrics(&self) -> &PrometheusMetrics {
//!         self.hotshot_qs.metrics()
//!     }
//! }
//! #[async_trait]
//! impl<D: StatusDataSource + Send + Sync> StatusDataSource for AppState<D> {
//!     async fn block_height(&self) -> QueryResult<usize> {
//!         self.hotshot_qs.block_height().await
//!     }
//! }
//!
//! // Implement data source traits for other modules, using additional state from AppState.
//! ```
//!
//! In the future, we may provide derive macros for
//! [AvailabilityDataSource](availability::AvailabilityDataSource),
//! [NodeDataSource](node::NodeDataSource), and [StatusDataSource](status::StatusDataSource) to
//! eliminate the boilerplate of implementing them for a custom type that has an existing
//! implementation as one of its fields.
//!
//! Once you have created your `AppState` type aggregating the state for each API module, you can
//! initialize the state as normal, instantiating `D` with a concrete implementation of a data
//! source and initializing `hotshot_qs` as you normally would that data source.
//!
//! _However_, this only works if you want the persistent storage for the availability and node
//! modules (managed by `hotshot_qs`) to be independent of the persistent storage for other modules.
//! You may well want to synchronize the storage for all modules together, so that updates to the
//! entire application state can be done atomically. This is particularly relevant if one of your
//! application-specific modules updates its storage based on a stream of HotShot leaves. Since the
//! availability and node modules also update with each new leaf, you probably want all of these
//! modules to stay in sync. The data source implementations provided by this crate provide means by
//! which you can add additional data to the same persistent store and synchronize the entire store
//! together. Refer to the documentation for you specific data source for information on how to
//! achieve this.
//!

pub mod availability;
pub mod data_source;
mod error;
pub mod explorer;
pub mod fetching;
pub mod merklized_state;
pub mod metrics;
pub mod migration;
pub mod node;
mod resolvable;
#[cfg(feature = "sqlite-options")]
pub mod sqlite_options;
pub mod status;
pub mod task;
pub mod testing;
pub mod types;

pub use error::Error;
pub use hotshot_query_service_types::{
    ErrorSnafu, Header, Leaf2, Metadata, MissingSnafu, NotFoundSnafu, Payload, QueryError,
    QueryResult, QuorumCertificate, SignatureKey, Transaction,
};
pub use resolvable::Resolvable;
