// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use async_broadcast::{Receiver, Sender};
use async_trait::async_trait;
use hotshot_task::task::TaskState;
use hotshot_types::{
    consensus::OuterConsensus,
    epoch_membership::EpochMembershipCoordinator,
    simple_vote::HasEpoch,
    traits::{
        block_contents::BlockHeader,
        network::{ConnectedNetwork, DataRequest, RequestKind},
        node_implementation::{NodeImplementation, NodeType},
        signature_key::SignatureKey,
    },
    utils::is_epoch_transition,
    vote::HasViewNumber,
};
use hotshot_utils::anytrace::*;
use rand::{seq::SliceRandom, thread_rng};
use sha2::{Digest, Sha256};
use tokio::{spawn, task::JoinHandle, time::sleep};
use tracing::instrument;

use crate::{events::HotShotEvent, helpers::broadcast_event};

/// Amount of time to try for a request before timing out.
pub const REQUEST_TIMEOUT: Duration = Duration::from_millis(500);

/// Long running task which will request information after a proposal is received.
/// The task will wait a it's `delay` and then send a request iteratively to peers
/// for any data they don't have related to the proposal.  For now it's just requesting VID
/// shares.
pub struct NetworkRequestState<TYPES: NodeType, I: NodeImplementation<TYPES>> {
    /// Network to send requests over
    /// The underlying network
    pub network: Arc<I::Network>,

    /// Consensus shared state so we can check if we've gotten the information
    /// before sending a request
    pub consensus: OuterConsensus<TYPES>,

    /// Last seen view, we won't request for proposals before older than this view
    pub view: TYPES::View,

    /// Delay before requesting peers
    pub delay: Duration,

    /// Membership (Used here only for DA)
    pub membership_coordinator: EpochMembershipCoordinator<TYPES>,

    /// This nodes public key
    pub public_key: TYPES::SignatureKey,

    /// This nodes private/signing key, used to sign requests.
    pub private_key: <TYPES::SignatureKey as SignatureKey>::PrivateKey,

    /// The node's id
    pub id: u64,

    /// A flag indicating that `HotShotEvent::Shutdown` has been received
    pub shutdown_flag: Arc<AtomicBool>,

    /// A flag indicating that `HotShotEvent::Shutdown` has been received
    pub spawned_tasks: BTreeMap<TYPES::View, Vec<JoinHandle<()>>>,

    /// Number of blocks in an epoch, zero means there are no epochs
    pub epoch_height: u64,
}

impl<TYPES: NodeType, I: NodeImplementation<TYPES>> Drop for NetworkRequestState<TYPES, I> {
    fn drop(&mut self) {
        self.cancel_subtasks();
    }
}

/// Alias for a signature
type Signature<TYPES> =
    <<TYPES as NodeType>::SignatureKey as SignatureKey>::PureAssembledSignatureType;

#[async_trait]
impl<TYPES: NodeType, I: NodeImplementation<TYPES>> TaskState for NetworkRequestState<TYPES, I> {
    type Event = HotShotEvent<TYPES>;

