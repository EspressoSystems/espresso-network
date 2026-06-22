use std::{
    collections::HashMap,
    future::Future,
    path::PathBuf,
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicI64, Ordering},
    },
    time::Duration,
};
#[cfg(unix)]
use std::{fs::Permissions, os::unix::fs::PermissionsExt};

use alloy::primitives::Address;
use anyhow::{Context, Result};
use derive_more::{Display, From};
use espresso_types::{
    BackoffParams, PubKey, Ratio, SeqTypes, StakeTableState, v0_3::RegisteredValidator,
};
use futures::TryStreamExt;
use hotshot_query_service_types::{
    HeightIndexed,
    availability::{BlockId, LeafId, LeafQueryData},
};
use hotshot_types::{data::EpochNumber, light_client::StateVerKey, x25519};
use serde::Serialize;
use serde_json::Value;
use sqlx::{QueryBuilder, SqlitePool, query, query_as, sqlite::SqlitePoolOptions};
use tempfile::{Builder, TempDir};
use vbs::version::Version;

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

/// Maximum number of retries for a failed write before propagating the error.
const WRITE_RETRY_MAX: u32 = 5;

/// Backoff for retrying failed writes. Staggered so concurrent writers don't lock-step.
const WRITE_BACKOFF: BackoffParams = BackoffParams::new(
    Duration::from_millis(50),
    Duration::from_millis(1_000),
    2,
    Ratio {
        numerator: 5,
        denominator: 10,
    },
);

/// In-memory LRU recency tracker shared across all clones of a [`SqliteStorage`].
///
/// `touch` is called on the read path (pure, no DB write). `drain` is called inside
/// `insert_leaf` to flush pending recency updates as part of the existing write transaction.
/// Held in an `Arc`, so its `Drop` runs exactly once, when the last `SqliteStorage` clone is
/// dropped, flushing any touches that no `insert_leaf` persisted (graceful shutdown).
#[derive(Debug)]
struct Recency {
    /// Monotonically increasing tick counter. Persisted maximum is seeded at `connect` time so
    /// ticks always exceed any value already stored in the DB after a restart.
    next_tick: AtomicI64,
    /// height -> latest tick; flushed to DB by the next `insert_leaf` or by `Drop`.
    dirty: std::sync::Mutex<HashMap<i64, i64>>,
    /// Pool handle kept alive for the on-drop flush.
    pool: SqlitePool,
}

impl Recency {
    fn touch(&self, height: i64) {
        let t = self.next_tick.fetch_add(1, Ordering::Relaxed);
        self.dirty.lock().unwrap().insert(height, t);
    }

    fn drain(&self) -> Vec<(i64, i64)> {
        self.dirty.lock().unwrap().drain().collect()
    }
}

impl Drop for Recency {
    /// Flush pending read-path touches to the DB on graceful shutdown.
    ///
    /// Touches are otherwise only persisted by the next `insert_leaf`; without this, GC after a
    /// restart would rank recently-read leaves by a stale `last_used` and could evict them.
    /// Best-effort: a failure only degrades GC ranking, it never corrupts data. The flush is a
    /// short, file-local write driven on the current thread; the SQLite work runs on sqlx's own
    /// worker thread, so it does not deadlock a runtime worker.
    fn drop(&mut self) {
        let pending: Vec<(i64, i64)> = self.dirty.get_mut().unwrap().drain().collect();
        if pending.is_empty() {
            return;
        }
        let pool = self.pool.clone();
        let flush = async move {
            let mut tx = pool.begin().await?;
            for (h, tick) in &pending {
                query("UPDATE leaf SET last_used = $1 WHERE height = $2")
                    .bind(tick)
                    .bind(h)
                    .execute(tx.as_mut())
                    .await?;
            }
            tx.commit().await
        };
        if let Err(err) = futures::executor::block_on(flush) {
            tracing::warn!(%err, "failed to flush LRU recency on drop");
        }
    }
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
    /// If such a stake table is available in the database, returns the loaded epoch number, the
    /// stake table state, the protocol version of the epoch root header under whose rules that
    /// stake table's active set was selected, and the protocol version of the epoch root header
    /// in the next epoch (used to seed iter 1 of a catchup that resumes from this row).
    fn stake_table_lower_bound(
        &self,
        epoch: EpochNumber,
    ) -> impl Send + Future<Output = Result<Option<(EpochNumber, StakeTableState, Version, Version)>>>;

