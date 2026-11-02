//! Persistence on a Postgres server.

use hotshot_query_service::{
    data_source::storage::sql::postgres::{self as qs, set_no_deferrable_on_read},
    include_migrations,
};
use light_client::storage::{LightClientDbOptions, LightClientPostgresOptions};

#[path = "../sql.rs"]
mod imp;

pub use imp::*;
use qs::{Config, SqlStorage};

use super::Options as SqlOptions;
#[cfg(any(test, feature = "testing"))]
use super::PostgresOptions;
use crate::{api::options::Query, persistence::storage_probe::StorageProbe};

impl BackendOptions for qs::Db {
    fn apply(mut cfg: Config, opt: &SqlOptions) -> anyhow::Result<Config> {
        cfg = cfg.query_max_connections(opt.query_max_connections.unwrap_or(opt.max_connections));
        cfg = cfg.query_min_connections(opt.query_min_connections.unwrap_or(opt.min_connections));
        set_no_deferrable_on_read(opt.postgres_options.no_deferrable);

        cfg = cfg.migrations(include_migrations!(
            "$CARGO_MANIFEST_DIR/api/migrations/postgres"
        ));

        let pg_options = &opt.postgres_options;

        if let Some(host) = &pg_options.host {
            cfg = cfg.host(host.clone());
        }

        if let Some(port) = pg_options.port {
            cfg = cfg.port(port);
        }

        if let Some(database) = &pg_options.database {
            cfg = cfg.database(database);
        }

        if let Some(user) = &pg_options.user {
            cfg = cfg.user(user);
        }

        if let Some(password) = &pg_options.password {
            cfg = cfg.password(password);
        }

        if pg_options.use_tls {
            cfg = cfg.tls();
        }

        Ok(cfg)
    }

    /// A Postgres node keeps its light client in the same database, so it survives restarts and
    /// container replacement without any extra volume.
    fn light_client_db(cfg: &Config, query: &Query) -> LightClientDbOptions {
        LightClientDbOptions::Postgres(Box::new(LightClientPostgresOptions {
            connect: cfg.connect_options().clone(),
            num_connections: query.light_client_db.num_connections,
            num_leaves: query.light_client_db.num_leaves,
        }))
    }

    async fn reset_light_client(cfg: &Config) -> anyhow::Result<()> {
        LightClientPostgresOptions::reset(cfg.connect_options()).await
    }

    async fn storage_probe(
        _db: &SqlStorage,
        _opt: &SqlOptions,
    ) -> anyhow::Result<Option<StorageProbe>> {
        Ok(None)
    }
}

