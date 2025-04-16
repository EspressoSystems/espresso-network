use std::{
    collections::{BTreeMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    ops::RangeInclusive,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{anyhow, Context};
use async_lock::RwLock;
use async_trait::async_trait;
use clap::Parser;
use espresso_types::{
    traits::MembershipPersistence,
    v0::traits::{EventConsumer, PersistenceOptions, SequencerPersistence},
    v0_3::{EventKey, IndexedStake, StakeTableEvent, Validator},
    Leaf, Leaf2, NetworkConfig, Payload, SeqTypes,
};
use hotshot::{types::BLSPubKey, InitializerEpochInfo};
use hotshot_types::{
    data::{
        vid_disperse::{ADVZDisperseShare, VidDisperseShare2},
        DaProposal, DaProposal2, EpochNumber, QuorumProposal, QuorumProposal2,
        QuorumProposalWrapper, VidCommitment, VidDisperseShare,
    },
    drb::DrbResult,
    event::{Event, EventType, HotShotAction, LeafInfo},
    message::{convert_proposal, Proposal},
    simple_certificate::{
        LightClientStateUpdateCertificate, NextEpochQuorumCertificate2, QuorumCertificate,
        QuorumCertificate2, UpgradeCertificate,
    },
    traits::{
        block_contents::{BlockHeader, BlockPayload},
        node_implementation::{ConsensusTime, NodeType},
    },
    vote::HasViewNumber,
};
use indexmap::IndexMap;
use itertools::Itertools;

use crate::ViewNumber;

/// Options for file system backed persistence.
#[derive(Parser, Clone, Debug)]
pub struct Options {
    /// Storage path for persistent data.
    #[clap(long, env = "ESPRESSO_SEQUENCER_STORAGE_PATH")]
    path: PathBuf,

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
        env = "ESPRESSO_SEQUENCER_CONSENSUS_VIEW_RETENTION",
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

        let migration_path = path.join("migration");
        let migrated = if migration_path.is_file() {
            let bytes = fs::read(&migration_path).context(format!(
                "unable to read migration from {}",
                migration_path.display()
            ))?;
            bincode::deserialize(&bytes).context("malformed migration file")?
        } else {
            HashSet::new()
        };

        Ok(Persistence {
            inner: Arc::new(RwLock::new(Inner {
                path,
                migrated,
                view_retention,
            })),
        })
    }

    async fn reset(self) -> anyhow::Result<()> {
        todo!()
    }
}

/// File system backed persistence.
#[derive(Clone, Debug)]
pub struct Persistence {
    // We enforce mutual exclusion on access to the data source, as the current file system
    // implementation does not support transaction isolation for concurrent reads and writes. We can
    // improve this in the future by switching to a SQLite-based file system implementation.
    inner: Arc<RwLock<Inner>>,
}

#[derive(Debug)]
struct Inner {
    path: PathBuf,
    view_retention: u64,
    migrated: HashSet<String>,
}

impl Inner {
    fn config_path(&self) -> PathBuf {
        self.path.join("hotshot.cfg")
    }

    fn migration(&self) -> PathBuf {
        self.path.join("migration")
    }

    fn voted_view_path(&self) -> PathBuf {
        self.path.join("highest_voted_view")
    }

    /// Path to a directory containing decided leaves.
    fn decided_leaf_path(&self) -> PathBuf {
        self.path.join("decided_leaves")
    }

    fn decided_leaf2_path(&self) -> PathBuf {
        self.path.join("decided_leaves2")
    }

    /// The path from previous versions where there was only a single file for anchor leaves.
    fn legacy_anchor_leaf_path(&self) -> PathBuf {
        self.path.join("anchor_leaf")
    }

    fn vid_dir_path(&self) -> PathBuf {
        self.path.join("vid")
    }

    fn vid2_dir_path(&self) -> PathBuf {
        self.path.join("vid2")
    }

    fn da_dir_path(&self) -> PathBuf {
        self.path.join("da")
    }

    fn da2_dir_path(&self) -> PathBuf {
        self.path.join("da2")
    }

    fn quorum_proposals_dir_path(&self) -> PathBuf {
        self.path.join("quorum_proposals")
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

    fn update_migration(&mut self) -> anyhow::Result<()> {
        let path = self.migration();
        let bytes = bincode::serialize(&self.migrated)?;

        self.replace(
            &path,
            |_| Ok(true),
            |mut file| {
                file.write_all(&bytes)?;
                Ok(())
            },
        )
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

    fn collect_garbage(
        &mut self,
        decided_view: ViewNumber,
        prune_intervals: &[RangeInclusive<ViewNumber>],
    ) -> anyhow::Result<()> {
        let prune_view = ViewNumber::new(decided_view.saturating_sub(self.view_retention));

        self.prune_files(self.da2_dir_path(), prune_view, None, prune_intervals)?;
        self.prune_files(self.vid2_dir_path(), prune_view, None, prune_intervals)?;
        self.prune_files(
            self.quorum_proposals2_dir_path(),
            prune_view,
            None,
            prune_intervals,
        )?;
        self.prune_files(
            self.state_cert_dir_path(),
            prune_view,
            None,
            prune_intervals,
        )?;

        // Save the most recent leaf as it will be our anchor point if the node restarts.
        self.prune_files(
            self.decided_leaf2_path(),
            prune_view,
            Some(decided_view),
            prune_intervals,
        )?;

        Ok(())
    }

    fn prune_files(
        &mut self,
        dir_path: PathBuf,
        prune_view: ViewNumber,
        keep_decided_view: Option<ViewNumber>,
        prune_intervals: &[RangeInclusive<ViewNumber>],
    ) -> anyhow::Result<()> {
        if !dir_path.is_dir() {
            return Ok(());
        }

        for (file_view, path) in view_files(dir_path)? {
            // If the view is the anchor view, keep it no matter what.
            if let Some(decided_view) = keep_decided_view {
                if decided_view == file_view {
                    continue;
                }
            }
            // Otherwise, delete it if it is time to prune this view _or_ if the given intervals,
            // which we've already successfully processed, contain the view; in this case we simply
            // don't need it anymore.
            if file_view < prune_view || prune_intervals.iter().any(|i| i.contains(&file_view)) {
                fs::remove_file(&path)?;
            }
        }

        Ok(())
    }

    /// Generate events based on persisted decided leaves.
    ///
    /// Returns a list of closed intervals of views which can be safely deleted, as all leaves
    /// within these view ranges have been processed by the event consumer.
    async fn generate_decide_events(
        &self,
        view: ViewNumber,
        consumer: &impl EventConsumer,
    ) -> anyhow::Result<Vec<RangeInclusive<ViewNumber>>> {
        // Generate a decide event for each leaf, to be processed by the event consumer. We make a
        // separate event for each leaf because it is possible we have non-consecutive leaves in our
        // storage, which would not be valid as a single decide with a single leaf chain.
        let mut leaves = BTreeMap::new();
        for (v, path) in view_files(self.decided_leaf2_path())? {
            if v > view {
                continue;
            }

            let bytes =
                fs::read(&path).context(format!("reading decided leaf {}", path.display()))?;
            let (mut leaf, qc) =
                bincode::deserialize::<(Leaf2, QuorumCertificate2<SeqTypes>)>(&bytes)
                    .context(format!("parsing decided leaf {}", path.display()))?;

            // Include the VID share if available.
            let vid_share = self.load_vid_share(v)?.map(|proposal| proposal.data);
            if vid_share.is_none() {
                tracing::debug!(?v, "VID share not available at decide");
            }

            // Move the state cert to the finalized dir if it exists.
            let state_cert = self.finalized_state_cert(v)?;

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

            leaves.insert(v, (info, qc));
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

        let mut intervals = vec![];
        let mut current_interval = None;
        for (view, (leaf, qc)) in leaves {
            let height = leaf.leaf.block_header().block_number();
            consumer
                .handle_event(&Event {
                    view_number: view,
                    event: EventType::Decide {
                        qc: Arc::new(qc),
                        leaf_chain: Arc::new(vec![leaf]),
                        block_size: None,
                    },
                })
                .await?;
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

    fn load_anchor_leaf(&self) -> anyhow::Result<Option<(Leaf2, QuorumCertificate2<SeqTypes>)>> {
        tracing::info!("Checking `Leaf2` to load the anchor leaf.");
        if self.decided_leaf2_path().is_dir() {
            let mut anchor: Option<(Leaf2, QuorumCertificate2<SeqTypes>)> = None;

            // Return the latest decided leaf.
            for (_, path) in view_files(self.decided_leaf2_path())? {
                let bytes =
                    fs::read(&path).context(format!("reading decided leaf {}", path.display()))?;
                let (leaf2, qc2) =
                    bincode::deserialize::<(Leaf2, QuorumCertificate2<SeqTypes>)>(&bytes)
                        .context(format!("parsing decided leaf {}", path.display()))?;
                if let Some((anchor_leaf, _)) = &anchor {
                    if leaf2.view_number() > anchor_leaf.view_number() {
                        anchor = Some((leaf2, qc2));
                    }
                } else {
                    anchor = Some((leaf2, qc2));
                }
            }

            return Ok(anchor);
        }

        tracing::warn!("Failed to find an anchor leaf in `Leaf2` storage. Checking legacy `Leaf` storage. This is very likely to fail.");
        if self.legacy_anchor_leaf_path().is_file() {
            // We may have an old version of storage, where there is just a single file for the
            // anchor leaf. Read it and return the contents.
            let mut file = File::open(self.legacy_anchor_leaf_path())?;

            // The first 8 bytes just contain the height of the leaf. We can skip this.
            file.seek(SeekFrom::Start(8)).context("seek")?;
            let bytes = file
                .bytes()
                .collect::<Result<Vec<_>, _>>()
                .context("read")?;
            return Ok(Some(bincode::deserialize(&bytes).context("deserialize")?));
        }

        Ok(None)
    }

    fn finalized_state_cert(
        &self,
        view: ViewNumber,
    ) -> anyhow::Result<Option<LightClientStateUpdateCertificate<SeqTypes>>> {
        let dir_path = self.state_cert_dir_path();

        let file_path = dir_path.join(view.u64().to_string()).with_extension("txt");

        if file_path.exists() {
            let bytes = fs::read(file_path)?;
            let state_cert: LightClientStateUpdateCertificate<SeqTypes> =
                bincode::deserialize(&bytes)?;
            let epoch = state_cert.epoch.u64();
            let finalized_dir_path = self.finalized_state_cert_dir_path();
            fs::create_dir_all(&finalized_dir_path).context("creating finalized state cert dir")?;
            let finalized_file_path = finalized_dir_path
                .join(epoch.to_string())
                .with_extension("txt");
            fs::write(finalized_file_path, bytes).context(format!(
                "finalizing light client state update certificate file for epoch {epoch:?}"
            ))?;
            return Ok(Some(state_cert));
        }

        Ok(None)
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

    async fn append_decided_leaves(
        &self,
        view: ViewNumber,
        leaf_chain: impl IntoIterator<Item = (&LeafInfo<SeqTypes>, QuorumCertificate2<SeqTypes>)> + Send,
        consumer: &impl EventConsumer,
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
            fs::write(new_file, bytes).context(format!("writing anchor leaf file {view}"))?;

            // Now we can remove the old file.
            fs::remove_file(&legacy_path).context("removing legacy anchor leaf file")?;
        }

        for (info, qc2) in leaf_chain {
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
                    let bytes = bincode::serialize(&(&info.leaf.clone(), qc2))?;
                    file.write_all(&bytes)?;
                    Ok(())
                },
            )?;
        }

        match inner.generate_decide_events(view, consumer).await {
            Err(err) => {
                // Event processing failure is not an error, since by this point we have at least
                // managed to persist the decided leaves successfully, and the event processing will
                // just run again at the next decide.
                tracing::warn!(?view, "event processing failed: {err:#}");
            },
            Ok(intervals) => {
                if let Err(err) = inner.collect_garbage(view, &intervals) {
                    // Similarly, garbage collection is not an error. We have done everything we
                    // strictly needed to do, and GC will run again at the next decide. Log the
                    // error but do not return it.
                    tracing::warn!(?view, "GC failed: {err:#}");
                }
            },
        }

        Ok(())
    }

    async fn load_anchor_leaf(
        &self,
    ) -> anyhow::Result<Option<(Leaf2, QuorumCertificate2<SeqTypes>)>> {
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
        proposal: &Proposal<SeqTypes, ADVZDisperseShare<SeqTypes>>,
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
                let proposal: Proposal<SeqTypes, VidDisperseShare<SeqTypes>> =
                    convert_proposal(proposal.clone());
                let proposal_bytes = bincode::serialize(&proposal).context("serialize proposal")?;
                file.write_all(&proposal_bytes)?;
                Ok(())
            },
        )
    }
    async fn append_vid2(
        &self,
        proposal: &Proposal<SeqTypes, VidDisperseShare2<SeqTypes>>,
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
                let proposal: Proposal<SeqTypes, VidDisperseShare<SeqTypes>> =
                    convert_proposal(proposal.clone());
                let proposal_bytes = bincode::serialize(&proposal).context("serialize proposal")?;
                file.write_all(&proposal_bytes)?;
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
                file.write_all(&proposal_bytes)?;
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
        // Todo Remove this after https://github.com/EspressoSystems/espresso-sequencer/issues/1931
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
        )
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

                file.write_all(&proposal_bytes)?;
                Ok(())
            },
        )
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
        for (view, path) in view_files(&dir_path)? {
            let proposal_bytes = fs::read(path)?;
            let proposal: Proposal<SeqTypes, QuorumProposal2<SeqTypes>> =
                match bincode::deserialize(&proposal_bytes) {
                    Ok(proposal) => proposal,
                    Err(err) => {
                        // At this point, if the file contents are invalid, it is most likely an
                        // error rather than a miscellaneous file somehow ending up in the
                        // directory. However, we continue on, because it is better to collect as
                        // many proposals as we can rather than letting one bad proposal cause the
                        // entire operation to fail, and it is still possible that this was just
                        // some unintended file whose name happened to match the naming convention.
                        tracing::warn!(?view, "ignoring malformed quorum proposal file: {err:#}");
                        continue;
                    },
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
        let proposal = bincode::deserialize(&bytes)?;

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

    async fn store_next_epoch_quorum_certificate(
        &self,
        high_qc: NextEpochQuorumCertificate2<SeqTypes>,
    ) -> anyhow::Result<()> {
        let mut inner = self.inner.write().await;
        let path = &inner.next_epoch_qc();

        inner.replace(
            path,
            |_| {
                // Always overwrite the previous file.
                Ok(true)
            },
            |mut file| {
                let bytes = bincode::serialize(&high_qc).context("serializing next epoch qc")?;
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
                file.write_all(&proposal_bytes)?;
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

    async fn migrate_anchor_leaf(&self) -> anyhow::Result<()> {
        let mut inner = self.inner.write().await;

        if inner.migrated.contains("anchor_leaf") {
            tracing::info!("decided leaves already migrated");
            return Ok(());
        }

        let new_leaf_dir = inner.decided_leaf2_path();

        fs::create_dir_all(new_leaf_dir.clone()).context("failed to create anchor leaf 2  dir")?;

        let old_leaf_dir = inner.decided_leaf_path();
        if !old_leaf_dir.is_dir() {
            return Ok(());
        }

        tracing::warn!("migrating decided leaves..");
        for entry in fs::read_dir(old_leaf_dir)? {
            let entry = entry?;
            let path = entry.path();

            let Some(file) = path.file_stem().and_then(|n| n.to_str()) else {
                continue;
            };
            let Ok(view) = file.parse::<u64>() else {
                continue;
            };

            let bytes =
                fs::read(&path).context(format!("reading decided leaf {}", path.display()))?;
            let (leaf, qc) = bincode::deserialize::<(Leaf, QuorumCertificate<SeqTypes>)>(&bytes)
                .context(format!("parsing decided leaf {}", path.display()))?;

            let leaf2: Leaf2 = leaf.into();
            let qc2 = qc.to_qc2();

            let new_leaf_path = new_leaf_dir.join(view.to_string()).with_extension("txt");

            inner.replace(
                &new_leaf_path,
                |_| {
                    tracing::warn!(view, "duplicate decided leaf");
                    Ok(false)
                },
                |mut file| {
                    let bytes = bincode::serialize(&(&leaf2.clone(), qc2))?;
                    file.write_all(&bytes)?;
                    Ok(())
                },
            )?;

            if view % 100 == 0 {
                tracing::info!(view, "decided leaves migration progress");
            }
        }

        inner.migrated.insert("anchor_leaf".to_string());
        inner.update_migration()?;
        tracing::warn!("successfully migrated decided leaves");
        Ok(())
    }
    async fn migrate_da_proposals(&self) -> anyhow::Result<()> {
        let mut inner = self.inner.write().await;

        if inner.migrated.contains("da_proposal") {
            tracing::info!("da proposals already migrated");
            return Ok(());
        }

        let new_da_dir = inner.da2_dir_path();

        fs::create_dir_all(new_da_dir.clone()).context("failed to create da proposals 2 dir")?;

        let old_da_dir = inner.da_dir_path();
        if !old_da_dir.is_dir() {
            return Ok(());
        }

        tracing::warn!("migrating da proposals..");

        for entry in fs::read_dir(old_da_dir)? {
            let entry = entry?;
            let path = entry.path();

            let Some(file) = path.file_stem().and_then(|n| n.to_str()) else {
                continue;
            };
            let Ok(view) = file.parse::<u64>() else {
                continue;
            };

            let bytes =
                fs::read(&path).context(format!("reading da proposal {}", path.display()))?;
            let proposal = bincode::deserialize::<Proposal<SeqTypes, DaProposal<SeqTypes>>>(&bytes)
                .context(format!("parsing da proposal {}", path.display()))?;

            let new_da_path = new_da_dir.join(view.to_string()).with_extension("txt");

            let proposal2: Proposal<SeqTypes, DaProposal2<SeqTypes>> = convert_proposal(proposal);

            inner.replace(
                &new_da_path,
                |_| {
                    tracing::warn!(view, "duplicate DA proposal 2");
                    Ok(false)
                },
                |mut file| {
                    let bytes = bincode::serialize(&proposal2)?;
                    file.write_all(&bytes)?;
                    Ok(())
                },
            )?;

            if view % 100 == 0 {
                tracing::info!(view, "DA proposals migration progress");
            }
        }

        inner.migrated.insert("da_proposal".to_string());
        inner.update_migration()?;
        tracing::warn!("successfully migrated da proposals");
        Ok(())
    }
    async fn migrate_vid_shares(&self) -> anyhow::Result<()> {
        let mut inner = self.inner.write().await;

        if inner.migrated.contains("vid_share") {
            tracing::info!("vid shares already migrated");
            return Ok(());
        }

        let new_vid_dir = inner.vid2_dir_path();

        fs::create_dir_all(new_vid_dir.clone()).context("failed to create vid shares 2 dir")?;

        let old_vid_dir = inner.vid_dir_path();
        if !old_vid_dir.is_dir() {
            return Ok(());
        }

        tracing::warn!("migrating vid shares..");

        for entry in fs::read_dir(old_vid_dir)? {
            let entry = entry?;
            let path = entry.path();

            let Some(file) = path.file_stem().and_then(|n| n.to_str()) else {
                continue;
            };
            let Ok(view) = file.parse::<u64>() else {
                continue;
            };

            let bytes = fs::read(&path).context(format!("reading vid share {}", path.display()))?;
            let proposal =
                bincode::deserialize::<Proposal<SeqTypes, ADVZDisperseShare<SeqTypes>>>(&bytes)
                    .context(format!("parsing vid share {}", path.display()))?;

            let new_vid_path = new_vid_dir.join(view.to_string()).with_extension("txt");

            let proposal2: Proposal<SeqTypes, VidDisperseShare<SeqTypes>> =
                convert_proposal(proposal);

            inner.replace(
                &new_vid_path,
                |_| {
                    tracing::warn!(view, "duplicate VID share ");
                    Ok(false)
                },
                |mut file| {
                    let bytes = bincode::serialize(&proposal2)?;
                    file.write_all(&bytes)?;
                    Ok(())
                },
            )?;

            if view % 100 == 0 {
                tracing::info!(view, "VID shares migration progress");
            }
        }

        inner.migrated.insert("vid_share".to_string());
        inner.update_migration()?;
        tracing::warn!("successfully migrated vid shares");
        Ok(())
    }

    async fn migrate_quorum_proposals(&self) -> anyhow::Result<()> {
        let mut inner = self.inner.write().await;

        if inner.migrated.contains("quorum_proposals") {
            tracing::info!("quorum proposals already migrated");
            return Ok(());
        }

        let new_quorum_proposals_dir = inner.quorum_proposals2_dir_path();

        fs::create_dir_all(new_quorum_proposals_dir.clone())
            .context("failed to create quorum proposals 2 dir")?;

        let old_quorum_proposals_dir = inner.quorum_proposals_dir_path();
        if !old_quorum_proposals_dir.is_dir() {
            tracing::info!("no existing quorum proposals found for migration");
            return Ok(());
        }

        tracing::warn!("migrating quorum proposals..");
        for entry in fs::read_dir(old_quorum_proposals_dir)? {
            let entry = entry?;
            let path = entry.path();

            let Some(file) = path.file_stem().and_then(|n| n.to_str()) else {
                continue;
            };
            let Ok(view) = file.parse::<u64>() else {
                continue;
            };

            let bytes =
                fs::read(&path).context(format!("reading quorum proposal {}", path.display()))?;
            let proposal =
                bincode::deserialize::<Proposal<SeqTypes, QuorumProposal<SeqTypes>>>(&bytes)
                    .context(format!("parsing quorum proposal {}", path.display()))?;

            let new_file_path = new_quorum_proposals_dir
                .join(view.to_string())
                .with_extension("txt");

            let proposal2: Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>> =
                convert_proposal(proposal);

            inner.replace(
                &new_file_path,
                |_| {
                    tracing::warn!(view, "duplicate Quorum proposal2 ");
                    Ok(false)
                },
                |mut file| {
                    let bytes = bincode::serialize(&proposal2)?;
                    file.write_all(&bytes)?;
                    Ok(())
                },
            )?;

            if view % 100 == 0 {
                tracing::info!(view, "Quorum proposals migration progress");
            }
        }

        inner.migrated.insert("quorum_proposals".to_string());
        inner.update_migration()?;
        tracing::warn!("successfully migrated quorum proposals");
        Ok(())
    }
    async fn migrate_quorum_certificates(&self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn add_drb_result(
        &self,
        epoch: EpochNumber,
        drb_result: DrbResult,
    ) -> anyhow::Result<()> {
        let inner = self.inner.write().await;
        let dir_path = inner.epoch_drb_result_dir_path();

        fs::create_dir_all(dir_path.clone()).context("failed to create epoch drb result dir")?;

        let drb_result_bytes = bincode::serialize(&drb_result).context("serialize drb result")?;

        let file_path = dir_path.join(epoch.to_string()).with_extension("txt");
        fs::write(file_path, drb_result_bytes)
            .context(format!("writing epoch drb result file for epoch {epoch:?}"))?;

        Ok(())
    }

    async fn add_epoch_root(
        &self,
        epoch: EpochNumber,
        block_header: <SeqTypes as NodeType>::BlockHeader,
    ) -> anyhow::Result<()> {
        let inner = self.inner.write().await;
        let dir_path = inner.epoch_root_block_header_dir_path();

        fs::create_dir_all(dir_path.clone())
            .context("failed to create epoch root block header dir")?;

        let block_header_bytes =
            bincode::serialize(&block_header).context("serialize block header")?;

        let file_path = dir_path.join(epoch.to_string()).with_extension("txt");
        fs::write(file_path, block_header_bytes).context(format!(
            "writing epoch root block header file for epoch {epoch:?}"
        ))?;

        Ok(())
    }

    async fn add_state_cert(
        &self,
        state_cert: LightClientStateUpdateCertificate<SeqTypes>,
    ) -> anyhow::Result<()> {
        let inner = self.inner.write().await;
        // let epoch = state_cert.epoch;
        let view = state_cert.light_client_state.view_number;
        let dir_path = inner.state_cert_dir_path();

        fs::create_dir_all(dir_path.clone())
            .context("failed to create light client state update certificate dir")?;

        let bytes = bincode::serialize(&state_cert)
            .context("serialize light client state update certificate")?;

        let file_path = dir_path.join(view.to_string()).with_extension("txt");
        fs::write(file_path, bytes).context(format!(
            "writing light client state update certificate file for view {view:?}"
        ))?;

        Ok(())
    }

    async fn load_start_epoch_info(&self) -> anyhow::Result<Vec<InitializerEpochInfo<SeqTypes>>> {
        let inner = self.inner.read().await;
        let drb_dir_path = inner.epoch_drb_result_dir_path();
        let block_header_dir_path = inner.epoch_root_block_header_dir_path();

        let mut result = Vec::new();

        if drb_dir_path.is_dir() {
            for (epoch, path) in epoch_files(drb_dir_path)? {
                let bytes = fs::read(&path)
                    .context(format!("reading epoch drb result {}", path.display()))?;
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
                        bincode::deserialize::<<SeqTypes as NodeType>::BlockHeader>(&bytes)
                            .context(format!(
                                "parsing epoch root block header {}",
                                block_header_path.display()
                            ))?,
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
        }

        result.sort_by(|a, b| a.epoch.cmp(&b.epoch));

        Ok(result)
    }

    async fn load_state_cert(
        &self,
    ) -> anyhow::Result<Option<LightClientStateUpdateCertificate<SeqTypes>>> {
        let inner = self.inner.read().await;
        let dir_path = inner.finalized_state_cert_dir_path();

        let mut result = None;

        if !dir_path.is_dir() {
            return Ok(result);
        }
        for (epoch, path) in epoch_files(dir_path)? {
            if result.as_ref().is_some_and(|cert| epoch <= cert.epoch) {
                continue;
            }
            let bytes = fs::read(&path).context(format!(
                "reading light client state update certificate {}",
                path.display()
            ))?;
            result = Some(
                bincode::deserialize::<LightClientStateUpdateCertificate<SeqTypes>>(&bytes)
                    .context(format!(
                        "parsing light client state update certificate {}",
                        path.display()
                    ))?,
            );
        }

        Ok(result)
    }
}

#[async_trait]
impl MembershipPersistence for Persistence {
    async fn load_stake(
        &self,
        epoch: EpochNumber,
    ) -> anyhow::Result<Option<IndexMap<alloy::primitives::Address, Validator<BLSPubKey>>>> {
        let inner = self.inner.read().await;
        let path = &inner.stake_table_dir_path();
        let file_path = path.join(epoch.to_string()).with_extension("txt");
        let bytes = fs::read(&file_path).context("read")?;
        Ok(Some(
            bincode::deserialize(&bytes).context("deserialize combined stake table")?,
        ))
    }

    async fn load_latest_stake(&self, limit: u64) -> anyhow::Result<Option<Vec<IndexedStake>>> {
        let limit = limit as usize;
        let inner = self.inner.read().await;
        let path = &inner.stake_table_dir_path();
        let sorted: Vec<_> = epoch_files(path)?
            .sorted_unstable_by_key(|t| t.0)
            .collect::<Vec<_>>();

        let len = sorted.len();
        let mut slice = &sorted[..];
        if len > limit {
            slice = &sorted[len - limit..len - 1]
        };
        slice
            .iter()
            .map(|(epoch, path)| -> anyhow::Result<Option<IndexedStake>> {
                let bytes = fs::read(path).context("read")?;
                let st =
                    bincode::deserialize(&bytes).context("deserialize combined stake table")?;
                Ok(Some((*epoch, st)))
            })
            .collect()
    }

    async fn store_stake(
        &self,
        epoch: EpochNumber,
        stake: IndexMap<alloy::primitives::Address, Validator<BLSPubKey>>,
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
                let bytes =
                    bincode::serialize(&stake).context("serializing combined stake table")?;
                file.write_all(&bytes)?;
                Ok(())
            },
        )
    }

    async fn store_events(
        &self,
        l1_block: u64,
        events: Vec<(EventKey, StakeTableEvent)>,
    ) -> anyhow::Result<()> {
        let mut inner = self.inner.write().await;
        let dir_path = &inner.stake_table_dir_path();
        let events_dir = dir_path.join("events");

        fs::create_dir_all(events_dir.clone()).context("failed to create events dir")?;

        let file_path = events_dir.with_extension("txt");

        inner.replace(
            &file_path,
            |_| Ok(true),
            |file| {
                let writer = BufWriter::new(file);
                serde_json::to_writer_pretty(writer, &(l1_block, events))?;
                Ok(())
            },
        )
    }

    async fn load_events(&self) -> anyhow::Result<Option<(u64, Vec<(EventKey, StakeTableEvent)>)>> {
        let inner = self.inner.write().await;
        let dir_path = &inner.stake_table_dir_path();
        let events_dir = dir_path.join("events");

        let file_path = events_dir.with_extension("txt");

        if !file_path.exists() {
            return Ok(None);
        }

        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let (l1_block, events) = serde_json::from_reader(reader)?;
        Ok(Some((l1_block, events)))
    }
}

/// Update a `NetworkConfig` that may have originally been persisted with an old version.
fn migrate_network_config(
    mut network_config: serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    let config = network_config
        .get_mut("config")
        .context("missing field `config`")?
        .as_object_mut()
        .context("`config` must be an object")?;

    if !config.contains_key("builder_urls") {
        // When multi-builder support was added, the configuration field `builder_url: Url` was
        // replaced by an array `builder_urls: Vec<Url>`. If the saved config has no `builder_urls`
        // field, it is older than this change. Populate `builder_urls` with a singleton array
        // formed from the old value of `builder_url`, and delete the no longer used `builder_url`.
        let url = config
            .remove("builder_url")
            .context("missing field `builder_url`")?;
        config.insert("builder_urls".into(), vec![url].into());
    }

    // HotShotConfig was upgraded to include parameters for proposing and voting on upgrades.
    // Configs which were persisted before this upgrade may be missing these parameters. This
    // migration initializes them with a default. By default, we use JS MAX_SAFE_INTEGER for the
    // start parameters so that nodes will never do an upgrade, unless explicitly configured
    // otherwise.
    if !config.contains_key("start_proposing_view") {
        config.insert("start_proposing_view".into(), 9007199254740991u64.into());
    }
    if !config.contains_key("stop_proposing_view") {
        config.insert("stop_proposing_view".into(), 0.into());
    }
    if !config.contains_key("start_voting_view") {
        config.insert("start_voting_view".into(), 9007199254740991u64.into());
    }
    if !config.contains_key("stop_voting_view") {
        config.insert("stop_voting_view".into(), 0.into());
    }
    if !config.contains_key("start_proposing_time") {
        config.insert("start_proposing_time".into(), 9007199254740991u64.into());
    }
    if !config.contains_key("stop_proposing_time") {
        config.insert("stop_proposing_time".into(), 0.into());
    }
    if !config.contains_key("start_voting_time") {
        config.insert("start_voting_time".into(), 9007199254740991u64.into());
    }
    if !config.contains_key("stop_voting_time") {
        config.insert("stop_voting_time".into(), 0.into());
    }

    // HotShotConfig was upgraded to include an `epoch_height` parameter. Initialize with a default
    // if missing.
    if !config.contains_key("epoch_height") {
        config.insert("epoch_height".into(), 0.into());
    }

    Ok(network_config)
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
        if path.extension()? != "txt" {
            tracing::debug!(%dir, ?entry, "ignoring non-text file in data directory");
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
mod testing {
    use tempfile::TempDir;

    use super::{super::testing::TestablePersistence, *};

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
}

#[cfg(test)]
mod generic_tests {
    use super::{super::persistence_tests, Persistence};
    // For some reason this is the only way to import the macro defined in another module of this
    // crate.
    use crate::*;

    instantiate_persistence_tests!(Persistence);
}

#[cfg(test)]
mod test {
    use std::marker::PhantomData;

    use committable::{Commitment, CommitmentBoundsArkless, Committable};
    use espresso_types::{Header, Leaf, NodeState, PubKey, ValidatedState};
    use hotshot::types::SignatureKey;
    use hotshot_example_types::node_types::TestVersions;
    use hotshot_query_service::testing::mocks::MockVersions;
    use hotshot_types::{
        data::{vid_commitment, QuorumProposal2},
        light_client::LightClientState,
        simple_certificate::QuorumCertificate,
        simple_vote::QuorumData,
        traits::{node_implementation::Versions, EncodeBytes},
        vid::advz::advz_scheme,
    };
    use jf_vid::VidScheme;
    use sequencer_utils::test_utils::setup_test;
    use serde_json::json;
    use vbs::version::StaticVersionType;

    use super::*;
    use crate::{persistence::testing::TestablePersistence, BLSPubKey};

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
                "epoch_height": 0
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
                "epoch_height": 0
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
                "epoch_height": 0
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
                "epoch_height": 0
            }
        });

        assert_eq!(migrate_network_config(before.clone()).unwrap(), before);
    }

    #[tokio::test(flavor = "multi_thread")]
    pub async fn test_consensus_migration() {
        setup_test();
        let rows = 300;
        let tmp = Persistence::tmp_storage().await;
        let mut opt = Persistence::options(&tmp);
        let storage = opt.create().await.unwrap();

        let inner = storage.inner.read().await;

        let decided_leaves_path = inner.decided_leaf_path();
        fs::create_dir_all(decided_leaves_path.clone()).expect("failed to create proposals dir");

        let qp_dir_path = inner.quorum_proposals_dir_path();
        fs::create_dir_all(qp_dir_path.clone()).expect("failed to create proposals dir");

        let state_cert_dir_path = inner.state_cert_dir_path();
        fs::create_dir_all(state_cert_dir_path.clone()).expect("failed to create state cert dir");
        drop(inner);

        assert!(storage.load_state_cert().await.unwrap().is_none());

        for i in 0..rows {
            let view = ViewNumber::new(i);
            let validated_state = ValidatedState::default();
            let instance_state = NodeState::default();

            let (pubkey, privkey) = BLSPubKey::generated_from_seed_indexed([0; 32], i);
            let (payload, metadata) =
                Payload::from_transactions([], &validated_state, &instance_state)
                    .await
                    .unwrap();
            let builder_commitment = payload.builder_commitment(&metadata);
            let payload_bytes = payload.encode();

            let payload_commitment = vid_commitment::<TestVersions>(
                &payload_bytes,
                &metadata.encode(),
                4,
                <TestVersions as Versions>::Base::VERSION,
            );

            let block_header = Header::genesis(
                &instance_state,
                payload_commitment,
                builder_commitment,
                metadata,
            );

            let state_cert = LightClientStateUpdateCertificate::<SeqTypes> {
                epoch: EpochNumber::new(i),
                light_client_state: LightClientState {
                    view_number: i,
                    block_height: i,
                    block_comm_root: Default::default(),
                },
                next_stake_table_state: Default::default(),
                signatures: vec![], // filling arbitrary value
            };
            assert!(storage.add_state_cert(state_cert).await.is_ok());

            let null_quorum_data = QuorumData {
                leaf_commit: Commitment::<Leaf>::default_commitment_no_preimage(),
            };

            let justify_qc = QuorumCertificate::new(
                null_quorum_data.clone(),
                null_quorum_data.commit(),
                view,
                None,
                PhantomData,
            );

            let quorum_proposal = QuorumProposal {
                block_header,
                view_number: view,
                justify_qc: justify_qc.clone(),
                upgrade_certificate: None,
                proposal_certificate: None,
            };

            let quorum_proposal_signature =
                BLSPubKey::sign(&privkey, &bincode::serialize(&quorum_proposal).unwrap())
                    .expect("Failed to sign quorum proposal");

            let proposal = Proposal {
                data: quorum_proposal.clone(),
                signature: quorum_proposal_signature,
                _pd: PhantomData,
            };

            let mut leaf = Leaf::from_quorum_proposal(&quorum_proposal);
            leaf.fill_block_payload::<TestVersions>(
                payload,
                4,
                <TestVersions as Versions>::Base::VERSION,
            )
            .unwrap();

            let mut inner = storage.inner.write().await;

            tracing::debug!("inserting decided leaves");
            let file_path = decided_leaves_path
                .join(view.to_string())
                .with_extension("txt");

            tracing::debug!("inserting decided leaves");

            inner
                .replace(
                    &file_path,
                    |_| Ok(true),
                    |mut file| {
                        let bytes = bincode::serialize(&(&leaf.clone(), justify_qc))?;
                        file.write_all(&bytes)?;
                        Ok(())
                    },
                )
                .expect("replace decided leaves");

            let file_path = qp_dir_path.join(view.to_string()).with_extension("txt");

            tracing::debug!("inserting qc for {view}");

            inner
                .replace(
                    &file_path,
                    |_| Ok(true),
                    |mut file| {
                        let proposal_bytes =
                            bincode::serialize(&proposal).context("serialize proposal")?;

                        file.write_all(&proposal_bytes)?;
                        Ok(())
                    },
                )
                .unwrap();

            drop(inner);
            let disperse = advz_scheme(4).disperse(payload_bytes.clone()).unwrap();

            let vid = ADVZDisperseShare::<SeqTypes> {
                view_number: ViewNumber::new(i),
                payload_commitment: Default::default(),
                share: disperse.shares[0].clone(),
                common: disperse.common,
                recipient_key: pubkey,
            };

            let (payload, metadata) =
                Payload::from_transactions([], &ValidatedState::default(), &NodeState::default())
                    .await
                    .unwrap();

            let da = DaProposal::<SeqTypes> {
                encoded_transactions: payload.encode(),
                metadata,
                view_number: ViewNumber::new(i),
            };

            let block_payload_signature =
                BLSPubKey::sign(&privkey, &payload_bytes).expect("Failed to sign block payload");

            let da_proposal = Proposal {
                data: da,
                signature: block_payload_signature,
                _pd: Default::default(),
            };

            tracing::debug!("inserting vid for {view}");
            storage
                .append_vid(&vid.to_proposal(&privkey).unwrap())
                .await
                .unwrap();

            tracing::debug!("inserting da for {view}");
            storage
                .append_da(&da_proposal, VidCommitment::V0(disperse.commit))
                .await
                .unwrap();
        }

        storage.migrate_consensus().await.unwrap();
        let inner = storage.inner.read().await;
        let decided_leaves = fs::read_dir(inner.decided_leaf2_path()).unwrap();
        let decided_leaves_count = decided_leaves
            .filter_map(Result::ok)
            .filter(|e| e.path().is_file())
            .count();
        assert_eq!(
            decided_leaves_count, rows as usize,
            "decided leaves count does not match",
        );

        let da_proposals = fs::read_dir(inner.da2_dir_path()).unwrap();
        let da_proposals_count = da_proposals
            .filter_map(Result::ok)
            .filter(|e| e.path().is_file())
            .count();
        assert_eq!(
            da_proposals_count, rows as usize,
            "da proposals does not match",
        );

        let vids = fs::read_dir(inner.vid2_dir_path()).unwrap();
        let vids_count = vids
            .filter_map(Result::ok)
            .filter(|e| e.path().is_file())
            .count();
        assert_eq!(vids_count, rows as usize, "vid shares count does not match",);

        let qps = fs::read_dir(inner.quorum_proposals2_dir_path()).unwrap();
        let qps_count = qps
            .filter_map(Result::ok)
            .filter(|e| e.path().is_file())
            .count();
        assert_eq!(
            qps_count, rows as usize,
            "quorum proposals count does not match",
        );

        let state_certs = fs::read_dir(inner.state_cert_dir_path()).unwrap();
        let state_cert_count = state_certs
            .filter_map(Result::ok)
            .filter(|e| e.path().is_file())
            .count();
        assert_eq!(
            state_cert_count, rows as usize,
            "light client state update certificate count does not match",
        );

        // Reinitialize the file system persistence using the same path.
        // re run the consensus migration.
        // No changes will occur, as the migration has already been completed.
        let storage = opt.create().await.unwrap();
        storage.migrate_consensus().await.unwrap();

        let inner = storage.inner.read().await;
        let decided_leaves = fs::read_dir(inner.decided_leaf2_path()).unwrap();
        let decided_leaves_count = decided_leaves
            .filter_map(Result::ok)
            .filter(|e| e.path().is_file())
            .count();
        assert_eq!(
            decided_leaves_count, rows as usize,
            "decided leaves count does not match",
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_load_quorum_proposals_invalid_extension() {
        setup_test();

        let tmp = Persistence::tmp_storage().await;
        let storage = Persistence::connect(&tmp).await;

        // Generate a couple of valid quorum proposals.
        let leaf = Leaf2::genesis::<MockVersions>(&Default::default(), &NodeState::mock()).await;
        let privkey = PubKey::generated_from_seed_indexed([0; 32], 1).1;
        let signature = PubKey::sign(&privkey, &[]).unwrap();
        let mut quorum_proposal = Proposal {
            data: QuorumProposalWrapper::<SeqTypes> {
                proposal: QuorumProposal2::<SeqTypes> {
                    epoch: None,
                    block_header: leaf.block_header().clone(),
                    view_number: ViewNumber::genesis(),
                    justify_qc: QuorumCertificate2::genesis::<TestVersions>(
                        &Default::default(),
                        &NodeState::mock(),
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

    #[tokio::test(flavor = "multi_thread")]
    async fn test_load_quorum_proposals_malformed_data() {
        setup_test();

        let tmp = Persistence::tmp_storage().await;
        let storage = Persistence::connect(&tmp).await;

        // Generate a valid quorum proposal.
        let leaf: Leaf2 = Leaf::genesis::<MockVersions>(&Default::default(), &NodeState::mock())
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
                    justify_qc: QuorumCertificate2::genesis::<TestVersions>(
                        &Default::default(),
                        &NodeState::mock(),
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

    #[tokio::test(flavor = "multi_thread")]
    async fn test_membership_persistence() -> anyhow::Result<()> {
        setup_test();

        let tmp = Persistence::tmp_storage().await;
        let mut opt = Persistence::options(&tmp);

        let storage = opt.create().await.unwrap();

        let validator = Validator::mock();
        let mut st = IndexMap::new();
        st.insert(validator.account, validator);
        storage
            .store_stake(EpochNumber::new(10), st.clone())
            .await?;

        let table = storage.load_stake(EpochNumber::new(10)).await?.unwrap();
        assert_eq!(st, table);

        let val2 = Validator::mock();
        let mut st2 = IndexMap::new();
        st2.insert(val2.account, val2);
        storage
            .store_stake(EpochNumber::new(11), st2.clone())
            .await?;

        let tables = storage.load_latest_stake(4).await?.unwrap();
        let mut iter = tables.iter();
        assert_eq!(Some(&(EpochNumber::new(10), st)), iter.next());
        assert_eq!(Some(&(EpochNumber::new(11), st2)), iter.next());
        assert_eq!(None, iter.next());

        Ok(())
    }
}
