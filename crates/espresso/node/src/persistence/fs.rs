#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    ops::{Range, RangeInclusive},
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

use alloy::primitives::Address;
use anyhow::{Context, anyhow, bail};
use async_lock::RwLock;
use async_trait::async_trait;
use clap::Parser;
use espresso_types::{
    AuthenticatedValidatorMap, Header, Leaf2, NetworkConfig, Payload, PubKey,
    RegisteredValidatorMap, SeqTypes, StakeTableHash,
    traits::{EventsPersistenceRead, MembershipPersistence, StakeTuple},
    v0::traits::{EventConsumer, PersistenceOptions, SequencerPersistence},
    v0_3::{
        AuthenticatedValidator, EventKey, IndexedStake, RegisteredValidator, RewardAmount,
        StakeTableEvent,
    },
};
use hotshot::InitializerEpochInfo;
use hotshot_libp2p_networking::network::behaviours::dht::store::persistent::{
    DhtPersistentStorage, SerializableRecord,
};
use hotshot_new_protocol::message::Certificate2;
use hotshot_types::{
    data::{
        DaProposal, DaProposal2, EpochNumber, QuorumProposalWrapper, QuorumProposalWrapperLegacy,
        VidCommitment, VidDisperseShare,
    },
    drb::{DrbInput, DrbResult},
    event::{Event, EventType, HotShotAction, LeafInfo},
    message::{Proposal, convert_proposal},
    new_protocol::CoordinatorEvent,
    simple_certificate::{
        CertificatePair, LightClientStateUpdateCertificateV1, LightClientStateUpdateCertificateV2,
        NextEpochQuorumCertificate2, QuorumCertificate2, UpgradeCertificate,
    },
    traits::{
        block_contents::{BlockHeader, BlockPayload},
        metrics::Metrics,
        node_implementation::NodeType,
    },
    vote::HasViewNumber,
};
use itertools::Itertools;

use super::{
    RegisteredValidatorNoX25519, RegisteredValidatorPreOption, RegisteredValidatorPreSchnorrOption,
};
use crate::{
    RECENT_STAKE_TABLES_LIMIT, ViewNumber,
    persistence::{migrate_network_config, persistence_metrics::PersistenceMetricsValue},
};

/// Deserialize a stake table from bytes, trying current and legacy formats.
/// Returns (stake_tuple, needs_rewrite) where needs_rewrite=true means legacy format was used.
fn deserialize_stake_table(bytes: &[u8]) -> anyhow::Result<(StakeTuple, bool)> {
    // Try current format (both keys as Option<KEY>).
    if let Ok(stake) = bincode::deserialize::<StakeTuple>(bytes) {
        return Ok((stake, false));
    }

    // Pre-Schnorr-Option: stake_table_key as Option<KEY>, state_ver_key as raw KEY.
    type PreSchnorrMap = indexmap::IndexMap<Address, RegisteredValidatorPreSchnorrOption>;
    type PreSchnorrTuple = (PreSchnorrMap, Option<RewardAmount>, Option<StakeTableHash>);
    if let Ok(pre_schnorr) = bincode::deserialize::<PreSchnorrTuple>(bytes) {
        let migrated: AuthenticatedValidatorMap = pre_schnorr
            .0
            .into_iter()
            .map(|(addr, v)| {
                let registered = v.migrate();
                let authenticated = AuthenticatedValidator::try_from(registered)?;
                Ok((addr, authenticated))
            })
            .collect::<anyhow::Result<_>>()?;
        return Ok(((migrated, pre_schnorr.1, pre_schnorr.2), true));
    }

    // Pre-Option: both keys raw, x25519/p2p fields present.
    type PreOptionMap = indexmap::IndexMap<Address, RegisteredValidatorPreOption>;
    type PreOptionTuple = (PreOptionMap, Option<RewardAmount>, Option<StakeTableHash>);
    if let Ok(pre_option) = bincode::deserialize::<PreOptionTuple>(bytes) {
        let migrated: AuthenticatedValidatorMap = pre_option
            .0
            .into_iter()
            .map(|(addr, v)| {
                let registered = v.migrate();
                let authenticated = AuthenticatedValidator::try_from(registered)?;
                Ok((addr, authenticated))
            })
            .collect::<anyhow::Result<_>>()?;
        return Ok(((migrated, pre_option.1, pre_option.2), true));
    }

    // Pre-x25519: RegisteredValidator without x25519_key/p2p_addr.
    type LegacyMap = indexmap::IndexMap<Address, RegisteredValidatorNoX25519>;
    type LegacyTuple = (LegacyMap, Option<RewardAmount>, Option<StakeTableHash>);
    let legacy: LegacyTuple = bincode::deserialize(bytes)
        .context("failed to deserialize stake table (tried current and legacy formats)")?;
    let migrated: AuthenticatedValidatorMap = legacy
        .0
        .into_iter()
        .map(|(addr, v)| {
            let registered = v.migrate();
            (
                addr,
                AuthenticatedValidator::try_from(registered)
                    .expect("stake tables only contain authenticated validators"),
            )
        })
        .collect();
    Ok(((migrated, legacy.1, legacy.2), true))
}

/// Options for file system backed persistence.
#[derive(Parser, Clone, Debug)]
pub struct Options {
    /// Storage path for persistent data.
    #[clap(long, env = "ESPRESSO_NODE_STORAGE_PATH")]
    pub(crate) path: PathBuf,

    /// Number of views to retain in consensus storage before data that hasn't been archived is
    /// garbage collected.
    ///
    /// The longer this is, the more certain that all data will eventually be archived, even if
    /// there are temporary problems with archive storage or partially missing data. This can be set
    /// very large, as most data is garbage collected as soon as it is finalized by consensus. This
    /// setting only applies to views which never get decided (ie forks in consensus) and views for
    /// which this node is partially offline. These should be exceptionally rare.
    ///
    /// The default of 130000 views equates to approximately 3 days (259200 seconds) at an average
    /// view time of 2s.
    #[clap(
        long,
        env = "ESPRESSO_NODE_CONSENSUS_VIEW_RETENTION",
        default_value = "130000"
    )]
    pub(crate) consensus_view_retention: u64,
}

impl Default for Options {
    fn default() -> Self {
        Self::parse_from(std::iter::empty::<String>())
    }
}

impl Options {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            consensus_view_retention: 130000,
        }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

#[async_trait]
impl PersistenceOptions for Options {
    type Persistence = Persistence;

    fn set_view_retention(&mut self, view_retention: u64) {
        self.consensus_view_retention = view_retention;
    }

    async fn create(&mut self) -> anyhow::Result<Self::Persistence> {
        let path = self.path.clone();
        let view_retention = self.consensus_view_retention;

        Ok(Persistence {
            inner: Arc::new(RwLock::new(Inner {
                path,
                view_retention,
                gc_floor: None,
                #[cfg(test)]
                view_files_scans: AtomicUsize::new(0),
            })),
            metrics: Arc::new(PersistenceMetricsValue::default()),
        })
    }

    async fn reset(self) -> anyhow::Result<()> {
        todo!()
    }
}

/// Caps both how far the retention floor advances in one decide and how many views of an
/// interval's span are unlinked in one pass, so a large jump in the decided view cannot stall
/// consensus behind one GC pass.
const MAX_UNLINK_VIEWS_PER_PASS: u64 = 1024;

/// File system backed persistence.
#[derive(Clone, Debug)]
pub struct Persistence {
    // We enforce mutual exclusion on access to the data source, as the current file system
    // implementation does not support transaction isolation for concurrent reads and writes. We can
    // improve this in the future by switching to a SQLite-based file system implementation.
    inner: Arc<RwLock<Inner>>,
    /// A reference to the metrics trait
    metrics: Arc<PersistenceMetricsValue>,
}

#[derive(Debug)]
struct Inner {
    path: PathBuf,
    view_retention: u64,
    /// Views below this have already been swept by retention. `None` until the one-time
    /// startup sweep arms it.
    gc_floor: Option<ViewNumber>,
    /// Number of times `view_files` has scanned a directory on this instance. Test-only, to
    /// assert GC no longer does a `read_dir` per decide once the retention floor is armed.
    #[cfg(test)]
    view_files_scans: AtomicUsize,
}

/// One decided leaf, hydrated from storage and ready to emit.
struct PendingDecide {
    view: ViewNumber,
    height: u64,
    event: CoordinatorEvent<SeqTypes>,
}

impl Inner {
    #[cfg(test)]
    fn view_files_scans(&self) -> usize {
        self.view_files_scans.load(Ordering::SeqCst)
    }

    /// Wraps the free `view_files` scan, counting it under `cfg(test)` so tests can assert GC
    /// no longer does a `read_dir` per decide once the retention floor is armed.
    ///
    /// Captures only `T`, not `&self`: callers hold the returned iterator across further
    /// `&mut self` calls (e.g. `store_finalized_state_cert`), which an implicit `&self` capture
    /// under Rust 2024's default `impl Trait` lifetime rules would forbid.
    fn view_files<T: AsRef<Path>>(
        &self,
        dir: T,
    ) -> anyhow::Result<impl Iterator<Item = (ViewNumber, PathBuf)> + use<T>> {
        #[cfg(test)]
        self.view_files_scans.fetch_add(1, Ordering::SeqCst);
        view_files(dir)
    }

    fn config_path(&self) -> PathBuf {
        self.path.join("hotshot.cfg")
    }

    fn voted_view_path(&self) -> PathBuf {
        self.path.join("highest_voted_view")
    }

    fn restart_view_path(&self) -> PathBuf {
        self.path.join("restart_view")
    }

    fn decided_leaf2_path(&self) -> PathBuf {
        self.path.join("decided_leaves2")
    }

    /// The path from previous versions where there was only a single file for anchor leaves.
    fn legacy_anchor_leaf_path(&self) -> PathBuf {
        self.path.join("anchor_leaf")
    }

    fn vid2_dir_path(&self) -> PathBuf {
        self.path.join("vid2")
    }

    fn da_dir_path(&self) -> PathBuf {
        self.path.join("da")
    }

    fn drb_dir_path(&self) -> PathBuf {
        self.path.join("drb")
    }

    fn da2_dir_path(&self) -> PathBuf {
        self.path.join("da2")
    }

    fn quorum_proposals2_dir_path(&self) -> PathBuf {
        self.path.join("quorum_proposals2")
    }

    fn upgrade_certificate_dir_path(&self) -> PathBuf {
        self.path.join("upgrade_certificate")
    }

    fn stake_table_dir_path(&self) -> PathBuf {
        self.path.join("stake_table")
    }

    fn next_epoch_qc(&self) -> PathBuf {
        self.path.join("next_epoch_quorum_certificate")
    }

    fn eqc(&self) -> PathBuf {
        self.path.join("eqc")
    }

    fn high_qc2(&self) -> PathBuf {
        self.path.join("high_qc2")
    }

    fn libp2p_dht_path(&self) -> PathBuf {
        self.path.join("libp2p_dht")
    }
    fn epoch_drb_result_dir_path(&self) -> PathBuf {
        self.path.join("epoch_drb_result")
    }

    fn epoch_root_block_header_dir_path(&self) -> PathBuf {
        self.path.join("epoch_root_block_header")
    }

    fn finalized_state_cert_dir_path(&self) -> PathBuf {
        self.path.join("finalized_state_cert")
    }

    fn state_cert_dir_path(&self) -> PathBuf {
        self.path.join("state_cert")
    }

    fn decided_cert2_dir_path(&self) -> PathBuf {
        self.path.join("decided_cert2")
    }

    /// cert2 is only persisted for the view that is directly finalized
    /// (the newest leaf in a decided chain). Ancestor views finalized
    /// indirectly have no cert2 file on disk
    /// for those, this returns `Ok(None)`
    fn load_cert2(&self, view: ViewNumber) -> anyhow::Result<Option<Certificate2<SeqTypes>>> {
        let file_path = self
            .decided_cert2_dir_path()
            .join(view.u64().to_string())
            .with_extension("bin");
        if !file_path.is_file() {
            return Ok(None);
        }
        let bytes = fs::read(&file_path).context("read cert2")?;
        Ok(Some(
            bincode::deserialize(&bytes).context("deserialize cert2")?,
        ))
    }

    /// Overwrite a file if a condition is met.
    ///
    /// The file at `path`, if it exists, is opened in read mode and passed to `pred`. If `pred`
    /// returns `true`, or if there was no existing file, then `write` is called to update the
    /// contents of the file. `write` receives a truncated file open in write mode and sets the
    /// contents of the file.
    ///
    /// The final replacement of the original file is atomic; that is, `path` will be modified only
    /// if the entire update succeeds.
    fn replace(
        &mut self,
        path: &Path,
        pred: impl FnOnce(File) -> anyhow::Result<bool>,
        write: impl FnOnce(File) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        if path.is_file() {
            // If there is an existing file, check if it is suitable to replace. Note that this
            // check is not atomic with respect to the subsequent write at the file system level,
            // but this object is the only one which writes to this file, and we have a mutable
            // reference, so this should be safe.
            if !pred(File::open(path)?)? {
                // If we are not overwriting the file, we are done and consider the whole operation
                // successful.
                return Ok(());
            }
        }

        // Either there is no existing file or we have decided to overwrite the file. Write the new
        // contents into a temporary file so we can update `path` atomically using `rename`.
        let mut swap_path = path.to_owned();
        swap_path.set_extension("swp");
        let swap = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&swap_path)?;
        write(swap)?;

