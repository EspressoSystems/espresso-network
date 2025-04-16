// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{
    collections::{BTreeMap, HashMap},
    hash::{DefaultHasher, Hash, Hasher},
    sync::Arc,
};

use async_broadcast::{Receiver, Sender};
use async_lock::RwLock;
use async_trait::async_trait;
use hotshot_task::task::TaskState;
use hotshot_types::{
    consensus::OuterConsensus,
    data::{VidDisperse, VidDisperseShare},
    epoch_membership::EpochMembershipCoordinator,
    event::{Event, EventType, HotShotAction},
    message::{
        convert_proposal, DaConsensusMessage, DataMessage, GeneralConsensusMessage, Message,
        MessageKind, Proposal, SequencingMessage, UpgradeLock,
    },
    simple_vote::HasEpoch,
    traits::{
        network::{
            BroadcastDelay, ConnectedNetwork, RequestKind, ResponseMessage, Topic, TransmitType,
            ViewMessage,
        },
        node_implementation::{ConsensusTime, NodeType, Versions},
        storage::Storage,
    },
    vote::{HasViewNumber, Vote},
};
use hotshot_utils::anytrace::*;
use tokio::{spawn, task::JoinHandle};
use tracing::instrument;

use crate::{
    events::{HotShotEvent, HotShotTaskCompleted},
    helpers::broadcast_event,
};

/// the network message task state
#[derive(Clone)]
pub struct NetworkMessageTaskState<TYPES: NodeType, V: Versions> {
    /// Sender to send internal events this task generates to other tasks
    pub internal_event_stream: Sender<Arc<HotShotEvent<TYPES>>>,

    /// Sender to send external events this task generates to the event stream
    pub external_event_stream: Sender<Event<TYPES>>,

    /// This nodes public key
    pub public_key: TYPES::SignatureKey,

    /// Transaction Cache to ignore previously seen transactions
    pub transactions_cache: lru::LruCache<u64, ()>,

    /// Lock for a decided upgrade
    pub upgrade_lock: UpgradeLock<TYPES, V>,

    /// Node's id
    pub id: u64,
}

