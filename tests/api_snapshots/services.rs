//! Probes for the demo services that still run on tide-disco: the orchestrator, the state relay
//! server, the builder, the transaction submitters and the node validator.
//!
//! These are the services the remaining "rm tide" pull requests migrate, so their snapshots are
//! the ones most likely to move. Only routes that are safe to call are probed: a `GET` that
//! reports state is fine, but the orchestrator's and builder's `POST` routes register nodes or
//! claim blocks, and calling them would perturb the network under test.
//!
//! Version prefixes are checked explicitly rather than through `Aliases::Check`, because
//! tide-disco mounts each module under every registered major version rather than aliasing `/v0`
//! onto `/v1`.

use crate::common::api_snapshot::{Probe, Service::*};

const SIGNATURES: &[&str] = &["signatures"];

pub const ORCHESTRATOR: &[Probe] = &[
    Probe::shape(Orchestrator, "api_start", "/api/start").no_aliases(),
    Probe::shape(Orchestrator, "api_peer_pub_ready", "/api/peer_pub_ready").no_aliases(),
    // `/api/builders` is not probed: it 404s ("Not all builders are registered yet") until as many
    // builders have registered as the orchestrator config expects, which the demo never reaches.
    Probe::shape(Orchestrator, "version", "/version").no_aliases(),
];

/// The relay server mounts the same module under `v1`, `v2` and `v3`. Each version is probed so
/// the migration cannot quietly drop one, and the unversioned form must serve the latest.
///
/// `signatures` is keyed by signer, and how many validators have signed the latest state by the
/// time it is read varies, so it is collapsed to the shape of an entry.
pub const STATE_RELAY: &[Probe] = &[
    Probe::shape(StateRelay, "api_lateststate", "/api/lateststate")
        .no_aliases()
        .collapse_fields(SIGNATURES),
    // `/v1/api/lateststate` serves the legacy light client, which a network on version 0.4 does
    // not sign for, so it answers "state signatures are not ready" forever.
    Probe::shape(StateRelay, "v2_api_lateststate", "/v2/api/lateststate")
        .no_aliases()
        .collapse_fields(SIGNATURES),
    Probe::shape(StateRelay, "v3_api_lateststate", "/v3/api/lateststate")
        .no_aliases()
        .collapse_fields(SIGNATURES),
    // `GET /api/state` is the deprecated spelling of `lateststate` and must keep working.
    Probe::shape(StateRelay, "api_state", "/api/state")
        .no_aliases()
        .collapse_fields(SIGNATURES),
    Probe::shape(StateRelay, "version", "/version").no_aliases(),
];

pub const BUILDER: &[Probe] = &[
    Probe::shape(
        Builder,
        "block_info_builderaddress",
        "/block_info/builderaddress",
    )
    .no_aliases(),
    Probe::shape(Builder, "version", "/version").no_aliases(),
];

/// The submitters run a tide-disco app with no modules at all, so `/version` is the whole of their
/// API and the tightest test of the built-in routes an axum port has to keep providing.
pub const SUBMIT_TRANSACTIONS: &[Probe] = &[
    Probe::shape(SubmitPublic, "version", "/version").no_aliases(),
    Probe::shape(SubmitPrivate, "version", "/version").no_aliases(),
];

/// The node validator's only route is the `node-validator/details` socket, which pushes nothing
/// until the network view it aggregates changes, so it is not probed: waiting on it would add
/// minutes to the run for one endpoint. Its `/version` route is still pinned.
///
/// Worth knowing when this service is ported: tide-disco answers the unversioned
/// `/node-validator/details` with a 307 redirect to `/v0/...`, which a WebSocket client cannot
/// follow, whereas the axum server rewrites such paths in place.
pub const NODE_METRICS: &[Probe] = &[Probe::shape(NodeMetrics, "version", "/version").no_aliases()];
