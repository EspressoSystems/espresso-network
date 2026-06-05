//! V1 API module - legacy APIs
//!
//! Serves internal espresso-types for backward compatibility.
//! No OpenAPI documentation.

pub mod availability;
pub mod reward_state_v2;

pub use availability::{AvailabilityApi, BlockId, HotShotAvailabilityApi, LeafId, PayloadId};
pub use reward_state_v2::RewardApi;
