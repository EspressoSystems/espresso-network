use std::{collections::BTreeMap, future::Future, sync::Arc, time::Instant};

use alloy::primitives::Address;
use anyhow::{Context, bail};
use async_trait::async_trait;
use committable::Committable;
use derive_more::{Deref, DerefMut};
use espresso_types::{
    AuthenticatedValidatorMap, BackoffParams, Header, Leaf, Leaf2, NetworkConfig, Payload, PubKey,
    RegisteredValidatorMap, StakeTableHash,
    traits::{EventsPersistenceRead, MembershipPersistence, StakeTuple},
    v0::traits::{
        EventConsumer, PersistenceOptions, SequencerPersistence, StateCatchup, StateCertPersistence,
    },
    v0_3::{
        AuthenticatedValidator, EventKey, IndexedStake, RegisteredValidator, RewardAmount,
        StakeTableEvent,
    },
};
use futures::stream::StreamExt;
use hotshot_new_protocol::message::Certificate2;
use hotshot_query_service::{
    data_source::{
        Transaction as _, VersionedDataSource,
        storage::{SerializableRetry, pruning::PrunerCfg, sql::StorageConnectionType},
    },
    fetching::{
        Provider,
        request::{PayloadRequest, VidCommonRequest},
    },
    serializable_retry,
};
use hotshot_types::{
    data::{
        DaProposal, DaProposal2, EpochNumber, QuorumProposal, QuorumProposalWrapper,
        QuorumProposalWrapperLegacy, VidCommitment, VidCommon, VidDisperseShare,
    },
    drb::{DrbInput, DrbResult},
    event::{HotShotAction, LeafInfo},
    initializer::InitializerEpochInfo,
    message::{Proposal, convert_proposal},
    new_protocol::CoordinatorEvent,
    simple_certificate::{
        CertificatePair, LightClientStateUpdateCertificateV1, LightClientStateUpdateCertificateV2,
        NextEpochQuorumCertificate2, QuorumCertificate2, UpgradeCertificate,
    },
    traits::{
        block_contents::{BlockHeader, BlockPayload},
        metrics::Metrics,
    },
    vote::HasViewNumber,
};
use indexmap::IndexMap;
use itertools::Itertools;
use light_client::storage::LightClientDbOptions;
use sqlx::{Executor, QueryBuilder, Row, query};

use super::{
    super::{ConsensusPruningOptions, Options as SqlOptions, within_gap_fill_horizon},
    qs::{
        Backend as _, Config, Db, DbKind, Read, SerializableRetryConfig, SqlStorage, Transaction,
        Write, query_as,
    },
};
use crate::{
    NodeType, RECENT_STAKE_TABLES_LIMIT, SeqTypes, ViewNumber,
    api::options::Query,
    catchup::SqlStateCatchup,
    persistence::{
        migrate_network_config, persistence_metrics::PersistenceMetricsValue,
        storage_probe::StorageProbe,
    },
};

/// Node setup that differs between the SQL backends.
pub(crate) trait BackendOptions {
    fn apply(cfg: Config, opt: &SqlOptions) -> anyhow::Result<Config>;

    fn light_client_db(cfg: &Config, query: &Query) -> LightClientDbOptions;

    async fn reset_light_client(cfg: &Config) -> anyhow::Result<()>;

    /// A probe of the database file and its settings, for a backend that keeps the database in one.
    async fn storage_probe(
        db: &SqlStorage,
        opt: &SqlOptions,
    ) -> anyhow::Result<Option<StorageProbe>>;
}

impl TryFrom<&SqlOptions> for Config {
    type Error = anyhow::Error;

    fn try_from(opt: &SqlOptions) -> Result<Self, Self::Error> {
        let mut cfg = match &opt.uri {
            Some(uri) => uri.parse()?,
            None => Self::default(),
        };

        if let Some(pool) = opt.pool.clone() {
            let pool = sqlx::Pool::<Db>::try_from(pool)
                .ok()
                .context("the connection pool is for the other SQL backend")?;
            cfg = cfg.pool(pool);
        }

        cfg = cfg.max_connections(opt.max_connections);
        cfg = cfg.idle_connection_timeout(opt.idle_connection_timeout);
        cfg = cfg.min_connections(opt.min_connections);
        cfg = cfg.connection_timeout(opt.connection_timeout);
        cfg = cfg.slow_statement_threshold(opt.slow_statement_threshold);
        cfg = cfg.statement_timeout(opt.statement_timeout);
        cfg = Db::apply(cfg, opt)?;

        if opt.prune {
            cfg = cfg.pruner_cfg(PrunerCfg::from(opt.pruning))?;
        }
        if opt.archive {
            cfg = cfg.archive();
        }

        let retry = &opt.serializable_retry;
        cfg = cfg.serializable_retry(SerializableRetryConfig::new(
            retry.backoff_base,
            retry.backoff_max,
            retry.backoff_factor,
            retry.backoff_jitter.into(),
            retry.retry_max,
            retry.pg_stat_diag,
        ));

        Ok(cfg)
    }
}

/// [`SqlOptions`] for persistence on this backend, built by
/// [`SqlOptions::into_backend`](super::super::Options::into_backend) or checked with `try_from`.
#[derive(Clone, Debug, Deref, DerefMut)]
pub struct Options(pub(in crate::persistence::sql) SqlOptions);

impl Options {
    pub fn into_inner(self) -> SqlOptions {
        self.0
    }
}

impl TryFrom<SqlOptions> for Options {
    type Error = anyhow::Error;

    fn try_from(opt: SqlOptions) -> anyhow::Result<Options> {
        anyhow::ensure!(
            opt.embedded_db == (Db::KIND == DbKind::Sqlite),
            "the SQL options select the other backend"
        );
        Ok(Options(opt))
    }
}

#[async_trait]
impl PersistenceOptions for Options {
    type Persistence = Persistence;

    fn set_view_retention(&mut self, view_retention: u64) {
        self.0.consensus_pruning.target_retention = view_retention;
        self.0.consensus_pruning.minimum_retention = view_retention;
        self.0.consensus_pruning.vid_share_retention = view_retention;
    }

    fn set_consensus_only(&mut self) {
        self.0.consensus_only = true;
    }

    async fn create(&mut self) -> anyhow::Result<Self::Persistence> {
        let config = Config::try_from(&self.0)?;
        let db = SqlStorage::connect(config, StorageConnectionType::Sequencer).await?;

        let probe = Db::storage_probe(&db, &self.0).await?;

        let persistence = Persistence {
            db,
            gc_opt: self.0.consensus_pruning,
            last_usage_check: Arc::default(),
            consensus_only: self.0.consensus_only,
            internal_metrics: PersistenceMetricsValue::default(),
            probe,
        };
        persistence.migrate_quorum_proposal_leaf_hashes().await?;
        self.0.pool = Some(persistence.db.pool().into());

        Ok(persistence)
    }

    async fn reset(self) -> anyhow::Result<()> {
        let config = Config::try_from(&self.0)?;
        Db::reset_light_client(&config).await?;
        SqlStorage::connect(config.reset_schema(), StorageConnectionType::Sequencer).await?;
        Ok(())
    }
}

/// SQL-backed persistence.
#[derive(Clone, Debug)]
pub struct Persistence {
    pub(super) db: SqlStorage,
    gc_opt: ConsensusPruningOptions,
    /// When consensus storage usage was last measured, shared by clones.
    last_usage_check: Arc<parking_lot::Mutex<Option<Instant>>>,
    consensus_only: bool,
    /// A reference to the internal metrics
    internal_metrics: PersistenceMetricsValue,
    /// Startup findings about the filesystem and SQLite pragmas backing `db`, on SQLite only.
    pub(super) probe: Option<StorageProbe>,
}

/// PostgreSQL error code for serialization failures under SERIALIZABLE isolation.
/// Transactions that fail with this code are safe to retry from scratch.
const PG_SERIALIZATION_FAILURE_CODE: &str = "40001";

#[derive(Debug)]
struct DecidedLeaf {
    info: LeafInfo<SeqTypes>,
    cert: CertificatePair<SeqTypes>,
}

/// The decide for `chain`, newest leaf first; `cert2`, if any, finalizes the newest leaf only.
fn decide_event_from_chain(
    chain: Vec<DecidedLeaf>,
    cert2: Option<Certificate2<SeqTypes>>,
) -> Option<CoordinatorEvent<SeqTypes>> {
    let Some(newest) = chain.first() else {
        if cert2.is_some() {
            tracing::warn!(
                "decide_event_from_chain called with cert2 but no leaves; cert2 will be dropped"
            );
        }
        return None;
    };
    let cert1 = newest.cert.qc().clone();
    let leaf_infos = chain.into_iter().map(|leaf| leaf.info).collect();
    Some(CoordinatorEvent::NewDecide {
        leaf_infos,
        cert1,
        cert2,
    })
}

