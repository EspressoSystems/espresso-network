//! V1 API module - legacy APIs
//!
//! Serves internal espresso-types for backward compatibility.
//! No OpenAPI documentation.

pub mod availability;
pub mod block_state;
pub mod reward_state_v2;

pub use availability::AvailabilityApi;
pub use block_state::BlockStateApi;
pub use reward_state_v2::RewardApi;
