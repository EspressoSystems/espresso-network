// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{sync::Arc, time::Duration};

use async_broadcast::Sender;
use chrono::Utc;
use hotshot_types::{
    event::{Event, EventType},
    simple_certificate::EpochRootQuorumCertificate,
    simple_vote::{EpochRootQuorumVote, HasEpoch, QuorumVote2, TimeoutData2, TimeoutVote2},
    traits::node_implementation::{ConsensusTime, NodeImplementation, NodeType},
    utils::{is_epoch_root, is_epoch_transition, is_last_block, EpochTransitionIndicator},
    vote::{HasViewNumber, Vote},
};
use hotshot_utils::anytrace::*;
use tokio::{spawn, time::sleep};
use tracing::instrument;
use vbs::version::StaticVersionType;

use super::ConsensusTaskState;
use crate::{
    consensus::Versions,
    events::HotShotEvent,
    helpers::{broadcast_event, check_qc_state_cert_correspondence},
    vote_collection::{handle_epoch_root_vote, handle_vote},
};

/// Handle a `QuorumVoteRecv` event.
pub(crate) async fn handle_quorum_vote_recv<
    TYPES: NodeType,
    I: NodeImplementation<TYPES>,
    V: Versions,
>(
    vote: &QuorumVote2<TYPES>,
    event: Arc<HotShotEvent<TYPES>>,
    sender: &Sender<Arc<HotShotEvent<TYPES>>>,
    task_state: &mut ConsensusTaskState<TYPES, I, V>,
) -> Result<()> {
    let in_transition = task_state
        .consensus
        .read()
        .await
        .is_high_qc_for_epoch_transition();
    let epoch_membership = task_state
        .membership_coordinator
        .membership_for_epoch(vote.data.epoch)
        .await
        .context(warn!("No stake table for epoch"))?;

    let we_are_leader =
        epoch_membership.leader(vote.view_number() + 1).await? == task_state.public_key;
    ensure!(
        in_transition || we_are_leader,
        info!(
            "We are not the leader for view {:?} and we are not in the epoch transition",
            vote.view_number() + 1
        )
    );

    let transition_indicator = if in_transition {
        EpochTransitionIndicator::InTransition
    } else {
        EpochTransitionIndicator::NotInTransition
    };
    handle_vote(
        &mut task_state.vote_collectors,
        vote,
        task_state.public_key.clone(),
        &epoch_membership,
        task_state.id,
        &event,
        sender,
        &task_state.upgrade_lock,
        transition_indicator.clone(),
    )
    .await?;

    if vote.epoch().is_some()
        && vote
            .data
            .block_number
            .is_some_and(|b| is_epoch_transition(b, task_state.epoch_height))
    {
        // If the vote sender belongs to the next epoch, collect it separately to form the second QC
        let has_stake = epoch_membership
            .next_epoch_stake_table()
            .await?
            .has_stake(&vote.signing_key())
            .await;
        if has_stake {
            handle_vote(
                &mut task_state.next_epoch_vote_collectors,
                &vote.clone().into(),
                task_state.public_key.clone(),
                // We eventually verify in `handle_vote` that we are the leader before assembling the certificate here,
                // so we must request the full randomized stake table.
                //
                // I'm not sure this is really necessary, but I've opted not to modify the logic.
                &epoch_membership.next_epoch().await?.clone(),
                task_state.id,
                &event,
                sender,
                &task_state.upgrade_lock,
                transition_indicator,
            )
            .await?;
        }
    }

    Ok(())
}

/// Handle a `QuorumVoteRecv` event.
pub(crate) async fn handle_epoch_root_quorum_vote_recv<
    TYPES: NodeType,
    I: NodeImplementation<TYPES>,
    V: Versions,