impl Persistence {
    /// Run `f` under the database's serialization-conflict retry policy.
    async fn serializable_retry<F, Fut, T>(&self, op: &'static str, f: F) -> anyhow::Result<T>
    where
        T: Send,
        F: Fn() -> Fut + Send + Sync,
        Fut: Future<Output = anyhow::Result<T>> + Send,
    {
        self.db.serializable_retry(op, f).await
    }

    /// Ensure the `leaf_hash` column is populated for all existing quorum proposals.
    ///
    /// This column was added in a migration, but because it requires computing a commitment of the
    /// existing data, it is not easy to populate in the SQL migration itself. Thus, on startup, we
    /// check if there are any just-migrated quorum proposals with a `NULL` value for this column,
    /// and if so we populate the column manually.
    async fn migrate_quorum_proposal_leaf_hashes(&self) -> anyhow::Result<()> {
        serializable_retry!(self, || async {
            let mut tx = self.db.write().await?;

            let mut proposals = tx.fetch("SELECT * FROM quorum_proposals");

            let mut updates = vec![];
            while let Some(row) = proposals.next().await {
                let row = row?;

                let hash: Option<String> = row.try_get("leaf_hash")?;
                if hash.is_none() {
                    let view: i64 = row.try_get("view")?;
                    let data: Vec<u8> = row.try_get("data")?;
                    let proposal: Proposal<SeqTypes, QuorumProposal<SeqTypes>> =
                        bincode::deserialize(&data)?;
                    let leaf = Leaf::from_quorum_proposal(&proposal.data);
                    let leaf_hash = Committable::commit(&leaf);
                    tracing::info!(view, %leaf_hash, "populating quorum proposal leaf hash");
                    updates.push((view, leaf_hash.to_string()));
                }
            }
            drop(proposals);

            tx.upsert("quorum_proposals", ["view", "leaf_hash"], ["view"], updates)
                .await?;

            tx.commit().await
        })
        .await
    }

    /// The `last_processed_view` cursor: highest view with a generated decide event, or `None`.
    async fn load_processed_view(&self) -> anyhow::Result<Option<ViewNumber>> {
        Ok(self
            .db
            .read()
            .await?
            .fetch_optional("SELECT last_processed_view FROM event_stream WHERE id = 1 LIMIT 1")
            .await?
            .map(|row| ViewNumber::new(row.get::<i64, _>("last_processed_view") as u64)))
    }

    async fn generate_decide_events(&self, consumer: &impl EventConsumer) -> anyhow::Result<()> {
        let mut last_processed_view: Option<i64> = serializable_retry!(self, || async {
            Ok(self
                .db
                .read()
                .await?
                .fetch_optional("SELECT last_processed_view FROM event_stream WHERE id = 1 LIMIT 1")
                .await?
                .map(|row| row.get("last_processed_view")))
        })
        .await?;
        // Seed the height-continuity check below from the leaf at the cursor; if retention
        // pruning has since removed it, fall back to accepting the first row as before.
        let mut last_processed_height: Option<u64> = match last_processed_view {
            Some(view) => {
                serializable_retry!(self, || async {
                    self.db
                        .read()
                        .await?
                        .fetch_optional(
                            query("SELECT leaf FROM anchor_leaf2 WHERE view = $1").bind(view),
                        )
                        .await?
                        .map(|row| -> anyhow::Result<u64> {
                            let leaf_data: Vec<u8> = row.get("leaf");
                            let leaf = bincode::deserialize::<Leaf2>(&leaf_data)?;
                            Ok(leaf.block_header().block_number())
                        })
                        .transpose()
                })
                .await?
            },
            None => None,
        };
        loop {
            // In SQLite, overlapping read and write transactions can lead to database errors. To
            // avoid this:
            // - start a read transaction to query and collect all the necessary data.
            // - Commit (or implicitly drop) the read transaction once the data is fetched.
            // - use the collected data to generate a "decide" event for the consumer.
            // - begin a write transaction to delete the data and update the event stream.

            // Retry the entire read section on serialization failures (40001) via
            // `serializable_retry!`. The closure returns `None` when there is no
            // more work to do, which we propagate out of the outer function.
            let Some((
                from_view,
                to_view,
                leaves,
                final_qc,
                mut vid_shares,
                mut da_proposals,
                state_certs,
                cert2,
            )) = serializable_retry!(self, || async {
                let mut tx = self.db.read().await?;

                // Collect a chain of consecutive leaves, starting from the first view after the
                // last decide. This will correspond to a decide event, and defines a range of
                // views which can be garbage collected. This may even include views for which
                // there was no leaf, for which we might still have artifacts like proposals that
                // never finalized.
                let from_view = match last_processed_view {
                    Some(v) => v + 1,
                    None => 0,
                };
                tracing::debug!(?from_view, "generate decide event");

                // The newest persisted decide; bounds how far back a gap-fill can arrive.
                let (watermark,): (Option<i64>,) = query_as("SELECT max(view) FROM anchor_leaf2")
                    .fetch_one(tx.as_mut())
                    .await?;

                let mut parent = last_processed_height;
                let mut rows = query(
                    "SELECT leaf, qc, next_epoch_qc FROM anchor_leaf2 WHERE view >= $1 ORDER BY \
                     view",
                )
                .bind(from_view)
                .fetch(tx.as_mut());
                let mut leaves: Vec<(Leaf2, CertificatePair<SeqTypes>)> = vec![];
                let mut final_qc = None;
                while let Some(row) = rows.next().await {
                    let row = match row {
                        Ok(row) => row,
                        Err(err) => {
                            if err.as_database_error().is_some_and(|e| {
                                e.code().as_deref() == Some(PG_SERIALIZATION_FAILURE_CODE)
                            }) {
                                drop(rows);
                                return Err(anyhow::Error::from(err));
                            }
                            // If there's an error getting a row, try generating an event with
                            // the rows we do have.
                            tracing::warn!("error loading row: {err:#}");
                            break;
                        },
                    };

                    let leaf_data: Vec<u8> = row.get("leaf");
                    let leaf = bincode::deserialize::<Leaf2>(&leaf_data)?;
                    let qc_data: Vec<u8> = row.get("qc");
                    let qc = bincode::deserialize::<QuorumCertificate2<SeqTypes>>(&qc_data)?;
                    let next_epoch_qc = match row.get::<Option<Vec<u8>>, _>("next_epoch_qc") {
                        Some(bytes) => Some(bincode::deserialize::<
                            NextEpochQuorumCertificate2<SeqTypes>,
                        >(&bytes)?),
                        None => None,
                    };
                    let height = leaf.block_header().block_number();

                    // Ensure we are only dealing with a consecutive chain of leaves. We don't want to
                    // garbage collect any views for which we missed a leaf or decide event; at least
                    // not right away, in case we need to recover that data later.
                    if let Some(parent) = parent
                        && height != parent + 1
                    {
                        if !leaves.is_empty() {
                            tracing::debug!(
                                height,
                                parent,
                                "ending decide event at non-consecutive leaf"
                            );
                            break;
                        }
                        // A height jump within `DECIDE_GAP_FILL_HORIZON` of the watermark
                        // can still be gap-filled: hold the cursor (emit nothing) until that
                        // decide upserts the missing row. Legacy or beyond-horizon gaps are
                        // permanent; skip as before, leaving the missing block to the
                        // consumer's own fetching.
                        let row_view = leaf.view_number().u64();
                        if height > parent + 1
                            && leaf.block_header().version() >= versions::NEW_PROTOCOL_VERSION
                            && watermark
                                .is_some_and(|w| within_gap_fill_horizon(row_view, w as u64))
                        {
                            tracing::info!(
                                height,
                                parent,
                                row_view,
                                ?watermark,
                                "waiting for gap-fill decide before advancing the decide cursor"
                            );
                            break;
                        }
                        // A first row at or below the cursor (a replayed decide) is accepted.
                    }
                    parent = Some(height);
                    let cert = CertificatePair::new(qc, next_epoch_qc);
                    final_qc = Some(cert.clone());
                    leaves.push((leaf, cert));
                }
                drop(rows);

                let Some(final_qc) = final_qc else {
                    // End event processing when there are no more decided views.
                    tracing::debug!(from_view, "no new leaves at decide");
                    return Ok(None);
                };

                // Find the range of views encompassed by this leaf chain. All data in this range can be
                // processed by the consumer and then deleted.
                let from_view = leaves[0].0.view_number();
                let to_view = leaves[leaves.len() - 1].0.view_number();

                // Collect VID shares for the decide event.
                let vid_rows = tx
                    .fetch_all(
                        query("SELECT view, data FROM vid_share2 where view >= $1 AND view <= $2")
                            .bind(from_view.u64() as i64)
                            .bind(to_view.u64() as i64),
                    )
                    .await?;
                let vid_shares = vid_rows
                    .into_iter()
                    .map(|row| {
                        let view: i64 = row.get("view");
                        let data: Vec<u8> = row.get("data");
                        let vid_proposal = bincode::deserialize::<
                            Proposal<SeqTypes, VidDisperseShare<SeqTypes>>,
                        >(&data)?;
                        Ok((view as u64, vid_proposal))
                    })
                    .collect::<anyhow::Result<BTreeMap<_, _>>>()?;

                // Collect DA proposals for the decide event.
                let da_rows = tx
                    .fetch_all(
                        query(
                            "SELECT view, data FROM da_proposal2 where view >= $1 AND view <= $2",
                        )
                        .bind(from_view.u64() as i64)
                        .bind(to_view.u64() as i64),
                    )
                    .await?;
                let da_proposals = da_rows
                    .into_iter()
                    .map(|row| {
                        let view: i64 = row.get("view");
                        let data: Vec<u8> = row.get("data");
                        let da_proposal = bincode::deserialize::<
                            Proposal<SeqTypes, DaProposal2<SeqTypes>>,
                        >(&data)?;
                        Ok((view as u64, da_proposal.data))
                    })
                    .collect::<anyhow::Result<BTreeMap<_, _>>>()?;

                // Collect state certs for the decide event.
                let state_certs = Self::load_state_certs(&mut tx, from_view, to_view)
                    .await
                    .with_context(|| {
                        format!("load_state_certs from_view={from_view:?} to_view={to_view:?}")
                    })?;

                let cert2_row = tx
                    .fetch_optional(
                        query("SELECT data FROM decided_cert2 WHERE view = $1")
                            .bind(to_view.u64() as i64),
                    )
                    .await?;
                let cert2 = cert2_row
                    .map(|row| {
                        let bytes: Vec<u8> = row.get("data");
                        bincode::deserialize::<Certificate2<SeqTypes>>(&bytes)
                            .context("deserializing decided cert2")
                    })
                    .transpose()?;
                drop(tx);
                Ok(Some((
                    from_view,
                    to_view,
                    leaves,
                    final_qc,
                    vid_shares,
                    da_proposals,
                    state_certs,
                    cert2,
                )))
            })
            .await?
            else {
                return Ok(());
            };

            let to_height = leaves
                .last()
                .map(|(leaf, _)| leaf.block_header().block_number());

            // Collate all the information by view number and construct a chain of leaves.
            let chain = leaves
                .into_iter()
                // Go in reverse chronological order, as expected by Decide events.
                .rev()
                .map(|(mut leaf, cert)| {
                    let view = leaf.view_number();

                    // Include the VID share if available.
                    let vid_proposal = vid_shares.remove(&view);
                    if vid_proposal.is_none() {
                        tracing::debug!(?view, "VID share not available at decide");
                    }
                    let vid_share = vid_proposal.as_ref().map(|proposal| proposal.data.clone());

                    // Fill in the full block payload using the DA proposals we had persisted.
                    if let Some(proposal) = da_proposals.remove(&view) {
                        let payload =
                            Payload::from_bytes(&proposal.encoded_transactions, &proposal.metadata);
                        leaf.fill_block_payload_unchecked(payload);
                    } else if view == ViewNumber::genesis() {
                        // We don't get a DA proposal for the genesis view, but we know what the
                        // payload always is.
                        leaf.fill_block_payload_unchecked(Payload::empty().0);
                    } else {
                        tracing::debug!(?view, "DA proposal not available at decide");
                    }

                    let state_cert = state_certs.get(&view).cloned();

                    let info = LeafInfo {
                        leaf,
                        vid_share,
                        state_cert,
                        // Note: the following fields are not used in Decide event processing,
                        // and should be removed. For now, we just default them.
                        state: Default::default(),
                        delta: Default::default(),
                    };
                    DecidedLeaf { info, cert }
                })
                .collect();

            tracing::debug!(
                ?from_view,
                ?to_view,
                ?final_qc,
                ?chain,
                "generating decide event"
            );

            if let Some(event) = decide_event_from_chain(chain, cert2) {
                consumer.handle_event(&event).await?;
            }

            let from_view_i64 = from_view.u64() as i64;
            let to_view_i64 = to_view.u64() as i64;
            let serialized_state_certs = state_certs
                .into_iter()
                .map(|(epoch, cert)| Ok((epoch as i64, bincode::serialize(&cert)?)))
                .collect::<anyhow::Result<Vec<(i64, Vec<u8>)>>>()?;

            // Now that we have definitely processed leaves up to `to_view`, we can update
            // `last_processed_view` so we don't process these leaves again. We may still fail at
            // this point, or shut down, and fail to complete this update. At worst this will lead
            // to us sending a duplicate decide event the next time we are called; this is fine as
            // the event consumer is required to be idempotent.
            serializable_retry!(self, || async {
                let mut tx = self.db.write().await?;
                tx.upsert(
                    "event_stream",
                    ["id", "last_processed_view"],
                    ["id"],
                    [(1i32, to_view_i64)],
                )
                .await?;

                // Store all the finalized state certs
                for (epoch, state_cert_bytes) in &serialized_state_certs {
                    tx.upsert(
                        "finalized_state_cert",
                        ["epoch", "state_cert"],
                        ["epoch"],
                        [(*epoch, state_cert_bytes.clone())],
                    )
                    .await?;
                }

                // Delete the data that has been fully processed.
                tx.execute(
                    query("DELETE FROM da_proposal2 where view >= $1 AND view <= $2")
                        .bind(from_view_i64)
                        .bind(to_view_i64),
                )
                .await?;
                tx.execute(
                    query("DELETE FROM quorum_proposals2 where view >= $1 AND view <= $2")
                        .bind(from_view_i64)
                        .bind(to_view_i64),
                )
                .await?;
                tx.execute(
                    query("DELETE FROM quorum_certificate2 where view >= $1 AND view <= $2")
                        .bind(from_view_i64)
                        .bind(to_view_i64),
                )
                .await?;
                tx.execute(
                    query("DELETE FROM state_cert where view >= $1 AND view <= $2")
                        .bind(from_view_i64)
                        .bind(to_view_i64),
                )
                .await?;
                tx.execute(
                    query("DELETE FROM decided_cert2 where view >= $1 AND view < $2")
                        .bind(from_view_i64)
                        .bind(to_view_i64),
                )
                .await?;

                // Clean up leaves, but do not delete the most recent one (all leaves with a view
                // number less than the given value). This is necessary to ensure that, in case of
                // a restart, we can resume from the last decided leaf.
                tx.execute(
                    query("DELETE FROM anchor_leaf2 WHERE view >= $1 AND view < $2")
                        .bind(from_view_i64)
                        .bind(to_view_i64),
                )
                .await?;

                tx.commit().await?;
                Ok(())
            })
            .await?;
            last_processed_view = Some(to_view_i64);
            last_processed_height = to_height;
        }
    }

    async fn skip_decide_events(&self, view: ViewNumber) -> anyhow::Result<()> {
        let processed = self.load_processed_view().await?;
        // A gap-fill decide reports the older view it filled. Writing that as the cursor would
        // rewind it, and a later restart with the query module would resume from pruned views.
        if processed.is_some_and(|processed| processed >= view) {
            return Ok(());
        }
        let from_view = processed.map_or(ViewNumber::genesis(), |processed| processed + 1);
        let state_certs = serializable_retry!(self, || async {
            let mut tx = self.db.read().await?;
            Self::load_state_certs(&mut tx, from_view, view).await
        })
        .await?;

        serializable_retry!(self, || async {
            let mut tx = self.db.write().await?;
            tx.upsert(
                "event_stream",
                ["id", "last_processed_view"],
                ["id"],
                [(1i32, view.u64() as i64)],
            )
            .await?;
            for (epoch, cert) in &state_certs {
                tx.upsert(
                    "finalized_state_cert",
                    ["epoch", "state_cert"],
                    ["epoch"],
                    [(*epoch as i64, bincode::serialize(cert)?)],
                )
                .await?;
            }
            prune_to_view(&mut tx, view.u64()).await?;
            prune_vid_shares(
                &mut tx,
                view.u64().saturating_sub(self.gc_opt.vid_share_retention),
            )
            .await?;
            tx.commit().await
        })
        .await
    }

    async fn load_state_certs(
        tx: &mut Transaction<Read>,
        from_view: ViewNumber,
        to_view: ViewNumber,
    ) -> anyhow::Result<BTreeMap<u64, LightClientStateUpdateCertificateV2<SeqTypes>>> {
        let rows = tx
            .fetch_all(
                query("SELECT view, state_cert FROM state_cert WHERE view >= $1 AND view <= $2")
                    .bind(from_view.u64() as i64)
                    .bind(to_view.u64() as i64),
            )
            .await?;

        let mut result = BTreeMap::new();

        for row in rows {
            let data: Vec<u8> = row.get("state_cert");

            let cert: LightClientStateUpdateCertificateV2<SeqTypes> = bincode::deserialize(&data)
                .or_else(|err_v2| {
                bincode::deserialize::<LightClientStateUpdateCertificateV1<SeqTypes>>(&data)
                    .map(Into::into)
                    .context(format!(
                        "Failed to deserialize LightClientStateUpdateCertificate: with v1 and v2. \
                         error: {err_v2}"
                    ))
            })?;

            result.insert(cert.epoch.u64(), cert);
        }

        Ok(result)
    }

    /// Prune everything older than the target retention, then, at most once per usage check
    /// interval, prune toward the minimum retention if storage is over the target usage.
    #[tracing::instrument(skip(self))]
    async fn prune_to_retention(&self, cur_view: ViewNumber) -> anyhow::Result<()> {
        let cur_view = cur_view.u64();
        let vid_share_view = cur_view.saturating_sub(self.gc_opt.vid_share_retention);
        self.prune_below(
            cur_view.saturating_sub(self.gc_opt.target_retention),
            vid_share_view,
        )
        .await?;

        if !self.usage_check_due() {
            return Ok(());
        }
        let usage = self.consensus_storage_usage().await?;
        tracing::debug!(usage, "consensus storage usage after pruning");
        if usage > self.gc_opt.target_usage {
            tracing::warn!(
                usage,
                gc_opt = ?self.gc_opt,
                "consensus storage is running out of space, pruning to minimum retention"
            );
            self.prune_below(cur_view.saturating_sub(self.gc_opt.minimum_retention), 0)
                .await?;
        }
        Ok(())
    }

    async fn prune_below(&self, view: u64, vid_share_view: u64) -> anyhow::Result<()> {
        // An empty write transaction still takes SQLite's write lock.
        if view == 0 && vid_share_view == 0 {
            return Ok(());
        }
        serializable_retry!(self, || async {
            let mut tx = self.db.write().await?;
            prune_to_view(&mut tx, view).await?;
            prune_vid_shares(&mut tx, vid_share_view).await?;
            tx.commit().await
        })
        .await
    }

    /// Measured in a read transaction: on SQLite it scans every page of the pruned tables, and
    /// holding the write lock that long stalls the vote and proposal writes consensus waits on.
    async fn consensus_storage_usage(&self) -> anyhow::Result<u64> {
        let usage_query = match Db::KIND {
            DbKind::Sqlite => format!(
                "SELECT sum(pgsize) FROM dbstat WHERE name IN ({})",
                PRUNE_TABLES
                    .iter()
                    .map(|table| format!("'{table}'"))
                    .join(",")
            ),
            DbKind::Postgres => {
                let table_sizes = PRUNE_TABLES
                    .iter()
                    .map(|table| format!("pg_table_size('{table}')"))
                    .join(" + ");
                format!("SELECT {table_sizes}")
            },
        };

        let mut tx = self.db.read().await?;
        let (usage,): (i64,) = query_as(&usage_query).fetch_one(tx.as_mut()).await?;
        Ok(usage as u64)
    }

    fn usage_check_due(&self) -> bool {
        let now = Instant::now();
        let mut last = self.last_usage_check.lock();
        if last.is_some_and(|last| now.duration_since(last) < self.gc_opt.usage_check_interval) {
            return false;
        }
        *last = Some(now);
        true
    }
}

// `vid_share2` is left out on purpose. It has its own retention, and counting a week of shares
// toward TARGET_USAGE would prune every other table down to MINIMUM_RETENTION.
const PRUNE_TABLES: &[&str] = &[
    "anchor_leaf2",
    "da_proposal2",
    "quorum_proposals2",
    "quorum_certificate2",
    "state_cert",
    "decided_cert2",
];

async fn prune_to_view(tx: &mut Transaction<Write>, view: u64) -> anyhow::Result<()> {
    tracing::debug!(view, "pruning consensus storage");

    for table in PRUNE_TABLES {
        prune_table(tx, table, view).await?;
    }

    Ok(())
}

async fn prune_table(
    tx: &mut Transaction<Write>,
    table: &'static str,
    view: u64,
) -> anyhow::Result<()> {
    let res = query(&format!("DELETE FROM {table} WHERE view < $1"))
        .bind(view as i64)
        .execute(tx.as_mut())
        .await
        .context(format!("pruning {table}"))?;
    if res.rows_affected() > 0 {
        tracing::info!(
            "garbage collected {} rows from {table}",
            res.rows_affected()
        );
    }
    Ok(())
}

// Small batches keep the SQLite write lock short, since consensus votes wait on it. Without them,
// lowering VID_SHARE_RETENTION would delete days of shares in one go. Only one share is added per
// view, so a batch per decide still clears any backlog.
const VID_SHARE_PRUNE_BATCH: i64 = 100;

async fn prune_vid_shares(tx: &mut Transaction<Write>, view: u64) -> anyhow::Result<()> {
    let res = query(
        "DELETE FROM vid_share2 WHERE view IN
            (SELECT view FROM vid_share2 WHERE view < $1 ORDER BY view LIMIT $2)",
    )
    .bind(view as i64)
    .bind(VID_SHARE_PRUNE_BATCH)
    .execute(tx.as_mut())
    .await
    .context("pruning vid_share2")?;
    if res.rows_affected() > 0 {
        tracing::info!(
            "garbage collected {} rows from vid_share2",
            res.rows_affected()
        );
    }
    Ok(())
}

#[async_trait]
impl SequencerPersistence for Persistence {
    fn into_catchup_provider(
        self,
        backoff: BackoffParams,
    ) -> anyhow::Result<Arc<dyn StateCatchup>> {
        Ok(Arc::new(SqlStateCatchup::new(Arc::new(self.db), backoff)))
    }

