//! Fills the gaps of the store from other API services.
//!
//! A gap is a height the store lacks below the newest it holds, or a stored height without its
//! payload or VID common data. Neither needs a stake table to check. A missing leaf hangs from the
//! stored leaf after it: it must have the commitment that leaf names as its parent, and the justify
//! QC of that leaf certifies it. Gaps are filled from the top down, so each filled leaf is the
//! trusted one for the leaf below it. A payload and its VID common data must recompute the payload
//! commitment of the stored header.
//!
//! A filled height is stored without the `Certificate2` and state certificate the peer may have,
//! since checking those needs the stake table of their epoch.
//!
//! Only final heights are filled, those [`FINALITY_VIEWS`] behind the newest: before that, the
//! API service's own consensus node can still send a missing height or payload.
//!
//! The peers are the API services the validators of the current epoch name, from [`Peers`], asked
//! in random order for each gap.

use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
    time::Duration,
};

use anyhow::{Context, bail, ensure};
use chain_api::{
    proto::{GetRecordsRequest, chain_service_client::ChainServiceClient},
    records::{RECORD_RANGE_LIMIT, from_proto},
};
use chain_store::{ChainStore, Item, Part, Parts, Record};
use committable::Committable;
use consensus_api::FINALITY_VIEWS;
use espresso_types::{Header, Payload};
use hotshot_types::{
    data::EpochNumber,
    traits::{
        EncodeBytes,
        block_contents::{BlockHeader, BlockPayload},
    },
};
use light_client::consensus::payload::PayloadProof;
use tokio::{
    task::spawn_blocking,
    time::{Instant, sleep, sleep_until, timeout},
};
use tonic::transport::{ClientTlsConfig, Endpoint};
use url::Url;

use crate::{
    membership::Membership,
    peers::{Peer, Peers},
};

/// The least time between two passes.
const PASS_INTERVAL: Duration = Duration::from_secs(1);

/// How many gaps one pass looks for past the cursor.
const GAPS_PER_PASS: usize = 16;

/// How long a peer may take to connect, to answer a request, and between two records it sends.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// How often a connection to another service is pinged, and how long a ping may go unanswered
/// before the connection is dropped.
const KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(10);
const KEEP_ALIVE_TIMEOUT: Duration = Duration::from_secs(20);

/// The largest record a peer may send, which a payload bounds.
const MAX_RECORD_SIZE: usize = 64 << 20;

/// How long a gap that no peer filled waits before it is tried again; the wait doubles with each
/// failure, up to [`MAX_RETRY_DELAY`].
const RETRY_DELAY: Duration = Duration::from_secs(2);

const MAX_RETRY_DELAY: Duration = Duration::from_secs(600);

pub struct GapFiller {
    pub(crate) store: ChainStore,
    pub(crate) peers: Peers,

    /// The stake tables to check certificates against; without them certificates are not
    /// filled.
    membership: Option<Membership>,

    /// How many heights behind the newest a height is final: [`FINALITY_VIEWS`], since each
    /// height takes a view at least, so a height that many behind is that many views behind.
    pub(crate) finality: u64,

    /// The newest height and the floor of the store when the cursors last moved.
    pub(crate) newest: Option<u64>,
    pub(crate) floor: u64,

    /// Every height before it has its item, but for the gaps in `item_retries`. The store is
    /// asked only about the heights from it on, so a pass does not read the whole chain.
    pub(crate) items_from: u64,

    /// Every height before it has its payload and VID common data, but for those in
    /// `part_retries`.
    parts_from: u64,

    /// The gaps no peer filled, by their first height, with when to try them again.
    pub(crate) item_retries: Retries,

    /// The heights whose payload or VID common data no peer had.
    part_retries: Retries,

    /// The epochs whose certificates no peer could answer for.
    pub(crate) epoch_retries: Retries,

    /// The epochs whose every height has its certificates known, as far as this filler has seen.
    pub(crate) checked_epochs: BTreeSet<EpochNumber>,
}

impl GapFiller {
    pub fn new(store: ChainStore, peers: Peers, membership: Option<Membership>) -> Self {
        Self {
            store,
            peers,
            membership,
            finality: FINALITY_VIEWS,
            newest: None,
            floor: 0,
            items_from: 0,
            parts_from: 0,
            item_retries: Retries::default(),
            part_retries: Retries::default(),
            epoch_retries: Retries::default(),
            checked_epochs: BTreeSet::new(),
        }
    }

