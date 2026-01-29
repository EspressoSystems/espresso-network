use std::{future::Future, path::PathBuf, str::FromStr};

use alloy::primitives::Address;
use anyhow::{Context, Result};
use derive_more::{Display, From};
use espresso_types::{v0_3::Validator, PubKey, SeqTypes, StakeTableState};
use futures::TryStreamExt;
use hotshot_query_service::{
    availability::{BlockId, LeafId, LeafQueryData},
    types::HeightIndexed,
};
use hotshot_types::{
    data::EpochNumber, light_client::StateVerKey, traits::node_implementation::ConsensusTime,
};
use serde_json::Value;
use sqlx::{
    query, query_as,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    QueryBuilder, SqlitePool,
};

/// Different ways to ask the database for a leaf.
#[derive(Clone, Copy, Debug, Display, From)]
pub enum LeafRequest {
    /// Ask for a leaf with a given ID.
    #[display("leaf {_0}")]
    Leaf(LeafId<SeqTypes>),

    /// Ask for the leaf containing a header with a given ID.
    #[display("header {_0}")]
    Header(BlockId<SeqTypes>),
}

/// Client-side database for a [`LightClient`].
pub trait Storage: Sized + Send + Sync + 'static {
    /// Create a default, empty instance of the state.
    ///
    /// This is an async, fallible version of [`Default::default`]. If `Self: Default`, this is
    /// equivalent to `ready(Ok(<Self as Default>::default()))`.
    fn default() -> impl Send + Future<Output = Result<Self>>;

    /// Get the number of blocks known to be in the chain.
    ///
    /// This is equivalent to one more than the block number of the latest known block.
    ///
    /// Because the database is not constantly being updated, this may be an underestimate of the
    /// true number of blocks that exist.
    fn block_height(&self) -> impl Send + Future<Output = Result<u64>>;

    /// Get the earliest available leaf which is later than or equal to the requested leaf.
    ///
    /// This will either be the leaf requested, or can be used as the known-finalized endpoint in a
    /// leaf chain proving that requested leaf is finalized (after the requested leaf is fetched
    /// from elsewhere).
    ///
    /// If there is no known leaf later than the requested leaf, the result is [`None`].
    fn leaf_upper_bound(
        &self,
        leaf: impl Into<LeafRequest> + Send,
    ) -> impl Send + Future<Output = Result<Option<LeafQueryData<SeqTypes>>>>;

    /// Get all leaves in the range [start, end)
    fn get_leaves_in_range(
        &self,
        start: u32,
        end: u32,
    ) -> impl Send + Future<Output = Result<Vec<LeafQueryData<SeqTypes>>>>;

    /// Add a leaf to the cache.
    ///
    /// This may result in an older leaf being removed.
    fn insert_leaf(&self, leaf: LeafQueryData<SeqTypes>)
        -> impl Send + Future<Output = Result<()>>;

    /// Get the stake table for the latest epoch which is not later than `epoch`.
    ///
    /// If such a stake table is available in the database, returns the ordered entries and the
    /// epoch number of the stake table that was loaded.
    fn stake_table_lower_bound(
        &self,
        epoch: EpochNumber,
    ) -> impl Send + Future<Output = Result<Option<(EpochNumber, StakeTableState)>>>;

    /// Add a stake table to the cache.
    ///
    /// This may result in an older stake table being removed.
    fn insert_stake_table(
        &self,
        epoch: EpochNumber,
        stake_table: &StakeTableState,
    ) -> impl Send + Future<Output = Result<()>>;
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "clap", derive(clap::Parser))]
pub struct LightClientSqliteOptions {
    /// Maximum number of simultaneous DB connections to allow.
    #[cfg_attr(
        feature = "clap",
        clap(
            long = "light-client-db-num-connections",
            env = "LIGHT_CLIENT_DB_NUM_CONNECTIONS",
            default_value = "5",
        )
    )]
    pub num_connections: u32,

    /// Maximum number of leaves to cache in the local DB.
    #[cfg_attr(
        feature = "clap",
        clap(
            long = "light-client-db-num-leaves",
            env = "LIGHT_CLIENT_DB_NUM_LEAVES",
            default_value = "100",
        )
    )]
    pub num_leaves: u32,

    /// Maximum number of stake tables to cache in the local DB.
    #[cfg_attr(
        feature = "clap",
        clap(
            long = "light-client-db-num-stake-tables",
            env = "LIGHT_CLIENT_DB_NUM_STAKE_TABLES",
            default_value = "100",
        )
    )]
    pub num_stake_tables: u32,

    /// Create or open storage that is persisted on the file system.
    ///
    /// If not present, the database will exist only in memory and will be destroyed when the
    /// [`SqlitePersistence`] object is dropped.
    #[cfg_attr(
        feature = "clap",
        clap(long = "light-client-db-path", env = "LIGHT_CLIENT_DB_PATH")
    )]
    pub lc_path: Option<PathBuf>,
}

