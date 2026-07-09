//! Verifiable information dispersal (VID) for the new protocol.
//!
//! A block's payload is erasure-coded per namespace and spread across the
//! committee so the block can be recovered from any subset of storage nodes
//! whose shards cover the recovery threshold. This module owns the three stages
//! of that lifecycle, one per submodule:
//!
//! - [`fanout`] -- the leader side. The block builder erasure-codes the block
//!   once (via `NsAvidmGf2Scheme::ns_disperse`, which yields the commitment) and
//!   [`fan_out`] coalesces namespaces into size-balanced buckets and unicasts to
//!   every node — including the leader itself, via loopback — a stream of
//!   [`AvidmGf2DisperseShareFragment`] messages (one per bucket), each carrying
//!   that node's shares for the bucket's namespaces.
//!
//! - [`fragments`] -- the receive side of dispersal, the mirror of
//!   [`fanout`]. [`VidFragmentAccumulator`] buffers the fragments a node
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
//! [`fan_out`]: fanout::fan_out

pub mod fanout;
mod fragments;
mod reconstruct;

pub use fragments::{VidFragmentAccumulator, VidFragmentError};
pub use reconstruct::{
    VidReconstructError, VidReconstructErrorKind, VidReconstructOutput, VidReconstructor,
};
