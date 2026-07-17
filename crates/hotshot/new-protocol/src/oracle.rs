//! Quint oracle instrumentation shim (Quint Studio).
//!
//! A thin wrapper over the Studio-vendored `quint-oracle-client`, gated behind
//! the `oracle` feature so release builds never link it and the call sites stay
//! one-liners. Each function maps to a Quint action of the fast-finality spec
//! (`quint-specs/new-protocol-_fast-finality_.qnt`); logged transitions are
//! scoped to this component so the oracle replays only its events.
//!
//! The spec abstracts a block to an opaque id and the honest chain logs a
//! constant (`BLOCK`); real leaf commitments are not in the spec's `BLOCKS`
//! domain, and equivocation is a simulation-only behavior.

/// Component key (config `eventScope`).
#[cfg(feature = "oracle")]
const SCOPE: &[&str] = &["new-protocol-_fast-finality_"];

/// Canonical opaque block id logged for the honest chain (spec domain `BLOCKS`).
#[cfg(feature = "oracle")]
const BLOCK: u64 = 1;

#[cfg(feature = "oracle")]
pub(crate) use quint_oracle_client::TestGuard;

/// Begin a test trace; hold the returned guard for the whole test scope.
#[cfg(feature = "oracle")]
pub(crate) fn start_test(name: &str) -> TestGuard {
    quint_oracle_client::start_test(name)
}

/// Log a transition carrying a view and the canonical block (propose, vote1,
/// form_qc1, vote2_and_lock, form_qc2, decide).
#[cfg(feature = "oracle")]
pub(crate) fn view_block(action: &str, view: u64) {
    quint_oracle_client::scope(SCOPE).log_action(
        action,
        &[
            quint_oracle_client::Arg { name: "view", value: view.into(), domain: Some("VIEWS") },
            quint_oracle_client::Arg { name: "block", value: BLOCK.into(), domain: Some("BLOCKS") },
        ],
    );
}

/// Log a transition carrying only a view (timeout, form_tc, advance_on_tc).
#[cfg(feature = "oracle")]
pub(crate) fn view(action: &str, view: u64) {
    quint_oracle_client::scope(SCOPE).log_action(
        action,
        &[quint_oracle_client::Arg { name: "view", value: view.into(), domain: Some("VIEWS") }],
    );
}

/// Log a transition with no arguments (apply_cutover_seed, advance_epoch).
#[cfg(feature = "oracle")]
pub(crate) fn bare(action: &str) {
    quint_oracle_client::scope(SCOPE).log_action(action, &[]);
}

// ── No-op shims when the `oracle` feature is off (normal builds/tests). ──

#[cfg(not(feature = "oracle"))]
pub(crate) struct TestGuard;

#[cfg(not(feature = "oracle"))]
pub(crate) fn start_test(_name: &str) -> TestGuard {
    TestGuard
}

#[cfg(not(feature = "oracle"))]
pub(crate) fn view_block(_action: &str, _view: u64) {}

#[cfg(not(feature = "oracle"))]
pub(crate) fn view(_action: &str, _view: u64) {}

#[cfg(not(feature = "oracle"))]
pub(crate) fn bare(_action: &str) {}