>(
    vote: &EpochRootQuorumVote<TYPES>,
    event: Arc<HotShotEvent<TYPES>>,
    sender: &Sender<Arc<HotShotEvent<TYPES>>>,
    task_state: &mut ConsensusTaskState<TYPES, I, V>,
) -> Result<()> {
    ensure!(
        vote.vote
            .data
            .block_number
            .is_some_and(|bn| is_epoch_root(bn, task_state.epoch_height)),
        error!("Received epoch root quorum vote for non epoch root block.")
    );

    let epoch_membership = task_state
        .membership_coordinator
        .membership_for_epoch(vote.vote.data.epoch)
        .await
        .context(warn!("No stake table for epoch"))?;

    let we_are_leader =
        epoch_membership.leader(vote.view_number() + 1).await? == task_state.public_key;
    ensure!(
        we_are_leader,
        info!(
            "We are not the leader for view {:?}",
            vote.view_number() + 1
        )
    );

    handle_epoch_root_vote(
        &mut task_state.epoch_root_vote_collectors,
        vote,
        task_state.public_key.clone(),
        &epoch_membership,
        task_state.id,
        &event,
        sender,
        &task_state.upgrade_lock,
    )
    .await?;

    Ok(())
}

/// Handle a `TimeoutVoteRecv` event.
pub(crate) async fn handle_timeout_vote_recv<
    TYPES: NodeType,
    I: NodeImplementation<TYPES>,
    V: Versions,
>(
    vote: &TimeoutVote2<TYPES>,
    event: Arc<HotShotEvent<TYPES>>,
    sender: &Sender<Arc<HotShotEvent<TYPES>>>,
    task_state: &mut ConsensusTaskState<TYPES, I, V>,
) -> Result<()> {
    let epoch_membership = task_state
        .membership_coordinator
        .membership_for_epoch(task_state.cur_epoch)
        .await
        .context(warn!("No stake table for epoch"))?;
    // Are we the leader for this view?
    ensure!(
        epoch_membership.leader(vote.view_number() + 1).await? == task_state.public_key,
        info!(
            "We are not the leader for view {:?}",
            vote.view_number() + 1
        )
    );

    handle_vote(
        &mut task_state.timeout_vote_collectors,
        vote,
        task_state.public_key.clone(),
        &task_state
            .membership_coordinator
            .membership_for_epoch(vote.data.epoch)
            .await?,
        task_state.id,
        &event,
        sender,
        &task_state.upgrade_lock,
        EpochTransitionIndicator::NotInTransition,
    )
    .await?;

    Ok(())
}

/// Send an event to the next leader containing the highest QC we have
/// This is a necessary part of HotStuff 2 but not the original HotStuff
///
/// #Errors
/// Returns and error if we can't get the version or the version doesn't
/// yet support HS 2
pub async fn send_high_qc<TYPES: NodeType, V: Versions, I: NodeImplementation<TYPES>>(
    new_view_number: TYPES::View,
    sender: &Sender<Arc<HotShotEvent<TYPES>>>,
    task_state: &mut ConsensusTaskState<TYPES, I, V>,
) -> Result<()> {
    let version = task_state.upgrade_lock.version(new_view_number).await?;
    ensure!(
        version >= V::Epochs::VERSION,
        debug!("HotStuff 2 upgrade not yet in effect")
    );

    let consensus_reader = task_state.consensus.read().await;
    let high_qc = consensus_reader.high_qc().clone();
    let is_eqc = high_qc
        .data
        .block_number
        .is_some_and(|b| is_last_block(b, task_state.epoch_height));
    let is_epoch_root = high_qc
        .data
        .block_number
        .is_some_and(|b| is_epoch_root(b, task_state.epoch_height));
    let state_cert = if is_epoch_root {
        consensus_reader.state_cert().cloned()
    } else {
        None
    };
    drop(consensus_reader);

    if is_eqc {
        let maybe_next_epoch_high_qc = task_state
            .consensus
            .read()
            .await
            .next_epoch_high_qc()
            .cloned();
        ensure!(
            maybe_next_epoch_high_qc
                .as_ref()
                .is_some_and(|neqc| neqc.data.leaf_commit == high_qc.data.leaf_commit),
            "We've seen an extended QC but we don't have a corresponding next epoch extended QC"
        );

        tracing::debug!(
            "Broadcasting Extended QC for view {:?} and epoch {:?}, my id {:?}.",
            high_qc.view_number(),
            high_qc.epoch(),
            task_state.id
        );
        broadcast_event(
            Arc::new(HotShotEvent::ExtendedQcSend(
                high_qc,
                maybe_next_epoch_high_qc.unwrap(),
                task_state.public_key.clone(),
            )),
            sender,
        )
        .await;
    } else {
        let leader = task_state
            .membership_coordinator
            .membership_for_epoch(task_state.cur_epoch)
            .await?
            .leader(new_view_number)
            .await?;

        let (high_qc, maybe_next_epoch_qc) = if high_qc
            .data
            .block_number
            .is_some_and(|b| is_epoch_transition(b, task_state.epoch_height))
        {
            let Some((qc, next_epoch_qc)) =
                task_state.consensus.read().await.transition_qc().cloned()
            else {
                bail!("We don't have a transition QC");
            };
            ensure!(
                next_epoch_qc.data.leaf_commit == qc.data.leaf_commit,
                "Transition QC is invalid because leaf commits are not equal."
            );
            (qc, Some(next_epoch_qc))
        } else {
            (high_qc, None)
        };

        if is_epoch_root {
            // For epoch root QC, we are sending high QC and state cert
            let Some(state_cert) = state_cert else {
                bail!("We are sending an epoch root QC but we don't have the corresponding state cert.");
            };
            ensure!(
                check_qc_state_cert_correspondence(&high_qc, &state_cert, task_state.epoch_height),
                "We are sending an epoch root QC but we don't have the corresponding state cert."
            );

            tracing::trace!(
                "Sending epoch root QC for view {:?}, height {:?}",
                high_qc.view_number(),
                high_qc.data.block_number
            );
            broadcast_event(
                Arc::new(HotShotEvent::EpochRootQcSend(
                    EpochRootQuorumCertificate {
                        qc: high_qc,
                        state_cert,
                    },
                    leader,
                    task_state.public_key.clone(),
                )),
                sender,
            )
            .await;
        } else {
            tracing::trace!(
                "Sending high QC for view {:?}, height {:?}",
                high_qc.view_number(),
                high_qc.data.block_number
            );
            broadcast_event(
                Arc::new(HotShotEvent::HighQcSend(
                    high_qc,
                    maybe_next_epoch_qc,
                    leader,
                    task_state.public_key.clone(),
                )),
                sender,
            )
            .await;
        }
    }
    Ok(())
}

