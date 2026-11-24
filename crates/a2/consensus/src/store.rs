//! The consensus state this node keeps on disk, as tables of a [`consensus_store::Store`].
//!
//! This module turns typed consensus data into batch operations and back. It decides keys and
//! encodings; the [`Storage`](crate::storage::Storage) writer decides when batches are applied.
//!
//! Keys that start with a view are the view as a big-endian `u64`, so the rows of earlier views
//! sort first and garbage collection is a range delete. Values are bincode.

use std::{
    collections::BTreeMap,
    marker::PhantomData,
    ops::Range,
    sync::{Arc, LazyLock},
};

use anyhow::{Context, bail};
use app_tables::{AppTables, AppWrite, TableName};
use async_trait::async_trait;
use consensus_store::{Batch, Bytes, KeyRange, Snapshot, Store, Table, key};
use hotshot_types::{
    data::{EpochNumber, Leaf2, ViewNumber},
    message::Proposal as SignedProposal,
    new_protocol::Proposal,
    simple_certificate::{
        Certificate1, Certificate2, LightClientStateUpdateCertificateV2, UpgradeCertificate,
    },
    simple_vote::LockView,
    traits::{node_implementation::NodeType, signature_key::SignatureKey},
    vid::avidm_gf2::AvidmGf2Common,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tokio::sync::{mpsc, oneshot, watch};

use crate::discovery::SignedServiceRecord;

/// Rows of which there is at most one, under fixed keys.
static SAFETY: LazyLock<Table> = LazyLock::new(|| table("safety"));

/// View to the proposal this node voted for or made.
static PROPOSALS: LazyLock<Table> = LazyLock::new(|| table("proposals"));

/// `(view, epoch)` of each lock this node took since the last decide, to the lock's
/// `Certificate1`. A restarted node resumes from the latest of them whose block's payload is in its
/// deployment's store, so every lock it may have voted vote2 on is kept.
static LOCKS: LazyLock<Table> = LazyLock::new(|| table("locks"));

/// Height to the decided leaf there, until the store acknowledges it.
static DECIDE_LOG: LazyLock<Table> = LazyLock::new(|| table("decide-log"));

/// View of the epoch root to its light client state certificate, until the view is decided.
static PENDING_STATE_CERTS: LazyLock<Table> = LazyLock::new(|| table("pending-state-certs"));

/// Epoch to the light client state certificate of its root, once decided.
static STATE_CERTS: LazyLock<Table> = LazyLock::new(|| table("state-certs"));

/// `(epoch, validator key)` to the newest record of the validator's API service in the epoch.
static SERVICE_RECORDS: LazyLock<Table> = LazyLock::new(|| table("service-records"));

/// The latest view this node voted, proposed or sent a timeout vote in.
const ACTED_VIEW: &[u8] = b"acted-view";

/// The view after the latest one this node voted in.
const RESTART_VIEW: &[u8] = b"restart-view";

/// This node's `Certificate1` of the last block of an epoch.
const BOUNDARY_QC: &[u8] = b"boundary-qc";

/// The decided upgrade certificate.
const UPGRADE_CERT: &[u8] = b"upgrade-cert";

/// The latest decided leaf, which a restarted node resumes from.
const ANCHOR: &[u8] = b"anchor";

/// Present once the state of the storage this store replaces has been imported.
const IMPORTED: &[u8] = b"imported";

/// An in-memory store, for tests.
#[cfg(any(test, feature = "testing"))]
pub use consensus_store::mem::MemStore;

/// Typed access to the consensus tables of a store.
pub struct ConsensusStore<T, S> {
    store: Arc<S>,
    changes: Arc<Changes>,
    _types: PhantomData<fn() -> T>,
}

impl<T, S> Clone for ConsensusStore<T, S> {
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
            changes: self.changes.clone(),
            _types: PhantomData,
        }
    }
}

/// Announcements of durable writes, for readers that follow the store.
struct Changes {
    /// Counts the durable writes to the decide log: new entries, and entries of heights decided
    /// late.
    decide_log_writes: watch::Sender<u64>,

    /// The bytes the decide log takes up.
    decide_log_bytes: watch::Sender<u64>,

    /// The writer of the store while one runs, which every other write goes through.
    writer: watch::Sender<Option<mpsc::UnboundedSender<ForeignWrite>>>,
}

