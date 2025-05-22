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

#![cfg(feature = "sql-data-source")]

pub use anyhow::Error;
use hotshot_types::traits::node_implementation::NodeType;
pub use refinery::Migration;
pub use sql::Transaction;

use super::{
    fetching,
    storage::sql::{self, SqlStorage},
    AvailabilityProvider, FetchingDataSource,
};
pub use crate::include_migrations;
use crate::{
    availability::{QueryableHeader, QueryablePayload},
    Header, Payload,
};

pub type Builder<Types, Provider> = fetching::Builder<Types, SqlStorage, Provider>;

pub type Config = sql::Config;

impl Config {
    /// Connect to the database with this config.
    pub async fn connect<Types, P: AvailabilityProvider<Types>>(
        self,
        provider: P,
    ) -> Result<SqlDataSource<Types, P>, Error>
    where
        Types: NodeType,
        Header<Types>: QueryableHeader<Types>,
        Payload<Types>: QueryablePayload<Types>,
    {
        self.builder(provider).await?.build().await
    }

    /// Connect to the database, setting options on the underlying [`FetchingDataSource`] using the
    /// [`fetching::Builder`] interface.
    pub async fn builder<Types, P: AvailabilityProvider<Types>>(
        self,
        provider: P,
    ) -> Result<Builder<Types, P>, Error>
    where
        Types: NodeType,
        Header<Types>: QueryableHeader<Types>,
        Payload<Types>: QueryablePayload<Types>,
    {
        SqlDataSource::connect(self, provider).await
    }
}

