//! Leader-side VID dispersal: erasure-code a block once and fan the shares out.
//!
//! The block builder Reed-Solomon-encodes every namespace a single time (via
//! `NsAvidmGf2Scheme::ns_disperse`, which also yields the payload commitment) and
//! then hands the shares to [`fan_out`] (on a background task) which coalesces
//! namespaces into size-balanced buckets and unicasts every node — including the
//! leader itself, via loopback — a stream of [`AvidmGf2DisperseShareFragment`]
//! messages (one per bucket).

use std::{mem, sync::LazyLock};

use hotshot_types::{
    data::{
        EpochNumber, ViewNumber,
        vid_disperse::{AvidmGf2DisperseShareFragment, AvidmGf2NamespacePiece},
    },
    message::Proposal as SignedProposal,
    traits::{node_implementation::NodeType, signature_key::SignatureKey},
    vid::avidm_gf2::{AvidmGf2Commitment, AvidmGf2Common, AvidmGf2Share},
};
use rayon::prelude::*;
use tracing::warn;

use crate::{
    message::{ConsensusMessage, Message, MessageType},
    network::{NetworkError, Sender},
};

static NUM_THREADS: LazyLock<usize> = LazyLock::new(|| rayon::current_num_threads().max(1));

/// Fan the encoded shares out to every recipient.
///
/// Coalesces namespaces into size-balanced buckets and assembles one
/// [`AvidmGf2DisperseShareFragment`] per (bucket, recipient), then unicasts it.
/// The recipient list includes the leader itself; the loopback send is how the
/// leader obtains its own share to vote. Parallel over recipients, overlapping
/// serialization with the network sends of others.
#[allow(clippy::too_many_arguments)]
pub fn fan_out<T: NodeType>(
    shares: Vec<AvidmGf2Share>,
    common: AvidmGf2Common,
    payload_commitment: AvidmGf2Commitment,
    recipients: Vec<T::SignatureKey>,
    view: ViewNumber,
    epoch: EpochNumber,
    network: Sender<T>,
    public_key: T::SignatureKey,
    private_key: <T::SignatureKey as SignatureKey>::PrivateKey,
) -> Result<(), FanoutError> {
    let signature = T::SignatureKey::sign(&private_key, payload_commitment.as_ref())
        .map_err(|err| FanoutError::Sign(err.into()))?;

    let num_namespaces = common.ns_lens.len();
    // We want buckets to be at least 256 KiB, otherwise the per-message overhead
    // is too large. Larger payloads are divided by the number of available
    // threads to fully utilise rayon.
    let threshold = common
        .payload_byte_len()
        .div_ceil(*NUM_THREADS)
        .max(256 * 1024);
    let buckets = bucketize(&common.ns_lens, threshold);

    shares.into_par_iter().zip(recipients).try_for_each_with(
        network,
        |network, (share, recipient)| -> Result<(), FanoutError> {
            // Buckets partition the namespaces contiguously in order, so draining
            // this recipient's shares in lockstep with the buckets pairs each
            // share with its namespace.
            let mut ns_shares = share.into_ns_shares().into_iter();
            for bucket in &buckets {
                let namespaces: Vec<_> = bucket
                    .iter()
                    .zip(ns_shares.by_ref())
                    .map(|(&ns_index, ns_share)| AvidmGf2NamespacePiece {
                        ns_index,
                        ns_payload_byte_len: common.ns_lens[ns_index],
                        ns_commit: common.ns_commits[ns_index],
                        ns_share,
                    })
                    .collect();
                let fragment = AvidmGf2DisperseShareFragment {
                    view_number: view,
                    epoch: Some(epoch),
                    target_epoch: Some(epoch),
                    payload_commitment,
                    recipient_key: recipient.clone(),
                    param: common.param.clone(),
                    num_namespaces,
                    namespaces,
                };
                let message = Message {
                    sender: public_key.clone(),
                    message_type: MessageType::Consensus(ConsensusMessage::VidShareFragment(
                        SignedProposal::new(fragment, signature.clone()),
                    )),
                };
                if let Err(err) = network.unicast(view, &recipient, &message) {
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

/// Group namespace indices into buckets whose payload sizes are each at least
/// `threshold` bytes (except possibly the last), coalescing small namespaces so
/// the disperser sends one message per bucket rather than one per namespace.
///
/// A `threshold` of 0 puts every namespace in its own bucket (no coalescing); a
/// threshold larger than the whole payload yields a single bucket. A namespace
/// at least `threshold` bytes on its own seals its bucket immediately, so large
/// namespaces stay separate while small ones accumulate.
fn bucketize(ns_lens: &[usize], threshold: usize) -> Vec<Vec<usize>> {
    let mut buckets = Vec::new();
    let mut current = Vec::new();
    let mut current_bytes = 0usize;
    for (ns_index, &len) in ns_lens.iter().enumerate() {
        current.push(ns_index);
        current_bytes += len;
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
pub enum FanoutError {
    #[error("network error: {0}")]
    Net(#[from] NetworkError),

    #[error("sign error: {0}")]
    Sign(#[source] Box<dyn std::error::Error + Send + Sync>),
}

#[cfg(test)]
mod tests {
    use quickcheck::{TestResult, quickcheck};

    use super::bucketize;

    /// Per-namespace byte lengths as `usize`.
    fn ns_lens(lens: &[u8]) -> Vec<usize> {
        lens.iter().map(|&len| usize::from(len)).collect()
    }

    /// Total payload bytes of the namespaces at `indices`.
    fn bytes(indices: &[usize], ns_lens: &[usize]) -> usize {
        indices.iter().map(|&i| ns_lens[i]).sum()
    }

    quickcheck! {
        /// The buckets partition the namespace indices: every index appears
        /// exactly once, contiguously, and in namespace-table order. This pins
        /// completeness, disjointness, and ordering in a single property.
        fn prop_buckets_partition(lens: Vec<u8>, threshold: u16) -> bool {
            let ns_lens = ns_lens(&lens);
            bucketize(&ns_lens, threshold.into())
                .into_iter()
                .flatten()
                .eq(0..ns_lens.len())
        }

        /// No bucket is ever empty.
        fn prop_buckets_non_empty(lens: Vec<u8>, threshold: u16) -> bool {
            let ns_lens = ns_lens(&lens);
            bucketize(&ns_lens, threshold.into())
                .iter()
                .all(|bucket| !bucket.is_empty())
        }

        /// Every bucket except the last reaches the threshold: small namespaces
        /// are coalesced until the bucket is large enough.
        fn prop_non_last_buckets_meet_threshold(lens: Vec<u8>, threshold: u16) -> bool {
            let ns_lens = ns_lens(&lens);
            let buckets = bucketize(&ns_lens, threshold.into());
            let non_last = buckets.len().saturating_sub(1);
            buckets
                .iter()
                .take(non_last)
                .all(|bucket| bytes(bucket, &ns_lens) >= usize::from(threshold))
        }

        /// Buckets are minimal: dropping a non-last bucket's final namespace
        /// leaves it below the threshold, so the disperser seals as soon as it
        /// reaches the cutoff rather than over-coalescing. (Vacuous at
        /// `threshold == 0`, where every namespace is its own bucket.)
        fn prop_non_last_buckets_are_minimal(lens: Vec<u8>, threshold: u16) -> TestResult {
            if threshold == 0 {
                return TestResult::discard();
            }
            let ns_lens = ns_lens(&lens);
            let buckets = bucketize(&ns_lens, threshold.into());
            let non_last = buckets.len().saturating_sub(1);
            TestResult::from_bool(buckets.iter().take(non_last).all(|bucket| {
                bytes(&bucket[..bucket.len() - 1], &ns_lens) < usize::from(threshold)
            }))
        }
    }
}