/// Handle a `ViewChange` event.
#[instrument(skip_all)]
pub(crate) async fn handle_view_change<
    TYPES: NodeType,
    I: NodeImplementation<TYPES>,
    V: Versions,
>(
    new_view_number: TYPES::View,
    epoch_number: Option<TYPES::Epoch>,
    sender: &Sender<Arc<HotShotEvent<TYPES>>>,
    task_state: &mut ConsensusTaskState<TYPES, I, V>,
) -> Result<()> {
    if epoch_number > task_state.cur_epoch {
        task_state.cur_epoch = epoch_number;
        if let Some(new_epoch) = epoch_number {
            let _ = task_state.consensus.write().await.update_epoch(new_epoch);
            tracing::info!("Progress: entered epoch {:>6}", *new_epoch);
        }
    }

    ensure!(
        new_view_number > task_state.cur_view,
        "New view is not larger than the current view"
    );

    let old_view_number = task_state.cur_view;
    tracing::debug!("Updating view from {old_view_number:?} to {new_view_number:?}");

    if *old_view_number / 100 != *new_view_number / 100 {
        tracing::info!("Progress: entered view {:>6}", *new_view_number);
    }

    // Send our high qc to the next leader immediately upon finishing a view.
    // Part of HotStuff 2
    let _ = send_high_qc(new_view_number, sender, task_state)
        .await
        .inspect_err(|e| {
            tracing::debug!("High QC sending failed with error: {e:?}");
        });

    // Move this node to the next view
    task_state.cur_view = new_view_number;
    task_state
        .consensus
        .write()
        .await
        .update_view(new_view_number)?;

    // If we have a decided upgrade certificate, the protocol version may also have been upgraded.
    let decided_upgrade_certificate_read = task_state
        .upgrade_lock
        .decided_upgrade_certificate
        .read()
        .await
        .clone();
    if let Some(cert) = decided_upgrade_certificate_read {
        if new_view_number == cert.data.new_version_first_view {
            tracing::error!("Version upgraded based on a decided upgrade cert: {cert:?}");
        }
    }

    // Spawn a timeout task if we did actually update view
    let timeout = task_state.timeout;
    let new_timeout_task = spawn({
        let stream = sender.clone();
        let view_number = new_view_number;
        async move {
            sleep(Duration::from_millis(timeout)).await;
            broadcast_event(
                Arc::new(HotShotEvent::Timeout(
                    TYPES::View::new(*view_number),
                    epoch_number,
                )),
                &stream,
            )
            .await;
        }
    });

    // Cancel the old timeout task
    std::mem::replace(&mut task_state.timeout_task, new_timeout_task).abort();

    let old_view_leader_key = task_state
        .membership_coordinator
        .membership_for_epoch(task_state.cur_epoch)
        .await
        .context(warn!("No stake table for epoch"))?
        .leader(old_view_number)
        .await?;

    let consensus_reader = task_state.consensus.read().await;
    consensus_reader
        .metrics
        .current_view
        .set(usize::try_from(task_state.cur_view.u64()).unwrap());
    let cur_view_time = Utc::now().timestamp();
    if old_view_leader_key == task_state.public_key {
        #[allow(clippy::cast_precision_loss)]
        consensus_reader
            .metrics
            .view_duration_as_leader
            .add_point((cur_view_time - task_state.cur_view_time) as f64);
    }
    task_state.cur_view_time = cur_view_time;

    // Do the comparison before the subtraction to avoid potential overflow, since
    // `last_decided_view` may be greater than `cur_view` if the node is catching up.
    if usize::try_from(task_state.cur_view.u64()).unwrap()
        > usize::try_from(consensus_reader.last_decided_view().u64()).unwrap()
    {
        consensus_reader
            .metrics
            .number_of_views_since_last_decide
            .set(
                usize::try_from(task_state.cur_view.u64()).unwrap()
                    - usize::try_from(consensus_reader.last_decided_view().u64()).unwrap(),
            );
    }

    broadcast_event(
        Event {
            view_number: old_view_number,
            event: EventType::ViewFinished {
                view_number: old_view_number,
            },
        },
        &task_state.output_event_stream,
    )
    .await;
    Ok(())
}

