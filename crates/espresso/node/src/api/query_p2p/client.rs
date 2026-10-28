use std::{
    collections::HashMap,
    future::Future,
    ops::Range,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use ::light_client::{
    client::Client,
    consensus::{
        header::HeaderProof, leaf::LeafProof, namespace::NamespaceProof, payload::PayloadProof,
    },
    storage::LeafRequest,
};
use anyhow::{Context as _, Result, bail, ensure};
use espresso_types::{Certificate2, Header, NamespaceId, PubKey, SeqTypes, v0_3::StakeTableEvent};
use hotshot_new_protocol::network::QuerySender;
use hotshot_query_service::{
    availability::{LeafId, LeafQueryData},
    node::BlockId,
    types::HeightIndexed,
};
use hotshot_types::{data::EpochNumber, utils::epoch_from_block_number};
use parking_lot::Mutex;
use rand::seq::SliceRandom;
use tokio::{sync::oneshot, time::timeout};
use tracing::debug;

use super::message::{QueryMessage, QueryReply, QueryRequest};
use crate::api::{ApiState, data_source::StakeTableDataSource};

/// Peers tried for one call before it falls back to HTTP.
const PEERS_PER_CALL: usize = 3;

/// A peer that fails a call is skipped for this long.
const COOLDOWN: Duration = Duration::from_secs(60);

/// How long to wait for a leaf proof or a set of leaves.
const SMALL_REPLY_TIMEOUT: Duration = Duration::from_secs(2);

/// How long to wait for one page of payload proofs.
const PAGE_TIMEOUT: Duration = Duration::from_secs(10);

/// How long one peer may take to serve all pages of a call.
const CALL_TIMEOUT: Duration = Duration::from_secs(60);

/// A light client [`Client`] that fetches from staked peers over the query lane.
///
/// Each call is served by one peer, chosen at random among staked peers that have not failed
/// recently. Methods the query lane does not serve return an error, so the provider moves on.
#[derive(Clone)]
pub(crate) struct PeerClient(Arc<Inner>);

struct Inner {
    state: ApiState,
    sender: OnceLock<QuerySender<SeqTypes>>,
    pending: Mutex<HashMap<(PubKey, u64), oneshot::Sender<QueryReply>>>,
    next_id: AtomicU64,
    cooldowns: Mutex<HashMap<PubKey, Instant>>,
}

impl std::fmt::Debug for PeerClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PeerClient").finish_non_exhaustive()
    }
}

impl PeerClient {
    pub(in crate::api) fn new(state: ApiState) -> PeerClient {
        PeerClient(Arc::new(Inner {
            state,
            sender: OnceLock::new(),
            pending: Default::default(),
            next_id: AtomicU64::new(0),
            cooldowns: Default::default(),
        }))
    }

    /// Start sending requests over the query lane.
    pub(super) fn connect(&self, sender: QuerySender<SeqTypes>) {
        _ = self.0.sender.set(sender);
    }

    /// Hand a reply to the call waiting for it, if any.
    pub(super) fn complete(&self, from: PubKey, id: u64, reply: QueryReply) {
        let waiter = self.0.pending.lock().remove(&(from, id));
        match waiter {
            Some(waiter) => _ = waiter.send(reply),
            None => debug!(%from, id, "unexpected query reply"),
        }
    }

