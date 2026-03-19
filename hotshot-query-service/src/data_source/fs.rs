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

#![cfg(feature = "file-system-data-source")]

use std::path::Path;

use atomic_store::AtomicStoreLoader;
use hotshot_types::traits::node_implementation::NodeType;

pub use super::storage::fs::Transaction;
use super::{AvailabilityProvider, FetchingDataSource, storage::FileSystemStorage};
use crate::{
    Header, Payload,
    availability::{QueryableHeader, query_data::QueryablePayload},
};

/// A data source for the APIs provided in this crate, backed by the local file system.
///
/// Synchronization and atomicity of persisted data structures are provided via [`atomic_store`].
/// The methods [`commit`](super::Transaction::commit), [`revert`](super::Transaction::revert), and
/// [`skip_version`](Self::skip_version) of this type and its associated [`Transaction`] type can be
/// used to control synchronization in the underlying [`AtomicStore`](atomic_store::AtomicStore).
///
/// Note that because [`AtomicStore`](atomic_store::AtomicStore) only allows changes to be made to
/// the underlying store, a [`Transaction`] takes full control of the whole store, and does not
/// permit concurrent readers or other transactions while in flight. This is enforced internally via
/// a global `RwLock`, and is a significant downside of this storage implementation, compared to the
/// more relaxed concurrency semantics of a SQL implementation.
///
/// # Extension and Composition
///
/// [`FileSystemDataSource`] is designed to be both extensible (so you can add additional state to
/// the API modules defined in this crate) and composable (so you can use [`FileSystemDataSource`]
/// as one component of a larger state type for an application with additional modules).
///
/// ## Extension
///
/// Adding additional, application-specific state to [`FileSystemDataSource`] is possible by
/// wrapping it in [`ExtensibleDataSource`](super::ExtensibleDataSource):
///
/// ```
/// # use hotshot_query_service::data_source::{ExtensibleDataSource, FileSystemDataSource};
/// # use hotshot_query_service::fetching::provider::NoFetching;
/// # use hotshot_query_service::testing::mocks::MockTypes as AppTypes;
/// # use std::path::Path;
/// # async fn doc(storage_path: &Path) -> Result<(), anyhow::Error> {
/// type AppState = &'static str;
///
/// let data_source: ExtensibleDataSource<FileSystemDataSource<AppTypes, NoFetching>, AppState> =
///     ExtensibleDataSource::new(FileSystemDataSource::create(storage_path, NoFetching).await?, "app state");
/// # Ok(())
/// # }
/// ```
///
/// The [`ExtensibleDataSource`](super::ExtensibleDataSource) wrapper implements all the same data
/// source traits as [`FileSystemDataSource`], and also provides access to the `AppState` parameter
/// for use in API endpoint handlers. This can be used to implement an app-specific data source
/// trait and add a new API endpoint that uses this app-specific data, as described in the
/// [extension guide](crate#extension).
///
/// ## Composition
///
/// Composing [`FileSystemDataSource`] with other module states is in principle simple -- just
/// create an aggregate struct containing both [`FileSystemDataSource`] and your additional module
/// states. A complication arises from how persistent storage is managed: if other modules have
/// their own persistent state, should the storage of [`FileSystemDataSource`] and the other modules
/// be completely independent, or synchronized under the control of a single
/// [`AtomicStore`](atomic_store::AtomicStore)? [`FileSystemDataSource`] supports both patterns:
/// when you create it with [`create`](Self::create) or [`open`](Self::open), it will open its own
/// [`AtomicStore`](atomic_store::AtomicStore) and manage the synchronization of its own storage,
/// independent of any other persistent data it might be composed with. But when you create it with
/// [`create_with_store`](Self::create_with_store) or [`open_with_store`](Self::open_with_store),
/// you may ask it to register its persistent data structures with an existing
/// [`AtomicStoreLoader`]. If you register other modules' persistent data structures with the same
/// loader, you can create one [`AtomicStore`](atomic_store::AtomicStore) that synchronizes all the
/// persistent data. Note, though, that when you choose to use
/// [`create_with_store`](Self::create_with_store) or [`open_with_store`](Self::open_with_store),
/// you become responsible for ensuring that calls to
/// [`AtomicStore::commit_version`](atomic_store::AtomicStore::commit_version) alternate with calls
/// to [`commit`](super::Transaction::commit) or [`skip_version`](Self::skip_version).
///
/// In the following example, we compose HotShot query service modules with other application-
/// specific modules, using a single top-level [`AtomicStore`](atomic_store::AtomicStore) to
/// synchronize all persistent storage.
///
/// ```
/// # use atomic_store::{AtomicStore, AtomicStoreLoader};
/// # use futures::StreamExt;
/// # use hotshot::types::SystemContextHandle;
/// # use hotshot_query_service::Error;
/// # use hotshot_query_service::data_source::{
/// #   FileSystemDataSource, Transaction, UpdateDataSource, VersionedDataSource,
/// # };
/// # use hotshot_query_service::fetching::provider::NoFetching;
/// # use hotshot_query_service::testing::mocks::{
/// #   MockNodeImpl as AppNodeImpl, MockTypes as AppTypes, MockVersions as AppVersions
/// # };
/// # use hotshot_example_types::node_types::TestVersions;
/// # use std::{path::Path, sync::Arc};
/// # use tide_disco::App;
/// # use tokio::{spawn, sync::RwLock};
/// # use vbs::version::StaticVersionType;
/// struct AppState {
///     // Top-level storage coordinator
///     store: AtomicStore,
///     hotshot_qs: FileSystemDataSource<AppTypes, NoFetching>,
///     // additional state for other modules
/// }
///
/// async fn init_server<Ver: StaticVersionType + 'static>(
///     storage_path: &Path,
///     hotshot: SystemContextHandle<AppTypes, AppNodeImpl, AppVersions>,
/// ) -> anyhow::Result<App<Arc<RwLock<AppState>>, Error>> {
///     let mut loader = AtomicStoreLoader::create(storage_path, "my_app")?; // or `open`
///     let hotshot_qs = FileSystemDataSource::create_with_store(&mut loader, NoFetching)
///         .await?;
///     // Initialize storage for other modules using the same loader.
///
///     let store = AtomicStore::open(loader)?;
///     let state = Arc::new(RwLock::new(AppState {
///         store,
///         hotshot_qs,
///         // additional state for other modules
///     }));
///     let mut app = App::with_state(state.clone());
///     // Register API modules.
///
///     spawn(async move {
///         let mut events = hotshot.event_stream();
///         while let Some(event) = events.next().await {
///             let mut state = state.write().await;
///             if state.hotshot_qs.update(&event).await.is_err() {
///                 continue;
///             }
///
///             // Update other modules' states based on `event`.
///             let mut tx = state.hotshot_qs.write().await.unwrap();
///             // Do updates
///             tx.commit().await.unwrap();
///
///             // Commit or skip versions for other modules' storage.
///             state.store.commit_version().unwrap();
///         }
///     });
///
///     Ok(app)
/// }
/// ```
pub type FileSystemDataSource<Types, P> = FetchingDataSource<Types, FileSystemStorage<Types>, P>;

