// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{sync::Arc, time::Instant};

use async_broadcast::{InactiveReceiver, Sender};
use async_lock::RwLock;
use chrono::Utc;
use committable::Committable;
use hotshot_types::{
    consensus::OuterConsensus,
    data::{Leaf2, QuorumProposalWrapper, VidDisperseShare},
    drb::{DrbResult, INITIAL_DRB_RESULT},
    epoch_membership::{EpochMembership, EpochMembershipCoordinator},
    event::{Event, EventType},
    light_client::compute_stake_table_commitment,
    message::{Proposal, UpgradeLock},
    simple_vote::{EpochRootQuorumVote, LightClientStateUpdateVote, QuorumData2, QuorumVote2},
    traits::{
        block_contents::BlockHeader,
        election::Membership,
        node_implementation::{ConsensusTime, NodeImplementation, NodeType},
        signature_key::{SignatureKey, StateSignatureKey},
        storage::Storage,
        ValidatedState,
    },
    utils::{
        epoch_from_block_number, is_epoch_transition, is_last_block, is_transition_block,
        option_epoch_from_block_number,
    },
    vote::HasViewNumber,
};
use hotshot_utils::anytrace::*;
use tracing::instrument;
use vbs::version::StaticVersionType;

use super::QuorumVoteTaskState;
use crate::{
    events::HotShotEvent,
    helpers::{
        broadcast_event, decide_from_proposal, decide_from_proposal_2, fetch_proposal,
        handle_drb_result, LeafChainTraversalOutcome,
    },
    quorum_vote::Versions,
};

/// Store the DRB result from the computation task to the shared `results` table.
///
/// Returns the result if it exists.
async fn get_computed_drb_result<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions>(
    epoch_number: TYPES::Epoch,
    task_state: &mut QuorumVoteTaskState<TYPES, I, V>,
) -> Option<DrbResult> {
    // Return the result if it's already in the table.
    task_state
        .consensus
        .read()
        .await
        .drb_results
        .results
        .get(&epoch_number)
        .cloned()
}

/// Verify the DRB result from the proposal for the next epoch if this is the last block of the
/// current epoch.
///
/// Uses the result from `start_drb_task`.
///
/// Returns an error if we should not vote.
async fn verify_drb_result<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions>(
    proposal: &QuorumProposalWrapper<TYPES>,
    task_state: &mut QuorumVoteTaskState<TYPES, I, V>,
) -> Result<()> {
    // Skip if this is not the expected block.
    if task_state.epoch_height == 0
        || !is_epoch_transition(
            proposal.block_header().block_number(),
            task_state.epoch_height,
        )
    {
        tracing::debug!("Skipping DRB result verification");
        return Ok(());
    }

    // #3967 REVIEW NOTE: Check if this is the right way to decide if we're doing epochs
    // Alternatively, should we just return Err() if epochs aren't happening here? Or can we assume
    // that epochs are definitely happening by virtue of getting here?
    let epoch = option_epoch_from_block_number::<TYPES>(
        task_state
            .upgrade_lock
            .epochs_enabled(proposal.view_number())
            .await,
        proposal.block_header().block_number(),
        task_state.epoch_height,
    );

    let proposal_result = proposal
        .next_drb_result()
        .context(info!("Proposal is missing the DRB result."))?;

    if let Some(epoch_val) = epoch {
        let has_stake_current_epoch = task_state
            .membership
            .stake_table_for_epoch(epoch)
            .await
            .context(warn!("No stake table for epoch {epoch_val}"))?
            .has_stake(&task_state.public_key)
            .await;

        if has_stake_current_epoch {
            let computed_result = get_computed_drb_result(epoch_val + 1, task_state)
                .await
                .context(warn!("DRB result not found"))?;

            ensure!(proposal_result == computed_result, warn!("Our calculated DRB result is {:?}, which does not match the proposed DRB result of {:?}", computed_result, proposal_result));
        }

        Ok(())
    } else {
        Err(error!("Epochs are not available"))
    }
}