    /// Fills gaps whenever the store changes, at most once per [`PASS_INTERVAL`]. Returns when
    /// the store's changes end.
    pub async fn run(mut self) {
        let mut changes = self.store.changes();
        loop {
            let started = Instant::now();
            changes.borrow_and_update();
            self.pass().await;
            sleep(PASS_INTERVAL.saturating_sub(started.elapsed())).await;
            let retry = self
                .next_retry()
                .unwrap_or(Instant::now() + MAX_RETRY_DELAY);
            tokio::select! {
                changed = changes.changed() => if changed.is_err() {
                    return;
                },
                () = sleep_until(retry) => {},
            }
        }
    }

    /// Fills the gaps of each kind, each from the first peer that has it, then derives the stake
    /// tables of the stored epoch roots and checks the certificates they allow. A step that
    /// fails does not stop the others.
    async fn pass(&mut self) {
        self.refresh_peers().await;
        if self.peers.shuffled().is_empty() {
            tracing::warn!("no API service to fill gaps from");
            return;
        }
        if let Err(err) = self.fill_gaps().await {
            tracing::warn!("filling gaps failed: {err:#}");
        }
        let Some(membership) = self.membership.clone() else {
            return;
        };
        if let Err(err) = self.follow_roots(&membership).await {
            tracing::warn!("deriving stake tables failed: {err:#}");
        }
        if let Err(err) = self.check_certs(&membership).await {
            tracing::warn!("checking certificates failed: {err:#}");
        }
    }

    /// Takes the peers of the epoch of the newest stored height, once per epoch.
    async fn refresh_peers(&mut self) {
        let Some(membership) = &self.membership else {
            return;
        };
        match self.store.newest().await {
            Ok(Some(newest)) => {
                let epoch = membership.epoch_of(newest);
                self.peers.refresh(epoch, membership).await;
            },
            Ok(None) => {},
            Err(err) => tracing::warn!("reading the newest height failed: {err:#}"),
        }
    }

    async fn fill_gaps(&mut self) -> anyhow::Result<()> {
        let Some(newest) = self.store.newest().await? else {
            return Ok(());
        };
        let floor = self.store.floor().await?;
        (self.newest, self.floor) = (Some(newest), floor);
        self.item_retries.forget_before(floor);
        self.part_retries.forget_before(floor);
        // Heights before it are final: consensus sends nothing more for them, so they are
        // asked of peers.
        let Some(final_until) = (newest + 1).checked_sub(self.finality) else {
            return Ok(());
        };
        for gap in self.item_gaps(newest, final_until).await? {
            // A filled height can lack its payload, and lies before the parts cursor.
            self.parts_from = self.parts_from.min(gap.start);
            match self.fill_items(gap.clone(), &self.peers.shuffled()).await {
                Ok(()) => self.item_retries.succeeded(gap.start),
                Err(err) => {
                    tracing::warn!(?gap, "filling a gap failed: {err:#}");
                    self.item_retries.failed(gap.start, gap.end);
                },
            }
        }
        let heights = self.part_heights(newest, final_until).await?;
        for run in runs(&heights) {
            self.fill_parts(run.clone(), &self.peers.shuffled()).await;
            let missing = match self.lacking_parts(run.clone()).await {
                Ok(missing) => missing,
                Err(err) => {
                    tracing::warn!(?run, "reading the store failed: {err:#}");
                    run.clone().collect()
                },
            };
            for height in run {
                match missing.contains(&height) {
                    true => self.part_retries.failed(height, height + 1),
                    false => self.part_retries.succeeded(height),
                }
            }
        }
        Ok(())
    }