impl<TYPES: NodeType, V: Versions> NetworkMessageTaskState<TYPES, V> {
    #[instrument(skip_all, name = "Network message task", fields(id = self.id), level = "trace")]
    /// Handles a (deserialized) message from the network
    pub async fn handle_message(&mut self, message: Message<TYPES>) {
        match &message.kind {
            MessageKind::Consensus(_) => {
                tracing::debug!("Received consensus message from network: {:?}", message)
            },
            MessageKind::Data(_) => {
                tracing::trace!("Received data message from network: {:?}", message)
            },
            MessageKind::External(_) => {
                tracing::trace!("Received external message from network: {:?}", message)
            },
        }

        // Match the message kind and send the appropriate event to the internal event stream
        let sender = message.sender;
        match message.kind {
            // Handle consensus messages
            MessageKind::Consensus(consensus_message) => {
                let event = match consensus_message {
                    SequencingMessage::General(general_message) => match general_message {
                        GeneralConsensusMessage::Proposal(proposal) => {
                            if self
                                .upgrade_lock
                                .epochs_enabled(proposal.data.view_number())
                                .await
                            {
                                tracing::warn!("received GeneralConsensusMessage::Proposal for view {} but epochs are enabled for that view", proposal.data.view_number());
                                return;
                            }
                            HotShotEvent::QuorumProposalRecv(convert_proposal(proposal), sender)
                        },
                        GeneralConsensusMessage::Proposal2(proposal) => {
                            if !self
                                .upgrade_lock
                                .epochs_enabled(proposal.data.view_number())
                                .await
                            {
                                tracing::warn!("received GeneralConsensusMessage::Proposal2 for view {} but epochs are not enabled for that view", proposal.data.view_number());
                                return;
                            }
                            HotShotEvent::QuorumProposalRecv(convert_proposal(proposal), sender)
                        },
                        GeneralConsensusMessage::ProposalRequested(req, sig) => {
                            HotShotEvent::QuorumProposalRequestRecv(req, sig)
                        },
                        GeneralConsensusMessage::ProposalResponse(proposal) => {
                            if self
                                .upgrade_lock
                                .epochs_enabled(proposal.data.view_number())
                                .await
                            {
                                tracing::warn!("received GeneralConsensusMessage::ProposalResponse for view {} but epochs are enabled for that view", proposal.data.view_number());
                                return;
                            }
                            HotShotEvent::QuorumProposalResponseRecv(convert_proposal(proposal))
                        },
                        GeneralConsensusMessage::ProposalResponse2(proposal) => {
                            if !self
                                .upgrade_lock
                                .epochs_enabled(proposal.data.view_number())
                                .await
                            {
                                tracing::warn!("received GeneralConsensusMessage::ProposalResponse2 for view {} but epochs are not enabled for that view", proposal.data.view_number());
                                return;
                            }
                            HotShotEvent::QuorumProposalResponseRecv(convert_proposal(proposal))
                        },
                        GeneralConsensusMessage::Vote(vote) => {
                            if self.upgrade_lock.epochs_enabled(vote.view_number()).await {
                                tracing::warn!("received GeneralConsensusMessage::Vote for view {} but epochs are enabled for that view", vote.view_number());
                                return;
                            }
                            HotShotEvent::QuorumVoteRecv(vote.to_vote2())
                        },
                        GeneralConsensusMessage::Vote2(vote) => {
                            if !self.upgrade_lock.epochs_enabled(vote.view_number()).await {
                                tracing::warn!("received GeneralConsensusMessage::Vote2 for view {} but epochs are not enabled for that view", vote.view_number());
                                return;
                            }
                            HotShotEvent::QuorumVoteRecv(vote)
                        },
                        GeneralConsensusMessage::ViewSyncPreCommitVote(view_sync_message) => {
                            if self
                                .upgrade_lock
                                .epochs_enabled(view_sync_message.view_number())
                                .await
                            {
                                tracing::warn!("received GeneralConsensusMessage::ViewSyncPreCommitVote for view {} but epochs are enabled for that view", view_sync_message.view_number());
                                return;
                            }
                            HotShotEvent::ViewSyncPreCommitVoteRecv(view_sync_message.to_vote2())
                        },
                        GeneralConsensusMessage::ViewSyncPreCommitVote2(view_sync_message) => {
                            if !self
                                .upgrade_lock
                                .epochs_enabled(view_sync_message.view_number())
                                .await
                            {
                                tracing::warn!("received GeneralConsensusMessage::ViewSyncPreCommitVote2 for view {} but epochs are not enabled for that view", view_sync_message.view_number());
                                return;
                            }
                            HotShotEvent::ViewSyncPreCommitVoteRecv(view_sync_message)
                        },
                        GeneralConsensusMessage::ViewSyncPreCommitCertificate(
                            view_sync_message,
                        ) => {
                            if self
                                .upgrade_lock
                                .epochs_enabled(view_sync_message.view_number())
                                .await
                            {
                                tracing::warn!("received GeneralConsensusMessage::ViewSyncPreCommitCertificate for view {} but epochs are enabled for that view", view_sync_message.view_number());
                                return;
                            }
                            HotShotEvent::ViewSyncPreCommitCertificateRecv(
                                view_sync_message.to_vsc2(),
                            )
                        },
                        GeneralConsensusMessage::ViewSyncPreCommitCertificate2(
                            view_sync_message,
                        ) => {
                            if !self
                                .upgrade_lock
                                .epochs_enabled(view_sync_message.view_number())
                                .await
                            {
                                tracing::warn!("received GeneralConsensusMessage::ViewSyncPreCommitCertificate2 for view {} but epochs are not enabled for that view", view_sync_message.view_number());
                                return;
                            }
                            HotShotEvent::ViewSyncPreCommitCertificateRecv(view_sync_message)
                        },
                        GeneralConsensusMessage::ViewSyncCommitVote(view_sync_message) => {
                            if self
                                .upgrade_lock
                                .epochs_enabled(view_sync_message.view_number())
                                .await
                            {
                                tracing::warn!("received GeneralConsensusMessage::ViewSyncCommitVote for view {} but epochs are enabled for that view", view_sync_message.view_number());
                                return;
                            }
                            HotShotEvent::ViewSyncCommitVoteRecv(view_sync_message.to_vote2())
                        },
                        GeneralConsensusMessage::ViewSyncCommitVote2(view_sync_message) => {
                            if !self
                                .upgrade_lock
                                .epochs_enabled(view_sync_message.view_number())
                                .await
                            {
                                tracing::warn!("received GeneralConsensusMessage::ViewSyncCommitVote2 for view {} but epochs are not enabled for that view", view_sync_message.view_number());
                                return;
                            }
                            HotShotEvent::ViewSyncCommitVoteRecv(view_sync_message)
                        },
                        GeneralConsensusMessage::ViewSyncCommitCertificate(view_sync_message) => {
                            if self
                                .upgrade_lock
                                .epochs_enabled(view_sync_message.view_number())
                                .await
                            {
                                tracing::warn!("received GeneralConsensusMessage::ViewSyncCommitCertificate for view {} but epochs are enabled for that view", view_sync_message.view_number());
                                return;
                            }
                            HotShotEvent::ViewSyncCommitCertificateRecv(view_sync_message.to_vsc2())
                        },
                        GeneralConsensusMessage::ViewSyncCommitCertificate2(view_sync_message) => {
                            if !self
                                .upgrade_lock
                                .epochs_enabled(view_sync_message.view_number())
                                .await
                            {
                                tracing::warn!("received GeneralConsensusMessage::ViewSyncCommitCertificate2 for view {} but epochs are not enabled for that view", view_sync_message.view_number());
                                return;
                            }
                            HotShotEvent::ViewSyncCommitCertificateRecv(view_sync_message)
                        },
                        GeneralConsensusMessage::ViewSyncFinalizeVote(view_sync_message) => {
                            if self
                                .upgrade_lock
                                .epochs_enabled(view_sync_message.view_number())
                                .await
                            {
                                tracing::warn!("received GeneralConsensusMessage::ViewSyncFinalizeVote for view {} but epochs are enabled for that view", view_sync_message.view_number());
                                return;
                            }
                            HotShotEvent::ViewSyncFinalizeVoteRecv(view_sync_message.to_vote2())
                        },
                        GeneralConsensusMessage::ViewSyncFinalizeVote2(view_sync_message) => {
                            if !self
                                .upgrade_lock
                                .epochs_enabled(view_sync_message.view_number())
                                .await
                            {
                                tracing::warn!("received GeneralConsensusMessage::ViewSyncFinalizeVote2 for view {} but epochs are not enabled for that view", view_sync_message.view_number());
                                return;
                            }
                            HotShotEvent::ViewSyncFinalizeVoteRecv(view_sync_message)
                        },
                        GeneralConsensusMessage::ViewSyncFinalizeCertificate(view_sync_message) => {
                            if self
                                .upgrade_lock
                                .epochs_enabled(view_sync_message.view_number())
                                .await
                            {
                                tracing::warn!("received GeneralConsensusMessage::ViewSyncFinalizeCertificate for view {} but epochs are enabled for that view", view_sync_message.view_number());
                                return;
                            }
                            HotShotEvent::ViewSyncFinalizeCertificateRecv(
                                view_sync_message.to_vsc2(),
                            )
                        },
                        GeneralConsensusMessage::ViewSyncFinalizeCertificate2(
                            view_sync_message,
                        ) => {
                            if !self
                                .upgrade_lock
                                .epochs_enabled(view_sync_message.view_number())
                                .await
                            {
                                tracing::warn!("received GeneralConsensusMessage::ViewSyncFinalizeCertificate2 for view {} but epochs are not enabled for that view", view_sync_message.view_number());
                                return;
                            }
                            HotShotEvent::ViewSyncFinalizeCertificateRecv(view_sync_message)
                        },
                        GeneralConsensusMessage::TimeoutVote(message) => {
                            if self
                                .upgrade_lock
                                .epochs_enabled(message.view_number())
                                .await
                            {
                                tracing::warn!("received GeneralConsensusMessage::TimeoutVote for view {} but epochs are enabled for that view", message.view_number());
                                return;
                            }
                            HotShotEvent::TimeoutVoteRecv(message.to_vote2())
                        },
                        GeneralConsensusMessage::TimeoutVote2(message) => {
                            if !self
                                .upgrade_lock
                                .epochs_enabled(message.view_number())
                                .await
                            {
                                tracing::warn!("received GeneralConsensusMessage::TimeoutVote2 for view {} but epochs are not enabled for that view", message.view_number());
                                return;
                            }
                            HotShotEvent::TimeoutVoteRecv(message)
                        },
                        GeneralConsensusMessage::UpgradeProposal(message) => {
                            HotShotEvent::UpgradeProposalRecv(message, sender)
                        },
                        GeneralConsensusMessage::UpgradeVote(message) => {
                            tracing::error!("Received upgrade vote!");
                            HotShotEvent::UpgradeVoteRecv(message)
                        },
                        GeneralConsensusMessage::HighQc(qc, next_qc) => {
                            HotShotEvent::HighQcRecv(qc, next_qc, sender)
                        },
                        GeneralConsensusMessage::ExtendedQc(qc, next_epoch_qc) => {
                            HotShotEvent::ExtendedQcRecv(qc, next_epoch_qc, sender)
                        },
                        GeneralConsensusMessage::EpochRootQuorumVote(vote) => {
                            if !self.upgrade_lock.epochs_enabled(vote.view_number()).await {
                                tracing::warn!("received GeneralConsensusMessage::EpochRootVote for view {} but epochs are not enabled for that view", vote.view_number());
                                return;
                            }
                            HotShotEvent::EpochRootQuorumVoteRecv(vote)
                        },
                        GeneralConsensusMessage::EpochRootQc(root_qc) => {
                            if !self
                                .upgrade_lock
                                .epochs_enabled(root_qc.view_number())
                                .await
                            {
                                tracing::warn!("received GeneralConsensusMessage::EpochRootQc for view {} but epochs are not enabled for that view", root_qc.view_number());
                                return;
                            }
                            HotShotEvent::EpochRootQcRecv(root_qc, sender)
                        },
                    },
                    SequencingMessage::Da(da_message) => match da_message {
                        DaConsensusMessage::DaProposal(proposal) => {
                            if self
                                .upgrade_lock
                                .epochs_enabled(proposal.data.view_number())
                                .await
                            {
                                tracing::warn!("received DaConsensusMessage::DaProposal for view {} but epochs are enabled for that view", proposal.data.view_number());
                                return;
                            }
                            HotShotEvent::DaProposalRecv(convert_proposal(proposal), sender)
                        },
                        DaConsensusMessage::DaProposal2(proposal) => {
                            if !self
                                .upgrade_lock
                                .epochs_enabled(proposal.data.view_number())
                                .await
                            {
                                tracing::warn!("received DaConsensusMessage::DaProposal2 for view {} but epochs are not enabled for that view", proposal.data.view_number());
                                return;
                            }
                            HotShotEvent::DaProposalRecv(proposal, sender)
                        },
                        DaConsensusMessage::DaVote(vote) => {
                            if self.upgrade_lock.epochs_enabled(vote.view_number()).await {
                                tracing::warn!("received DaConsensusMessage::DaVote for view {} but epochs are enabled for that view", vote.view_number());
                                return;
                            }
                            HotShotEvent::DaVoteRecv(vote.clone().to_vote2())
                        },
                        DaConsensusMessage::DaVote2(vote) => {
                            if !self.upgrade_lock.epochs_enabled(vote.view_number()).await {
                                tracing::warn!("received DaConsensusMessage::DaVote2 for view {} but epochs are not enabled for that view", vote.view_number());
                                return;
                            }
                            HotShotEvent::DaVoteRecv(vote.clone())
                        },
                        DaConsensusMessage::DaCertificate(cert) => {
                            if self.upgrade_lock.epochs_enabled(cert.view_number()).await {
                                tracing::warn!("received DaConsensusMessage::DaCertificate for view {} but epochs are enabled for that view", cert.view_number());
                                return;
                            }
                            HotShotEvent::DaCertificateRecv(cert.to_dac2())
                        },
                        DaConsensusMessage::DaCertificate2(cert) => {
                            if !self.upgrade_lock.epochs_enabled(cert.view_number()).await {
                                tracing::warn!("received DaConsensusMessage::DaCertificate2 for view {} but epochs are not enabled for that view", cert.view_number());
                                return;
                            }
                            HotShotEvent::DaCertificateRecv(cert)
                        },
                        DaConsensusMessage::VidDisperseMsg(proposal) => {
                            if self
                                .upgrade_lock
                                .epochs_enabled(proposal.data.view_number())
                                .await
                            {
                                tracing::warn!("received DaConsensusMessage::VidDisperseMsg for view {} but epochs are enabled for that view", proposal.data.view_number());
                                return;
                            }
                            HotShotEvent::VidShareRecv(sender, convert_proposal(proposal))
                        },
                        DaConsensusMessage::VidDisperseMsg2(proposal) => {
                            if !self
                                .upgrade_lock
                                .epochs_enabled(proposal.data.view_number())
                                .await
                            {
                                tracing::warn!("received DaConsensusMessage::VidDisperseMsg2 for view {} but epochs are not enabled for that view", proposal.data.view_number());
                                return;
                            }
                            HotShotEvent::VidShareRecv(sender, convert_proposal(proposal))
                        },
                    },
                };
                broadcast_event(Arc::new(event), &self.internal_event_stream).await;
            },

            // Handle data messages
            MessageKind::Data(message) => match message {
                DataMessage::SubmitTransaction(transaction, _) => {
                    let mut hasher = DefaultHasher::new();
                    transaction.hash(&mut hasher);
                    if self.transactions_cache.put(hasher.finish(), ()).is_some() {
                        return;
                    }
                    broadcast_event(
                        Arc::new(HotShotEvent::TransactionsRecv(vec![transaction])),
                        &self.internal_event_stream,
                    )
                    .await;
                },
                DataMessage::DataResponse(response) => {
                    if let ResponseMessage::Found(message) = response {
                        match message {
                            SequencingMessage::Da(DaConsensusMessage::VidDisperseMsg(proposal)) => {
                                broadcast_event(
                                    Arc::new(HotShotEvent::VidResponseRecv(
                                        sender,
                                        convert_proposal(proposal),
                                    )),
                                    &self.internal_event_stream,
                                )
                                .await;
                            },
                            SequencingMessage::Da(DaConsensusMessage::VidDisperseMsg2(
                                proposal,
                            )) => {
                                broadcast_event(
                                    Arc::new(HotShotEvent::VidResponseRecv(
                                        sender,
                                        convert_proposal(proposal),
                                    )),
                                    &self.internal_event_stream,
                                )
                                .await;
                            },
                            _ => {},
                        }
                    }
                },
                DataMessage::RequestData(data) => {
                    let req_data = data.clone();
                    if let RequestKind::Vid(_view_number, _key) = req_data.request {
                        broadcast_event(
                            Arc::new(HotShotEvent::VidRequestRecv(data, sender)),
                            &self.internal_event_stream,
                        )
                        .await;
                    }
                },
            },

            // Handle external messages
            MessageKind::External(data) => {
                if sender == self.public_key {
                    return;
                }
                // Send the external message to the external event stream so it can be processed
                broadcast_event(
                    Event {
                        view_number: TYPES::View::new(1),
                        event: EventType::ExternalMessageReceived { sender, data },
                    },
                    &self.external_event_stream,
                )
                .await;
            },
        }
    }
}

