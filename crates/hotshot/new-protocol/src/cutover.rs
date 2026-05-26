//! Legacy → new-protocol cutover machinery.
//!
//! Three concerns live here:
//! - [`extract_pre_cutover_seed`] walks a live legacy [`SystemContextHandle`]
//!   and produces a [`PreCutoverSeed`].
//! - [`CutoverGate`] latches once legacy has crossed into the new version,
//!   extracts the seed, and dispatches it into the new coordinator.
//! - [`forward_legacy_timeout_votes`] and [`forward_legacy_epoch_changes`]
//!   tail the legacy event stream and bridge those events into the
//!   coordinator's client API so the new protocol can form TC2s and
//!   refresh its peer set at epoch boundaries.

use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicBool, Ordering},
};

use async_broadcast::InactiveReceiver;
use futures::StreamExt;
use hotshot::{traits::NodeImplementation, types::SystemContextHandle};
use hotshot_types::{
    data::{EpochNumber, Leaf2},
    event::{Event, EventType},
    traits::{block_contents::BlockHeader, node_implementation::NodeType},
    utils::epoch_from_block_number,
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

/// Latches once legacy crosses into the new protocol. The single API
/// production and tests share: poll [`check`](Self::check) on every
/// coordinator loop iteration; once it returns `true` the cutover has
/// happened and subsequent calls short-circuit.
#[derive(Debug, Default)]
pub struct CutoverGate {
    active: AtomicBool,
}

impl CutoverGate {
    pub fn new() -> Self {
        Self::default()
    }

    /// `true` once a previous `check` succeeded. Lets callers skip the
    /// legacy read lock when already crossed.
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }

    /// Returns `true` once legacy has crossed into the new protocol.
    /// On the first call that observes the crossing, extracts the
    /// pre-cutover seed and dispatches it into the coordinator;
    /// subsequent calls short-circuit.
    pub async fn check<T, I>(
        &self,
        legacy: &SystemContextHandle<T, I>,
        client_api: &ClientApi<T>,
    ) -> bool
    where
        T: NodeType,
        I: NodeImplementation<T>,
    {
        if self.is_active() {
            return true;
        }
        let cur_view = legacy.cur_view().await;
        let crossed =
            legacy.hotshot.upgrade_lock.version_infallible(cur_view) >= NEW_PROTOCOL_VERSION;
        if !crossed {
            return false;
        }

        if let Some(seed) = extract_pre_cutover_seed(legacy).await {
            if let Err(err) = client_api.seed_pre_cutover(seed).await {
                tracing::warn!(%err, "seed_pre_cutover client request failed");
            }
        } else {
            tracing::warn!("seed extraction returned None; coordinator will not be seeded");
        }

        self.active.store(true, Ordering::Relaxed);
        true
    }
}

/// Forward legacy `TimeoutVote2` events into the new-protocol timeout
/// collectors so the first new leader can form TC2 at the boundary.
pub async fn forward_legacy_timeout_votes<T: NodeType>(
    legacy_event_rx: InactiveReceiver<Event<T>>,
    client_api: ClientApi<T>,
) {
    let mut rx = legacy_event_rx.activate_cloned();
    while let Some(event) = rx.next().await {
        if let EventType::LegacyTimeoutVoteEmitted { vote } = event.event
            && let Err(err) = client_api.submit_timeout_vote(vote).await
        {
            tracing::warn!(%err, "failed to forward legacy TimeoutVote2 to new-protocol coordinator");
        }
    }
}

/// Forward legacy epoch transitions into `bump_network_epoch`.
/// `epoch_height == 0` disables forwarding.
pub async fn forward_legacy_epoch_changes<T: NodeType>(
    legacy_event_rx: InactiveReceiver<Event<T>>,
    client_api: ClientApi<T>,
    epoch_height: u64,
) {
    if epoch_height == 0 {
        return;
    }
    let mut rx = legacy_event_rx.activate_cloned();
    let mut last_forwarded: Option<EpochNumber> = None;
    while let Some(event) = rx.next().await {
        let EventType::Decide { leaf_chain, .. } = &event.event else {
            continue;
        };
        let Some(newest) = leaf_chain.first() else {
            continue;
        };
        let block_number = newest.leaf.block_header().block_number();
        let epoch = EpochNumber::new(epoch_from_block_number(block_number, epoch_height));
        if last_forwarded.is_some_and(|prev| epoch <= prev) {
            continue;
        }
        if let Err(err) = client_api.bump_network_epoch(epoch).await {
            tracing::warn!(%epoch, %err, "failed to forward legacy epoch change to new-protocol coordinator");
            continue;
        }
        last_forwarded = Some(epoch);
    }
}
