//! Healthcheck types, re-exported from [`http_wire`].
//!
//! The types' wire compatibility (variant order, the `Unavailabale` misspelling) is pinned by
//! tests in `http-wire`, next to the one implementation both sides share.

pub use http_wire::{AppHealth, HealthCheck, HealthStatus};
