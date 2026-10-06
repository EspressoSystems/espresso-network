//! Verifiable information dispersal (VID) for the new protocol.
//!
//! A block's payload is erasure-coded per namespace and spread across the
//! committee so the block can be recovered from any subset of storage nodes
//! whose shards cover the recovery threshold. This module owns the three stages
//! of that lifecycle, one per submodule:
//!
//! - [`disperse`] -- the leader side. [`VidDisperser`] erasure-codes each
//!   namespace, coalesces namespaces into size-balanced buckets, and unicasts
//!   to every node a stream of [`AvidmGf2DisperseShareFragment`] messages
//!   (one per bucket), each carrying that node's shares for the bucket's
//!   namespaces.
//!
//! - [`fragments`] -- the receive side of dispersal, the mirror of
//!   [`VidDisperser`]. [`VidFragmentAccumulator`] buffers the fragments a node
//!   receives for its *own* share and, once every namespace has arrived,
//!   reassembles them into a complete [`VidDisperseShare2`]. That share is then
//!   verified, attached to this node's vote, and fed to the reconstructor.
//!
//! - [`reconstruct`] -- block recovery. [`VidReconstructor`] collects the
//!   verified shares contributed by *many* voters (each node's own share,
//!   carried on its vote) and decodes the payload once their shards cover the
//!   recovery threshold.
//!
//! [`AvidmGf2DisperseShareFragment`]: hotshot_types::data::vid_disperse::AvidmGf2DisperseShareFragment
//! [`VidDisperseShare2`]: hotshot_types::data::VidDisperseShare2

mod disperse;
mod fragments;
mod reconstruct;

pub use disperse::{VidDisperseError, VidDisperseOutput, VidDisperseRequest, VidDisperser};
pub use fragments::{VidFragmentAccumulator, VidFragmentError};
use hotshot_types::{
    data::{EpochNumber, ns_table::parse_ns_table, vid_disperse::vid_total_weight},
    epoch_membership::EpochMembershipCoordinator,
    traits::node_implementation::NodeType,
    vid::avidm_gf2::{AvidmGf2Common, AvidmGf2Param, init_avidm_gf2_param},
};
pub(crate) use reconstruct::matches_commitment;
pub use reconstruct::{
    ObtainedPayload, VidReconstructError, VidReconstructErrorKind, VidReconstructor,
};

/// The VID erasure parameters the committee for `epoch` fixes, matching what
/// an honest disperser derives. Used to reject shares whose `common.param` is
/// forged (the commitment binds `ns_commits`, not `param`) and to verify
/// payloads fetched whole. `None` if the committee cannot be resolved.
pub fn expected_vid_param<T: NodeType>(
    membership: &EpochMembershipCoordinator<T>,
    epoch: EpochNumber,
) -> Option<AvidmGf2Param> {
    let membership = membership.stake_table_for_epoch(Some(epoch)).ok()?;
    let total_weight = vid_total_weight::<T, _>(membership.stake_table(), Some(epoch));
    init_avidm_gf2_param(total_weight).ok()
}

/// Whether `common.ns_lens` are the namespace lengths an honest disperser
/// derives from the block's metadata at the payload length the common claims.
///
/// The commitment binds `ns_commits` but not `ns_lens`, so a leader could
/// otherwise pair honest namespace commitments with lengths shuffled between
/// namespaces. Such a block reconstructs and decides, but its stored common
/// then slices the payload at the wrong namespace boundaries and no namespace
/// proof for it can be served.
///
/// The total is not checked here: the application's header validation pins it
/// to the header, and reconstruction recommits the recovered bytes at their
/// true length.
pub(crate) fn ns_lens_match_metadata(common: &AvidmGf2Common, ns_table: &[u8]) -> bool {
    let expected = parse_ns_table(common.payload_byte_len(), ns_table);
    common.ns_lens.len() == expected.len()
        && common
            .ns_lens
            .iter()
            .zip(&expected)
            .all(|(len, range)| *len == range.len())
}