/// A write that does not come from the writer: an acknowledgement or a write to the
/// application's tables. The writer adds it to its batch and reports once it is durable.
pub(crate) struct ForeignWrite {
    pub(crate) batch: Batch,
    pub(crate) resizes_decide_log: bool,
    pub(crate) done: oneshot::Sender<anyhow::Result<()>>,
}

/// One decided leaf, as the decide log holds it. Its payload is stored apart.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct Decided<T: NodeType> {
    /// The leaf, without its payload.
    pub leaf: Leaf2<T>,

    /// The certificate that certifies the leaf: the decide's own for its newest leaf, the next
    /// leaf's justify QC for an older one.
    pub cert1: Certificate1<T>,

    /// The certificate that decided the leaf, for the newest leaf of a decide.
    pub cert2: Option<Certificate2<T>>,

    /// The VID common data of the leaf's block, if this node had its share.
    pub vid_common: Option<AvidmGf2Common>,

    /// The state certificate of the epoch whose root this leaf is, if this node has it.
    pub state_cert: Option<LightClientStateUpdateCertificateV2<T>>,

    /// The parts of the state after the leaf that its header only commits to, if this node
    /// validated the leaf.
    pub state: Option<StateChange>,
}

/// The parts of the state after a decided leaf that its header only commits to, each encoded by
/// the application, which knows the state's types.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateChange {
    /// The frontier of the block Merkle tree; empty if the tree is empty.
    pub frontier: Vec<u8>,

    pub chain_config: Vec<u8>,

    /// The reward balances that differ from those of the parent leaf's state.
    pub reward_changes: Vec<u8>,
}

/// The latest decided leaf with the certificates that decided it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct Anchor<T: NodeType> {
    /// The leaf, without its payload.
    pub leaf: Leaf2<T>,
    pub cert1: Certificate1<T>,

    /// Absent for genesis, and for an anchor imported without one.
    pub cert2: Option<Certificate2<T>>,
}

/// What a restarted node resumes from.
#[derive(Debug)]
pub struct Restored<T: NodeType> {
    pub proposals: BTreeMap<ViewNumber, SignedProposal<T, Proposal<T>>>,

    /// The latest lock.
    pub locked_qc: Option<Certificate1<T>>,

    /// The locks before the latest one since the last decide, earliest first.
    pub earlier_locks: Vec<Certificate1<T>>,

    pub boundary_qc: Option<Certificate1<T>>,
    pub acted_view: Option<ViewNumber>,
    pub restart_view: Option<ViewNumber>,
    pub upgrade_cert: Option<UpgradeCertificate<T>>,

    /// The state certificate of the latest decided epoch root.
    pub state_cert: Option<LightClientStateUpdateCertificateV2<T>>,

    /// The latest decided leaf.
    pub anchor: Option<Anchor<T>>,

    /// The newest record of each validator's API service.
    pub service_records: Vec<SignedServiceRecord<T>>,
}

impl<T: NodeType> Default for Restored<T> {
    fn default() -> Self {
        Self {
            proposals: BTreeMap::new(),
            locked_qc: None,
            earlier_locks: Vec::new(),
            boundary_qc: None,
            acted_view: None,
            restart_view: None,
            upgrade_cert: None,
            state_cert: None,
            anchor: None,
            service_records: Vec::new(),
        }
    }
}

/// The state imported from the storage this store replaces.
pub struct Import<T: NodeType> {
    pub proposals: Vec<SignedProposal<T, Proposal<T>>>,
    pub locked_qc: Option<Certificate1<T>>,
    pub boundary_qc: Option<Certificate1<T>>,
    pub acted_view: Option<ViewNumber>,
    pub restart_view: Option<ViewNumber>,
    pub upgrade_cert: Option<UpgradeCertificate<T>>,
    pub state_cert: Option<LightClientStateUpdateCertificateV2<T>>,

    /// The latest decided leaf.
    pub anchor: Option<Anchor<T>>,

    /// The application's tables, as [`ConsensusStore::app_write`] writes them.
    pub app: Vec<AppWrite>,
}

