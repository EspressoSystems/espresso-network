//! Re-creation of a node's lost VID shares from peers' shares.

mod run;
pub mod serve;

use std::{cmp::Reverse, collections::BTreeSet, num::NonZeroUsize, sync::Arc, time::Duration};

use hotshot_types::{
    data::{
        EpochNumber, Leaf2, VidCommitment, VidCommitment2, VidDisperseShare2, ViewNumber,
        ns_table::ns_table_payload_byte_len, vid_disperse::AvidmGf2DisperseParams,
    },
    epoch_membership::EpochMembershipCoordinator,
    traits::{
        block_contents::{BlockHeader, EncodeBytes},
        metrics::{Counter, CounterFamily, Gauge, HistogramFamily, Metrics},
        node_implementation::NodeType,
        signature_key::SignatureKey,
    },
};
use tokio::sync::{mpsc, watch};
use tokio_util::task::AbortOnDropHandle;
use tracing::warn;

pub use self::run::{HealCtx, ShareResponse, run};
use crate::{
    message::{
        Message, MessageType,
        share::{ShareFetchMessage, ShareFetchRequest, ShareFetchResponse},
    },
    network::Sender,
    storage::NewProtocolStorage,
    vid::{self, Metadata, VidDisperseError},
};

/// Requests the healer can have queued for the network task.
const REQUEST_QUEUE: usize = 256;
/// Responses queued for the healer; further ones are dropped, the healer re-asks on timeout.
const RESPONSE_QUEUE: usize = 256;

/// Tuning of the share healer and of serving shares to peers.
#[derive(Clone, Debug)]
pub struct HealConfig {
    /// How long shares are retained, measured in header time.
    pub w: Duration,
    /// Views behind the current one a height must be before it is healed.
    pub tip_margin: u64,
    /// Heights whose share would expire within this long are not worth healing.
    pub min_remaining: Duration,
    pub scan_interval: Duration,
    pub req_timeout: Duration,
    /// How long a peer that answered `Busy` is left alone before it is asked again.
    pub busy_backoff: Duration,
    /// Heights healed concurrently.
    pub heights_inflight: NonZeroUsize,
    /// Requests outstanding to one peer across all heights.
    pub peer_inflight: NonZeroUsize,
    /// Memory budget over the heights in flight, see `HealTarget::cost`. At least one height is
    /// always admitted.
    pub inflight_bytes: usize,
    pub serve_inflight: NonZeroUsize,
    pub max_response_bytes: usize,
}

impl HealConfig {
    /// The oldest header time worth healing at `now`: the share must outlive `min_remaining`.
    pub fn oldest_ts(&self, now: u64) -> u64 {
        now.saturating_sub(self.w.as_secs()) + self.min_remaining.as_secs()
    }
}

impl Default for HealConfig {
    fn default() -> Self {
        Self {
            w: Duration::from_secs(7 * 24 * 3600),
            tip_margin: 100,
            min_remaining: Duration::from_secs(3600),
            scan_interval: Duration::from_secs(60),
            req_timeout: Duration::from_secs(10),
            busy_backoff: Duration::from_secs(1),
            heights_inflight: NonZeroUsize::new(8).unwrap(),
            peer_inflight: NonZeroUsize::new(2).unwrap(),
            inflight_bytes: 512 << 20,
            serve_inflight: NonZeroUsize::new(16).unwrap(),
            max_response_bytes: 64 << 20,
        }
    }
}

/// A decided height whose share this node lost.
#[derive(Clone, Debug)]
pub struct HealTarget<T: NodeType> {
    pub view: ViewNumber,
    pub epoch: EpochNumber,
    pub header_ts: u64,
    pub commit: VidCommitment2,
    pub metadata: Metadata<T>,
}

/// Peak memory of a height in flight, in multiples of its payload `P`. The recovery threshold is a
/// third of the weight, so the shares fetched to 1.5 times the threshold are about 1.5 P.
/// - Decode: those shares, the recovered bytes and the payload built from them, 3.5 P.
/// - Re-encode, fetched shares and the original payload dropped: the copy of the payload in the
///   dispersal params and the pieces for every recipient, P + 3 P = 4 P.
const COST_FACTOR: usize = 4;

impl<T: NodeType> HealTarget<T> {
    /// Estimated memory while in flight, from the payload length in the header's namespace table.
    /// A table that does not parse has an unknown payload size and costs `usize::MAX`, so the
    /// height runs alone. Every height costs at least a byte.
    pub fn cost(&self) -> usize {
        ns_table_payload_byte_len(&self.metadata.encode())
            .map_or(usize::MAX, |len| len.saturating_mul(COST_FACTOR))
            .max(1)
    }
}

