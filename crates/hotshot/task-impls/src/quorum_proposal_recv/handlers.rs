// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

#![allow(dead_code)]

use std::sync::Arc;

use async_broadcast::{broadcast, Receiver, Sender};
use async_lock::{RwLock, RwLockUpgradableReadGuard};
use committable::Committable;
use hotshot_types::{
    consensus::OuterConsensus,
    data::{Leaf2, QuorumProposal, QuorumProposalWrapper},
    epoch_membership::EpochMembershipCoordinator,
    message::Proposal,
    simple_certificate::{QuorumCertificate, QuorumCertificate2},
    simple_vote::HasEpoch,
    traits::{
        block_contents::{BlockHeader, BlockPayload},
        election::Membership,
        node_implementation::{ConsensusTime, NodeImplementation, NodeType},
        signature_key::SignatureKey,
        storage::Storage,
        ValidatedState,
    },
    utils::{
        epoch_from_block_number, is_epoch_root, is_epoch_transition, is_transition_block,
        option_epoch_from_block_number, View, ViewInner,
    },
    vote::{Certificate, HasViewNumber},
};
use hotshot_utils::anytrace::*;
use tokio::spawn;
use tracing::instrument;
use vbs::version::StaticVersionType;

use super::{QuorumProposalRecvTaskState, ValidationInfo};
use crate::{
    events::HotShotEvent,
    helpers::{
        broadcast_event, check_qc_state_cert_correspondence, fetch_proposal, update_high_qc,
        validate_epoch_transition_qc, validate_light_client_state_update_certificate,
        validate_proposal_safety_and_liveness, validate_proposal_view_and_certs,
        validate_qc_and_next_epoch_qc,
    },
    quorum_proposal_recv::{UpgradeLock, Versions},
};

/// Spawn a task which will fire a request to get a proposal, and store it.
#[allow(clippy::too_many_arguments)]
fn spawn_fetch_proposal<TYPES: NodeType, V: Versions>(
    qc: &QuorumCertificate2<TYPES>,
    event_sender: Sender<Arc<HotShotEvent<TYPES>>>,
    event_receiver: Receiver<Arc<HotShotEvent<TYPES>>>,
    membership: EpochMembershipCoordinator<TYPES>,
    consensus: OuterConsensus<TYPES>,
    sender_public_key: TYPES::SignatureKey,
    sender_private_key: <TYPES::SignatureKey as SignatureKey>::PrivateKey,
    upgrade_lock: UpgradeLock<TYPES, V>,
    epoch_height: u64,
) {
    let qc = qc.clone();
    spawn(async move {
        let lock = upgrade_lock;

        let _ = fetch_proposal(
            &qc,
            event_sender,
            event_receiver,
            membership,
            consensus,
            sender_public_key,
            sender_private_key,
            &lock,
            epoch_height,
        )
        .await;
    });
}

/// Update states in the event that the parent state is not found for a given `proposal`.
#[instrument(skip_all)]
pub async fn validate_proposal_liveness<
    TYPES: NodeType,
    I: NodeImplementation<TYPES>,
    V: Versions,
>(
    proposal: &Proposal<TYPES, QuorumProposalWrapper<TYPES>>,
    validation_info: &ValidationInfo<TYPES, I, V>,
) -> Result<()> {
    let mut valid_epoch_transition = false;
    if validation_info
        .upgrade_lock
        .version(proposal.data.view_number())
        .await
        .is_ok_and(|v| v >= V::Epochs::VERSION)
    {
        let Some(block_number) = proposal.data.justify_qc().data.block_number else {
            bail!("Quorum Proposal has no block number but it's after the epoch upgrade");
        };
        if is_epoch_transition(block_number, validation_info.epoch_height) {
            validate_epoch_transition_qc(proposal, validation_info).await?;
            valid_epoch_transition = true;
        }
    }
    let mut consensus_writer = validation_info.consensus.write().await;

    let leaf = Leaf2::from_quorum_proposal(&proposal.data);

    let state = Arc::new(
        <TYPES::ValidatedState as ValidatedState<TYPES>>::from_header(proposal.data.block_header()),
    );

    if let Err(e) = consensus_writer.update_leaf(leaf.clone(), state, None) {
        tracing::trace!("{e:?}");
    }

    let liveness_check = proposal.data.justify_qc().view_number() > consensus_writer.locked_view();
    // if we are using HS2 we update our locked view for any QC from a leader greater than our current lock
    if liveness_check
        && validation_info
            .upgrade_lock
            .version(leaf.view_number())
            .await
            .is_ok_and(|v| v >= V::Epochs::VERSION)
    {
        consensus_writer.update_locked_view(proposal.data.justify_qc().view_number())?;
    }

    drop(consensus_writer);

    if !liveness_check && !valid_epoch_transition {
        bail!("Quorum Proposal failed the liveness check");
    }

    Ok(())
}

