use std::{
    collections::BTreeMap,
    mem,
    ops::Range,
    sync::{Arc, LazyLock},
};

use hotshot_types::{
    PeerConfig,
    data::{
        EpochNumber, VidCommitment2, VidDisperse2, ViewNumber,
        vid_disperse::{
            AvidmGf2DisperseParams, AvidmGf2DisperseShareFragment, AvidmGf2NamespacePiece,
        },
    },
    epoch_membership::EpochMembershipCoordinator,
    message::Proposal as SignedProposal,
    traits::{
        BlockPayload, metrics::Histogram, node_implementation::NodeType,
        signature_key::SignatureKey,
    },
    vid::avidm_gf2::{AvidmGf2NsDispersal, AvidmGf2Scheme},
};
use hotshot_utils::anytrace::{self, Wrap};
use rayon::prelude::*;
use tokio::task::{AbortHandle, JoinSet};
use tracing::{debug_span, error, warn};

use crate::{
    coordinator::metrics::{Measurement, finish_measurement},
    message::{ConsensusMessage, Message, MessageType, Validated},
    network::{NetworkError, Sender},
};

static NUM_THREADS: LazyLock<usize> = LazyLock::new(|| rayon::current_num_threads().max(1));

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct VidDisperseRequest<T: NodeType> {
    pub view: ViewNumber,
    pub epoch: EpochNumber,
    pub block: T::BlockPayload,
    pub metadata: <T::BlockPayload as BlockPayload<T>>::Metadata,
    pub payload_commitment: VidCommitment2,
}

/// A block's dispersal, computed by the block builder together with its commitment, so the
/// disperser only signs and sends it.
pub struct BuiltDispersal<T: NodeType> {
    params: AvidmGf2DisperseParams<T>,
    /// One per namespace, in namespace-table order.
    dispersals: Vec<AvidmGf2NsDispersal>,
}

impl<T: NodeType> BuiltDispersal<T> {
    /// Erasure-codes every namespace of an encoded payload for the nodes of `stake_table`.
    pub fn new(
        stake_table: &[PeerConfig<T>],
        payload: Arc<[u8]>,
        metadata: &[u8],
    ) -> Result<Self, VidDisperseError> {
        let params = AvidmGf2DisperseParams::new(stake_table, payload, metadata)
            .map_err(VidDisperseError::Vid)?;
        let dispersals = (0..params.ns_table.len())
            .into_par_iter()
            .map(|ns_index| disperse_namespace(&params, ns_index))
            .collect::<Result<_, _>>()?;
        Ok(Self { params, dispersals })
    }

    /// The payload commitment, folded from the namespaces' commits as
    /// [`AvidmGf2Scheme::commit`] folds them.
    pub fn payload_commitment(&self) -> Result<VidCommitment2, VidDisperseError> {
        let commits: Vec<_> = self
            .dispersals
            .iter()
            .map(|dispersal| dispersal.commit)
            .collect();
        AvidmGf2Scheme::aggregate_commit(&commits)
            .wrap()
            .map_err(VidDisperseError::Vid)
    }
}

/// A block for [`disperse_payload`] to send.
pub enum Dispersal<T: NodeType> {
    /// Encoded by the block builder, so it is only signed and sent.
    Built(BuiltDispersal<T>),
    /// Encoded bucket by bucket while it is sent.
    Encode(AvidmGf2DisperseParams<T>),
}

pub struct VidDisperseOutput {
    pub view: ViewNumber,
    pub payload_commitment: VidCommitment2,
}

pub struct VidDisperser<T: NodeType> {
    calculations: BTreeMap<(ViewNumber, EpochNumber, VidCommitment2), AbortHandle>,
    epoch_membership_coordinator: EpochMembershipCoordinator<T>,
    network: Sender<T>,
    public_key: T::SignatureKey,
    private_key: <T::SignatureKey as SignatureKey>::PrivateKey,
    tasks: JoinSet<Result<VidDisperseOutput, VidDisperseError>>,
    duration_metric: Option<Arc<dyn Histogram>>,
}