impl<T: NodeType, S: Store> ConsensusStore<T, S> {
    /// Takes over `store`, an opened store of any backend, and returns it with what a restarted
    /// node resumes from.
    ///
    /// On the first open, `import` provides the state of the storage this store replaces, and
    /// the store takes it over. A failed import fails the open: a node that lost its votes
    /// and locks could vote twice in a view.
    pub async fn open(
        store: S,
        import: impl AsyncFnOnce() -> anyhow::Result<Import<T>>,
    ) -> anyhow::Result<(Self, Restored<T>)> {
        let store = Self::new(Arc::new(store));
        if !store.imported()? {
            let state = import().await.context("reading the state to import")?;
            store.import(state).context("importing state")?;
        }
        let restored = store.load().context("loading the consensus store")?;
        Ok((store, restored))
    }
}

impl<T: NodeType, S: Store> ConsensusStore<T, S> {
    pub fn new(store: Arc<S>) -> Self {
        let decide_log_bytes = store
            .snapshot()
            .and_then(|snap| decide_log_bytes(&snap))
            .unwrap_or(0);
        let changes = Changes {
            decide_log_writes: watch::channel(0).0,
            decide_log_bytes: watch::channel(decide_log_bytes).0,
            writer: watch::channel(None).0,
        };
        Self {
            store,
            changes: Arc::new(changes),
            _types: PhantomData,
        }
    }

    /// Changes after each durable write to the decide log. Writes can add entries below the
    /// newest, of heights decided late.
    pub fn decide_log_writes(&self) -> watch::Receiver<u64> {
        self.changes.decide_log_writes.subscribe()
    }

    /// The bytes the decide log takes up, as it grows and as the store acknowledges entries.
    pub fn decide_log_bytes(&self) -> watch::Receiver<u64> {
        self.changes.decide_log_bytes.subscribe()
    }

    /// Measures the decide log and announces its size. Blocks on the store.
    pub(crate) fn measure_decide_log(&self) -> anyhow::Result<()> {
        let bytes = decide_log_bytes(&self.store.snapshot()?)?;
        self.changes.decide_log_bytes.send_replace(bytes);
        Ok(())
    }

    /// Announces a durable write to the decide log.
    pub(crate) fn announce(&self) {
        self.changes.decide_log_writes.send_modify(|n| *n += 1);
    }

    pub(crate) fn store(&self) -> &Arc<S> {
        &self.store
    }

    /// Reads what a restarted node resumes from.
    pub fn load(&self) -> anyhow::Result<Restored<T>> {
        let snap = self.store.snapshot()?;
        let mut proposals = BTreeMap::new();
        for entry in snap.range(&PROPOSALS, &KeyRange::all())? {
            let (k, v) = entry?;
            let p: SignedProposal<T, Proposal<T>> = decode(&v).context("proposal")?;
            proposals.insert(view_of(&k)?, p);
        }
        let state_cert = match snap
            .range(&STATE_CERTS, &KeyRange::all())?
            .last()
            .transpose()?
        {
            Some((_, v)) => Some(decode(&v).context("state certificate")?),
            None => None,
        };
        let service_records = snap
            .range(&SERVICE_RECORDS, &KeyRange::all())?
            .map(|entry| decode(&entry?.1).context("service record"))
            .collect::<anyhow::Result<_>>()?;
        let mut earlier_locks = snap
            .range(&LOCKS, &KeyRange::all())?
            .map(|entry| decode(&entry?.1).context("lock"))
            .collect::<anyhow::Result<Vec<Certificate1<T>>>>()?;
        earlier_locks.sort_by_key(LockView::of);
        let locked_qc = earlier_locks.pop();
        Ok(Restored {
            proposals,
            locked_qc,
            earlier_locks,
            boundary_qc: get(&snap, &SAFETY, BOUNDARY_QC)?,
            acted_view: get_view(&snap, ACTED_VIEW)?,
            restart_view: get_view(&snap, RESTART_VIEW)?,
            upgrade_cert: get(&snap, &SAFETY, UPGRADE_CERT)?,
            state_cert,
            anchor: get(&snap, &SAFETY, ANCHOR)?,
            service_records,
        })
    }

    /// At most `limit` decide log entries from `height` on, in height order.
    pub fn decided_from(&self, height: u64, limit: usize) -> anyhow::Result<Vec<Decided<T>>> {
        let snap = self.store.snapshot()?;
        let from = KeyRange::starting_at(Bytes::copy_from_slice(&key::u64(height)));
        let mut entries = Vec::new();
        for entry in snap.range(&DECIDE_LOG, &from)?.take(limit) {
            entries.push(decode(&entry?.1).context("decide log entry")?);
        }
        Ok(entries)
    }