    /// Add a stake table to the cache.
    ///
    /// `epoch_root_protocol_version` is the protocol version of the epoch root header in epoch
    /// `e-2` (the snapshot point), used so that future cache hits can apply the same active-set
    /// selection rules without re-fetching the root.
    ///
    /// `next_epoch_root_protocol_version` is the protocol version of the epoch root header in
    /// epoch `e-1` (the snapshot point for `e+1`), used so that a future catchup resuming from
    /// this row can seed iter 1's filter version without re-fetching that root.
    ///
    /// This may result in an older stake table being removed.
    fn insert_stake_table(
        &self,
        epoch: EpochNumber,
        stake_table: &StakeTableState,
        epoch_root_protocol_version: Version,
        next_epoch_root_protocol_version: Version,
    ) -> impl Send + Future<Output = Result<()>>;
}

#[derive(Clone, Debug, Serialize)]
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

    /// Path at which the light client database is persisted.
    ///
    /// If not present, the database is created in a temporary directory that is removed when the
    /// storage is dropped. Set this in production so cached leaves and stake tables survive
    /// restarts.
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
        let (path, _tmp) = match self.lc_path {
            Some(p) => {
                if let Some(parent) = p.parent().filter(|d| !d.as_os_str().is_empty()) {
                    std::fs::create_dir_all(parent)
                        .with_context(|| format!("creating parent directory {parent:?}"))?;
                }
                (p, None)
            },
            None => {
                let mut builder = Builder::new();
                builder.prefix("espresso-lc-");
                #[cfg(unix)]
                builder.permissions(Permissions::from_mode(0o700));
                let dir = builder.tempdir().context(
                    "creating temporary directory for light client database; set \
                     LIGHT_CLIENT_DB_PATH to use a persistent location",
                )?;
                let path = dir.path().join("lc.db");
                (path, Some(Arc::new(dir)))
            },
        };

        let opt = hotshot_query_service::sqlite_options::sqlite_options().filename(&path);
        let pool = SqlitePoolOptions::default()
            .max_connections(self.num_connections)
            .connect_with(opt)
            .await?;
        sqlx::migrate!("./migrations").run(&pool).await?;

        // Seed the tick counter so new ticks always exceed any value persisted in the DB.
        let (max_used,): (i64,) = sqlx::query_as("SELECT COALESCE(MAX(last_used), 0) FROM leaf")
            .fetch_one(&pool)
            .await?;
        let recency = Arc::new(Recency {
            next_tick: AtomicI64::new(max_used + 1),
            dirty: Default::default(),
            pool: pool.clone(),
        });

        Ok(SqliteStorage {
            pool,
            num_leaves: self.num_leaves,
            num_stake_tables: self.num_stake_tables,
            recency,
            _tmp,
        })
    }
}

/// [`Storage`] based on a SQLite database.
#[derive(Clone, Debug)]
pub struct SqliteStorage {
    pool: SqlitePool,
    num_leaves: u32,
    num_stake_tables: u32,
    /// Shared across all clones; all map operations are sync and never held across `.await`.
    recency: Arc<Recency>,
    _tmp: Option<Arc<TempDir>>,
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
        let mut q = QueryBuilder::new("SELECT data FROM leaf WHERE ");
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

        let Some((data,)) = q
            .build_query_as::<(serde_json::Value,)>()
            .fetch_optional(&self.pool)
            .await?
        else {
            return Ok(None);
        };