    #[instrument(skip_all, fields(id = self.id), name = "NetworkRequestState")]
    async fn handle_event(
        &mut self,
        event: Arc<Self::Event>,
        sender: &Sender<Arc<Self::Event>>,
        _receiver: &Receiver<Arc<Self::Event>>,
    ) -> Result<()> {
        match event.as_ref() {
            HotShotEvent::QuorumProposalValidated(proposal, _) => {
                let prop_view = proposal.data.view_number();
                let prop_epoch = proposal.data.epoch();

                // Request VID share only if:
                // 1. we are part of the current epoch or
                // 2. we are part of the next epoch and this is a proposal for in transition.
                let membership = self
                    .membership_coordinator
                    .stake_table_for_epoch(prop_epoch)
                    .await?;
                let mut target_epochs = BTreeSet::new();
                if membership.has_stake(&self.public_key).await {
                    target_epochs.insert(prop_epoch);
                }
                if is_epoch_transition(
                    proposal.data.block_header().block_number(),
                    self.epoch_height,
                ) && membership
                    .next_epoch_stake_table()
                    .await?
                    .has_stake(&self.public_key)
                    .await
                {
                    target_epochs.insert(prop_epoch.map(|e| e + 1));
                }

                ensure!(
                    !target_epochs.is_empty(),
                    "We don't belong to the current epoch and \
                we don't belong to the next epoch. Do not request VID share."
                );

                let consensus_reader = self.consensus.read().await;
                let maybe_vid_share = consensus_reader
                    .vid_shares()
                    .get(&prop_view)
                    .and_then(|shares| shares.get(&self.public_key));
                // If we already have the VID shares for the next view, do nothing.
                if prop_view >= self.view
                    && (maybe_vid_share.is_none()
                        || !target_epochs
                            .iter()
                            .all(|e| maybe_vid_share.unwrap().contains_key(e)))
                {
                    drop(consensus_reader);
                    self.spawn_requests(prop_view, prop_epoch, sender, target_epochs)
                        .await;
                }
                Ok(())
            },
            HotShotEvent::VidResponseRecv(sender_key, vid_proposal) => {
                let view = vid_proposal.data.view_number();
                let epoch = vid_proposal.data.epoch();

                // Get the committee members for the view and the leader, if applicable
                let membership_reader = self
                    .membership_coordinator
                    .membership_for_epoch(epoch)
                    .await?;
                let mut da_committee_for_view = membership_reader.da_committee_members(view).await;
                if let Ok(leader) = membership_reader.leader(view).await {
                    da_committee_for_view.insert(leader);
                }
                drop(membership_reader);

                ensure!(
                    self.spawned_tasks.contains_key(&view),
                    info!(
                        "Received VidResponseRecv for view we didn't expect, view {:?}",
                        view
                    )
                );

                ensure!(
                    da_committee_for_view.contains(sender_key),
                    warn!(
                        "Received VidResponseRecv from unexpected sender key {:?}",
                        sender_key
                    )
                );

                ensure!(
                    sender_key.validate(
                        &vid_proposal.signature,
                        vid_proposal.data.payload_commitment_ref()
                    ),
                    warn!("Received VidResponseRecv with invalid signature")
                );

                tracing::debug!("Received VidResponseRecv {:?}", vid_proposal,);
                broadcast_event(
                    Arc::new(HotShotEvent::VidShareRecv(
                        sender_key.clone(),
                        vid_proposal.clone(),
                    )),
                    sender,
                )
                .await;
                Ok(())
            },
            HotShotEvent::ViewChange(view, _) => {
                let view = *view;
                if view > self.view {
                    self.view = view;
                }
                Ok(())
            },
            _ => Ok(()),
        }
    }

    fn cancel_subtasks(&mut self) {
        self.shutdown_flag.store(true, Ordering::Relaxed);

        while !self.spawned_tasks.is_empty() {
            let Some((_, handles)) = self.spawned_tasks.pop_first() else {
                break;
            };

            for handle in handles {
                handle.abort();
            }
        }
    }
}

impl<TYPES: NodeType, I: NodeImplementation<TYPES>> NetworkRequestState<TYPES, I> {
    /// Creates and signs the payload, then will create a request task
    async fn spawn_requests(
        &mut self,
        view: TYPES::View,
        prop_epoch: Option<TYPES::Epoch>,
        sender: &Sender<Arc<HotShotEvent<TYPES>>>,
        target_epochs: BTreeSet<Option<TYPES::Epoch>>,
    ) {
        let request = RequestKind::Vid(view, self.public_key.clone());

        // First sign the request for the VID shares.
        if let Some(signature) = self.serialize_and_sign(&request) {
            self.create_vid_request_task(
                request,
                signature,
                sender.clone(),
                view,
                prop_epoch,
                target_epochs,
            )
            .await;
        }
    }

    /// Creates a task that will request a VID share from a DA member and wait for the `HotShotEvent::VidResponseRecv`event
    /// If we get the VID disperse share, broadcast `HotShotEvent::VidShareRecv` and terminate task
    async fn create_vid_request_task(
        &mut self,
        request: RequestKind<TYPES>,
        signature: Signature<TYPES>,
        sender: Sender<Arc<HotShotEvent<TYPES>>>,
        view: TYPES::View,
        prop_epoch: Option<TYPES::Epoch>,
        mut target_epochs: BTreeSet<Option<TYPES::Epoch>>,
    ) {
        let consensus = OuterConsensus::new(Arc::clone(&self.consensus.inner_consensus));
        let network = Arc::clone(&self.network);
        let shutdown_flag = Arc::clone(&self.shutdown_flag);
        let delay = self.delay;
        let public_key = self.public_key.clone();

        // Get the committee members for the view and the leader, if applicable
        let membership_reader = match self
            .membership_coordinator
            .membership_for_epoch(prop_epoch)
            .await
        {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(e.message);
                return;
            },
        };
        // Get committee members for view
        let mut recipients: Vec<TYPES::SignatureKey> = membership_reader
            .da_committee_members(view)
            .await
            .into_iter()
            .collect();