/// Store the DRB result for the next epoch if we received it in a decided leaf.
async fn store_drb_result<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions>(
    task_state: &mut QuorumVoteTaskState<TYPES, I, V>,
    decided_leaf: &Leaf2<TYPES>,
) -> Result<()> {
    if task_state.epoch_height == 0 {
        tracing::info!("Epoch height is 0, skipping DRB storage.");
        return Ok(());
    }

    let decided_block_number = decided_leaf.block_header().block_number();
    let current_epoch_number = TYPES::Epoch::new(epoch_from_block_number(
        decided_block_number,
        task_state.epoch_height,
    ));
    // Skip storing the received result if this is not the transition block.
    if is_transition_block(decided_block_number, task_state.epoch_height) {
        if let Some(result) = decided_leaf.next_drb_result {
            // We don't need to check value existence and consistency because it should be
            // impossible to decide on a block with different DRB results.
            handle_drb_result::<TYPES, I>(
                task_state.membership.membership(),
                current_epoch_number + 1,
                &task_state.storage,
                &task_state.consensus,
                result,
            )
            .await;
        } else {
            bail!("The last block of the epoch is decided but doesn't contain a DRB result.");
        }
    }
    Ok(())
}

/// Handles the `QuorumProposalValidated` event.
#[instrument(skip_all, fields(id = task_state.id, view = *proposal.view_number()))]
pub(crate) async fn handle_quorum_proposal_validated<
    TYPES: NodeType,
    I: NodeImplementation<TYPES>,
    V: Versions,