impl<Types: NodeType, P> FileSystemDataSource<Types, P>
where
    Payload<Types>: QueryablePayload<Types>,
    Header<Types>: QueryableHeader<Types>,
    P: AvailabilityProvider<Types>,
{
    /// Create a new [FileSystemDataSource] with storage at `path`.
    ///
    /// If there is already data at `path`, it will be archived.
    ///
    /// The [FileSystemDataSource] will manage its own persistence synchronization.
    pub async fn create(path: &Path, provider: P) -> anyhow::Result<Self> {
        FileSystemDataSource::builder(FileSystemStorage::create(path).await?, provider)
            .build()
            .await
    }

    /// Open an existing [FileSystemDataSource] from storage at `path`.
    ///
    /// If there is no data at `path`, a new store will be created.
    ///
    /// The [FileSystemDataSource] will manage its own persistence synchronization.
    pub async fn open(path: &Path, provider: P) -> anyhow::Result<Self> {
        FileSystemDataSource::builder(FileSystemStorage::open(path).await?, provider)
            .build()
            .await
    }

    /// Create a new [FileSystemDataSource] using a persistent storage loader.
    ///
    /// If there is existing data corresponding to the [FileSystemDataSource] data structures, it
    /// will be archived.
    ///
    /// The [FileSystemDataSource] will register its persistent data structures with `loader`. The
    /// caller is responsible for creating an [AtomicStore](atomic_store::AtomicStore) from `loader`
    /// and managing synchronization of the store.
    pub async fn create_with_store(
        loader: &mut AtomicStoreLoader,
        provider: P,
    ) -> anyhow::Result<Self> {
        FileSystemDataSource::builder(
            FileSystemStorage::create_with_store(loader).await?,
            provider,
        )
        .build()
        .await
    }

    /// Open an existing [FileSystemDataSource] using a persistent storage loader.
    ///
    /// If there is no existing data corresponding to the [FileSystemDataSource] data structures, a
    /// new store will be created.
    ///
    /// The [FileSystemDataSource] will register its persistent data structures with `loader`. The
    /// caller is responsible for creating an [AtomicStore](atomic_store::AtomicStore) from `loader`
    /// and managing synchronization of the store.
    pub async fn open_with_store(
        loader: &mut AtomicStoreLoader,
        provider: P,
    ) -> anyhow::Result<Self> {
        FileSystemDataSource::builder(FileSystemStorage::open_with_store(loader).await?, provider)
            .build()
            .await
    }

    /// Advance the version of the persistent store without committing changes to persistent state.
    ///
    /// This function is useful when the [AtomicStore](atomic_store::AtomicStore) synchronizing
    /// storage for this [FileSystemDataSource] is being managed by the caller. The caller may want
    /// to persist some changes to other modules whose state is managed by the same
    /// [AtomicStore](atomic_store::AtomicStore). In order to call
    /// [AtomicStore::commit_version](atomic_store::AtomicStore::commit_version), the version of
    /// this [FileSystemDataSource] must be advanced, either by [commit](super::Transaction::commit)
    /// or, if there are no outstanding changes, [skip_version](Self::skip_version).
    pub async fn skip_version(&self) -> anyhow::Result<()> {
        self.as_ref().skip_version().await?;
        Ok(())
    }
}

