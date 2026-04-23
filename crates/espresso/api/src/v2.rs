//! V2 API module - proto-based APIs
//!
//! Serves proto-generated types with OpenAPI documentation and gRPC support.

pub mod consensus;
pub mod data;
pub mod rewards;

pub use consensus::ConsensusApi;
pub use data::DataApi;
pub use rewards::RewardApi;