impl<T: NodeType> VidDisperser<T> {
    pub fn new(
        epoch_membership_coordinator: EpochMembershipCoordinator<T>,
        network: Sender<T>,
        public_key: T::SignatureKey,
        private_key: <T::SignatureKey as SignatureKey>::PrivateKey,
    ) -> Self {
        Self {
            calculations: BTreeMap::new(),
            epoch_membership_coordinator,
            network,
            public_key,
            private_key,
            tasks: JoinSet::new(),
            duration_metric: None,
        }
    }

    pub fn with_metrics(mut self, hist: Option<Arc<dyn Histogram>>) -> Self {
        self.duration_metric = hist;
        self
    }

    /// Disperses a block: sends `built` if the block builder computed it, else encodes the
    /// payload first. The coordinator always hands over its block builder's dispersal; only the
    /// bench node, which skips the block builder, and tests that feed `BlockBuilt` directly pass
    /// `None`. A repeated request is dropped, `built` with it.
    pub fn request_vid_disperse(
        &mut self,
        vid_disperse_request: VidDisperseRequest<T>,
        built: Option<BuiltDispersal<T>>,
    ) {
        let key = (
            vid_disperse_request.view,
            vid_disperse_request.epoch,
            vid_disperse_request.payload_commitment,
        );
        if self.calculations.contains_key(&key) {
            return;
        }
        let membership = self.epoch_membership_coordinator.clone();
        let network = self.network.clone();
        let public_key = self.public_key.clone();
        let private_key = self.private_key.clone();
        let duration_metric = self.duration_metric.clone();
        let handle = self.tasks.spawn_blocking(move || {
            let measurement = duration_metric.map(Measurement::start);
            let result = handle_vid_disperse_request(
                membership,
                network,
                public_key,
                private_key,
                vid_disperse_request,
                built,
            );
            finish_measurement(measurement);
            result
        });
        self.calculations.insert(key, handle);
    }

    pub async fn next(&mut self) -> Option<Result<VidDisperseOutput, VidDisperseError>> {
        loop {
            match self.tasks.join_next().await {
                Some(Ok(result)) => return Some(result),
                Some(Err(err)) => {
                    if err.is_panic() {
                        error!(%err, "vid disperse task panic");
                    }
                    continue;
                },
                None => return None,
            }
        }
    }

    pub fn gc(&mut self, view_number: ViewNumber) {
        let keep = self.calculations.split_off(&(
            view_number,
            EpochNumber::new(0),
            VidCommitment2::default(),
        ));
        for handle in self.calculations.values_mut() {
            handle.abort();
        }
        self.calculations = keep;
    }
}

fn handle_vid_disperse_request<T: NodeType>(
    epoch_membership_coordinator: EpochMembershipCoordinator<T>,
    network: Sender<T>,
    public_key: T::SignatureKey,
    private_key: <T::SignatureKey as SignatureKey>::PrivateKey,
    vid_disperse_request: VidDisperseRequest<T>,
    built: Option<BuiltDispersal<T>>,
) -> Result<VidDisperseOutput, VidDisperseError> {
    let view = vid_disperse_request.view;
    let epoch = vid_disperse_request.epoch;
    let payload_commitment = vid_disperse_request.payload_commitment;

    let dispersal = match built {
        Some(built) => Dispersal::Built(built),
        None => Dispersal::Encode(
            VidDisperse2::<T>::disperse_params(
                &vid_disperse_request.block,
                &epoch_membership_coordinator,
                Some(epoch),
                &vid_disperse_request.metadata,
            )
            .map_err(VidDisperseError::Vid)?,
        ),
    };

    disperse_payload(
        dispersal,
        view,
        epoch,
        payload_commitment,
        &public_key,
        &private_key,
        move |recipient, message| network.unicast(view, recipient, message),
    )?;

    Ok(VidDisperseOutput {
        view,
        payload_commitment,
    })
}