>(
    proposal: &QuorumProposalWrapper<TYPES>,
    task_state: &mut QuorumVoteTaskState<TYPES, I, V>,
    event_sender: &Sender<Arc<HotShotEvent<TYPES>>>,
) -> Result<()> {
    let version = task_state
        .upgrade_lock
        .version(proposal.view_number())
        .await?;

    if version >= V::Epochs::VERSION {
        // Don't vote if the DRB result verification fails.
        verify_drb_result(proposal, task_state).await?;
    }

    let LeafChainTraversalOutcome {
        new_locked_view_number,
        new_decided_view_number,
        new_decide_qc,
        leaf_views,
        included_txns,
        decided_upgrade_cert,
    } = if version >= V::Epochs::VERSION {
        // Skip the decide rule for the last block of the epoch.  This is so
        // that we do not decide the block with epoch_height -2 before we enter the new epoch
        if !is_last_block(
            proposal.block_header().block_number(),
            task_state.epoch_height,
        ) {
            decide_from_proposal_2::<TYPES, I, V>(
                proposal,
                OuterConsensus::new(Arc::clone(&task_state.consensus.inner_consensus)),
                Arc::clone(&task_state.upgrade_lock.decided_upgrade_certificate),
                &task_state.public_key,
                version >= V::Epochs::VERSION,
                &task_state.membership,
                &task_state.storage,
            )
            .await
        } else {
            LeafChainTraversalOutcome::default()
        }
    } else {
        decide_from_proposal::<TYPES, I, V>(
            proposal,
            OuterConsensus::new(Arc::clone(&task_state.consensus.inner_consensus)),
            Arc::clone(&task_state.upgrade_lock.decided_upgrade_certificate),
            &task_state.public_key,
            version >= V::Epochs::VERSION,
            task_state.membership.membership(),
            &task_state.storage,
            task_state.epoch_height,
        )
        .await
    };

    if let (Some(cert), Some(_)) = (decided_upgrade_cert.clone(), new_decided_view_number) {
        let mut decided_certificate_lock = task_state
            .upgrade_lock
            .decided_upgrade_certificate
            .write()
            .await;
        *decided_certificate_lock = Some(cert.clone());
        drop(decided_certificate_lock);

        if cert.data.new_version == V::Epochs::VERSION {
            let epoch_height = task_state.consensus.read().await.epoch_height;
            let first_epoch_number = TYPES::Epoch::new(epoch_from_block_number(
                proposal.block_header().block_number(),
                epoch_height,
            ));

            tracing::debug!("Calling set_first_epoch for epoch {:?}", first_epoch_number);
            task_state
                .membership
                .membership()
                .write()
                .await
                .set_first_epoch(first_epoch_number, INITIAL_DRB_RESULT);

            broadcast_event(
                Arc::new(HotShotEvent::SetFirstEpoch(
                    cert.data.new_version_first_view,
                    first_epoch_number,
                )),
                event_sender,
            )
            .await;
        }

        let _ = task_state
            .storage
            .write()
            .await
            .update_decided_upgrade_certificate(Some(cert.clone()))
            .await;
    }

    let mut consensus_writer = task_state.consensus.write().await;
    if let Some(locked_view_number) = new_locked_view_number {
        let _ = consensus_writer.update_locked_view(locked_view_number);
    }

    #[allow(clippy::cast_precision_loss)]
    if let Some(decided_view_number) = new_decided_view_number {
        // Bring in the cleanup crew. When a new decide is indeed valid, we need to clear out old memory.

        let old_decided_view = consensus_writer.last_decided_view();
        consensus_writer.collect_garbage(old_decided_view, decided_view_number);

        // Set the new decided view.
        consensus_writer
            .update_last_decided_view(decided_view_number)
            .context(|e| {
                warn!(
                    "`update_last_decided_view` failed; this should never happen. Error: {}",
                    e
                )
            })?;

        consensus_writer
            .metrics
            .last_decided_time
            .set(Utc::now().timestamp().try_into().unwrap());
        consensus_writer.metrics.invalid_qc.set(0);
        consensus_writer
            .metrics
            .last_decided_view
            .set(usize::try_from(consensus_writer.last_decided_view().u64()).unwrap());
        let cur_number_of_views_per_decide_event =
            *proposal.view_number() - consensus_writer.last_decided_view().u64();
        consensus_writer
            .metrics
            .number_of_views_per_decide_event
            .add_point(cur_number_of_views_per_decide_event as f64);

        // We don't need to hold this while we broadcast
        drop(consensus_writer);

        for leaf_info in &leaf_views {
            tracing::info!(
                "Sending decide for view {:?} at height {:?}",
                leaf_info.leaf.view_number(),
                leaf_info.leaf.block_header().block_number(),
            );
        }

        // Send an update to everyone saying that we've reached a decide
        broadcast_event(
            Event {
                view_number: decided_view_number,
                event: EventType::Decide {
                    leaf_chain: Arc::new(leaf_views.clone()),
                    // This is never none if we've reached a new decide, so this is safe to unwrap.
                    qc: Arc::new(new_decide_qc.clone().unwrap()),
                    block_size: included_txns.map(|txns| txns.len().try_into().unwrap()),
                },
            },
            &task_state.output_event_stream,
        )
        .await;

        tracing::debug!(
            "Successfully sent decide event, leaf views: {:?}, leaf views len: {:?}, qc view: {:?}",
            decided_view_number,
            leaf_views.len(),
            new_decide_qc.as_ref().unwrap().view_number()
        );

        if version >= V::Epochs::VERSION {
            for leaf_view in leaf_views {
                store_drb_result(task_state, &leaf_view.leaf).await?;
            }
        }
    }

    Ok(())
}

/// Updates the shared consensus state with the new voting data.
#[instrument(skip_all, target = "VoteDependencyHandle", fields(view = *view_number))]
#[allow(clippy::too_many_arguments)]
pub(crate) async fn update_shared_state<
    TYPES: NodeType,
    I: NodeImplementation<TYPES>,
    V: Versions,