        let leaf: LeafQueryData<SeqTypes> = serde_json::from_value(data)?;
        self.recency.touch(leaf.height() as i64);
        Ok(Some(leaf))
    }

    async fn get_leaves_in_range(
        &self,
        start_height: u32,
        end_height: u32,
    ) -> Result<Vec<LeafQueryData<SeqTypes>>> {
        query_as::<_, (i64, serde_json::Value)>(
            "SELECT height, data FROM leaf WHERE height >= $1 AND height < $2 ORDER BY height",
        )
        .bind(start_height as i64)
        .bind(end_height as i64)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|(height, data)| {
            let leaf = serde_json::from_value(data)?;
            self.recency.touch(height);
            Ok(leaf)
        })
        .collect::<Result<Vec<_>, serde_json::Error>>()
        .map_err(anyhow::Error::new)
    }

    async fn insert_leaf(&self, leaf: LeafQueryData<SeqTypes>) -> Result<()> {
        let height = leaf.height() as i64;
        let hash = leaf.hash().to_string();
        let block_hash = leaf.block_hash().to_string();
        let payload_hash = leaf.payload_hash().to_string();
        let data = serde_json::to_value(&leaf)?;

        // Compute both values before the retry loop so retries reuse them (idempotent).
        let pending = self.recency.drain();
        let insert_tick = self.recency.next_tick.fetch_add(1, Ordering::Relaxed);

        let result = WRITE_BACKOFF
            .retry_if(
                WRITE_RETRY_MAX,
                |_| true,
                || async {
                    let mut tx = self.pool.begin().await?;

                    tracing::debug!(height, hash, "inserting leaf");
                    query(
                        "INSERT INTO leaf (height, hash, block_hash, payload_hash, data, \
                         last_used) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (height) DO \
                         UPDATE SET data = excluded.data, last_used = excluded.last_used",
                    )
                    .bind(height)
                    .bind(&hash)
                    .bind(&block_hash)
                    .bind(&payload_hash)
                    .bind(&data)
                    .bind(insert_tick)
                    .execute(tx.as_mut())
                    .await
                    .context("inserting new leaf")?;
                    tracing::debug!(height, hash, "inserted leaf");

                    // Flush pending recency touches accumulated since the last insert.
                    for (h, tick) in &pending {
                        query("UPDATE leaf SET last_used = $1 WHERE height = $2")
                            .bind(tick)
                            .bind(h)
                            .execute(tx.as_mut())
                            .await
                            .context("flushing recency touch")?;
                    }

                    // GC: evict least-recently-used leaves until count <= num_leaves.
                    let (num_leaves,): (u32,) = query_as("SELECT count(*) FROM leaf")
                        .fetch_one(tx.as_mut())
                        .await
                        .context("counting leaves")?;
                    let to_delete = num_leaves.saturating_sub(self.num_leaves);
                    if to_delete > 0 {
                        tracing::info!("garbage collecting {to_delete} leaves");
                        let res = query(
                            "DELETE FROM leaf WHERE height IN (SELECT height FROM leaf ORDER BY \
                             last_used ASC, height ASC LIMIT $1)",
                        )
                        .bind(to_delete)
                        .execute(tx.as_mut())
                        .await
                        .context("deleting old leaves")?;
                        tracing::info!("deleted {} leaves", res.rows_affected());
                    }

                    tx.commit().await?;
                    Ok(())
                },
            )
            .await;

        if result.is_err() {
            // Restore drained touches so a recently-read leaf is not wrongly evicted later.
            // Use `or_insert` so any newer touch already recorded for that height wins.
            let mut dirty = self.recency.dirty.lock().unwrap();
            for (h, tick) in pending {
                dirty.entry(h).or_insert(tick);
            }
        }

        result
    }

    async fn stake_table_lower_bound(
        &self,
        epoch: EpochNumber,
    ) -> Result<Option<(EpochNumber, StakeTableState, Version, Version)>> {
        let mut tx = self.pool.begin().await?;

        let Some((epoch, epoch_root_protocol_version, next_epoch_root_protocol_version)) =
            query_as::<_, (i64, String, String)>(
                "SELECT epoch, epoch_root_protocol_version, next_epoch_root_protocol_version FROM \
                 stake_table_epoch WHERE epoch <= $1 ORDER BY epoch DESC LIMIT 1",
            )
            .bind(*epoch as i64)
            .fetch_optional(tx.as_mut())
            .await
            .context("loading epoch lower bound")?
        else {
            return Ok(None);
        };
        let epoch_root_protocol_version = versions::parse_version(&epoch_root_protocol_version)
            .with_context(|| {
                format!(
                    "parsing stored epoch root protocol version {epoch_root_protocol_version:?} \
                     for epoch {epoch}"
                )
            })?;
        let next_epoch_root_protocol_version =
            versions::parse_version(&next_epoch_root_protocol_version).with_context(|| {
                format!(
                    "parsing stored next epoch root protocol version \
                     {next_epoch_root_protocol_version:?} for epoch {epoch}"
                )
            })?;

        let validators = query_as::<_, (Value,)>(
            "SELECT data FROM stake_table_validator WHERE epoch = $1 ORDER BY idx",
        )
        .bind(epoch)
        .fetch(tx.as_mut())
        .map_err(anyhow::Error::new)
        .and_then(|(json,)| async move {
            let validator: RegisteredValidator<PubKey> = serde_json::from_value(json)?;
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

        let used_x25519_keys =
            query_as::<_, (String,)>("SELECT key FROM stake_table_x25519_key WHERE epoch <= $1")
                .bind(epoch)
                .fetch(tx.as_mut())
                .map_err(anyhow::Error::new)
                .and_then(|(s,)| async move { Ok(x25519::PublicKey::from_str(&s)?) })
                .try_collect()
                .await
                .context(format!("loading x25519 keys for epoch {epoch}"))?;

        Ok(Some((
            EpochNumber::new(epoch as u64),
            StakeTableState::new(
                validators,
                validator_exits,
                used_bls_keys,
                used_schnorr_keys,
                used_x25519_keys,
            ),
            epoch_root_protocol_version,
            next_epoch_root_protocol_version,
        )))
    }

    async fn insert_stake_table(
        &self,
        epoch: EpochNumber,
        stake_table: &StakeTableState,
        epoch_root_protocol_version: Version,
        next_epoch_root_protocol_version: Version,
    ) -> Result<()> {
        let epoch = i64::try_from(*epoch).context("epoch overflow")?;
        let epoch_root_protocol_version_str = epoch_root_protocol_version.to_string();
        let next_epoch_root_protocol_version_str = next_epoch_root_protocol_version.to_string();
        let validators = stake_table
            .validators()
            .values()
            .cloned()
            .map(serde_json::to_value)
            .collect::<Result<Vec<_>, _>>()?;

        WRITE_BACKOFF
            .retry_if(
                WRITE_RETRY_MAX,
                |_| true,
                || async {
                    let mut tx = self.pool.begin().await?;

                    // Record that the stake table for this epoch is available, along with the
                    // versions of the epoch root headers in epochs `e-2` and `e-1` (snapshot points
                    // for `e` and `e+1`).
                    query(
                        "INSERT INTO stake_table_epoch (epoch, epoch_root_protocol_version, \
                         next_epoch_root_protocol_version) VALUES ($1, $2, $3)",
                    )
                    .bind(epoch)
                    .bind(&epoch_root_protocol_version_str)
                    .bind(&next_epoch_root_protocol_version_str)
                    .execute(tx.as_mut())
                    .await
                    .context(format!(
                        "recording stake table availability for epoch {epoch}"
                    ))?;

                    QueryBuilder::new("INSERT INTO stake_table_validator (epoch, idx, data) ")
                        .push_values(validators.iter().enumerate(), |mut q, (i, data)| {
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
                    // If we insert keys out of order, make sure `epoch` reflects the earliest time
                    // when this key was added to the state.
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
                    // If we insert keys out of order, make sure `epoch` reflects the earliest time
                    // when this key was added to the state.
                    .push(" ON CONFLICT (key) DO UPDATE SET epoch = min(epoch, excluded.epoch)")
                    .build()
                    .execute(tx.as_mut())
                    .await
                    .context(format!(
                        "inserting newly used Schnorr keys for epoch {epoch}"
                    ))?;

                    // Insert only newly used x25519 keys.
                    if !stake_table.used_x25519_keys().is_empty() {
                        QueryBuilder::new("INSERT INTO stake_table_x25519_key (epoch, key) ")
                        .push_values(stake_table.used_x25519_keys(), |mut q, key| {
                            q.push_bind(epoch).push_bind(key.to_string());
                        })
                        // If we insert keys out of order, make sure `epoch` reflects the earliest
                        // time when this key was added to the state.
                        .push(" ON CONFLICT (key) DO UPDATE SET epoch = min(epoch, excluded.epoch)")
                        .build()
                        .execute(tx.as_mut())
                        .await
                        .context(format!(
                            "inserting newly used x25519 keys for epoch {epoch}"
                        ))?;
                    }

                    // Insert only the new validator exits.
                    if !stake_table.validator_exits().is_empty() {
                        QueryBuilder::new("INSERT INTO stake_table_exit (epoch, address) ")
                        .push_values(stake_table.validator_exits(), |mut q, address| {
                            q.push_bind(epoch).push_bind(address.to_string());
                        })
                        // If we insert exits out of order, make sure `epoch` reflects the earliest
                        // time when this exit was added to the state.
                        .push(
                            " ON CONFLICT (address) DO UPDATE SET epoch = min(epoch, \
                             excluded.epoch)",
                        )
                        .build()
                        .execute(tx.as_mut())
                        .await
                        .context(format!("inserting new validator exits for epoch {epoch}"))?;
                    }

                    // Delete the second oldest stake table if necessary to ensure the number of stake
                    // tables stored does not exceed `num_stake_tables`.
                    let (num_stake_tables,): (u32,) =
                        query_as("SELECT count(*) FROM stake_table_epoch")
                            .fetch_one(tx.as_mut())
                            .await
                            .context("counting stake tables")?;
                    if num_stake_tables > self.num_stake_tables {
                        // We always delete the _second oldest_ stake table. We want to keep the oldest
                        // around because it is the hardest to catch up for if we need it again (we
                        // would have to go all the way back to genesis). The second oldest is the
                        // least likely to be used again after the oldest, while still being easy to
                        // replay if we do need it (because we can just replay from the cached oldest).
                        let (epoch_to_delete,): (i64,) = query_as(
                            "SELECT epoch FROM stake_table_epoch ORDER BY epoch LIMIT 1 OFFSET 1",
                        )
                        .fetch_one(tx.as_mut())
                        .await
                        .context("find second oldest epoch")?;
                        tracing::info!(epoch_to_delete, "garbage collecting stake table");

                        // Delete from the main epoch table. The corresponding rows from
                        // `stake_table_validator` will be deleted automatically by cascading. The
                        // corresponding rows in the BLS keys, Schnorr keys, and validator exits tables
                        // cannot be deleted, because those tables are cumulative over later epochs.
                        query("DELETE FROM stake_table_epoch WHERE epoch = $1")
                            .bind(epoch_to_delete)
                            .execute(tx.as_mut())
                            .await
                            .context("garbage collecting stake table")?;
                    }

                    tx.commit().await?;
                    Ok(())
                },
            )
            .await
    }
}

#[cfg(test)]
mod test {
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    use pretty_assertions::assert_eq;
    use sqlx::sqlite::SqliteConnectOptions;
    use tempfile::tempdir;
    use versions::{CLIQUENET_VERSION, EPOCH_VERSION};

    use super::*;
    use crate::testing::{leaf_chain, random_validator};

    #[tokio::test]
    #[test_log::test]
    async fn test_default_storage_survives_connection_churn() {
        let db = SqliteStorage::default().await.unwrap();

        {
            let conn = db.pool.acquire().await.unwrap();
            drop(conn);
        }

        let leaf = leaf_chain(0..1, EPOCH_VERSION).await.remove(0);
        db.insert_leaf(leaf).await.unwrap();
        assert_eq!(db.block_height().await.unwrap(), 1);
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_default_storage_survives_idle_reap() {
        use std::time::Duration;

        let db = SqliteStorage::default().await.unwrap();
        let path = db
            ._tmp
            .as_ref()
            .expect("default storage must own a tempdir")
            .path()
            .join("lc.db");

        let opt = SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(false);
        let pool = SqlitePoolOptions::default()
            .max_connections(5)
            .min_connections(0)
            .idle_timeout(Some(Duration::from_millis(50)))
            .connect_with(opt)
            .await
            .unwrap();

        let (height,): (i64,) = sqlx::query_as("SELECT COALESCE(max(height) + 1, 0) FROM leaf")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(height, 0);

        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while pool.size() > 0 && tokio::time::Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert_eq!(pool.size(), 0, "pool did not reap idle connections in time");

        let (height,): (i64,) = sqlx::query_as("SELECT COALESCE(max(height) + 1, 0) FROM leaf")
            .fetch_one(&pool)
            .await
            .expect("schema must survive the pool reaping idle connections");
        assert_eq!(height, 0);

        drop(db);
    }

    // Regression: `leaf_upper_bound` previously did a SELECT then an UPDATE
    // (LRU recency bump) inside one write transaction, so every read took the
    // WAL write lock and contended with concurrent inserts under a full-history
    // backfill, producing SQLITE_BUSY. The read path is now a pure SELECT.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_concurrent_reads_do_not_lock() {
        let db = Arc::new(SqliteStorage::default().await.unwrap());
        let leaves = leaf_chain(0..40, EPOCH_VERSION).await;

        let mut tasks = Vec::new();
        for leaf in leaves {
            let db = db.clone();
            tasks.push(tokio::spawn(async move {
                db.insert_leaf(leaf).await.map(|_| ())
            }));
        }
        for h in 0..40usize {
            let read_db = db.clone();
            tasks.push(tokio::spawn(async move {
                read_db
                    .leaf_upper_bound(LeafId::Number(h))
                    .await
                    .map(|_| ())
            }));
            let height_db = db.clone();
            tasks.push(tokio::spawn(async move {
                height_db.block_height().await.map(|_| ())
            }));
        }

        for task in tasks {
            task.await
                .unwrap()
                .expect("concurrent op must not fail with a database lock error");
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    #[test_log::test]
    async fn test_default_storage_tempdir_is_owner_only() {
        let db = SqliteStorage::default().await.unwrap();
        let dir = db
            ._tmp
            .as_ref()
            .expect("default storage must own a tempdir");
        let mode = std::fs::metadata(dir.path()).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700, "tempdir must be owner-only, got {mode:o}");
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_file_backed_creates_parent_dir() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nested").join("subdir").join("lc.db");
        let db = LightClientSqliteOptions {
            lc_path: Some(path.clone()),
            ..Default::default()
        }
        .connect()
        .await
        .unwrap();
        assert_eq!(db.block_height().await.unwrap(), 0);
        assert!(path.exists(), "sqlite file should have been created");
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_block_height() {
        let db = SqliteStorage::default().await.unwrap();

        // Test with empty db.
        assert_eq!(db.block_height().await.unwrap(), 0);

        // Test with nonconsecutive leaves.
        let leaf = leaf_chain(100..101, EPOCH_VERSION).await.remove(0);
        db.insert_leaf(leaf).await.unwrap();
        assert_eq!(db.block_height().await.unwrap(), 101);
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_leaf_upper_bound_exact() {
        let db = SqliteStorage::default().await.unwrap();

        let leaf = leaf_chain(0..1, EPOCH_VERSION).await.remove(0);
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

        let leaves = leaf_chain(0..=1, EPOCH_VERSION).await;
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

        let leaves = leaf_chain(0..=2, EPOCH_VERSION).await;
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

        let leaves = leaf_chain(0..=1, EPOCH_VERSION).await;
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

        let leaves = leaf_chain(0..=1, EPOCH_VERSION).await;
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

    // LRU GC: leaf touched by a read is kept; untouched leaf is evicted.
    #[tokio::test]
    #[test_log::test]
    async fn test_gc_evicts_least_recently_used() {
        let db = LightClientSqliteOptions {
            num_leaves: 2,
            ..Default::default()
        }
        .connect()
        .await
        .unwrap();

        let leaves = leaf_chain(0..=2, EPOCH_VERSION).await;
        db.insert_leaf(leaves[0].clone()).await.unwrap();
        db.insert_leaf(leaves[1].clone()).await.unwrap();

        // Touch leaf 0 via a read, making leaf 1 the least-recently-used.
        db.leaf_upper_bound(LeafId::Number(0)).await.unwrap();

        // Insert leaf 2; GC evicts 1 (LRU) to bring count back to 2.
        db.insert_leaf(leaves[2].clone()).await.unwrap();

        // Check exact presence/absence via hash lookups (the Number variant is an
        // upper bound, so it would return a higher leaf instead of None).
        assert_eq!(
            db.leaf_upper_bound(LeafId::Hash(leaves[0].hash()))
                .await
                .unwrap(),
            Some(leaves[0].clone()),
            "leaf 0 was recently read and must be kept"
        );
        assert_eq!(
            db.leaf_upper_bound(LeafId::Hash(leaves[1].hash()))
                .await
                .unwrap(),
            None,
            "leaf 1 was least-recently-used and must be evicted"
        );
        assert_eq!(
            db.leaf_upper_bound(LeafId::Hash(leaves[2].hash()))
                .await
                .unwrap(),
            Some(leaves[2].clone()),
            "leaf 2 was just inserted and must be kept"
        );
    }

    // Recency (last_used) must survive a process restart so GC after reopen
    // still honours access history. Specifically: `next_tick` must be seeded
    // above the persisted MAX(last_used) so that a touch recorded after reopen
    // outranks all pre-restart last_used values.
    #[tokio::test]
    #[test_log::test]
    async fn test_recency_survives_restart() {
        let dir = tempdir().unwrap();
        let lc_path = dir.path().join("lc.db");

        let opts = || LightClientSqliteOptions {
            lc_path: Some(lc_path.clone()),
            num_leaves: 2,
            ..Default::default()
        };

        let leaves = leaf_chain(0..=3, EPOCH_VERSION).await;

        {
            let db = opts().connect().await.unwrap();
            db.insert_leaf(leaves[0].clone()).await.unwrap();
            db.insert_leaf(leaves[1].clone()).await.unwrap();

            // Touch leaf 0 so it has a higher last_used than leaf 1.
            db.leaf_upper_bound(LeafId::Number(0)).await.unwrap();

            // Insert leaf 2; GC evicts leaf 1 (LRU); flushes the touch for leaf 0.
            // After GC: {leaf 0, leaf 2} remain.
            db.insert_leaf(leaves[2].clone()).await.unwrap();
        }

        // Reopen. next_tick seeds from MAX(last_used) + 1, so any post-reopen
        // tick is strictly greater than all persisted last_used values.
        {
            let db = opts().connect().await.unwrap();

            // Touch leaf 2 after reopen; its tick now exceeds all pre-restart last_used.
            db.leaf_upper_bound(LeafId::Hash(leaves[2].hash()))
                .await
                .unwrap();

            // Insert leaf 3; GC evicts the leaf with the lowest last_used (leaf 0,
            // whose pre-restart tick is lower than leaf 2's post-reopen tick).
            // After GC: {leaf 2, leaf 3} remain.
            db.insert_leaf(leaves[3].clone()).await.unwrap();

            assert_eq!(
                db.leaf_upper_bound(LeafId::Hash(leaves[0].hash()))
                    .await
                    .unwrap(),
                None,
                "leaf 0 had the lowest persisted last_used and must be evicted"
            );
            assert_eq!(
                db.leaf_upper_bound(LeafId::Hash(leaves[2].hash()))
                    .await
                    .unwrap(),
                Some(leaves[2].clone()),
                "leaf 2 was touched after reopen and must survive"
            );
            assert_eq!(
                db.leaf_upper_bound(LeafId::Hash(leaves[3].hash()))
                    .await
                    .unwrap(),
                Some(leaves[3].clone()),
                "leaf 3 was just inserted and must survive"
            );
        }
    }

    // Dropping the last clone (graceful shutdown) must persist read-path touches
    // when no insert follows the final reads, so GC after restart honours them.
    #[tokio::test]
    #[test_log::test]
    async fn test_recency_flushed_on_drop_without_insert() {
        let dir = tempdir().unwrap();
        let lc_path = dir.path().join("lc.db");

        let opts = || LightClientSqliteOptions {
            lc_path: Some(lc_path.clone()),
            num_leaves: 2,
            ..Default::default()
        };

        let leaves = leaf_chain(0..=2, EPOCH_VERSION).await;

        {
            let db = opts().connect().await.unwrap();
            db.insert_leaf(leaves[0].clone()).await.unwrap();
            db.insert_leaf(leaves[1].clone()).await.unwrap();

            // Touch leaf 0 via a read; recorded only in memory.
            db.leaf_upper_bound(LeafId::Number(0)).await.unwrap();

            // No insert follows. Dropping db at the end of this scope must flush the touch.
        }

        // Reopen and insert leaf 2; GC evicts the lowest last_used. With the touch
        // flushed, leaf 0 outranks leaf 1, so leaf 1 is evicted.
        {
            let db = opts().connect().await.unwrap();
            db.insert_leaf(leaves[2].clone()).await.unwrap();

            assert_eq!(
                db.leaf_upper_bound(LeafId::Hash(leaves[0].hash()))
                    .await
                    .unwrap(),
                Some(leaves[0].clone()),
                "leaf 0 was touched before the shutdown flush and must survive"
            );
            assert_eq!(
                db.leaf_upper_bound(LeafId::Hash(leaves[1].hash()))
                    .await
                    .unwrap(),
                None,
                "leaf 1 was least-recently-used and must be evicted"
            );
        }
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_get_leaves_in_range() {
        let db = SqliteStorage::default().await.unwrap();

        let leaves = leaf_chain(0..5, EPOCH_VERSION).await;
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

        let leaves = leaf_chain(0..3, EPOCH_VERSION).await;
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
        db.insert_stake_table(epoch, &state, EPOCH_VERSION, EPOCH_VERSION)
            .await
            .unwrap();
        assert_eq!(
            db.stake_table_lower_bound(epoch).await.unwrap().unwrap(),
            (epoch, state, EPOCH_VERSION, EPOCH_VERSION)
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_stake_table_lower_bound_loose() {
        let db = SqliteStorage::default().await.unwrap();

        let epoch = EpochNumber::new(1);
        let state = random_stake_table();
        db.insert_stake_table(epoch, &state, EPOCH_VERSION, EPOCH_VERSION)
            .await
            .unwrap();
        assert_eq!(
            db.stake_table_lower_bound(epoch + 1)
                .await
                .unwrap()
                .unwrap(),
            (epoch, state, EPOCH_VERSION, EPOCH_VERSION)
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_stake_table_lower_bound_greatest_lower_bound() {
        let db = SqliteStorage::default().await.unwrap();

        let state1 = random_stake_table();
        let state2 = chain_stake_table(&state1);
        db.insert_stake_table(EpochNumber::new(1), &state1, EPOCH_VERSION, EPOCH_VERSION)
            .await
            .unwrap();
        db.insert_stake_table(EpochNumber::new(2), &state2, EPOCH_VERSION, EPOCH_VERSION)
            .await
            .unwrap();

        assert_eq!(
            db.stake_table_lower_bound(EpochNumber::new(3))
                .await
                .unwrap()
                .unwrap(),
            (EpochNumber::new(2), state2, EPOCH_VERSION, EPOCH_VERSION)
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_stake_table_lower_bound_not_found() {
        let db = SqliteStorage::default().await.unwrap();
        db.insert_stake_table(
            EpochNumber::new(2),
            &random_stake_table(),
            EPOCH_VERSION,
            EPOCH_VERSION,
        )
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
        db.insert_stake_table(EpochNumber::new(1), &state1, EPOCH_VERSION, EPOCH_VERSION)
            .await
            .unwrap();
        db.insert_stake_table(EpochNumber::new(2), &state2, EPOCH_VERSION, EPOCH_VERSION)
            .await
            .unwrap();
        db.insert_stake_table(EpochNumber::new(3), &state3, EPOCH_VERSION, EPOCH_VERSION)
            .await
            .unwrap();

        assert_eq!(
            db.stake_table_lower_bound(EpochNumber::new(1))
                .await
                .unwrap()
                .unwrap(),
            (
                EpochNumber::new(1),
                state1.clone(),
                EPOCH_VERSION,
                EPOCH_VERSION
            )
        );
        assert_eq!(
            db.stake_table_lower_bound(EpochNumber::new(2))
                .await
                .unwrap()
                .unwrap(),
            (EpochNumber::new(1), state1, EPOCH_VERSION, EPOCH_VERSION)
        );
        assert_eq!(
            db.stake_table_lower_bound(EpochNumber::new(3))
                .await
                .unwrap()
                .unwrap(),
            (EpochNumber::new(3), state3, EPOCH_VERSION, EPOCH_VERSION)
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_stake_table_insert_out_of_order() {
        let db = SqliteStorage::default().await.unwrap();

        let state1 = random_stake_table();
        let state2 = chain_stake_table(&state1);
        db.insert_stake_table(EpochNumber::new(2), &state2, EPOCH_VERSION, EPOCH_VERSION)
            .await
            .unwrap();
        db.insert_stake_table(EpochNumber::new(1), &state1, EPOCH_VERSION, EPOCH_VERSION)
            .await
            .unwrap();

        assert_eq!(
            db.stake_table_lower_bound(EpochNumber::new(1))
                .await
                .unwrap()
                .unwrap(),
            (EpochNumber::new(1), state1, EPOCH_VERSION, EPOCH_VERSION)
        );
        assert_eq!(
            db.stake_table_lower_bound(EpochNumber::new(2))
                .await
                .unwrap()
                .unwrap(),
            (EpochNumber::new(2), state2, EPOCH_VERSION, EPOCH_VERSION)
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_stake_table_epoch_root_protocol_version_roundtrip() {
        let db = SqliteStorage::default().await.unwrap();

        let epoch = EpochNumber::new(1);
        let state = random_stake_table();
        db.insert_stake_table(epoch, &state, CLIQUENET_VERSION, EPOCH_VERSION)
            .await
            .unwrap();
        let (loaded_epoch, loaded_state, loaded_version, loaded_next_version) =
            db.stake_table_lower_bound(epoch).await.unwrap().unwrap();
        assert_eq!(loaded_epoch, epoch);
        assert_eq!(loaded_state, state);
        assert_eq!(loaded_version, CLIQUENET_VERSION);
        assert_eq!(loaded_next_version, EPOCH_VERSION);
    }

    /// Regression: storage previously dropped `used_x25519_keys` on round trip,
    /// so a reloaded `StakeTableState::commit()` diverged from the proposer's
    /// hash once any V3 events had been applied.
    #[tokio::test]
    #[test_log::test]
    async fn test_stake_table_x25519_keys_round_trip() {
        use committable::Committable;

        let db = SqliteStorage::default().await.unwrap();
        let epoch = EpochNumber::new(1);
        let state = random_stake_table();
        assert!(
            !state.used_x25519_keys().is_empty(),
            "random_stake_table must populate used_x25519_keys for this test to be meaningful"
        );

        db.insert_stake_table(epoch, &state, CLIQUENET_VERSION, CLIQUENET_VERSION)
            .await
            .unwrap();
        let (_, loaded, ..) = db.stake_table_lower_bound(epoch).await.unwrap().unwrap();

        assert_eq!(loaded.used_x25519_keys(), state.used_x25519_keys());
        assert_eq!(loaded.commit(), state.commit());
    }

    /// Make a stake table state with all fields populated.
    fn random_stake_table() -> StakeTableState {
        let validator = random_validator();
        let candidate: RegisteredValidator<PubKey> = validator.clone().into();
        let x25519_key =
            x25519::PublicKey::try_from(rand::random::<[u8; 32]>().as_slice()).unwrap();
        let candidate_bls = candidate
            .stake_table_key
            .expect("random_validator returns authenticated validator");
        StakeTableState::new(
            [(candidate.account, candidate.clone())]
                .into_iter()
                .collect(),
            [Address::random()].into_iter().collect(),
            [candidate_bls].into_iter().collect(),
            [candidate
                .state_ver_key
                .expect("random_validator has valid schnorr key")]
            .into_iter()
            .collect(),
            [x25519_key].into_iter().collect(),
        )
    }

    /// Create a new stake table state which is a possible successor to the given state.
    fn chain_stake_table(state: &StakeTableState) -> StakeTableState {
        let new_validator = random_validator();
        let new_candidate: RegisteredValidator<PubKey> = new_validator.clone().into();
        let new_candidate_bls = new_candidate
            .stake_table_key
            .expect("random_validator returns authenticated validator");
        let new_exit = Address::random();
        let new_x25519 =
            x25519::PublicKey::try_from(rand::random::<[u8; 32]>().as_slice()).unwrap();
        StakeTableState::new(
            state
                .validators()
                .values()
                .chain([&new_candidate])
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
                .chain([&new_candidate_bls])
                .cloned()
                .collect(),
            state
                .used_schnorr_keys()
                .iter()
                .chain([new_candidate
                    .state_ver_key
                    .as_ref()
                    .expect("random_validator has valid schnorr key")])
                .cloned()
                .collect(),
            state
                .used_x25519_keys()
                .iter()
                .chain([&new_x25519])
                .cloned()
                .collect(),
        )
    }
}
