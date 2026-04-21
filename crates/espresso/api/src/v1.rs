//! V1 API module - legacy reward state API
//!
//! Serves internal espresso-types at `/v1/reward-state-v2/*` for backward compatibility.
//! No OpenAPI documentation.

pub mod reward_state_v2;

pub use reward_state_v2::RewardApi;