>(
    consensus: OuterConsensus<TYPES>,
    sender: Sender<Arc<HotShotEvent<TYPES>>>,
    receiver: InactiveReceiver<Arc<HotShotEvent<TYPES>>>,
    membership: EpochMembershipCoordinator<TYPES>,
    public_key: TYPES::SignatureKey,
    private_key: <TYPES::SignatureKey as SignatureKey>::PrivateKey,
    upgrade_lock: UpgradeLock<TYPES, V>,
    view_number: TYPES::View,
    instance_state: Arc<TYPES::InstanceState>,
    proposed_leaf: &Leaf2<TYPES>,
    vid_share: &Proposal<TYPES, VidDisperseShare<TYPES>>,
    parent_view_number: Option<TYPES::View>,
    epoch_height: u64,
) -> Result<()> {
    let justify_qc = &proposed_leaf.justify_qc();

    let consensus_reader = consensus.read().await;
    // Try to find the validated view within the validated state map. This will be present
    // if we have the saved leaf, but if not we'll get it when we fetch_proposal.
    let mut maybe_validated_view = parent_view_number.and_then(|view_number| {
        consensus_reader
            .validated_state_map()
            .get(&view_number)
            .cloned()
    });

    // Justify qc's leaf commitment should be the same as the parent's leaf commitment.
    let mut maybe_parent = consensus_reader
        .saved_leaves()
        .get(&justify_qc.data.leaf_commit)
        .cloned();

    drop(consensus_reader);

    maybe_parent = match maybe_parent {
        Some(p) => Some(p),
        None => {
            match fetch_proposal(
                justify_qc,
                sender.clone(),
                receiver.activate_cloned(),
                membership.clone(),
                OuterConsensus::new(Arc::clone(&consensus.inner_consensus)),
                public_key.clone(),
                private_key.clone(),
                &upgrade_lock,
                epoch_height,
            )
            .await
            .ok()
            {
                Some((leaf, view)) => {
                    maybe_validated_view = Some(view);
                    Some(leaf)
                },
                None => None,
            }
        },
    };

    let parent = maybe_parent.context(info!(
        "Proposal's parent missing from storage with commitment: {:?}, proposal view {:?}",
        justify_qc.data.leaf_commit,
        proposed_leaf.view_number(),
    ))?;

    let Some(validated_view) = maybe_validated_view else {
        bail!("Failed to fetch view for parent, parent view {parent_view_number:?}");
    };

    let (Some(parent_state), _) = validated_view.state_and_delta() else {
        bail!("Parent state not found! Consensus internally inconsistent");
    };

    let version = upgrade_lock.version(view_number).await?;

    let now = Instant::now();
    let (validated_state, state_delta) = parent_state
        .validate_and_apply_header(
            &instance_state,
            &parent,
            &proposed_leaf.block_header().clone(),
            vid_share.data.payload_byte_len(),
            version,
            *view_number,
        )
        .await
        .wrap()
        .context(warn!("Block header doesn't extend the proposal!"))?;
    let duration = now.elapsed();
    tracing::debug!("Validation time: {:?}", duration);

    let now = Instant::now();
    // Now that we've rounded everyone up, we need to update the shared state
    let mut consensus_writer = consensus.write().await;

    if let Err(e) = consensus_writer.update_leaf(
        proposed_leaf.clone(),
        Arc::new(validated_state),
        Some(Arc::new(state_delta)),
    ) {
        tracing::trace!("{e:?}");
    }

    drop(consensus_writer);
    let duration = now.elapsed();
    tracing::debug!("update_leaf time: {:?}", duration);

    Ok(())
}