/// Signs a block's dispersal and hands every recipient's fragment of each bucket of namespaces to
/// `send`, which the disperser points at the network. A [`Dispersal::Encode`] is erasure-coded
/// bucket by bucket first. The `block_build` bench in espresso-types passes a `send` that only
/// serializes; the spans time each step.
pub fn disperse_payload<T, S>(
    dispersal: Dispersal<T>,
    view: ViewNumber,
    epoch: EpochNumber,
    payload_commitment: VidCommitment2,
    public_key: &T::SignatureKey,
    private_key: &<T::SignatureKey as SignatureKey>::PrivateKey,
    send: S,
) -> Result<(), VidDisperseError>
where
    T: NodeType,
    S: Fn(&T::SignatureKey, &Message<T, Validated>) -> Result<(), NetworkError> + Clone + Send,
{
    let (params, dispersals) = match dispersal {
        Dispersal::Built(BuiltDispersal { params, dispersals }) => (params, Some(dispersals)),
        Dispersal::Encode(params) => (params, None),
    };

    let signature = debug_span!("sign")
        .in_scope(|| T::SignatureKey::sign(private_key, payload_commitment.as_ref()))
        .map_err(|err| VidDisperseError::Sign(err.into()))?;

    // Coalesce namespaces into size-balanced buckets, roughly one per core,
    // but never below the minimum size, so a block of many tiny namespaces
    // goes out as a few balanced messages per recipient instead of one tiny
    // message each. Each bucket is one parallel unit and one message per
    // recipient.
    let ns_buckets: Vec<Vec<usize>> = {
        // We want buckets to be at least 256 KiB, otherwise the per-message
        // overhead is too large. Larger payloads are divided by the number
        // of available threads to fully utilise rayon.
        let threshold = params.payload.len().div_ceil(*NUM_THREADS).max(256 * 1024);
        bucketize(&params.ns_table, threshold)
    };
    let buckets: Vec<Bucket> = match dispersals {
        Some(dispersals) => into_buckets(dispersals, &ns_buckets)
            .into_iter()
            .map(Bucket::Built)
            .collect(),
        None => ns_buckets.into_iter().map(Bucket::Encode).collect(),
    };

    buckets.into_par_iter().try_for_each_with(
        send,
        |send, bucket| -> Result<(), VidDisperseError> {
            let dispersals = bucket.dispersals(&params)?;
            let bucket_fragments = debug_span!("fragments")
                .in_scope(|| fragments(view, epoch, payload_commitment, &params, dispersals));
            for fragment in bucket_fragments {
                let recipient = fragment.recipient_key.clone();
                let message = Message {
                    sender: public_key.clone(),
                    message_type: MessageType::Consensus(ConsensusMessage::VidShareFragment(
                        SignedProposal::new(fragment, signature.clone()),
                    )),
                };
                if let Err(err) = send(&recipient, &message) {
                    if err.is_critical() {
                        return Err(err.into());
                    }
                    warn!(%err, "network error while sending vid share fragment");
                }
            }
            Ok(())
        },
    )
}

/// The namespace dispersals of one bucket, which go out as one fragment per recipient.
enum Bucket {
    /// Computed by the block builder.
    Built(Vec<AvidmGf2NsDispersal>),
    /// Still to encode, for these namespace indices.
    Encode(Vec<usize>),
}

impl Bucket {
    fn dispersals<T: NodeType>(
        self,
        params: &AvidmGf2DisperseParams<T>,
    ) -> Result<Vec<AvidmGf2NsDispersal>, VidDisperseError> {
        match self {
            Self::Built(dispersals) => Ok(dispersals),
            Self::Encode(ns_indices) => debug_span!("ns_encode").in_scope(|| {
                ns_indices
                    .into_iter()
                    .map(|ns_index| disperse_namespace(params, ns_index))
                    .collect()
            }),
        }
    }
}

