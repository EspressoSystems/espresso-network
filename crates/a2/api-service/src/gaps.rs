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
//! The peers are the API services of the validators, from the discovery records consensus holds,
//! and the bootstrap URLs the operator gives.

use std::{collections::BTreeSet, net::SocketAddr, ops::Range, sync::Mutex, time::Duration};

use anyhow::{Context, bail, ensure};
use chain_store::{ChainStore, Item, Part, Parts, Record};
use committable::Committable;
use espresso_types::{Header, Payload};
use hotshot_types::{
    data::EpochNumber,
    traits::{
        EncodeBytes,
        block_contents::{BlockHeader, BlockPayload},
    },
};
use light_client::consensus::payload::PayloadProof;
use tokio::time::{Instant, sleep};
use tonic::transport::{ClientTlsConfig, Endpoint};
use url::Url;

use crate::{
    Membership,
    chain::{RECORD_RANGE_LIMIT, from_proto},
    discovery::service_records,
    proto::{GetRecordsRequest, chain_service_client::ChainServiceClient},
};

/// The least time between two passes.
const PASS_INTERVAL: Duration = Duration::from_secs(1);

/// How many gaps one pass fills.
const GAPS_PER_PASS: usize = 16;

/// How long one request to a peer may take.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// The largest record a peer may send, which a payload bounds.
const MAX_RECORD_SIZE: usize = 64 << 20;

pub struct GapFiller {
    pub(crate) store: ChainStore,
    consensus_api: SocketAddr,
    bootstrap: Vec<Url>,

    /// The stake tables to check certificates against; without them certificates are not
    /// filled.
    membership: Option<Membership>,

    /// The epochs the store records as checked, as far as this filler has seen.
    pub(crate) checked_epochs: Mutex<BTreeSet<EpochNumber>>,
}

impl GapFiller {
    pub fn new(
        store: ChainStore,
        consensus_api: SocketAddr,
        bootstrap: Vec<Url>,
        membership: Option<Membership>,
    ) -> Self {
        Self {
            store,
            consensus_api,
            bootstrap,
            membership,
            checked_epochs: Mutex::default(),
        }
    }

    /// Fills gaps whenever the store changes, at most once per [`PASS_INTERVAL`]. Never returns.
    pub async fn run(self) {
        let mut changes = self.store.changes();
        loop {
            let started = Instant::now();
            changes.borrow_and_update();
            if let Err(err) = self.pass().await {
                tracing::warn!("filling gaps failed: {err:#}");
            }
            sleep(PASS_INTERVAL.saturating_sub(started.elapsed())).await;
            if changes.changed().await.is_err() {
                return;
            }
        }
    }

    /// Fills the first gaps of each kind, each from the first peer that has it, then derives the
    /// stake tables of the stored epoch roots and checks the certificates they allow.
    async fn pass(&self) -> anyhow::Result<()> {
        let peers = self.peers().await;
        ensure!(!peers.is_empty(), "no API service to fill gaps from");
        self.fill_gaps(&peers).await?;
        if let Some(membership) = &self.membership {
            self.follow_roots(membership).await?;
            self.check_certs(membership, &peers).await?;
        }
        Ok(())
    }

    async fn fill_gaps(&self, peers: &[Url]) -> anyhow::Result<()> {
        let missing = self.store.missing_items(0, GAPS_PER_PASS).await?;
        let mut parts: BTreeSet<u64> = BTreeSet::new();
        for part in [Part::Payload, Part::VidCommon] {
            let limit = GAPS_PER_PASS * RECORD_RANGE_LIMIT as usize;
            parts.extend(self.store.missing_parts(part, 0, limit).await?);
        }
        for gap in missing {
            if let Err(err) = self.fill_items(gap.clone(), peers).await {
                tracing::warn!(?gap, "filling a gap failed: {err:#}");
            }
        }
        for heights in runs(&parts) {
            if let Err(err) = self.fill_parts(heights.clone(), peers).await {
                tracing::warn!(?heights, "filling payloads failed: {err:#}");
            }
        }
        Ok(())
    }

    /// The bootstrap URLs, then the API services of the validators that consensus knows.
    async fn peers(&self) -> Vec<Url> {
        let mut peers = self.bootstrap.clone();
        match service_records(self.consensus_api, None).await {
            Ok((_, records)) => {
                for record in records {
                    match record.record.api_url.parse() {
                        Ok(url) if !peers.contains(&url) => peers.push(url),
                        Ok(_) => {},
                        Err(err) => tracing::debug!(%err, "a record with a bad API URL"),
                    }
                }
            },
            Err(err) => tracing::debug!(%err, "no records from consensus"),
        }
        peers
    }