/// Submits the `QuorumVoteSend` event if all the dependencies are met.
#[instrument(skip_all, fields(name = "Submit quorum vote", level = "error"))]
#[allow(clippy::too_many_arguments)]
pub(crate) async fn submit_vote<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions>(
    sender: Sender<Arc<HotShotEvent<TYPES>>>,
    membership: EpochMembership<TYPES>,
    public_key: TYPES::SignatureKey,
    private_key: <TYPES::SignatureKey as SignatureKey>::PrivateKey,
    upgrade_lock: UpgradeLock<TYPES, V>,
    view_number: TYPES::View,
    storage: Arc<RwLock<I::Storage>>,
    leaf: Leaf2<TYPES>,
    vid_share: Proposal<TYPES, VidDisperseShare<TYPES>>,
    extended_vote: bool,
    epoch_root_vote: bool,
    epoch_height: u64,
    state_private_key: &<TYPES::StateSignatureKey as StateSignatureKey>::StatePrivateKey,
) -> Result<()> {
    let committee_member_in_current_epoch = membership.has_stake(&public_key).await;
    // If the proposed leaf is for the last block in the epoch and the node is part of the quorum committee
    // in the next epoch, the node should vote to achieve the double quorum.
    let committee_member_in_next_epoch = leaf.with_epoch
        && is_epoch_transition(leaf.height(), epoch_height)
        && membership
            .next_epoch_stake_table()
            .await?
            .has_stake(&public_key)
            .await;

    ensure!(
        committee_member_in_current_epoch || committee_member_in_next_epoch,
        info!("We were not chosen for quorum committee on {view_number:?}")
    );

    let height = if membership.epoch().is_some() {
        Some(leaf.height())
    } else {
        None
    };

    // Create and send the vote.
    let vote = QuorumVote2::<TYPES>::create_signed_vote(
        QuorumData2 {
            leaf_commit: leaf.commit(),
            epoch: membership.epoch(),
            block_number: height,
        },
        view_number,
        &public_key,
        &private_key,
        &upgrade_lock,
    )
    .await
    .wrap()
    .context(error!("Failed to sign vote. This should never happen."))?;
    let now = Instant::now();
    // Add to the storage.
    storage
        .write()
        .await
        .append_vid_general(&vid_share)
        .await
        .wrap()
        .context(error!("Failed to store VID share"))?;
    let duration = now.elapsed();
    tracing::debug!("append_vid_general time: {:?}", duration);

    // Make epoch root vote

    let epoch_enabled = upgrade_lock.epochs_enabled(view_number).await;
    if extended_vote && epoch_enabled {
        tracing::debug!("sending extended vote to everybody",);
        broadcast_event(
            Arc::new(HotShotEvent::ExtendedQuorumVoteSend(vote)),
            &sender,
        )
        .await;
    } else if epoch_root_vote && epoch_enabled {
        tracing::debug!(
            "sending epoch root vote to next quorum leader {:?}",
            vote.view_number() + 1
        );
        let light_client_state = leaf
            .block_header()
            .get_light_client_state(view_number)
            .wrap()
            .context(error!("Failed to generate light client state"))?;
        let next_membership = membership.next_epoch_stake_table().await?;
        let next_stake_table_state = compute_stake_table_commitment(
            &next_membership.stake_table().await,
            hotshot_types::light_client::STAKE_TABLE_CAPACITY,
        );
        let signature = <TYPES::StateSignatureKey as StateSignatureKey>::sign_state(
            state_private_key,
            &light_client_state,
            &next_stake_table_state,
        )
        .wrap()
        .context(error!("Failed to sign the light client state"))?;
        let state_vote = LightClientStateUpdateVote {
            epoch: TYPES::Epoch::new(epoch_from_block_number(leaf.height(), epoch_height)),
            light_client_state,
            next_stake_table_state,
            signature,
        };
        broadcast_event(
            Arc::new(HotShotEvent::EpochRootQuorumVoteSend(EpochRootQuorumVote {
                vote,
                state_vote,
            })),
            &sender,
        )
        .await;
    } else {
        tracing::debug!(
            "sending vote to next quorum leader {:?}",
            vote.view_number() + 1
        );
        broadcast_event(Arc::new(HotShotEvent::QuorumVoteSend(vote)), &sender).await;
    }

    Ok(())
}