/// network event task state
pub struct NetworkEventTaskState<
    TYPES: NodeType,
    V: Versions,
    NET: ConnectedNetwork<TYPES::SignatureKey>,
    S: Storage<TYPES>,
> {
    /// comm network
    pub network: Arc<NET>,

    /// view number
    pub view: TYPES::View,

    /// epoch number
    pub epoch: Option<TYPES::Epoch>,

    /// network memberships
    pub membership_coordinator: EpochMembershipCoordinator<TYPES>,

    /// Storage to store actionable events
    pub storage: Arc<RwLock<S>>,

    /// Shared consensus state
    pub consensus: OuterConsensus<TYPES>,

    /// Lock for a decided upgrade
    pub upgrade_lock: UpgradeLock<TYPES, V>,

    /// map view number to transmit tasks
    pub transmit_tasks: BTreeMap<TYPES::View, Vec<JoinHandle<()>>>,

    /// Number of blocks in an epoch, zero means there are no epochs
    pub epoch_height: u64,

    /// Node's id
    pub id: u64,
}

#[async_trait]
impl<
        TYPES: NodeType,
        V: Versions,
        NET: ConnectedNetwork<TYPES::SignatureKey>,
        S: Storage<TYPES> + 'static,
    > TaskState for NetworkEventTaskState<TYPES, V, NET, S>
{
    type Event = HotShotEvent<TYPES>;

    async fn handle_event(
        &mut self,
        event: Arc<Self::Event>,
        _sender: &Sender<Arc<Self::Event>>,
        _receiver: &Receiver<Arc<Self::Event>>,
    ) -> Result<()> {
        self.handle(event).await;

        Ok(())
    }

    fn cancel_subtasks(&mut self) {}
}