#[cfg(any(test, feature = "testing"))]
mod impl_testable_data_source {
    use async_trait::async_trait;
    use hotshot::types::Event;
    use tempfile::TempDir;

    use super::*;
    use crate::{
        data_source::UpdateDataSource,
        testing::{consensus::DataSourceLifeCycle, mocks::MockTypes},
    };

    #[async_trait]
    impl<P: AvailabilityProvider<MockTypes> + Default> DataSourceLifeCycle
        for FileSystemDataSource<MockTypes, P>
    {
        type Storage = TempDir;

        async fn create(node_id: usize) -> Self::Storage {
            TempDir::with_prefix(format!("file_system_data_source_{node_id}")).unwrap()
        }

        async fn connect(storage: &Self::Storage) -> Self {
            Self::open(storage.path(), Default::default())
                .await
                .unwrap()
        }

        async fn reset(storage: &Self::Storage) -> Self {
            Self::create(storage.path(), Default::default())
                .await
                .unwrap()
        }

        async fn handle_event(&self, event: &Event<MockTypes>) {
            self.update(event).await.unwrap();
        }
    }
}

#[cfg(test)]
mod test {
    use super::FileSystemDataSource;
    // For some reason this is the only way to import the macro defined in another module of this
    // crate.
    use crate::*;
    use crate::{fetching::provider::NoFetching, testing::mocks::MockTypes};

    instantiate_data_source_tests!(FileSystemDataSource<MockTypes, NoFetching>);
}
