//! Type definitions used by the HotShot Query Service.
//!
//! Crates which wish to interact with the query service without importing the full server logic
//! (e.g. client crates) may import these types, which are the same types returned by the various
//! query APIs.
//!
//! Note that these types are all re-exported from the `hotshot-query-service` crate itself, so
//! crates which do import the full server do not also need to import this types crate.

use hotshot_types::traits::{BlockPayload, node_implementation::NodeType};

pub mod availability;
pub mod explorer;
pub mod merklized_state;
pub mod node;
pub mod resolvable;
pub mod status;

mod error;
mod height_indexed;

pub use hotshot_types::{data::Leaf2, simple_certificate::QuorumCertificate};

pub use self::{error::*, height_indexed::HeightIndexed, resolvable::Resolvable};

pub type Payload<Types> = <Types as NodeType>::BlockPayload;
pub type Header<Types> = <Types as NodeType>::BlockHeader;
pub type Metadata<Types> = <Payload<Types> as BlockPayload<Types>>::Metadata;
/// Item within a [`Payload`].
pub type Transaction<Types> = <Payload<Types> as BlockPayload<Types>>::Transaction;
pub type SignatureKey<Types> = <Types as NodeType>::SignatureKey;
