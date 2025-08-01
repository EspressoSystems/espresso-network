use anyhow::{anyhow, Context};
use async_lock::RwLock;
use async_trait::async_trait;
use clap::Parser;
use espresso_types::{
    v0::traits::{EventConsumer, PersistenceOptions, SequencerPersistence},
    Leaf, Leaf2, NetworkConfig, Payload, SeqTypes,
};
use hotshot_types::{
    data::{
        vid_disperse::ADVZDisperseShare, DaProposal, EpochNumber, QuorumProposal, QuorumProposal2,
        QuorumProposalWrapper,
    },
    event::{Event, EventType, HotShotAction, LeafInfo},
    message::{convert_proposal, Proposal},
    simple_certificate::{
        NextEpochQuorumCertificate2, QuorumCertificate, QuorumCertificate2, UpgradeCertificate,
    },
    traits::{
        block_contents::{BlockHeader, BlockPayload},
        node_implementation::ConsensusTime,
    },
    vid::VidSchemeType,
    vote::HasViewNumber,
};
use jf_vid::VidScheme;
use std::sync::Arc;
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    ops::RangeInclusive,
    path::{Path, PathBuf},
};

use crate::ViewNumber;

use espresso_types::downgrade_leaf;

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

        Ok(Persistence {
            inner: Arc::new(RwLock::new(Inner {
                path,
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
}

impl Inner {
    fn config_path(&self) -> PathBuf {
        self.path.join("hotshot.cfg")
    }

    fn voted_view_path(&self) -> PathBuf {
        self.path.join("highest_voted_view")
    }

    /// Path to a directory containing decided leaves.
    fn decided_leaf_path(&self) -> PathBuf {
        self.path.join("decided_leaves")
    }

    /// The path from previous versions where there was only a single file for anchor leaves.
    fn legacy_anchor_leaf_path(&self) -> PathBuf {
        self.path.join("anchor_leaf")
    }

    fn vid_dir_path(&self) -> PathBuf {
        self.path.join("vid")
    }

    fn da_dir_path(&self) -> PathBuf {
        self.path.join("da")
    }

    fn quorum_proposals_dir_path(&self) -> PathBuf {
        self.path.join("quorum_proposals")
    }

    fn upgrade_certificate_dir_path(&self) -> PathBuf {
        self.path.join("upgrade_certificate")
    }

    fn next_epoch_qc(&self) -> PathBuf {
        self.path.join("next_epoch_quorum_certificate")
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
        view: ViewNumber,
        intervals: &[RangeInclusive<u64>],
    ) -> anyhow::Result<()> {
        let view_number = view.u64();
        let prune_view = view.saturating_sub(self.view_retention);

        let delete_files =
            |intervals: &[RangeInclusive<u64>], keep, dir_path: PathBuf| -> anyhow::Result<()> {
                if !dir_path.is_dir() {
                    return Ok(());
                }

                for entry in fs::read_dir(dir_path)? {
                    let entry = entry?;
                    let path = entry.path();

                    if let Some(file) = path.file_stem().and_then(|n| n.to_str()) {
                        if let Ok(v) = file.parse::<u64>() {
                            // If the view is the anchor view, keep it no matter what.
                            if let Some(keep) = keep {
                                if keep == v {
                                    continue;
                                }
                            }
                            // Otherwise, delete it if it is time to prune this view _or_ if the
                            // given intervals, which we've already successfully processed, contain
                            // the view; in this case we simply don't need it anymore.
                            if v < prune_view || intervals.iter().any(|i| i.contains(&v)) {
                                fs::remove_file(&path)?;
                            }
                        }
                    }
                }

                Ok(())
            };

        delete_files(intervals, None, self.da_dir_path())?;
        delete_files(intervals, None, self.vid_dir_path())?;
        delete_files(intervals, None, self.quorum_proposals_dir_path())?;

        // Save the most recent leaf as it will be our anchor point if the node restarts.
        delete_files(intervals, Some(view_number), self.decided_leaf_path())?;

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
    ) -> anyhow::Result<Vec<RangeInclusive<u64>>> {
        // Generate a decide event for each leaf, to be processed by the event consumer. We make a
        // separate event for each leaf because it is possible we have non-consecutive leaves in our
        // storage, which would not be valid as a single decide with a single leaf chain.
        let mut leaves = BTreeMap::new();
        for entry in fs::read_dir(self.decided_leaf_path())? {
            let entry = entry?;
            let path = entry.path();

            let Some(file) = path.file_stem().and_then(|n| n.to_str()) else {
                continue;
            };
            let Ok(v) = file.parse::<u64>() else {
                continue;
            };
            if v > view.u64() {
                continue;
            }

            let bytes =
                fs::read(&path).context(format!("reading decided leaf {}", path.display()))?;
            let (mut leaf, qc) =
                bincode::deserialize::<(Leaf, QuorumCertificate<SeqTypes>)>(&bytes)
                    .context(format!("parsing decided leaf {}", path.display()))?;

            // Include the VID share if available.
            let vid_share = self
                .load_vid_share(ViewNumber::new(v))?
                .map(|proposal| proposal.data);
            if vid_share.is_none() {
                tracing::debug!(view = v, "VID share not available at decide");
            }

            // Fill in the full block payload using the DA proposals we had persisted.
            if let Some(proposal) = self.load_da_proposal(ViewNumber::new(v))? {
                let payload = Payload::from_bytes(
                    &proposal.data.encoded_transactions,
                    &proposal.data.metadata,
                );
                leaf.fill_block_payload_unchecked(payload);
            } else {
                tracing::debug!(view = v, "DA proposal not available at decide");
            }

            let info = LeafInfo {
                leaf: leaf.into(),
                vid_share: vid_share.map(Into::into),

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
            if *oldest_view > 0 {
                leaves.pop_first();
            }
        }

        let mut intervals = vec![];
        let mut current_interval = None;
        for (view, (leaf, qc)) in leaves {
            let height = leaf.leaf.block_header().block_number();
            consumer
                .handle_event(&Event {
                    view_number: ViewNumber::new(view),
                    event: EventType::Decide {
                        qc: Arc::new(qc.to_qc2()),
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
    ) -> anyhow::Result<Option<Proposal<SeqTypes, DaProposal<SeqTypes>>>> {
        let dir_path = self.da_dir_path();

        let file_path = dir_path.join(view.u64().to_string()).with_extension("txt");

        if !file_path.exists() {
            return Ok(None);
        }

        let da_bytes = fs::read(file_path)?;

        let da_proposal: Proposal<SeqTypes, DaProposal<SeqTypes>> =
            bincode::deserialize(&da_bytes)?;
        Ok(Some(da_proposal))
    }

    fn load_vid_share(
        &self,
        view: ViewNumber,
    ) -> anyhow::Result<Option<Proposal<SeqTypes, ADVZDisperseShare<SeqTypes>>>> {
        let dir_path = self.vid_dir_path();

        let file_path = dir_path.join(view.u64().to_string()).with_extension("txt");

        if !file_path.exists() {
            return Ok(None);
        }

        let vid_share_bytes = fs::read(file_path)?;
        let vid_share: Proposal<SeqTypes, ADVZDisperseShare<SeqTypes>> =
            bincode::deserialize(&vid_share_bytes)?;
        Ok(Some(vid_share))
    }

    fn load_anchor_leaf(&self) -> anyhow::Result<Option<(Leaf2, QuorumCertificate2<SeqTypes>)>> {
        if self.decided_leaf_path().is_dir() {
            let mut anchor: Option<(Leaf2, QuorumCertificate2<SeqTypes>)> = None;

            // Return the latest decided leaf.
            for entry in
                fs::read_dir(self.decided_leaf_path()).context("opening decided leaf directory")?
            {
                let file = entry.context("reading decided leaf directory")?.path();
                let bytes =
                    fs::read(&file).context(format!("reading decided leaf {}", file.display()))?;
                let (leaf, qc) =
                    bincode::deserialize::<(Leaf, QuorumCertificate<SeqTypes>)>(&bytes)
                        .context(format!("parsing decided leaf {}", file.display()))?;
                if let Some((anchor_leaf, _)) = &anchor {
                    if leaf.view_number() > anchor_leaf.view_number() {
                        let leaf2 = leaf.into();
                        let qc2 = qc.to_qc2();
                        anchor = Some((leaf2, qc2));
                    }
                } else {
                    let leaf2 = leaf.into();
                    let qc2 = qc.to_qc2();
                    anchor = Some((leaf2, qc2));
                }
            }

            return Ok(anchor);
        }

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
        let path = inner.decided_leaf_path();

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
                    let leaf = downgrade_leaf(info.leaf.clone());
                    let qc = qc2.to_qc();
                    let bytes = bincode::serialize(&(&leaf, qc))?;
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
            }
            Ok(intervals) => {
                if let Err(err) = inner.collect_garbage(view, &intervals) {
                    // Similarly, garbage collection is not an error. We have done everything we
                    // strictly needed to do, and GC will run again at the next decide. Log the
                    // error but do not return it.
                    tracing::warn!(?view, "GC failed: {err:#}");
                }
            }
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
    ) -> anyhow::Result<Option<Proposal<SeqTypes, DaProposal<SeqTypes>>>> {
        self.inner.read().await.load_da_proposal(view)
    }

    async fn load_vid_share(
        &self,
        view: ViewNumber,
    ) -> anyhow::Result<Option<Proposal<SeqTypes, ADVZDisperseShare<SeqTypes>>>> {
        self.inner.read().await.load_vid_share(view)
    }

    async fn append_vid(
        &self,
        proposal: &Proposal<SeqTypes, ADVZDisperseShare<SeqTypes>>,
    ) -> anyhow::Result<()> {
        let mut inner = self.inner.write().await;
        let view_number = proposal.data.view_number().u64();
        let dir_path = inner.vid_dir_path();

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
                let proposal_bytes = bincode::serialize(&proposal).context("serialize proposal")?;
                file.write_all(&proposal_bytes)?;
                Ok(())
            },
        )
    }
    async fn append_da(
        &self,
        proposal: &Proposal<SeqTypes, DaProposal<SeqTypes>>,
        _vid_commit: <VidSchemeType as VidScheme>::Commit,
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

    async fn append_quorum_proposal(
        &self,
        proposal: &Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>>,
    ) -> anyhow::Result<()> {
        let proposal: Proposal<SeqTypes, QuorumProposal<SeqTypes>> =
            convert_proposal(proposal.clone());
        let mut inner = self.inner.write().await;
        let view_number = proposal.data.view_number().u64();
        let dir_path = inner.quorum_proposals_dir_path();

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
        let dir_path = inner.quorum_proposals_dir_path();
        if !dir_path.is_dir() {
            return Ok(Default::default());
        }

        // Then, we want to get the entries in this directory since they'll be the
        // key/value pairs for our map.
        let files = fs::read_dir(dir_path.clone())?.filter_map(|entry| {
            let entry = entry.ok()?;
            if entry.file_type().ok()?.is_file() && entry.path().extension()? == "txt" {
                Some(entry.path())
            } else {
                None
            }
        });

        let mut map = BTreeMap::new();
        for file in files {
            // Parse each file into a proposal if possible. We ignore files we don't recognize or
            // can't parse, as sometimes we can end up with random extra files (e.g. swap files) in
            // the directory.
            //
            // Get the stem to remove the ".txt" from the end.
            let Some(file_name) = file.file_stem() else {
                continue;
            };

            // Parse the filename (which corresponds to the view)
            let file_name = file_name.to_string_lossy();
            let Ok(view_number) = file_name.parse::<u64>() else {
                tracing::info!(
                    %file_name,
                    "ignoring extraneous file in quorum proposals directory"
                );
                continue;
            };

            // Now, we'll try and load the proposal associated with this function. In this case we
            // do propagate errors: errors from the file system are more likely to be some transient
            // issue (e.g. failure to connect to a network-mounted file system) than an issue with
            // the file itself (like a swap file being left over in the directory). Thus, this file
            // likely does have good data even if we can't read it, so we do want to propagate the
            // error and eventually retry.
            let proposal_bytes = fs::read(file)?;

            // Then, deserialize.
            let proposal: Proposal<SeqTypes, QuorumProposal<SeqTypes>> =
                match bincode::deserialize(&proposal_bytes) {
                    Ok(proposal) => proposal,
                    Err(err) => {
                        // At this point, if the file contents are invalid, it is most likely an
                        // error rather than a miscellaneous file somehow ending up in the
                        // directory. However, we continue on, because it is better to collect as
                        // many proposals as we can rather than letting one bad proposal cause the
                        // entire operation to fail, and it is still possible that this was just
                        // some unintended file whose name happened to match the naming convention.
                        tracing::warn!(
                            view_number,
                            "ignoring malformed quorum proposal file: {err:#}"
                        );
                        continue;
                    }
                };
            let proposal2 = convert_proposal(proposal);

            // Push to the map and we're done.
            map.insert(ViewNumber::new(view_number), proposal2);
        }

        Ok(map)
    }

    async fn load_quorum_proposal(
        &self,
        view: ViewNumber,
    ) -> anyhow::Result<Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>>> {
        let inner = self.inner.read().await;
        let dir_path = inner.quorum_proposals_dir_path();
        let file_path = dir_path.join(view.to_string()).with_extension("txt");
        let bytes = fs::read(file_path)?;
        let proposal: Proposal<SeqTypes, QuorumProposal<SeqTypes>> = bincode::deserialize(&bytes)?;
        // TODO: rather than converting, we should store the value of QuorumProposalWrapper::with_epoch
        let proposal_wrapper = convert_proposal(proposal);
        Ok(proposal_wrapper)
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

    async fn migrate_consensus(
        &self,
        _migrate_leaf: fn(Leaf) -> Leaf2,
        _migrate_proposal: fn(
            Proposal<SeqTypes, QuorumProposal<SeqTypes>>,
        ) -> Proposal<SeqTypes, QuorumProposal2<SeqTypes>>,
    ) -> anyhow::Result<()> {
        // TODO: https://github.com/EspressoSystems/espresso-sequencer/issues/2357
        Ok(())
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
    use espresso_types::{NodeState, PubKey};
    use hotshot::types::SignatureKey;
    use hotshot_example_types::node_types::TestVersions;
    use hotshot_query_service::testing::mocks::MockVersions;
    use sequencer_utils::test_utils::setup_test;
    use serde_json::json;

    use super::*;
    use crate::persistence::testing::TestablePersistence;

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
    async fn test_load_quorum_proposals_invalid_extension() {
        setup_test();

        let tmp = Persistence::tmp_storage().await;
        let storage = Persistence::connect(&tmp).await;

        // Generate a couple of valid quorum proposals.
        let leaf: Leaf2 = Leaf::genesis::<MockVersions>(&Default::default(), &NodeState::mock())
            .await
            .into();
        let privkey = PubKey::generated_from_seed_indexed([0; 32], 1).1;
        let signature = PubKey::sign(&privkey, &[]).unwrap();
        let mut quorum_proposal = Proposal {
            data: QuorumProposalWrapper::<SeqTypes> {
                proposal: QuorumProposal2::<SeqTypes> {
                    epoch: None,
                    block_header: leaf.block_header().clone(),
                    view_number: ViewNumber::genesis(),
                    justify_qc: QuorumCertificate::genesis::<TestVersions>(
                        &Default::default(),
                        &NodeState::mock(),
                    )
                    .await
                    .to_qc2(),
                    upgrade_certificate: None,
                    view_change_evidence: None,
                    next_drb_result: None,
                    next_epoch_justify_qc: None,
                },
            },
            signature,
            _pd: Default::default(),
        };

        // Store quorum proposals.
        let quorum_proposal1 = quorum_proposal.clone();
        storage
            .append_quorum_proposal(&quorum_proposal1)
            .await
            .unwrap();
        quorum_proposal.data.proposal.view_number = ViewNumber::new(1);
        let quorum_proposal2 = quorum_proposal.clone();
        storage
            .append_quorum_proposal(&quorum_proposal2)
            .await
            .unwrap();

        // Change one of the file extensions. It can happen that we end up with files with the wrong
        // extension if, for example, the node is killed before cleaning up a swap file.
        fs::rename(
            tmp.path().join("quorum_proposals/1.txt"),
            tmp.path().join("quorum_proposals/1.swp"),
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
                    justify_qc: QuorumCertificate::genesis::<TestVersions>(
                        &Default::default(),
                        &NodeState::mock(),
                    )
                    .await
                    .to_qc2(),
                    upgrade_certificate: None,
                    view_change_evidence: None,
                    next_drb_result: None,
                    next_epoch_justify_qc: None,
                },
            },
            signature,
            _pd: Default::default(),
        };

        // First store an invalid quorum proposal.
        fs::create_dir_all(tmp.path().join("quorum_proposals")).unwrap();
        fs::write(
            tmp.path().join("quorum_proposals/0.txt"),
            "invalid data".as_bytes(),
        )
        .unwrap();

        // Store valid quorum proposal.
        storage
            .append_quorum_proposal(&quorum_proposal)
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
}