/// Handle a `Timeout` event.
#[instrument(skip_all)]
pub(crate) async fn handle_timeout<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions>(
    view_number: TYPES::View,
    epoch: Option<TYPES::Epoch>,
    sender: &Sender<Arc<HotShotEvent<TYPES>>>,
    task_state: &mut ConsensusTaskState<TYPES, I, V>,
) -> Result<()> {
    ensure!(
        task_state.cur_view <= view_number,
        "Timeout event is for an old view"
    );

    ensure!(
        task_state
            .membership_coordinator
            .stake_table_for_epoch(epoch)
            .await
            .context(warn!("No stake table for epoch"))?
            .has_stake(&task_state.public_key)
            .await,
        debug!("We were not chosen for the consensus committee for view {view_number:?}",)
    );

    let vote = TimeoutVote2::create_signed_vote(
        TimeoutData2::<TYPES> {
            view: view_number,
            epoch,
        },
        view_number,
        &task_state.public_key,
        &task_state.private_key,
        &task_state.upgrade_lock,
    )
    .await
    .wrap()
    .context(error!("Failed to sign TimeoutData"))?;

    broadcast_event(Arc::new(HotShotEvent::TimeoutVoteSend(vote)), sender).await;
    broadcast_event(
        Event {
            view_number,
            event: EventType::ViewTimeout { view_number },
        },
        &task_state.output_event_stream,
    )
    .await;

    tracing::error!("We did not receive evidence for view {view_number} in time, sending timeout vote for that view!");

    broadcast_event(
        Event {
            view_number,
            event: EventType::ReplicaViewTimeout { view_number },
        },
        &task_state.output_event_stream,
    )
    .await;

    let leader = task_state
        .membership_coordinator
        .membership_for_epoch(task_state.cur_epoch)
        .await
        .context(warn!("No stake table for epoch"))?
        .leader(view_number)
        .await;

    let consensus_reader = task_state.consensus.read().await;
    consensus_reader.metrics.number_of_timeouts.add(1);
    if leader? == task_state.public_key {
        consensus_reader.metrics.number_of_timeouts_as_leader.add(1);
    }

    Ok(())
}