    /// The final gaps past the cursor, which it then moves past them, and the gaps due to be
    /// tried again, as far as they are still missing. The cursor stops at the first gap that is
    /// not final yet.
    async fn item_gaps(
        &mut self,
        newest: u64,
        final_until: u64,
    ) -> anyhow::Result<Vec<Range<u64>>> {
        let mut found = self
            .store
            .missing_items(self.items_from, GAPS_PER_PASS)
            .await?;
        let pending = found.iter().position(|gap| gap.end > final_until);
        let mut gaps = Vec::new();
        for (start, end) in self.item_retries.due() {
            match self.store.missing_items(start, 1).await?.first() {
                Some(gap) if gap.start < end => {
                    // Pruning can have moved the gap's start.
                    self.item_retries.moved(start, gap.start);
                    gaps.push(gap.start..gap.end.min(end));
                },
                _ => self.item_retries.succeeded(start),
            }
        }
        // Only now that nothing can fail before the gaps are filled or retried.
        self.items_from = match (pending, found.last()) {
            (Some(i), _) => found[i].start,
            (None, Some(gap)) if found.len() == GAPS_PER_PASS => gap.end,
            (None, _) => newest + 1,
        };
        found.truncate(pending.unwrap_or(found.len()));
        gaps.extend(
            found
                .into_iter()
                .filter(|gap| !self.item_retries.contains(gap.start)),
        );
        Ok(gaps)
    }

    /// The final heights past the cursor that lack a payload or VID common data, which it then
    /// moves past them, and those due to be tried again. The cursor stops at the first height
    /// that is not final yet.
    async fn part_heights(
        &mut self,
        newest: u64,
        final_until: u64,
    ) -> anyhow::Result<BTreeSet<u64>> {
        let limit = GAPS_PER_PASS * RECORD_RANGE_LIMIT as usize;
        let mut heights = BTreeSet::new();
        let mut next = u64::MAX;
        for part in [Part::Payload, Part::VidCommon] {
            let mut found = self
                .store
                .missing_parts(part, self.parts_from, limit)
                .await?;
            let pending = found.iter().position(|height| *height >= final_until);
            next = next.min(match (pending, found.last()) {
                (Some(i), _) => found[i],
                (None, Some(last)) if found.len() == limit => *last + 1,
                (None, _) => newest + 1,
            });
            found.truncate(pending.unwrap_or(found.len()));
            heights.extend(found);
        }
        heights.retain(|height| !self.part_retries.contains(*height));
        heights.extend(
            self.part_retries
                .due()
                .into_iter()
                .map(|(height, _)| height),
        );
        // A height still missing before the item cursor can be stored later, without its parts.
        self.parts_from = next.min(self.items_from);
        Ok(heights)
    }

    fn next_retry(&self) -> Option<Instant> {
        [
            self.item_retries.next(),
            self.part_retries.next(),
            self.epoch_retries.next(),
        ]
        .into_iter()
        .flatten()
        .min()
    }

    /// Fills `gap` from the top down, from the first peer that continues the chain.
    async fn fill_items(&self, gap: Range<u64>, peers: &[Peer]) -> anyhow::Result<()> {
        let mut child = self
            .store
            .item(gap.end)
            .await?
            .context("the height after a gap is stored")?;
        let mut end = gap.end;
        while end > gap.start {
            let start = gap.start.max(end.saturating_sub(RECORD_RANGE_LIMIT));
            let mut filled = false;
            for peer in peers {
                match self.fill_items_from(peer, start..end, child.clone()).await {
                    Ok((new_end, new_child)) if new_end < end => {
                        (end, child, filled) = (new_end, new_child, true);
                        break;
                    },
                    Ok(_) => {},
                    Err(err) => tracing::debug!(%peer, "{err:#}"),
                }
            }
            if !filled {
                bail!("no peer has heights {start}..{end}");
            }
        }
        Ok(())
    }

    /// Stores the heights of `heights` that `peer` has and that continue the chain down from
    /// `child`, the leaf at `heights.end`. Returns the lowest height filled, and its item.
    async fn fill_items_from(
        &self,
        peer: &Peer,
        heights: Range<u64>,
        mut child: Item,
    ) -> anyhow::Result<(u64, Item)> {
        let records = fetch(peer, heights.clone(), false).await?;
        let mut end = heights.end;
        for record in records.into_iter().rev() {
            if record.height + 1 != end {
                break;
            }
            let item = checked_item(&record, &child)?;
            let mut parts = checked_parts(item.leaf.block_header(), &record)
                .await
                .inspect_err(|err| tracing::debug!(%peer, "payload not taken: {err:#}"))
                .unwrap_or_default();
            // The genesis block has no certificates to check, and belongs to no epoch whose
            // certificates are checked.
            parts.certs_known = record.height == 0;
            self.store.put(&item, &parts).await?;
            (end, child) = (record.height, item);
        }
        Ok((end, child))
    }