impl Default for LightClientSqliteOptions {
    fn default() -> Self {
        Self {
            num_connections: 5,
            num_leaves: 100,
            num_stake_tables: 100,
            lc_path: None,
        }
    }
}

impl LightClientSqliteOptions {
    /// Create or connect to a database with the given options.
    pub async fn connect(self) -> Result<SqliteStorage> {
        let path = match &self.lc_path {
            Some(path) => path.to_str().context("invalid file path")?,
            None => ":memory:",
        };
        let opt = SqliteConnectOptions::from_str(path)?.create_if_missing(true);
        let pool = SqlitePoolOptions::default()
            .max_connections(self.num_connections)
            .connect_with(opt)
            .await?;
        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(SqliteStorage {
            pool,
            num_leaves: self.num_leaves,
            num_stake_tables: self.num_stake_tables,
        })
    }
}

/// [`Storage`] based on a SQLite database.
#[derive(Clone, Debug)]
pub struct SqliteStorage {
    pool: SqlitePool,
    num_leaves: u32,
    num_stake_tables: u32,
}

impl Storage for SqliteStorage {
    async fn default() -> Result<Self> {
        LightClientSqliteOptions::default().connect().await
    }

    async fn block_height(&self) -> Result<u64> {
        let mut tx = self.pool.begin().await?;
        let (height,) = query_as("SELECT COALESCE(max(height) + 1, 0) FROM leaf")
            .fetch_one(tx.as_mut())
            .await?;
        Ok(height)
    }

    async fn leaf_upper_bound(
        &self,
        id: impl Into<LeafRequest> + Send,
    ) -> Result<Option<LeafQueryData<SeqTypes>>> {
        let mut tx = self.pool.begin().await?;

        let mut q = QueryBuilder::new("SELECT height, data FROM leaf WHERE ");
        match id.into() {
            LeafRequest::Leaf(LeafId::Number(n)) | LeafRequest::Header(BlockId::Number(n)) => {
                q.push("height >= ")
                    .push_bind(n as i64)
                    .push("ORDER BY HEIGHT");
            },
            LeafRequest::Leaf(LeafId::Hash(h)) => {
                q.push("hash = ").push_bind(h.to_string());
            },
            LeafRequest::Header(BlockId::Hash(h)) => {
                q.push("block_hash = ").push_bind(h.to_string());
            },
            LeafRequest::Header(BlockId::PayloadHash(h)) => {
                q.push("payload_hash = ")
                    .push_bind(h.to_string())
                    .push("ORDER BY height");
            },
        }
        q.push(" LIMIT 1");

        let Some((height, data)) = q
            .build_query_as::<(i64, _)>()
            .fetch_optional(tx.as_mut())
            .await?
        else {
            return Ok(None);
        };
        let leaf = serde_json::from_value(data)?;

        // Mark this leaf as recently used.
        let (id,): (i32,) = query_as("SELECT max(id) + 1 FROM leaf")
            .fetch_one(tx.as_mut())
            .await?;
        query("UPDATE leaf SET id = $1 WHERE height = $2")
            .bind(id)
            .bind(height)
            .execute(tx.as_mut())
            .await?;
        tx.commit().await?;

        Ok(Some(leaf))
    }

