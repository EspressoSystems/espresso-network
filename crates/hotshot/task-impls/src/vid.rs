// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{marker::PhantomData, sync::Arc};

use async_broadcast::{Receiver, Sender};
use async_trait::async_trait;
use hotshot_task::task::TaskState;
use hotshot_types::{
    consensus::{OuterConsensus, PayloadWithMetadata},
    data::{PackedBundle, VidDisperse, VidDisperseShare},
    epoch_membership::EpochMembershipCoordinator,
    message::{Proposal, UpgradeLock},
    simple_vote::HasEpoch,
    traits::{
        block_contents::BlockHeader,
        node_implementation::{NodeImplementation, NodeType, Versions},
        signature_key::SignatureKey,
        BlockPayload,
    },
    utils::{is_epoch_transition, option_epoch_from_block_number},
};
use hotshot_utils::anytrace::Result;
use tracing::{debug, error, info, instrument};

use crate::{
    events::{HotShotEvent, HotShotTaskCompleted},
    helpers::broadcast_event,
};

/// Tracks state of a VID task
pub struct VidTaskState<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> {
    /// View number this view is executing in.
    pub cur_view: TYPES::View,

    /// Epoch number this node is executing in.
    pub cur_epoch: Option<TYPES::Epoch>,

    /// Reference to consensus. Leader will require a read lock on this.
    pub consensus: OuterConsensus<TYPES>,

    /// The underlying network
    pub network: Arc<I::Network>,

    /// Membership for the quorum
    pub membership_coordinator: EpochMembershipCoordinator<TYPES>,

    /// This Nodes Public Key
    pub public_key: TYPES::SignatureKey,

    /// Our Private Key
    pub private_key: <TYPES::SignatureKey as SignatureKey>::PrivateKey,

    /// This state's ID
    pub id: u64,

    /// Lock for a decided upgrade
    pub upgrade_lock: UpgradeLock<TYPES, V>,

    /// Number of blocks in an epoch, zero means there are no epochs
    pub epoch_height: u64,
}

