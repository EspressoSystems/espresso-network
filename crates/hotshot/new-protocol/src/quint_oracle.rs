//! Quint oracle instrumentation for the `new-consensus-protocol` component.
//!
//! Reports [`crate::consensus::Consensus`]'s transitions to the Quint oracle so
//! they can be replayed against `quint-specs/new-consensus-protocol.qnt`. Every
//! function here is a no-op unless the oracle set `QUINT_ORACLE_URL`, so normal
//! builds and test runs are unaffected.

use std::{
    collections::HashMap,
    fmt::Display,
    sync::{Mutex, OnceLock},
};

use quint_oracle_client::{Arg, scope};

/// The Studio component key these events target (the config's `eventScope`).
const COMPONENT: &str = "new-consensus-protocol";

/// Leaf commitment -> branch index, per view. The spec encodes a leaf as
/// `2 * view + branch`, where `branch` only has to distinguish conflicting
/// proposals at one view; assigning it in first-seen order makes the canonical
/// chain branch 0 in every honest run.
static BRANCHES: Mutex<Option<HashMap<(u64, String), u64>>> = Mutex::new(None);

fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var("QUINT_ORACLE_URL").is_ok())
}

/// Open a trace for the running test and clear the branch registry, so leaf ids
/// are stable within one trace and never carry over into the next.
///
/// The returned guard must be held for the test's scope; the test harnesses
/// keep it in their own struct, which is why no individual test needs editing.
/// The name is taken from the thread the test runs on, which the Rust test
/// harness names after the test itself.
#[must_use = "hold the guard for the test's scope; dropping it flushes the trace"]
pub fn start_test() -> quint_oracle_client::TestGuard {
    let name = std::thread::current()
        .name()
        .unwrap_or("unnamed")
        .to_string();
    let guard = quint_oracle_client::start_test(&name);
    if enabled() {
        let mut registry = BRANCHES.lock().unwrap_or_else(|e| e.into_inner());
        *registry = Some(HashMap::new());
    }
    guard
}

/// The spec `Leaf` id for a leaf commitment at `view`.
pub fn leaf_id(view: u64, commitment: &impl Display) -> u64 {
    if !enabled() {
        return 0;
    }
    let mut registry = BRANCHES.lock().unwrap_or_else(|e| e.into_inner());
    let branches = registry.get_or_insert_with(HashMap::new);
    let next_branch = branches.keys().filter(|(v, _)| *v == view).count() as u64;
    *branches
        .entry((view, commitment.to_string()))
        .or_insert(2 * view + next_branch)
}

fn arg(name: &'static str, domain: &'static str, value: u64) -> Arg {
    Arg {
        name,
        value: value.into(),
        domain: Some(domain),
    }
}

fn log(action: &str, args: &[Arg]) {
    scope(&[COMPONENT]).log_action(action, args);
}

fn log_view(action: &str, view: u64) {
    if !enabled() {
        return;
    }
    log(action, &[arg("view", "VIEWS", view)]);
}

/// KNOWN LIMITATION: `parent` is not logged. The spec's `receive_proposal`
/// takes both the leaf and the leaf its justify_qc certifies, but logging both
/// never pinned a replay choice; logging only `leaf` is the configuration in
/// which the most traces replay. See doc/quint-oracle-replay-blocker.md.
fn log_proposal(action: &str, leaf: u64, _parent: u64) {
    if !enabled() {
        return;
    }
    log(action, &[arg("leaf", "LEAVES", leaf)]);
}

fn log_leaf(action: &str, leaf: u64) {
    if !enabled() {
        return;
    }
    log(action, &[arg("leaf", "LEAVES", leaf)]);
}

pub fn receive_proposal(leaf: u64, parent: u64) {
    log_proposal("receive_proposal", leaf, parent);
}

pub fn reject_unsafe_proposal(leaf: u64, parent: u64) {
    log_proposal("reject_unsafe_proposal", leaf, parent);
}

pub fn receive_cert1(leaf: u64) {
    log_leaf("receive_cert1", leaf);
}

pub fn receive_cert2(leaf: u64) {
    log_leaf("receive_cert2", leaf);
}

pub fn state_validated(leaf: u64) {
    log_leaf("state_validated", leaf);
}

pub fn block_reconstructed(view: u64) {
    log_view("block_reconstructed", view);
}

pub fn timeout(view: u64) {
    log_view("timeout", view);
}

/// `view` is the TIMED-OUT view the certificate covers; the node enters
/// `view + 1`.
pub fn timeout_certificate(view: u64) {
    log_view("timeout_certificate", view);
}

pub fn vote_1(view: u64) {
    log_view("vote_1", view);
}

pub fn vote_2_and_update_lock(view: u64) {
    log_view("vote_2_and_update_lock", view);
}

pub fn decide(view: u64) {
    log_view("decide", view);
}

pub fn propose(view: u64) {
    log_view("propose", view);
}

pub fn stored(view: u64) {
    log_view("stored", view);
}

pub fn stored_high_qc(view: u64) {
    log_view("stored_high_qc", view);
}