impl<
        TYPES: NodeType,
        V: Versions,
        NET: ConnectedNetwork<TYPES::SignatureKey>,
        S: Storage<TYPES> + 'static,
    > NetworkEventTaskState<TYPES, V, NET, S>
{
    /// Handle the given event.
    ///
    /// Returns the completion status.
    #[instrument(skip_all, fields(id = self.id, view = *self.view), name = "Network Task", level = "error")]
    pub async fn handle(&mut self, event: Arc<HotShotEvent<TYPES>>) {
        let mut maybe_action = None;
        if let Some((sender, message_kind, transmit)) =
            self.parse_event(event, &mut maybe_action).await
        {
            self.spawn_transmit_task(message_kind, maybe_action, transmit, sender)
                .await;
        };
    }

    /// handle `VidDisperseSend`
    async fn handle_vid_disperse_proposal(
        &self,
        vid_proposal: Proposal<TYPES, VidDisperse<TYPES>>,
        sender: &<TYPES as NodeType>::SignatureKey,
    ) -> Option<HotShotTaskCompleted> {
        let view = vid_proposal.data.view_number();
        let epoch = vid_proposal.data.epoch();
        let vid_share_proposals = VidDisperseShare::to_vid_share_proposals(vid_proposal);
        let mut messages = HashMap::new();

        for proposal in vid_share_proposals {
            let recipient = proposal.data.recipient_key().clone();
            let message = if self
                .upgrade_lock
                .epochs_enabled(proposal.data.view_number())
                .await
            {
                let vid_share_proposal = if let VidDisperseShare::V1(data) = proposal.data {
                    Proposal {
                        data,
                        signature: proposal.signature,
                        _pd: proposal._pd,
                    }
                } else {
                    tracing::warn!(
                        "Epochs are enabled for view {} but didn't receive VidDisperseShare2",
                        proposal.data.view_number()
                    );
                    return None;
                };
                Message {
                    sender: sender.clone(),
                    kind: MessageKind::<TYPES>::from_consensus_message(SequencingMessage::Da(
                        DaConsensusMessage::VidDisperseMsg2(vid_share_proposal),
                    )),
                }
            } else {
                let vid_share_proposal = if let VidDisperseShare::V0(data) = proposal.data {
                    Proposal {
                        data,
                        signature: proposal.signature,
                        _pd: proposal._pd,
                    }
                } else {
                    tracing::warn!(
                        "Epochs are not enabled for view {} but didn't receive ADVZDisperseShare",
                        proposal.data.view_number()
                    );
                    return None;
                };
                Message {
                    sender: sender.clone(),
                    kind: MessageKind::<TYPES>::from_consensus_message(SequencingMessage::Da(
                        DaConsensusMessage::VidDisperseMsg(vid_share_proposal),
                    )),
                }
            };
            let serialized_message = match self.upgrade_lock.serialize(&message).await {
                Ok(serialized) => serialized,
                Err(e) => {
                    tracing::error!("Failed to serialize message: {e}");
                    continue;
                },
            };

            messages.insert(recipient, serialized_message);
        }

        let net = Arc::clone(&self.network);
        let storage = Arc::clone(&self.storage);
        let consensus = OuterConsensus::new(Arc::clone(&self.consensus.inner_consensus));
        spawn(async move {
            if NetworkEventTaskState::<TYPES, V, NET, S>::maybe_record_action(
                Some(HotShotAction::VidDisperse),
                storage,
                consensus,
                view,
                epoch,
            )
            .await
            .is_err()
            {
                return;
            }
            match net.vid_broadcast_message(messages).await {
                Ok(()) => {},
                Err(e) => tracing::warn!("Failed to send message from network task: {e:?}"),
            }
        });

        None
    }

    /// Record `HotShotAction` if available
    async fn maybe_record_action(
        maybe_action: Option<HotShotAction>,
        storage: Arc<RwLock<S>>,
        consensus: OuterConsensus<TYPES>,
        view: <TYPES as NodeType>::View,
        epoch: Option<<TYPES as NodeType>::Epoch>,
    ) -> std::result::Result<(), ()> {
        if let Some(mut action) = maybe_action {
            if !consensus.write().await.update_action(action, view) {
                return Err(());
            }
            // If the action was view sync record it as a vote, but we don't
            // want to limit to 1 View sync vote above so change the action here.
            if matches!(action, HotShotAction::ViewSyncVote) {
                action = HotShotAction::Vote;
            }
            match storage
                .write()
                .await
                .record_action(view, epoch, action)
                .await
            {
                Ok(()) => Ok(()),
                Err(e) => {
                    tracing::warn!("Not Sending {action:?} because of storage error: {e:?}");
                    Err(())
                },
            }
        } else {
            Ok(())
        }
    }

    /// Cancel all tasks for previous views
    pub fn cancel_tasks(&mut self, view: TYPES::View) {
        let keep = self.transmit_tasks.split_off(&view);

        while let Some((_, tasks)) = self.transmit_tasks.pop_first() {
            for task in tasks {
                task.abort();
            }
        }

        self.transmit_tasks = keep;
    }

    /// Parses a `HotShotEvent` and returns a tuple of: (sender's public key, `MessageKind`, `TransmitType`)
    /// which will be used to create a message and transmit on the wire.
    /// Returns `None` if the parsing result should not be sent on the wire.
    /// Handles the `VidDisperseSend` event separately using a helper method.
    #[allow(clippy::too_many_lines)]
    async fn parse_event(
        &mut self,
        event: Arc<HotShotEvent<TYPES>>,
        maybe_action: &mut Option<HotShotAction>,
    ) -> Option<(
        <TYPES as NodeType>::SignatureKey,
        MessageKind<TYPES>,
        TransmitType<TYPES>,
    )> {
        match event.as_ref().clone() {
            HotShotEvent::QuorumProposalSend(proposal, sender) => {
                *maybe_action = Some(HotShotAction::Propose);

                let message = if self
                    .upgrade_lock
                    .epochs_enabled(proposal.data.view_number())
                    .await
                {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::General(
                        GeneralConsensusMessage::Proposal2(convert_proposal(proposal)),
                    ))
                } else {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::General(
                        GeneralConsensusMessage::Proposal(convert_proposal(proposal)),
                    ))
                };

                Some((sender, message, TransmitType::Broadcast))
            },

            // ED Each network task is subscribed to all these message types.  Need filters per network task
            HotShotEvent::QuorumVoteSend(vote) => {
                *maybe_action = Some(HotShotAction::Vote);
                let view_number = vote.view_number() + 1;
                let leader = match self
                    .membership_coordinator
                    .membership_for_epoch(vote.epoch())
                    .await
                    .ok()?
                    .leader(view_number)
                    .await
                {
                    Ok(l) => l,
                    Err(e) => {
                        tracing::warn!("Failed to calculate leader for view number {view_number}. Error: {e:?}");
                        return None;
                    },
                };

                let message = if self.upgrade_lock.epochs_enabled(vote.view_number()).await {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::General(
                        GeneralConsensusMessage::Vote2(vote.clone()),
                    ))
                } else {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::General(
                        GeneralConsensusMessage::Vote(vote.clone().to_vote()),
                    ))
                };

                Some((vote.signing_key(), message, TransmitType::Direct(leader)))
            },
            HotShotEvent::EpochRootQuorumVoteSend(vote) => {
                *maybe_action = Some(HotShotAction::Vote);
                let view_number = vote.view_number() + 1;
                let leader = match self
                    .membership_coordinator
                    .membership_for_epoch(vote.epoch())
                    .await
                    .ok()?
                    .leader(view_number)
                    .await
                {
                    Ok(l) => l,
                    Err(e) => {
                        tracing::warn!(
                            "Failed to calculate leader for view number {:?}. Error: {:?}",
                            view_number,
                            e
                        );
                        return None;
                    },
                };

                let message = if self.upgrade_lock.epochs_enabled(vote.view_number()).await {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::General(
                        GeneralConsensusMessage::EpochRootQuorumVote(vote.clone()),
                    ))
                } else {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::General(
                        GeneralConsensusMessage::Vote(vote.vote.clone().to_vote()),
                    ))
                };

                Some((
                    vote.vote.signing_key(),
                    message,
                    TransmitType::Direct(leader),
                ))
            },
            HotShotEvent::ExtendedQuorumVoteSend(vote) => {
                *maybe_action = Some(HotShotAction::Vote);
                let message = if self.upgrade_lock.epochs_enabled(vote.view_number()).await {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::General(
                        GeneralConsensusMessage::Vote2(vote.clone()),
                    ))
                } else {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::General(
                        GeneralConsensusMessage::Vote(vote.clone().to_vote()),
                    ))
                };

                Some((vote.signing_key(), message, TransmitType::Broadcast))
            },
            HotShotEvent::QuorumProposalRequestSend(req, signature) => Some((
                req.key.clone(),
                MessageKind::<TYPES>::from_consensus_message(SequencingMessage::General(
                    GeneralConsensusMessage::ProposalRequested(req.clone(), signature),
                )),
                TransmitType::Broadcast,
            )),
            HotShotEvent::QuorumProposalResponseSend(sender_key, proposal) => {
                let message = if self
                    .upgrade_lock
                    .epochs_enabled(proposal.data.view_number())
                    .await
                {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::General(
                        GeneralConsensusMessage::ProposalResponse2(convert_proposal(proposal)),
                    ))
                } else {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::General(
                        GeneralConsensusMessage::ProposalResponse(convert_proposal(proposal)),
                    ))
                };

                Some((
                    sender_key.clone(),
                    message,
                    TransmitType::Direct(sender_key),
                ))
            },
            HotShotEvent::VidDisperseSend(proposal, sender) => {
                self.handle_vid_disperse_proposal(proposal, &sender).await;
                None
            },
            HotShotEvent::DaProposalSend(proposal, sender) => {
                *maybe_action = Some(HotShotAction::DaPropose);

                let message = if self
                    .upgrade_lock
                    .epochs_enabled(proposal.data.view_number())
                    .await
                {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::Da(
                        DaConsensusMessage::DaProposal2(proposal),
                    ))
                } else {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::Da(
                        DaConsensusMessage::DaProposal(convert_proposal(proposal)),
                    ))
                };

                Some((sender, message, TransmitType::DaCommitteeBroadcast))
            },
            HotShotEvent::DaVoteSend(vote) => {
                *maybe_action = Some(HotShotAction::DaVote);
                let view_number = vote.view_number();
                let leader = match self
                    .membership_coordinator
                    .membership_for_epoch(vote.epoch())
                    .await
                    .ok()?
                    .leader(view_number)
                    .await
                {
                    Ok(l) => l,
                    Err(e) => {
                        tracing::warn!("Failed to calculate leader for view number {view_number}. Error: {e:?}");
                        return None;
                    },
                };

                let message = if self.upgrade_lock.epochs_enabled(view_number).await {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::Da(
                        DaConsensusMessage::DaVote2(vote.clone()),
                    ))
                } else {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::Da(
                        DaConsensusMessage::DaVote(vote.clone().to_vote()),
                    ))
                };

                Some((vote.signing_key(), message, TransmitType::Direct(leader)))
            },
            HotShotEvent::DacSend(certificate, sender) => {
                *maybe_action = Some(HotShotAction::DaCert);
                let message = if self
                    .upgrade_lock
                    .epochs_enabled(certificate.view_number())
                    .await
                {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::Da(
                        DaConsensusMessage::DaCertificate2(certificate),
                    ))
                } else {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::Da(
                        DaConsensusMessage::DaCertificate(certificate.to_dac()),
                    ))
                };

                Some((sender, message, TransmitType::Broadcast))
            },
            HotShotEvent::ViewSyncPreCommitVoteSend(vote) => {
                let view_number = vote.view_number() + vote.date().relay;
                let leader = match self
                    .membership_coordinator
                    .membership_for_epoch(self.epoch)
                    .await
                    .ok()?
                    .leader(view_number)
                    .await
                {
                    Ok(l) => l,
                    Err(e) => {
                        tracing::warn!("Failed to calculate leader for view number {view_number}. Error: {e:?}");
                        return None;
                    },
                };
                let message = if self.upgrade_lock.epochs_enabled(vote.view_number()).await {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::General(
                        GeneralConsensusMessage::ViewSyncPreCommitVote2(vote.clone()),
                    ))
                } else {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::General(
                        GeneralConsensusMessage::ViewSyncPreCommitVote(vote.clone().to_vote()),
                    ))
                };

                Some((vote.signing_key(), message, TransmitType::Direct(leader)))
            },
            HotShotEvent::ViewSyncCommitVoteSend(vote) => {
                *maybe_action = Some(HotShotAction::ViewSyncVote);
                let view_number = vote.view_number() + vote.date().relay;
                let leader = match self
                    .membership_coordinator
                    .membership_for_epoch(self.epoch)
                    .await
                    .ok()?
                    .leader(view_number)
                    .await
                {
                    Ok(l) => l,
                    Err(e) => {
                        tracing::warn!("Failed to calculate leader for view number {view_number}. Error: {e:?}");
                        return None;
                    },
                };
                let message = if self.upgrade_lock.epochs_enabled(vote.view_number()).await {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::General(
                        GeneralConsensusMessage::ViewSyncCommitVote2(vote.clone()),
                    ))
                } else {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::General(
                        GeneralConsensusMessage::ViewSyncCommitVote(vote.clone().to_vote()),
                    ))
                };

                Some((vote.signing_key(), message, TransmitType::Direct(leader)))
            },
            HotShotEvent::ViewSyncFinalizeVoteSend(vote) => {
                *maybe_action = Some(HotShotAction::ViewSyncVote);
                let view_number = vote.view_number() + vote.date().relay;
                let leader = match self
                    .membership_coordinator
                    .membership_for_epoch(self.epoch)
                    .await
                    .ok()?
                    .leader(view_number)
                    .await
                {
                    Ok(l) => l,
                    Err(e) => {
                        tracing::warn!("Failed to calculate leader for view number {view_number:?}. Error: {e:?}");
                        return None;
                    },
                };
                let message = if self.upgrade_lock.epochs_enabled(vote.view_number()).await {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::General(
                        GeneralConsensusMessage::ViewSyncFinalizeVote2(vote.clone()),
                    ))
                } else {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::General(
                        GeneralConsensusMessage::ViewSyncFinalizeVote(vote.clone().to_vote()),
                    ))
                };

                Some((vote.signing_key(), message, TransmitType::Direct(leader)))
            },
            HotShotEvent::ViewSyncPreCommitCertificateSend(certificate, sender) => {
                let view_number = certificate.view_number();
                let message = if self.upgrade_lock.epochs_enabled(view_number).await {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::General(
                        GeneralConsensusMessage::ViewSyncPreCommitCertificate2(certificate),
                    ))
                } else {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::General(
                        GeneralConsensusMessage::ViewSyncPreCommitCertificate(certificate.to_vsc()),
                    ))
                };

                Some((sender, message, TransmitType::Broadcast))
            },
            HotShotEvent::ViewSyncCommitCertificateSend(certificate, sender) => {
                let view_number = certificate.view_number();
                let message = if self.upgrade_lock.epochs_enabled(view_number).await {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::General(
                        GeneralConsensusMessage::ViewSyncCommitCertificate2(certificate),
                    ))
                } else {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::General(
                        GeneralConsensusMessage::ViewSyncCommitCertificate(certificate.to_vsc()),
                    ))
                };

                Some((sender, message, TransmitType::Broadcast))
            },
            HotShotEvent::ViewSyncFinalizeCertificateSend(certificate, sender) => {
                let view_number = certificate.view_number();
                let message = if self.upgrade_lock.epochs_enabled(view_number).await {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::General(
                        GeneralConsensusMessage::ViewSyncFinalizeCertificate2(certificate),
                    ))
                } else {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::General(
                        GeneralConsensusMessage::ViewSyncFinalizeCertificate(certificate.to_vsc()),
                    ))
                };

                Some((sender, message, TransmitType::Broadcast))
            },
            HotShotEvent::TimeoutVoteSend(vote) => {
                *maybe_action = Some(HotShotAction::Vote);
                let view_number = vote.view_number() + 1;
                let leader = match self
                    .membership_coordinator
                    .membership_for_epoch(self.epoch)
                    .await
                    .ok()?
                    .leader(view_number)
                    .await
                {
                    Ok(l) => l,
                    Err(e) => {
                        tracing::warn!("Failed to calculate leader for view number {view_number}. Error: {e:?}");
                        return None;
                    },
                };
                let message = if self.upgrade_lock.epochs_enabled(vote.view_number()).await {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::General(
                        GeneralConsensusMessage::TimeoutVote2(vote.clone()),
                    ))
                } else {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::General(
                        GeneralConsensusMessage::TimeoutVote(vote.clone().to_vote()),
                    ))
                };

                Some((vote.signing_key(), message, TransmitType::Direct(leader)))
            },
            HotShotEvent::UpgradeProposalSend(proposal, sender) => Some((
                sender,
                MessageKind::<TYPES>::from_consensus_message(SequencingMessage::General(
                    GeneralConsensusMessage::UpgradeProposal(proposal),
                )),
                TransmitType::Broadcast,
            )),
            HotShotEvent::UpgradeVoteSend(vote) => {
                tracing::error!("Sending upgrade vote!");
                let view_number = vote.view_number();
                let leader = match self
                    .membership_coordinator
                    .membership_for_epoch(self.epoch)
                    .await
                    .ok()?
                    .leader(view_number)
                    .await
                {
                    Ok(l) => l,
                    Err(e) => {
                        tracing::warn!("Failed to calculate leader for view number {view_number}. Error: {e:?}");
                        return None;
                    },
                };
                Some((
                    vote.signing_key(),
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::General(
                        GeneralConsensusMessage::UpgradeVote(vote.clone()),
                    )),
                    TransmitType::Direct(leader),
                ))
            },
            HotShotEvent::ViewChange(view, epoch) => {
                self.view = view;
                if epoch > self.epoch {
                    self.epoch = epoch;
                }
                let keep_view = TYPES::View::new(view.saturating_sub(1));
                self.cancel_tasks(keep_view);
                let net = Arc::clone(&self.network);
                let epoch = self.epoch.map(|x| x.u64());
                let membership_coordinator = self.membership_coordinator.clone();
                spawn(async move {
                    net.update_view::<TYPES>(*keep_view, epoch, membership_coordinator)
                        .await;
                });
                None
            },
            HotShotEvent::VidRequestSend(req, sender, to) => Some((
                sender,
                MessageKind::Data(DataMessage::RequestData(req)),
                TransmitType::Direct(to),
            )),
            HotShotEvent::VidResponseSend(sender, to, proposal) => {
                let epochs_enabled = self
                    .upgrade_lock
                    .epochs_enabled(proposal.data.view_number())
                    .await;
                let message = match proposal.data {
                    VidDisperseShare::V0(data) => {
                        if epochs_enabled {
                            tracing::warn!(
                        "Epochs are enabled for view {} but didn't receive VidDisperseShare2",
                        data.view_number()
                    );
                            return None;
                        }
                        let vid_share_proposal = Proposal {
                            data,
                            signature: proposal.signature,
                            _pd: proposal._pd,
                        };
                        MessageKind::Data(DataMessage::DataResponse(ResponseMessage::Found(
                            SequencingMessage::Da(DaConsensusMessage::VidDisperseMsg(
                                vid_share_proposal,
                            )),
                        )))
                    },
                    VidDisperseShare::V1(data) => {
                        if !epochs_enabled {
                            tracing::warn!(
                                "Epochs are enabled for view {} but didn't receive ADVZDisperseShare",
                                data.view_number()
                            );
                            return None;
                        }
                        let vid_share_proposal = Proposal {
                            data,
                            signature: proposal.signature,
                            _pd: proposal._pd,
                        };
                        MessageKind::Data(DataMessage::DataResponse(ResponseMessage::Found(
                            SequencingMessage::Da(DaConsensusMessage::VidDisperseMsg2(
                                vid_share_proposal,
                            )),
                        )))
                    },
                };
                Some((sender, message, TransmitType::Direct(to)))
            },
            HotShotEvent::HighQcSend(quorum_cert, next_epoch_qc, leader, sender) => Some((
                sender,
                MessageKind::Consensus(SequencingMessage::General(
                    GeneralConsensusMessage::HighQc(quorum_cert, next_epoch_qc),
                )),
                TransmitType::Direct(leader),
            )),
            HotShotEvent::EpochRootQcSend(epoch_root_qc, sender, leader) => Some((
                sender,
                MessageKind::Consensus(SequencingMessage::General(
                    GeneralConsensusMessage::EpochRootQc(epoch_root_qc),
                )),
                TransmitType::Direct(leader),
            )),
            HotShotEvent::ExtendedQcSend(quorum_cert, next_epoch_qc, sender) => Some((
                sender,
                MessageKind::Consensus(SequencingMessage::General(
                    GeneralConsensusMessage::ExtendedQc(quorum_cert, next_epoch_qc),
                )),
                TransmitType::Broadcast,
            )),
            _ => None,
        }
    }

    /// Creates a network message and spawns a task that transmits it on the wire.
    async fn spawn_transmit_task(
        &mut self,
        message_kind: MessageKind<TYPES>,
        maybe_action: Option<HotShotAction>,
        transmit: TransmitType<TYPES>,
        sender: TYPES::SignatureKey,
    ) {
        let broadcast_delay = match &message_kind {
            MessageKind::Consensus(
                SequencingMessage::General(GeneralConsensusMessage::Vote(_))
                | SequencingMessage::Da(_),
            ) => BroadcastDelay::View(*message_kind.view_number()),
            _ => BroadcastDelay::None,
        };
        let message = Message {
            sender,
            kind: message_kind,
        };
        let view_number = message.kind.view_number();
        let epoch = message.kind.epoch();
        let committee_topic = Topic::Global;
        let Ok(mem) = self
            .membership_coordinator
            .stake_table_for_epoch(self.epoch)
            .await
        else {
            return;
        };
        let da_committee = mem.da_committee_members(view_number).await;
        let network = Arc::clone(&self.network);
        let storage = Arc::clone(&self.storage);
        let consensus = OuterConsensus::new(Arc::clone(&self.consensus.inner_consensus));
        let upgrade_lock = self.upgrade_lock.clone();
        let handle = spawn(async move {
            if NetworkEventTaskState::<TYPES, V, NET, S>::maybe_record_action(
                maybe_action,
                Arc::clone(&storage),
                consensus,
                view_number,
                epoch,
            )
            .await
            .is_err()
            {
                return;
            }
            if let MessageKind::Consensus(SequencingMessage::General(
                GeneralConsensusMessage::Proposal(prop),
            )) = &message.kind
            {
                if storage
                    .write()
                    .await
                    .append_proposal2(&convert_proposal(prop.clone()))
                    .await
                    .is_err()
                {
                    return;
                }
            }

            let serialized_message = match upgrade_lock.serialize(&message).await {
                Ok(serialized) => serialized,
                Err(e) => {
                    tracing::error!("Failed to serialize message: {e}");
                    return;
                },
            };

            let transmit_result = match transmit {
                TransmitType::Direct(recipient) => {
                    network.direct_message(serialized_message, recipient).await
                },
                TransmitType::Broadcast => {
                    network
                        .broadcast_message(serialized_message, committee_topic, broadcast_delay)
                        .await
                },
                TransmitType::DaCommitteeBroadcast => {
                    network
                        .da_broadcast_message(
                            serialized_message,
                            da_committee.iter().cloned().collect(),
                            broadcast_delay,
                        )
                        .await
                },
            };

            match transmit_result {
                Ok(()) => {},
                Err(e) => tracing::warn!("Failed to send message task: {e:?}"),
            }
        });
        self.transmit_tasks
            .entry(view_number)
            .or_default()
            .push(handle);
    }
}