/// The decided `leaves` whose share is missing and worth recreating, newest first.
///
/// A height qualifies when its header time leaves at least `min_remaining` before the share would
/// expire, it is `tip_margin` views behind `anchor_view`, it is not in `held`, it commits with
/// VID V2 and `recipient_epoch` says this node held a share in its epoch (returning that epoch).
pub fn heal_targets<T: NodeType>(
    leaves: &[Leaf2<T>],
    held: &BTreeSet<ViewNumber>,
    anchor_view: ViewNumber,
    now_ts: u64,
    recipient_epoch: impl Fn(&Leaf2<T>) -> Option<EpochNumber>,
    cfg: &HealConfig,
) -> Vec<HealTarget<T>> {
    let oldest_ts = cfg.oldest_ts(now_ts);
    let mut targets: Vec<_> = leaves
        .iter()
        .filter_map(|leaf| {
            let view = leaf.view_number();
            let header = leaf.block_header();
            let VidCommitment::V2(commit) = header.payload_commitment() else {
                return None;
            };
            let wanted = view != ViewNumber::genesis()
                && view.u64() + cfg.tip_margin <= anchor_view.u64()
                && header.timestamp() >= oldest_ts
                && !held.contains(&view);
            wanted.then_some(())?;
            Some(HealTarget {
                view,
                epoch: recipient_epoch(leaf)?,
                header_ts: header.timestamp(),
                commit,
                metadata: header.metadata().clone(),
            })
        })
        .collect();
    targets.sort_by_key(|target| Reverse(target.view));
    targets
}

#[derive(Clone)]
pub struct HealMetrics {
    targets: Arc<dyn Gauge>,
    inflight: Arc<dyn Gauge>,
    unreachable: Arc<dyn Gauge>,
    done: Arc<dyn Counter>,
    fetch_bytes: Arc<dyn Counter>,
    verify_fail: Arc<dyn Counter>,
    skipped: Arc<dyn CounterFamily>,
    height_seconds: Arc<dyn HistogramFamily>,
}

impl HealMetrics {
    pub fn new(metrics: &dyn Metrics) -> Self {
        let counter = |name: &str| metrics.create_counter(name.into(), None).into();
        let gauge = |name: &str| metrics.create_gauge(name.into(), None).into();
        Self {
            targets: gauge("share_heal_targets"),
            inflight: gauge("share_heal_inflight"),
            unreachable: gauge("share_heal_unreachable"),
            done: counter("share_heal_done_total"),
            fetch_bytes: counter("share_heal_fetch_bytes_total"),
            verify_fail: counter("share_heal_verify_fail_total"),
            skipped: metrics
                .counter_family("share_heal_skipped_total".into(), vec!["reason".into()])
                .into(),
            height_seconds: metrics
                .histogram_family("share_heal_height_seconds".into(), vec!["phase".into()])
                .into(),
        }
    }

    fn skipped(&self, reason: &str) {
        self.skipped.create(vec![reason.into()]).add(1);
    }

    fn phase(&self, phase: &str, took: Duration) {
        self.height_seconds
            .create(vec![phase.into()])
            .add_point(took.as_secs_f64());
    }
}

/// The healer's link to the coordinator: peers' responses in, the node's view in, and the tasks
/// that heal and send requests, stopped when this is dropped.
pub struct Healer<T: NodeType, S> {
    responses: mpsc::Sender<ShareResponse<T>>,
    view: watch::Sender<ViewNumber>,
    unstarted: Option<Unstarted<T, S>>,
    tasks: Vec<AbortOnDropHandle<()>>,
}

struct Unstarted<T: NodeType, S> {
    ctx: HealCtx<T, S>,
    responses: mpsc::Receiver<ShareResponse<T>>,
    requests: mpsc::Receiver<(T::SignatureKey, ShareFetchRequest)>,
    network: Sender<T>,
}