    async fn get_leaves_in_range(
        &self,
        start_height: u32,
        end_height: u32,
    ) -> Result<Vec<LeafQueryData<SeqTypes>>> {
        let mut tx = self.pool.begin().await?;

        let leaves = query_as::<_, (i64, serde_json::Value)>(
            "SELECT height, data FROM leaf WHERE height >= $1 AND height < $2 ORDER BY height",
        )
        .bind(start_height as i64)
        .bind(end_height as i64)
        .fetch_all(tx.as_mut())
        .await?
        .into_iter()
        .map(|(_height, data)| serde_json::from_value(data))
        .collect::<Result<Vec<_>, _>>()?;

        tx.commit().await?;

        Ok(leaves)
    }

    async fn insert_leaf(&self, leaf: LeafQueryData<SeqTypes>) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        let height = leaf.height() as i64;
        let hash = leaf.hash().to_string();
        let block_hash = leaf.block_hash().to_string();
        let payload_hash = leaf.payload_hash().to_string();
        let data = serde_json::to_value(leaf)?;

        tracing::debug!(height, hash, "inserting leaf");
        let (id,): (i32,) = query_as(
            "INSERT INTO leaf (height, hash, block_hash, payload_hash, data) VALUES ($1, $2, $3, \
             $4, $5)
                    ON CONFLICT (height) DO UPDATE SET id = excluded.id
                    RETURNING id",
        )
        .bind(height)
        .bind(&hash)
        .bind(&block_hash)
        .bind(&payload_hash)
        .bind(data)
        .fetch_one(tx.as_mut())
        .await
        .context("inserting new leaf")?;
        tracing::debug!(height, hash, id, "inserted leaf");

        // Delete the oldest leaves as necessary until the number of leaves stored does not exceed
        // `num_leaves`.
        let (num_leaves,): (u32,) = query_as("SELECT count(*) FROM leaf")
            .fetch_one(tx.as_mut())
            .await
            .context("counting leaves")?;
        let to_delete = num_leaves.saturating_sub(self.num_leaves);
        if to_delete > 0 {
            let (id_to_delete,): (i64,) =
                query_as("SELECT id FROM leaf ORDER BY id LIMIT 1 OFFSET $1")
                    .bind(to_delete - 1)
                    .fetch_one(tx.as_mut())
                    .await
                    .context("finding timestamp for GC")?;
            tracing::info!(id_to_delete, "garbage collecting {to_delete} leaves");
            let res = query("DELETE FROM leaf WHERE id <= $1")
                .bind(id_to_delete)
                .execute(tx.as_mut())
                .await
                .context("deleting old leaves")?;
            tracing::info!("deleted {} leaves", res.rows_affected());
        }