    /// The earliest height the decide log still holds.
    pub fn earliest_decided(&self) -> anyhow::Result<Option<u64>> {
        let snap = self.store.snapshot()?;
        match snap.range(&DECIDE_LOG, &KeyRange::all())?.next() {
            Some(entry) => Ok(key::decode_u64(&entry?.0)),
            None => Ok(None),
        }
    }

    /// The encoded records of `epoch`, or of the latest epoch with records if `epoch` is `None`,
    /// with the epoch they are for.
    pub fn service_records(
        &self,
        epoch: Option<EpochNumber>,
    ) -> anyhow::Result<Option<(EpochNumber, Vec<Bytes>)>> {
        let snap = self.store.snapshot()?;
        let epoch = match epoch {
            Some(epoch) => epoch,
            None => match snap
                .range(&SERVICE_RECORDS, &KeyRange::all())?
                .last()
                .transpose()?
            {
                Some((k, _)) => EpochNumber::new(
                    k.get(..8)
                        .and_then(key::decode_u64)
                        .context("service record key does not start with an epoch")?,
                ),
                None => return Ok(None),
            },
        };
        let range = KeyRange::between(
            Bytes::copy_from_slice(&key::u64(*epoch)),
            Bytes::copy_from_slice(&key::u64(*epoch + 1)),
        );
        let records = snap
            .range(&SERVICE_RECORDS, &range)?
            .map(|entry| Ok(entry?.1))
            .collect::<anyhow::Result<Vec<_>>>()?;
        Ok(Some((epoch, records)))
    }

    /// Frees the decide log entries of the heights in `ranges`.
    pub async fn ack(&self, ranges: &[Range<u64>]) -> anyhow::Result<()> {
        let mut batch = Batch::new();
        for range in ranges.iter().filter(|range| !range.is_empty()) {
            drop_decided(&mut batch, range);
        }
        self.write(batch, true).await
    }

    /// The state certificate of the epoch root proposed in `view`, if stored and not yet decided.
    pub fn pending_state_cert(
        &self,
        view: ViewNumber,
    ) -> anyhow::Result<Option<LightClientStateUpdateCertificateV2<T>>> {
        get(
            &self.store.snapshot()?,
            &PENDING_STATE_CERTS,
            &key::u64(*view),
        )
    }

    /// The value of `key` in the application's `table`. Consensus does not read these tables. A
    /// table name is lowercase ASCII letters, digits, `_` and `-`.
    pub fn app_get(&self, table: &TableName, key: &[u8]) -> anyhow::Result<Option<Bytes>> {
        let snap = self.store.snapshot()?;
        Ok(snap.get(&app_table(table), key)?)
    }

    /// The entries of the application's `table` with keys from `from` on, and before `until` if
    /// set, in key order.
    pub fn app_entries(
        &self,
        table: &TableName,
        from: &[u8],
        until: Option<&[u8]>,
    ) -> anyhow::Result<Vec<(Bytes, Bytes)>> {
        let snap = self.store.snapshot()?;
        let from = Bytes::copy_from_slice(from);
        let range = match until {
            Some(until) => KeyRange::between(from, Bytes::copy_from_slice(until)),
            None => KeyRange::starting_at(from),
        };
        Ok(snap
            .range(&app_table(table), &range)?
            .collect::<Result<_, _>>()?)
    }

    /// Applies `writes` to the application's tables, all or none.
    pub async fn app_write(&self, writes: &[AppWrite]) -> anyhow::Result<()> {
        let mut batch = Batch::new();
        put_app(&mut batch, writes);
        self.write(batch, false).await
    }

    /// Makes `writer` the writer of the store: from now on every other write goes through it.
    pub(crate) fn attach_writer(&self, writer: mpsc::UnboundedSender<ForeignWrite>) {
        self.changes.writer.send_replace(Some(writer));
    }

