//! A backend in Postgres, which several processes can share.
//!
//! Every backend `LISTEN`s, and announces its writes with a `NOTIFY`, so a write by any process
//! reaches the [`changes`](Backend::changes) of all of them. The announcement is not part of the
//! write: a transaction that notifies holds a lock of the whole server while its commit is
//! flushed, so every notifying commit on the server would wait for the others. A task sends one
//! announcement after the writes that happened meanwhile, without waiting for the flush, so a
//! reader learns of a write a moment after it is committed, never before.

use std::{
    ops::{Range, RangeInclusive},
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use bytes::Bytes;
use sqlx::{
    PgPool, Postgres, Row as _, Transaction,
    postgres::{PgListener, PgPoolOptions},
};
use tokio::{
    sync::{Notify, watch},
    task::JoinSet,
    time::sleep,
};

use crate::{AppWrite, Backend, Column, KeyBounds, Part, PayloadIndex, Row, TableName, Totals};

/// The channel writes are announced on.
const CHANNEL: &str = "chain_store";

/// How long the listener waits before listening again after its connection failed.
const RELISTEN_DELAY: Duration = Duration::from_secs(1);

/// How long the notifier gathers writes into one announcement.
const NOTIFY_DELAY: Duration = Duration::from_millis(5);

/// How long the notifier waits before announcing again after an announcement failed.
const RENOTIFY_DELAY: Duration = Duration::from_millis(500);

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS chain_items (
    height  BIGINT PRIMARY KEY,
    leaf    TEXT NOT NULL,
    block   TEXT NOT NULL,
    payload TEXT NOT NULL,
    item    BYTEA NOT NULL
);
CREATE INDEX IF NOT EXISTS chain_items_leaf ON chain_items (leaf);
CREATE INDEX IF NOT EXISTS chain_items_block ON chain_items (block);
CREATE INDEX IF NOT EXISTS chain_items_payload ON chain_items (payload, height);
CREATE TABLE IF NOT EXISTS chain_parts (
    height BIGINT NOT NULL,
    part   SMALLINT NOT NULL,
    data   BYTEA NOT NULL,
    PRIMARY KEY (height, part)
);
CREATE TABLE IF NOT EXISTS chain_transactions (
    hash     TEXT NOT NULL,
    height   BIGINT NOT NULL,
    position BIGINT NOT NULL,
    PRIMARY KEY (hash, height, position)
);
CREATE INDEX IF NOT EXISTS chain_transactions_height ON chain_transactions (height);
CREATE TABLE IF NOT EXISTS chain_totals (
    height       BIGINT NOT NULL,
    namespace    BIGINT NOT NULL,
    transactions BIGINT NOT NULL,
    bytes        BIGINT NOT NULL,
    PRIMARY KEY (namespace, height)
);
CREATE INDEX IF NOT EXISTS chain_totals_height ON chain_totals (height);
CREATE TABLE IF NOT EXISTS chain_floor (
    id     BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (id),
    height BIGINT NOT NULL
);
INSERT INTO chain_floor (height) VALUES (0) ON CONFLICT DO NOTHING;
CREATE TABLE IF NOT EXISTS chain_app (
    tbl   TEXT NOT NULL,
    key   BYTEA NOT NULL,
    value BYTEA NOT NULL,
    PRIMARY KEY (tbl, key)
);
CREATE TABLE IF NOT EXISTS chain_leases (
    name    TEXT PRIMARY KEY,
    holder  TEXT NOT NULL,
    expires TIMESTAMPTZ NOT NULL
);
";

#[derive(Debug)]
pub struct PostgresBackend {
    pool: PgPool,
    changes: watch::Sender<u64>,

    /// Woken after each write, to announce it.
    written: Arc<Notify>,

    /// The tasks that turn announcements into changes and announce writes, which end with the
    /// backend.
    _tasks: JoinSet<()>,
}

impl PostgresBackend {
    /// Connects to the database at `url`, creating the tables if they do not exist.
    pub async fn connect(url: &str) -> anyhow::Result<Self> {
        let pool = PgPoolOptions::new().connect(url).await?;
        sqlx::raw_sql(SCHEMA).execute(&pool).await?;
        let mut listener = PgListener::connect_with(&pool).await?;
        listener.listen(CHANNEL).await?;
        let changes = watch::channel(0).0;
        let written = Arc::new(Notify::new());
        let mut tasks = JoinSet::new();
        tasks.spawn(listen(listener, changes.clone()));
        tasks.spawn(notify(pool.clone(), written.clone()));
        Ok(Self {
            pool,
            changes,
            written,
            _tasks: tasks,
        })
    }

    /// Commits `tx`, a write to the chain, and has the [`notify`] task announce it.
    async fn commit(&self, tx: Transaction<'_, Postgres>) -> anyhow::Result<()> {
        tx.commit().await?;
        // The announcement reaches this backend too, later; readers here need not wait for it.
        self.changes.send_modify(|n| *n += 1);
        self.written.notify_one();
        Ok(())
    }
}

/// Turns announcements into changes. A lost connection is followed by a change, since writes
/// announced while it was down are missed.
async fn listen(mut listener: PgListener, changes: watch::Sender<u64>) {
    loop {
        // `try_recv`, unlike `recv`, reports a lost connection instead of reconnecting silently.
        match listener.try_recv().await {
            Ok(Some(_)) => changes.send_modify(|n| *n += 1),
            Ok(None) => {
                tracing::warn!(
                    "lost the connection listening for chain store writes; reconnecting"
                );
                changes.send_modify(|n| *n += 1);
            },
            Err(err) => {
                tracing::warn!("listening for chain store writes failed; listening again: {err:#}");
                // The change comes once listening again, so no write between falls through.
                while let Err(err) = listener.listen(CHANNEL).await {
                    tracing::debug!("listening again failed: {err:#}");
                    tokio::time::sleep(RELISTEN_DELAY).await;
                }
                changes.send_modify(|n| *n += 1);
            },
        }
    }
}

/// Announces the writes `written` tells of: once per [`NOTIFY_DELAY`] at most, for every write
/// before it, in a transaction of its own that does not wait for its commit to be flushed. A lost
/// announcement only delays readers, which take a lost connection for a change too.
async fn notify(pool: PgPool, written: Arc<Notify>) {
    loop {
        written.notified().await;
        sleep(NOTIFY_DELAY).await;
        while let Err(err) = announce(&pool).await {
            tracing::warn!("announcing chain store writes failed; trying again: {err:#}");
            sleep(RENOTIFY_DELAY).await;
        }
    }
}

async fn announce(pool: &PgPool) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL synchronous_commit = off")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SELECT pg_notify($1, '')")
        .bind(CHANNEL)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