async fn validate_epoch_transition_block<
    TYPES: NodeType,
    I: NodeImplementation<TYPES>,
    V: Versions,
>(
    proposal: &Proposal<TYPES, QuorumProposalWrapper<TYPES>>,
    validation_info: &ValidationInfo<TYPES, I, V>,
) -> Result<()> {
    if !validation_info
        .upgrade_lock
        .epochs_enabled(proposal.data.view_number())
        .await
    {
        return Ok(());
    }
    if !is_epoch_transition(
        proposal.data.block_header().block_number(),
        validation_info.epoch_height,
    ) {
        return Ok(());
    }
    // transition block does not have to be empty
    if is_transition_block(
        proposal.data.block_header().block_number(),
        validation_info.epoch_height,
    ) {
        return Ok(());
    }
    // TODO: Is this the best way to do this?
    let (empty_payload, metadata) = <TYPES as NodeType>::BlockPayload::empty();
    let header = proposal.data.block_header();
    ensure!(
        empty_payload.builder_commitment(&metadata) == header.builder_commitment()
            && &metadata == header.metadata(),
        "Block is not empty"
    );
    Ok(())
}

async fn validate_current_epoch<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions>(
    proposal: &Proposal<TYPES, QuorumProposalWrapper<TYPES>>,
    validation_info: &ValidationInfo<TYPES, I, V>,
) -> Result<()> {
    let upgrade_view = validation_info
        .upgrade_lock
        .upgrade_view()
        .await
        .unwrap_or(TYPES::View::new(0));
    if !validation_info
        .upgrade_lock
        .epochs_enabled(proposal.data.view_number())
        .await
        || proposal.data.justify_qc().view_number() <= upgrade_view
    {
        return Ok(());
    }
    if validation_info
        .consensus
        .read()
        .await
        .high_qc()
        .view_number()
        <= upgrade_view
    {
        return Ok(());
    }

    let block_number = proposal.data.block_header().block_number();

    let Some(high_block_number) = validation_info
        .consensus
        .read()
        .await
        .high_qc()
        .data
        .block_number
    else {
        bail!("High QC has no block number");
    };

    ensure!(
        epoch_from_block_number(block_number, validation_info.epoch_height)
            >= epoch_from_block_number(high_block_number + 1, validation_info.epoch_height),
        "Quorum proposal has an inconsistent epoch"
    );

    Ok(())
}

/// Validate that the proposal's block height is one greater than the justification QC's block height.
async fn validate_block_height<TYPES: NodeType>(
    proposal: &Proposal<TYPES, QuorumProposalWrapper<TYPES>>,
) -> Result<()> {
    let Some(qc_block_number) = proposal.data.justify_qc().data.block_number else {
        return Ok(());
    };
    ensure!(
        qc_block_number + 1 == proposal.data.block_header().block_number(),
        "Quorum proposal has an inconsistent block height"
    );
    Ok(())
}

/// Handles the `QuorumProposalRecv` event by first validating the cert itself for the view, and then
/// updating the states, which runs when the proposal cannot be found in the internal state map.
///
/// This code can fail when:
/// - The justify qc is invalid.
/// - The task is internally inconsistent.
/// - The sequencer storage update fails.
#[allow(clippy::too_many_lines)]
#[instrument(skip_all)]
pub(crate) async fn handle_quorum_proposal_recv<
    TYPES: NodeType,
    I: NodeImplementation<TYPES>,
    V: Versions,
