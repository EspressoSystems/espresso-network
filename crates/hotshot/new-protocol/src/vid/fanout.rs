//! Leader-side VID dispersal: erasure-code a block once and fan the shares out.
//!
//! The block builder calls [`encode`] to Reed-Solomon-encode every namespace a
//! single time, deriving the payload commitment from that same computation, and
//! then hands the shares to [`fan_out`] (on a background task) which coalesces
//! namespaces into size-balanced buckets and unicasts every node — including the
//! leader itself, via loopback — a stream of [`AvidmGf2DisperseShareFragment`]
//! messages (one per bucket).

use std::{mem, ops::Range, sync::LazyLock};

use hotshot_types::{
    data::{
        EpochNumber, ViewNumber,
        vid_disperse::{
            AvidmGf2DisperseParams, AvidmGf2DisperseShareFragment, AvidmGf2NamespacePiece,
        },
    },
    message::Proposal as SignedProposal,
    traits::{node_implementation::NodeType, signature_key::SignatureKey},
    vid::avidm_gf2::{AvidmGf2Commitment, AvidmGf2Param, AvidmGf2Scheme},
};
use hotshot_utils::anytrace::{self, Wrap};
use rayon::prelude::*;
use tracing::warn;
use vid::avidm_gf2::namespaced::NsDispersal;

use crate::{
    message::{ConsensusMessage, Message, MessageType},
    network::{NetworkError, Sender},
};

static NUM_THREADS: LazyLock<usize> = LazyLock::new(|| rayon::current_num_threads().max(1));

/// Erasure-code every namespace of an already-resolved dispersal once.
///
/// Returns the block's payload commitment (derived from the per-namespace
/// commits, so it cannot disagree with the shares) and the per-namespace
/// dispersals grouped by transmission bucket, ready for [`fan_out`].
pub(crate) fn encode<T: NodeType>(
    params: &AvidmGf2DisperseParams<T>,
) -> Result<(AvidmGf2Commitment, Vec<Vec<NsDispersal>>), FanoutError> {
    // We want buckets to be at least 256 KiB, otherwise the per-message overhead
    // is too large. Larger payloads are divided by the number of available
    // threads to fully utilise rayon.
    let threshold = params.payload.len().div_ceil(*NUM_THREADS).max(256 * 1024);
    let buckets = bucketize(&params.ns_table, threshold);

    // Per-namespace dispersals are independent; run each bucket on rayon.
    let per_bucket: Vec<Vec<NsDispersal>> = buckets
        .par_iter()
        .map(|bucket| {
            bucket
                .iter()
                .map(|&ns_index| {
                    AvidmGf2Scheme::ns_disperse_one(
                        &params.param,
                        &params.weights,
                        &params.payload[params.ns_table[ns_index].clone()],
                        ns_index,
                    )
                    .wrap()
                    .map_err(FanoutError::Vid)
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Buckets partition the namespace indices contiguously in order, so flattening
    // yields the commits in namespace-table order — the order `aggregate_commit`
    // expects to match `vid_commitment`.
    let ns_commits: Vec<_> = per_bucket.iter().flatten().map(|d| d.commit).collect();
    let commitment = AvidmGf2Scheme::aggregate_commit(&ns_commits)
        .wrap()
        .map_err(FanoutError::Vid)?;

    Ok((commitment, per_bucket))
}

/// Fan the encoded shares out to every recipient.
///
/// Assembles one [`AvidmGf2DisperseShareFragment`] per (bucket, recipient) and
/// unicasts it. The recipient list includes the leader itself; the loopback send
/// is how the leader obtains its own share to vote. Runs the per-bucket assembly
/// on rayon, overlapping serialization with the network sends of other buckets.
#[allow(clippy::too_many_arguments)]
pub(crate) fn fan_out<T: NodeType>(
    per_bucket: Vec<Vec<NsDispersal>>,
    payload_commitment: AvidmGf2Commitment,
    param: AvidmGf2Param,
    recipients: Vec<T::SignatureKey>,
    num_namespaces: usize,
    view: ViewNumber,
    epoch: EpochNumber,
    network: Sender<T>,
    public_key: T::SignatureKey,
    private_key: <T::SignatureKey as SignatureKey>::PrivateKey,
) -> Result<(), FanoutError> {
    let signature = T::SignatureKey::sign(&private_key, payload_commitment.as_ref())
        .map_err(|err| FanoutError::Sign(err.into()))?;

    per_bucket.into_par_iter().try_for_each_with(
        network,
        |network, bucket| -> Result<(), FanoutError> {
            let mut pieces = vec![Vec::new(); recipients.len()];

            for dispersal in bucket {
                let ns_index = dispersal.ns_index;
                let ns_payload_byte_len = dispersal.payload_byte_len;
                let ns_commit = dispersal.commit;
                for (pieces, ns_share) in pieces.iter_mut().zip(dispersal.shares) {
                    pieces.push(AvidmGf2NamespacePiece {
                        ns_index,
                        ns_payload_byte_len,
                        ns_commit,
                        ns_share,
                    });
                }
            }

            for (recipient, pieces) in recipients.iter().zip(pieces) {
                let fragment = AvidmGf2DisperseShareFragment {
                    view_number: view,
                    epoch: Some(epoch),
                    target_epoch: Some(epoch),
                    payload_commitment,
                    recipient_key: recipient.clone(),
                    param: param.clone(),
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
pub enum FanoutError {
    #[error("network error: {0}")]
    Net(#[from] NetworkError),

    #[error("vid error: {0}")]
    Vid(#[source] anytrace::Error),

    #[error("sign error: {0}")]
    Sign(#[source] Box<dyn std::error::Error + Send + Sync>),
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