/// A data source for the APIs provided in this crate, backed by a remote PostgreSQL database.
///
/// # Administration
///
/// This data source will automatically connect to and perform queries on a remote SQL database.
/// However, _administration_ of the database, such as initialization, resetting, and backups, is
/// left out of the scope of this implementation, and is expected to be performed manually using
/// off-the-shelf DBMS administration tools. The one exception is migrations, which are handled
/// transparently by the [`SqlDataSource`].
///
/// ## Schema
///
/// All the objects created and used by [`SqlDataSource`] are grouped under a schema for easy
/// management. By default, the schema is named `hotshot`, and is created the first time a
/// [`SqlDataSource`] is constructed. The name of the schema can be configured by setting
/// [`Config::schema`].
///
/// ## Initialization
///
/// When creating a PostgreSQL [`SqlDataSource`], the caller can use [`Config`] to specify the host, user, and
/// database for the connection. If the `embedded-db` feature is enabled, the caller can instead specify the
/// file path for an SQLite database.
/// As such, [`SqlDataSource`] is not very opinionated about how the
/// database instance is set up. The administrator must simply ensure that there is a database
/// dedicated to the [`SqlDataSource`] and a user with appropriate permissions (all on `SCHEMA` and
/// all on `DATABASE`) over that database.
///
/// Here is an example of how a sufficient database could be initialized. When using the standard
/// `postgres` Docker image, these statements could be placed in
/// `/docker-entrypoint-initdb.d/init.sql` to automatically initialize the database upon startup.
///
/// ```sql
/// CREATE DATABASE hotshot_query_service;
/// \connect hotshot_query_service;
/// CREATE USER hotshot_user WITH PASSWORD 'password';
/// GRANT ALL ON SCHEMA public TO hotshot_user;
/// GRANT ALL ON DATABASE hotshot_query_service TO hotshot_user WITH GRANT OPTION;
/// ```
///
/// For SQLite, simply provide the file path, and the file will be created if it does not already exist.
///
/// One could then connect to this database with the following [`Config`] for postgres:
///
/// ```
/// # use hotshot_query_service::data_source::sql::Config;
/// #[cfg(not(feature= "embedded-db"))]
/// Config::default()
///     .host("postgres.database.hostname")
///     .database("hotshot_query_service")
///     .user("hotshot_user")
///     .password("password")
/// # ;
/// ```
/// Or, if the `embedded-db` feature is enabled, configure it as follows for SQLite:
///
/// ```
/// # use hotshot_query_service::data_source::sql::Config;
/// #[cfg(feature= "embedded-db")]
/// Config::default()
///     .db_path("temp.db".into())
/// # ;
/// ```
/// ## Resetting
///
/// In general, resetting the database when necessary is left up to the administrator. However, for
/// convenience, we do provide a [`reset_schema`](Config::reset_schema) option which can be used to
/// wipe out existing state and create a fresh instance of the query service. This is particularly
/// useful for development and staging environments. This function will permanently delete all
/// tables associated with the schema used by this query service, but will not reset other schemas
/// or database.
///
/// ## Migrations
///
/// For the [`SqlDataSource`] to work, the database must be initialized with the appropriate schema,
/// and the schema must be kept up to date when deploying a new version of this software which
/// depends on a different schema. Both of these tasks are accomplished via _migrations_.
///
/// Each release of this software is bundled with a sequence of migration files: one migration for
/// each release that changed the schema, including the latest one. Replaying these SQL files
/// against a database with an older version of the schema, including a completely empty database,
/// will bring it up to date with the schema required by this version of the software. Upon creating
/// an instance of [`SqlDataSource`] and connecting to a database, the data source will
/// automatically fetch the current version from the database and, if it is old, replay the
/// necessary migration files.
///
/// ## Custom Migrations
///
/// In keeping with the philosophy of this crate, [`SqlDataSource`] is designed to be
/// [extensible and composable](#extension-and-composition). When extending the provided APIs with
/// new, application-specific queries, it will often be desirable to alter the schema of the
/// database in some way, such as adding additional columns to some of the tables or creating new
/// indices. When composing the provided APIs with additional API modules, it may also be desirable
/// to alter the schema, although the changes are more likely to be completely independent of the
/// schema used by this data source, such as adding entirely new tables.
///
/// In either case, the default schema can be modified by inserting additional migrations between
/// the migrations distributed with this crate. The new migrations will then automatically be
/// replayed as necessary when initializing a [`SqlDataSource`]. New custom migrations can be
/// added with each software update, to keep the custom data up to date as the default schema
/// changes.
///
/// Custom migrations can be inserted using [`Config::migrations`]. Each custom migration will be
/// inserted into the overall sequence of migrations in order of version number. The migrations
/// provided by this crate only use version numbers which are multiples of 100, so the non-multiples
/// can be used to insert custom migrations between the default migrations. You can also replace a
/// default migration completely by providing a custom migration with the same version number. This
/// may be useful when an earlier custom migration has altered the schema in such a way that a later
/// migration no longer works as-is. However, this technique is error prone and should be used only
/// when necessary.
///
/// When using custom migrations, it is the user's responsibility to ensure that the resulting
/// schema is compatible with the schema expected by [`SqlDataSource`]. Adding things (tables,
/// columns, indices) should usually be safe. Removing, altering, or renaming things should be done
/// with extreme caution.
///
/// It is standard to store custom migrations as SQL files in a sub-directory of the crate. For ease
/// of release and deployment, such directories can be embedded into a Rust binary and parsed into
/// a list of [`Migration`] objects using the [`include_migrations`] macro.
///
/// It is also possible to take complete control over migrating the schema using
/// [`Config::no_migrations`] to prevent the [`SqlDataSource`] from running its own migrations. The
/// database administrator then becomes responsible for manually migrating the database, ensuring the
/// schema is up to date, and ensuring that the schema is at all times compatible with the schema
/// expected by the current version of this software. Nevertheless, this may be the best option when
/// your application-specific schema has diverged significantly from the default schema.
///
/// # Synchronization
///
/// [`SqlDataSource`] implements [`VersionedDataSource`](super::VersionedDataSource), which means
/// changes are applied to the underlying database via transactions. [`Transaction`] maps exactly to
/// a transaction in the underling RDBMS, and inherits the underlying concurrency semantics.
///
/// # Extension and Composition
///
/// [`SqlDataSource`] is designed to be both extensible (so you can add additional state to the API
/// modules defined in this crate) and composable (so you can use [`SqlDataSource`] as one component
/// of a larger state type for an application with additional modules).
///
/// ## Extension
///
/// It is possible to add additional, application-specific state to [`SqlDataSource`]. If the new
/// state should live in memory, simply wrap the [`SqlDataSource`] in an
/// [`ExtensibleDataSource`](super::ExtensibleDataSource):
///
/// ```
/// # use hotshot_query_service::data_source::{
/// #   sql::{Config, Error}, ExtensibleDataSource, SqlDataSource,
/// # };
/// # use hotshot_query_service::fetching::provider::NoFetching;
/// # use hotshot_query_service::testing::mocks::MockTypes as AppTypes;
/// # async fn doc(config: Config) -> Result<(), Error> {
/// type AppState = &'static str;
///
/// let data_source: ExtensibleDataSource<SqlDataSource<AppTypes, NoFetching>, AppState> =
///     ExtensibleDataSource::new(config.connect(NoFetching).await?, "app state");
/// # Ok(())
/// # }
/// ```
///
/// The [`ExtensibleDataSource`](super::ExtensibleDataSource) wrapper implements all the same data
/// source traits as [`SqlDataSource`], and also provides access to the `AppState` parameter for use
/// in API endpoint handlers. This can be used to implement an app-specific data source trait and
/// add a new API endpoint that uses this app-specific data, as described in the
/// [extension guide](crate#extension).
///
/// If the new application-specific state should live in the SQL database itself, the implementation
/// is more involved, but still possible. Follow the steps for [custom
/// migrations](#custom-migrations) to modify the database schema to account for the new data you
/// want to store. You can then access this data through the [`SqlDataSource`] using
/// [`read`](super::VersionedDataSource::read) to run a custom read-only SQL query or
/// [`write`](super::VersionedDataSource::write) to execute a custom atomic mutation of the
/// database.
///
/// ## Composition
///
/// Composing [`SqlDataSource`] with other module states is fairly simple -- just
/// create an aggregate struct containing both [`SqlDataSource`] and your additional module
/// states, as described in the [composition guide](crate#composition). If the additional modules
/// have data that should live in the same database as the [`SqlDataSource`] data, you can follow
/// the steps in [custom migrations](#custom-migrations) to accommodate this.
///
/// ```
/// # use futures::StreamExt;
/// # use hotshot::types::SystemContextHandle;
/// # use hotshot_query_service::Error;
/// # use hotshot_query_service::data_source::{
/// #   sql::Config, Transaction, SqlDataSource, UpdateDataSource, VersionedDataSource,
/// # };
/// # use hotshot_query_service::fetching::provider::NoFetching;
/// # use hotshot_query_service::testing::mocks::{
/// #   MockNodeImpl as AppNodeImpl, MockTypes as AppTypes, MockVersions as AppVersions
/// # };
/// # use hotshot_example_types::node_types::TestVersions;
/// # use std::sync::Arc;
/// # use tide_disco::App;
/// # use tokio::spawn;
/// # use vbs::version::StaticVersionType;
/// struct AppState {
///     hotshot_qs: SqlDataSource<AppTypes, NoFetching>,
///     // additional state for other modules
/// }
///
/// async fn init_server<Ver: StaticVersionType + 'static>(
///     config: Config,
///     hotshot: SystemContextHandle<AppTypes, AppNodeImpl, AppVersions>,
/// ) -> anyhow::Result<App<Arc<AppState>, Error>> {
///     let mut hotshot_qs = config.connect(NoFetching).await?;
///     // Initialize storage for other modules, using `hotshot_qs` to access the database.
///     let tx = hotshot_qs.write().await?;
///     // ...
///     tx.commit().await?;
///
///     let state = Arc::new(AppState {
///         hotshot_qs,
///         // additional state for other modules
///     });
///     let mut app = App::with_state(state.clone());
///     // Register API modules.
///
///     spawn(async move {
///         let mut events = hotshot.event_stream();
///         while let Some(event) = events.next().await {
///             if state.hotshot_qs.update(&event).await.is_err() {
///                 continue;
///             }
///
///             let mut tx = state.hotshot_qs.write().await.unwrap();
///             // Update other modules' states based on `event`, using `tx` to access the database.
///             tx.commit().await.unwrap();
///         }
///     });
///
///     Ok(app)
/// }
/// ```
pub type SqlDataSource<Types, P> = FetchingDataSource<Types, SqlStorage, P>;