impl<T: NodeType, S: NewProtocolStorage<T>> Healer<T, S> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: HealConfig,
        storage: S,
        membership: EpochMembershipCoordinator<T>,
        public_key: T::SignatureKey,
        private_key: <T::SignatureKey as SignatureKey>::PrivateKey,
        network: Sender<T>,
        view: ViewNumber,
        metrics: &dyn Metrics,
    ) -> Self {
        let (requests_tx, requests) = mpsc::channel(REQUEST_QUEUE);
        let (responses_tx, responses) = mpsc::channel(RESPONSE_QUEUE);
        let (view_tx, view_rx) = watch::channel(view);
        let ctx = HealCtx {
            config,
            storage,
            membership,
            public_key,
            private_key,
            requests: requests_tx,
            view: view_rx,
            metrics: HealMetrics::new(metrics),
        };
        Self {
            responses: responses_tx,
            view: view_tx,
            unstarted: Some(Unstarted {
                ctx,
                responses,
                requests,
                network,
            }),
            tasks: Vec::new(),
        }
    }

    /// Spawn the healer and its request sender. Does nothing when already started.
    pub fn start(&mut self) {
        let Some(Unstarted {
            ctx,
            responses,
            requests,
            network,
        }) = self.unstarted.take()
        else {
            return;
        };
        let sender = forward_requests(requests, network, ctx.public_key.clone(), ctx.view.clone());
        self.tasks
            .push(AbortOnDropHandle::new(tokio::spawn(sender)));
        self.tasks
            .push(AbortOnDropHandle::new(tokio::spawn(run(ctx, responses))));
    }

    /// The node's current view, as last passed to `on_view_changed`.
    pub fn view_receiver(&self) -> watch::Receiver<ViewNumber> {
        self.view.subscribe()
    }

    pub fn on_view_changed(&self, view: ViewNumber) {
        self.view.send_replace(view);
    }

    pub fn on_response(&self, from: T::SignatureKey, response: ShareFetchResponse<T>) {
        if self.responses.try_send((from, response)).is_err() {
            warn!("share heal is behind, dropping a share response");
        }
    }
}