    /// Applies `batch` through the writer, in its next commit, so that it does not contend with
    /// the writes consensus waits on. Without a writer, as while the node starts, or once it has
    /// stopped, the batch is applied directly.
    ///
    /// A write the writer took but did not report, because it stopped, is not applied again: it
    /// may have been, and a later write to the same key could be lost.
    async fn write(&self, batch: Batch, resizes_decide_log: bool) -> anyhow::Result<()> {
        let writer = self.changes.writer.borrow().clone();
        let batch = match writer {
            Some(writer) => {
                let (done, durable) = oneshot::channel();
                let write = ForeignWrite {
                    batch,
                    resizes_decide_log,
                    done,
                };
                match writer.send(write) {
                    Ok(()) => {
                        return durable
                            .await
                            .context("the writer stopped before it reported the write")?;
                    },
                    Err(returned) => returned.0.batch,
                }
            },
            None => batch,
        };
        let store = self.clone();
        tokio::task::spawn_blocking(move || {
            store.store.apply(&batch)?;
            if resizes_decide_log {
                store.measure_decide_log()?;
            }
            Ok(())
        })
        .await?
    }

    /// Whether [`Self::import`] has run on this store.
    pub fn imported(&self) -> anyhow::Result<bool> {
        Ok(self.store.snapshot()?.get(&SAFETY, IMPORTED)?.is_some())
    }

    /// Writes the state of the storage this store replaces, and marks the import as done.
    ///
    /// Refuses a store that already holds an import, so a node never mixes the state of two
    /// runs.
    pub fn import(&self, import: Import<T>) -> anyhow::Result<()> {
        if self.imported()? {
            bail!("the consensus store already holds imported state");
        }
        let mut batch = Batch::new();
        for p in &import.proposals {
            put_proposal(&mut batch, p)?;
        }
        if let Some(qc) = &import.locked_qc {
            put_lock(&mut batch, qc)?;
        }
        if let Some(qc) = &import.boundary_qc {
            put_boundary_qc(&mut batch, qc)?;
        }
        if let Some(view) = import.acted_view {
            put_acted_view(&mut batch, view);
        }
        if let Some(view) = import.restart_view {
            put_restart_view(&mut batch, view);
        }
        if let Some(cert) = &import.upgrade_cert {
            put_upgrade_cert(&mut batch, cert)?;
        }
        if let Some(cert) = &import.state_cert {
            batch.put(&STATE_CERTS, key::u64(*cert.epoch).to_vec(), encode(cert)?);
        }
        if let Some(anchor) = &import.anchor {
            put_anchor(&mut batch, anchor)?;
        }
        put_app(&mut batch, &import.app);
        batch.put(&SAFETY, IMPORTED, Bytes::new());
        self.store.apply(&batch)?;
        Ok(())
    }

    /// Moves the state certificates of views up to `decided` to the decided ones.
    ///
    /// Like the decide path of the storage this replaces, every certificate in the range is
    /// kept, by its epoch.
    pub(crate) fn finalize_state_certs(
        &self,
        batch: &mut Batch,
        decided: ViewNumber,
        queued: &BTreeMap<ViewNumber, LightClientStateUpdateCertificateV2<T>>,
    ) -> anyhow::Result<()> {
        // Certificates queued in `batch` or one being applied are not in the snapshot.
        for cert in queued.range(..=decided).map(|(_, cert)| cert) {
            batch.put(&STATE_CERTS, key::u64(*cert.epoch).to_vec(), encode(cert)?);
        }
        let snap = self.store.snapshot()?;
        let up_to = KeyRange::new(
            std::ops::Bound::Unbounded,
            std::ops::Bound::Included(Bytes::copy_from_slice(&key::u64(*decided))),
        );
        for entry in snap.range(&PENDING_STATE_CERTS, &up_to)? {
            let (_, v) = entry?;
            let cert: LightClientStateUpdateCertificateV2<T> =
                decode(&v).context("state certificate")?;
            batch.put(&STATE_CERTS, key::u64(*cert.epoch).to_vec(), v);
        }
        batch.delete_range(&PENDING_STATE_CERTS, up_to);
        Ok(())
    }
}

/// Stores `view` as the latest view this node acted in.
pub(crate) fn put_acted_view(batch: &mut Batch, view: ViewNumber) {
    batch.put(&SAFETY, ACTED_VIEW, key::u64(*view).to_vec());
}

/// Stores `view` as the view a restarted node resumes in.
pub(crate) fn put_restart_view(batch: &mut Batch, view: ViewNumber) {
    batch.put(&SAFETY, RESTART_VIEW, key::u64(*view).to_vec());
}