impl<Types, P: AvailabilityProvider<Types>> SqlDataSource<Types, P>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
{
    /// Connect to a remote database.
    ///
    /// This function returns a [`fetching::Builder`] which can be used to set options on the
    /// underlying [`FetchingDataSource`], before constructing the [`SqlDataSource`] with
    /// [`build`](fetching::Builder::build). For a convenient constructor that uses the default
    /// fetching options, see [`Config::connect`].
    pub async fn connect(config: Config, provider: P) -> Result<Builder<Types, P>, Error> {
        Ok(Self::builder(SqlStorage::connect(config).await?, provider))
    }
}

// These tests run the `postgres` Docker image, which doesn't work on Windows.
#[cfg(all(any(test, feature = "testing"), not(target_os = "windows")))]
pub mod testing {
    use async_trait::async_trait;
    use hotshot::types::Event;
    pub use sql::testing::TmpDb;

    use super::*;
    use crate::{
        data_source::UpdateDataSource,
        testing::{consensus::DataSourceLifeCycle, mocks::MockTypes},
    };

    #[async_trait]
    impl<P: AvailabilityProvider<MockTypes> + Default> DataSourceLifeCycle
        for SqlDataSource<MockTypes, P>
    {
        type Storage = TmpDb;

        async fn create(_node_id: usize) -> Self::Storage {
            TmpDb::init().await
        }

        async fn connect(tmp_db: &Self::Storage) -> Self {
            tmp_db.config().connect(Default::default()).await.unwrap()
        }

        async fn reset(tmp_db: &Self::Storage) -> Self {
            tmp_db
                .config()
                .reset_schema()
                .connect(Default::default())
                .await
                .unwrap()
        }

        async fn leaf_only_ds(tmp_db: &Self::Storage) -> Self {
            let config = tmp_db.config();
            let builder = config.builder(Default::default()).await.unwrap();

            builder
                .leaf_only()
                .build()
                .await
                .expect("failed to build leaf only sql ds")
        }

        async fn handle_event(&self, event: &Event<MockTypes>) {
            self.update(event).await.unwrap();
        }
    }
}