    /// Fills the payloads and VID common data of the stored `heights` from the peers, until
    /// none lacks them or every peer was asked.
    async fn fill_parts(&self, heights: Range<u64>, peers: &[Peer]) {
        for peer in peers {
            if let Err(err) = self.fill_parts_from(peer, heights.clone()).await {
                tracing::debug!(%peer, ?heights, "{err:#}");
            }
            match self.lacking_parts(heights.clone()).await {
                Ok(missing) if missing.is_empty() => return,
                Ok(_) => {},
                Err(err) => {
                    tracing::warn!(?heights, "reading the store failed: {err:#}");
                    return;
                },
            }
        }
    }

    async fn fill_parts_from(&self, peer: &Peer, heights: Range<u64>) -> anyhow::Result<()> {
        for record in fetch(peer, heights, false).await? {
            let Some(item) = self.store.item(record.height).await? else {
                continue;
            };
            if let Ok(parts) = checked_parts(item.leaf.block_header(), &record).await {
                self.store.put_parts(record.height, &parts).await?;
            }
        }
        Ok(())
    }

    /// The heights of `heights` that lack a payload or VID common data.
    async fn lacking_parts(&self, heights: Range<u64>) -> anyhow::Result<BTreeSet<u64>> {
        let mut missing = BTreeSet::new();
        let limit = (heights.end - heights.start) as usize;
        for part in [Part::Payload, Part::VidCommon] {
            let found = self.store.missing_parts(part, heights.start, limit).await?;
            missing.extend(found.into_iter().filter(|h| heights.contains(h)));
        }
        Ok(missing)
    }
}

/// What to try again: ranges of heights, or of epochs, by their first, each with its end, when it
/// is due and how long it waited last.
#[derive(Debug, Default)]
pub(crate) struct Retries(BTreeMap<u64, (u64, Instant, Duration)>);

impl Retries {
    pub(crate) fn contains(&self, start: u64) -> bool {
        self.0.contains_key(&start)
    }

    /// Whether the gap at `start` failed and is not due yet.
    pub(crate) fn waiting(&self, start: u64) -> bool {
        self.0
            .get(&start)
            .is_some_and(|(_, at, _)| *at > Instant::now())
    }

    /// Keeps the gap at `from` as starting at `to`, with its wait.
    fn moved(&mut self, from: u64, to: u64) {
        if from != to
            && let Some(entry) = self.0.remove(&from)
        {
            self.0.insert(to, entry);
        }
    }

    /// The gaps that are due, as their first and end heights.
    fn due(&self) -> Vec<(u64, u64)> {
        let now = Instant::now();
        self.0
            .iter()
            .filter(|(_, (_, at, _))| *at <= now)
            .map(|(start, (end, ..))| (*start, *end))
            .collect()
    }

    fn next(&self) -> Option<Instant> {
        self.0.values().map(|(_, at, _)| *at).min()
    }

    /// The end of the newest gap.
    pub(crate) fn newest_end(&self) -> Option<u64> {
        self.0.values().map(|(end, ..)| *end).max()
    }

    pub(crate) fn failed(&mut self, start: u64, end: u64) {
        let delay = match self.0.get(&start) {
            Some((_, _, delay)) => (*delay * 2).min(MAX_RETRY_DELAY),
            None => RETRY_DELAY,
        };
        self.0.insert(start, (end, Instant::now() + delay, delay));
    }

    pub(crate) fn succeeded(&mut self, start: u64) {
        self.0.remove(&start);
    }

    /// Forgets the gaps that end at or before `floor`, which the store no longer keeps.
    fn forget_before(&mut self, floor: u64) {
        self.0.retain(|_, (end, ..)| *end > floor);
    }
}

/// The records of `heights` that `peer` has, in height order, without payloads and VID common
/// data if `skip_payloads`. A peer that sends nothing for [`REQUEST_TIMEOUT`] fails; one that
/// keeps sending may take as long as its records need.
pub(crate) async fn fetch(
    peer: &Peer,
    heights: Range<u64>,
    skip_payloads: bool,
) -> anyhow::Result<Vec<Record>> {
    let mut client =
        ChainServiceClient::new(peer.channel.clone()).max_decoding_message_size(MAX_RECORD_SIZE);
    let mut stream = client
        .get_records(GetRecordsRequest {
            from: Some(heights.start),
            until: Some(heights.end),
            skip_payloads,
        })
        .await?
        .into_inner();
    let mut records = Vec::new();
    while (records.len() as u64) < heights.end.saturating_sub(heights.start) {
        let next = timeout(REQUEST_TIMEOUT, stream.message())
            .await
            .with_context(|| format!("{peer} stopped sending"))??;
        let Some(record) = next else {
            break;
        };
        ensure!(
            heights.contains(&record.height),
            "{peer} sent height {} for {heights:?}",
            record.height
        );
        records.push(from_proto(record));
    }
    records.sort_by_key(|record| record.height);
    records.dedup_by_key(|record| record.height);
    Ok(records)
}

