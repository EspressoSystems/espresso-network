// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{marker::PhantomData, sync::Arc};

use async_broadcast::{Receiver, Sender};
use async_trait::async_trait;
use hotshot_task::task::TaskState;
use hotshot_types::{
    consensus::{Consensus, OuterConsensus, PayloadWithMetadata},
    data::{vid_commitment, vid_disperse::vid_total_weight, DaProposal2, PackedBundle},
    epoch_membership::EpochMembershipCoordinator,
    event::{Event, EventType},
    message::{Proposal, UpgradeLock},
    simple_certificate::DaCertificate2,
    simple_vote::{DaData2, DaVote2},
    traits::{
        network::ConnectedNetwork,
        node_implementation::{NodeImplementation, NodeType, Versions},
        signature_key::SignatureKey,
        storage::Storage,
        BlockPayload, EncodeBytes,
    },
    utils::EpochTransitionIndicator,
    vote::HasViewNumber,
};
use hotshot_utils::anytrace::*;
use sha2::{Digest, Sha256};
use tokio::{spawn, task::spawn_blocking};
use tracing::instrument;

use crate::{
    events::HotShotEvent,
    helpers::broadcast_event,
    vote_collection::{handle_vote, VoteCollectorsMap},
};

/// Tracks state of a DA task
pub struct DaTaskState<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> {
    /// Output events to application
    pub output_event_stream: async_broadcast::Sender<Event<TYPES>>,

    /// View number this view is executing in.
    pub cur_view: TYPES::View,

    /// Epoch number this node is executing in.
    pub cur_epoch: Option<TYPES::Epoch>,

    /// Reference to consensus. Leader will require a read lock on this.
    pub consensus: OuterConsensus<TYPES>,

    /// Membership for the DA committee and quorum committee.
    /// We need the latter only for calculating the proper VID scheme
    /// from the number of nodes in the quorum.
    pub membership_coordinator: EpochMembershipCoordinator<TYPES>,

    /// The underlying network
    pub network: Arc<I::Network>,

    /// A map of `DaVote` collector tasks.
    pub vote_collectors: VoteCollectorsMap<TYPES, DaVote2<TYPES>, DaCertificate2<TYPES>, V>,

    /// This Nodes public key
    pub public_key: TYPES::SignatureKey,

    /// This Nodes private key
    pub private_key: <TYPES::SignatureKey as SignatureKey>::PrivateKey,

    /// This state's ID
    pub id: u64,

    /// This node's storage ref
    pub storage: I::Storage,

    /// Lock for a decided upgrade
    pub upgrade_lock: UpgradeLock<TYPES, V>,
}