/// Applies `writes` to the application tables in `tx`.
async fn apply(tx: &mut Transaction<'_, Postgres>, writes: Vec<AppWrite>) -> anyhow::Result<()> {
    for write in writes {
        match write {
            AppWrite::Put { table, key, value } => {
                sqlx::query(
                    "INSERT INTO chain_app (tbl, key, value) VALUES ($1, $2, $3) ON CONFLICT \
                     (tbl, key) DO UPDATE SET value = EXCLUDED.value",
                )
                .bind(table.as_str())
                .bind(&key[..])
                .bind(&value[..])
                .execute(&mut **tx)
                .await?;
            },
            AppWrite::DeleteBefore { table, key } => {
                sqlx::query("DELETE FROM chain_app WHERE tbl = $1 AND key < $2")
                    .bind(table.as_str())
                    .bind(&key[..])
                    .execute(&mut **tx)
                    .await?;
            },
            AppWrite::Clear { table } => {
                sqlx::query("DELETE FROM chain_app WHERE tbl = $1")
                    .bind(table.as_str())
                    .execute(&mut **tx)
                    .await?;
            },
        }
    }
    Ok(())
}

/// The floor, locked against pruning until `tx` ends.
async fn lock_floor(tx: &mut Transaction<'_, Postgres>) -> anyhow::Result<u64> {
    let height: i64 = sqlx::query_scalar("SELECT height FROM chain_floor FOR SHARE")
        .fetch_one(&mut **tx)
        .await?;
    Ok(height as u64)
}

