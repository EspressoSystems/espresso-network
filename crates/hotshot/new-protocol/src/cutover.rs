//! Legacy → new-protocol cutover machinery.
//!
//! Two concerns live here:
//! - [`extract_pre_cutover_seed`] walks a live legacy [`SystemContextHandle`]
//!   and produces a [`PreCutoverSeed`].
//! - [`forward_legacy_timeout_votes`] and [`forward_legacy_high_qc`] tail the
//!   legacy event stream and bridge those events into the coordinator's
//!   client API so the new protocol can form TC2s and propose at the
//!   boundary.

use std::collections::BTreeMap;

use async_broadcast::InactiveReceiver;
use futures::StreamExt;
use hotshot::{traits::NodeImplementation, types::SystemContextHandle};
use hotshot_types::{
    data::Leaf2,
    event::{Event, EventType},
    message::UpgradeLock,
    traits::node_implementation::NodeType,
};
use versions::NEW_PROTOCOL_VERSION;

use crate::{client::ClientApi, consensus::PreCutoverSeed};

/// Walk legacy state to produce a [`PreCutoverSeed`]; `None` on
/// a broken walk.
pub async fn extract_pre_cutover_seed<T, I>(
    handle: &SystemContextHandle<T, I>,
) -> Option<PreCutoverSeed<T>>
where
    T: NodeType,
    I: NodeImplementation<T>,
{
    let cutover_view = match handle.hotshot.upgrade_lock.decided_upgrade_cert() {
        Some(cert) => cert.data.new_version_first_view,
        None => {
            tracing::warn!("no decided upgrade certificate; aborting seed extraction");
            return None;
        },
    };

    let consensus_arc = handle.hotshot.consensus();
    let consensus = consensus_arc.read().await;
    let decided_anchor = consensus.decided_leaf();
    let decided_view = decided_anchor.view_number();

    let high_qc = consensus.high_qc().clone();
    let saved = consensus.saved_leaves();

    // `saved_leaves` is canonical — a non-canonical entry would break legacy
    // decide — so we can take every leaf above `decided_view` without
    // re-validating via a `justify_qc` walk.
    let mut undecided: Vec<Leaf2<T>> = saved
        .values()
        .filter(|leaf| leaf.view_number() > decided_view)
        .cloned()
        .collect();
    undecided.sort_by_key(|leaf| leaf.view_number());

    let mut validated_states = BTreeMap::new();
    if let Some(state) = consensus.state(decided_view) {
        validated_states.insert(decided_view, state.clone());
    } else {
        tracing::warn!(%decided_view, "no validated state for decided anchor");
    }
    for leaf in &undecided {
        let view = leaf.view_number();
        if let Some(state) = consensus.state(view) {
            validated_states.insert(view, state.clone());
        } else {
            tracing::warn!(%view, "no validated state for undecided leaf");
        }
    }

    Some(PreCutoverSeed {
        decided_anchor,
        undecided,
        high_qc: Some(high_qc),
        validated_states,
        cutover_view,
    })
}

/// Bridged requests only matter at the `NEW_PROTOCOL_VERSION` cutover;
/// forwarding earlier fills the bounded request queue the parked coordinator
/// can't drain.
fn cutover_decided<T: NodeType>(upgrade_lock: &UpgradeLock<T>) -> bool {
    upgrade_lock
        .decided_upgrade_cert()
        .is_some_and(|cert| cert.data.new_version >= NEW_PROTOCOL_VERSION)
}

/// Forward legacy `TimeoutVote2` events into the new-protocol timeout
/// collectors so the first new leader can form TC2 at the boundary.
pub async fn forward_legacy_timeout_votes<T: NodeType>(
    legacy_event_rx: InactiveReceiver<Event<T>>,
    client_api: ClientApi<T>,
    upgrade_lock: UpgradeLock<T>,
) {
    let mut rx = legacy_event_rx.activate_cloned();
    while let Some(event) = rx.next().await {
        if let EventType::LegacyTimeoutVoteEmitted { vote } = event.event
            && cutover_decided(&upgrade_lock)
            && let Err(err) = client_api.try_submit_legacy_timeout_vote(vote)
        {
            tracing::warn!(%err, "failed to forward legacy TimeoutVote2 to new-protocol coordinator");
        }
    }
}

/// Forward the last legacy view's QC into the coordinator, so the cutover-view
/// leader can propose on it instead of waiting out a timeout when the cutover
/// seed was snapshotted before the QC formed.
pub async fn forward_legacy_high_qc<T: NodeType>(
    legacy_event_rx: InactiveReceiver<Event<T>>,
    client_api: ClientApi<T>,
    upgrade_lock: UpgradeLock<T>,
) {
    let mut rx = legacy_event_rx.activate_cloned();
    while let Some(event) = rx.next().await {
        if let EventType::LegacyHighQcFormed { qc } = event.event
            && cutover_decided(&upgrade_lock)
            && let Err(err) = client_api.try_submit_legacy_high_qc(qc)
        {
            tracing::warn!(%err, "failed to forward legacy high QC to new-protocol coordinator");
        }
    }
}
