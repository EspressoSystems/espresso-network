//! Harvest legacy state and dispatch the seed via `ClientApi`.

use std::{collections::BTreeMap, sync::Arc};

use async_broadcast::InactiveReceiver;
use committable::Committable;
use futures::StreamExt;
use hotshot::{traits::NodeImplementation, types::SystemContextHandle};
use hotshot_types::{
    data::{EpochNumber, Leaf2, ViewNumber},
    event::{Event, EventType},
    simple_certificate::QuorumCertificate2,
    traits::{block_contents::BlockHeader, node_implementation::NodeType},
    utils::epoch_from_block_number,
};
use versions::CLIQUENET_VERSION;

use crate::client::ClientApi;

/// Inputs to `seed_pre_cutover`.
pub struct LegacyPreCutoverSeed<T: NodeType> {
    pub decided_anchor: Leaf2<T>,
    /// Oldest-first chain above the anchor.
    pub undecided: Vec<Leaf2<T>>,
    pub high_qc: QuorumCertificate2<T>,
    pub validated_states: BTreeMap<ViewNumber, Arc<T::ValidatedState>>,
    /// `upgrade_cert.new_version_first_view`.
    pub cutover_view: ViewNumber,
}

/// Walk legacy state to produce a [`LegacyPreCutoverSeed`]; `None` on
/// a broken walk.
pub async fn harvest_legacy_pre_cutover_seed<T, I>(
    handle: &SystemContextHandle<T, I>,
) -> Option<LegacyPreCutoverSeed<T>>
where
    T: NodeType,
    I: NodeImplementation<T>,
{
    let cutover_view = match handle.hotshot.upgrade_lock.decided_upgrade_cert() {
        Some(cert) => cert.data.new_version_first_view,
        None => {
            tracing::warn!(
                "harvest_legacy_pre_cutover_seed: no decided upgrade certificate; aborting",
            );
            return None;
        },
    };

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
        cutover_view,
    })
}

/// Returns `true` once legacy crossed into the new version, dispatching
/// the seed along the way. Callers should gate repeats with a once-flag.
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
                seed.cutover_view,
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

/// Bridge legacy epoch transitions into `bump_network_epoch`.
/// `epoch_height == 0` disables the bridge.
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
