//! Legacy → new-protocol handover: harvest legacy state and dispatch
//! the seed via `ClientApi`. Shared by `ConsensusHandle::new_protocol`
//! (production) and `tests::legacy_handover` (integration).

use std::{collections::BTreeMap, sync::Arc};

use async_broadcast::InactiveReceiver;
use committable::Committable;
use futures::StreamExt;
use hotshot::{traits::NodeImplementation, types::SystemContextHandle};
use hotshot_types::{
    data::{Leaf2, ViewNumber},
    event::{Event, EventType},
    simple_certificate::QuorumCertificate2,
    traits::node_implementation::NodeType,
};
use versions::CLIQUENET_VERSION;

use crate::client::ClientApi;

/// Inputs to the new protocol's `seed_pre_cutover` request.
pub struct LegacyPreCutoverSeed<T: NodeType> {
    pub decided_anchor: Leaf2<T>,
    /// Oldest-first chain above the anchor, walked from `high_qc` via
    /// `justify_qc` back to the anchor.
    pub undecided: Vec<Leaf2<T>>,
    pub high_qc: QuorumCertificate2<T>,
    /// Per-view validated state for the anchor + each undecided leaf.
    /// The first post-cutover header request needs the parent view's
    /// state to build against.
    pub validated_states: BTreeMap<ViewNumber, Arc<T::ValidatedState>>,
}

/// Walk the legacy `Consensus` to produce a [`LegacyPreCutoverSeed`].
/// `None` on a broken walk (fork or missing leaf). Validated states
/// are best-effort; missing entries are tolerated by the handler.
pub async fn harvest_legacy_pre_cutover_seed<T, I>(
    handle: &SystemContextHandle<T, I>,
) -> Option<LegacyPreCutoverSeed<T>>
where
    T: NodeType,
    I: NodeImplementation<T>,
{
    let consensus_arc = handle.hotshot.consensus();
    let consensus = consensus_arc.read().await;
    let decided_anchor = consensus.decided_leaf();
    let decided_view = decided_anchor.view_number();
    let decided_commit = decided_anchor.commit();

    let high_qc = consensus.high_qc().clone();
    let saved = consensus.saved_leaves();

    let mut chain: Vec<Leaf2<T>> = Vec::new();
    let mut next_commit = high_qc.data.leaf_commit;
    loop {
        if next_commit == decided_commit {
            break;
        }
        let Some(leaf) = saved.get(&next_commit) else {
            tracing::warn!(
                %next_commit,
                "harvest_legacy_pre_cutover_seed: missing leaf in saved_leaves; aborting",
            );
            return None;
        };
        if leaf.view_number() <= decided_view {
            tracing::warn!(
                leaf_view = *leaf.view_number(),
                %decided_view,
                "harvest_legacy_pre_cutover_seed: walked below decided view without matching commit; aborting",
            );
            return None;
        }
        chain.push(leaf.clone());
        next_commit = leaf.justify_qc().data.leaf_commit;
    }

    chain.reverse();

    let mut validated_states = BTreeMap::new();
    if let Some(state) = consensus.state(decided_view) {
        validated_states.insert(decided_view, state.clone());
    } else {
        tracing::warn!(
            %decided_view,
            "harvest_legacy_pre_cutover_seed: no validated state for decided anchor",
        );
    }
    for leaf in &chain {
        let view = leaf.view_number();
        if let Some(state) = consensus.state(view) {
            validated_states.insert(view, state.clone());
        } else {
            tracing::warn!(
                %view,
                "harvest_legacy_pre_cutover_seed: no validated state for undecided leaf",
            );
        }
    }

    Some(LegacyPreCutoverSeed {
        decided_anchor,
        undecided: chain,
        high_qc,
        validated_states,
    })
}

/// Returns `true` once `legacy.cur_view`'s version >= `CLIQUENET_VERSION`,
/// dispatching a best-effort `seed_pre_cutover` through `client_api` on
/// the way. Logs but does not surface harvest/seed failures — the boundary
/// signal stands regardless so callers don't flip back to legacy.
///
/// Re-seeding is idempotent at the consensus layer; callers should still
/// gate repeats with a once-flag.
pub async fn try_perform_handover<T, I>(
    legacy: &SystemContextHandle<T, I>,
    client_api: &ClientApi<T>,
) -> bool
where
    T: NodeType,
    I: NodeImplementation<T>,
{
    let cur_view = legacy.cur_view().await;
    let crossed = legacy.hotshot.upgrade_lock.version_infallible(cur_view) >= CLIQUENET_VERSION;
    if !crossed {
        return false;
    }

    if let Some(seed) = harvest_legacy_pre_cutover_seed(legacy).await {
        if let Err(err) = client_api
            .seed_pre_cutover(
                seed.decided_anchor,
                seed.undecided,
                Some(seed.high_qc),
                seed.validated_states,
            )
            .await
        {
            tracing::warn!(%err, "seed_pre_cutover client request failed");
        }
    } else {
        tracing::warn!(
            "harvest_legacy_pre_cutover_seed returned None; coordinator will not be seeded",
        );
    }

    true
}

/// Forward `LegacyTimeoutVoteEmitted` events from the legacy task into the
/// new-protocol coordinator's timeout collectors. Lets the first 0.8
/// leader form a `TimeoutCertificate2` for the boundary view if 0.4
/// timed out before its QC formed.
///
/// Run as a long-lived task. Spawned by `ConsensusHandle::new` in
/// production and by the integration test for the same parity reason
/// `try_perform_handover` is shared.
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