/// Moves each namespace's dispersal into its bucket. `dispersals` holds one per namespace, in
/// namespace-table order, and `ns_buckets` partitions the namespace indices.
fn into_buckets(
    dispersals: Vec<AvidmGf2NsDispersal>,
    ns_buckets: &[Vec<usize>],
) -> Vec<Vec<AvidmGf2NsDispersal>> {
    let mut dispersals: Vec<_> = dispersals.into_iter().map(Some).collect();
    ns_buckets
        .iter()
        .map(|bucket| {
            bucket
                .iter()
                .map(|&ns_index| {
                    dispersals[ns_index]
                        .take()
                        .expect("buckets partition the namespaces")
                })
                .collect()
        })
        .collect()
}

/// Erasure-codes the namespace at `ns_index` of the payload in `params`.
fn disperse_namespace<T: NodeType>(
    params: &AvidmGf2DisperseParams<T>,
    ns_index: usize,
) -> Result<AvidmGf2NsDispersal, VidDisperseError> {
    AvidmGf2Scheme::ns_disperse_one(
        &params.param,
        &params.weights,
        &params.payload[params.ns_table[ns_index].clone()],
        ns_index,
    )
    .wrap()
    .map_err(VidDisperseError::Vid)
}

/// One fragment per recipient in `params`, carrying its shares of `dispersals`.
fn fragments<T: NodeType>(
    view: ViewNumber,
    epoch: EpochNumber,
    payload_commitment: VidCommitment2,
    params: &AvidmGf2DisperseParams<T>,
    dispersals: Vec<AvidmGf2NsDispersal>,
) -> Vec<AvidmGf2DisperseShareFragment<T>> {
    let mut pieces = vec![Vec::new(); params.recipients.len()];
    for dispersal in dispersals {
        for (pieces, ns_share) in pieces.iter_mut().zip(dispersal.shares) {
            pieces.push(AvidmGf2NamespacePiece {
                ns_index: dispersal.ns_index,
                ns_payload_byte_len: dispersal.payload_byte_len,
                ns_commit: dispersal.commit,
                ns_share,
            });
        }
    }
    params
        .recipients
        .iter()
        .zip(pieces)
        .map(|(recipient, pieces)| AvidmGf2DisperseShareFragment {
            view_number: view,
            epoch: Some(epoch),
            target_epoch: Some(epoch),
            payload_commitment,
            recipient_key: recipient.clone(),
            param: params.param.clone(),
            num_namespaces: params.ns_table.len(),
            namespaces: pieces,
        })
        .collect()
}

/// Group namespace indices into buckets whose payload sizes are each at least
/// `threshold` bytes (except possibly the last), coalescing small namespaces so
/// the disperser sends one message per bucket rather than one per namespace.
///
/// A `threshold` of 0 puts every namespace in its own bucket (no coalescing); a
/// threshold larger than the whole payload yields a single bucket. A namespace
/// at least `threshold` bytes on its own seals its bucket immediately, so large
/// namespaces stay separate while small ones accumulate.
fn bucketize(ns_table: &[Range<usize>], threshold: usize) -> Vec<Vec<usize>> {
    let mut buckets = Vec::new();
    let mut current = Vec::new();
    let mut current_bytes = 0usize;
    for (ns_index, range) in ns_table.iter().enumerate() {
        current.push(ns_index);
        current_bytes += range.len();
        if current_bytes >= threshold {
            buckets.push(mem::take(&mut current));
            current_bytes = 0;
        }
    }
    if !current.is_empty() {
        buckets.push(current);
    }
    buckets
}