    /// Run `attempt` against up to [`PEERS_PER_CALL`] peers until one succeeds.
    async fn with_any_peer<T, F, Fut>(&self, what: &str, attempt: F) -> Result<T>
    where
        F: Fn(PubKey) -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        ensure!(
            self.0.sender.get().is_some(),
            "query lane not connected yet"
        );
        for peer in self.healthy_peers().await.into_iter().take(PEERS_PER_CALL) {
            match timeout(CALL_TIMEOUT, attempt(peer)).await {
                Ok(Ok(value)) => return Ok(value),
                Ok(Err(err)) => {
                    debug!(%peer, what, err = %format_args!("{err:#}"), "peer failed query")
                },
                Err(_) => debug!(%peer, what, "peer timed out"),
            }
            self.0
                .cooldowns
                .lock()
                .insert(peer, Instant::now() + COOLDOWN);
        }
        bail!("no staked peer served {what}")
    }

    /// Staked peers not cooling down, in random order.
    async fn healthy_peers(&self) -> Vec<PubKey> {
        let ctx = self.0.state.context().await;
        let me = ctx.validator_config().map(|config| config.public_key);
        let consensus = ctx.consensus();
        let epoch = consensus.current_epoch().await;
        let membership = consensus.membership_coordinator().await;
        let stake_table = match membership.stake_table_for_epoch(epoch) {
            Ok(stake_table) => stake_table,
            Err(err) => {
                debug!(?epoch, err = %format_args!("{err:#}"), "no stake table for query peers");
                return Vec::new();
            },
        };
        let now = Instant::now();
        let mut peers = {
            let mut cooldowns = self.0.cooldowns.lock();
            cooldowns.retain(|_, until| *until > now);
            stake_table
                .stake_table()
                .map(|entry| entry.stake_table_entry.stake_key)
                .filter(|key| Some(*key) != me)
                .filter(|key| !cooldowns.contains_key(key))
                .collect::<Vec<_>>()
        };
        peers.shuffle(&mut rand::thread_rng());
        peers
    }

    /// Send one request to `peer` and wait for its reply.
    async fn ask(&self, peer: PubKey, request: QueryRequest, wait: Duration) -> Result<QueryReply> {
        let sender = self
            .0
            .sender
            .get()
            .context("query lane not connected yet")?;
        let id = self.0.next_id.fetch_add(1, Ordering::Relaxed);
        let (waiter, reply) = oneshot::channel();
        self.0.pending.lock().insert((peer, id), waiter);
        let _pending = Pending {
            client: self,
            key: (peer, id),
        };
        sender.send(&peer, QueryMessage::Request { id, request }.encode()?)?;
        match timeout(wait, reply).await {
            Ok(Ok(QueryReply::Unavailable)) => bail!("peer cannot serve the request"),
            Ok(Ok(reply)) => Ok(reply),
            Ok(Err(_)) => bail!("query lane closed"),
            Err(_) => bail!("no reply within {wait:?}"),
        }
    }

    async fn leaves_from(
        &self,
        peer: PubKey,
        ranges: &[Range<u64>],
    ) -> Result<Vec<LeafQueryData<SeqTypes>>> {
        let request = QueryRequest::Leaves {
            ranges: ranges.to_vec(),
        };
        let QueryReply::Leaves(leaves) = self.ask(peer, request, SMALL_REPLY_TIMEOUT).await? else {
            bail!("reply does not match the request");
        };
        // Checked here, not only by the light client, so a peer that serves the wrong heights
        // cools down.
        let expected = ranges.iter().cloned().flatten();
        ensure!(
            leaves.iter().map(|leaf| leaf.height()).eq(expected),
            "leaves do not match the requested heights"
        );
        Ok(leaves)
    }

    async fn payload_proofs_from(
        &self,
        peer: PubKey,
        ranges: &[Range<u64>],
    ) -> Result<Vec<PayloadProof>> {
        let wanted = heights(ranges);
        let mut remaining = ranges.to_vec();
        let mut proofs = Vec::new();
        loop {
            let request = QueryRequest::PayloadProofs {
                ranges: remaining.clone(),
            };
            let QueryReply::PayloadProofs { proofs: page, next } =
                self.ask(peer, request, PAGE_TIMEOUT).await?
            else {
                bail!("reply does not match the request");
            };
            ensure!(!page.is_empty(), "empty page");
            remaining = skip_heights(&remaining, page.len() as u64);
            proofs.extend(page);
            match next {
                None => {
                    ensure!(proofs.len() as u64 == wanted, "missing payload proofs");
                    return Ok(proofs);
                },
                Some(next) => ensure!(
                    remaining.first().map(|range| range.start) == Some(next),
                    "page does not resume at the next requested height"
                ),
            }
        }
    }
}

/// Removes a call's waiter when the call ends, however it ends.
struct Pending<'a> {
    client: &'a PeerClient,
    key: (PubKey, u64),
}

impl Drop for Pending<'_> {
    fn drop(&mut self) {
        self.client.0.pending.lock().remove(&self.key);
    }
}

fn heights(ranges: &[Range<u64>]) -> u64 {
    ranges.iter().map(|range| range.end - range.start).sum()
}

/// `ranges` without their first `n` heights.
fn skip_heights(ranges: &[Range<u64>], mut n: u64) -> Vec<Range<u64>> {
    let mut rest = Vec::new();
    for range in ranges {
        let len = range.end - range.start;
        if n >= len {
            n -= len;
            continue;
        }
        rest.push(range.start + n..range.end);
        n = 0;
    }
    rest
}

impl Client for PeerClient {
    async fn block_height(&self) -> Result<u64> {
        bail!("block height is not served over the query lane")
    }

    async fn leaf_proof(
        &self,
        id: impl Into<LeafRequest> + Send,
        finalized: Option<u64>,
    ) -> Result<LeafProof> {
        let height = match id.into() {
            LeafRequest::Leaf(LeafId::Number(n)) | LeafRequest::Header(BlockId::Number(n)) => {
                n as u64
            },
            other => bail!("the query lane serves leaf proofs by height, not {other}"),
        };
        self.with_any_peer("leaf proof", |peer| async move {
            let request = QueryRequest::LeafProof { height, finalized };
            match self.ask(peer, request, SMALL_REPLY_TIMEOUT).await? {
                QueryReply::LeafProof(proof) => Ok(proof),
                _ => bail!("reply does not match the request"),
            }
        })
        .await
    }