    /// Fills `gap` from the top down, from the first peer that continues the chain.
    async fn fill_items(&self, gap: Range<u64>, peers: &[Url]) -> anyhow::Result<()> {
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
        peer: &Url,
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
            let parts = checked_parts(item.leaf.block_header(), &record)
                .inspect_err(|err| tracing::debug!(%peer, "payload not taken: {err:#}"))
                .unwrap_or_default();
            self.store.put(&item, &parts).await?;
            (end, child) = (record.height, item);
        }
        Ok((end, child))
    }

    /// Fills the payloads and VID common data of the stored `heights` from the peers.
    async fn fill_parts(&self, heights: Range<u64>, peers: &[Url]) -> anyhow::Result<()> {
        for peer in peers {
            let records = match fetch(peer, heights.clone(), false).await {
                Ok(records) => records,
                Err(err) => {
                    tracing::debug!(%peer, "{err:#}");
                    continue;
                },
            };
            for record in records {
                let Some(item) = self.store.item(record.height).await? else {
                    continue;
                };
                if let Ok(parts) = checked_parts(item.leaf.block_header(), &record) {
                    self.store.put_parts(record.height, &parts).await?;
                }
            }
            if self.lacks_none(heights.clone()).await? {
                return Ok(());
            }
        }
        bail!("no peer has the payloads of heights {heights:?}")
    }

    async fn lacks_none(&self, heights: Range<u64>) -> anyhow::Result<bool> {
        for part in [Part::Payload, Part::VidCommon] {
            let missing = self.store.missing_parts(part, heights.start, 1).await?;
            if missing.first().is_some_and(|h| heights.contains(h)) {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

/// The records of `heights` that `peer` has, in height order, without payloads and VID common
/// data if `skip_payloads`.
pub(crate) async fn fetch(
    peer: &Url,
    heights: Range<u64>,
    skip_payloads: bool,
) -> anyhow::Result<Vec<Record>> {
    let mut endpoint = Endpoint::from_shared(peer.to_string())?
        .connect_timeout(REQUEST_TIMEOUT)
        .timeout(REQUEST_TIMEOUT);
    if peer.scheme() == "https" {
        endpoint = endpoint.tls_config(ClientTlsConfig::new().with_webpki_roots())?;
    }
    let mut client = ChainServiceClient::new(endpoint.connect().await?)
        .max_decoding_message_size(MAX_RECORD_SIZE);
    let mut stream = client
        .get_records(GetRecordsRequest {
            from: Some(heights.start),
            until: Some(heights.end),
            skip_payloads,
        })
        .await?
        .into_inner();
    let mut records = Vec::new();
    while let Some(record) = stream.message().await? {
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
    Ok(item)
}

/// The payload and VID common data of `record`, if both are there and recompute the payload
/// commitment of `header`. The payload is returned as encoded again after the check.
fn checked_parts(header: &Header, record: &Record) -> anyhow::Result<Parts> {
    let parts = record.parts()?;
    let (Some(payload), Some(common)) = (parts.payload, parts.vid_common) else {
        bail!("the record of height {} lacks parts", record.height);
    };
    let payload = Payload::from_bytes(&payload, header.metadata());
    let (payload, common) = PayloadProof::new(payload, common).verify_with_vid_common(header)?;
    Ok(Parts {
        payload: Some(payload.encode().to_vec().into()),
        vid_common: Some(common),
        ..Default::default()
    })
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
    use crate::ApiService;

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

    /// A store holding `item1` only, filling its gaps from `peer`.
    fn filling(item1: &Item, peer: Url) -> ChainStore {
        let store = ChainStore::new(MemBackend::default());
        let filler = GapFiller::new(
            store.clone(),
            "127.0.0.1:1".parse().unwrap(),
            vec![peer],
            None,
        );
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

    #[test]
    fn splits_heights_into_runs() {
        let heights: BTreeSet<u64> = [1, 2, 3, 7, 8].into_iter().chain(10..215).collect();
        assert_eq!(
            runs(&heights),
            vec![1..4, 7..9, 10..110, 110..210, 210..215]
        );
    }
}