        // Randomize the recipients so all replicas don't overload the same 1 recipients
        // and so we don't implicitly rely on the same replica all the time.
        recipients.shuffle(&mut thread_rng());

        // prepare request
        let data_request = DataRequest::<TYPES> {
            request,
            view,
            signature,
        };
        let my_id = self.id;
        let handle: JoinHandle<()> = spawn(async move {
            // Do the delay only if primary is up and then start sending
            if !network.is_primary_down() {
                sleep(delay).await;
            }

            let mut recipients_it = recipients.iter();
            // First check if we got the data before continuing
            while !Self::cancel_vid_request_task(
                &consensus,
                &sender,
                &public_key,
                &view,
                &shutdown_flag,
                my_id,
                &mut target_epochs,
            )
            .await
            {
                // Cycle da members we send the request to each time
                if let Some(recipient) = recipients_it.next() {
                    if *recipient == public_key {
                        // no need to send a message to ourselves.
                        // just check for the data at start of loop in `cancel_vid_request_task`
                        continue;
                    }
                    tracing::debug!(
                        "Sending VidRequestSend {:?}, my id {:?}",
                        data_request,
                        my_id
                    );
                    // First send request to a random DA member for the view
                    broadcast_event(
                        HotShotEvent::VidRequestSend(
                            data_request.clone(),
                            public_key.clone(),
                            recipient.clone(),
                        )
                        .into(),
                        &sender,
                    )
                    .await;
                    // Wait before sending the request to the next recipient.
                    sleep(REQUEST_TIMEOUT).await;
                } else {
                    // This shouldn't be possible `recipients_it.next()` should clone original and start over if `None`
                    tracing::warn!("Sent VID request to all available DA members and got no response for view: {view:?}, my id: {my_id:?}");
                    return;
                }
            }
        });
        self.spawned_tasks.entry(view).or_default().push(handle);
    }

    /// Returns true if we got the data we wanted, a shutdown event was received, or the view has moved on.
    async fn cancel_vid_request_task(
        consensus: &OuterConsensus<TYPES>,
        sender: &Sender<Arc<HotShotEvent<TYPES>>>,
        public_key: &<TYPES as NodeType>::SignatureKey,
        view: &TYPES::View,
        shutdown_flag: &Arc<AtomicBool>,
        id: u64,
        target_epochs: &mut BTreeSet<Option<TYPES::Epoch>>,
    ) -> bool {
        let consensus_reader = consensus.read().await;

        let maybe_vid_shares = consensus_reader
            .vid_shares()
            .get(view)
            .and_then(|key_map| key_map.get(public_key));
        if let Some(vid_shares) = maybe_vid_shares {
            tracing::debug!("Send own vid share: {:?}, my id {:?}", vid_shares, id,);
            for vid_share in vid_shares.values() {
                broadcast_event(
                    Arc::new(HotShotEvent::VidShareRecv(
                        public_key.clone(),
                        vid_share.clone(),
                    )),
                    sender,
                )
                .await;
                target_epochs.remove(&vid_share.data.target_epoch());
            }
        }
        let cancel = shutdown_flag.load(Ordering::Relaxed)
            || consensus_reader.cur_view() > *view
            || target_epochs.is_empty();
        if cancel {
            tracing::debug!(
                "Canceling vid request for view {:?}, cur view is {:?}, my id {:?}",
                view,
                consensus_reader.cur_view(),
                id,
            );
        }
        cancel
    }

    /// Sign the serialized version of the request
    fn serialize_and_sign(&self, request: &RequestKind<TYPES>) -> Option<Signature<TYPES>> {
        let Ok(data) = bincode::serialize(&request) else {
            tracing::error!("Failed to serialize request!");
            return None;
        };
        let Ok(signature) = TYPES::SignatureKey::sign(&self.private_key, &Sha256::digest(data))
        else {
            tracing::error!("Failed to sign Data Request");
            return None;
        };
        Some(signature)
    }
}