impl<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> DaTaskState<TYPES, I, V> {
    /// main task event handler
    #[instrument(skip_all, fields(id = self.id, view = *self.cur_view, epoch = self.cur_epoch.map(|x| *x)), name = "DA Main Task", level = "error", target = "DaTaskState")]
    pub async fn handle(
        &mut self,
        event: Arc<HotShotEvent<TYPES>>,
        event_stream: Sender<Arc<HotShotEvent<TYPES>>>,
    ) -> Result<()> {
        match event.as_ref() {
            HotShotEvent::DaProposalRecv(proposal, sender) => {
                let sender = sender.clone();
                tracing::debug!(
                    "DA proposal received for view: {:?}",
                    proposal.data.view_number()
                );
                // ED NOTE: Assuming that the next view leader is the one who sends DA proposal for this view
                let view = proposal.data.view_number();

                // Allow a DA proposal that is one view older, in case we have voted on a quorum
                // proposal and updated the view.
                //
                // Anything older is discarded because it is no longer relevant.
                ensure!(
                    self.cur_view <= view + 1,
                    "Throwing away DA proposal that is more than one view older"
                );

                if let Some(entry) = self.consensus.read().await.saved_payloads().get(&view) {
                    ensure!(
                        entry.payload.encode() == proposal.data.encoded_transactions,
                        "Received DA proposal for view {view:?} but we already have a payload for that view and they are not identical.  Throwing it away",
                    );
                }

                let encoded_transactions_hash = Sha256::digest(&proposal.data.encoded_transactions);
                let view_leader_key = self
                    .membership_coordinator
                    .membership_for_epoch(proposal.data.epoch)
                    .await
                    .context(warn!("No stake table for epoch {:?}", proposal.data.epoch))?
                    .leader(view)
                    .await?;
                ensure!(
                    view_leader_key == sender,
                    warn!(
                      "DA proposal doesn't have expected leader key for view {} \n DA proposal is: {:?}",
                      *view,
                      proposal.data.clone()
                    )
                );

                ensure!(
                    view_leader_key.validate(&proposal.signature, &encoded_transactions_hash),
                    warn!("Could not verify proposal.")
                );

                broadcast_event(
                    Arc::new(HotShotEvent::DaProposalValidated(proposal.clone(), sender)),
                    &event_stream,
                )
                .await;
            },
            HotShotEvent::DaProposalValidated(proposal, sender) => {
                let cur_view = self.consensus.read().await.cur_view();
                let view_number = proposal.data.view_number();
                let epoch_number = proposal.data.epoch;
                let membership = self
                    .membership_coordinator
                    .stake_table_for_epoch(epoch_number)
                    .await
                    .context(warn!("No stake table for epoch"))?;

                ensure!(
                  cur_view <= view_number + 1,
                  debug!(
                    "Validated DA proposal for prior view but it's too old now Current view {:?}, DA Proposal view {:?}", 
                    cur_view,
                    proposal.data.view_number()
                  )
                );

                // Proposal is fresh and valid, notify the application layer
                broadcast_event(
                    Event {
                        view_number,
                        event: EventType::DaProposal {
                            proposal: proposal.clone(),
                            sender: sender.clone(),
                        },
                    },
                    &self.output_event_stream,
                )
                .await;

                ensure!(
                    membership.has_da_stake(&self.public_key).await,
                    debug!("We were not chosen for consensus committee for view {view_number} in epoch {epoch_number:?}")
                );
                let total_weight =
                    vid_total_weight::<TYPES>(membership.stake_table().await, epoch_number);

                let mut next_epoch_total_weight = total_weight;
                if epoch_number.is_some() {
                    next_epoch_total_weight = vid_total_weight::<TYPES>(
                        membership
                            .next_epoch_stake_table()
                            .await?
                            .stake_table()
                            .await,
                        epoch_number.map(|epoch| epoch + 1),
                    );
                }

                let version = self.upgrade_lock.version_infallible(view_number).await;

                let txns = Arc::clone(&proposal.data.encoded_transactions);
                let txns_clone = Arc::clone(&txns);
                let metadata = proposal.data.metadata.encode();
                let metadata_clone = metadata.clone();
                let payload_commitment = spawn_blocking(move || {
                    vid_commitment::<V>(&txns, &metadata, total_weight, version)
                })
                .await;
                let payload_commitment = payload_commitment.unwrap();
                let next_epoch_payload_commitment = if matches!(
                    proposal.data.epoch_transition_indicator,
                    EpochTransitionIndicator::InTransition
                ) && self
                    .upgrade_lock
                    .epochs_enabled(proposal.data.view_number())
                    .await
                {
                    let commit_result = spawn_blocking(move || {
                        vid_commitment::<V>(
                            &txns_clone,
                            &metadata_clone,
                            next_epoch_total_weight,
                            version,
                        )
                    })
                    .await;
                    Some(commit_result.unwrap())
                } else {
                    None
                };

                self.storage
                    .append_da2(proposal, payload_commitment)
                    .await
                    .wrap()
                    .context(error!("Failed to append DA proposal to storage"))?;
                // Generate and send vote
                let vote = DaVote2::create_signed_vote(
                    DaData2 {
                        payload_commit: payload_commitment,
                        next_epoch_payload_commit: next_epoch_payload_commitment,
                        epoch: epoch_number,
                    },
                    view_number,
                    &self.public_key,
                    &self.private_key,
                    &self.upgrade_lock,
                )
                .await?;

                tracing::debug!("Sending vote to the DA leader {:?}", vote.view_number());

                broadcast_event(Arc::new(HotShotEvent::DaVoteSend(vote)), &event_stream).await;
                let mut consensus_writer = self.consensus.write().await;

                // Ensure this view is in the view map for garbage collection.

                if let Err(e) =
                    consensus_writer.update_da_view(view_number, epoch_number, payload_commitment)
                {
                    tracing::trace!("{e:?}");
                }

                let payload_with_metadata = Arc::new(PayloadWithMetadata {
                    payload: TYPES::BlockPayload::from_bytes(
                        proposal.data.encoded_transactions.as_ref(),
                        &proposal.data.metadata,
                    ),
                    metadata: proposal.data.metadata.clone(),
                });

                // Record the payload we have promised to make available.
                if let Err(e) =
                    consensus_writer.update_saved_payloads(view_number, payload_with_metadata)
                {
                    tracing::trace!("{e:?}");
                }
                drop(consensus_writer);

                // Optimistically calculate and update VID if we know that the primary network is down.
                if self.network.is_primary_down() {
                    let my_id = self.id;
                    let consensus =
                        OuterConsensus::new(Arc::clone(&self.consensus.inner_consensus));
                    let pk = self.private_key.clone();
                    let public_key = self.public_key.clone();
                    let chan = event_stream.clone();
                    let upgrade_lock = self.upgrade_lock.clone();
                    let next_epoch = epoch_number.map(|epoch| epoch + 1);

                    let mut target_epochs = vec![];
                    if membership.has_stake(&public_key).await {
                        target_epochs.push(epoch_number);
                    }
                    if membership
                        .next_epoch_stake_table()
                        .await?
                        .has_stake(&public_key)
                        .await
                    {
                        target_epochs.push(next_epoch);
                    }
                    if target_epochs.is_empty() {
                        bail!("Not calculating VID, the node doesn't belong to the current epoch or the next epoch.");
                    };

                    tracing::debug!(
                        "Primary network is down. Optimistically calculate own VID share."
                    );
                    let membership = membership.clone();
                    spawn(async move {
                        for target_epoch in target_epochs {
                            Consensus::calculate_and_update_vid::<V>(
                                OuterConsensus::new(Arc::clone(&consensus.inner_consensus)),
                                view_number,
                                target_epoch,
                                membership.coordinator.clone(),
                                &pk,
                                &upgrade_lock,
                            )
                            .await;
                            if let Some(vid_share) = consensus
                                .read()
                                .await
                                .vid_shares()
                                .get(&view_number)
                                .and_then(|key_map| key_map.get(&public_key))
                                .and_then(|epoch_map| epoch_map.get(&target_epoch))
                            {
                                tracing::debug!(
                                    "Primary network is down. Calculated own VID share for epoch {:?}, my id {:?}",
                                    target_epoch,
                                    my_id
                                );
                                broadcast_event(
                                    Arc::new(HotShotEvent::VidShareRecv(
                                        public_key.clone(),
                                        vid_share.clone(),
                                    )),
                                    &chan,
                                )
                                .await;
                            }
                        }
                    });
                }
            },
            HotShotEvent::DaVoteRecv(ref vote) => {
                tracing::debug!("DA vote recv, Main Task {:?}", vote.view_number());
                // Check if we are the leader and the vote is from the sender.
                let view = vote.view_number();
                let epoch = vote.data.epoch;
                let membership = self
                    .membership_coordinator
                    .membership_for_epoch(epoch)
                    .await
                    .context(warn!("No stake table for epoch"))?;

                ensure!(
                    membership.leader(view).await? == self.public_key,
                    debug!(
                      "We are not the DA committee leader for view {} are we leader for next view? {}",
                      *view,
                      membership.leader(view + 1).await? == self.public_key
                    )
                );

                handle_vote(
                    &mut self.vote_collectors,
                    vote,
                    self.public_key.clone(),
                    &membership,
                    self.id,
                    &event,
                    &event_stream,
                    &self.upgrade_lock,
                    EpochTransitionIndicator::NotInTransition,
                )
                .await?;
            },
            HotShotEvent::ViewChange(view, epoch) => {
                if *epoch > self.cur_epoch {
                    self.cur_epoch = *epoch;
                }

                let view = *view;
                ensure!(
                    *self.cur_view < *view,
                    info!("Received a view change to an older view.")
                );

                if *view - *self.cur_view > 1 {
                    tracing::info!("View changed by more than 1 going to view {view:?}");
                }
                self.cur_view = view;
            },
            HotShotEvent::BlockRecv(packed_bundle) => {
                let PackedBundle::<TYPES> {
                    encoded_transactions,
                    metadata,
                    view_number,
                    ..
                } = packed_bundle;
                let view_number = *view_number;

                // quick hash the encoded txns with sha256
                let encoded_transactions_hash = Sha256::digest(encoded_transactions);

                // sign the encoded transactions as opposed to the VID commitment
                let signature =
                    TYPES::SignatureKey::sign(&self.private_key, &encoded_transactions_hash)
                        .wrap()?;

                let epoch = self.cur_epoch;
                let leader = self
                    .membership_coordinator
                    .membership_for_epoch(epoch)
                    .await
                    .context(warn!("No stake table for epoch"))?
                    .leader(view_number)
                    .await?;
                if leader != self.public_key {
                    tracing::debug!(
                        "We are not the leader in the current epoch. Do not send the DA proposal"
                    );
                    return Ok(());
                }
                let epoch_transition_indicator =
                    if self.consensus.read().await.is_high_qc_ge_root_block() {
                        EpochTransitionIndicator::InTransition
                    } else {
                        EpochTransitionIndicator::NotInTransition
                    };
                let data: DaProposal2<TYPES> = DaProposal2 {
                    encoded_transactions: Arc::clone(encoded_transactions),
                    metadata: metadata.clone(),
                    // Upon entering a new view we want to send a DA Proposal for the next view -> Is it always the case that this is cur_view + 1?
                    view_number,
                    epoch,
                    epoch_transition_indicator,
                };

                let message = Proposal {
                    data,
                    signature,
                    _pd: PhantomData,
                };

                broadcast_event(
                    Arc::new(HotShotEvent::DaProposalSend(
                        message.clone(),
                        self.public_key.clone(),
                    )),
                    &event_stream,
                )
                .await;
                let payload_with_metadata = Arc::new(PayloadWithMetadata {
                    payload: TYPES::BlockPayload::from_bytes(
                        encoded_transactions.as_ref(),
                        metadata,
                    ),
                    metadata: metadata.clone(),
                });
                // Save the payload early because we might need it to calculate VID for the next epoch nodes.
                if let Err(e) = self
                    .consensus
                    .write()
                    .await
                    .update_saved_payloads(view_number, payload_with_metadata)
                {
                    tracing::trace!("{e:?}");
                }
            },
            _ => {},
        }
        Ok(())
    }
}

#[async_trait]
/// task state implementation for DA Task
impl<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> TaskState
    for DaTaskState<TYPES, I, V>
{
    type Event = HotShotEvent<TYPES>;

    async fn handle_event(
        &mut self,
        event: Arc<Self::Event>,
        sender: &Sender<Arc<Self::Event>>,
        _receiver: &Receiver<Arc<Self::Event>>,
    ) -> Result<()> {
        self.handle(event, sender.clone()).await
    }

    fn cancel_subtasks(&mut self) {}
}