        // Now we can replace the original file.
        fs::rename(swap_path, path)?;

        Ok(())
    }

    /// The five directories GC prunes by view span: retention floor and interval unlink agree
    /// on this set, so the two cannot disagree.
    fn span_dirs(&self) -> [(PathBuf, &'static str); 5] {
        [
            (self.da2_dir_path(), "txt"),
            (self.vid2_dir_path(), "txt"),
            (self.quorum_proposals2_dir_path(), "txt"),
            (self.state_cert_dir_path(), "txt"),
            (self.decided_cert2_dir_path(), "bin"),
        ]
    }

    /// `span_dirs` plus `decided_leaves2` last. Used by the retention floor, which is a
    /// low-priority safety net over every directory, and by the one-time startup sweep.
    fn pruned_dirs(&self) -> [(PathBuf, &'static str); 6] {
        let [d0, d1, d2, d3, d4] = self.span_dirs();
        [d0, d1, d2, d3, d4, (self.decided_leaf2_path(), "txt")]
    }

    /// `decided_view` predicates this pass by the leaves actually read in phase 1, so a
    /// concurrent `persist_decided_leaves` writing views above `decided_view` cannot race it;
    /// see `SequencerPersistence::process_decided_events`.
    fn collect_garbage(
        &mut self,
        decided_view: ViewNumber,
        prune_intervals: &[RangeInclusive<ViewNumber>],
        emitted: &[ViewNumber],
    ) -> anyhow::Result<()> {
        let prune_view = ViewNumber::new(decided_view.saturating_sub(self.view_retention));
        let mut failures = 0usize;

        if self.gc_floor.is_none() && self.sweep_all(prune_view, decided_view).is_err() {
            failures += 1;
        }

        if let Some(floor) = self.gc_floor {
            let mut swept_to = floor;
            for v in sweep_range(floor, prune_view, MAX_UNLINK_VIEWS_PER_PASS) {
                if self
                    .remove_view_files(ViewNumber::new(v), decided_view)
                    .is_err()
                {
                    // Stop advancing the floor at the first failure so the next decide retries
                    // this view; the loop otherwise keeps going, since later views are unrelated.
                    failures += 1;
                    break;
                }
                swept_to = ViewNumber::new(v + 1);
            }
            self.gc_floor = Some(swept_to);
        }

        // The exact views processed this pass, minus the anchor, are pruned from
        // `decided_leaves2` directly. Capping this to the span cap like the other five
        // directories would leave leaf files behind that `load_pending_decides` re-reads and
        // re-emits every subsequent decide.
        let leaves_dir = self.decided_leaf2_path();
        for &view in emitted {
            if view == decided_view {
                continue;
            }
            if let Err(err) = unlink_view(&leaves_dir, view, "txt") {
                tracing::warn!(?view, dir = %leaves_dir.display(), "GC: failed to prune: {err:#}");
                failures += 1;
            }
        }

        // The remaining five directories: unlink every view in each interval's integer span,
        // capped per pass. A span left partly uncollected costs disk only; the retention floor
        // collects it within `view_retention`. Per directory, stop at the first failure rather
        // than retrying every remaining view: a permanently broken directory would otherwise log
        // once per file, every decide.
        let span_dirs = self.span_dirs();
        for interval in prune_intervals {
            let start = interval.start().u64();
            let end = interval
                .end()
                .u64()
                .min(start.saturating_add(MAX_UNLINK_VIEWS_PER_PASS - 1));
            for (dir, ext) in &span_dirs {
                for v in start..=end {
                    if let Err(err) = unlink_view(dir, ViewNumber::new(v), ext) {
                        tracing::warn!(dir = %dir.display(), ?interval, "GC: failed to prune: {err:#}");
                        failures += 1;
                        break;
                    }
                }
            }
        }

        if failures == 0 {
            Ok(())
        } else {
            bail!("GC failed to prune {failures} file(s)");
        }
    }

    /// One full scan of every view directory: delete below `prune_view`, then arm `gc_floor`.
    /// Runs once per process. If any directory fails, `gc_floor` is left `None` so the next
    /// decide retries the full sweep; arming it unconditionally would leak that directory's
    /// below-floor files for the process lifetime.
    fn sweep_all(&mut self, prune_view: ViewNumber, anchor: ViewNumber) -> anyhow::Result<()> {
        let [
            da2,
            vid2,
            quorum_proposals2,
            state_cert,
            decided_cert2,
            decided_leaves2,
        ] = self.pruned_dirs();
        let total = self.pruned_dirs().len();
        let mut failures = 0usize;
        for (dir, _) in [da2, vid2, quorum_proposals2, state_cert, decided_cert2] {
            if let Err(err) = self.prune_files(dir.clone(), prune_view, None) {
                tracing::warn!(dir = %dir.display(), "GC: startup sweep failed: {err:#}");
                failures += 1;
            }
        }
        let (leaves_dir, _) = decided_leaves2;
        if let Err(err) = self.prune_files(leaves_dir.clone(), prune_view, Some(anchor)) {
            tracing::warn!(dir = %leaves_dir.display(), "GC: startup sweep failed: {err:#}");
            failures += 1;
        }

        if failures == 0 {
            self.gc_floor = Some(prune_view);
            Ok(())
        } else {
            bail!("startup GC sweep failed for {failures} of {total} directories");
        }
    }

    /// Delete every view-keyed file in `dir_path` that is below `prune_view`, except
    /// `keep_decided_view`. Used only by the one-time startup sweep; steady-state GC unlinks by
    /// constructed path instead of scanning.
    fn prune_files(
        &mut self,
        dir_path: PathBuf,
        prune_view: ViewNumber,
        keep_decided_view: Option<ViewNumber>,
    ) -> anyhow::Result<()> {
        if !dir_path.is_dir() {
            return Ok(());
        }

        for (file_view, path) in self.view_files(dir_path)? {
            if let Some(decided_view) = keep_decided_view
                && decided_view == file_view
            {
                continue;
            }
            if file_view < prune_view {
                match fs::remove_file(&path) {
                    Ok(()) => {},
                    Err(err) if err.kind() == std::io::ErrorKind::NotFound => {},
                    Err(err) => return Err(err).context(format!("removing {}", path.display())),
                }
            }
        }

        Ok(())
    }

    /// Unlink `<view>.<ext>` from each directory in `pruned_dirs`, treating a missing file as
    /// success. `anchor` is never removed from `decided_leaves2`.
    fn remove_view_files(&self, view: ViewNumber, anchor: ViewNumber) -> anyhow::Result<()> {
        let [
            da2,
            vid2,
            quorum_proposals2,
            state_cert,
            decided_cert2,
            decided_leaves2,
        ] = self.pruned_dirs();
        let total = self.pruned_dirs().len();
        let mut failures = 0usize;
        for (dir, ext) in [da2, vid2, quorum_proposals2, state_cert, decided_cert2] {
            if let Err(err) = unlink_view(&dir, view, ext) {
                tracing::warn!(?view, dir = %dir.display(), "GC: failed to prune: {err:#}");
                failures += 1;
            }
        }

        let (leaves_dir, leaves_ext) = decided_leaves2;
        if view != anchor
            && let Err(err) = unlink_view(&leaves_dir, view, leaves_ext)
        {
            tracing::warn!(?view, dir = %leaves_dir.display(), "GC: failed to prune: {err:#}");
            failures += 1;
        }

        if failures == 0 {
            Ok(())
        } else {
            bail!("failed to prune {failures} of {total} directories for view {view}");
        }
    }

    fn parse_decided_leaf(
        &self,
        bytes: &[u8],
    ) -> anyhow::Result<(Leaf2, CertificatePair<SeqTypes>)> {
        // Old versions of the software did not store the next epoch QC. Without knowing which
        // version this file was created with, we can simply try parsing both ways and then
        // reconstruct a certificate pair with or without the next epoch QC.
        match bincode::deserialize(bytes) {
            Ok((leaf, cert)) => Ok((leaf, cert)),
            Err(err) => {
                tracing::info!(
                    "error parsing decided leaf, maybe file was created without next epoch QC? \
                     {err}"
                );
                let (leaf, qc) =
                    bincode::deserialize::<(Leaf2, QuorumCertificate2<SeqTypes>)>(bytes)
                        .context("parsing decided leaf")?;
                Ok((leaf, CertificatePair::non_epoch_change(qc)))
            },
        }
    }

    /// Phase 1. Read every persisted leaf at or below `view`, hydrate it, and build the events
    /// to emit. Holds the write lock; awaits nothing external.
    fn load_pending_decides(
        &mut self,
        view: ViewNumber,
        deciding_qc: Option<Arc<CertificatePair<SeqTypes>>>,
    ) -> anyhow::Result<Vec<PendingDecide>> {
        // We build a separate event for each leaf because it is possible we have non-consecutive
        // leaves in our storage, which would not be valid as a single decide with a single leaf
        // chain.
        let mut leaves = BTreeMap::new();
        for (v, path) in self.view_files(self.decided_leaf2_path())? {
            if v > view {
                continue;
            }

            let bytes =
                fs::read(&path).context(format!("reading decided leaf {}", path.display()))?;
            let (mut leaf, cert) = self.parse_decided_leaf(&bytes)?;

            // Include the VID share if available.
            let vid_proposal = self.load_vid_share(v)?;
            if vid_proposal.is_none() {
                tracing::debug!(?v, "VID share not available at decide");
            }
            let vid_share = vid_proposal.as_ref().map(|proposal| proposal.data.clone());

            // Move the state cert to the finalized dir if it exists.
            let state_cert = self.store_finalized_state_cert(v)?;

            // Fill in the full block payload using the DA proposals we had persisted.
            if let Some(proposal) = self.load_da_proposal(v)? {
                let payload = Payload::from_bytes(
                    &proposal.data.encoded_transactions,
                    &proposal.data.metadata,
                );
                leaf.fill_block_payload_unchecked(payload);
            } else {
                tracing::debug!(?v, "DA proposal not available at decide");
            }

            let info = LeafInfo {
                leaf,
                vid_share,
                state_cert,
                // Note: the following fields are not used in Decide event processing, and should be
                // removed. For now, we just default them.
                state: Default::default(),
                delta: Default::default(),
            };

            leaves.insert(v, (info, cert));
        }

        // The invariant is that the oldest existing leaf in the `anchor_leaf` table -- if there is
        // one -- was always included in the _previous_ decide event...but not removed from the
        // database, because we always persist the most recent anchor leaf.
        if let Some((oldest_view, _)) = leaves.first_key_value() {
            // The only exception is when the oldest leaf is the genesis leaf; then there was no
            // previous decide event.
            if *oldest_view > ViewNumber::genesis() {
                leaves.pop_first();
            }
        }

        let mut pending = Vec::with_capacity(leaves.len());
        for (view, (leaf, cert)) in leaves {
            let height = leaf.leaf.block_header().block_number();

            let event = if leaf.leaf.block_header().version() >= versions::NEW_PROTOCOL_VERSION {
                let cert2 = self.load_cert2(view)?;
                // One event per view. cert2 is only stored for the
                // directly finalized view
                // ancestors get `cert2: None`,
                // which is what update() expects for indirectly decided leaves.
                CoordinatorEvent::NewDecide {
                    leaf_infos: vec![leaf],
                    cert1: cert.qc().clone(),
                    cert2,
                }
            } else {
                let deciding_qc = deciding_qc
                    .as_ref()
                    .filter(|qc| qc.view_number() == cert.view_number() + 1)
                    .cloned();
                CoordinatorEvent::LegacyEvent(Event {
                    view_number: view,
                    event: EventType::Decide {
                        committing_qc: Arc::new(cert),
                        deciding_qc,
                        leaf_chain: Arc::new(vec![leaf]),
                        block_size: None,
                    },
                })
            };

            pending.push(PendingDecide {
                view,
                height,
                event,
            });
        }

        Ok(pending)
    }

    fn load_da_proposal(
        &self,
        view: ViewNumber,
    ) -> anyhow::Result<Option<Proposal<SeqTypes, DaProposal2<SeqTypes>>>> {
        let dir_path = self.da2_dir_path();

        let file_path = dir_path.join(view.u64().to_string()).with_extension("txt");

        if !file_path.exists() {
            return Ok(None);
        }

        let da_bytes = fs::read(file_path)?;

        let da_proposal: Proposal<SeqTypes, DaProposal2<SeqTypes>> =
            bincode::deserialize(&da_bytes)?;
        Ok(Some(da_proposal))
    }

    fn load_vid_share(
        &self,
        view: ViewNumber,
    ) -> anyhow::Result<Option<Proposal<SeqTypes, VidDisperseShare<SeqTypes>>>> {
        let dir_path = self.vid2_dir_path();

        let file_path = dir_path.join(view.u64().to_string()).with_extension("txt");

        if !file_path.exists() {
            return Ok(None);
        }

        let vid_share_bytes = fs::read(file_path)?;
        let vid_share: Proposal<SeqTypes, VidDisperseShare<SeqTypes>> =
            bincode::deserialize(&vid_share_bytes)?;
        Ok(Some(vid_share))
    }

    fn load_anchor_leaf(&self) -> anyhow::Result<Option<(Leaf2, CertificatePair<SeqTypes>)>> {
        tracing::info!("Checking `Leaf2` to load the anchor leaf.");
        if self.decided_leaf2_path().is_dir() {
            let mut anchor: Option<(Leaf2, CertificatePair<SeqTypes>)> = None;

            // Return the latest decided leaf.
            for (_, path) in self.view_files(self.decided_leaf2_path())? {
                let bytes =
                    fs::read(&path).context(format!("reading decided leaf {}", path.display()))?;
                let (leaf, cert) = self.parse_decided_leaf(&bytes)?;
                if let Some((anchor_leaf, _)) = &anchor {
                    if leaf.view_number() > anchor_leaf.view_number() {
                        anchor = Some((leaf, cert));
                    }
                } else {
                    anchor = Some((leaf, cert));
                }
            }

            return Ok(anchor);
        }

        tracing::warn!(
            "Failed to find an anchor leaf in `Leaf2` storage. Checking legacy `Leaf` storage. \
             This is very likely to fail."
        );
        if self.legacy_anchor_leaf_path().is_file() {
            // We may have an old version of storage, where there is just a single file for the
            // anchor leaf. Read it and return the contents.
            let mut file = BufReader::new(File::open(self.legacy_anchor_leaf_path())?);

            // The first 8 bytes just contain the height of the leaf. We can skip this.
            file.seek(SeekFrom::Start(8)).context("seek")?;
            let bytes = file
                .bytes()
                .collect::<Result<Vec<_>, _>>()
                .context("read")?;
            let (leaf2, qc2): (Leaf2, QuorumCertificate2<SeqTypes>) =
                bincode::deserialize(&bytes).context("deserialize")?;
            let cert_pair = CertificatePair::new(qc2, None);
            return Ok(Some((leaf2, cert_pair)));
        }

        Ok(None)
    }

    fn store_finalized_state_cert(
        &mut self,
        view: ViewNumber,
    ) -> anyhow::Result<Option<LightClientStateUpdateCertificateV2<SeqTypes>>> {
        let dir_path = self.state_cert_dir_path();
        let file_path = dir_path.join(view.u64().to_string()).with_extension("txt");

        if !file_path.exists() {
            return Ok(None);
        }

        let bytes = fs::read(&file_path)?;

        let state_cert: LightClientStateUpdateCertificateV2<SeqTypes> =
            bincode::deserialize(&bytes).or_else(|err_v2| {
                tracing::info!(
                    error = %err_v2,
                    path = %file_path.display(),
                    "Failed to deserialize state certificate, attempting with v1"
                );

                bincode::deserialize::<LightClientStateUpdateCertificateV1<SeqTypes>>(&bytes)
                    .map(Into::into)
                    .with_context(|| {
                        format!(
                            "Failed to deserialize with both v2 and v1 from file '{}'. error: \
                             {err_v2}",
                            file_path.display()
                        )
                    })
            })?;

        let epoch = state_cert.epoch.u64();
        let finalized_dir_path = self.finalized_state_cert_dir_path();
        fs::create_dir_all(&finalized_dir_path).context("creating finalized state cert dir")?;

        let finalized_file_path = finalized_dir_path
            .join(epoch.to_string())
            .with_extension("txt");

        self.replace(
            &finalized_file_path,
            |_| Ok(true),
            |mut file| {
                file.write_all(&bytes)?;
                Ok(())
            },
        )
        .context(format!(
            "finalizing light client state update certificate file for epoch {epoch:?}"
        ))?;

        Ok(Some(state_cert))
    }
}

#[async_trait]
impl SequencerPersistence for Persistence {
    async fn load_config(&self) -> anyhow::Result<Option<NetworkConfig>> {
        let inner = self.inner.read().await;
        let path = inner.config_path();
        if !path.is_file() {
            tracing::info!("config not found at {}", path.display());
            return Ok(None);
        }
        tracing::info!("loading config from {}", path.display());

        let bytes =
            fs::read(&path).context(format!("unable to read config from {}", path.display()))?;
        let json = serde_json::from_slice(&bytes).context("config file is not valid JSON")?;
        let json = migrate_network_config(json).context("migration of network config failed")?;
        let config = serde_json::from_value(json).context("malformed config file")?;
        Ok(Some(config))
    }

    async fn save_config(&self, cfg: &NetworkConfig) -> anyhow::Result<()> {
        let inner = self.inner.write().await;
        let path = inner.config_path();
        tracing::info!("saving config to {}", path.display());
        Ok(cfg.to_file(path.display().to_string())?)
    }

    async fn load_latest_acted_view(&self) -> anyhow::Result<Option<ViewNumber>> {
        let inner = self.inner.read().await;
        let path = inner.voted_view_path();
        if !path.is_file() {
            return Ok(None);
        }
        let bytes = fs::read(inner.voted_view_path())?
            .try_into()
            .map_err(|bytes| anyhow!("malformed voted view file: {bytes:?}"))?;
        Ok(Some(ViewNumber::new(u64::from_le_bytes(bytes))))
    }

    async fn load_restart_view(&self) -> anyhow::Result<Option<ViewNumber>> {
        let inner = self.inner.read().await;
        let path = inner.restart_view_path();
        if !path.is_file() {
            return Ok(None);
        }
        let bytes = fs::read(path)?
            .try_into()
            .map_err(|bytes| anyhow!("malformed restart view file: {bytes:?}"))?;
        Ok(Some(ViewNumber::new(u64::from_le_bytes(bytes))))
    }

    async fn persist_decided_leaves(
        &self,
        _view: ViewNumber,
        leaf_chain: impl IntoIterator<Item = (&LeafInfo<SeqTypes>, CertificatePair<SeqTypes>)> + Send,
        _deciding_qc: Option<Arc<CertificatePair<SeqTypes>>>,
        _consumer: &(impl EventConsumer + 'static),
    ) -> anyhow::Result<()> {
        let mut inner = self.inner.write().await;
        let path = inner.decided_leaf2_path();

        // Ensure the anchor leaf directory exists.
        fs::create_dir_all(&path).context("creating anchor leaf directory")?;

        // Earlier versions stored only a single decided leaf in a regular file. If our storage is
        // still on this version, migrate to a directory structure storing (possibly) many leaves.
        let legacy_path = inner.legacy_anchor_leaf_path();
        if !path.is_dir() && legacy_path.is_file() {
            tracing::info!("migrating to multi-leaf storage");

            // Move the existing data into the new directory.
            let (leaf, qc) = inner
                .load_anchor_leaf()?
                .context("anchor leaf file exists but unable to load contents")?;
            let view = leaf.view_number().u64();
            let bytes = bincode::serialize(&(leaf, qc))?;
            let new_file = path.join(view.to_string()).with_extension("txt");
            inner
                .replace(
                    &new_file,
                    |_| Ok(true),
                    |mut file| {
                        file.write_all(&bytes)?;
                        Ok(())
                    },
                )
                .context(format!("writing anchor leaf file {view}"))?;

            // Now we can remove the old file.
            fs::remove_file(&legacy_path).context("removing legacy anchor leaf file")?;
        }

        for (info, cert) in leaf_chain {
            let view = info.leaf.view_number().u64();
            let file_path = path.join(view.to_string()).with_extension("txt");
            inner.replace(
                &file_path,
                |_| {
                    // Don't overwrite an existing leaf, but warn about it as this is likely not
                    // intended behavior from HotShot.
                    tracing::warn!(view, "duplicate decided leaf");
                    Ok(false)
                },
                |mut file| {
                    let bytes = bincode::serialize(&(&info.leaf, cert))?;
                    file.write_all(&bytes)?;
                    Ok(())
                },
            )?;
        }

        Ok(())
    }

    /// Callers must not invoke this concurrently with itself: phase 2 runs with no persistence
    /// lock held, so two overlapping passes could interleave GC against each other's still-live
    /// leaf files. `process_decided_events_task` is the sole production caller, a single serial
    /// loop over a `watch` channel, which already guarantees this.
    async fn process_decided_events(
        &self,
        view: ViewNumber,
        deciding_qc: Option<Arc<CertificatePair<SeqTypes>>>,
        consumer: &(impl EventConsumer + 'static),
    ) -> anyhow::Result<Option<ViewNumber>> {
        // Started before the lock acquisition: this metric spans both write-lock holds (phase 1
        // and phase 3) and the lock-free consumer time in between (phase 2), so it reflects the
        // full pass, not just time under lock.
        let now = Instant::now();
        let pending = self
            .inner
            .write()
            .await
            .load_pending_decides(view, deciding_qc)?;
        let emitted: Vec<ViewNumber> = pending.iter().map(|p| p.view).collect();

        // No persistence lock held here: the consumer runs unbounded I/O (query-service
        // ingestion), and consensus appends proceed concurrently.
        let intervals = emit_decides(pending, consumer).await?;

        // Highest view we generated an event for; unprocessed leaves stay on disk (the cursor).
        let processed = intervals.iter().map(|i| *i.end()).max();

        // On error, GC does not run over the failed range, so the leaves stay on disk and are
        // retried; no data is lost. Best-effort: runs again at the next decide.
        let res = self
            .inner
            .write()
            .await
            .collect_garbage(view, &intervals, &emitted);
        if let Err(err) = res {
            tracing::warn!(?view, "GC failed: {err:#}");
        }
        self.metrics
            .internal_process_decided_events_duration
            .add_point(now.elapsed().as_secs_f64());

        Ok(processed)
    }

    async fn load_anchor_leaf(&self) -> anyhow::Result<Option<(Leaf2, CertificatePair<SeqTypes>)>> {
        self.inner.read().await.load_anchor_leaf()
    }

    async fn load_da_proposal(
        &self,
        view: ViewNumber,
    ) -> anyhow::Result<Option<Proposal<SeqTypes, DaProposal2<SeqTypes>>>> {
        self.inner.read().await.load_da_proposal(view)
    }

    async fn load_vid_share(
        &self,
        view: ViewNumber,
    ) -> anyhow::Result<Option<Proposal<SeqTypes, VidDisperseShare<SeqTypes>>>> {
        self.inner.read().await.load_vid_share(view)
    }

    async fn append_vid(
        &self,
        proposal: &Proposal<SeqTypes, VidDisperseShare<SeqTypes>>,
    ) -> anyhow::Result<()> {
        let mut inner = self.inner.write().await;
        let view_number = proposal.data.view_number().u64();
        let dir_path = inner.vid2_dir_path();

        fs::create_dir_all(dir_path.clone()).context("failed to create vid dir")?;

        let file_path = dir_path.join(view_number.to_string()).with_extension("txt");
        inner.replace(
            &file_path,
            |_| {
                // Don't overwrite an existing share, but warn about it as this is likely not intended
                // behavior from HotShot.
                tracing::warn!(view_number, "duplicate VID share");
                Ok(false)
            },
            |mut file| {
                let proposal_bytes = bincode::serialize(proposal).context("serialize proposal")?;
                let now = Instant::now();
                file.write_all(&proposal_bytes)?;
                self.metrics
                    .internal_append_vid_duration
                    .add_point(now.elapsed().as_secs_f64());
                Ok(())
            },
        )
    }

    async fn append_da(
        &self,
        proposal: &Proposal<SeqTypes, DaProposal<SeqTypes>>,
        _vid_commit: VidCommitment,
    ) -> anyhow::Result<()> {
        let mut inner = self.inner.write().await;
        let view_number = proposal.data.view_number().u64();
        let dir_path = inner.da_dir_path();

        fs::create_dir_all(dir_path.clone()).context("failed to create da dir")?;

        let file_path = dir_path.join(view_number.to_string()).with_extension("txt");
        inner.replace(
            &file_path,
            |_| {
                // Don't overwrite an existing proposal, but warn about it as this is likely not
                // intended behavior from HotShot.
                tracing::warn!(view_number, "duplicate DA proposal");
                Ok(false)
            },
            |mut file| {
                let proposal_bytes = bincode::serialize(&proposal).context("serialize proposal")?;
                let now = Instant::now();
                file.write_all(&proposal_bytes)?;
                self.metrics
                    .internal_append_da_duration
                    .add_point(now.elapsed().as_secs_f64());
                Ok(())
            },
        )
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
        let mut inner = self.inner.write().await;
        let path = &inner.voted_view_path();
        inner.replace(
            path,
            |mut file| {
                let mut bytes = vec![];
                file.read_to_end(&mut bytes)?;
                let bytes = bytes
                    .try_into()
                    .map_err(|bytes| anyhow!("malformed voted view file: {bytes:?}"))?;
                let saved_view = ViewNumber::new(u64::from_le_bytes(bytes));

                // Overwrite the file if the saved view is older than the new view.
                Ok(saved_view < view)
            },
            |mut file| {
                file.write_all(&view.u64().to_le_bytes())?;
                Ok(())
            },
        )?;

        if matches!(action, HotShotAction::Vote) {
            let restart_view_path = &inner.restart_view_path();
            let restart_view = view + 1;
            inner.replace(
                restart_view_path,
                |mut file| {
                    let mut bytes = vec![];
                    file.read_to_end(&mut bytes)?;
                    let bytes = bytes
                        .try_into()
                        .map_err(|bytes| anyhow!("malformed voted view file: {bytes:?}"))?;
                    let saved_view = ViewNumber::new(u64::from_le_bytes(bytes));

                    // Overwrite the file if the saved view is older than the new view.
                    Ok(saved_view < restart_view)
                },
                |mut file| {
                    file.write_all(&restart_view.u64().to_le_bytes())?;
                    Ok(())
                },
            )?;
        }
        Ok(())
    }

    async fn append_quorum_proposal2(
        &self,
        proposal: &Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>>,
    ) -> anyhow::Result<()> {
        let mut inner = self.inner.write().await;
        let view_number = proposal.data.view_number().u64();
        let dir_path = inner.quorum_proposals2_dir_path();

        fs::create_dir_all(dir_path.clone()).context("failed to create proposals dir")?;

        let file_path = dir_path.join(view_number.to_string()).with_extension("txt");
        inner.replace(
            &file_path,
            |_| {
                // Always overwrite the previous file
                Ok(true)
            },
            |mut file| {
                let proposal_bytes = bincode::serialize(&proposal).context("serialize proposal")?;
                let now = Instant::now();
                file.write_all(&proposal_bytes)?;
                self.metrics
                    .internal_append_quorum2_duration
                    .add_point(now.elapsed().as_secs_f64());
                Ok(())
            },
        )
    }

    async fn append_cert2(
        &self,
        view: ViewNumber,
        cert2: Certificate2<SeqTypes>,
    ) -> anyhow::Result<()> {
        let mut inner = self.inner.write().await;
        let dir_path = inner.decided_cert2_dir_path();
        fs::create_dir_all(dir_path.clone()).context("failed to create decided_cert2 dir")?;
        let file_path = dir_path.join(view.u64().to_string()).with_extension("bin");
        inner.replace(
            &file_path,
            |_| Ok(true),
            |mut file| {
                let bytes = bincode::serialize(&cert2).context("serialize cert2")?;
                file.write_all(&bytes)?;
                Ok(())
            },
        )
    }

    async fn load_cert2(&self, view: ViewNumber) -> anyhow::Result<Option<Certificate2<SeqTypes>>> {
        let inner = self.inner.read().await;
        let dir_path = inner.decided_cert2_dir_path();
        let file_path = dir_path.join(view.u64().to_string()).with_extension("bin");
        if !file_path.is_file() {
            return Ok(None);
        }
        let bytes = fs::read(&file_path).context("read cert2")?;
        Ok(Some(
            bincode::deserialize(&bytes).context("deserialize cert2")?,
        ))
    }
    async fn load_quorum_proposals(
        &self,
    ) -> anyhow::Result<BTreeMap<ViewNumber, Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>>>>
    {
        let inner = self.inner.read().await;

        // First, get the proposal directory.
        let dir_path = inner.quorum_proposals2_dir_path();
        if !dir_path.is_dir() {
            return Ok(Default::default());
        }

        // Read quorum proposals from every data file in this directory.
        let mut map = BTreeMap::new();
        for (view, path) in inner.view_files(&dir_path)? {
            let proposal_bytes = fs::read(path)?;
            let Some(proposal) = bincode::deserialize::<
                Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>>,
            >(&proposal_bytes)
            .or_else(|error| {
                bincode::deserialize::<Proposal<SeqTypes, QuorumProposalWrapperLegacy<SeqTypes>>>(
                    &proposal_bytes,
                )
                .map(convert_proposal)
                .inspect_err(|err_v3| {
                    // At this point, if the file contents are invalid, it is most likely an
                    // error rather than a miscellaneous file somehow ending up in the
                    // directory. However, we continue on, because it is better to collect as
                    // many proposals as we can rather than letting one bad proposal cause the
                    // entire operation to fail, and it is still possible that this was just
                    // some unintended file whose name happened to match the naming convention.

                    tracing::warn!(
                        ?view,
                        %error,
                        error_v3 = %err_v3,
                        "ignoring malformed quorum proposal file"
                    );
                })
            })
            .ok() else {
                continue;
            };

            let proposal2 = convert_proposal(proposal);

            // Push to the map and we're done.
            map.insert(view, proposal2);
        }

        Ok(map)
    }

    async fn load_quorum_proposal(
        &self,
        view: ViewNumber,
    ) -> anyhow::Result<Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>>> {
        let inner = self.inner.read().await;
        let dir_path = inner.quorum_proposals2_dir_path();
        let file_path = dir_path.join(view.to_string()).with_extension("txt");
        let bytes = fs::read(file_path)?;
        let proposal: Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>> =
            bincode::deserialize(&bytes).or_else(|error| {
                bincode::deserialize::<Proposal<SeqTypes, QuorumProposalWrapperLegacy<SeqTypes>>>(
                    &bytes,
                )
                .map(convert_proposal)
                .context(format!(
                    "Failed to deserialize quorum proposal for view {view:?}: {error}."
                ))
            })?;
        Ok(proposal)
    }

    async fn load_upgrade_certificate(
        &self,
    ) -> anyhow::Result<Option<UpgradeCertificate<SeqTypes>>> {
        let inner = self.inner.read().await;
        let path = inner.upgrade_certificate_dir_path();
        if !path.is_file() {
            return Ok(None);
        }
        let bytes = fs::read(&path).context("read")?;
        Ok(Some(
            bincode::deserialize(&bytes).context("deserialize upgrade certificate")?,
        ))
    }

    async fn store_upgrade_certificate(
        &self,
        decided_upgrade_certificate: Option<UpgradeCertificate<SeqTypes>>,
    ) -> anyhow::Result<()> {
        let mut inner = self.inner.write().await;
        let path = &inner.upgrade_certificate_dir_path();
        let certificate = match decided_upgrade_certificate {
            Some(cert) => cert,
            None => return Ok(()),
        };
        inner.replace(
            path,
            |_| {
                // Always overwrite the previous file.
                Ok(true)
            },
            |mut file| {
                let bytes =
                    bincode::serialize(&certificate).context("serializing upgrade certificate")?;
                file.write_all(&bytes)?;
                Ok(())
            },
        )
    }

    async fn load_next_epoch_quorum_certificate(
        &self,
    ) -> anyhow::Result<Option<NextEpochQuorumCertificate2<SeqTypes>>> {
        let inner = self.inner.read().await;
        let path = inner.next_epoch_qc();
        if !path.is_file() {
            return Ok(None);
        }
        let bytes = fs::read(&path).context("read")?;
        Ok(Some(
            bincode::deserialize(&bytes).context("deserialize next epoch qc")?,
        ))
    }

    async fn append_next_epoch_high_qc2(
        &self,
        next_epoch_high_qc: NextEpochQuorumCertificate2<SeqTypes>,
    ) -> anyhow::Result<()> {
        let mut inner = self.inner.write().await;
        let path = &inner.next_epoch_qc();
        let view = next_epoch_high_qc.view_number();
        inner.replace(
            path,
            |mut file| {
                // Overwrite only when the new QC is newer. The whole replace runs under the inner
                // write lock, so this compare-and-set is atomic and a stale concurrent write cannot
                // regress the stored view (mirrors `append_high_qc2`).
                let mut bytes = vec![];
                file.read_to_end(&mut bytes)?;
                let existing: NextEpochQuorumCertificate2<SeqTypes> = bincode::deserialize(&bytes)
                    .context("deserializing existing next epoch high_qc2")?;
                Ok(existing.view_number() < view)
            },
            |mut file| {
                let bytes = bincode::serialize(&next_epoch_high_qc)
                    .context("serializing next epoch high_qc2")?;
                file.write_all(&bytes)?;
                Ok(())
            },
        )
    }

    async fn store_eqc(
        &self,
        high_qc: QuorumCertificate2<SeqTypes>,
        next_epoch_high_qc: NextEpochQuorumCertificate2<SeqTypes>,
    ) -> anyhow::Result<()> {
        let mut inner = self.inner.write().await;
        let path = &inner.eqc();

        inner.replace(
            path,
            |_| {
                // Always overwrite the previous file.
                Ok(true)
            },
            |mut file| {
                let bytes = bincode::serialize(&(high_qc, next_epoch_high_qc))
                    .context("serializing next epoch qc")?;
                file.write_all(&bytes)?;
                Ok(())
            },
        )
    }

    async fn load_eqc(
        &self,
    ) -> Option<(
        QuorumCertificate2<SeqTypes>,
        NextEpochQuorumCertificate2<SeqTypes>,
    )> {
        let inner = self.inner.read().await;
        let path = inner.eqc();
        if !path.is_file() {
            return None;
        }
        let bytes = fs::read(&path).ok()?;

        bincode::deserialize(&bytes).ok()
    }

    async fn append_high_qc2(&self, high_qc: QuorumCertificate2<SeqTypes>) -> anyhow::Result<()> {
        let mut inner = self.inner.write().await;
        let path = &inner.high_qc2();
        let view = high_qc.view_number();
        inner.replace(
            path,
            |mut file| {
                // Overwrite only when the new lock is newer. The whole replace
                // runs under the inner write lock, so this compare-and-set is
                // atomic and a stale concurrent write cannot regress the lock.
                let mut bytes = vec![];
                file.read_to_end(&mut bytes)?;
                let existing: QuorumCertificate2<SeqTypes> =
                    bincode::deserialize(&bytes).context("deserializing existing high_qc2")?;
                Ok(existing.view_number() < view)
            },
            |mut file| {
                let bytes = bincode::serialize(&high_qc).context("serializing high_qc2")?;
                file.write_all(&bytes)?;
                Ok(())
            },
        )
    }

    async fn load_high_qc2(&self) -> anyhow::Result<Option<QuorumCertificate2<SeqTypes>>> {
        let inner = self.inner.read().await;
        let path = inner.high_qc2();
        if !path.is_file() {
            return Ok(None);
        }
        let bytes = fs::read(&path).context("reading high_qc2")?;
        Ok(Some(
            bincode::deserialize(&bytes).context("deserializing high_qc2")?,
        ))
    }

    async fn append_da2(
        &self,
        proposal: &Proposal<SeqTypes, DaProposal2<SeqTypes>>,
        _vid_commit: VidCommitment,
    ) -> anyhow::Result<()> {
        let mut inner = self.inner.write().await;
        let view_number = proposal.data.view_number().u64();
        let dir_path = inner.da2_dir_path();

        fs::create_dir_all(dir_path.clone()).context("failed to create da dir")?;

        let file_path = dir_path.join(view_number.to_string()).with_extension("txt");
        inner.replace(
            &file_path,
            |_| {
                // Don't overwrite an existing proposal, but warn about it as this is likely not
                // intended behavior from HotShot.
                tracing::warn!(view_number, "duplicate DA proposal");
                Ok(false)
            },
            |mut file| {
                let proposal_bytes = bincode::serialize(&proposal).context("serialize proposal")?;
                let now = Instant::now();
                file.write_all(&proposal_bytes)?;
                self.metrics
                    .internal_append_da2_duration
                    .add_point(now.elapsed().as_secs_f64());
                Ok(())
            },
        )
    }

    async fn append_proposal2(
        &self,
        proposal: &Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>>,
    ) -> anyhow::Result<()> {
        self.append_quorum_proposal2(proposal).await
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

        let mut inner = self.inner.write().await;
        let dir_path = inner.drb_dir_path();

        fs::create_dir_all(dir_path.clone()).context("failed to create drb dir")?;

        let drb_input_bytes =
            bincode::serialize(&drb_input).context("failed to serialize drb_input")?;

        let file_path = dir_path
            .join(drb_input.epoch.to_string())
            .with_extension("bin");

        inner.replace(
            &file_path,
            |_| {
                // Always overwrite the previous file.
                Ok(true)
            },
            |mut file| {
                file.write_all(&drb_input_bytes).context(format!(
                    "writing epoch drb_input file for epoch {:?} at {:?}",
                    drb_input.epoch, file_path
                ))
            },
        )
    }

    async fn load_drb_input(&self, epoch: u64) -> anyhow::Result<DrbInput> {
        let inner = self.inner.read().await;
        let path = &inner.drb_dir_path();
        let file_path = path.join(epoch.to_string()).with_extension("bin");
        let bytes = fs::read(&file_path).context("read")?;
        Ok(bincode::deserialize(&bytes)
            .context(format!("failed to deserialize DrbInput for epoch {epoch}"))?)
    }

    async fn store_drb_result(
        &self,
        epoch: EpochNumber,
        drb_result: DrbResult,
    ) -> anyhow::Result<()> {
        let mut inner = self.inner.write().await;
        let dir_path = inner.epoch_drb_result_dir_path();

        fs::create_dir_all(dir_path.clone()).context("failed to create epoch drb result dir")?;

        let drb_result_bytes = bincode::serialize(&drb_result).context("serialize drb result")?;

        let file_path = dir_path.join(epoch.to_string()).with_extension("txt");

        inner.replace(
            &file_path,
            |_| {
                // Always overwrite the previous file.
                Ok(true)
            },
            |mut file| {
                file.write_all(&drb_result_bytes)
                    .context(format!("writing epoch drb result file for epoch {epoch:?}"))
            },
        )
    }

    async fn add_state_cert(
        &self,
        state_cert: LightClientStateUpdateCertificateV2<SeqTypes>,
    ) -> anyhow::Result<()> {
        let mut inner = self.inner.write().await;
        // let epoch = state_cert.epoch;
        let view = state_cert.light_client_state.view_number;
        let dir_path = inner.state_cert_dir_path();

        fs::create_dir_all(dir_path.clone())
            .context("failed to create light client state update certificate dir")?;

        let bytes = bincode::serialize(&state_cert)
            .context("serialize light client state update certificate")?;

        let file_path = dir_path.join(view.to_string()).with_extension("txt");
        inner
            .replace(
                &file_path,
                |_| Ok(true),
                |mut file| {
                    file.write_all(&bytes)?;
                    Ok(())
                },
            )
            .context(format!(
                "writing light client state update certificate file for view {view:?}"
            ))?;

        Ok(())
    }

    async fn load_start_epoch_info(&self) -> anyhow::Result<Vec<InitializerEpochInfo<SeqTypes>>> {
        let inner = self.inner.read().await;
        let drb_dir_path = inner.epoch_drb_result_dir_path();
        let block_header_dir_path = inner.epoch_root_block_header_dir_path();

        let mut result = Vec::new();

        if !drb_dir_path.is_dir() {
            return Ok(Vec::new());
        }
        for (epoch, path) in epoch_files(drb_dir_path)? {
            let bytes =
                fs::read(&path).context(format!("reading epoch drb result {}", path.display()))?;
            let drb_result = bincode::deserialize::<DrbResult>(&bytes)
                .context(format!("parsing epoch drb result {}", path.display()))?;

            let block_header_path = block_header_dir_path
                .join(epoch.to_string())
                .with_extension("txt");
            let block_header = if block_header_path.is_file() {
                let bytes = fs::read(&block_header_path).context(format!(
                    "reading epoch root block header {}",
                    block_header_path.display()
                ))?;
                Some(
                    bincode::deserialize::<<SeqTypes as NodeType>::BlockHeader>(&bytes).context(
                        format!(
                            "parsing epoch root block header {}",
                            block_header_path.display()
                        ),
                    )?,
                )
            } else {
                None
            };

            result.push(InitializerEpochInfo::<SeqTypes> {
                epoch,
                drb_result,
                block_header,
            });
        }

        result.sort_by_key(|a| a.epoch);

        // Keep only the most recent epochs
        let start = result
            .len()
            .saturating_sub(RECENT_STAKE_TABLES_LIMIT as usize);
        let recent = result[start..].to_vec();

        Ok(recent)
    }

    async fn load_state_cert(
        &self,
    ) -> anyhow::Result<Option<LightClientStateUpdateCertificateV2<SeqTypes>>> {
        let inner = self.inner.read().await;
        let dir_path = inner.finalized_state_cert_dir_path();

        if !dir_path.is_dir() {
            return Ok(None);
        }

        let mut result: Option<LightClientStateUpdateCertificateV2<SeqTypes>> = None;

        for (epoch, path) in epoch_files(dir_path)? {
            if result.as_ref().is_some_and(|cert| epoch <= cert.epoch) {
                continue;
            }
            let bytes = fs::read(&path).context(format!(
                "reading light client state update certificate {}",
                path.display()
            ))?;
            let cert =
                bincode::deserialize::<LightClientStateUpdateCertificateV2<SeqTypes>>(&bytes)
                    .or_else(|error| {
                        tracing::info!(
                            %error,
                            path = %path.display(),
                            "Failed to deserialize LightClientStateUpdateCertificateV2"
                        );

                        bincode::deserialize::<LightClientStateUpdateCertificateV1<SeqTypes>>(
                            &bytes,
                        )
                        .map(Into::into)
                        .with_context(|| {
                            format!(
                                "Failed to deserialize with v1 and v2. path='{}'. error: {error}",
                                path.display()
                            )
                        })
                    })?;

            result = Some(cert);
        }

        Ok(result)
    }

    async fn get_state_cert_by_epoch(
        &self,
        epoch: u64,
    ) -> anyhow::Result<Option<LightClientStateUpdateCertificateV2<SeqTypes>>> {
        let inner = self.inner.read().await;
        let dir_path = inner.finalized_state_cert_dir_path();

        let file_path = dir_path.join(epoch.to_string()).with_extension("txt");

        if !file_path.exists() {
            return Ok(None);
        }

        let bytes = fs::read(&file_path).context(format!(
            "reading light client state update certificate {}",
            file_path.display()
        ))?;

        let cert = bincode::deserialize::<LightClientStateUpdateCertificateV2<SeqTypes>>(&bytes)
            .or_else(|error| {
                tracing::info!(
                    %error,
                    path = %file_path.display(),
                    "Failed to deserialize LightClientStateUpdateCertificateV2"
                );

                bincode::deserialize::<LightClientStateUpdateCertificateV1<SeqTypes>>(&bytes)
                    .map(Into::into)
                    .with_context(|| {
                        format!(
                            "Failed to deserialize with v1 and v2. path='{}'. error: {error}",
                            file_path.display()
                        )
                    })
            })?;

        Ok(Some(cert))
    }

    async fn insert_state_cert(
        &self,
        epoch: u64,
        cert: LightClientStateUpdateCertificateV2<SeqTypes>,
    ) -> anyhow::Result<()> {
        let inner = self.inner.read().await;
        let dir_path = inner.finalized_state_cert_dir_path();

        fs::create_dir_all(&dir_path)
            .context(format!("creating state cert dir {}", dir_path.display()))?;

        let file_path = dir_path.join(epoch.to_string()).with_extension("txt");
        let bytes = bincode::serialize(&cert)
            .context("serializing light client state update certificate")?;

        fs::write(&file_path, bytes).context(format!(
            "writing light client state update certificate {}",
            file_path.display()
        ))?;

        Ok(())
    }

    fn enable_metrics(&mut self, metrics: &dyn Metrics) {
        self.metrics = Arc::new(PersistenceMetricsValue::new(metrics));
    }
}

#[async_trait]
impl MembershipPersistence for Persistence {
    async fn load_stake(&self, epoch: EpochNumber) -> anyhow::Result<Option<StakeTuple>> {
        let inner = self.inner.read().await;
        let path = &inner.stake_table_dir_path();
        let file_path = path.join(epoch.to_string()).with_extension("txt");

        if !file_path.exists() {
            return Ok(None);
        }

        let bytes = fs::read(&file_path).with_context(|| {
            format!("failed to read stake table file at {}", file_path.display())
        })?;

        let (stake, _needs_rewrite) = deserialize_stake_table(&bytes).with_context(|| {
            format!(
                "failed to deserialize stake table at {}",
                file_path.display()
            )
        })?;
        Ok(Some(stake))
    }

    async fn load_drb_result(&self, epoch: EpochNumber) -> anyhow::Result<Option<DrbResult>> {
        let inner = self.inner.read().await;
        let file_path = inner
            .epoch_drb_result_dir_path()
            .join(epoch.to_string())
            .with_extension("txt");

        if !file_path.is_file() {
            return Ok(None);
        }

        let bytes = fs::read(&file_path)
            .context(format!("reading epoch drb result {}", file_path.display()))?;
        let drb_result = bincode::deserialize::<DrbResult>(&bytes)
            .context(format!("parsing epoch drb result {}", file_path.display()))?;
        Ok(Some(drb_result))
    }

    async fn load_epoch_root(&self, epoch: EpochNumber) -> anyhow::Result<Option<Header>> {
        let inner = self.inner.read().await;
        let file_path = inner
            .epoch_root_block_header_dir_path()
            .join(epoch.to_string())
            .with_extension("txt");

        if !file_path.is_file() {
            return Ok(None);
        }

        let bytes = fs::read(&file_path).context(format!(
            "reading epoch root block header {}",
            file_path.display()
        ))?;
        let header = bincode::deserialize::<Header>(&bytes).context(format!(
            "parsing epoch root block header {}",
            file_path.display()
        ))?;
        Ok(Some(header))
    }

    async fn store_epoch_root(
        &self,
        epoch: EpochNumber,
        block_header: Header,
    ) -> anyhow::Result<()> {
        let mut inner = self.inner.write().await;
        let dir_path = inner.epoch_root_block_header_dir_path();

        fs::create_dir_all(dir_path.clone())
            .context("failed to create epoch root block header dir")?;

        let block_header_bytes =
            bincode::serialize(&block_header).context("serialize block header")?;

        let file_path = dir_path.join(epoch.to_string()).with_extension("txt");
        inner
            .replace(
                &file_path,
                |_| Ok(true),
                |mut file| {
                    file.write_all(&block_header_bytes)?;
                    Ok(())
                },
            )
            .context(format!(
                "writing epoch root block header file for epoch {epoch:?}"
            ))?;

        Ok(())
    }

    async fn load_latest_stake(&self, limit: u64) -> anyhow::Result<Option<Vec<IndexedStake>>> {
        let limit = limit as usize;
        let inner = self.inner.read().await;
        let path = &inner.stake_table_dir_path();
        let sorted_files = epoch_files(path)?
            .sorted_by(|(e1, _), (e2, _)| e2.cmp(e1))
            .take(limit);
        let mut validator_sets: Vec<IndexedStake> = Vec::new();

        for (epoch, file_path) in sorted_files {
            let bytes = fs::read(&file_path).with_context(|| {
                format!("failed to read stake table file at {}", file_path.display())
            })?;

            let (stake, _needs_rewrite) = deserialize_stake_table(&bytes).with_context(|| {
                format!(
                    "failed to deserialize stake table at {}",
                    file_path.display()
                )
            })?;
            validator_sets.push((epoch, (stake.0, stake.1), stake.2));
        }

        Ok(Some(validator_sets))
    }

    async fn store_stake(
        &self,
        epoch: EpochNumber,
        stake: AuthenticatedValidatorMap,
        block_reward: Option<RewardAmount>,
        stake_table_hash: Option<StakeTableHash>,
    ) -> anyhow::Result<()> {
        let mut inner = self.inner.write().await;
        let dir_path = &inner.stake_table_dir_path();

        fs::create_dir_all(dir_path.clone()).context("failed to create stake table dir")?;

        let file_path = dir_path.join(epoch.to_string()).with_extension("txt");

        inner.replace(
            &file_path,
            |_| {
                // Always overwrite the previous file.
                Ok(true)
            },
            |mut file| {
                let data: StakeTuple = (stake, block_reward, stake_table_hash);
                let bytes =
                    bincode::serialize(&data).context("serializing combined stake table")?;
                file.write_all(&bytes)?;
                Ok(())
            },
        )
    }

    /// store stake table events upto the l1 block
    async fn store_events(
        &self,
        to_l1_block: u64,
        events: Vec<(EventKey, StakeTableEvent)>,
    ) -> anyhow::Result<()> {
        let mut inner = self.inner.write().await;
        let dir_path = &inner.stake_table_dir_path();
        let events_dir = dir_path.join("events");

        fs::create_dir_all(events_dir.clone()).context("failed to create events dir")?;
        // Read the last l1 finalized for which events has been stored
        let last_l1_finalized_path = events_dir.join("last_l1_finalized").with_extension("bin");

        // check if the last l1 events is higher than the incoming one
        if last_l1_finalized_path.exists() {
            let bytes = fs::read(&last_l1_finalized_path).with_context(|| {
                format!("Failed to read file at path: {last_l1_finalized_path:?}")
            })?;
            let mut buf = [0; 8];
            bytes
                .as_slice()
                .read_exact(&mut buf[..8])
                .with_context(|| {
                    format!("Failed to read 8 bytes from file at path: {last_l1_finalized_path:?}")
                })?;
            let persisted_l1_block = u64::from_le_bytes(buf);
            if persisted_l1_block > to_l1_block {
                tracing::debug!(?persisted_l1_block, ?to_l1_block, "stored l1 is greater");
                return Ok(());
            }
        }

        // stores each event in a separate file
        // this can cause performance issue when, for example, reading all the files
        // However, the plan is to remove file system completely in future
        for (event_key, event) in events {
            let (block_number, event_index) = event_key;
            // file name is like block_index.json
            let filename = format!("{block_number}_{event_index}");
            let file_path = events_dir.join(filename).with_extension("json");

            if file_path.exists() {
                continue;
            }

            inner
                .replace(
                    &file_path,
                    |_| Ok(true),
                    |file| {
                        let writer = BufWriter::new(file);

                        serde_json::to_writer_pretty(writer, &event)?;
                        Ok(())
                    },
                )
                .context("Failed to write event to file")?;
        }

        // update the l1 block for which we have processed events
        inner.replace(
            &last_l1_finalized_path,
            |_| Ok(true),
            |mut file| {
                let bytes = to_l1_block.to_le_bytes();

                file.write_all(&bytes)?;
                tracing::debug!("updated l1 finalized ={to_l1_block:?}");
                Ok(())
            },
        )
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
        let inner = self.inner.read().await;
        let dir_path = inner.stake_table_dir_path();
        let events_dir = dir_path.join("events");

        // check if we have any events in storage
        // we can do this by checking last l1 finalized block for which we processed events
        let last_l1_finalized_path = events_dir.join("last_l1_finalized").with_extension("bin");

        if !last_l1_finalized_path.exists() || !events_dir.exists() {
            return Ok((None, Vec::new()));
        }

        let mut events = Vec::new();

        let bytes = fs::read(&last_l1_finalized_path)
            .with_context(|| format!("Failed to read file at path: {last_l1_finalized_path:?}"))?;
        let mut buf = [0; 8];
        bytes
            .as_slice()
            .read_exact(&mut buf[..8])
            .with_context(|| {
                format!("Failed to read 8 bytes from file at path: {last_l1_finalized_path:?}")
            })?;

        let last_processed_l1_block = u64::from_le_bytes(buf);

        // Determine the L1 block for querying events.
        // If the last stored L1 block is greater than the requested block, limit the query to the requested block.
        // Otherwise, query up to the last stored block.
        let query_l1_block = if last_processed_l1_block > to_l1_block {
            to_l1_block
        } else {
            last_processed_l1_block
        };

        for entry in fs::read_dir(&events_dir).context("events directory")? {
            let entry = entry?;
            let path = entry.path();

            if !entry.file_type()?.is_file() {
                continue;
            }

            if path
                .extension()
                .context(format!("extension for path={path:?}"))?
                != "json"
            {
                continue;
            }

            let filename = path
                .file_stem()
                .and_then(|f| f.to_str())
                .unwrap_or_default();

            let parts: Vec<&str> = filename.split('_').collect();
            if parts.len() != 2 {
                continue;
            }

            let block_number = parts[0].parse::<u64>()?;
            let log_index = parts[1].parse::<u64>()?;

            if block_number < from_l1_block || block_number > query_l1_block {
                continue;
            }

            let file =
                File::open(&path).context(format!("Failed to open event file. path={path:?}"))?;
            let reader = BufReader::new(file);

            let event: StakeTableEvent = serde_json::from_reader(reader)
                .context(format!("Failed to deserialize event at path={path:?}"))?;

            events.push(((block_number, log_index), event));
        }

        events.sort_by_key(|(key, _)| *key);

        if query_l1_block == to_l1_block {
            Ok((Some(EventsPersistenceRead::Complete), events))
        } else {
            Ok((
                Some(EventsPersistenceRead::UntilL1Block(query_l1_block)),
                events,
            ))
        }
    }

    async fn delete_stake_tables(&self) -> anyhow::Result<()> {
        let inner = self.inner.write().await;
        let events_dir = inner.stake_table_dir_path().join("events");
        if events_dir.exists() {
            fs::remove_dir_all(&events_dir)
                .with_context(|| format!("Failed to remove events dir: {events_dir:?}"))?;
        }
        let validators_dir = inner.stake_table_dir_path().join("validators");
        if validators_dir.exists() {
            fs::remove_dir_all(&validators_dir)
                .with_context(|| format!("Failed to remove validators dir: {validators_dir:?}"))?;
        }
        let drb_dir = inner.epoch_drb_result_dir_path();
        if drb_dir.exists() {
            fs::remove_dir_all(&drb_dir)
                .with_context(|| format!("Failed to remove epoch DRB result dir: {drb_dir:?}"))?;
        }
        Ok(())
    }

    async fn store_all_validators(
        &self,
        epoch: EpochNumber,
        all_validators: RegisteredValidatorMap,
    ) -> anyhow::Result<()> {
        let mut inner = self.inner.write().await;
        let dir_path = inner.stake_table_dir_path();
        let validators_dir = dir_path.join("validators");

        // Ensure validators directory exists
        fs::create_dir_all(&validators_dir)
            .with_context(|| format!("Failed to create validators dir: {validators_dir:?}"))?;

        // Path = validators/epoch_<number>.json
        let file_path = validators_dir.join(format!("epoch_{epoch}.json"));

        inner
            .replace(
                &file_path,
                |_| Ok(true),
                |file| {
                    let writer = BufWriter::new(file);

                    serde_json::to_writer_pretty(writer, &all_validators).with_context(|| {
                        format!("Failed to serialize validators for epoch {epoch}")
                    })?;
                    Ok(())
                },
            )
            .with_context(|| format!("Failed to write validator file: {file_path:?}"))?;

        Ok(())
    }

    async fn load_all_validators(
        &self,
        epoch: EpochNumber,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<Vec<RegisteredValidator<PubKey>>> {
        let inner = self.inner.read().await;
        let dir_path = inner.stake_table_dir_path();
        let validators_dir = dir_path.join("validators");
        let file_path = validators_dir.join(format!("epoch_{epoch}.json"));

        if !file_path.exists() {
            bail!("Validator file not found for epoch {epoch}");
        }

        let file = File::open(&file_path)
            .with_context(|| format!("Failed to open validator file: {file_path:?}"))?;
        let reader = BufReader::new(file);

        let map: RegisteredValidatorMap = serde_json::from_reader(reader).with_context(|| {
            format!("Failed to deserialize validators at {file_path:?}. epoch = {epoch}")
        })?;

        let mut values: Vec<RegisteredValidator<PubKey>> = map.into_values().collect();
        values.sort_by_key(|v| v.account);

        let start = offset as usize;
        let end = (start + limit as usize).min(values.len());

        if start >= values.len() {
            return Ok(vec![]);
        }

        Ok(values[start..end].to_vec())
    }
}

#[async_trait]
impl DhtPersistentStorage for Persistence {
    /// Save the DHT to the file on disk
    ///
    /// # Errors
    /// - If we fail to serialize the records
    /// - If we fail to write the serialized records to the file
    async fn save(&self, records: Vec<SerializableRecord>) -> anyhow::Result<()> {
        // Bincode-serialize the records
        let to_save =
            bincode::serialize(&records).with_context(|| "failed to serialize records")?;

        // Get the path to save the file to
        let path = self.inner.read().await.libp2p_dht_path();

        // Create the directory if it doesn't exist
        fs::create_dir_all(path.parent().with_context(|| "directory had no parent")?)
            .with_context(|| "failed to create directory")?;

        // Get a write lock on the inner struct
        let mut inner = self.inner.write().await;

        // Save the file, replacing the previous one if it exists
        inner
            .replace(
                &path,
                |_| {
                    // Always overwrite the previous file
                    Ok(true)
                },
                |mut file| {
                    file.write_all(&to_save)
                        .with_context(|| "failed to write records to file")?;
                    Ok(())
                },
            )
            .with_context(|| "failed to save records to file")?;

        Ok(())
    }

    /// Load the DHT from the file on disk
    ///
    /// # Errors
    /// - If we fail to read the file
    /// - If we fail to deserialize the records
    async fn load(&self) -> anyhow::Result<Vec<SerializableRecord>> {
        // Read the contents of the file
        let contents = std::fs::read(self.inner.read().await.libp2p_dht_path())
            .with_context(|| "Failed to read records from file")?;

        // Deserialize the contents
        let records: Vec<SerializableRecord> =
            bincode::deserialize(&contents).with_context(|| "Failed to deserialize records")?;

        Ok(records)
    }
}

/// Phase 2. Emit in view order with no lock held, returning the height-contiguous view
/// intervals processed. Propagates the first consumer error.
async fn emit_decides(
    pending: Vec<PendingDecide>,
    consumer: &impl EventConsumer,
) -> anyhow::Result<Vec<RangeInclusive<ViewNumber>>> {
    let mut intervals = vec![];
    let mut current_interval = None;
    for PendingDecide {
        view,
        height,
        event,
    } in pending
    {
        consumer.handle_event(&event).await?;

        if let Some((start, end, current_height)) = current_interval.as_mut() {
            if height == *current_height + 1 {
                // If we have a chain of consecutive leaves, extend the current interval of
                // views which are safe to delete.
                *current_height += 1;
                *end = view;
            } else {
                // Otherwise, end the current interval and start a new one.
                intervals.push(*start..=*end);
                current_interval = Some((view, view, height));
            }
        } else {
            // Start a new interval.
            current_interval = Some((view, view, height));
        }
    }
    if let Some((start, end, _)) = current_interval {
        intervals.push(start..=end);
    }

    Ok(intervals)
}

/// Views the retention branch deletes this pass: `[floor, prune_view)`, at most `max`.
fn sweep_range(floor: ViewNumber, prune_view: ViewNumber, max: u64) -> Range<u64> {
    let start = floor.u64();
    let end = prune_view.u64().min(start.saturating_add(max));
    if end < start {
        start..start
    } else {
        start..end
    }
}

/// Unlink `<dir>/<view>.<ext>`. A missing file is success.
fn unlink_view(dir: &Path, view: ViewNumber, ext: &str) -> std::io::Result<()> {
    let path = dir.join(view.u64().to_string()).with_extension(ext);
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

/// Get all paths under `dir` whose name is of the form <view number>.txt.
fn view_files(
    dir: impl AsRef<Path>,
) -> anyhow::Result<impl Iterator<Item = (ViewNumber, PathBuf)>> {
    Ok(fs::read_dir(dir.as_ref())?.filter_map(move |entry| {
        let dir = dir.as_ref().display();
        let entry = entry.ok()?;
        if !entry.file_type().ok()?.is_file() {
            tracing::debug!(%dir, ?entry, "ignoring non-file in data directory");
            return None;
        }
        let path = entry.path();
        // Most view-keyed files use a `.txt` extension; cert2 files use `.bin`. Both hold bincode.
        let ext = path.extension()?;
        if ext != "txt" && ext != "bin" {
            tracing::debug!(%dir, ?entry, "ignoring file with unrecognized extension in data directory");
            return None;
        }
        let file_name = path.file_stem()?;
        let Ok(view_number) = file_name.to_string_lossy().parse::<u64>() else {
            tracing::debug!(%dir, ?file_name, "ignoring extraneous file in data directory");
            return None;
        };
        Some((ViewNumber::new(view_number), entry.path().to_owned()))
    }))
}

/// Get all paths under `dir` whose name is of the form <epoch number>.txt.
/// Should probably be made generic and merged with view_files.
fn epoch_files(
    dir: impl AsRef<Path>,
) -> anyhow::Result<impl Iterator<Item = (EpochNumber, PathBuf)>> {
    Ok(fs::read_dir(dir.as_ref())?.filter_map(move |entry| {
        let dir = dir.as_ref().display();
        let entry = entry.ok()?;
        if !entry.file_type().ok()?.is_file() {
            tracing::debug!(%dir, ?entry, "ignoring non-file in data directory");
            return None;
        }
        let path = entry.path();
        if path.extension()? != "txt" {
            tracing::debug!(%dir, ?entry, "ignoring non-text file in data directory");
            return None;
        }
        let file_name = path.file_stem()?;
        let Ok(epoch_number) = file_name.to_string_lossy().parse::<u64>() else {
            tracing::debug!(%dir, ?file_name, "ignoring extraneous file in data directory");
            return None;
        };
        Some((EpochNumber::new(epoch_number), entry.path().to_owned()))
    }))
}

#[cfg(test)]
mod test {
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::{collections::BTreeSet, marker::PhantomData, time::Duration};

    use committable::Committable;
    use espresso_types::{Leaf, NodeState, PubKey, ValidatedState, traits::NullEventConsumer};
    use hotshot::types::SignatureKey;
    use hotshot_example_types::node_types::TEST_VERSIONS;
    use hotshot_query_service::{metrics::PrometheusMetrics, testing::mocks::MOCK_UPGRADE};
    use hotshot_types::{
        data::{QuorumProposal2, ns_table::parse_ns_table, vid_disperse::AvidMDisperseShare},
        simple_vote::Vote2Data,
        traits::EncodeBytes,
        vid::avidm::{AvidMScheme, init_avidm_param},
    };
    use serde_json::json;
    use tempfile::TempDir;

    use super::*;
    use crate::{
        BLSPubKey,
        persistence::tests::{
            TestablePersistence, chain_with_views_and_heights, consecutive_height_chain, leaf_info,
        },
    };

    #[async_trait]
    impl TestablePersistence for Persistence {
        type Storage = TempDir;

        async fn tmp_storage() -> Self::Storage {
            TempDir::new().unwrap()
        }

        fn options(storage: &Self::Storage) -> impl PersistenceOptions<Persistence = Self> {
            Options::new(storage.path().into())
        }
    }

    #[test]
    fn test_config_migrations_add_builder_urls() {
        let before = json!({
            "config": {
                "builder_url": "https://test:8080",
                "start_proposing_view": 1,
                "stop_proposing_view": 2,
                "start_voting_view": 1,
                "stop_voting_view": 2,
                "start_proposing_time": 1,
                "stop_proposing_time": 2,
                "start_voting_time": 1,
                "stop_voting_time": 2
            }
        });
        let after = json!({
            "config": {
                "builder_urls": ["https://test:8080"],
                "start_proposing_view": 1,
                "stop_proposing_view": 2,
                "start_voting_view": 1,
                "stop_voting_view": 2,
                "start_proposing_time": 1,
                "stop_proposing_time": 2,
                "start_voting_time": 1,
                "stop_voting_time": 2,
                "epoch_height": 0,
                "drb_difficulty": 0,
                "drb_upgrade_difficulty": 0,
                "da_committees": [],
            }
        });

        assert_eq!(migrate_network_config(before).unwrap(), after);
    }

    #[test]
    fn test_config_migrations_existing_builder_urls() {
        let before = json!({
            "config": {
                "builder_urls": ["https://test:8080", "https://test:8081"],
                "start_proposing_view": 1,
                "stop_proposing_view": 2,
                "start_voting_view": 1,
                "stop_voting_view": 2,
                "start_proposing_time": 1,
                "stop_proposing_time": 2,
                "start_voting_time": 1,
                "stop_voting_time": 2,
                "epoch_height": 0,
                "drb_difficulty": 0,
                "drb_upgrade_difficulty": 0,
                "da_committees": [],
            }
        });

        assert_eq!(migrate_network_config(before.clone()).unwrap(), before);
    }

    #[test]
    fn test_config_migrations_add_upgrade_params() {
        let before = json!({
            "config": {
                "builder_urls": ["https://test:8080", "https://test:8081"]
            }
        });
        let after = json!({
            "config": {
                "builder_urls": ["https://test:8080", "https://test:8081"],
                "start_proposing_view": 9007199254740991u64,
                "stop_proposing_view": 0,
                "start_voting_view": 9007199254740991u64,
                "stop_voting_view": 0,
                "start_proposing_time": 9007199254740991u64,
                "stop_proposing_time": 0,
                "start_voting_time": 9007199254740991u64,
                "stop_voting_time": 0,
                "epoch_height": 0,
                "drb_difficulty": 0,
                "drb_upgrade_difficulty": 0,
                "da_committees": [],
            }
        });

        assert_eq!(migrate_network_config(before).unwrap(), after);
    }

    #[test]
    fn test_config_migrations_existing_upgrade_params() {
        let before = json!({
            "config": {
                "builder_urls": ["https://test:8080", "https://test:8081"],
                "start_proposing_view": 1,
                "stop_proposing_view": 2,
                "start_voting_view": 1,
                "stop_voting_view": 2,
                "start_proposing_time": 1,
                "stop_proposing_time": 2,
                "start_voting_time": 1,
                "stop_voting_time": 2,
                "epoch_height": 0,
                "drb_difficulty": 0,
                "drb_upgrade_difficulty": 0,
                "da_committees": [],
            }
        });

        assert_eq!(migrate_network_config(before.clone()).unwrap(), before);
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_load_quorum_proposals_invalid_extension() {
        let tmp = Persistence::tmp_storage().await;
        let storage = Persistence::connect(&tmp).await;

        // Generate a couple of valid quorum proposals.
        let leaf = Leaf2::genesis(&Default::default(), &NodeState::mock(), MOCK_UPGRADE.base).await;
        let privkey = PubKey::generated_from_seed_indexed([0; 32], 1).1;
        let signature = PubKey::sign(&privkey, &[]).unwrap();
        let mut quorum_proposal = Proposal {
            data: QuorumProposalWrapper::<SeqTypes> {
                proposal: QuorumProposal2::<SeqTypes> {
                    epoch: None,
                    block_header: leaf.block_header().clone(),
                    view_number: ViewNumber::genesis(),
                    justify_qc: QuorumCertificate2::genesis(
                        &Default::default(),
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
            },
            signature,
            _pd: Default::default(),
        };

        // Store quorum proposals.
        let quorum_proposal1 = quorum_proposal.clone();
        storage
            .append_quorum_proposal2(&quorum_proposal1)
            .await
            .unwrap();
        quorum_proposal.data.proposal.view_number = ViewNumber::new(1);
        let quorum_proposal2 = quorum_proposal.clone();
        storage
            .append_quorum_proposal2(&quorum_proposal2)
            .await
            .unwrap();

        // Change one of the file extensions. It can happen that we end up with files with the wrong
        // extension if, for example, the node is killed before cleaning up a swap file.
        fs::rename(
            tmp.path().join("quorum_proposals2/1.txt"),
            tmp.path().join("quorum_proposals2/1.swp"),
        )
        .unwrap();

        // Loading should simply ignore the unrecognized extension.
        assert_eq!(
            storage.load_quorum_proposals().await.unwrap(),
            [(ViewNumber::genesis(), quorum_proposal1)]
                .into_iter()
                .collect::<BTreeMap<_, _>>()
        );
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_cert2_persisted_as_bin() {
        let tmp = Persistence::tmp_storage().await;
        let storage = Persistence::connect(&tmp).await;
        let view = ViewNumber::new(7);
        let leaf = Leaf2::genesis(&Default::default(), &NodeState::mock(), MOCK_UPGRADE.base).await;
        let data = Vote2Data {
            leaf_commit: leaf.commit(),
            epoch: EpochNumber::new(1),
            block_number: leaf.height(),
        };
        let cert2 = Certificate2::new(data.clone(), data.commit(), view, None, PhantomData);

        storage.append_cert2(view, cert2.clone()).await.unwrap();

        assert!(tmp.path().join("decided_cert2/7.bin").is_file());
        assert!(!tmp.path().join("decided_cert2/7.txt").exists());
        assert_eq!(storage.load_cert2(view).await.unwrap(), Some(cert2));
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_load_quorum_proposals_malformed_data() {
        let tmp = Persistence::tmp_storage().await;
        let storage = Persistence::connect(&tmp).await;

        // Generate a valid quorum proposal.
        let leaf: Leaf2 = Leaf::genesis(&Default::default(), &NodeState::mock(), MOCK_UPGRADE.base)
            .await
            .into();
        let privkey = PubKey::generated_from_seed_indexed([0; 32], 1).1;
        let signature = PubKey::sign(&privkey, &[]).unwrap();
        let quorum_proposal = Proposal {
            data: QuorumProposalWrapper::<SeqTypes> {
                proposal: QuorumProposal2::<SeqTypes> {
                    epoch: None,
                    block_header: leaf.block_header().clone(),
                    view_number: ViewNumber::new(1),
                    justify_qc: QuorumCertificate2::genesis(
                        &Default::default(),
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
            },
            signature,
            _pd: Default::default(),
        };

        // First store an invalid quorum proposal.
        fs::create_dir_all(tmp.path().join("quorum_proposals2")).unwrap();
        fs::write(
            tmp.path().join("quorum_proposals2/0.txt"),
            "invalid data".as_bytes(),
        )
        .unwrap();

        // Store valid quorum proposal.
        storage
            .append_quorum_proposal2(&quorum_proposal)
            .await
            .unwrap();

        // Loading should ignore the invalid data and return the valid proposal.
        assert_eq!(
            storage.load_quorum_proposals().await.unwrap(),
            [(ViewNumber::new(1), quorum_proposal)]
                .into_iter()
                .collect::<BTreeMap<_, _>>()
        );
    }

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

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_store_all_validators_authenticated_and_unauthenticated() {
        use std::collections::HashMap;

        use alloy::primitives::{Address, U256};
        use espresso_types::v0_3::RegisteredValidator;
        use indexmap::IndexMap;

        let tmp = Persistence::tmp_storage().await;
        let mut opt = Persistence::options(&tmp);
        let storage = opt.create().await.unwrap();

        // Create an authenticated validator
        let authenticated_validator = RegisteredValidator {
            account: Address::random(),
            stake_table_key: Some(BLSPubKey::generated_from_seed_indexed([0u8; 32], 0).0),
            state_ver_key: Some(hotshot_types::light_client::StateVerKey::default()),
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
            state_ver_key: Some(hotshot_types::light_client::StateVerKey::default()),
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

    fn write_legacy_stake_file(
        path: &std::path::Path,
        epoch: u64,
        validator: RegisteredValidatorPreOption,
    ) {
        use indexmap::IndexMap;

        let mut map: IndexMap<Address, RegisteredValidatorPreOption> = IndexMap::new();
        map.insert(validator.account, validator);
        type PreOptionTuple = (
            IndexMap<Address, RegisteredValidatorPreOption>,
            Option<RewardAmount>,
            Option<StakeTableHash>,
        );
        let data: PreOptionTuple = (map, None, None);
        let bytes = bincode::serialize(&data).unwrap();
        fs::create_dir_all(path).unwrap();
        fs::write(path.join(format!("{epoch}.txt")), &bytes).unwrap();
    }

    fn pre_option_validator(seed: u8, stake: u64) -> RegisteredValidatorPreOption {
        use std::collections::HashMap;

        use alloy::primitives::U256;

        RegisteredValidatorPreOption {
            account: Address::random(),
            stake_table_key: BLSPubKey::generated_from_seed_indexed([seed; 32], 0).0,
            state_ver_key: hotshot_types::light_client::StateVerKey::default(),
            stake: U256::from(stake),
            commission: 0,
            delegators: HashMap::new(),
            authenticated: true,
            x25519_key: None,
            p2p_addr: None,
        }
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_load_stake_legacy_storage() {
        let tmp = Persistence::tmp_storage().await;
        let mut opt = Persistence::options(&tmp);
        let storage = opt.create().await.unwrap();

        let v1 = pre_option_validator(1, 100);
        let v2 = pre_option_validator(2, 200);
        let v1_addr = v1.account;
        let v2_addr = v2.account;

        let path = {
            let inner = storage.inner.read().await;
            inner.stake_table_dir_path()
        };
        write_legacy_stake_file(&path, 1, v1);
        write_legacy_stake_file(&path, 2, v2);

        let (loaded1, ..) = storage
            .load_stake(EpochNumber::new(1))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded1.len(), 1);
        assert!(loaded1.get(&v1_addr).unwrap().stake_table_key.is_some());

        let (loaded2, ..) = storage
            .load_stake(EpochNumber::new(2))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded2.len(), 1);
        assert!(loaded2.get(&v2_addr).unwrap().stake_table_key.is_some());

        let latest = storage.load_latest_stake(10).await.unwrap().unwrap();
        assert_eq!(latest.len(), 2);
        let epochs: Vec<_> = latest.iter().map(|(e, ..)| *e).collect();
        assert!(epochs.contains(&EpochNumber::new(1)));
        assert!(epochs.contains(&EpochNumber::new(2)));
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_load_stake_mixed_storage() {
        use indexmap::IndexMap;

        let tmp = Persistence::tmp_storage().await;
        let mut opt = Persistence::options(&tmp);
        let storage = opt.create().await.unwrap();

        let legacy_v = pre_option_validator(3, 300);
        let legacy_addr = legacy_v.account;
        let path = {
            let inner = storage.inner.read().await;
            inner.stake_table_dir_path()
        };
        write_legacy_stake_file(&path, 5, legacy_v);

        let current_v = espresso_types::v0_3::AuthenticatedValidator::mock();
        let current_addr = current_v.account;
        let mut current_map = IndexMap::new();
        current_map.insert(current_addr, current_v);
        storage
            .store_stake(EpochNumber::new(6), current_map, None, None)
            .await
            .unwrap();

        let latest = storage.load_latest_stake(10).await.unwrap().unwrap();
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

    fn write_dummy(dir: &Path, view: u64, ext: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join(view.to_string()).with_extension(ext), b"x").unwrap();
    }

    fn views_in_dir(dir: &Path) -> BTreeSet<u64> {
        if !dir.is_dir() {
            return BTreeSet::new();
        }
        view_files(dir).unwrap().map(|(v, _)| v.u64()).collect()
    }

    /// Persist and process a decide in one call, mirroring `decide_range` but for an explicit
    /// leaf set instead of a consecutive `chain[range]` slice.
    async fn decide_leaves(
        storage: &Persistence,
        leaves: &[(Leaf2, QuorumCertificate2<SeqTypes>)],
        decided_view: ViewNumber,
        consumer: &(impl EventConsumer + 'static),
    ) {
        let leaf_chain = leaves
            .iter()
            .map(|(leaf, qc)| (leaf_info(leaf.clone()), qc.clone()))
            .collect::<Vec<_>>();
        storage
            .append_decided_leaves(
                decided_view,
                leaf_chain
                    .iter()
                    .map(|(leaf, qc)| (leaf, CertificatePair::non_epoch_change(qc.clone()))),
                None,
                consumer,
            )
            .await
            .unwrap();
    }

    async fn vid_proposal(view: u64) -> Proposal<SeqTypes, VidDisperseShare<SeqTypes>> {
        let leaf = Leaf2::genesis(
            &ValidatedState::default(),
            &NodeState::mock(),
            MOCK_UPGRADE.base,
        )
        .await;
        let leaf_payload = leaf.block_payload().unwrap();
        let leaf_payload_bytes = leaf_payload.encode();
        let avidm_param = init_avidm_param(1).unwrap();
        let ns_table = parse_ns_table(
            leaf_payload.byte_len().as_usize(),
            &leaf_payload.ns_table().encode(),
        );
        let (payload_commitment, shares) =
            AvidMScheme::ns_disperse(&avidm_param, &[1u32], &leaf_payload_bytes, ns_table).unwrap();
        let (pubkey, privkey) = BLSPubKey::generated_from_seed_indexed([0; 32], 1);
        let vid = AvidMDisperseShare::<SeqTypes> {
            view_number: ViewNumber::new(view),
            payload_commitment,
            share: shares[0].clone(),
            recipient_key: pubkey,
            epoch: None,
            target_epoch: None,
            common: avidm_param,
        };
        convert_proposal(vid.to_proposal(&privkey).unwrap().clone())
    }

    #[test]
    fn test_sweep_range() {
        assert_eq!(
            sweep_range(ViewNumber::new(5), ViewNumber::new(5), 10),
            5..5
        );
        assert_eq!(
            sweep_range(ViewNumber::new(5), ViewNumber::new(3), 10),
            5..5
        );

        // length is `max`, and a following call continues from the advanced floor.
        let first = sweep_range(ViewNumber::new(0), ViewNumber::new(100), 10);
        assert_eq!(first, 0..10);
        let second = sweep_range(ViewNumber::new(first.end), ViewNumber::new(100), 10);
        assert_eq!(second, 10..20);
    }

    /// The interval branch must delete the full integer span of an interval, not just the
    /// views that had leaves, or views 1 and 3 (which never decide) are left behind.
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_gc_interval_span() {
        let tmp = Persistence::tmp_storage().await;
        let storage = Persistence::connect(&tmp).await;
        let leaves = chain_with_views_and_heights(&[(0, 0), (2, 1), (4, 2)]).await;

        let span_dirs = {
            let inner = storage.inner.read().await;
            inner.span_dirs()
        };
        for view in [1u64, 3] {
            for (dir, ext) in &span_dirs {
                write_dummy(dir, view, ext);
            }
        }

        decide_leaves(&storage, &leaves, ViewNumber::new(4), &NullEventConsumer).await;

        for view in 0u64..=3 {
            for (dir, ext) in &span_dirs {
                assert!(
                    !dir.join(view.to_string()).with_extension(*ext).exists(),
                    "view {view} should be gone from {}",
                    dir.display()
                );
            }
        }

        let leaves2_dir = storage.inner.read().await.decided_leaf2_path();
        assert_eq!(
            views_in_dir(&leaves2_dir),
            BTreeSet::from([4]),
            "view 0 is genesis and not popped, so it is emitted and falls inside the interval; \
             only the anchor (4) is exempt"
        );
    }

    /// Empty leaf chains isolate the retention branch from the interval branch, which would
    /// otherwise delete the same files first and mask what is under test.
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_gc_floor_retention() {
        let tmp = Persistence::tmp_storage().await;
        let mut options = Persistence::options(&tmp);
        options.set_view_retention(2);
        let storage = options.create().await.unwrap();

        let span_dirs = {
            let inner = storage.inner.read().await;
            inner.span_dirs()
        };
        for view in 0u64..=5 {
            for (dir, ext) in &span_dirs {
                write_dummy(dir, view, ext);
            }
        }

        for decided in 0u64..=5 {
            storage
                .append_decided_leaves(ViewNumber::new(decided), [], None, &NullEventConsumer)
                .await
                .unwrap();
            for view in 0u64..=5 {
                let should_be_gone = view < decided.saturating_sub(2);
                for (dir, ext) in &span_dirs {
                    let exists = dir.join(view.to_string()).with_extension(*ext).exists();
                    assert_eq!(
                        exists, !should_be_gone,
                        "view {view} at decided view {decided}: exists={exists}"
                    );
                }
            }
        }
    }

    /// Retention 2, so the first decide's `prune_view` is non-zero and the one-time sweep
    /// actually has something to collect.
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_gc_startup_sweep() {
        let tmp = Persistence::tmp_storage().await;
        {
            let mut options = Persistence::options(&tmp);
            options.set_view_retention(2);
            let storage = options.create().await.unwrap();
            let inner = storage.inner.read().await;
            for (dir, ext) in inner.span_dirs() {
                write_dummy(&dir, 0, ext);
            }
        }

        let mut options = Persistence::options(&tmp);
        options.set_view_retention(2);
        let storage = options.create().await.unwrap();

        storage
            .append_decided_leaves(ViewNumber::new(5), [], None, &NullEventConsumer)
            .await
            .unwrap();

        let inner = storage.inner.read().await;
        for (dir, ext) in inner.span_dirs() {
            assert!(
                !dir.join("0").with_extension(ext).exists(),
                "pre-existing file below retention should be gone after the startup sweep: {}",
                dir.display()
            );
        }
    }

    /// A 5000-file backlog that never decides and stays inside the retention window must not
    /// grow the per-decide scan count.
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_gc_scans_once_regardless_of_backlog() {
        let tmp = Persistence::tmp_storage().await;
        let storage = Persistence::connect(&tmp).await;

        let span_dirs = {
            let inner = storage.inner.read().await;
            inner.span_dirs()
        };
        for view in 1000u64..2000 {
            for (dir, ext) in &span_dirs {
                write_dummy(dir, view, ext);
            }
        }

        // Warm up: this decide runs the one-time startup sweep.
        storage
            .append_decided_leaves(ViewNumber::new(0), [], None, &NullEventConsumer)
            .await
            .unwrap();

        for decided in 1u64..=3 {
            let before = storage.inner.read().await.view_files_scans();
            storage
                .append_decided_leaves(ViewNumber::new(decided), [], None, &NullEventConsumer)
                .await
                .unwrap();
            let after = storage.inner.read().await.view_files_scans();
            assert_eq!(
                after - before,
                1,
                "only the decided_leaves2 read in phase 1 should scan a directory"
            );
        }

        // The backlog is well inside the retention window and never decided, so nothing
        // collects it; this is what makes the scan count constant rather than merely bounded.
        for view in [1000u64, 1999] {
            for (dir, ext) in &span_dirs {
                assert!(dir.join(view.to_string()).with_extension(ext).exists());
            }
        }
    }

    /// `da2` (first in `span_dirs`) is made unwritable after seeding a real file in it, so
    /// unlink fails with a permission error
    /// rather than the vacuous `NotFound` a missing file would give (removing a file that was
    /// never there returns `NotFound` before the permission check runs). Being first, and given
    /// the per-directory break-on-first-failure in the span loop, `da2` fails and stops early on
    /// its own span while `vid2`/`quorum_proposals2`/`state_cert` (processed after it) are what
    /// this test actually exercises. The canaries sit at views 1 and 5, inside each decide's
    /// interval span but never decided themselves, so `load_pending_decides` never tries to parse
    /// them. Two decides of two leaves each still collect the other span directories, and
    /// `decided_leaves2` stays at its steady-state size of 2 instead of growing.
    #[cfg(unix)]
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_gc_survives_one_failing_directory() {
        let tmp = Persistence::tmp_storage().await;
        let storage = Persistence::connect(&tmp).await;
        let leaves = chain_with_views_and_heights(&[(0, 0), (2, 1), (4, 2), (6, 3)]).await;

        let (da2_dir, span_dirs) = {
            let inner = storage.inner.read().await;
            (inner.da2_dir_path(), inner.span_dirs())
        };
        for view in [1u64, 5] {
            for (dir, ext) in &span_dirs {
                write_dummy(dir, view, ext);
            }
        }

        fs::set_permissions(&da2_dir, fs::Permissions::from_mode(0o500)).unwrap();

        // Root ignores directory write permission bits, so the isolation under test cannot be
        // forced; detect that and skip rather than assert something meaningless.
        let root_ignores_permissions =
            fs::remove_file(da2_dir.join("1").with_extension("txt")).is_ok();
        if root_ignores_permissions {
            fs::set_permissions(&da2_dir, fs::Permissions::from_mode(0o700)).unwrap();
            return;
        }

        decide_leaves(
            &storage,
            &leaves[0..2],
            ViewNumber::new(2),
            &NullEventConsumer,
        )
        .await;
        decide_leaves(
            &storage,
            &leaves[2..4],
            ViewNumber::new(6),
            &NullEventConsumer,
        )
        .await;

        fs::set_permissions(&da2_dir, fs::Permissions::from_mode(0o700)).unwrap();

        for view in [1u64, 5] {
            for (dir, ext) in span_dirs.iter().filter(|(dir, _)| *dir != da2_dir) {
                assert!(
                    !dir.join(view.to_string()).with_extension(ext).exists(),
                    "view {view} should be gone from {} despite da2 failing",
                    dir.display()
                );
            }
        }

        let leaves2_dir = storage.inner.read().await.decided_leaf2_path();
        assert_eq!(
            views_in_dir(&leaves2_dir).len(),
            2,
            "decided_leaves2 must stay at its steady-state size, not grow, when another directory \
             permanently fails"
        );
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_gc_keeps_anchor_leaf() {
        let tmp = Persistence::tmp_storage().await;
        let storage = Persistence::connect(&tmp).await;
        let leaves = consecutive_height_chain(3).await;

        decide_leaves(&storage, &leaves, ViewNumber::new(2), &NullEventConsumer).await;

        let (anchor, _) = storage.load_anchor_leaf().await.unwrap().unwrap();
        assert_eq!(anchor.view_number(), ViewNumber::new(2));

        let leaves2_dir = storage.inner.read().await.decided_leaf2_path();
        assert!(leaves2_dir.join("2.txt").exists());
    }

    /// Pins the property `emit_decides` running off-lock depends on: every concurrent writer
    /// targets views strictly above `decided_view`, so GC unlinking by computed path never races
    /// a live append.
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_gc_above_decided_untouched() {
        let tmp = Persistence::tmp_storage().await;
        let storage = Persistence::connect(&tmp).await;
        let leaves = consecutive_height_chain(2).await;

        let dirs = {
            let inner = storage.inner.read().await;
            inner.pruned_dirs()
        };
        for (dir, ext) in &dirs {
            write_dummy(dir, 2, ext);
        }

        decide_leaves(&storage, &leaves, ViewNumber::new(1), &NullEventConsumer).await;

        for (dir, ext) in &dirs {
            assert!(
                dir.join("2").with_extension(*ext).exists(),
                "a view above decided_view must survive GC: {}",
                dir.display()
            );
        }
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_gc_missing_dir() {
        let tmp = Persistence::tmp_storage().await;
        let storage = Persistence::connect(&tmp).await;

        storage
            .inner
            .write()
            .await
            .collect_garbage(ViewNumber::new(0), &[], &[])
            .unwrap();
    }

    #[derive(Clone, Debug)]
    struct AppendingConsumer {
        storage: Persistence,
    }

    #[async_trait]
    impl EventConsumer for AppendingConsumer {
        async fn handle_event(&self, _event: &CoordinatorEvent<SeqTypes>) -> anyhow::Result<()> {
            let qc = QuorumCertificate2::genesis(
                &ValidatedState::default(),
                &NodeState::mock(),
                TEST_VERSIONS.test,
            )
            .await;
            self.storage.append_high_qc2(qc).await
        }
    }

    /// Under the pre-split code, the consumer's write-lock-taking call deadlocks against the
    /// write lock `process_decided_events` still holds; wrapped in a timeout so a regression
    /// hangs the test instead of the run.
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_decide_releases_lock_before_consumer() {
        let tmp = Persistence::tmp_storage().await;
        let storage = Persistence::connect(&tmp).await;
        let leaves = consecutive_height_chain(1).await;
        let consumer = AppendingConsumer {
            storage: storage.clone(),
        };

        let leaf_chain = leaves
            .iter()
            .map(|(leaf, qc)| (leaf_info(leaf.clone()), qc.clone()))
            .collect::<Vec<_>>();
        storage
            .persist_decided_leaves(
                ViewNumber::new(0),
                leaf_chain
                    .iter()
                    .map(|(leaf, qc)| (leaf, CertificatePair::non_epoch_change(qc.clone()))),
                None,
                &consumer,
            )
            .await
            .unwrap();

        tokio::time::timeout(
            Duration::from_secs(5),
            storage.process_decided_events(ViewNumber::new(0), None, &consumer),
        )
        .await
        .expect("consumer append must not deadlock behind the persistence write lock")
        .unwrap();
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_enable_metrics_registers_histograms() {
        let tmp = Persistence::tmp_storage().await;
        let mut opt = Persistence::options(&tmp);
        let mut storage = opt.create().await.unwrap();

        let metrics = PrometheusMetrics::default();
        storage.enable_metrics(&metrics);

        let leaves = consecutive_height_chain(1).await;
        storage.append_vid(&vid_proposal(0).await).await.unwrap();
        decide_leaves(&storage, &leaves, ViewNumber::new(0), &NullEventConsumer).await;

        assert_eq!(
            metrics
                .get_histogram("internal_append_vid_duration")
                .unwrap()
                .sample_count(),
            1
        );
        assert!(
            metrics
                .get_histogram("internal_process_decided_events_duration")
                .unwrap()
                .sample_count()
                >= 1
        );
    }
}