/// The `chain_totals` namespace of a payload's totals across all its namespaces.
const ALL_NAMESPACES: i64 = -1;

/// Writes `part` of `height`. The index of a payload goes to the tables its queries read.
async fn insert_part(
    tx: &mut Transaction<'_, Postgres>,
    height: u64,
    part: Part,
    data: &[u8],
) -> anyhow::Result<()> {
    if part == Part::Index {
        return insert_index(tx, height, &crate::decode(data)?).await;
    }
    sqlx::query(
        "INSERT INTO chain_parts (height, part, data) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
    )
    .bind(height as i64)
    .bind(part.id())
    .bind(data)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Writes the rows of `index`, the index of the payload of `height`, unless the height has them.
async fn insert_index(
    tx: &mut Transaction<'_, Postgres>,
    height: u64,
    index: &PayloadIndex,
) -> anyhow::Result<()> {
    let indexed: Option<i32> =
        sqlx::query_scalar("SELECT 1 FROM chain_totals WHERE namespace = $1 AND height = $2")
            .bind(ALL_NAMESPACES)
            .bind(height as i64)
            .fetch_optional(&mut **tx)
            .await?;
    if indexed.is_some() {
        return Ok(());
    }
    let positions: Vec<i64> = (0..index.transactions.len() as i64).collect();
    sqlx::query(
        "INSERT INTO chain_transactions (hash, height, position)
         SELECT hash, $2, position FROM UNNEST($1::TEXT[], $3::BIGINT[]) AS t (hash, position)
         ON CONFLICT DO NOTHING",
    )
    .bind(&index.transactions)
    .bind(height as i64)
    .bind(&positions)
    .execute(&mut **tx)
    .await?;
    let totals = std::iter::once((ALL_NAMESPACES, index.total)).chain(
        index
            .namespaces
            .iter()
            .map(|(namespace, totals)| (i64::from(*namespace), *totals)),
    );
    let (namespaces, totals): (Vec<i64>, Vec<Totals>) = totals.unzip();
    let transactions: Vec<i64> = totals.iter().map(|t| t.transactions as i64).collect();
    let bytes: Vec<i64> = totals.iter().map(|t| t.bytes as i64).collect();
    sqlx::query(
        "INSERT INTO chain_totals (height, namespace, transactions, bytes)
         SELECT $1, namespace, transactions, bytes
           FROM UNNEST($2::BIGINT[], $3::BIGINT[], $4::BIGINT[])
             AS t (namespace, transactions, bytes)
         ON CONFLICT DO NOTHING",
    )
    .bind(height as i64)
    .bind(&namespaces)
    .bind(&transactions)
    .bind(&bytes)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

#[async_trait]
impl Backend for PostgresBackend {
    async fn put_with(
        &self,
        row: Row,
        parts: Vec<(Part, Bytes)>,
        writes: Vec<AppWrite>,
    ) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        if row.height < lock_floor(&mut tx).await? {
            return Ok(());
        }
        sqlx::query(
            "INSERT INTO chain_items (height, leaf, block, payload, item) VALUES ($1, $2, $3, $4, \
             $5) ON CONFLICT DO NOTHING",
        )
        .bind(row.height as i64)
        .bind(&row.leaf)
        .bind(&row.block)
        .bind(&row.payload)
        .bind(&row.item[..])
        .execute(&mut *tx)
        .await?;
        for (part, data) in parts {
            insert_part(&mut tx, row.height, part, &data).await?;
        }
        apply(&mut tx, writes).await?;
        self.commit(tx).await
    }

    async fn put_parts(&self, height: u64, parts: Vec<(Part, Bytes)>) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        if height < lock_floor(&mut tx).await? {
            return Ok(());
        }
        for (part, data) in parts {
            insert_part(&mut tx, height, part, &data).await?;
        }
        self.commit(tx).await
    }

    async fn item(&self, height: u64) -> anyhow::Result<Option<Bytes>> {
        let item: Option<Vec<u8>> =
            sqlx::query_scalar("SELECT item FROM chain_items WHERE height = $1")
                .bind(height as i64)
                .fetch_optional(&self.pool)
                .await?;
        Ok(item.map(Bytes::from))
    }

    async fn part(&self, height: u64, part: Part) -> anyhow::Result<Option<Bytes>> {
        let data: Option<Vec<u8>> =
            sqlx::query_scalar("SELECT data FROM chain_parts WHERE height = $1 AND part = $2")
                .bind(height as i64)
                .bind(part.id())
                .fetch_optional(&self.pool)
                .await?;
        Ok(data.map(Bytes::from))
    }

    async fn record(
        &self,
        height: u64,
        payload: bool,
    ) -> anyhow::Result<Option<(Bytes, Vec<(Part, Bytes)>)>> {
        let rows = sqlx::query(
            "SELECT i.item, p.part, p.data FROM chain_items i
             LEFT JOIN chain_parts p ON p.height = i.height AND ($2 OR p.part <> $3)
             WHERE i.height = $1 ORDER BY p.part",
        )
        .bind(height as i64)
        .bind(payload)
        .bind(Part::Payload.id())
        .fetch_all(&self.pool)
        .await?;
        let Some(first) = rows.first() else {
            return Ok(None);
        };
        let item: Vec<u8> = first.try_get("item")?;
        let mut parts = Vec::new();
        for row in &rows {
            let Some(id) = row.try_get::<Option<i16>, _>("part")? else {
                continue;
            };
            let part = Part::from_id(id).ok_or_else(|| anyhow::anyhow!("unknown part {id}"))?;
            let data: Vec<u8> = row.try_get("data")?;
            parts.push((part, data.into()));
        }
        Ok(Some((item.into(), parts)))
    }

    async fn height_of(&self, column: Column, value: &str) -> anyhow::Result<Option<u64>> {
        let column = match column {
            Column::Leaf => "leaf",
            Column::Block => "block",
            Column::Payload => "payload",
        };
        let height: Option<i64> = sqlx::query_scalar(&format!(
            "SELECT height FROM chain_items WHERE {column} = $1 ORDER BY height LIMIT 1"
        ))
        .bind(value)
        .fetch_optional(&self.pool)
        .await?;
        Ok(height.map(|h| h as u64))
    }

    async fn newest(&self) -> anyhow::Result<Option<u64>> {
        let height: Option<i64> = sqlx::query_scalar("SELECT max(height) FROM chain_items")
            .fetch_one(&self.pool)
            .await?;
        Ok(height.map(|h| h as u64))
    }

    async fn floor(&self) -> anyhow::Result<u64> {
        let height: i64 = sqlx::query_scalar("SELECT height FROM chain_floor")
            .fetch_one(&self.pool)
            .await?;
        Ok(height as u64)
    }

    async fn missing_items(&self, from: u64, limit: usize) -> anyhow::Result<Vec<Range<u64>>> {
        // The row before the first kept height stands in for it, so a gap at the start counts.
        let rows = sqlx::query(
            "WITH lo AS (SELECT GREATEST($1::BIGINT, height) AS lo FROM chain_floor),
                  h AS (SELECT height FROM chain_items, lo WHERE height >= lo.lo
                        UNION ALL SELECT lo.lo - 1 FROM lo)
             SELECT height + 1 AS start, next AS end
             FROM (SELECT height, lead(height) OVER (ORDER BY height) AS next FROM h) t
             WHERE next > height + 1
             ORDER BY height
             LIMIT $2",
        )
        .bind(from as i64)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                let start: i64 = row.try_get("start")?;
                let end: i64 = row.try_get("end")?;
                Ok(start as u64..end as u64)
            })
            .collect()
    }

    /// Walks the heights down from the newest, so a gap near the top is found at once.
    async fn newest_gap(&self) -> anyhow::Result<Option<Range<u64>>> {
        let row = sqlx::query(
            "WITH f AS (SELECT height FROM chain_floor),
                  e AS (SELECT i.height FROM chain_items i, f
                        WHERE i.height > f.height
                          AND NOT EXISTS (SELECT 1 FROM chain_items p WHERE p.height = i.height - \
             1)
                        ORDER BY i.height DESC LIMIT 1)
             SELECT COALESCE((SELECT max(i.height) + 1 FROM chain_items i, e
                              WHERE i.height < e.height), (SELECT height FROM f)) AS start,
                    e.height AS end
             FROM e",
        )
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| {
            let start: i64 = row.try_get("start")?;
            let end: i64 = row.try_get("end")?;
            Ok(start.max(0) as u64..end as u64)
        })
        .transpose()
    }

    async fn missing_parts(&self, part: Part, from: u64, limit: usize) -> anyhow::Result<Vec<u64>> {
        let heights: Vec<i64> = sqlx::query_scalar(
            "SELECT i.height FROM chain_items i
             LEFT JOIN chain_parts p ON p.height = i.height AND p.part = $2
             WHERE i.height >= GREATEST($1::BIGINT, (SELECT height FROM chain_floor))
               AND p.height IS NULL
             ORDER BY i.height
             LIMIT $3",
        )
        .bind(from as i64)
        .bind(part.id())
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;
        Ok(heights.into_iter().map(|h| h as u64).collect())
    }

    async fn transaction_position(&self, hash: &str) -> anyhow::Result<Option<(u64, u64)>> {
        let position: Option<(i64, i64)> = sqlx::query_as(
            "SELECT height, position FROM chain_transactions
             WHERE hash = $1 ORDER BY height, position LIMIT 1",
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await?;
        Ok(position.map(|(height, position)| (height as u64, position as u64)))
    }

    async fn totals(
        &self,
        heights: RangeInclusive<u64>,
        namespace: Option<u32>,
    ) -> anyhow::Result<Totals> {
        let (transactions, bytes): (i64, i64) = sqlx::query_as(
            "SELECT COALESCE(SUM(transactions), 0)::BIGINT, COALESCE(SUM(bytes), 0)::BIGINT
               FROM chain_totals
              WHERE namespace = $1 AND height >= $2 AND height <= $3",
        )
        .bind(namespace.map_or(ALL_NAMESPACES, i64::from))
        .bind(i64::try_from(*heights.start()).unwrap_or(i64::MAX))
        .bind(i64::try_from(*heights.end()).unwrap_or(i64::MAX))
        .fetch_one(&self.pool)
        .await?;
        Ok(Totals {
            transactions: transactions as u64,
            bytes: bytes as u64,
        })
    }

    async fn app_get(&self, table: &TableName, key: &[u8]) -> anyhow::Result<Option<Bytes>> {
        let value: Option<Vec<u8>> =
            sqlx::query_scalar("SELECT value FROM chain_app WHERE tbl = $1 AND key = $2")
                .bind(table.as_str())
                .bind(key)
                .fetch_optional(&self.pool)
                .await?;
        Ok(value.map(Bytes::from))
    }

    /// Postgres orders `bytea` bytewise, as the other backends order keys.
    async fn app_entries(
        &self,
        table: &TableName,
        keys: KeyBounds<'_>,
    ) -> anyhow::Result<Vec<(Bytes, Bytes)>> {
        let rows = sqlx::query(
            "SELECT key, value FROM chain_app
             WHERE tbl = $1 AND key >= $2 AND ($3::BYTEA IS NULL OR key < $3)
             ORDER BY key LIMIT $4",
        )
        .bind(table.as_str())
        .bind(keys.from)
        .bind(keys.until)
        .bind(i64::try_from(keys.limit).unwrap_or(i64::MAX))
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                let key: Vec<u8> = row.try_get("key")?;
                let value: Vec<u8> = row.try_get("value")?;
                Ok((key.into(), value.into()))
            })
            .collect()
    }

    async fn app_write(&self, writes: Vec<AppWrite>) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        apply(&mut tx, writes).await?;
        // Not a write to the chain, so not announced.
        tx.commit().await?;
        Ok(())
    }

    async fn prune_below(&self, height: u64) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        let floor: i64 = sqlx::query_scalar(
            "UPDATE chain_floor SET height = GREATEST(height, $1) RETURNING height",
        )
        .bind(height as i64)
        .fetch_one(&mut *tx)
        .await?;
        for table in [
            "chain_items",
            "chain_parts",
            "chain_transactions",
            "chain_totals",
        ] {
            sqlx::query(&format!("DELETE FROM {table} WHERE height < $1"))
                .bind(floor)
                .execute(&mut *tx)
                .await?;
        }
        self.commit(tx).await
    }

    async fn lease(&self, name: &str, holder: &str, ttl: Duration) -> anyhow::Result<bool> {
        // Times are the database's, so the holders' clocks do not matter. Leases whose time ran
        // out long ago go, so leases on passing work do not pile up.
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM chain_leases WHERE expires < now() - interval '1 hour'")
            .execute(&mut *tx)
            .await?;
        let held: Option<String> = sqlx::query_scalar(
            "INSERT INTO chain_leases (name, holder, expires) VALUES ($1, $2, now() + $3 * \
             interval '1 millisecond') ON CONFLICT (name) DO UPDATE SET holder = EXCLUDED.holder, \
             expires = EXCLUDED.expires WHERE chain_leases.holder = EXCLUDED.holder OR \
             chain_leases.expires < now() RETURNING holder",
        )
        .bind(name)
        .bind(holder)
        .bind(i64::try_from(ttl.as_millis()).unwrap_or(i64::MAX))
        .fetch_optional(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(held.is_some())
    }

    fn changes(&self) -> watch::Receiver<u64> {
        self.changes.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::testing::{TmpPostgres, row};

    #[tokio::test]
    async fn conforms() {
        let db = TmpPostgres::init().await;
        let backend = PostgresBackend::connect(&db.url()).await.unwrap();
        crate::testing::conformance(&backend).await;
    }

    /// A write by one process reaches the readers of another.
    #[tokio::test]
    async fn announces_writes_to_other_backends() {
        let db = TmpPostgres::init().await;
        let writer = PostgresBackend::connect(&db.url()).await.unwrap();
        let reader = PostgresBackend::connect(&db.url()).await.unwrap();
        let mut changes = reader.changes();
        changes.borrow_and_update();

        writer.put(row(0, "x"), Vec::new()).await.unwrap();
        tokio::time::timeout(Duration::from_secs(10), changes.changed())
            .await
            .expect("the reader hears of the write")
            .unwrap();
        assert!(reader.item(0).await.unwrap().is_some());
    }

    /// Writes announced while the listening connection is down are missed, so a lost connection
    /// counts as a change.
    #[tokio::test]
    async fn a_lost_listener_connection_is_a_change() {
        let db = TmpPostgres::init().await;
        let backend = PostgresBackend::connect(&db.url()).await.unwrap();
        let mut changes = backend.changes();
        changes.borrow_and_update();

        sqlx::query(
            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity
             WHERE query LIKE 'LISTEN%' AND pid <> pg_backend_pid()",
        )
        .execute(&backend.pool)
        .await
        .unwrap();
        tokio::time::timeout(Duration::from_secs(10), changes.changed())
            .await
            .expect("the lost connection is a change")
            .unwrap();
    }
}