impl<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> VidTaskState<TYPES, I, V> {
    /// main task event handler
    #[instrument(skip_all, fields(id = self.id, view = *self.cur_view, epoch = self.cur_epoch.map(|x| *x)), name = "VID Main Task", level = "error", target = "VidTaskState")]
    pub async fn handle(
        &mut self,
        event: Arc<HotShotEvent<TYPES>>,
        event_stream: Sender<Arc<HotShotEvent<TYPES>>>,
    ) -> Option<HotShotTaskCompleted> {
        match event.as_ref() {
            HotShotEvent::BlockRecv(packed_bundle) => {
                let PackedBundle::<TYPES> {
                    encoded_transactions,
                    metadata,
                    view_number,
                    sequencing_fees,
                    auction_result,
                    ..
                } = packed_bundle;
                let payload =
                    <TYPES as NodeType>::BlockPayload::from_bytes(encoded_transactions, metadata);
                let builder_commitment = payload.builder_commitment(metadata);
                let epoch = self.cur_epoch;
                if self
                    .membership_coordinator
                    .membership_for_epoch(epoch)
                    .await
                    .ok()?
                    .leader(*view_number)
                    .await
                    .ok()?
                    != self.public_key
                {
                    tracing::debug!(
                        "We are not the leader in the current epoch. Do not send the VID dispersal."
                    );
                    return None;
                }
                let vid_disperse = VidDisperse::calculate_vid_disperse::<V>(
                    &payload,
                    &self.membership_coordinator,
                    *view_number,
                    epoch,
                    epoch,
                    metadata,
                    &self.upgrade_lock,
                )
                .await
                .ok()?;
                let payload_commitment = vid_disperse.payload_commitment();
                let shares = VidDisperseShare::from_vid_disperse(vid_disperse.clone());
                let payload_with_metadata = Arc::new(PayloadWithMetadata {
                    payload,
                    metadata: metadata.clone(),
                });

                let mut consensus_writer = self.consensus.write().await;
                // Make sure we save the payload; we might need it to send the next epoch VID shares.
                if let Err(e) =
                    consensus_writer.update_saved_payloads(*view_number, payload_with_metadata)
                {
                    tracing::debug!(error=?e);
                }
                for share in shares {
                    if let Some(share) = share.to_proposal(&self.private_key) {
                        consensus_writer.update_vid_shares(*view_number, share);
                    }
                }
                drop(consensus_writer);

                // send the commitment and metadata to consensus for block building
                broadcast_event(
                    Arc::new(HotShotEvent::SendPayloadCommitmentAndMetadata(
                        payload_commitment,
                        builder_commitment,
                        metadata.clone(),
                        *view_number,
                        sequencing_fees.clone(),
                        auction_result.clone(),
                    )),
                    &event_stream,
                )
                .await;

                let view_number = *view_number;
                let Ok(signature) = TYPES::SignatureKey::sign(
                    &self.private_key,
                    vid_disperse.payload_commitment_ref(),
                ) else {
                    error!("VID: failed to sign dispersal payload");
                    return None;
                };
                debug!("publishing VID disperse for view {view_number} and epoch {epoch:?}");
                broadcast_event(
                    Arc::new(HotShotEvent::VidDisperseSend(
                        Proposal {
                            signature,
                            data: vid_disperse,
                            _pd: PhantomData,
                        },
                        self.public_key.clone(),
                    )),
                    &event_stream,
                )
                .await;
            },

            HotShotEvent::ViewChange(view, epoch) => {
                if *epoch > self.cur_epoch {
                    self.cur_epoch = *epoch;
                }

                let view = *view;
                if (*view != 0 || *self.cur_view > 0) && *self.cur_view >= *view {
                    return None;
                }

                if *view - *self.cur_view > 1 {
                    info!("View changed by more than 1 going to view {view:?}");
                }
                self.cur_view = view;

                return None;
            },

            HotShotEvent::QuorumProposalSend(proposal, _) => {
                let proposed_block_number = proposal.data.block_header().block_number();
                if proposal.data.epoch().is_none()
                    || !is_epoch_transition(proposed_block_number, self.epoch_height)
                {
                    // This is not the last block in the epoch, do nothing.
                    return None;
                }
                // We just sent a proposal for the last block in the epoch. We need to calculate
                // and send VID for the nodes in the next epoch so that they can vote.
                let proposal_view_number = proposal.data.view_number();
                let sender_epoch = option_epoch_from_block_number::<TYPES>(
                    true,
                    proposed_block_number,
                    self.epoch_height,
                );
                let target_epoch = sender_epoch.map(|x| x + 1);

                let consensus_reader = self.consensus.read().await;
                let Some(payload) = consensus_reader.saved_payloads().get(&proposal_view_number)
                else {
                    tracing::warn!(
                        "We need to calculate VID for the nodes in the next epoch \
                         but we don't have the transactions"
                    );
                    return None;
                };
                let payload = Arc::clone(payload);
                drop(consensus_reader);

                let next_epoch_vid_disperse = VidDisperse::calculate_vid_disperse::<V>(
                    &payload.payload,
                    &self.membership_coordinator,
                    proposal_view_number,
                    target_epoch,
                    sender_epoch,
                    &payload.metadata,
                    &self.upgrade_lock,
                )
                .await
                .ok()?;
                let Ok(next_epoch_signature) = TYPES::SignatureKey::sign(
                    &self.private_key,
                    next_epoch_vid_disperse.payload_commitment().as_ref(),
                ) else {
                    error!("VID: failed to sign dispersal payload for the next epoch");
                    return None;
                };
                debug!(
                    "publishing VID disperse for view {proposal_view_number} and epoch {target_epoch:?}"
                );
                broadcast_event(
                    Arc::new(HotShotEvent::VidDisperseSend(
                        Proposal {
                            signature: next_epoch_signature,
                            data: next_epoch_vid_disperse.clone(),
                            _pd: PhantomData,
                        },
                        self.public_key.clone(),
                    )),
                    &event_stream,
                )
                .await;
            },
            HotShotEvent::Shutdown => {
                return Some(HotShotTaskCompleted);
            },
            _ => {},
        }
        None
    }
}

#[async_trait]
/// task state implementation for VID Task
impl<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> TaskState
    for VidTaskState<TYPES, I, V>
{
    type Event = HotShotEvent<TYPES>;

    async fn handle_event(
        &mut self,
        event: Arc<Self::Event>,
        sender: &Sender<Arc<Self::Event>>,
        _receiver: &Receiver<Arc<Self::Event>>,
    ) -> Result<()> {
        self.handle(event, sender.clone()).await;
        Ok(())
    }

    fn cancel_subtasks(&mut self) {}
}
