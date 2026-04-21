//! V2 API module - proto-based rewards API
//!
//! Serves proto-generated types at `/v2/rewards/*` with OpenAPI documentation.

pub mod rewards;

pub use rewards::RewardApi;