#[cfg(any(test, feature = "testing"))]
pub(crate) fn tmp_options(db: &qs::testing::TmpDb) -> SqlOptions {
    PostgresOptions {
        port: Some(db.port()),
        host: Some(db.host()),
        user: Some("postgres".into()),
        password: Some("password".into()),
        ..Default::default()
    }
    .into()
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use committable::Committable as _;
    use espresso_types::{
        FeeAccount, Header, Leaf, Leaf2, NodeState, Payload, Transaction as Tx,
        traits::{PersistenceOptions as _, SequencerPersistence as _},
    };
    use hotshot_example_types::node_types::TEST_VERSIONS;
    use hotshot_query_service::{
        availability::{BlockQueryData, LeafQueryData},
        data_source::{
            Transaction as _, VersionedDataSource as _, storage::UpdateAvailabilityStorage,
        },
    };
    use hotshot_types::{
        data::{EpochNumber, QuorumProposal, ViewNumber, vid_commitment},
        event::HotShotAction,
        simple_certificate::QuorumCertificate,
        traits::{
            EncodeBytes,
            block_contents::{
                BlockHeader, BlockPayload as _, BuilderFee, GENESIS_VID_NUM_STORAGE_NODES,
            },
            election::Membership,
            signature_key::BuilderSignatureKey,
        },
    };
    use sqlx::{Row as _, query};

    use super::*;
    use crate::{SeqTypes, persistence::tests::TestablePersistence as _};

    #[test]
    fn sqlite_options_do_not_build_postgres_persistence() {
        let opt = SqlOptions {
            embedded_db: true,
            ..SqlOptions::default()
        };
        assert!(Options::try_from(opt.clone()).is_err());
        assert!(matches!(
            opt.into_backend(),
            super::super::SqlBackendOptions::Sqlite(_)
        ));
    }

    #[tokio::test]
    async fn a_sqlite_pool_is_not_reused_for_postgres() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect(":memory:")
            .await
            .unwrap();
        let opt = SqlOptions {
            pool: Some(pool.into()),
            ..SqlOptions::default()
        };
        let Err(err) = Config::try_from(&opt) else {
            panic!("a SQLite pool was accepted for Postgres");
        };
        assert!(format!("{err:#}").contains("other SQL backend"), "{err:#}");
    }

    async fn test_postgres_read_ns_table(instance_state: NodeState) {
        instance_state
            .coordinator
            .membership()
            .set_first_epoch(EpochNumber::genesis(), Default::default());

        let tmp = Persistence::tmp_storage().await;
        let mut opt = Persistence::options(&tmp);
        let storage = opt.create().await.unwrap();

        let txs = [
            Tx::new(10001u32.into(), vec![1, 2, 3]),
            Tx::new(10001u32.into(), vec![4, 5, 6]),
            Tx::new(10009u32.into(), vec![7, 8, 9]),
        ];

        let validated_state = Default::default();
        let justify_qc =
            QuorumCertificate::genesis(&validated_state, &instance_state, TEST_VERSIONS.test).await;
        let view_number: ViewNumber = justify_qc.view_number + 1;
        let parent_leaf = Leaf::genesis(&validated_state, &instance_state, TEST_VERSIONS.test.base)
            .await
            .into();

        let (payload, ns_table) =
            Payload::from_transactions(&txs, &validated_state, &instance_state)
                .await
                .unwrap();
        let payload_bytes = payload.encode();
        let payload_commitment = vid_commitment(
            &payload_bytes,
            &ns_table.encode(),
            GENESIS_VID_NUM_STORAGE_NODES,
            instance_state.current_version,
        );
        let builder_commitment = payload.builder_commitment(&ns_table);
        let (fee_account, fee_key) = FeeAccount::generated_from_seed_indexed([0; 32], 0);
        let fee_amount = 0;
        let fee_signature = FeeAccount::sign_fee(&fee_key, fee_amount, &ns_table).unwrap();
        let block_header = Header::new(
            &validated_state,
            &instance_state,
            &parent_leaf,
            payload_commitment,
            builder_commitment,
            ns_table,
            BuilderFee {
                fee_amount,
                fee_account,
                fee_signature,
            },
            instance_state.current_version,
            view_number.u64(),
        )
        .await
        .unwrap();
        let proposal = QuorumProposal {
            block_header: block_header.clone(),
            view_number,
            justify_qc: justify_qc.clone(),
            upgrade_certificate: None,
            proposal_certificate: None,
        };
        let leaf: Leaf2 = Leaf::from_quorum_proposal(&proposal).into();
        let mut qc = justify_qc.to_qc2();
        qc.data.leaf_commit = leaf.commit();
        qc.view_number = view_number;

        let mut tx = storage.db.write().await.unwrap();
        tx.insert_leaf(&LeafQueryData::new(leaf, qc).unwrap())
            .await
            .unwrap();
        tx.insert_block(&BlockQueryData::<SeqTypes>::new(block_header, payload))
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let mut tx = storage.db.read().await.unwrap();
        let rows = query(
            "
            SELECT ns_id, read_ns_id(get_ns_table(h.data), t.ns_index) AS read_ns_id
              FROM header AS h
              JOIN transactions AS t ON t.block_height = h.height
              ORDER BY t.ns_index, t.position
        ",
        )
        .fetch_all(tx.as_mut())
        .await
        .unwrap();
        assert_eq!(rows.len(), txs.len());
        for (i, row) in rows.into_iter().enumerate() {
            let ns = u64::from(txs[i].namespace()) as i64;
            assert_eq!(row.get::<i64, _>("ns_id"), ns);
            assert_eq!(row.get::<i64, _>("read_ns_id"), ns);
        }
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_postgres_read_ns_table_v0_1() {
        let v0_1 = versions::version(0, 1);
        test_postgres_read_ns_table(
            NodeState::mock()
                .with_current_version(v0_1)
                .with_genesis_version(v0_1),
        )
        .await;
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_postgres_read_ns_table_v0_2() {
        test_postgres_read_ns_table(NodeState::mock_v2()).await;
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_postgres_read_ns_table_v0_3() {
        test_postgres_read_ns_table(NodeState::mock_v3().with_epoch_height(0)).await;
    }

    /// Verify that concurrent calls to `record_action` all succeed under
    /// PostgreSQL SERIALIZABLE isolation. `self.serializable_backoff.retry_if` handles any
    /// 40001 serialization failures that arise when many tasks race to update
    /// the same row.
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_record_action_concurrent() {
        let tmp = Persistence::tmp_storage().await;
        let storage = Arc::new(Persistence::connect(&tmp).await);

        let handles: Vec<_> = (0u64..20)
            .map(|i| {
                let storage = Arc::clone(&storage);
                tokio::spawn(async move {
                    storage
                        .record_action(ViewNumber::new(i), HotShotAction::Vote)
                        .await
                })
            })
            .collect();

        for handle in handles {
            handle.await.unwrap().unwrap();
        }

        let latest = storage.load_latest_acted_view().await.unwrap();
        assert_eq!(latest, Some(ViewNumber::new(19)));
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_light_client_shares_the_node_database() {
        use hotshot_query_service::availability::LeafId;
        use light_client::storage::Storage as _;

        use crate::api::{data_source::DataSourceOptions as _, options::Query};

        let tmp = Persistence::tmp_storage().await;
        let mut opt = Persistence::options(&tmp);
        let persistence = opt.create().await.unwrap();

        let db = opt
            .light_client_db(&Query::default())
            .unwrap()
            .connect()
            .await
            .unwrap();
        let leaf = light_client::testing::leaf_chain(0..1, versions::EPOCH_VERSION)
            .await
            .remove(0);
        db.insert_leaf(leaf.clone()).await.unwrap();
        assert_eq!(
            db.leaf_upper_bound(LeafId::Number(0)).await.unwrap(),
            Some(leaf)
        );

        let (leaves,): (i64,) = sqlx::query_as("SELECT count(*) FROM light_client.leaf")
            .fetch_one(&persistence.db.pool())
            .await
            .unwrap();
        assert_eq!(leaves, 1);
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_reset_drops_light_client_cache() {
        use light_client::storage::Storage as _;

        use crate::api::{data_source::DataSourceOptions as _, options::Query};

        let tmp = Persistence::tmp_storage().await;
        let opt = Persistence::options(&tmp);
        let light_client = || opt.light_client_db(&Query::default()).unwrap().connect();
        let leaf = light_client::testing::leaf_chain(0..1, versions::EPOCH_VERSION)
            .await
            .remove(0);
        let db = light_client().await.unwrap();
        db.insert_leaf(leaf).await.unwrap();
        drop(db);

        opt.clone().reset().await.unwrap();

        let db = light_client().await.unwrap();
        assert_eq!(db.block_height().await.unwrap(), 0);
    }
}