    async fn load_config(&self) -> anyhow::Result<Option<NetworkConfig>> {
        tracing::info!("loading config from Postgres");

        serializable_retry!(self, || async {
            // Select the most recent config (although there should only be one).
            let Some(row) = self
                .db
                .read()
                .await?
                .fetch_optional("SELECT config FROM network_config ORDER BY id DESC LIMIT 1")
                .await?
            else {
                tracing::info!("config not found");
                return Ok(None);
            };
            let json = row.try_get("config")?;

            let json =
                migrate_network_config(json).context("migration of network config failed")?;
            let config = serde_json::from_value(json).context("malformed config file")?;

            Ok(Some(config))
        })
        .await
    }

    async fn save_config(&self, cfg: &NetworkConfig) -> anyhow::Result<()> {
        tracing::info!("saving config to database");
        let json = serde_json::to_value(cfg)?;

        serializable_retry!(self, || async {
            let mut tx = self.db.write().await?;
            tx.execute(query("INSERT INTO network_config (config) VALUES ($1)").bind(json.clone()))
                .await?;
            tx.commit().await
        })
        .await
    }

    async fn persist_decided_leaves(
        &self,
        leaf_chain: impl IntoIterator<Item = (&LeafInfo<SeqTypes>, CertificatePair<SeqTypes>)> + Send,
        cert2: Option<&Certificate2<SeqTypes>>,
        _consumer: &(impl EventConsumer + 'static),
    ) -> anyhow::Result<()> {
        let cert2_row = cert2
            .map(|cert2| {
                let data = bincode::serialize(cert2).context("serializing cert2")?;
                anyhow::Ok((cert2.view_number.u64() as i64, data))
            })
            .transpose()?;
        let values = leaf_chain
            .into_iter()
            .map(|(info, cert)| {
                // The leaf may come with a large payload attached. We don't care about this payload
                // because we already store it separately, as part of the DA proposal. Storing it
                // here contributes to load on the DB for no reason, so we remove it before
                // serializing the leaf.
                let mut leaf = info.leaf.clone();
                leaf.unfill_block_payload();

                let view = cert.view_number().u64() as i64;
                let leaf_bytes = bincode::serialize(&leaf)?;
                let qc_bytes = bincode::serialize(cert.qc())?;
                let next_epoch_qc_bytes = match cert.next_epoch_qc() {
                    Some(qc) => Some(bincode::serialize(qc)?),
                    None => None,
                };
                Ok((view, leaf_bytes, qc_bytes, next_epoch_qc_bytes))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        // Append the new leaves and their cert2. We do this in its own transaction because even if
        // GC or the event consumer later fails, there is no need to abort the storage of the leaves.
        serializable_retry!(self, || async {
            let mut tx = self.db.write().await?;
            tx.upsert(
                "anchor_leaf2",
                ["view", "leaf", "qc", "next_epoch_qc"],
                ["view"],
                values.clone(),
            )
            .await?;
            if let Some(row) = &cert2_row {
                tx.upsert("decided_cert2", ["view", "data"], ["view"], [row.clone()])
                    .await?;
            }
            tx.commit().await
        })
        .await?;

        Ok(())
    }

    async fn process_decided_events(
        &self,
        view: ViewNumber,
        consumer: &(impl EventConsumer + 'static),
    ) -> anyhow::Result<Option<ViewNumber>> {
        let now = Instant::now();
        if self.consensus_only {
            self.skip_decide_events(view).await?;
        } else {
            // Generate events for the new leaves, then GC. On error `last_processed_view` is not
            // advanced past the failure point, so no data is lost and the range is retried.
            self.generate_decide_events(consumer).await?;

            // Best-effort GC of data not included in any decide event; runs again at the next
            // decide.
            if let Err(err) = self.prune_to_retention(view).await {
                tracing::warn!(?view, "pruning failed: {err:#}");
            }
        }
        self.internal_metrics
            .internal_process_decided_events_duration
            .add_point(now.elapsed().as_secs_f64());

        self.load_processed_view().await
    }

    async fn load_latest_acted_view(&self) -> anyhow::Result<Option<ViewNumber>> {
        serializable_retry!(self, || async {
            Ok(self
                .db
                .read()
                .await?
                .fetch_optional(query("SELECT view FROM highest_voted_view WHERE id = 0"))
                .await?
                .map(|row| {
                    let view: i64 = row.get("view");
                    ViewNumber::new(view as u64)
                }))
        })
        .await
    }

    async fn load_restart_view(&self) -> anyhow::Result<Option<ViewNumber>> {
        serializable_retry!(self, || async {
            Ok(self
                .db
                .read()
                .await?
                .fetch_optional(query("SELECT view FROM restart_view WHERE id = 0"))
                .await?
                .map(|row| {
                    let view: i64 = row.get("view");
                    ViewNumber::new(view as u64)
                }))
        })
        .await
    }

    async fn load_anchor_leaf(&self) -> anyhow::Result<Option<(Leaf2, CertificatePair<SeqTypes>)>> {
        serializable_retry!(self, || async {
            let Some(row) = self
                .db
                .read()
                .await?
                .fetch_optional(
                    "SELECT leaf, qc, next_epoch_qc FROM anchor_leaf2 ORDER BY view DESC LIMIT 1",
                )
                .await?
            else {
                return Ok(None);
            };

            let leaf_bytes: Vec<u8> = row.get("leaf");
            let leaf2: Leaf2 = bincode::deserialize(&leaf_bytes)?;

            let qc_bytes: Vec<u8> = row.get("qc");
            let qc2: QuorumCertificate2<SeqTypes> = bincode::deserialize(&qc_bytes)?;

            let maybe_next_qc_bytes: Option<Vec<u8>> = row.try_get("next_epoch_qc").ok();
            let maybe_next_qc2 = maybe_next_qc_bytes
                .and_then(|next_qc_bytes| bincode::deserialize(&next_qc_bytes).ok());

            let cert_pair = CertificatePair::new(qc2, maybe_next_qc2);

            Ok(Some((leaf2, cert_pair)))
        })
        .await
    }

    async fn load_anchor_view(&self) -> anyhow::Result<ViewNumber> {
        serializable_retry!(self, || async {
            let mut tx = self.db.read().await?;
            let (view,) = query_as::<(i64,)>("SELECT coalesce(max(view), 0) FROM anchor_leaf2")
                .fetch_one(tx.as_mut())
                .await?;
            Ok(ViewNumber::new(view as u64))
        })
        .await
    }

    async fn load_da_proposal(
        &self,
        view: ViewNumber,
    ) -> anyhow::Result<Option<Proposal<SeqTypes, DaProposal2<SeqTypes>>>> {
        serializable_retry!(self, || async {
            let result = self
                .db
                .read()
                .await?
                .fetch_optional(
                    query("SELECT data FROM da_proposal2 where view = $1").bind(view.u64() as i64),
                )
                .await?;

            result
                .map(|row| {
                    let bytes: Vec<u8> = row.get("data");
                    anyhow::Result::<_>::Ok(bincode::deserialize(&bytes)?)
                })
                .transpose()
        })
        .await
    }

    async fn load_vid_share(
        &self,
        view: ViewNumber,
    ) -> anyhow::Result<Option<Proposal<SeqTypes, VidDisperseShare<SeqTypes>>>> {
        serializable_retry!(self, || async {
            let result = self
                .db
                .read()
                .await?
                .fetch_optional(
                    query("SELECT data FROM vid_share2 where view = $1").bind(view.u64() as i64),
                )
                .await?;

            result
                .map(|row| {
                    let bytes: Vec<u8> = row.get("data");
                    anyhow::Result::<_>::Ok(bincode::deserialize(&bytes)?)
                })
                .transpose()
        })
        .await
    }

    async fn load_quorum_proposals(
        &self,
    ) -> anyhow::Result<BTreeMap<ViewNumber, Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>>>>
    {
        serializable_retry!(self, || async {
            let rows = self
                .db
                .read()
                .await?
                .fetch_all("SELECT * FROM quorum_proposals2")
                .await?;

            Ok(BTreeMap::from_iter(
                rows.into_iter()
                    .map(|row| {
                        let view: i64 = row.get("view");
                        let view_number: ViewNumber = ViewNumber::new(view.try_into()?);
                        let bytes: Vec<u8> = row.get("data");
                        let proposal: Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>> =
                            bincode::deserialize(&bytes).or_else(|error| {
                                bincode::deserialize::<
                                    Proposal<SeqTypes, QuorumProposalWrapperLegacy<SeqTypes>>,
                                >(&bytes)
                                .map(convert_proposal)
                                .inspect_err(|err_v3| {
                                    tracing::warn!(
                                        ?view_number,
                                        %error,
                                        %err_v3,
                                        "ignoring malformed quorum proposal DB row"
                                    );
                                })
                            })?;
                        Ok((view_number, proposal))
                    })
                    .collect::<anyhow::Result<Vec<_>>>()?,
            ))
        })
        .await
    }

    async fn load_quorum_proposal(
        &self,
        view: ViewNumber,
    ) -> anyhow::Result<Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>>> {
        serializable_retry!(self, || async {
            let mut tx = self.db.read().await?;
            let (data,) = query_as::<(Vec<u8>,)>(
                "SELECT data FROM quorum_proposals2 WHERE view = $1 LIMIT 1",
            )
            .bind(view.u64() as i64)
            .fetch_one(tx.as_mut())
            .await?;
            let proposal: Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>> =
                bincode::deserialize(&data).or_else(|error| {
                    bincode::deserialize::<
                                Proposal<SeqTypes, QuorumProposalWrapperLegacy<SeqTypes>>,
                            >(&data)
                            .map(convert_proposal)
                            .context(format!(
                                "Failed to deserialize quorum proposal for view {view}. \
                                 error={error}"
                            ))
                })?;
            Ok(proposal)
        })
        .await
    }

    async fn append_vid(
        &self,
        proposal: &Proposal<SeqTypes, VidDisperseShare<SeqTypes>>,
    ) -> anyhow::Result<()> {
        let view = proposal.data.view_number().u64();
        let payload_hash = proposal.data.payload_commitment();
        let data_bytes = bincode::serialize(proposal).unwrap();

        let now = Instant::now();
        let res = serializable_retry!(self, || async {
            let mut tx = self.db.write().await?;
            tx.upsert(
                "vid_share2",
                ["view", "data", "payload_hash"],
                ["view"],
                [(view as i64, data_bytes.clone(), payload_hash.to_string())],
            )
            .await?;
            tx.commit().await
        })
        .await;
        self.internal_metrics
            .internal_append_vid_duration
            .add_point(now.elapsed().as_secs_f64());
        res
    }

    async fn append_da(
        &self,
        proposal: &Proposal<SeqTypes, DaProposal<SeqTypes>>,
        vid_commit: VidCommitment,
    ) -> anyhow::Result<()> {
        let data = &proposal.data;
        let view = data.view_number().u64();
        let data_bytes = bincode::serialize(proposal).unwrap();

        let now = Instant::now();
        let res = serializable_retry!(self, || async {
            let mut tx = self.db.write().await?;
            tx.upsert(
                "da_proposal",
                ["view", "data", "payload_hash"],
                ["view"],
                [(view as i64, data_bytes.clone(), vid_commit.to_string())],
            )
            .await?;
            tx.commit().await
        })
        .await;
        self.internal_metrics
            .internal_append_da_duration
            .add_point(now.elapsed().as_secs_f64());
        res
    }

    async fn record_action(
        &self,
        view: ViewNumber,
        _epoch: Option<EpochNumber>,
        action: HotShotAction,
    ) -> anyhow::Result<()> {
        // Todo Remove this after https://github.com/EspressoSystems/espresso-network/issues/1931
        if !matches!(action, HotShotAction::Propose | HotShotAction::Vote) {
            return Ok(());
        }

        serializable_retry!(self, || async {
            let stmt = format!(
                "INSERT INTO highest_voted_view (id, view) VALUES (0, $1)
                ON CONFLICT (id) DO UPDATE SET view = {MAX_FN}(highest_voted_view.view, \
                 excluded.view)",
                MAX_FN = Db::MAX_FN,
            );

            let mut tx = self.db.write().await?;
            tx.execute(query(&stmt).bind(view.u64() as i64)).await?;

            if matches!(action, HotShotAction::Vote) {
                let restart_view = view + 1;
                let stmt = format!(
                    "INSERT INTO restart_view (id, view) VALUES (0, $1)
                    ON CONFLICT (id) DO UPDATE SET view = {MAX_FN}(restart_view.view, \
                     excluded.view)",
                    MAX_FN = Db::MAX_FN,
                );
                tx.execute(query(&stmt).bind(restart_view.u64() as i64))
                    .await?;
            }

            tx.commit().await
        })
        .await
    }

    async fn append_quorum_proposal2(
        &self,
        proposal: &Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>>,
    ) -> anyhow::Result<()> {
        let view_number = proposal.data.view_number().u64();

        let proposal_bytes = bincode::serialize(&proposal).context("serializing proposal")?;
        let leaf_hash = Committable::commit(&Leaf2::from_quorum_proposal(&proposal.data));

        // We also keep track of any QC we see in case we need it to recover our archival storage.
        let justify_qc = proposal.data.justify_qc();
        let justify_qc_bytes = bincode::serialize(&justify_qc).context("serializing QC")?;
        let justify_qc_view = justify_qc.view_number.u64() as i64;
        let justify_qc_leaf_commit = justify_qc.data.leaf_commit.to_string();

        let now = Instant::now();
        let res = serializable_retry!(self, || async {
            let mut tx = self.db.write().await?;
            tx.upsert(
                "quorum_proposals2",
                ["view", "leaf_hash", "data"],
                ["view"],
                [(
                    view_number as i64,
                    leaf_hash.to_string(),
                    proposal_bytes.clone(),
                )],
            )
            .await?;
            tx.upsert(
                "quorum_certificate2",
                ["view", "leaf_hash", "data"],
                ["view"],
                [(
                    justify_qc_view,
                    justify_qc_leaf_commit.clone(),
                    justify_qc_bytes.clone(),
                )],
            )
            .await?;
            tx.commit().await
        })
        .await;
        self.internal_metrics
            .internal_append_quorum2_duration
            .add_point(now.elapsed().as_secs_f64());
        res
    }

    async fn append_cert2(
        &self,
        view: ViewNumber,
        cert2: Certificate2<SeqTypes>,
    ) -> anyhow::Result<()> {
        let data = bincode::serialize(&cert2).context("serializing cert2")?;
        let view_i64 = view.u64() as i64;
        serializable_retry!(self, || async {
            let mut tx = self.db.write().await?;
            tx.upsert(
                "decided_cert2",
                ["view", "data"],
                ["view"],
                [(view_i64, data.clone())],
            )
            .await?;
            tx.commit().await
        })
        .await
    }

    async fn load_cert2(&self, view: ViewNumber) -> anyhow::Result<Option<Certificate2<SeqTypes>>> {
        let row = self
            .db
            .read()
            .await?
            .fetch_optional(
                query("SELECT data FROM decided_cert2 WHERE view = $1").bind(view.u64() as i64),
            )
            .await?;
        row.map(|row| {
            let bytes: Vec<u8> = row.get("data");
            bincode::deserialize::<Certificate2<SeqTypes>>(&bytes).context("deserializing cert2")
        })
        .transpose()
    }

    async fn append_high_qc2(&self, high_qc: QuorumCertificate2<SeqTypes>) -> anyhow::Result<()> {
        let view = high_qc.view_number();
        let data = bincode::serialize(&high_qc).context("serializing high_qc2")?;
        serializable_retry!(self, || async {
            let mut tx = self.db.write().await?;
            // Compare-and-set inside one write transaction so a stale
            // concurrent write can never regress the stored view: under
            // SERIALIZABLE a racing writer conflicts and retries; on SQLite
            // writes are serialized.
            let stored_view = query("SELECT data FROM high_qc2 WHERE id = true")
                .fetch_optional(tx.as_mut())
                .await?
                .map(|row| {
                    let bytes: Vec<u8> = row.get("data");
                    bincode::deserialize::<QuorumCertificate2<SeqTypes>>(&bytes)
                        .context("deserializing existing high_qc2")
                        .map(|qc| qc.view_number())
                })
                .transpose()?;
            if stored_view.is_some_and(|stored| stored >= view) {
                return Ok(());
            }
            tx.upsert("high_qc2", ["id", "data"], ["id"], [(true, data.clone())])
                .await?;
            tx.commit().await
        })
        .await
    }

    async fn load_high_qc2(&self) -> anyhow::Result<Option<QuorumCertificate2<SeqTypes>>> {
        let row = self
            .db
            .read()
            .await?
            .fetch_optional("SELECT data FROM high_qc2 WHERE id = true")
            .await?;
        row.map(|row| {
            let bytes: Vec<u8> = row.get("data");
            bincode::deserialize::<QuorumCertificate2<SeqTypes>>(&bytes)
                .context("deserializing high_qc2")
        })
        .transpose()
    }

    async fn load_upgrade_certificate(
        &self,
    ) -> anyhow::Result<Option<UpgradeCertificate<SeqTypes>>> {
        let result = self
            .db
            .read()
            .await?
            .fetch_optional("SELECT * FROM upgrade_certificate where id = true")
            .await?;

        result
            .map(|row| {
                let bytes: Vec<u8> = row.get("data");
                anyhow::Result::<_>::Ok(bincode::deserialize(&bytes)?)
            })
            .transpose()
    }

    async fn store_upgrade_certificate(
        &self,
        decided_upgrade_certificate: Option<UpgradeCertificate<SeqTypes>>,
    ) -> anyhow::Result<()> {
        let certificate = match decided_upgrade_certificate {
            Some(cert) => cert,
            None => return Ok(()),
        };
        let upgrade_certificate_bytes =
            bincode::serialize(&certificate).context("serializing upgrade certificate")?;
        serializable_retry!(self, || async {
            let mut tx = self.db.write().await?;
            tx.upsert(
                "upgrade_certificate",
                ["id", "data"],
                ["id"],
                [(true, upgrade_certificate_bytes.clone())],
            )
            .await?;
            tx.commit().await
        })
        .await
    }

    async fn load_next_epoch_quorum_certificate(
        &self,
    ) -> anyhow::Result<Option<NextEpochQuorumCertificate2<SeqTypes>>> {
        let result = self
            .db
            .read()
            .await?
            .fetch_optional("SELECT * FROM next_epoch_quorum_certificate where id = true")
            .await?;

        result
            .map(|row| {
                let bytes: Vec<u8> = row.get("data");
                anyhow::Result::<_>::Ok(bincode::deserialize(&bytes)?)
            })
            .transpose()
    }

    async fn append_next_epoch_high_qc2(
        &self,
        next_epoch_high_qc: NextEpochQuorumCertificate2<SeqTypes>,
    ) -> anyhow::Result<()> {
        let view = next_epoch_high_qc.view_number();
        let data =
            bincode::serialize(&next_epoch_high_qc).context("serializing next epoch high_qc2")?;
        serializable_retry!(self, || async {
            let mut tx = self.db.write().await?;
            let stored_view =
                query("SELECT data FROM next_epoch_quorum_certificate WHERE id = true")
                    .fetch_optional(tx.as_mut())
                    .await?
                    .map(|row| {
                        let bytes: Vec<u8> = row.get("data");
                        bincode::deserialize::<NextEpochQuorumCertificate2<SeqTypes>>(&bytes)
                            .context("deserializing existing next epoch high_qc2")
                            .map(|qc| qc.view_number())
                    })
                    .transpose()?;
            if stored_view.is_some_and(|stored| stored >= view) {
                return Ok(());
            }
            tx.upsert(
                "next_epoch_quorum_certificate",
                ["id", "data"],
                ["id"],
                [(true, data.clone())],
            )
            .await?;
            tx.commit().await
        })
        .await
    }

    async fn store_eqc(
        &self,
        high_qc: QuorumCertificate2<SeqTypes>,
        next_epoch_high_qc: NextEpochQuorumCertificate2<SeqTypes>,
    ) -> anyhow::Result<()> {
        let eqc_bytes =
            bincode::serialize(&(high_qc, next_epoch_high_qc)).context("serializing eqc")?;
        serializable_retry!(self, || async {
            let mut tx = self.db.write().await?;
            tx.upsert("eqc", ["id", "data"], ["id"], [(true, eqc_bytes.clone())])
                .await?;
            tx.commit().await
        })
        .await
    }

    async fn load_eqc(
        &self,
    ) -> Option<(
        QuorumCertificate2<SeqTypes>,
        NextEpochQuorumCertificate2<SeqTypes>,
    )> {
        let result = self
            .db
            .read()
            .await
            .ok()?
            .fetch_optional("SELECT * FROM eqc where id = true")
            .await
            .ok()?;

        result
            .map(|row| {
                let bytes: Vec<u8> = row.get("data");
                bincode::deserialize(&bytes)
            })
            .transpose()
            .ok()?
    }

    async fn append_da2(
        &self,
        proposal: &Proposal<SeqTypes, DaProposal2<SeqTypes>>,
        vid_commit: VidCommitment,
    ) -> anyhow::Result<()> {
        if self.consensus_only {
            return Ok(());
        }
        let data = &proposal.data;
        let view = data.view_number().u64();
        let data_bytes = bincode::serialize(proposal).unwrap();

        let now = Instant::now();
        let res = serializable_retry!(self, || async {
            let mut tx = self.db.write().await?;
            tx.upsert(
                "da_proposal2",
                ["view", "data", "payload_hash"],
                ["view"],
                [(view as i64, data_bytes.clone(), vid_commit.to_string())],
            )
            .await?;
            tx.commit().await
        })
        .await;
        self.internal_metrics
            .internal_append_da2_duration
            .add_point(now.elapsed().as_secs_f64());
        res
    }

    async fn store_drb_result(
        &self,
        epoch: EpochNumber,
        drb_result: DrbResult,
    ) -> anyhow::Result<()> {
        let epoch_i64 = epoch.u64() as i64;
        let drb_result_vec = Vec::from(drb_result);
        serializable_retry!(self, || async {
            let mut tx = self.db.write().await?;
            tx.upsert(
                "epoch_drb_and_root",
                ["epoch", "drb_result"],
                ["epoch"],
                [(epoch_i64, drb_result_vec.clone())],
            )
            .await?;
            tx.commit().await
        })
        .await
    }

    async fn store_drb_input(&self, drb_input: DrbInput) -> anyhow::Result<()> {
        if let Ok(loaded_drb_input) = self.load_drb_input(drb_input.epoch).await {
            if loaded_drb_input.difficulty_level != drb_input.difficulty_level {
                tracing::error!("Overwriting {loaded_drb_input:?} in storage with {drb_input:?}");
            } else if loaded_drb_input.iteration >= drb_input.iteration {
                anyhow::bail!(
                    "DrbInput in storage {:?} is more recent than {:?}, refusing to update",
                    loaded_drb_input,
                    drb_input
                )
            }
        }

        let drb_epoch_i64 = drb_input.epoch as i64;
        let drb_input_bytes = bincode::serialize(&drb_input)
            .context("Failed to serialize DrbInput. This is not fatal, but should never happen.")?;

        serializable_retry!(self, || async {
            let mut tx = self.db.write().await?;
            tx.upsert(
                "drb",
                ["epoch", "drb_input"],
                ["epoch"],
                [(drb_epoch_i64, drb_input_bytes.clone())],
            )
            .await?;
            tx.commit().await
        })
        .await
    }

    async fn load_drb_input(&self, epoch: u64) -> anyhow::Result<DrbInput> {
        let row = self
            .db
            .read()
            .await?
            .fetch_optional(query("SELECT drb_input FROM drb WHERE epoch = $1").bind(epoch as i64))
            .await?;

        match row {
            None => anyhow::bail!("No DrbInput for epoch {} in storage", epoch),
            Some(row) => {
                let drb_input_bytes: Vec<u8> = row.try_get("drb_input")?;
                let drb_input = bincode::deserialize(&drb_input_bytes)
                    .context("Failed to deserialize drb_input from storage")?;

                Ok(drb_input)
            },
        }
    }

    async fn add_state_cert(
        &self,
        state_cert: LightClientStateUpdateCertificateV2<SeqTypes>,
    ) -> anyhow::Result<()> {
        let view_number = state_cert.light_client_state.view_number as i64;
        let state_cert_bytes = bincode::serialize(&state_cert)
            .context("serializing light client state update certificate")?;

        serializable_retry!(self, || async {
            let mut tx = self.db.write().await?;
            tx.upsert(
                "state_cert",
                ["view", "state_cert"],
                ["view"],
                [(view_number, state_cert_bytes.clone())],
            )
            .await?;
            tx.commit().await
        })
        .await
    }

    async fn load_state_cert(
        &self,
    ) -> anyhow::Result<Option<LightClientStateUpdateCertificateV2<SeqTypes>>> {
        let Some(row) = self
            .db
            .read()
            .await?
            .fetch_optional(
                "SELECT state_cert FROM finalized_state_cert ORDER BY epoch DESC LIMIT 1",
            )
            .await?
        else {
            return Ok(None);
        };
        let bytes: Vec<u8> = row.get("state_cert");

        let cert = match bincode::deserialize(&bytes) {
            Ok(cert) => cert,
            Err(err) => {
                tracing::info!(
                    error = %err,
                    "Failed to deserialize state certificate with v2. attempting with v1"
                );

                let v1_cert =
                    bincode::deserialize::<LightClientStateUpdateCertificateV1<SeqTypes>>(&bytes)
                        .with_context(|| {
                        format!("Failed to deserialize using both v1 and v2. error: {err}")
                    })?;

                v1_cert.into()
            },
        };

        Ok(Some(cert))
    }

    async fn load_start_epoch_info(&self) -> anyhow::Result<Vec<InitializerEpochInfo<SeqTypes>>> {
        let rows = self
            .db
            .read()
            .await?
            .fetch_all(
                query("SELECT * from epoch_drb_and_root ORDER BY epoch DESC LIMIT $1")
                    .bind(RECENT_STAKE_TABLES_LIMIT as i64),
            )
            .await?;

        // reverse the rows vector to return the most recent epochs, but in ascending order
        rows.into_iter()
            .rev()
            .map(|row| {
                let epoch: i64 = row.try_get("epoch")?;
                let drb_result: Option<Vec<u8>> = row.try_get("drb_result")?;
                let block_header: Option<Vec<u8>> = row.try_get("block_header")?;
                if let Some(drb_result) = drb_result {
                    let drb_result_array = drb_result
                        .try_into()
                        .or_else(|_| bail!("invalid drb result"))?;
                    let block_header: Option<<SeqTypes as NodeType>::BlockHeader> = block_header
                        .map(|data| bincode::deserialize(&data))
                        .transpose()?;
                    Ok(Some(InitializerEpochInfo::<SeqTypes> {
                        epoch: EpochNumber::new(epoch as u64),
                        drb_result: drb_result_array,
                        block_header,
                    }))
                } else {
                    // Right now we skip the epoch_drb_and_root row if there is no drb result.
                    // This seems reasonable based on the expected order of events, but please double check!
                    Ok(None)
                }
            })
            .filter_map(|e| match e {
                Err(v) => Some(Err(v)),
                Ok(Some(v)) => Some(Ok(v)),
                Ok(None) => None,
            })
            .collect()
    }

    fn enable_metrics(&mut self, metrics: &dyn Metrics) {
        self.internal_metrics = PersistenceMetricsValue::new(metrics);

        if let Some(probe) = &self.probe {
            probe.register(&*metrics.subgroup("disk".into()));
        }
    }
}

#[async_trait]
impl StateCertPersistence for Persistence {
    async fn get_state_cert_by_epoch(
        &self,
        epoch: u64,
    ) -> anyhow::Result<Option<LightClientStateUpdateCertificateV2<SeqTypes>>> {
        let Some(row) = self
            .db
            .read()
            .await?
            .fetch_optional(
                query("SELECT state_cert FROM finalized_state_cert WHERE epoch = $1")
                    .bind(epoch as i64),
            )
            .await?
        else {
            return Ok(None);
        };
        let bytes: Vec<u8> = row.get("state_cert");

        let cert = match bincode::deserialize(&bytes) {
            Ok(cert) => cert,
            Err(err) => {
                tracing::info!(
                    error = %err,
                    "Failed to deserialize state certificate with v2. attempting with v1"
                );

                let v1_cert =
                    bincode::deserialize::<LightClientStateUpdateCertificateV1<SeqTypes>>(&bytes)
                        .with_context(|| {
                        format!("Failed to deserialize using both v1 and v2. error: {err}")
                    })?;

                v1_cert.into()
            },
        };

        Ok(Some(cert))
    }

    async fn insert_state_cert(
        &self,
        epoch: u64,
        cert: LightClientStateUpdateCertificateV2<SeqTypes>,
    ) -> anyhow::Result<()> {
        let epoch_i64 = epoch as i64;
        let bytes = bincode::serialize(&cert)
            .with_context(|| format!("Failed to serialize state cert for epoch {epoch}"))?;

        serializable_retry!(self, || async {
            let mut tx = self.db.write().await?;
            tx.upsert(
                "finalized_state_cert",
                ["epoch", "state_cert"],
                ["epoch"],
                [(epoch_i64, bytes.clone())],
            )
            .await?;
            tx.commit().await
        })
        .await
    }
}

fn deserialize_authenticated_validator_map(
    bytes: &[u8],
) -> anyhow::Result<AuthenticatedValidatorMap> {
    if let Ok(map) = bincode::deserialize::<AuthenticatedValidatorMap>(bytes) {
        return Ok(map);
    }

    // Pre-Schnorr-Option: stake_table_key as Option<KEY>, state_ver_key as raw KEY.
    if let Ok(pre_schnorr) = bincode::deserialize::<
        IndexMap<Address, crate::persistence::RegisteredValidatorPreSchnorrOption>,
    >(bytes)
    {
        return pre_schnorr
            .into_iter()
            .map(|(addr, v)| {
                let registered = v.migrate();
                let authenticated = AuthenticatedValidator::try_from(registered)?;
                Ok((addr, authenticated))
            })
            .collect();
    }

    // Pre-Option: both keys raw, with x25519/p2p fields.
    let legacy: IndexMap<Address, crate::persistence::RegisteredValidatorPreOption> =
        bincode::deserialize(bytes).context("deserializing stake table")?;
    legacy
        .into_iter()
        .map(|(addr, v)| {
            let registered = v.migrate();
            let authenticated = AuthenticatedValidator::try_from(registered)?;
            Ok((addr, authenticated))
        })
        .collect()
}

#[async_trait]
impl MembershipPersistence for Persistence {
    async fn load_stake(&self, epoch: EpochNumber) -> anyhow::Result<Option<StakeTuple>> {
        let result = self
            .db
            .read()
            .await?
            .fetch_optional(
                query(
                    "SELECT stake, block_reward, stake_table_hash FROM epoch_drb_and_root WHERE \
                     epoch = $1 AND stake IS NOT NULL",
                )
                .bind(epoch.u64() as i64),
            )
            .await?;

        result
            .map(|row| {
                let stake_table_bytes: Vec<u8> = row.get("stake");
                let reward_bytes: Option<Vec<u8>> = row.get("block_reward");
                let stake_table_hash_bytes: Option<Vec<u8>> = row.get("stake_table_hash");
                let stake_table = deserialize_authenticated_validator_map(&stake_table_bytes)?;
                let reward: Option<RewardAmount> = reward_bytes
                    .map(|b| bincode::deserialize(&b).context("deserializing block_reward"))
                    .transpose()?;
                let stake_table_hash: Option<StakeTableHash> = stake_table_hash_bytes
                    .map(|b| bincode::deserialize(&b).context("deserializing stake table hash"))
                    .transpose()?;

                Ok((stake_table, reward, stake_table_hash))
            })
            .transpose()
    }

    async fn load_drb_result(&self, epoch: EpochNumber) -> anyhow::Result<Option<DrbResult>> {
        let result = self
            .db
            .read()
            .await?
            .fetch_optional(
                query(
                    "SELECT drb_result FROM epoch_drb_and_root WHERE epoch = $1 AND drb_result IS \
                     NOT NULL",
                )
                .bind(epoch.u64() as i64),
            )
            .await?;

        result
            .map(|row| {
                let bytes: Vec<u8> = row.get("drb_result");
                bytes.try_into().or_else(|_| bail!("invalid drb result"))
            })
            .transpose()
    }

    async fn load_epoch_root(&self, epoch: EpochNumber) -> anyhow::Result<Option<Header>> {
        let result = self
            .db
            .read()
            .await?
            .fetch_optional(
                query(
                    "SELECT block_header FROM epoch_drb_and_root WHERE epoch = $1 AND \
                     block_header IS NOT NULL",
                )
                .bind(epoch.u64() as i64),
            )
            .await?;

        result
            .map(|row| {
                let bytes: Vec<u8> = row.get("block_header");
                bincode::deserialize(&bytes).context("deserializing block header")
            })
            .transpose()
    }

    async fn store_epoch_root(
        &self,
        epoch: EpochNumber,
        block_header: Header,
    ) -> anyhow::Result<()> {
        let epoch_i64 = epoch.u64() as i64;
        let block_header_bytes =
            bincode::serialize(&block_header).context("serializing block header")?;

        serializable_retry!(self, || async {
            let mut tx = self.db.write().await?;
            tx.upsert(
                "epoch_drb_and_root",
                ["epoch", "block_header"],
                ["epoch"],
                [(epoch_i64, block_header_bytes.clone())],
            )
            .await?;
            tx.commit().await
        })
        .await
    }

    async fn load_latest_stake(&self, limit: u64) -> anyhow::Result<Option<Vec<IndexedStake>>> {
        let mut tx = self.db.read().await?;

        let rows = match query_as::<(i64, Vec<u8>, Option<Vec<u8>>, Option<Vec<u8>>)>(
            "SELECT epoch, stake, block_reward, stake_table_hash FROM epoch_drb_and_root WHERE \
             stake is NOT NULL ORDER BY epoch DESC LIMIT $1",
        )
        .bind(limit as i64)
        .fetch_all(tx.as_mut())
        .await
        {
            Ok(bytes) => bytes,
            Err(err) => {
                tracing::error!("error loading stake tables: {err:#}");
                bail!("{err:#}");
            },
        };

        let stakes: anyhow::Result<Vec<IndexedStake>> = rows
            .into_iter()
            .map(
                |(id, stake_bytes, reward_bytes_opt, stake_table_hash_bytes_opt)| {
                    let stake_table = deserialize_authenticated_validator_map(&stake_bytes)?;

                    let block_reward: Option<RewardAmount> = reward_bytes_opt
                        .map(|b| bincode::deserialize(&b).context("deserializing block_reward"))
                        .transpose()?;

                    let stake_table_hash: Option<StakeTableHash> = stake_table_hash_bytes_opt
                        .map(|b| bincode::deserialize(&b).context("deserializing stake table hash"))
                        .transpose()?;

                    Ok((
                        EpochNumber::new(id as u64),
                        (stake_table, block_reward),
                        stake_table_hash,
                    ))
                },
            )
            .collect();

        Ok(Some(stakes?))
    }

    async fn store_stake(
        &self,
        epoch: EpochNumber,
        stake: AuthenticatedValidatorMap,
        block_reward: Option<RewardAmount>,
        stake_table_hash: Option<StakeTableHash>,
    ) -> anyhow::Result<()> {
        let epoch_i64 = epoch.u64() as i64;
        let stake_table_bytes = bincode::serialize(&stake).context("serializing stake table")?;
        let reward_bytes = block_reward
            .map(|r| bincode::serialize(&r).context("serializing block reward"))
            .transpose()?;
        let stake_table_hash_bytes = stake_table_hash
            .map(|h| bincode::serialize(&h).context("serializing stake table hash"))
            .transpose()?;
        serializable_retry!(self, || async {
            let mut tx = self.db.write().await?;
            tx.upsert(
                "epoch_drb_and_root",
                ["epoch", "stake", "block_reward", "stake_table_hash"],
                ["epoch"],
                [(
                    epoch_i64,
                    stake_table_bytes.clone(),
                    reward_bytes.clone(),
                    stake_table_hash_bytes.clone(),
                )],
            )
            .await?;
            tx.commit().await
        })
        .await
    }

    async fn store_events(
        &self,
        l1_finalized: u64,
        events: Vec<(EventKey, StakeTableEvent)>,
    ) -> anyhow::Result<()> {
        let l1_finalized_i64: i64 = l1_finalized.try_into()?;
        let serialized_events = events
            .into_iter()
            .map(|((block_number, index), event)| {
                Ok((
                    i64::try_from(block_number)?,
                    i64::try_from(index)?,
                    serde_json::to_value(event).context("l1 event to value")?,
                ))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        serializable_retry!(self, || async {
            let mut tx = self.db.write().await?;

            // check last l1 block if there is any
            let last_processed_l1_block = query_as::<(i64,)>(
                "SELECT last_l1_block FROM stake_table_events_l1_block where id = 0",
            )
            .fetch_optional(tx.as_mut())
            .await?
            .map(|(l1,)| l1);

            tracing::debug!("last l1 finalizes in database = {last_processed_l1_block:?}");

            // skip events storage if the database already has higher l1 block events
            let serialized_events_len = serialized_events.len();
            if last_processed_l1_block > Some(l1_finalized_i64) {
                tracing::debug!(
                    ?last_processed_l1_block,
                    l1_finalized,
                    serialized_events_len,
                    "last l1 finalized stored is already higher"
                );
                return Ok(());
            }

            if !serialized_events.is_empty() {
                let mut query_builder: sqlx::QueryBuilder<Db> = sqlx::QueryBuilder::new(
                    "INSERT INTO stake_table_events (l1_block, log_index, event) ",
                );

                query_builder.push_values(
                    serialized_events.iter().cloned(),
                    |mut b, (l1_block, log_index, event)| {
                        b.push_bind(l1_block).push_bind(log_index).push_bind(event);
                    },
                );

                query_builder.push(" ON CONFLICT DO NOTHING");
                let query = query_builder.build();

                query.execute(tx.as_mut()).await?;
            }

            // update l1 block
            tx.upsert(
                "stake_table_events_l1_block",
                ["id", "last_l1_block"],
                ["id"],
                [(0_i32, l1_finalized_i64)],
            )
            .await?;

            tx.commit().await?;

            Ok(())
        })
        .await
    }

    /// Loads all events from persistent storage up to the specified L1 block.
    ///
    /// # Returns
    ///
    /// Returns a tuple containing:
    /// - `Option<u64>` - The queried L1 block for which all events have been successfully fetched.
    /// - `Vec<(EventKey, StakeTableEvent)>` - A list of events, where each entry is a tuple of the event key
    /// event key is (l1 block number, log index)
    ///   and the corresponding StakeTable event.
    ///
    async fn load_events(
        &self,
        from_l1_block: u64,
        to_l1_block: u64,
    ) -> anyhow::Result<(
        Option<EventsPersistenceRead>,
        Vec<(EventKey, StakeTableEvent)>,
    )> {
        let mut tx = self.db.read().await?;

        // check last l1 block if there is any
        let res = query_as::<(i64,)>(
            "SELECT last_l1_block FROM stake_table_events_l1_block where id = 0",
        )
        .fetch_optional(tx.as_mut())
        .await?;

        let Some((last_processed_l1_block,)) = res else {
            // this just means we dont have any events stored
            return Ok((None, Vec::new()));
        };

        // Determine the L1 block for querying events.
        // If the last stored L1 block is greater than the requested block, limit the query to the requested block.
        // Otherwise, query up to the last stored block.
        let to_l1_block = to_l1_block.try_into()?;
        let query_l1_block = if last_processed_l1_block > to_l1_block {
            to_l1_block
        } else {
            last_processed_l1_block
        };

        let rows = query(
            "SELECT l1_block, log_index, event FROM stake_table_events WHERE $1 <= l1_block AND \
             l1_block <= $2 ORDER BY l1_block ASC, log_index ASC",
        )
        .bind(i64::try_from(from_l1_block)?)
        .bind(query_l1_block)
        .fetch_all(tx.as_mut())
        .await?;

        let events = rows
            .into_iter()
            .map(|row| {
                let l1_block: i64 = row.try_get("l1_block")?;
                let log_index: i64 = row.try_get("log_index")?;
                let event = serde_json::from_value(row.try_get("event")?)?;

                Ok(((l1_block.try_into()?, log_index.try_into()?), event))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        // Determine the read state based on the queried block range.
        // - If the persistence returned events up to the requested block, the read is complete.
        // - Otherwise, indicate that the read is up to the last processed block.
        if query_l1_block == to_l1_block {
            Ok((Some(EventsPersistenceRead::Complete), events))
        } else {
            Ok((
                Some(EventsPersistenceRead::UntilL1Block(
                    query_l1_block.try_into()?,
                )),
                events,
            ))
        }
    }

    async fn delete_stake_tables(&self) -> anyhow::Result<()> {
        serializable_retry!(self, || async {
            let mut tx = self.db.write().await?;
            if Db::KIND == DbKind::Postgres {
                query(
                    "TRUNCATE stake_table_events, stake_table_events_l1_block, \
                     epoch_drb_and_root, stake_table_validators",
                )
                .execute(tx.as_mut())
                .await?;
            } else {
                query("DELETE FROM stake_table_events")
                    .execute(tx.as_mut())
                    .await?;
                query("DELETE FROM stake_table_events_l1_block")
                    .execute(tx.as_mut())
                    .await?;
                query("DELETE FROM epoch_drb_and_root")
                    .execute(tx.as_mut())
                    .await?;
                query("DELETE FROM stake_table_validators")
                    .execute(tx.as_mut())
                    .await?;
            }
            tx.commit().await?;
            Ok(())
        })
        .await
    }

    async fn store_all_validators(
        &self,
        epoch: EpochNumber,
        all_validators: RegisteredValidatorMap,
    ) -> anyhow::Result<()> {
        if all_validators.is_empty() {
            return Ok(());
        }

        let epoch_i64 = epoch.u64() as i64;
        let serialized_validators = all_validators
            .into_iter()
            .map(|(address, validator)| {
                let validator_json =
                    serde_json::to_value(&validator).context("serializing validator to json")?;
                Ok((address.to_string(), validator_json))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        serializable_retry!(self, || async {
            let mut tx = self.db.write().await?;

            let mut query_builder = QueryBuilder::new(
                "INSERT INTO stake_table_validators (epoch, address, validator) ",
            );

            query_builder.push_values(
                serialized_validators.iter().cloned(),
                |mut b, (address, validator)| {
                    b.push_bind(epoch_i64)
                        .push_bind(address)
                        .push_bind(validator);
                },
            );

            query_builder
                .push(" ON CONFLICT (epoch, address) DO UPDATE SET validator = EXCLUDED.validator");

            let query = query_builder.build();

            query.execute(tx.as_mut()).await?;

            tx.commit().await?;
            Ok(())
        })
        .await
    }

    async fn load_all_validators(
        &self,
        epoch: EpochNumber,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<Vec<RegisteredValidator<PubKey>>> {
        let mut tx = self.db.read().await?;

        // Use LOWER(address) in ORDER BY to ensure consistent ordering for SQlite and Postgres.
        // Postgres sorts text case sensitively by default, while SQLite sorts case insensitively.
        // Applying LOWER() makes the result consistent.
        let rows = query(
            "SELECT address, validator
         FROM stake_table_validators
         WHERE epoch = $1
         ORDER BY LOWER(address) ASC
         LIMIT $2 OFFSET $3",
        )
        .bind(epoch.u64() as i64)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(tx.as_mut())
        .await?;
        rows.into_iter()
            .map(|row| {
                let validator_json: serde_json::Value = row.try_get("validator")?;
                serde_json::from_value::<RegisteredValidator<PubKey>>(validator_json)
                    .map_err(Into::into)
            })
            .collect()
    }
}

#[async_trait]
impl Provider<SeqTypes, VidCommonRequest> for Persistence {
    #[tracing::instrument(skip(self))]
    async fn fetch(&self, req: VidCommonRequest) -> Option<VidCommon> {
        let mut tx = match self.db.read().await {
            Ok(tx) => tx,
            Err(err) => {
                tracing::warn!("could not open transaction: {err:#}");
                return None;
            },
        };

        let bytes = match query_as::<(Vec<u8>,)>(
            "SELECT data FROM vid_share2 WHERE payload_hash = $1 LIMIT 1",
        )
        .bind(req.0.to_string())
        .fetch_optional(tx.as_mut())
        .await
        {
            Ok(Some((bytes,))) => bytes,
            Ok(None) => return None,
            Err(err) => {
                tracing::error!("error loading VID share: {err:#}");
                return None;
            },
        };

        let share: Proposal<SeqTypes, VidDisperseShare<SeqTypes>> =
            match bincode::deserialize(&bytes) {
                Ok(share) => share,
                Err(err) => {
                    tracing::warn!("error decoding VID share: {err:#}");
                    return None;
                },
            };

        match share.data {
            VidDisperseShare::V0(vid) => Some(VidCommon::V0(vid.common)),
            VidDisperseShare::V1(vid) => Some(VidCommon::V1(vid.common)),
            VidDisperseShare::V2(vid) => Some(VidCommon::V2(vid.common)),
        }
    }
}

#[async_trait]
impl Provider<SeqTypes, PayloadRequest> for Persistence {
    #[tracing::instrument(skip(self))]
    async fn fetch(&self, req: PayloadRequest) -> Option<Payload> {
        let mut tx = match self.db.read().await {
            Ok(tx) => tx,
            Err(err) => {
                tracing::warn!("could not open transaction: {err:#}");
                return None;
            },
        };

        let bytes = match query_as::<(Vec<u8>,)>(
            "SELECT data FROM da_proposal2 WHERE payload_hash = $1 LIMIT 1",
        )
        .bind(req.0.to_string())
        .fetch_optional(tx.as_mut())
        .await
        {
            Ok(Some((bytes,))) => bytes,
            Ok(None) => return None,
            Err(err) => {
                tracing::warn!("error loading DA proposal: {err:#}");
                return None;
            },
        };

        let proposal: Proposal<SeqTypes, DaProposal2<SeqTypes>> = match bincode::deserialize(&bytes)
        {
            Ok(proposal) => proposal,
            Err(err) => {
                tracing::error!("error decoding DA proposal: {err:#}");
                return None;
            },
        };

        Some(Payload::from_bytes(
            &proposal.data.encoded_transactions,
            &proposal.data.metadata,
        ))
    }
}

#[cfg(test)]
mod testing {
    use super::{super::qs::testing::TmpDb, *};
    use crate::persistence::tests::TestablePersistence;

    #[async_trait]
    impl TestablePersistence for Persistence {
        type Storage = Arc<TmpDb>;

        const DELETES_VID_SHARES_ON_DECIDE: bool = false;

        async fn tmp_storage() -> Self::Storage {
            Arc::new(TmpDb::init().await)
        }

        #[allow(refining_impl_trait)]
        fn options(db: &Self::Storage) -> Options {
            Options(super::super::tmp_options(db))
        }
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use espresso_types::{Leaf, NodeState, PubKey, ValidatedState, traits::NullEventConsumer};
    use futures::stream::TryStreamExt;
    use hotshot_example_types::node_types::TEST_VERSIONS;
    use hotshot_types::{
        data::{
            EpochNumber, QuorumProposal2, ns_table::parse_ns_table,
            vid_disperse::AvidMDisperseShare,
        },
        message::convert_proposal,
        simple_certificate::QuorumCertificate,
        traits::{EncodeBytes, signature_key::SignatureKey},
        utils::EpochTransitionIndicator,
        vid::avidm::{AvidMScheme, init_avidm_param},
    };

    use super::*;
    use crate::{BLSPubKey, persistence::tests::TestablePersistence as _};

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_quorum_proposals_leaf_hash_migration() {
        // Create some quorum proposals to test with.
        let leaf: Leaf2 = Leaf::genesis(
            &ValidatedState::default(),
            &NodeState::mock(),
            TEST_VERSIONS.test.base,
        )
        .await
        .into();
        let privkey = BLSPubKey::generated_from_seed_indexed([0; 32], 1).1;
        let signature = PubKey::sign(&privkey, &[]).unwrap();
        let mut quorum_proposal = Proposal {
            data: QuorumProposal2::<SeqTypes> {
                epoch: None,
                block_header: leaf.block_header().clone(),
                view_number: ViewNumber::genesis(),
                justify_qc: QuorumCertificate::genesis(
                    &ValidatedState::default(),
                    &NodeState::mock(),
                    TEST_VERSIONS.test,
                )
                .await
                .to_qc2(),
                upgrade_certificate: None,
                view_change_evidence: None,
                next_drb_result: None,
                next_epoch_justify_qc: None,
                state_cert: None,
            },
            signature,
            _pd: Default::default(),
        };

        let qp1: Proposal<SeqTypes, QuorumProposal<SeqTypes>> =
            convert_proposal(quorum_proposal.clone());

        quorum_proposal.data.view_number = ViewNumber::new(1);

        let qp2: Proposal<SeqTypes, QuorumProposal<SeqTypes>> =
            convert_proposal(quorum_proposal.clone());
        let qps = [qp1, qp2];

        // Create persistence and add the quorum proposals with NULL leaf hash.
        let db = Persistence::tmp_storage().await;
        let persistence = Persistence::connect(&db).await;
        let mut tx = persistence.db.write().await.unwrap();
        let params = qps
            .iter()
            .map(|qp| {
                (
                    qp.data.view_number.u64() as i64,
                    bincode::serialize(&qp).unwrap(),
                )
            })
            .collect::<Vec<_>>();
        tx.upsert("quorum_proposals", ["view", "data"], ["view"], params)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        // Create a new persistence and ensure the commitments get populated.
        let persistence = Persistence::connect(&db).await;
        let mut tx = persistence.db.read().await.unwrap();
        let rows = tx
            .fetch("SELECT * FROM quorum_proposals ORDER BY view ASC")
            .try_collect::<Vec<_>>()
            .await
            .unwrap();
        assert_eq!(rows.len(), qps.len());
        for (row, qp) in rows.into_iter().zip(qps) {
            assert_eq!(row.get::<i64, _>("view"), qp.data.view_number.u64() as i64);
            assert_eq!(
                row.get::<Vec<u8>, _>("data"),
                bincode::serialize(&qp).unwrap()
            );
            assert_eq!(
                row.get::<String, _>("leaf_hash"),
                Committable::commit(&Leaf::from_quorum_proposal(&qp.data)).to_string()
            );
        }
    }

    fn pre_option_validator(
        seed: u8,
        stake: u64,
    ) -> crate::persistence::RegisteredValidatorPreOption {
        use std::collections::HashMap;

        use alloy::primitives::U256;
        use hotshot_types::light_client::StateVerKey;

        crate::persistence::RegisteredValidatorPreOption {
            account: Address::random(),
            stake_table_key: BLSPubKey::generated_from_seed_indexed([seed; 32], 0).0,
            state_ver_key: StateVerKey::default(),
            stake: U256::from(stake),
            commission: 0,
            delegators: HashMap::new(),
            authenticated: true,
            x25519_key: None,
            p2p_addr: None,
        }
    }

    async fn insert_legacy_stake_row(
        persistence: &Persistence,
        epoch: i64,
        validator: crate::persistence::RegisteredValidatorPreOption,
    ) {
        let mut map: IndexMap<Address, crate::persistence::RegisteredValidatorPreOption> =
            IndexMap::new();
        map.insert(validator.account, validator);
        let stake_bytes = bincode::serialize(&map).unwrap();
        let mut tx = persistence.db.write().await.unwrap();
        tx.execute(
            query("INSERT INTO epoch_drb_and_root (epoch, stake) VALUES ($1, $2)")
                .bind(epoch)
                .bind(&stake_bytes),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_load_stake_legacy_storage() {
        let tmp = Persistence::tmp_storage().await;
        let persistence = Persistence::connect(&tmp).await;

        let v1 = pre_option_validator(1, 100);
        let v2 = pre_option_validator(2, 200);
        let v1_addr = v1.account;
        let v2_addr = v2.account;
        insert_legacy_stake_row(&persistence, 1, v1).await;
        insert_legacy_stake_row(&persistence, 2, v2).await;

        let (loaded1, ..) = persistence
            .load_stake(EpochNumber::new(1))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded1.len(), 1);
        assert!(loaded1.get(&v1_addr).unwrap().stake_table_key.is_some());

        let (loaded2, ..) = persistence
            .load_stake(EpochNumber::new(2))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded2.len(), 1);
        assert!(loaded2.get(&v2_addr).unwrap().stake_table_key.is_some());

        let latest = persistence.load_latest_stake(10).await.unwrap().unwrap();
        assert_eq!(latest.len(), 2);
        let epochs: Vec<_> = latest.iter().map(|(e, ..)| *e).collect();
        assert!(epochs.contains(&EpochNumber::new(1)));
        assert!(epochs.contains(&EpochNumber::new(2)));
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_load_stake_mixed_storage() {
        let tmp = Persistence::tmp_storage().await;
        let persistence = Persistence::connect(&tmp).await;

        let legacy_v = pre_option_validator(3, 300);
        let legacy_addr = legacy_v.account;
        insert_legacy_stake_row(&persistence, 5, legacy_v).await;

        let current_v = espresso_types::v0_3::AuthenticatedValidator::mock();
        let current_addr = current_v.account;
        let mut current_map = IndexMap::new();
        current_map.insert(current_addr, current_v);
        persistence
            .store_stake(EpochNumber::new(6), current_map, None, None)
            .await
            .unwrap();

        let latest = persistence.load_latest_stake(10).await.unwrap().unwrap();
        assert_eq!(latest.len(), 2);
        let by_epoch: std::collections::HashMap<_, _> = latest
            .into_iter()
            .map(|(e, (map, _), _)| (e, map))
            .collect();
        assert!(
            by_epoch
                .get(&EpochNumber::new(5))
                .unwrap()
                .contains_key(&legacy_addr)
        );
        assert!(
            by_epoch
                .get(&EpochNumber::new(6))
                .unwrap()
                .contains_key(&current_addr)
        );
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_store_all_validators_authenticated_and_unauthenticated() {
        use std::collections::HashMap;

        use alloy::primitives::{Address, U256};
        use hotshot_types::light_client::StateVerKey;
        use indexmap::IndexMap;

        let tmp = Persistence::tmp_storage().await;
        let storage = Persistence::connect(&tmp).await;

        // Create an authenticated validator
        let authenticated_validator = RegisteredValidator {
            account: Address::random(),
            stake_table_key: Some(BLSPubKey::generated_from_seed_indexed([0u8; 32], 0).0),
            state_ver_key: Some(StateVerKey::default()),
            stake: U256::from(1000),
            commission: 100,
            delegators: HashMap::new(),
            authenticated: true,
            x25519_key: None,
            p2p_addr: None,
        };

        // Create an unauthenticated validator
        let unauthenticated_validator = RegisteredValidator {
            account: Address::random(),
            stake_table_key: Some(BLSPubKey::generated_from_seed_indexed([0u8; 32], 1).0),
            state_ver_key: Some(StateVerKey::default()),
            stake: U256::from(2000),
            commission: 200,
            delegators: HashMap::new(),
            authenticated: false,
            x25519_key: None,
            p2p_addr: None,
        };

        let mut validators: IndexMap<Address, RegisteredValidator<BLSPubKey>> = IndexMap::new();
        validators.insert(
            authenticated_validator.account,
            authenticated_validator.clone(),
        );
        validators.insert(
            unauthenticated_validator.account,
            unauthenticated_validator.clone(),
        );

        // Store both validators
        storage
            .store_all_validators(EpochNumber::new(1), validators)
            .await
            .unwrap();

        // Load and verify
        let loaded = storage
            .load_all_validators(EpochNumber::new(1), 0, 100)
            .await
            .unwrap();
        assert_eq!(loaded.len(), 2);

        // Find each validator and verify authenticated state is preserved
        let loaded_auth = loaded
            .iter()
            .find(|v| v.account == authenticated_validator.account)
            .unwrap();
        assert!(
            loaded_auth.authenticated,
            "authenticated validator should remain authenticated"
        );

        let loaded_unauth = loaded
            .iter()
            .find(|v| v.account == unauthenticated_validator.account)
            .unwrap();
        assert!(
            !loaded_unauth.authenticated,
            "unauthenticated validator should remain unauthenticated"
        );
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_fetching_providers() {
        let tmp = Persistence::tmp_storage().await;
        let storage = Persistence::connect(&tmp).await;

        // Mock up some data.
        let leaf = Leaf2::genesis(
            &ValidatedState::default(),
            &NodeState::mock(),
            TEST_VERSIONS.test.base,
        )
        .await;
        let leaf_payload = leaf.block_payload().unwrap();
        let leaf_payload_bytes_arc = leaf_payload.encode();

        let avidm_param = init_avidm_param(2).unwrap();
        let weights = vec![1u32; 2];

        let ns_table = parse_ns_table(
            leaf_payload.byte_len().as_usize(),
            &leaf_payload.ns_table().encode(),
        );
        let (payload_commitment, shares) =
            AvidMScheme::ns_disperse(&avidm_param, &weights, &leaf_payload_bytes_arc, ns_table)
                .unwrap();
        let (pubkey, privkey) = BLSPubKey::generated_from_seed_indexed([0; 32], 1);
        let vid_share = convert_proposal(
            AvidMDisperseShare::<SeqTypes> {
                view_number: ViewNumber::new(0),
                payload_commitment,
                share: shares[0].clone(),
                recipient_key: pubkey,
                epoch: None,
                target_epoch: None,
                common: avidm_param.clone(),
            }
            .to_proposal(&privkey)
            .unwrap()
            .clone(),
        );

        let quorum_proposal = QuorumProposalWrapper::<SeqTypes> {
            proposal: QuorumProposal2::<SeqTypes> {
                block_header: leaf.block_header().clone(),
                view_number: leaf.view_number(),
                justify_qc: leaf.justify_qc().clone(),
                upgrade_certificate: None,
                view_change_evidence: None,
                next_drb_result: None,
                next_epoch_justify_qc: None,
                epoch: None,
                state_cert: None,
            },
        };
        let quorum_proposal_signature =
            BLSPubKey::sign(&privkey, &bincode::serialize(&quorum_proposal).unwrap())
                .expect("Failed to sign quorum proposal");
        let quorum_proposal = Proposal {
            data: quorum_proposal,
            signature: quorum_proposal_signature,
            _pd: Default::default(),
        };

        let block_payload_signature = BLSPubKey::sign(&privkey, &leaf_payload_bytes_arc)
            .expect("Failed to sign block payload");
        let da_proposal = Proposal {
            data: DaProposal2::<SeqTypes> {
                encoded_transactions: leaf_payload_bytes_arc,
                metadata: leaf_payload.ns_table().clone(),
                view_number: ViewNumber::new(0),
                epoch: None,
                epoch_transition_indicator: EpochTransitionIndicator::NotInTransition,
            },
            signature: block_payload_signature,
            _pd: Default::default(),
        };

        let mut next_quorum_proposal = quorum_proposal.clone();
        next_quorum_proposal.data.proposal.view_number += 1;
        next_quorum_proposal.data.proposal.justify_qc.view_number += 1;
        next_quorum_proposal
            .data
            .proposal
            .justify_qc
            .data
            .leaf_commit = Committable::commit(&leaf.clone());

        // Add to database.
        storage
            .append_da2(&da_proposal, VidCommitment::V1(payload_commitment))
            .await
            .unwrap();
        storage.append_vid(&vid_share).await.unwrap();
        storage
            .append_quorum_proposal2(&quorum_proposal)
            .await
            .unwrap();

        // Add an extra quorum proposal so we have a QC pointing back at `leaf`.
        storage
            .append_quorum_proposal2(&next_quorum_proposal)
            .await
            .unwrap();

        // Fetch it as if we were rebuilding an archive.
        assert_eq!(
            Some(VidCommon::V1(avidm_param)),
            storage
                .fetch(VidCommonRequest(vid_share.data.payload_commitment()))
                .await
        );
        assert_eq!(
            leaf_payload,
            storage
                .fetch(PayloadRequest(vid_share.data.payload_commitment()))
                .await
                .unwrap()
        );
    }

    /// Test conditions that trigger pruning.
    ///
    /// This is a configurable test that can be used to test different configurations of GC,
    /// `pruning_opt`. The test populates the database with some data for view 1, asserts that it is
    /// retained for view 2, and then asserts that it is pruned by view 3. There are various
    /// different configurations that can achieve this behavior, such that the data is retained and
    /// then pruned due to different logic and code paths.
    async fn test_pruning_helper(pruning_opt: ConsensusPruningOptions) {
        let tmp = Persistence::tmp_storage().await;
        let mut opt = Persistence::options(&tmp);
        opt.consensus_pruning = pruning_opt;
        let storage = opt.create().await.unwrap();

        let data_view = ViewNumber::new(1);

        // Populate some data.
        let leaf = Leaf2::genesis(
            &ValidatedState::default(),
            &NodeState::mock(),
            TEST_VERSIONS.test.base,
        )
        .await;
        let leaf_payload = leaf.block_payload().unwrap();
        let leaf_payload_bytes_arc = leaf_payload.encode();

        let avidm_param = init_avidm_param(2).unwrap();
        let weights = vec![1u32; 2];

        let ns_table = parse_ns_table(
            leaf_payload.byte_len().as_usize(),
            &leaf_payload.ns_table().encode(),
        );
        let (payload_commitment, shares) =
            AvidMScheme::ns_disperse(&avidm_param, &weights, &leaf_payload_bytes_arc, ns_table)
                .unwrap();

        let (pubkey, privkey) = BLSPubKey::generated_from_seed_indexed([0; 32], 1);
        let vid = convert_proposal(
            AvidMDisperseShare::<SeqTypes> {
                view_number: data_view,
                payload_commitment,
                share: shares[0].clone(),
                recipient_key: pubkey,
                epoch: None,
                target_epoch: None,
                common: avidm_param,
            }
            .to_proposal(&privkey)
            .unwrap()
            .clone(),
        );
        let quorum_proposal = QuorumProposalWrapper::<SeqTypes> {
            proposal: QuorumProposal2::<SeqTypes> {
                epoch: None,
                block_header: leaf.block_header().clone(),
                view_number: data_view,
                justify_qc: QuorumCertificate2::genesis(
                    &ValidatedState::default(),
                    &NodeState::mock(),
                    TEST_VERSIONS.test,
                )
                .await,
                upgrade_certificate: None,
                view_change_evidence: None,
                next_drb_result: None,
                next_epoch_justify_qc: None,
                state_cert: None,
            },
        };
        let quorum_proposal_signature =
            BLSPubKey::sign(&privkey, &bincode::serialize(&quorum_proposal).unwrap())
                .expect("Failed to sign quorum proposal");
        let quorum_proposal = Proposal {
            data: quorum_proposal,
            signature: quorum_proposal_signature,
            _pd: Default::default(),
        };

        let block_payload_signature = BLSPubKey::sign(&privkey, &leaf_payload_bytes_arc)
            .expect("Failed to sign block payload");
        let da_proposal = Proposal {
            data: DaProposal2::<SeqTypes> {
                encoded_transactions: leaf_payload_bytes_arc.clone(),
                metadata: leaf_payload.ns_table().clone(),
                view_number: data_view,
                epoch: Some(EpochNumber::new(0)),
                epoch_transition_indicator: EpochTransitionIndicator::NotInTransition,
            },
            signature: block_payload_signature,
            _pd: Default::default(),
        };

        tracing::info!(?vid, ?da_proposal, ?quorum_proposal, "append data");
        storage.append_vid(&vid).await.unwrap();
        storage
            .append_da2(&da_proposal, VidCommitment::V1(payload_commitment))
            .await
            .unwrap();
        storage
            .append_quorum_proposal2(&quorum_proposal)
            .await
            .unwrap();

        // Populate the view-indexed cert tables. Contents are opaque to the pruner (it deletes by
        // `view`), so raw bytes suffice.
        {
            let mut tx = storage.db.write().await.unwrap();
            tx.upsert(
                "state_cert",
                ["view", "state_cert"],
                ["view"],
                [(data_view.u64() as i64, b"state_cert".to_vec())],
            )
            .await
            .unwrap();
            tx.upsert(
                "decided_cert2",
                ["view", "data"],
                ["view"],
                [(data_view.u64() as i64, b"cert2".to_vec())],
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
        }

        // The first decide doesn't trigger any garbage collection, even though our usage exceeds
        // the target, because of the minimum retention.
        tracing::info!("decide view 1");
        storage
            .append_decided_leaves(data_view + 1, [], &NullEventConsumer)
            .await
            .unwrap();
        assert_eq!(
            storage.load_vid_share(data_view).await.unwrap().unwrap(),
            vid
        );
        assert_eq!(
            storage.load_da_proposal(data_view).await.unwrap().unwrap(),
            da_proposal
        );
        assert_eq!(
            storage.load_quorum_proposal(data_view).await.unwrap(),
            quorum_proposal
        );
        assert!(view_row_exists(&storage, "state_cert", data_view).await);
        assert!(view_row_exists(&storage, "decided_cert2", data_view).await);

        // After another view, our data is beyond the minimum retention (though not the target
        // retention) so it gets pruned.
        tracing::info!("decide view 2");
        storage
            .append_decided_leaves(data_view + 2, [], &NullEventConsumer)
            .await
            .unwrap();
        assert!(storage.load_vid_share(data_view).await.unwrap().is_none(),);
        assert!(storage.load_da_proposal(data_view).await.unwrap().is_none());
        storage.load_quorum_proposal(data_view).await.unwrap_err();
        assert!(!view_row_exists(&storage, "state_cert", data_view).await);
        assert!(!view_row_exists(&storage, "decided_cert2", data_view).await);
    }

    /// Whether a view-indexed consensus table has a row at `view`.
    async fn view_row_exists(storage: &Persistence, table: &str, view: ViewNumber) -> bool {
        storage
            .db
            .read()
            .await
            .unwrap()
            .fetch_optional(
                query(&format!("SELECT view FROM {table} WHERE view = $1")).bind(view.u64() as i64),
            )
            .await
            .unwrap()
            .is_some()
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_pruning_minimum_retention() {
        test_pruning_helper(ConsensusPruningOptions {
            // Use a very low target usage, to show that we still retain data up to the minimum
            // retention even when usage is above target.
            target_usage: 0,
            minimum_retention: 1,
            // Use a very high target retention, so that pruning is only triggered by the minimum
            // retention.
            target_retention: u64::MAX,
            usage_check_interval: Duration::ZERO,
            vid_share_retention: 1,
        })
        .await
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_pruning_target_retention() {
        test_pruning_helper(ConsensusPruningOptions {
            target_retention: 1,
            // Use a very low minimum retention, so that data is only kept around due to the target
            // retention.
            minimum_retention: 0,
            // Use a very high target usage, so that pruning is only triggered by the target
            // retention.
            target_usage: u64::MAX,
            usage_check_interval: Duration::ZERO,
            vid_share_retention: 1,
        })
        .await
    }

    #[rstest::rstest]
    #[case::decide_events(false)]
    #[case::consensus_only(true)]
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_vid_share_retention(#[case] consensus_only: bool) {
        let tmp = Persistence::tmp_storage().await;
        let mut opt = Persistence::options(&tmp);
        opt.consensus_pruning = ConsensusPruningOptions {
            target_retention: 0,
            minimum_retention: 0,
            target_usage: 0,
            usage_check_interval: Duration::ZERO,
            vid_share_retention: 2,
        };
        if consensus_only {
            opt.set_consensus_only();
        }
        let storage = opt.create().await.unwrap();

        let leaf = Leaf2::genesis(
            &ValidatedState::default(),
            &NodeState::mock(),
            TEST_VERSIONS.test.base,
        )
        .await;
        let qc = QuorumCertificate2::genesis(
            &ValidatedState::default(),
            &NodeState::mock(),
            TEST_VERSIONS.test,
        )
        .await;
        let payload = leaf.block_payload().unwrap();
        let avidm_param = init_avidm_param(2).unwrap();
        let ns_table = parse_ns_table(payload.byte_len().as_usize(), &payload.ns_table().encode());
        let (payload_commitment, shares) =
            AvidMScheme::ns_disperse(&avidm_param, &[1, 1], &payload.encode(), ns_table).unwrap();
        let (pubkey, privkey) = BLSPubKey::generated_from_seed_indexed([0; 32], 1);
        let vid = convert_proposal(
            AvidMDisperseShare::<SeqTypes> {
                view_number: leaf.view_number(),
                payload_commitment,
                share: shares[0].clone(),
                recipient_key: pubkey,
                epoch: None,
                target_epoch: None,
                common: avidm_param,
            }
            .to_proposal(&privkey)
            .unwrap(),
        );
        storage.append_vid(&vid).await.unwrap();

        let view = leaf.view_number();
        let info = LeafInfo {
            leaf,
            vid_share: None,
            state: Default::default(),
            delta: None,
            state_cert: None,
        };
        storage
            .append_decided_leaves(
                view,
                [(&info, CertificatePair::non_epoch_change(qc))],
                &NullEventConsumer,
            )
            .await
            .unwrap();
        assert_eq!(storage.load_vid_share(view).await.unwrap(), Some(vid));

        storage
            .append_decided_leaves(view + 2, [], &NullEventConsumer)
            .await
            .unwrap();
        assert!(!view_row_exists(&storage, "anchor_leaf2", view).await);
        assert!(view_row_exists(&storage, "vid_share2", view).await);

        storage
            .append_decided_leaves(view + 3, [], &NullEventConsumer)
            .await
            .unwrap();
        assert!(!view_row_exists(&storage, "vid_share2", view).await);
    }

    /// Regression test for an ambiguous behavior in `store_events`/`load_events`.
    ///
    /// Previously, `store_events` did nothing when given an empty events list (in fact,
    /// `fetch_and_store_stake_table_events` was not even calling it). But this means that the
    /// `stake_table_events_l1_block` column does not get updated when we enter a new epoch with no
    /// new stake table events. This makes it impossible to distinguish between two very different
    /// scenarios:
    ///
    /// 1. The node has successfully processed events through the latest L1 finalized block, but
    ///    there are no new events from the last epoch.
    /// 2. The node is lagging behind the latest L1 finalized block, and is possibly missing some
    ///    new events.
    ///
    /// In scenario 1, clients of this node should be able to treat the empty list of stake table
    /// events as authoritative, and derive the stake table for the next epoch (which will end up
    /// being the same as the previous one. However, in scenario 2, clients need to wait, because we
    /// don't yet know whether there could be any events that modify the stake table. Thus,
    /// distinguishing these two scenarios is important.
    ///
    /// This regression test ensures that even if there are no new events, at least the
    /// `stake_table_events_l1_block` column gets updated. We can then distinguish the two scenarios
    /// using the `EventsPersistenceRead`` return value from load_events.
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_store_events_empty() {
        let tmp = Persistence::tmp_storage().await;
        let mut opt = Persistence::options(&tmp);
        let storage = opt.create().await.unwrap();

        assert_eq!(storage.load_events(0, 100).await.unwrap(), (None, vec![]));

        // Storing an empty events list still updates the latest L1 block.
        for i in 1..=2 {
            tracing::info!(i, "update l1 height");
            storage.store_events(i, vec![]).await.unwrap();
            assert_eq!(
                storage.load_events(0, 100).await.unwrap(),
                (Some(EventsPersistenceRead::UntilL1Block(i)), vec![])
            );
        }
    }

    /// A gap-fill decide reports the older view it filled, after a newer decide already moved
    /// the cursor past it.
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_consensus_only_decide_never_rewinds_cursor() {
        let tmp = Persistence::tmp_storage().await;
        let mut opt = Persistence::options(&tmp);
        opt.set_consensus_only();
        let storage = opt.create().await.unwrap();

        for view in [10, 5] {
            storage
                .append_decided_leaves(ViewNumber::new(view), [], &NullEventConsumer)
                .await
                .unwrap();
        }
        assert_eq!(
            storage.load_processed_view().await.unwrap(),
            Some(ViewNumber::new(10))
        );
    }
}
