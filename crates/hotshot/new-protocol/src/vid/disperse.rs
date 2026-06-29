use std::{collections::BTreeMap, mem, ops::Range, sync::LazyLock};

use hotshot::traits::BlockPayload;
use hotshot_types::{
    data::{
        EpochNumber, VidCommitment2, VidDisperse2, ViewNumber,
        vid_disperse::{AvidmGf2DisperseShareFragment, AvidmGf2NamespacePiece},
    },
    epoch_membership::EpochMembershipCoordinator,
    message::Proposal as SignedProposal,
    traits::{node_implementation::NodeType, signature_key::SignatureKey},
    vid::avidm_gf2::AvidmGf2Scheme,
};
use hotshot_utils::anytrace::{self, Wrap};
use rayon::prelude::*;
use tokio::task::{AbortHandle, JoinSet};
use tracing::{error, warn};

use crate::{
    message::{ConsensusMessage, Message, MessageType},
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

pub struct VidDisperseOutput {
    pub view: ViewNumber,
    pub payload_commitment: VidCommitment2,
}

pub struct VidDisperser<T: NodeType> {
    calculations: BTreeMap<(ViewNumber, VidCommitment2), AbortHandle>,
    epoch_membership_coordinator: EpochMembershipCoordinator<T>,
    network: Sender<T>,
    public_key: T::SignatureKey,
    private_key: <T::SignatureKey as SignatureKey>::PrivateKey,
    tasks: JoinSet<Result<VidDisperseOutput, VidDisperseError>>,
    /// Optional leader-event tracer (wired by the bench).  Threaded through to
    /// each dispersal task so the share-sign and parallel fan-out window emit
    /// their `ShareSignLoop*` / `VidSharesQueued` / `VidSharesUnicast*` /
    /// `NsDisperseEnd` trace events from inside the disperser, where the work
    /// actually happens.
    tracer: Option<crate::leader_trace::LeaderTracerHandle>,
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
            tracer: None,
        }
    }

    /// Register a leader-event tracer.  Production builds leave this `None`.
    pub fn set_tracer(&mut self, tracer: Option<crate::leader_trace::LeaderTracerHandle>) {
        self.tracer = tracer;
    }

    pub fn request_vid_disperse(&mut self, vid_disperse_request: VidDisperseRequest<T>) {
        let key = (
            vid_disperse_request.view,
            vid_disperse_request.payload_commitment,
        );
        if self.calculations.contains_key(&key) {
            return;
        }
        let membership = self.epoch_membership_coordinator.clone();
        let network = self.network.clone();
        let public_key = self.public_key.clone();
        let private_key = self.private_key.clone();
        let tracer = self.tracer.clone();
        let handle = self.tasks.spawn_blocking(move || {
            handle_vid_disperse_request(
                membership,
                network,
                public_key,
                private_key,
                vid_disperse_request,
                tracer,
            )
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
        let keep = self
            .calculations
            .split_off(&(view_number, VidCommitment2::default()));
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
    tracer: Option<crate::leader_trace::LeaderTracerHandle>,
) -> Result<VidDisperseOutput, VidDisperseError> {
    let view = vid_disperse_request.view;
    let epoch = vid_disperse_request.epoch;
    let payload_commitment = vid_disperse_request.payload_commitment;

    let params = VidDisperse2::<T>::disperse_params(
        &vid_disperse_request.block,
        &epoch_membership_coordinator,
        Some(epoch),
        &vid_disperse_request.metadata,
    )
    .map_err(VidDisperseError::Vid)?;

    // ShareSignLoop brackets the per-share signing path.
    crate::trace_leader_event!(
        tracer,
        view,
        crate::leader_trace::LeaderEvent::ShareSignLoopStart
    );
    let signature = T::SignatureKey::sign(&private_key, payload_commitment.as_ref())
        .map_err(|err| VidDisperseError::Sign(err.into()))?;
    crate::trace_leader_event!(
        tracer,
        view,
        crate::leader_trace::LeaderEvent::ShareSignLoopEnd
    );

    let num_namespaces = params.ns_table.len();

    // Coalesce namespaces into size-balanced buckets, roughly one per core,
    // but never below the minimum size, so a block of many tiny namespaces
    // goes out as a few balanced messages per recipient instead of one tiny
    // message each. Each bucket is one parallel unit and one message per
    // recipient.
    let buckets: Vec<Vec<usize>> = {
        // We want buckets to be at least 256 KiB, otherwise the per-message
        // overhead is too large. Larger payloads are divided by the number
        // of available threads to fully utilise rayon.
        let threshold = params.payload.len().div_ceil(*NUM_THREADS).max(256 * 1024);
        bucketize(&params.ns_table, threshold)
    };

    // The signed shares are about to be encoded and handed to the network.
    // In this fused design each bucket erasure-codes its namespaces and
    // unicasts the resulting fragments in the same parallel unit, so the
    // unicast window spans the whole fan-out (encode + send).
    crate::trace_leader_event!(
        tracer,
        view,
        crate::leader_trace::LeaderEvent::VidSharesQueued
    );
    crate::trace_leader_event!(
        tracer,
        view,
        crate::leader_trace::LeaderEvent::VidSharesUnicastStart
    );

    buckets.par_iter().try_for_each_with(
        network,
        |network, bucket| -> Result<(), VidDisperseError> {
            let mut pieces = vec![Vec::new(); params.recipients.len()];

            for &ns_index in bucket {
                let dispersal = AvidmGf2Scheme::ns_disperse_one(
                    &params.param,
                    &params.weights,
                    &params.payload[params.ns_table[ns_index].clone()],
                    ns_index,
                )
                .wrap()
                .map_err(VidDisperseError::Vid)?;

                let ns_payload_byte_len = dispersal.payload_byte_len;
                let ns_commit = dispersal.commit;
                for (pieces, ns_share) in pieces.iter_mut().zip(dispersal.shares) {
                    pieces.push(AvidmGf2NamespacePiece {
                        ns_index: dispersal.ns_index,
                        ns_payload_byte_len,
                        ns_commit,
                        ns_share,
                    });
                }
            }

            for (recipient, pieces) in params.recipients.iter().zip(pieces) {
                let fragment = AvidmGf2DisperseShareFragment {
                    view_number: view,
                    epoch: Some(epoch),
                    target_epoch: Some(epoch),
                    payload_commitment,
                    recipient_key: recipient.clone(),
                    param: params.param.clone(),
                    num_namespaces,
                    namespaces: pieces,
                };
                let message = Message {
                    sender: public_key.clone(),
                    message_type: MessageType::Consensus(ConsensusMessage::VidShareFragment(
                        SignedProposal::new(fragment, signature.clone()),
                    )),
                };
                if let Err(err) = network.unicast(view, recipient, &message) {
                    if err.is_critical() {
                        return Err(err.into());
                    }
                    warn!(%err, "network error while sending vid share fragment");
                }
            }
            Ok(())
        },
    )?;
    crate::trace_leader_event!(
        tracer,
        view,
        crate::leader_trace::LeaderEvent::VidSharesUnicastEnd
    );
    // All namespaces have been dispersed (encoded) and sent.
    crate::trace_leader_event!(
        tracer,
        view,
        crate::leader_trace::LeaderEvent::NsDisperseEnd
    );

    Ok(VidDisperseOutput {
        view,
        payload_commitment,
    })
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
    use std::ops::Range;

    use quickcheck::{TestResult, quickcheck};

    use super::bucketize;

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
}
