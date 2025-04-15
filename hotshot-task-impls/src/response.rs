// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{sync::Arc, time::Duration};

use async_broadcast::{Receiver, Sender};
use committable::Committable;
use hotshot_types::{
    consensus::{Consensus, LockedConsensusState, OuterConsensus},
    data::VidDisperseShare,
    epoch_membership::EpochMembershipCoordinator,
    message::{Proposal, UpgradeLock},
    traits::{
        network::DataRequest,
        node_implementation::{NodeType, Versions},
        signature_key::SignatureKey,
    },
};
use sha2::{Digest, Sha256};
use tokio::{spawn, task::JoinHandle, time::sleep};
use tracing::instrument;

use crate::{events::HotShotEvent, helpers::broadcast_event};
/// Time to wait for txns before sending `ResponseMessage::NotFound`
const TXNS_TIMEOUT: Duration = Duration::from_millis(100);

/// Task state for the Network Request Task. The task is responsible for handling
/// requests sent to this node by the network.  It will validate the sender,
/// parse the request, and try to find the data request in the consensus stores.
pub struct NetworkResponseState<TYPES: NodeType, V: Versions> {
    /// Locked consensus state
    consensus: LockedConsensusState<TYPES>,

    /// Quorum membership for checking if requesters have state
    membership: EpochMembershipCoordinator<TYPES>,

    /// This replicas public key
    pub_key: TYPES::SignatureKey,

    /// This replicas private key
    private_key: <TYPES::SignatureKey as SignatureKey>::PrivateKey,

    /// The node's id
    id: u64,

    /// Lock for a decided upgrade
    upgrade_lock: UpgradeLock<TYPES, V>,
}

impl<TYPES: NodeType, V: Versions> NetworkResponseState<TYPES, V> {
    /// Create the network request state with the info it needs
    pub fn new(
        consensus: LockedConsensusState<TYPES>,
        membership: EpochMembershipCoordinator<TYPES>,
        pub_key: TYPES::SignatureKey,
        private_key: <TYPES::SignatureKey as SignatureKey>::PrivateKey,
        id: u64,
        upgrade_lock: UpgradeLock<TYPES, V>,
    ) -> Self {
        Self {
            consensus,
            membership,
            pub_key,
            private_key,
            id,
            upgrade_lock,
        }
    }

    /// Process request events or loop until a `HotShotEvent::Shutdown` is received.
    async fn run_response_loop(
        self,
        mut receiver: Receiver<Arc<HotShotEvent<TYPES>>>,
        event_sender: Sender<Arc<HotShotEvent<TYPES>>>,
    ) {
        loop {
            match receiver.recv_direct().await {
                Ok(event) => {
                    // break loop when false, this means shutdown received
                    match event.as_ref() {
                        HotShotEvent::VidRequestRecv(request, sender) => {
                            let cur_epoch = self.consensus.read().await.cur_epoch();
                            let next_epoch = cur_epoch.map(|epoch| epoch + 1);
                            let target_epoch = if self.valid_sender(sender, cur_epoch).await {
                                cur_epoch
                            } else if self.valid_sender(sender, next_epoch).await {
                                next_epoch
                            } else {
                                // The sender neither belongs to the current nor to the next epoch.
                                continue;
                            };
                            // Verify request is valid
                            if !valid_signature::<TYPES>(request, sender) {
                                continue;
                            }
                            if let Some(proposal) = self
                                .get_or_calc_vid_share(request.view, target_epoch, sender)
                                .await
                            {
                                broadcast_event(
                                    HotShotEvent::VidResponseSend(
                                        self.pub_key.clone(),
                                        sender.clone(),
                                        proposal,
                                    )
                                    .into(),
                                    &event_sender,
                                )
                                .await;
                            }
                        },
                        HotShotEvent::QuorumProposalRequestRecv(req, signature) => {
                            // Make sure that this request came from who we think it did
                            if !req.key.validate(signature, req.commit().as_ref()) {
                                tracing::warn!("Invalid signature key on proposal request.");
                                return;
                            }

                            let quorum_proposal_result = self
                                .consensus
                                .read()
                                .await
                                .last_proposals()
                                .get(&req.view_number)
                                .cloned();
                            if let Some(quorum_proposal) = quorum_proposal_result {
                                broadcast_event(
                                    HotShotEvent::QuorumProposalResponseSend(
                                        req.key.clone(),
                                        quorum_proposal,
                                    )
                                    .into(),
                                    &event_sender,
                                )
                                .await;
                            }
                        },
                        HotShotEvent::Shutdown => {
                            return;
                        },
                        _ => {},
                    }
                },
                Err(e) => {
                    tracing::error!("Failed to receive event: {e:?}");
                },
            }
        }
    }

    /// Get the VID share from consensus storage, or calculate it from the payload for
    /// the view, if we have the payload.  Stores all the shares calculated from the payload
    /// if the calculation was done
    #[instrument(skip_all, target = "NetworkResponseState", fields(id = self.id))]
    async fn get_or_calc_vid_share(
        &self,
        view: TYPES::View,
        target_epoch: Option<TYPES::Epoch>,
        key: &TYPES::SignatureKey,
    ) -> Option<Proposal<TYPES, VidDisperseShare<TYPES>>> {
        let consensus_reader = self.consensus.read().await;
        if let Some(view) = consensus_reader.vid_shares().get(&view) {
            if let Some(share) = view.get(key) {
                return Some(share.clone());
            }
        }

        drop(consensus_reader);

        if Consensus::calculate_and_update_vid::<V>(
            OuterConsensus::new(Arc::clone(&self.consensus)),
            view,
            target_epoch,
            self.membership.clone(),
            &self.private_key,
            &self.upgrade_lock,
        )
        .await
        .is_none()
        {
            // Sleep in hope we receive txns in the meantime
            sleep(TXNS_TIMEOUT).await;
            Consensus::calculate_and_update_vid::<V>(
                OuterConsensus::new(Arc::clone(&self.consensus)),
                view,
                target_epoch,
                self.membership.clone(),
                &self.private_key,
                &self.upgrade_lock,
            )
            .await?;
        }
        return self
            .consensus
            .read()
            .await
            .vid_shares()
            .get(&view)?
            .get(key)
            .cloned();
    }

    /// Makes sure the sender is allowed to send a request in the given epoch.
    async fn valid_sender(
        &self,
        sender: &TYPES::SignatureKey,
        epoch: Option<TYPES::Epoch>,
    ) -> bool {
        let Ok(memb) = self.membership.stake_table_for_epoch(epoch).await else {
            return false;
        };
        memb.has_stake(sender).await
    }
}

/// Check the signature
fn valid_signature<TYPES: NodeType>(
    req: &DataRequest<TYPES>,
    sender: &TYPES::SignatureKey,
) -> bool {
    let Ok(data) = bincode::serialize(&req.request) else {
        return false;
    };
    sender.validate(&req.signature, &Sha256::digest(data))
}

/// Spawn the network response task to handle incoming request for data
/// from other nodes.  It will shutdown when it gets `HotshotEvent::Shutdown`
/// on the `event_stream` arg.
pub fn run_response_task<TYPES: NodeType, V: Versions>(
    task_state: NetworkResponseState<TYPES, V>,
    event_stream: Receiver<Arc<HotShotEvent<TYPES>>>,
    sender: Sender<Arc<HotShotEvent<TYPES>>>,
) -> JoinHandle<()> {
    spawn(task_state.run_response_loop(event_stream, sender))
}