    async fn header_proof(&self, _root: u64, _id: BlockId<SeqTypes>) -> Result<HeaderProof> {
        bail!("header proofs are not served over the query lane")
    }

    async fn get_leaves_in_range(
        &self,
        start: usize,
        end: usize,
    ) -> Result<Vec<LeafQueryData<SeqTypes>>> {
        let range = start as u64..end as u64;
        self.get_leaves_for_ranges(std::slice::from_ref(&range))
            .await
    }

    async fn get_leaves_for_ranges(
        &self,
        ranges: &[Range<u64>],
    ) -> Result<Vec<LeafQueryData<SeqTypes>>> {
        self.with_any_peer("leaves", |peer| self.leaves_from(peer, ranges))
            .await
    }

    async fn payload_proof(&self, height: u64) -> Result<PayloadProof> {
        let range = height..height + 1;
        let mut proofs = self
            .payload_proofs_for_ranges(std::slice::from_ref(&range))
            .await?;
        proofs.pop().context("missing payload proof")
    }

    async fn payload_proofs_in_range(&self, start: u64, end: u64) -> Result<Vec<PayloadProof>> {
        let range = start..end;
        self.payload_proofs_for_ranges(std::slice::from_ref(&range))
            .await
    }

    async fn payload_proofs_for_ranges(&self, ranges: &[Range<u64>]) -> Result<Vec<PayloadProof>> {
        self.with_any_peer("payload proofs", |peer| {
            self.payload_proofs_from(peer, ranges)
        })
        .await
    }

    async fn namespace_proof(
        &self,
        _height: u64,
        _namespace: NamespaceId,
    ) -> Result<NamespaceProof> {
        bail!("namespace proofs are not served over the query lane")
    }

    async fn namespace_proofs_in_range(
        &self,
        _start: u64,
        _end: u64,
        _namespace: NamespaceId,
    ) -> Result<Vec<NamespaceProof>> {
        bail!("namespace proofs are not served over the query lane")
    }

    async fn namespaces_proofs_in_range(
        &self,
        _start: u64,
        _end: u64,
        _namespaces: &[NamespaceId],
    ) -> Result<Vec<HashMap<NamespaceId, NamespaceProof>>> {
        bail!("namespace proofs are not served over the query lane")
    }

    /// Read from this node's own stake table events, the same way the light client API serves
    /// them.
    async fn stake_table_events(&self, epoch: EpochNumber) -> Result<Vec<StakeTableEvent>> {
        let ctx = self.0.state.context().await;
        let node_state = ctx.node_state();
        let epoch_height = node_state.epoch_height.context("epoch height not set")?;
        let first_epoch = epoch_from_block_number(node_state.epoch_start_block, epoch_height);
        ensure!(
            *epoch >= first_epoch + 2,
            "no stake table events before epoch {}",
            first_epoch + 2
        );

        fn l1_finalized(root: Option<Header>) -> Result<u64> {
            Ok(root
                .context("missing epoch root")?
                .l1_finalized()
                .context("epoch root has no L1 finalized block")?
                .number())
        }

        // The root of epoch `e - 2` is stored under `e`.
        let membership = ctx.membership();
        let to_l1_block = l1_finalized(membership.load_epoch_root(epoch).await?)?;
        let from_l1_block = if *epoch >= first_epoch + 3 {
            l1_finalized(membership.load_epoch_root(epoch - 1).await?)? + 1
        } else {
            0
        };
        StakeTableDataSource::stake_table_events(&self.0.state, from_l1_block, to_l1_block).await
    }

    async fn cert2(&self, _height: u64) -> Result<Option<Certificate2<SeqTypes>>> {
        bail!("certificates are not served over the query lane")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[expect(clippy::single_range_in_vec_init, reason = "a list holding one range")]
    fn skip_heights_crosses_ranges() {
        let ranges = [0..3, 10..12];
        assert_eq!(skip_heights(&ranges, 0), Vec::from([0..3, 10..12]));
        assert_eq!(skip_heights(&ranges, 2), Vec::from([2..3, 10..12]));
        assert_eq!(skip_heights(&ranges, 3), Vec::from([10..12]));
        assert_eq!(skip_heights(&ranges, 4), Vec::from([11..12]));
        assert_eq!(skip_heights(&ranges, 5), Vec::<Range<u64>>::new());
    }
}