/// A module with test helpers
pub mod test {
    use std::ops::{Deref, DerefMut};

    use async_trait::async_trait;

    use super::{
        Arc, ConnectedNetwork, HotShotEvent, MessageKind, NetworkEventTaskState, NodeType,
        Receiver, Result, Sender, Storage, TaskState, TransmitType, Versions,
    };

    /// A dynamic type alias for a function that takes the result of `NetworkEventTaskState::parse_event`
    /// and changes it before transmitting on the network.
    pub type ModifierClosure<TYPES> = dyn Fn(
            &mut <TYPES as NodeType>::SignatureKey,
            &mut MessageKind<TYPES>,
            &mut TransmitType<TYPES>,
            &<TYPES as NodeType>::Membership,
        ) + Send
        + Sync;

    /// A helper wrapper around `NetworkEventTaskState` that can modify its behaviour for tests
    pub struct NetworkEventTaskStateModifier<
        TYPES: NodeType,
        V: Versions,
        NET: ConnectedNetwork<TYPES::SignatureKey>,
        S: Storage<TYPES>,
    > {
        /// The real `NetworkEventTaskState`
        pub network_event_task_state: NetworkEventTaskState<TYPES, V, NET, S>,
        /// A function that takes the result of `NetworkEventTaskState::parse_event` and
        /// changes it before transmitting on the network.
        pub modifier: Arc<ModifierClosure<TYPES>>,
    }