>(
    proposal: &Proposal<TYPES, QuorumProposalWrapper<TYPES>>,
    quorum_proposal_sender_key: &TYPES::SignatureKey,
    event_sender: &Sender<Arc<HotShotEvent<TYPES>>>,
    event_receiver: &Receiver<Arc<HotShotEvent<TYPES>>>,
    validation_info: ValidationInfo<TYPES, I, V>,
) -> Result<()> {
    proposal
        .data
        .validate_epoch(&validation_info.upgrade_lock, validation_info.epoch_height)
        .await?;
    // validate the proposal's epoch matches ours
    validate_current_epoch(proposal, &validation_info).await?;
    let quorum_proposal_sender_key = quorum_proposal_sender_key.clone();

    validate_proposal_view_and_certs(proposal, &validation_info)
        .await
        .context(warn!("Failed to validate proposal view or attached certs"))?;

    validate_block_height(proposal).await?;

    let view_number = proposal.data.view_number();

    let justify_qc = proposal.data.justify_qc().clone();
    let maybe_next_epoch_justify_qc = proposal.data.next_epoch_justify_qc().clone();

    let proposal_block_number = proposal.data.block_header().block_number();
    let proposal_epoch = option_epoch_from_block_number::<TYPES>(
        proposal.data.epoch().is_some(),
        proposal_block_number,
        validation_info.epoch_height,
    );

    if justify_qc
        .data
        .block_number
        .is_some_and(|bn| is_epoch_root(bn, validation_info.epoch_height))
    {
        let Some(state_cert) = proposal.data.state_cert() else {
            bail!("Epoch root QC has no state cert");
        };
        ensure!(
            check_qc_state_cert_correspondence(
                &justify_qc,
                state_cert,
                validation_info.epoch_height
            ),
            "Epoch root QC has no corresponding state cert"
        );
        validate_light_client_state_update_certificate(
            state_cert,
            &validation_info.membership.coordinator,
        )
        .await?;
    }

    validate_epoch_transition_block(proposal, &validation_info).await?;

    validate_qc_and_next_epoch_qc(
        &justify_qc,
        maybe_next_epoch_justify_qc.as_ref(),
        &validation_info.consensus,
        &validation_info.membership.coordinator,
        &validation_info.upgrade_lock,
        validation_info.epoch_height,
    )
    .await?;

    broadcast_event(
        Arc::new(HotShotEvent::QuorumProposalPreliminarilyValidated(
            proposal.clone(),
        )),
        event_sender,
    )
    .await;

    // Get the parent leaf and state.
    let parent_leaf = validation_info
        .consensus
        .read()
        .await
        .saved_leaves()
        .get(&justify_qc.data.leaf_commit)
        .cloned();

    if parent_leaf.is_none() {
        spawn_fetch_proposal(
            &justify_qc,
            event_sender.clone(),
            event_receiver.clone(),
            validation_info.membership.coordinator.clone(),
            OuterConsensus::new(Arc::clone(&validation_info.consensus.inner_consensus)),
            // Note that we explicitly use the node key here instead of the provided key in the signature.
            // This is because the key that we receive is for the prior leader, so the payload would be routed
            // incorrectly.
            validation_info.public_key.clone(),
            validation_info.private_key.clone(),
            validation_info.upgrade_lock.clone(),
            validation_info.epoch_height,
        );
    }
    let consensus_reader = validation_info.consensus.read().await;

    let parent = match parent_leaf {
        Some(leaf) => {
            if let (Some(state), _) = consensus_reader.state_and_delta(leaf.view_number()) {
                Some((leaf, Arc::clone(&state)))
            } else {
                bail!("Parent state not found! Consensus internally inconsistent");
            }
        },
        None => None,
    };
    drop(consensus_reader);
    if justify_qc.view_number()
        > validation_info
            .consensus
            .read()
            .await
            .high_qc()
            .view_number()
    {
        update_high_qc(proposal, &validation_info).await?;
    }

    let Some((parent_leaf, _parent_state)) = parent else {
        tracing::warn!(
            "Proposal's parent missing from storage with commitment: {:?}",
            justify_qc.data.leaf_commit
        );
        validate_proposal_liveness(proposal, &validation_info).await?;
        tracing::trace!("Sending ViewChange for view {view_number} and epoch {proposal_epoch:?}");
        validation_info
            .consensus
            .write()
            .await
            .update_highest_block(proposal_block_number);
        broadcast_event(
            Arc::new(HotShotEvent::ViewChange(view_number, proposal_epoch)),
            event_sender,
        )
        .await;
        return Ok(());
    };

    // Validate the proposal
    validate_proposal_safety_and_liveness::<TYPES, I, V>(
        proposal.clone(),
        parent_leaf,
        &validation_info,
        event_sender.clone(),
        quorum_proposal_sender_key,
    )
    .await?;

    tracing::trace!("Sending ViewChange for view {view_number} and epoch {proposal_epoch:?}");
    validation_info
        .consensus
        .write()
        .await
        .update_highest_block(proposal_block_number);
    {
        validation_info.consensus.write().await.highest_block = proposal_block_number;
    }
    broadcast_event(
        Arc::new(HotShotEvent::ViewChange(view_number, proposal_epoch)),
        event_sender,
    )
    .await;

    Ok(())
}
