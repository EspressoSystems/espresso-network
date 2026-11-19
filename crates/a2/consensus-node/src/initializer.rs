//! Where consensus begins: from what the consensus store holds, or from genesis.

use std::{cmp::max, sync::Arc};

use anyhow::{Context, ensure};
use consensus::store::{ConsensusStore, Restored};
use espresso_types::{Leaf2, NodeState, ValidatedState};
use hotshot_types::{
    data::{QuorumProposalWrapper, ViewNumber},
    initializer::HotShotInitializer,
    message::Proposal,
    simple_certificate::QuorumCertificate2,
    traits::states::ValidatedState as _,
};
use versions::Upgrade;

use crate::{SeqTypes, decided_state, membership_store::MembershipStore};

/// The initializer of a node with node state `state`, resuming from `restored`, what `store`
/// held when it was opened, or starting from genesis if it held no decided leaf.
pub async fn initializer(
    state: NodeState,
    upgrade: Upgrade,
    store: &ConsensusStore<SeqTypes>,
    restored: &Restored<SeqTypes>,
    membership: &MembershipStore,
) -> anyhow::Result<HotShotInitializer<SeqTypes>> {
    let epoch_height = state.epoch_height.unwrap_or_default();
    let genesis_state = ValidatedState::genesis(&state).0;
    let (leaf, cert1, validated_state) = match &restored.anchor {
        Some(anchor) => {
            ensure!(
                anchor.leaf.view_number() == anchor.cert1.view_number,
                "the stored anchor is of view {}, but its certificate of view {}",
                anchor.leaf.view_number(),
                anchor.cert1.view_number
            );
            // What the store kept of the anchor's state; catchup fetches the rest.
            let validated_state = decided_state::load(store, &anchor.leaf)
                .inspect_err(|err| tracing::warn!("cannot load the decided state: {err:#}"))
                .unwrap_or_else(|_| ValidatedState::from_header(anchor.leaf.block_header()));
            tracing::info!(
                height = anchor.leaf.height(),
                "resuming from the stored anchor"
            );
            (anchor.leaf.clone(), anchor.cert1.clone(), validated_state)
        },
        None => {
            tracing::info!("starting from genesis");
            let leaf = Leaf2::genesis(&genesis_state, &state, upgrade.base).await;
            let cert1 = QuorumCertificate2::genesis(&genesis_state, &state, upgrade).await;
            (leaf, cert1, genesis_state)
        },
    };

    // The lock may be later than the anchor's certificate.
    let high_qc = match &restored.locked_qc {
        Some(locked) if locked.view_number > cert1.view_number => locked.clone(),
        _ => cert1,
    };
    let acted_view = restored.acted_view.unwrap_or_else(ViewNumber::genesis);
    // Never earlier than the anchor, so no view is entered again and catchup does not start
    // before the anchor.
    let restart_view = max(
        restored.restart_view.unwrap_or_else(ViewNumber::genesis),
        leaf.view_number(),
    );
    let proposals = restored
        .proposals
        .iter()
        .map(|(view, signed)| {
            let proposal = Proposal {
                data: QuorumProposalWrapper::from(signed.data.clone()),
                signature: signed.signature.clone(),
                _pd: std::marker::PhantomData,
            };
            (*view, proposal)
        })
        .collect();
    let start_epoch_info = membership
        .start_epoch_info()
        .await
        .context("reading the stored DRB results")?;

    Ok(HotShotInitializer::load(
        state,
        epoch_height,
        start_epoch_info,
        (leaf, Some(Arc::new(validated_state))),
        restart_view,
        high_qc,
        acted_view,
        proposals,
        restored.upgrade_cert.clone(),
        restored.state_cert.clone(),
    ))
}