// These tests run the `postgres` Docker image, which doesn't work on Windows.
#[cfg(all(test, not(target_os = "windows")))]
mod generic_test {
    use super::SqlDataSource;
    // For some reason this is the only way to import the macro defined in another module of this
    // crate.
    use crate::*;
    use crate::{fetching::provider::NoFetching, testing::mocks::MockTypes};

    instantiate_data_source_tests!(SqlDataSource<MockTypes, NoFetching>);
}

#[cfg(all(test, not(target_os = "windows")))]
mod test {
    use hotshot_example_types::state_types::{TestInstanceState, TestValidatedState};
    use hotshot_types::{data::VidShare, vid::advz::advz_scheme};
    use jf_vid::VidScheme;

    use super::*;
    use crate::{
        availability::{
            AvailabilityDataSource, BlockInfo, LeafQueryData, UpdateAvailabilityData,
            VidCommonQueryData,
        },
        data_source::{
            storage::{NodeStorage, UpdateAvailabilityStorage},
            Transaction, VersionedDataSource,
        },
        fetching::provider::NoFetching,
        testing::{consensus::DataSourceLifeCycle, mocks::MockTypes, setup_test},
    };

    type D = SqlDataSource<MockTypes, NoFetching>;

    // This function should be generic, but the file system data source does not currently support
    // storing VID common and later the corresponding share.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_vid_monotonicity() {
        use hotshot_example_types::node_types::TestVersions;

        setup_test();

        let storage = D::create(0).await;
        let ds = <D as DataSourceLifeCycle>::connect(&storage).await;

        // Generate some test VID data.
        let disperse = advz_scheme(2).disperse([]).unwrap();

        // Insert test data with VID common but no share.
        let leaf = LeafQueryData::<MockTypes>::genesis::<TestVersions>(
            &TestValidatedState::default(),
            &TestInstanceState::default(),
        )
        .await;
        let common =
            VidCommonQueryData::new(leaf.header().clone(), crate::VidCommon::V0(disperse.common));
        ds.append(BlockInfo::new(leaf, None, Some(common.clone()), None, None))
            .await
            .unwrap();

        assert_eq!(ds.get_vid_common(0).await.await, common);
        NodeStorage::<MockTypes>::vid_share(&mut ds.read().await.unwrap(), 0)
            .await
            .unwrap_err();

        // Re-insert the common data with the share.
        let share0 = VidShare::V0(disperse.shares[0].clone());
        let mut tx = ds.write().await.unwrap();
        tx.insert_vid(common.clone(), Some(share0.clone()))
            .await
            .unwrap();
        tx.commit().await.unwrap();
        assert_eq!(ds.get_vid_common(0).await.await, common);
        assert_eq!(
            NodeStorage::<MockTypes>::vid_share(&mut ds.read().await.unwrap(), 0)
                .await
                .unwrap(),
            share0
        );
    }
}