        tx.commit().await?;
        Ok(())
    }

    async fn stake_table_lower_bound(
        &self,
        epoch: EpochNumber,
    ) -> Result<Option<(EpochNumber, StakeTableState)>> {
        let mut tx = self.pool.begin().await?;

        let Some((epoch,)) = query_as::<_, (i64,)>(
            "SELECT epoch FROM stake_table_epoch WHERE epoch <= $1 ORDER BY epoch DESC LIMIT 1",
        )
        .bind(*epoch as i64)
        .fetch_optional(tx.as_mut())
        .await
        .context("loading epoch lower bound")?
        else {
            return Ok(None);
        };

        let validators = query_as::<_, (Value,)>(
            "SELECT data FROM stake_table_validator WHERE epoch = $1 ORDER BY idx",
        )
        .bind(epoch)
        .fetch(tx.as_mut())
        .map_err(anyhow::Error::new)
        .and_then(|(json,)| async move {
            let validator: Validator<PubKey> = serde_json::from_value(json)?;
            Ok((validator.account, validator))
        })
        .try_collect()
        .await
        .context(format!("loading stake table for epoch {epoch}"))?;

        let validator_exits =
            query_as::<_, (String,)>("SELECT address FROM stake_table_exit WHERE epoch <= $1")
                .bind(epoch)
                .fetch(tx.as_mut())
                .map_err(anyhow::Error::new)
                .and_then(|(s,)| async move { Ok(Address::from_str(&s)?) })
                .try_collect()
                .await
                .context(format!("loading validator exits for epoch {epoch}"))?;

        let used_bls_keys =
            query_as::<_, (String,)>("SELECT key FROM stake_table_bls_key WHERE epoch <= $1")
                .bind(epoch)
                .fetch(tx.as_mut())
                .map_err(anyhow::Error::new)
                .and_then(|(s,)| async move { Ok(PubKey::from_str(&s)?) })
                .try_collect()
                .await
                .context(format!("loading BLS keys for epoch {epoch}"))?;

        let used_schnorr_keys =
            query_as::<_, (String,)>("SELECT key FROM stake_table_schnorr_key WHERE epoch <= $1")
                .bind(epoch)
                .fetch(tx.as_mut())
                .map_err(anyhow::Error::new)
                .and_then(|(s,)| async move { Ok(StateVerKey::from_str(&s)?) })
                .try_collect()
                .await
                .context(format!("loading Schnorr keys for epoch {epoch}"))?;

        Ok(Some((
            EpochNumber::new(epoch as u64),
            StakeTableState::new(
                validators,
                validator_exits,
                used_bls_keys,
                used_schnorr_keys,
            ),
        )))
    }

    async fn insert_stake_table(
        &self,
        epoch: EpochNumber,
        stake_table: &StakeTableState,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Record that the stake table for this epoch is available.
        let epoch = i64::try_from(*epoch).context("epoch overflow")?;
        query("INSERT INTO stake_table_epoch (epoch) VALUES ($1)")
            .bind(epoch)
            .execute(tx.as_mut())
            .await
            .context(format!(
                "recording stake table availability for epoch {epoch}"
            ))?;

        // Insert validators for the new stake table.
        let validators = stake_table
            .validators()
            .values()
            .cloned()
            .map(serde_json::to_value)
            .collect::<Result<Vec<_>, _>>()?;
        QueryBuilder::new("INSERT INTO stake_table_validator (epoch, idx, data) ")
            .push_values(validators.into_iter().enumerate(), |mut q, (i, data)| {
                q.push_bind(epoch).push_bind(i as i64).push_bind(data);
            })
            .build()
            .execute(tx.as_mut())
            .await
            .context(format!("inserting validators for epoch {epoch}"))?;

        // Insert only newly used BLS keys.
        QueryBuilder::new("INSERT INTO stake_table_bls_key (epoch, key) ")
            .push_values(stake_table.used_bls_keys(), |mut q, key| {
                q.push_bind(epoch).push_bind(key.to_string());
            })
            // If we insert keys out of order, make sure `epoch` reflects the earliest time when
            // this key was added to the state.
            .push(" ON CONFLICT (key) DO UPDATE SET epoch = min(epoch, excluded.epoch)")
            .build()
            .execute(tx.as_mut())
            .await
            .context(format!("inserting newly used BLS keys for epoch {epoch}"))?;

        // Insert only newly used Schnorr keys.
        QueryBuilder::new("INSERT INTO stake_table_schnorr_key (epoch, key) ")
            .push_values(stake_table.used_schnorr_keys(), |mut q, key| {
                q.push_bind(epoch).push_bind(key.to_string());
            })
            // If we insert keys out of order, make sure `epoch` reflects the earliest time when
            // this key was added to the state.
            .push(" ON CONFLICT (key) DO UPDATE SET epoch = min(epoch, excluded.epoch)")
            .build()
            .execute(tx.as_mut())
            .await
            .context(format!(
                "inserting newly used Schnorr keys for epoch {epoch}"
            ))?;

        // Insert only the new validator exits.
        if !stake_table.validator_exits().is_empty() {
            QueryBuilder::new("INSERT INTO stake_table_exit (epoch, address) ")
                .push_values(stake_table.validator_exits(), |mut q, address| {
                    q.push_bind(epoch).push_bind(address.to_string());
                })
                // If we insert exits out of order, make sure `epoch` reflects the earliest time
                // when this exit was added to the state.
                .push(" ON CONFLICT (address) DO UPDATE SET epoch = min(epoch, excluded.epoch)")
                .build()
                .execute(tx.as_mut())
                .await
                .context(format!("inserting new validator exits for epoch {epoch}"))?;
        }

        // Delete the second oldest stake table if necessary to ensure the number of stake tables
        // stored does not exceed `num_stake_tables`.
        let (num_stake_tables,): (u32,) = query_as("SELECT count(*) FROM stake_table_epoch")
            .fetch_one(tx.as_mut())
            .await
            .context("counting stake tables")?;
        if num_stake_tables > self.num_stake_tables {
            // We always delete the _second oldest_ stake table. We want to keep the oldest around
            // because it is the hardest to catch up for if we need it again (we would have to go
            // all the way back to genesis). The second oldest is the least likely to be used again
            // after the oldest, while still being easy to replay if we do need it (because we can
            // just replay from the cached oldest).
            let (epoch_to_delete,): (i64,) =
                query_as("SELECT epoch FROM stake_table_epoch ORDER BY epoch LIMIT 1 OFFSET 1")
                    .fetch_one(tx.as_mut())
                    .await
                    .context("find second oldest epoch")?;
            tracing::info!(epoch_to_delete, "garbage collecting stake table");

            // Delete from the main epoch table. The corresponding rows from `stake_table_validator`
            // will be deleted automatically by cascading. The corresponding rows in the BLS keys,
            // Schnorr keys, and validator exits tables cannot be deleted, because those tables are
            // cumulative over later epochs.
            query("DELETE FROM stake_table_epoch WHERE epoch = $1")
                .bind(epoch_to_delete)
                .execute(tx.as_mut())
                .await
                .context("garbage collecting stake table")?;
        }

        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use espresso_types::EpochVersion;
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::testing::{leaf_chain, random_validator};

    #[tokio::test]
    #[test_log::test]
    async fn test_block_height() {
        let db = SqliteStorage::default().await.unwrap();

        // Test with empty db.
        assert_eq!(db.block_height().await.unwrap(), 0);

        // Test with nonconsecutive leaves.
        let leaf = leaf_chain::<EpochVersion>(100..101).await.remove(0);
        db.insert_leaf(leaf).await.unwrap();
        assert_eq!(db.block_height().await.unwrap(), 101);
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_leaf_upper_bound_exact() {
        let db = SqliteStorage::default().await.unwrap();

        let leaf = leaf_chain::<EpochVersion>(0..1).await.remove(0);
        db.insert_leaf(leaf.clone()).await.unwrap();
        assert_eq!(
            db.leaf_upper_bound(LeafId::Number(0))
                .await
                .unwrap()
                .unwrap(),
            leaf
        );
        assert_eq!(
            db.leaf_upper_bound(LeafId::Hash(leaf.hash()))
                .await
                .unwrap()
                .unwrap(),
            leaf
        );
        assert_eq!(
            db.leaf_upper_bound(BlockId::Number(0))
                .await
                .unwrap()
                .unwrap(),
            leaf
        );
        assert_eq!(
            db.leaf_upper_bound(BlockId::Hash(leaf.block_hash()))
                .await
                .unwrap()
                .unwrap(),
            leaf
        );
        assert_eq!(
            db.leaf_upper_bound(BlockId::PayloadHash(leaf.payload_hash()))
                .await
                .unwrap()
                .unwrap()
                .payload_hash(),
            leaf.payload_hash()
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_leaf_upper_bound_loose() {
        let db = SqliteStorage::default().await.unwrap();

        let leaves = leaf_chain::<EpochVersion>(0..=1).await;
        db.insert_leaf(leaves[1].clone()).await.unwrap();
        assert_eq!(
            db.leaf_upper_bound(LeafId::Number(0))
                .await
                .unwrap()
                .unwrap(),
            leaves[1]
        );
        // Searching by hash either gives an exact match or fails, there is no way of "upper
        // bounding" a hash.
        assert_eq!(
            db.leaf_upper_bound(LeafId::Hash(leaves[0].hash()))
                .await
                .unwrap(),
            None
        );
        assert_eq!(
            db.leaf_upper_bound(BlockId::Hash(leaves[0].block_hash()))
                .await
                .unwrap(),
            None
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_leaf_upper_bound_least_upper_bound() {
        let db = SqliteStorage::default().await.unwrap();

        let leaves = leaf_chain::<EpochVersion>(0..=2).await;
        db.insert_leaf(leaves[2].clone()).await.unwrap();
        db.insert_leaf(leaves[1].clone()).await.unwrap();
        assert_eq!(
            db.leaf_upper_bound(LeafId::Number(0))
                .await
                .unwrap()
                .unwrap(),
            leaves[1]
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_leaf_upper_bound_not_found() {
        let db = SqliteStorage::default().await.unwrap();

        let leaves = leaf_chain::<EpochVersion>(0..=1).await;
        db.insert_leaf(leaves[0].clone()).await.unwrap();
        assert_eq!(db.leaf_upper_bound(LeafId::Number(1)).await.unwrap(), None);
        assert_eq!(
            db.leaf_upper_bound(LeafId::Hash(leaves[1].hash()))
                .await
                .unwrap(),
            None
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_gc_last_inserted() {
        let db = LightClientSqliteOptions {
            num_leaves: 1,
            ..Default::default()
        }
        .connect()
        .await
        .unwrap();

        let leaves = leaf_chain::<EpochVersion>(0..=1).await;
        db.insert_leaf(leaves[1].clone()).await.unwrap();
        db.insert_leaf(leaves[0].clone()).await.unwrap();

        assert_eq!(
            db.leaf_upper_bound(LeafId::Number(0))
                .await
                .unwrap()
                .unwrap(),
            leaves[0]
        );
        assert_eq!(db.leaf_upper_bound(LeafId::Number(1)).await.unwrap(), None);
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_gc_last_selected() {
        let db = LightClientSqliteOptions {
            num_leaves: 2,
            ..Default::default()
        }
        .connect()
        .await
        .unwrap();

        let leaves = leaf_chain::<EpochVersion>(0..=2).await;
        db.insert_leaf(leaves[0].clone()).await.unwrap();
        db.insert_leaf(leaves[1].clone()).await.unwrap();

        // Select leaf 0, making it more recently used than leaf 1.
        assert_eq!(
            db.leaf_upper_bound(LeafId::Number(0))
                .await
                .unwrap()
                .unwrap(),
            leaves[0]
        );

        // Insert a third leaf, causing the least recently used (leaf 1) to be garbage collected.
        db.insert_leaf(leaves[2].clone()).await.unwrap();

        assert_eq!(
            db.leaf_upper_bound(LeafId::Number(0))
                .await
                .unwrap()
                .unwrap(),
            leaves[0]
        );
        assert_eq!(
            db.leaf_upper_bound(LeafId::Number(1))
                .await
                .unwrap()
                .unwrap(),
            leaves[2]
        );
        assert_eq!(
            db.leaf_upper_bound(LeafId::Number(2))
                .await
                .unwrap()
                .unwrap(),
            leaves[2]
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_get_leaves_in_range() {
        let db = SqliteStorage::default().await.unwrap();

        let leaves = leaf_chain::<EpochVersion>(0..5).await;
        for leaf in &leaves {
            db.insert_leaf(leaf.clone()).await.unwrap();
        }

        let fetched = db.get_leaves_in_range(1, 4).await.unwrap();
        assert_eq!(fetched, leaves[1..4]);
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_get_leaves_in_range_not_found() {
        let db = SqliteStorage::default().await.unwrap();

        let leaves = leaf_chain::<EpochVersion>(0..3).await;
        for leaf in &leaves {
            db.insert_leaf(leaf.clone()).await.unwrap();
        }

        let fetched = db.get_leaves_in_range(3, 5).await.unwrap();
        assert!(fetched.is_empty());
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_stake_table_lower_bound_exact() {
        let db = SqliteStorage::default().await.unwrap();

        let epoch = EpochNumber::new(1);
        let state = random_stake_table();
        db.insert_stake_table(epoch, &state).await.unwrap();
        assert_eq!(
            db.stake_table_lower_bound(epoch).await.unwrap().unwrap(),
            (epoch, state)
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_stake_table_lower_bound_loose() {
        let db = SqliteStorage::default().await.unwrap();

        let epoch = EpochNumber::new(1);
        let state = random_stake_table();
        db.insert_stake_table(epoch, &state).await.unwrap();
        assert_eq!(
            db.stake_table_lower_bound(epoch + 1)
                .await
                .unwrap()
                .unwrap(),
            (epoch, state)
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_stake_table_lower_bound_greatest_lower_bound() {
        let db = SqliteStorage::default().await.unwrap();

        let state1 = random_stake_table();
        let state2 = chain_stake_table(&state1);
        db.insert_stake_table(EpochNumber::new(1), &state1)
            .await
            .unwrap();
        db.insert_stake_table(EpochNumber::new(2), &state2)
            .await
            .unwrap();

        assert_eq!(
            db.stake_table_lower_bound(EpochNumber::new(3))
                .await
                .unwrap()
                .unwrap(),
            (EpochNumber::new(2), state2)
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_stake_table_lower_bound_not_found() {
        let db = SqliteStorage::default().await.unwrap();
        db.insert_stake_table(EpochNumber::new(2), &random_stake_table())
            .await
            .unwrap();
        assert_eq!(
            db.stake_table_lower_bound(EpochNumber::new(1))
                .await
                .unwrap(),
            None
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_stake_table_gc() {
        let db = LightClientSqliteOptions {
            num_stake_tables: 2,
            ..Default::default()
        }
        .connect()
        .await
        .unwrap();

        let state1 = random_stake_table();
        let state2 = chain_stake_table(&state1);
        let state3 = chain_stake_table(&state2);
        db.insert_stake_table(EpochNumber::new(1), &state1)
            .await
            .unwrap();
        db.insert_stake_table(EpochNumber::new(2), &state2)
            .await
            .unwrap();
        db.insert_stake_table(EpochNumber::new(3), &state3)
            .await
            .unwrap();

        assert_eq!(
            db.stake_table_lower_bound(EpochNumber::new(1))
                .await
                .unwrap()
                .unwrap(),
            (EpochNumber::new(1), state1.clone())
        );
        assert_eq!(
            db.stake_table_lower_bound(EpochNumber::new(2))
                .await
                .unwrap()
                .unwrap(),
            (EpochNumber::new(1), state1)
        );
        assert_eq!(
            db.stake_table_lower_bound(EpochNumber::new(3))
                .await
                .unwrap()
                .unwrap(),
            (EpochNumber::new(3), state3)
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_stake_table_insert_out_of_order() {
        let db = SqliteStorage::default().await.unwrap();

        let state1 = random_stake_table();
        let state2 = chain_stake_table(&state1);
        db.insert_stake_table(EpochNumber::new(2), &state2)
            .await
            .unwrap();
        db.insert_stake_table(EpochNumber::new(1), &state1)
            .await
            .unwrap();

        assert_eq!(
            db.stake_table_lower_bound(EpochNumber::new(1))
                .await
                .unwrap()
                .unwrap(),
            (EpochNumber::new(1), state1)
        );
        assert_eq!(
            db.stake_table_lower_bound(EpochNumber::new(2))
                .await
                .unwrap()
                .unwrap(),
            (EpochNumber::new(2), state2)
        );
    }

    /// Make a stake table state with all fields populated.
    fn random_stake_table() -> StakeTableState {
        let validator = random_validator();
        StakeTableState::new(
            [(validator.account, validator.clone())]
                .into_iter()
                .collect(),
            [Address::random()].into_iter().collect(),
            [validator.stake_table_key].into_iter().collect(),
            [validator.state_ver_key].into_iter().collect(),
        )
    }

    /// Create a new stake table state which is a possible successor to the given state.
    fn chain_stake_table(state: &StakeTableState) -> StakeTableState {
        let new_validator = random_validator();
        let new_exit = Address::random();
        StakeTableState::new(
            state
                .validators()
                .values()
                .chain([&new_validator])
                .map(|v| (v.account, v.clone()))
                .collect(),
            state
                .validator_exits()
                .iter()
                .chain([&new_exit])
                .cloned()
                .collect(),
            state
                .used_bls_keys()
                .iter()
                .chain([&new_validator.stake_table_key])
                .cloned()
                .collect(),
            state
                .used_schnorr_keys()
                .iter()
                .chain([&new_validator.state_ver_key])
                .cloned()
                .collect(),
        )
    }
}