pub(crate) fn put_proposal<T: NodeType>(
    batch: &mut Batch,
    proposal: &SignedProposal<T, Proposal<T>>,
) -> anyhow::Result<()> {
    batch.put(
        &PROPOSALS,
        key::u64(*proposal.data.view_number).to_vec(),
        encode(proposal)?,
    );
    Ok(())
}

pub(crate) fn put_pending_state_cert<T: NodeType>(
    batch: &mut Batch,
    view: ViewNumber,
    cert: &LightClientStateUpdateCertificateV2<T>,
) -> anyhow::Result<()> {
    batch.put(
        &PENDING_STATE_CERTS,
        key::u64(*view).to_vec(),
        encode(cert)?,
    );
    Ok(())
}

/// Adds `qc` to the locks.
pub(crate) fn put_lock<T: NodeType>(batch: &mut Batch, qc: &Certificate1<T>) -> anyhow::Result<()> {
    let lock = LockView::of(qc);
    batch.put(
        &LOCKS,
        key::u64_pair(*lock.view, *lock.epoch).to_vec(),
        encode(qc)?,
    );
    Ok(())
}

/// Stores `qc` as the boundary certificate. The writer checks that it is later than the stored
/// one.
pub(crate) fn put_boundary_qc<T: NodeType>(
    batch: &mut Batch,
    qc: &Certificate1<T>,
) -> anyhow::Result<()> {
    batch.put(&SAFETY, BOUNDARY_QC, encode(qc)?);
    Ok(())
}

pub(crate) fn put_upgrade_cert<T: NodeType>(
    batch: &mut Batch,
    cert: &UpgradeCertificate<T>,
) -> anyhow::Result<()> {
    batch.put(&SAFETY, UPGRADE_CERT, encode(cert)?);
    Ok(())
}

/// Deletes the decide log entries of the heights in `range`.
pub(crate) fn drop_decided(batch: &mut Batch, range: &Range<u64>) {
    batch.delete_range(
        &DECIDE_LOG,
        KeyRange::between(
            Bytes::copy_from_slice(&key::u64(range.start)),
            Bytes::copy_from_slice(&key::u64(range.end)),
        ),
    );
}

/// Appends a decide to the decide log: `leaves` newest first, as consensus decides them,
/// with the certificates of the newest and the VID common data and state of each.
pub(crate) fn put_decided<T: NodeType>(
    batch: &mut Batch,
    leaves: &[Leaf2<T>],
    cert1: &Certificate1<T>,
    cert2: Option<&Certificate2<T>>,
    vid_common: &[Option<AvidmGf2Common>],
    state_certs: &[Option<LightClientStateUpdateCertificateV2<T>>],
    states: &[Option<StateChange>],
) -> anyhow::Result<()> {
    for (i, leaf) in leaves.iter().enumerate() {
        let mut leaf = leaf.clone();
        leaf.unfill_block_payload();
        let newest = i == 0;
        let height = key::u64(leaf.height()).to_vec();
        // Leaves are newest first: the one before in the slice is the child.
        let certifying = match i {
            0 => cert1.clone(),
            _ => leaves[i - 1].justify_qc().clone(),
        };
        let decided = Decided {
            cert1: certifying,
            cert2: if newest { cert2.cloned() } else { None },
            vid_common: vid_common.get(i).cloned().flatten(),
            state_cert: state_certs.get(i).cloned().flatten(),
            state: states.get(i).cloned().flatten(),
            leaf,
        };
        batch.put(&DECIDE_LOG, height, encode(&decided)?);
    }
    Ok(())
}

/// The bytes the decide log takes up in `snap`.
fn decide_log_bytes(snap: &impl Snapshot) -> Result<u64, consensus_store::Error> {
    let usage = snap.usage(&DECIDE_LOG)?;
    Ok(usage.bytes)
}

/// The application tables of the store, whose calls block, so they run on a blocking thread.
#[async_trait]
impl<T: NodeType, S: Store> AppTables for ConsensusStore<T, S> {
    async fn get(&self, table: &TableName, key: &[u8]) -> anyhow::Result<Option<Bytes>> {
        let (store, table, key) = (self.clone(), table.clone(), key.to_vec());
        tokio::task::spawn_blocking(move || store.app_get(&table, &key)).await?
    }