/// Unicast the healer's requests in the node's current view.
async fn forward_requests<T: NodeType>(
    mut requests: mpsc::Receiver<(T::SignatureKey, ShareFetchRequest)>,
    network: Sender<T>,
    public_key: T::SignatureKey,
    view: watch::Receiver<ViewNumber>,
) {
    while let Some((peer, request)) = requests.recv().await {
        let message = Message {
            sender: public_key.clone(),
            message_type: MessageType::ShareFetch(ShareFetchMessage::Req(request)),
        };
        let slot = *view.borrow();
        if let Err(err) = network.unicast(slot, &peer, &message) {
            warn!(%peer, %err, "failed to send share request");
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum HealError {
    #[error("own key has no share in the committee")]
    NotRecipient,

    #[error("re-encoding failed: {0}")]
    Encode(#[from] VidDisperseError),

    #[error("re-encoded share is inconsistent with the commitment")]
    Inconsistent,

    #[error("re-encoded share fails verification")]
    Unverified,
}

/// Re-encode the payload in `params` and return the share `own` received when
/// it was originally dispersed, byte-equal to what the live path stored.
pub fn rederive_own_share<T: NodeType>(
    params: &AvidmGf2DisperseParams<T>,
    own: &T::SignatureKey,
    view: ViewNumber,
    epoch: EpochNumber,
    commit: VidCommitment2,
) -> Result<VidDisperseShare2<T>, HealError> {
    let index = params
        .recipients
        .iter()
        .position(|key| key == own)
        .ok_or(HealError::NotRecipient)?;
    let all: Vec<usize> = (0..params.ns_table.len()).collect();
    let pieces = vid::encode_namespaces(params, &all)?.swap_remove(index);
    let share = vid::assemble_share(
        view,
        epoch,
        commit,
        own.clone(),
        params.param.clone(),
        pieces,
    );
    if !share.is_consistent() {
        return Err(HealError::Inconsistent);
    }
    Ok(share)
}

#[cfg(test)]
mod tests {
    use hotshot_example_types::node_types::TestTypes;
    use hotshot_types::{
        data::VidDisperse2, signature_key::BLSPubKey, traits::signature_key::SignatureKey,
    };

    use super::*;
    use crate::tests::common::utils::{TestData, mock_membership};

    #[tokio::test]
    async fn heal_rederive_matches_live_ok() {
        let data = TestData::new(3).await;
        let membership = mock_membership();
        for view in &data.views {
            let params = VidDisperse2::<TestTypes>::disperse_params(
                &view.payload,
                &membership,
                Some(view.epoch_number),
                &view.proposal.data.block_header.metadata,
            )
            .unwrap();
            let commit = view.vid_commitment();
            for i in 0..10u64 {
                let own = BLSPubKey::generated_from_seed_indexed([0u8; 32], i).0;
                let share =
                    rederive_own_share(&params, &own, view.view_number, view.epoch_number, commit)
                        .unwrap();
                assert_eq!(share, view.vid_share_for(&own));
            }
        }
    }

    #[tokio::test]
    async fn heal_rederive_not_recipient_err() {
        let data = TestData::new(1).await;
        let view = &data.views[0];
        let params = VidDisperse2::<TestTypes>::disperse_params(
            &view.payload,
            &mock_membership(),
            Some(view.epoch_number),
            &view.proposal.data.block_header.metadata,
        )
        .unwrap();
        let stranger = BLSPubKey::generated_from_seed_indexed([0u8; 32], 99).0;
        let err = rederive_own_share(
            &params,
            &stranger,
            view.view_number,
            view.epoch_number,
            view.vid_commitment(),
        )
        .unwrap_err();
        assert!(matches!(err, HealError::NotRecipient));
    }

    fn cfg() -> HealConfig {
        HealConfig {
            w: Duration::from_secs(1000),
            tip_margin: 2,
            min_remaining: Duration::from_secs(100),
            ..HealConfig::default()
        }
    }

    /// One leaf per view, view `i + 1` at header time `100 * (i + 1)`.
    async fn leaves(count: usize) -> Vec<Leaf2<TestTypes>> {
        let data = TestData::new(count).await;
        data.views
            .iter()
            .enumerate()
            .map(|(i, view)| {
                let mut leaf = view.leaf.clone();
                leaf.block_header_mut().timestamp = 100 * (i as u64 + 1);
                leaf
            })
            .collect()
    }

    fn views(targets: &[HealTarget<TestTypes>]) -> Vec<u64> {
        targets.iter().map(|t| t.view.u64()).collect()
    }

    fn member(leaf: &Leaf2<TestTypes>) -> Option<EpochNumber> {
        leaf.epoch(10)
    }

    #[tokio::test]
    async fn heal_targets_newest_first_ok() {
        let leaves = leaves(6).await;
        let held = BTreeSet::from([ViewNumber::new(2)]);
        let targets = heal_targets(&leaves, &held, ViewNumber::new(10), 1000, member, &cfg());
        // Views 1..=6; 2 is held; the `now - w + min_remaining` bound of 100 admits all times.
        assert_eq!(views(&targets), vec![6, 5, 4, 3, 1]);
        assert!(targets.iter().all(|t| t.header_ts == 100 * t.view.u64()));
    }

    #[tokio::test]
    async fn heal_targets_tip_margin_ok() {
        let leaves = leaves(6).await;
        let targets = heal_targets(
            &leaves,
            &BTreeSet::new(),
            ViewNumber::new(5),
            1000,
            member,
            &cfg(),
        );
        assert_eq!(views(&targets), vec![3, 2, 1]);
    }

    #[tokio::test]
    async fn heal_targets_near_expiry_ok() {
        let leaves = leaves(6).await;
        // Bound 1500 - 1000 + 100 = 600: only views at time 600 and later remain.
        let targets = heal_targets(
            &leaves,
            &BTreeSet::new(),
            ViewNumber::new(10),
            1500,
            member,
            &cfg(),
        );
        assert_eq!(views(&targets), vec![6]);
    }

    #[tokio::test]
    async fn heal_targets_not_recipient_ok() {
        let leaves = leaves(3).await;
        let targets = heal_targets(
            &leaves,
            &BTreeSet::new(),
            ViewNumber::new(10),
            1000,
            |leaf| {
                (leaf.view_number() != ViewNumber::new(2))
                    .then(|| leaf.epoch(10))
                    .flatten()
            },
            &cfg(),
        );
        assert_eq!(views(&targets), vec![3, 1]);
    }

    #[tokio::test]
    async fn heal_targets_v1_skipped_ok() {
        let mut leaves = leaves(3).await;
        leaves[1].block_header_mut().payload_commitment = VidCommitment::V1(Default::default());
        let targets = heal_targets(
            &leaves,
            &BTreeSet::new(),
            ViewNumber::new(10),
            1000,
            member,
            &cfg(),
        );
        assert_eq!(views(&targets), vec![3, 1]);
    }

    #[tokio::test]
    async fn heal_target_unparsable_ns_table_costs_everything_ok() {
        // The test metadata encodes to no bytes, which is not a namespace table.
        let leaves = leaves(3).await;
        let targets = heal_targets(
            &leaves,
            &BTreeSet::new(),
            ViewNumber::new(10),
            1000,
            member,
            &cfg(),
        );
        assert!(targets.iter().all(|t| t.cost() == usize::MAX));
    }
}
