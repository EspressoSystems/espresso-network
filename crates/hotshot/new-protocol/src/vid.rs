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
    data::{EpochNumber, vid_disperse::vid_total_weight},
    epoch_membership::EpochMembershipCoordinator,
    traits::node_implementation::NodeType,
    vid::avidm_gf2::{AvidmGf2Param, init_avidm_gf2_param},
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
