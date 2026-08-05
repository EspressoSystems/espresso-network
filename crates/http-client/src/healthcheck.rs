//! Healthcheck types, re-exported from [`espresso_wire`].
//!
//! The types' wire compatibility (variant order, the `Unavailabale` misspelling) is pinned by
//! tests in `espresso-wire`, next to the one implementation both sides share.

pub use espresso_wire::{AppHealth, HealthCheck, HealthStatus};