    async fn entries(
        &self,
        table: &TableName,
        from: &[u8],
        until: Option<&[u8]>,
    ) -> anyhow::Result<Vec<(Bytes, Bytes)>> {
        let (store, table, from) = (self.clone(), table.clone(), from.to_vec());
        let until = until.map(<[u8]>::to_vec);
        tokio::task::spawn_blocking(move || store.app_entries(&table, &from, until.as_deref()))
            .await?
    }

    async fn write(&self, writes: Vec<AppWrite>) -> anyhow::Result<()> {
        self.app_write(&writes).await
    }
}

/// Stores `anchor` as the latest decided leaf.
pub(crate) fn put_anchor<T: NodeType>(batch: &mut Batch, anchor: &Anchor<T>) -> anyhow::Result<()> {
    batch.put(&SAFETY, ANCHOR, encode(anchor)?);
    Ok(())
}

/// Deletes the proposals and locks of views before `view`, which is no later than the anchor's.
pub(crate) fn collect_garbage(batch: &mut Batch, view: ViewNumber) {
    let before = || KeyRange::before(Bytes::copy_from_slice(&key::u64(*view)));
    batch.delete_range(&PROPOSALS, before());
    batch.delete_range(&LOCKS, before());
}

/// Replaces the stored record of the record's signer in its epoch.
pub(crate) fn put_service_record<T: NodeType>(
    batch: &mut Batch,
    record: &SignedServiceRecord<T>,
) -> anyhow::Result<()> {
    let mut k = key::u64(*record.record.epoch).to_vec();
    k.extend(record.signer.to_bytes());
    batch.put(&SERVICE_RECORDS, k, encode(record)?);
    Ok(())
}

/// Deletes the records of the epochs before `epoch`.
pub(crate) fn drop_service_records(batch: &mut Batch, epoch: EpochNumber) {
    batch.delete_range(
        &SERVICE_RECORDS,
        KeyRange::before(Bytes::copy_from_slice(&key::u64(*epoch))),
    );
}

/// Queues `writes` to the application's tables in `batch`.
pub(crate) fn put_app(batch: &mut Batch, writes: &[AppWrite]) {
    for write in writes {
        match write {
            AppWrite::Put { table, key, value } => {
                batch.put(&app_table(table), key.clone(), value.clone());
            },
            AppWrite::DeleteBefore { table, key } => {
                batch.delete_range(&app_table(table), KeyRange::before(key.clone()));
            },
            AppWrite::Clear { table } => {
                batch.delete_range(&app_table(table), KeyRange::all());
            },
        }
    }
}

fn view_of(k: &[u8]) -> anyhow::Result<ViewNumber> {
    let n = k
        .get(..8)
        .and_then(key::decode_u64)
        .context("key does not start with a view")?;
    Ok(ViewNumber::new(n))
}

fn get<V: DeserializeOwned>(
    snap: &impl Snapshot,
    table: &Table,
    name: &[u8],
) -> anyhow::Result<Option<V>> {
    match snap.get(table, name)? {
        Some(v) => {
            Ok(Some(decode(&v).with_context(|| {
                String::from_utf8_lossy(name).into_owned()
            })?))
        },
        None => Ok(None),
    }
}

fn get_view(snap: &impl Snapshot, name: &[u8]) -> anyhow::Result<Option<ViewNumber>> {
    match snap.get(&SAFETY, name)? {
        Some(v) => {
            let n = key::decode_u64(&v)
                .with_context(|| format!("malformed {}", String::from_utf8_lossy(name)))?;
            Ok(Some(ViewNumber::new(n)))
        },
        None => Ok(None),
    }
}

fn encode<V: Serialize>(value: &V) -> anyhow::Result<Bytes> {
    Ok(bincode::serialize(value)?.into())
}

fn decode<V: DeserializeOwned>(bytes: &[u8]) -> anyhow::Result<V> {
    Ok(bincode::deserialize(bytes)?)
}

/// The application's table `name`, apart from the consensus tables. An application table name
/// leaves room for the prefix within the store's own rules.
fn app_table(name: &TableName) -> Table {
    Table::new(format!("app-{name}")).expect("an application table name makes a valid table")
}

fn table(name: &str) -> Table {
    Table::new(name).expect("valid table name")
}