/// The endpoint of the API service at `peer`, with TLS for an `https` URL.
pub(crate) fn endpoint(peer: &Url) -> anyhow::Result<Endpoint> {
    let mut endpoint = Endpoint::from_shared(peer.to_string())?
        .connect_timeout(REQUEST_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        // A connection is shared by many requests; pings drop one that died without closing.
        .http2_keep_alive_interval(KEEP_ALIVE_INTERVAL)
        .keep_alive_timeout(KEEP_ALIVE_TIMEOUT)
        .keep_alive_while_idle(true);
    if peer.scheme() == "https" {
        endpoint = endpoint.tls_config(ClientTlsConfig::new().with_webpki_roots())?;
    }
    Ok(endpoint)
}

/// The item of `record`, which must be the parent of `child`, certified by `child`'s justify QC
/// and without what cannot be checked here.
fn checked_item(record: &Record, child: &Item) -> anyhow::Result<Item> {
    let mut item = record.item()?;
    ensure!(
        item.height() == record.height && item.height() + 1 == child.height(),
        "a record of height {} is not the parent of height {}",
        item.height(),
        child.height()
    );
    ensure!(
        item.leaf.commit() == child.leaf.parent_commitment(),
        "the leaf of height {} is not the parent of the stored leaf after it",
        item.height()
    );
    let qc = child.leaf.justify_qc();
    ensure!(
        qc.data.leaf_commit == item.leaf.commit(),
        "the justify QC of height {} does not certify its parent",
        child.height()
    );
    item.cert1 = qc.clone();
    // The leaf commitment does not cover a payload the peer filled in; only checked parts may
    // carry one.
    item.leaf.unfill_block_payload();
    Ok(item)
}

/// The payload and VID common data of `record`, if both are there and recompute the payload
/// commitment of `header`. The payload is returned as encoded again after the check, which runs
/// on a blocking thread: it encodes the whole payload.
async fn checked_parts(header: &Header, record: &Record) -> anyhow::Result<Parts> {
    let parts = record.parts()?;
    let (Some(payload), Some(common)) = (parts.payload, parts.vid_common) else {
        bail!("the record of height {} lacks parts", record.height);
    };
    let header = header.clone();
    spawn_blocking(move || {
        let payload = Payload::from_bytes(&payload, header.metadata());
        let (payload, common) =
            PayloadProof::new(payload, common).verify_with_vid_common(&header)?;
        Ok(Parts {
            payload: Some(payload.encode().to_vec().into()),
            vid_common: Some(common),
            ..Default::default()
        })
    })
    .await?
}

/// `heights` as runs of consecutive heights, each at most a record request long.
pub(crate) fn runs(heights: &BTreeSet<u64>) -> Vec<Range<u64>> {
    let mut runs: Vec<Range<u64>> = Vec::new();
    for &height in heights {
        match runs.last_mut() {
            Some(run) if run.end == height && run.end - run.start < RECORD_RANGE_LIMIT => {
                run.end += 1
            },
            _ => runs.push(height..height + 1),
        }
    }
    runs
}

#[cfg(test)]
mod tests {
    use api_service::ApiService;
    use bytes::Bytes;
    use chain_store::mem::MemBackend;
    use espresso_types::{BlockMerkleTree, Leaf2, NodeState, SeqTypes, ValidatedState};
    use hotshot_query_service_types::availability::VidCommonQueryData;
    use hotshot_types::{
        data::{QuorumProposal2, QuorumProposalWrapper, VidCommon, ViewNumber},
        simple_certificate::QuorumCertificate2,
    };
    use jf_merkle_tree_compat::{AppendableMerkleTreeScheme, MerkleTreeScheme};
    use tokio::net::TcpListener;
    use versions::{NEW_PROTOCOL_VERSION, Upgrade};

    use super::*;

    /// The genesis block with its payload and VID common data, and a block after it.
    async fn chain() -> ((Item, Bytes, VidCommon), Item) {
        let instance = NodeState::mock().with_current_version(NEW_PROTOCOL_VERSION);
        let state = ValidatedState {
            chain_config: instance.chain_config.into(),
            ..Default::default()
        };
        let upgrade = Upgrade::trivial(NEW_PROTOCOL_VERSION);
        let leaf0 = Leaf2::genesis(&state, &instance, NEW_PROTOCOL_VERSION).await;
        let qc0 = QuorumCertificate2::genesis(&state, &instance, upgrade).await;
        assert_eq!(qc0.data.leaf_commit, leaf0.commit());
        let common =
            VidCommonQueryData::<SeqTypes>::genesis(&state, &instance, NEW_PROTOCOL_VERSION)
                .await
                .common;
        let payload = Payload::empty().0.encode().to_vec().into();

        let mut header = leaf0.block_header().clone();
        *header.height_mut() = 1;
        let leaf1 = Leaf2::from_quorum_proposal(&QuorumProposalWrapper {
            proposal: QuorumProposal2 {
                block_header: header,
                view_number: ViewNumber::new(1),
                epoch: None,
                justify_qc: qc0.clone(),
                next_epoch_justify_qc: None,
                upgrade_certificate: None,
                view_change_evidence: None,
                next_drb_result: None,
                state_cert: None,
            },
        });
        let mut qc1 = qc0.clone();
        qc1.data.leaf_commit = leaf1.commit();
        let item = |leaf, cert1| Item { leaf, cert1 };
        ((item(leaf0, qc0), payload, common), item(leaf1, qc1))
    }

    fn parts(payload: &Bytes, common: &VidCommon) -> Parts {
        Parts {
            payload: Some(payload.clone()),
            vid_common: Some(common.clone()),
            ..Default::default()
        }
    }

    /// Serves `store`, returning its URL.
    async fn serve(store: ChainStore) -> Url {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let service = ApiService::new(store, "127.0.0.1:1".parse().unwrap());
        tokio::spawn(service.serve(listener));
        url.parse().unwrap()
    }

    /// A store holding `item1` only, filling its gaps from `peer`, with every height final.
    fn filling(item1: &Item, peer: Url) -> ChainStore {
        filling_with(item1, peer, 0)
    }

    /// A store holding `item1` only, filling its gaps from `peer`, with heights `finality`
    /// behind the newest final.
    fn filling_with(item1: &Item, peer: Url, finality: u64) -> ChainStore {
        let store = ChainStore::new(MemBackend::default());
        let peers = Peers::new(chain_api::Tables::new(store.clone()), vec![peer], None);
        let mut filler = GapFiller::new(store.clone(), peers, None);
        filler.finality = finality;
        let item1 = item1.clone();
        let writer = store.clone();
        tokio::spawn(async move {
            writer.put(&item1, &Parts::default()).await.unwrap();
            filler.run().await;
        });
        store
    }

    #[tokio::test]
    async fn fills_a_missing_height_and_its_parts_from_a_peer() {
        let ((item0, payload, common), item1) = chain().await;
        let peer = ChainStore::new(MemBackend::default());
        peer.put(&item0, &parts(&payload, &common)).await.unwrap();
        peer.put(&item1, &Parts::default()).await.unwrap();
        let store = filling(&item1, serve(peer).await);

        tokio::time::timeout(Duration::from_secs(10), async {
            while store.vid_common(0).await.unwrap().is_none() {
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("the gap is filled");
        let filled = store.item(0).await.unwrap().unwrap();
        assert_eq!(filled.leaf, item0.leaf);
        assert_eq!(&filled.cert1, item1.leaf.justify_qc());
        assert_eq!(store.payload(0).await.unwrap(), Some(payload));
        assert_eq!(store.vid_common(0).await.unwrap(), Some(common));
        let genesis = store.record(0).await.unwrap().unwrap();
        assert!(
            genesis
                .parts
                .iter()
                .any(|(part, _)| *part == Part::CertsKnown),
            "genesis has no certificates to check"
        );
    }

    /// A payload inside a peer's leaf is not covered by the leaf commitment, so it is not taken.
    #[tokio::test]
    async fn drops_the_payload_inside_a_peers_leaf() {
        let ((item0, payload, common), item1) = chain().await;
        let mut stuffed = item0.clone();
        let (other, _) = Payload::from_transactions_sync(
            &[espresso_types::Transaction::new(1u32.into(), vec![7])],
            NodeState::mock().chain_config,
        )
        .unwrap();
        stuffed.leaf.fill_block_payload_unchecked(other);
        let peer = ChainStore::new(MemBackend::default());
        peer.put(&stuffed, &parts(&payload, &common)).await.unwrap();
        peer.put(&item1, &Parts::default()).await.unwrap();
        let store = filling(&item1, serve(peer).await);

        tokio::time::timeout(Duration::from_secs(10), async {
            while store.item(0).await.unwrap().is_none() {
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("the gap is filled");
        let filled = store.item(0).await.unwrap().unwrap();
        assert_eq!(filled.leaf.block_payload(), None);
    }

    /// The parts cursor does not pass a missing height that is not final yet: it can be stored
    /// later, without its parts.
    #[tokio::test]
    async fn the_parts_cursor_stops_at_a_pending_height() {
        let (_, item1) = chain().await;
        let store = ChainStore::new(MemBackend::default());
        store.put(&item1, &Parts::default()).await.unwrap();
        let peers = Peers::new(chain_api::Tables::new(store.clone()), Vec::new(), None);
        let mut filler = GapFiller::new(store, peers, None);
        filler.finality = 2;

        filler.fill_gaps().await.unwrap();
        assert_eq!(filler.items_from, 0);
        assert_eq!(filler.parts_from, 0);
    }

    /// A height that is not final yet is left to the API service's own consensus node.
    #[tokio::test]
    async fn leaves_heights_that_are_not_final() {
        let ((item0, payload, common), item1) = chain().await;
        let peer = ChainStore::new(MemBackend::default());
        peer.put(&item0, &parts(&payload, &common)).await.unwrap();
        peer.put(&item1, &Parts::default()).await.unwrap();
        let store = filling_with(&item1, serve(peer).await, FINALITY_VIEWS);

        sleep(Duration::from_secs(2)).await;
        assert_eq!(store.item(0).await.unwrap(), None);
    }

    #[tokio::test]
    async fn refuses_a_leaf_that_is_not_the_parent() {
        let ((mut item0, payload, common), item1) = chain().await;
        // Another genesis: the same height with a different block Merkle root.
        let mut tree = BlockMerkleTree::new(32);
        tree.push(item0.leaf.block_header().commit()).unwrap();
        *item0.leaf.block_header_mut().block_merkle_tree_root_mut() = tree.commitment();
        assert_ne!(item0.leaf.commit(), item1.leaf.parent_commitment());
        let peer = ChainStore::new(MemBackend::default());
        peer.put(&item0, &parts(&payload, &common)).await.unwrap();
        peer.put(&item1, &Parts::default()).await.unwrap();
        let store = filling(&item1, serve(peer).await);

        sleep(Duration::from_secs(2)).await;
        assert_eq!(store.item(0).await.unwrap(), None);
    }

    /// A gap that failed waits before it is tried again, twice as long after each failure.
    #[tokio::test(start_paused = true)]
    async fn backs_off_a_failing_gap() {
        let mut retries = Retries::default();
        retries.failed(3, 7);
        assert!(retries.contains(3));
        assert!(retries.due().is_empty());
        tokio::time::advance(RETRY_DELAY).await;
        assert_eq!(retries.due(), vec![(3, 7)]);

        retries.failed(3, 5);
        tokio::time::advance(RETRY_DELAY).await;
        assert!(retries.due().is_empty(), "the second wait is longer");
        tokio::time::advance(RETRY_DELAY).await;
        assert_eq!(retries.due(), vec![(3, 5)]);
        assert_eq!(retries.newest_end(), Some(5));

        retries.moved(3, 4);
        assert!(!retries.contains(3));
        assert_eq!(
            retries.due(),
            vec![(4, 5)],
            "a moved gap keeps its end and wait"
        );

        retries.forget_before(5);
        assert!(!retries.contains(4));
        retries.failed(9, 10);
        retries.succeeded(9);
        assert_eq!(retries.next(), None);
    }

    #[test]
    fn splits_heights_into_runs() {
        let heights: BTreeSet<u64> = [1, 2, 3, 7, 8].into_iter().chain(10..215).collect();
        assert_eq!(
            runs(&heights),
            vec![1..4, 7..9, 10..110, 110..210, 210..215]
        );
    }
}