    impl<
            TYPES: NodeType,
            V: Versions,
            NET: ConnectedNetwork<TYPES::SignatureKey>,
            S: Storage<TYPES> + 'static,
        > NetworkEventTaskStateModifier<TYPES, V, NET, S>
    {
        /// Handles the received event modifying it before sending on the network.
        pub async fn handle(&mut self, event: Arc<HotShotEvent<TYPES>>) {
            let mut maybe_action = None;
            if let Some((mut sender, mut message_kind, mut transmit)) =
                self.parse_event(event, &mut maybe_action).await
            {
                // Modify the values acquired by parsing the event.
                (self.modifier)(
                    &mut sender,
                    &mut message_kind,
                    &mut transmit,
                    &*self.membership_coordinator.membership().read().await,
                );

                self.spawn_transmit_task(message_kind, maybe_action, transmit, sender)
                    .await;
            }
        }
    }

    #[async_trait]
    impl<
            TYPES: NodeType,
            V: Versions,
            NET: ConnectedNetwork<TYPES::SignatureKey>,
            S: Storage<TYPES> + 'static,
        > TaskState for NetworkEventTaskStateModifier<TYPES, V, NET, S>
    {
        type Event = HotShotEvent<TYPES>;

        async fn handle_event(
            &mut self,
            event: Arc<Self::Event>,
            _sender: &Sender<Arc<Self::Event>>,
            _receiver: &Receiver<Arc<Self::Event>>,
        ) -> Result<()> {
            self.handle(event).await;

            Ok(())
        }

        fn cancel_subtasks(&mut self) {}
    }

    impl<
            TYPES: NodeType,
            V: Versions,
            NET: ConnectedNetwork<TYPES::SignatureKey>,
            S: Storage<TYPES>,
        > Deref for NetworkEventTaskStateModifier<TYPES, V, NET, S>
    {
        type Target = NetworkEventTaskState<TYPES, V, NET, S>;

        fn deref(&self) -> &Self::Target {
            &self.network_event_task_state
        }
    }

    impl<
            TYPES: NodeType,
            V: Versions,
            NET: ConnectedNetwork<TYPES::SignatureKey>,
            S: Storage<TYPES>,
        > DerefMut for NetworkEventTaskStateModifier<TYPES, V, NET, S>
    {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.network_event_task_state
        }
    }
}