#[derive(Debug, thiserror::Error)]
pub enum VidDisperseError {
    #[error("network error: {0}")]
    Net(#[from] NetworkError),

    #[error("vid error: {0}")]
    Vid(#[source] anytrace::Error),

    #[error("sign error: {0}")]
    Sign(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl VidDisperseError {
    pub fn is_critical(&self) -> bool {
        match self {
            Self::Net(e) => e.is_critical(),
            Self::Vid(_) | Self::Sign(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{BTreeMap, BTreeSet},
        mem,
        ops::Range,
        sync::{Arc, Mutex},
    };

    use alloy::primitives::U256;
    use hotshot_example_types::{
        node_stake::{TestNodeStakes, gen_node_lists},
        node_types::TestTypes,
    };
    use hotshot_types::{
        PeerConfig,
        data::{
            EpochNumber, VidCommitment, VidCommitment2, ViewNumber, vid_commitment,
            vid_disperse::{
                AvidmGf2DisperseParams, AvidmGf2DisperseShareFragment, vid_total_weight,
            },
        },
        signature_key::BLSPubKey,
        traits::signature_key::{SignatureKey, StakeTableEntryType},
    };
    use quickcheck::{TestResult, quickcheck};
    use versions::NEW_PROTOCOL_VERSION;

    use super::{
        Bucket, BuiltDispersal, Dispersal, VidDisperseError, bucketize, disperse_payload,
        fragments, into_buckets,
    };
    use crate::{
        message::{ConsensusMessage, Message, MessageType, Validated},
        network::NetworkError,
        tests::common::utils::ns_table,
    };

    /// Build a contiguous namespace table from per-namespace byte lengths.
    /// Only the lengths matter to `bucketize`; the offsets are incidental.
    fn ns_ranges(lens: &[u8]) -> Vec<Range<usize>> {
        let mut start = 0;
        lens.iter()
            .map(|&len| {
                let range = start..start + usize::from(len);
                start += usize::from(len);
                range
            })
            .collect()
    }

    /// Total payload bytes of the namespaces at `indices`.
    fn bytes(indices: &[usize], ns_table: &[Range<usize>]) -> usize {
        indices.iter().map(|&i| ns_table[i].len()).sum()
    }

    quickcheck! {
        /// The buckets partition the namespace indices: every index appears
        /// exactly once, contiguously, and in namespace-table order. This pins
        /// completeness, disjointness, and ordering in a single property.
        fn prop_buckets_partition(lens: Vec<u8>, threshold: u16) -> bool {
            let ns_table = ns_ranges(&lens);
            bucketize(&ns_table, threshold.into())
                .into_iter()
                .flatten()
                .eq(0..ns_table.len())
        }

        /// No bucket is ever empty.
        fn prop_buckets_non_empty(lens: Vec<u8>, threshold: u16) -> bool {
            let ns_table = ns_ranges(&lens);
            bucketize(&ns_table, threshold.into())
                .iter()
                .all(|bucket| !bucket.is_empty())
        }

        /// Every bucket except the last reaches the threshold: small namespaces
        /// are coalesced until the bucket is large enough.
        fn prop_non_last_buckets_meet_threshold(lens: Vec<u8>, threshold: u16) -> bool {
            let ns_table = ns_ranges(&lens);
            let buckets = bucketize(&ns_table, threshold.into());
            let non_last = buckets.len().saturating_sub(1);
            buckets
                .iter()
                .take(non_last)
                .all(|bucket| bytes(bucket, &ns_table) >= usize::from(threshold))
        }

        /// Buckets are minimal: dropping a non-last bucket's final namespace
        /// leaves it below the threshold, so the disperser seals as soon as it
        /// reaches the cutoff rather than over-coalescing. (Vacuous at
        /// `threshold == 0`, where every namespace is its own bucket.)
        fn prop_non_last_buckets_are_minimal(lens: Vec<u8>, threshold: u16) -> TestResult {
            if threshold == 0 {
                return TestResult::discard();
            }
            let ns_table = ns_ranges(&lens);
            let buckets = bucketize(&ns_table, threshold.into());
            let non_last = buckets.len().saturating_sub(1);
            TestResult::from_bool(buckets.iter().take(non_last).all(|bucket| {
                bytes(&bucket[..bucket.len() - 1], &ns_table) < usize::from(threshold)
            }))
        }
    }

    /// Nodes with unequal stakes, so their VID weights differ.
    fn stake_table() -> Vec<PeerConfig<TestTypes>> {
        let stakes = (0..5).fold(TestNodeStakes::default(), |stakes, i| {
            stakes.with_stake(i, U256::from(i + 1))
        });
        gen_node_lists(5, 0, &stakes).0
    }

    /// A payload with three namespaces of different sizes.
    fn payload() -> (Vec<u8>, Vec<u8>) {
        let payload = (0..6000).map(|i| (i % 251) as u8).collect();
        (payload, ns_table(&[1000, 3000, 6000]))
    }

    #[test]
    fn built_dispersal_commits_like_vid_commitment() {
        let stake_table = stake_table();
        // How the builder weighted the commitment before it took it from the dispersal.
        let total_weight = vid_total_weight(stake_table.iter(), Some(EpochNumber::genesis()));
        for ((payload, metadata), num_namespaces) in [(payload(), 3), ((vec![], ns_table(&[])), 1)]
        {
            let built =
                BuiltDispersal::new(&stake_table, payload.clone().into(), &metadata).unwrap();
            assert_eq!(built.dispersals.len(), num_namespaces);
            assert_eq!(
                VidCommitment::V2(built.payload_commitment().unwrap()),
                vid_commitment(&payload, &metadata, total_weight, NEW_PROTOCOL_VERSION),
            );
        }
    }

    #[test]
    fn built_fragments_equal_encoded_fragments() {
        let (payload, metadata) = payload();
        let built = BuiltDispersal::new(&stake_table(), payload.into(), &metadata).unwrap();
        let commitment = built.payload_commitment().unwrap();
        // The first two namespaces share a bucket.
        let ns_buckets = bucketize(&built.params.ns_table, 2500);
        assert_eq!(ns_buckets, vec![vec![0, 1], vec![2]]);
        let BuiltDispersal { params, dispersals } = built;
        let fragments = |dispersals| {
            fragments(
                ViewNumber::new(7),
                EpochNumber::new(3),
                commitment,
                &params,
                dispersals,
            )
        };
        for (ns_bucket, built) in ns_buckets.iter().zip(into_buckets(dispersals, &ns_buckets)) {
            let encoded = Bucket::Encode(ns_bucket.clone())
                .dispersals(&params)
                .unwrap();
            assert_eq!(fragments(built), fragments(encoded));
        }
    }

    /// A payload with three namespaces of 256 KiB, so each fills a bucket of its own on a machine
    /// with at least three cores, and fewer cores coalesce them.
    fn large_payload() -> (Vec<u8>, Vec<u8>) {
        const NS: u32 = 256 * 1024;
        let payload = (0..3 * NS).map(|i| (i % 251) as u8).collect();
        (payload, ns_table(&[NS, 2 * NS, 3 * NS]))
    }

    /// The built dispersal of `payload` to [`stake_table`], with its commitment.
    fn built_dispersal(
        (payload, metadata): (Vec<u8>, Vec<u8>),
    ) -> (BuiltDispersal<TestTypes>, VidCommitment2) {
        let built = BuiltDispersal::new(&stake_table(), payload.into(), &metadata).unwrap();
        let commitment = built.payload_commitment().unwrap();
        (built, commitment)
    }

    /// Runs [`disperse_payload`] with a `send` that checks each message is the leader's signed
    /// fragment for its recipient, records the fragment, and returns `reply()`.
    fn disperse_recording<R>(
        dispersal: Dispersal<TestTypes>,
        commitment: VidCommitment2,
        reply: R,
    ) -> (
        Result<(), VidDisperseError>,
        Vec<AvidmGf2DisperseShareFragment<TestTypes>>,
    )
    where
        R: Fn() -> Result<(), NetworkError> + Clone + Send,
    {
        let (leader, private_key) = BLSPubKey::generated_from_seed_indexed([1; 32], 0);
        let sent = Arc::new(Mutex::new(Vec::new()));
        let send = {
            let (leader, sent) = (leader, Arc::clone(&sent));
            move |recipient: &BLSPubKey, message: &Message<TestTypes, Validated>| {
                let MessageType::Consensus(ConsensusMessage::VidShareFragment(fragment)) =
                    &message.message_type
                else {
                    panic!("the disperser sends only VID share fragments");
                };
                assert_eq!(message.sender, leader);
                assert!(leader.validate(&fragment.signature, commitment.as_ref()));
                assert_eq!(&fragment.data.recipient_key, recipient);
                sent.lock().unwrap().push(fragment.data.clone());
                reply()
            }
        };
        let result = disperse_payload(
            dispersal,
            ViewNumber::new(7),
            EpochNumber::new(3),
            commitment,
            &leader,
            &private_key,
            send,
        );
        let sent = mem::take(&mut *sent.lock().unwrap());
        (result, sent)
    }

    /// Each recipient's fragments, ordered by their first namespace.
    fn by_recipient(
        sent: Vec<AvidmGf2DisperseShareFragment<TestTypes>>,
    ) -> BTreeMap<BLSPubKey, Vec<AvidmGf2DisperseShareFragment<TestTypes>>> {
        let mut by_recipient: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for fragment in sent {
            by_recipient
                .entry(fragment.recipient_key)
                .or_default()
                .push(fragment);
        }
        for fragments in by_recipient.values_mut() {
            fragments.sort_by_key(|fragment| fragment.namespaces[0].ns_index);
        }
        by_recipient
    }

    /// The namespace indices each fragment carries: one list per bucket.
    fn buckets(fragments: &[AvidmGf2DisperseShareFragment<TestTypes>]) -> Vec<Vec<usize>> {
        fragments
            .iter()
            .map(|fragment| {
                fragment
                    .namespaces
                    .iter()
                    .map(|piece| piece.ns_index)
                    .collect()
            })
            .collect()
    }

    #[test]
    fn disperse_payload_sends_one_fragment_per_recipient_and_bucket() {
        let (payload, metadata) = large_payload();
        let params =
            AvidmGf2DisperseParams::new(&stake_table(), payload.clone().into(), &metadata).unwrap();
        let (built, commitment) = built_dispersal((payload, metadata));

        let (result, sent) = disperse_recording(Dispersal::Built(built), commitment, || Ok(()));
        result.unwrap();
        let sent = by_recipient(sent);
        let (result, encoded) =
            disperse_recording(Dispersal::Encode(params), commitment, || Ok(()));
        result.unwrap();
        assert_eq!(
            sent,
            by_recipient(encoded),
            "a built dispersal sends what encoding while sending does"
        );

        let recipients: BTreeSet<_> = stake_table()
            .iter()
            .map(|peer| peer.stake_table_entry.public_key())
            .collect();
        assert_eq!(sent.keys().cloned().collect::<BTreeSet<_>>(), recipients);
        let first = buckets(sent.values().next().unwrap());
        assert!(
            first.concat().into_iter().eq(0..3),
            "the buckets hold every namespace once"
        );
        for fragments in sent.values() {
            assert_eq!(
                buckets(fragments),
                first,
                "every recipient gets each bucket once"
            );
            assert!(
                fragments
                    .iter()
                    .all(|fragment| fragment.num_namespaces == 3)
            );
        }
    }

    /// The small payload fits one bucket, so nothing else is sent after the first error.
    #[test]
    fn disperse_payload_stops_at_a_critical_network_error() {
        let (built, commitment) = built_dispersal(payload());
        let (result, sent) = disperse_recording(Dispersal::Built(built), commitment, || {
            Err(NetworkError::Cliquenet(
                cliquenet::NetworkError::ChannelClosed,
            ))
        });
        assert!(matches!(result, Err(VidDisperseError::Net(err)) if err.is_critical()));
        assert_eq!(sent.len(), 1);
    }

    #[test]
    fn disperse_payload_goes_on_after_a_non_critical_network_error() {
        let (built, commitment) = built_dispersal(payload());
        let (result, sent) = disperse_recording(Dispersal::Built(built), commitment, || {
            Err(NetworkError::Cliquenet(cliquenet::NetworkError::Timeout))
        });
        result.unwrap();
        assert_eq!(
            sent.len(),
            stake_table().len(),
            "every recipient is still sent to"
        );
    }
}
