use std::{
    collections::{BTreeMap, HashMap, HashSet},
    iter::once,
    sync::Arc,
};

use anyhow::Context;
use async_lock::RwLock;
use async_trait::async_trait;
use hotshot::{
    tasks::EventTransformerState,
    types::{SignatureKey, SystemContextHandle},
};
use hotshot_task_impls::{
    events::HotShotEvent,
    network::{
        test::{ModifierClosure, NetworkEventTaskStateModifier},
        NetworkEventTaskState,
    },
};
use hotshot_types::{
    consensus::OuterConsensus,
    data::QuorumProposalWrapper,
    epoch_membership::EpochMembershipCoordinator,
    message::{
        convert_proposal, GeneralConsensusMessage, Message, MessageKind, Proposal,
        SequencingMessage, UpgradeLock,
    },
    simple_vote::{
        HasEpoch, QuorumVote2, ViewSyncPreCommitData, ViewSyncPreCommitData2,
        ViewSyncPreCommitVote, ViewSyncPreCommitVote2,
    },
    traits::{
        election::Membership,
        network::ConnectedNetwork,
        node_implementation::{ConsensusTime, NodeImplementation, NodeType, Versions},
    },
    vote::HasViewNumber,
};

#[derive(Debug)]
/// An `EventTransformerState` that multiplies `QuorumProposalSend` events, incrementing the view number of the proposal
pub struct BadProposalViewDos {
    /// The number of times to duplicate a `QuorumProposalSend` event
    pub multiplier: u64,
    /// The view number increment each time it's duplicatedjust
    pub increment: u64,
}

#[async_trait]
impl<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> EventTransformerState<TYPES, I, V>
    for BadProposalViewDos
{
    async fn recv_handler(&mut self, event: &HotShotEvent<TYPES>) -> Vec<HotShotEvent<TYPES>> {
        vec![event.clone()]
    }

    async fn send_handler(
        &mut self,
        event: &HotShotEvent<TYPES>,
        _public_key: &TYPES::SignatureKey,
        _private_key: &<TYPES::SignatureKey as SignatureKey>::PrivateKey,
        _upgrade_lock: &UpgradeLock<TYPES, V>,
        consensus: OuterConsensus<TYPES>,
        _membership_coordinator: EpochMembershipCoordinator<TYPES>,
        _network: Arc<I::Network>,
    ) -> Vec<HotShotEvent<TYPES>> {
        match event {
            HotShotEvent::QuorumProposalSend(proposal, signature) => {
                let mut result = Vec::new();

                for n in 1..self.multiplier {
                    let mut modified_proposal = proposal.clone();

                    modified_proposal.data.proposal.view_number += n * self.increment;

                    result.push(HotShotEvent::QuorumProposalSend(
                        modified_proposal,
                        signature.clone(),
                    ));
                }

                consensus.write().await.reset_actions();
                result
            },
            _ => vec![event.clone()],
        }
    }
}

#[derive(Debug)]
/// An `EventHandlerState` that doubles the `QuorumVoteSend` and `QuorumProposalSend` events
pub struct DoubleProposeVote;

#[async_trait]
impl<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> EventTransformerState<TYPES, I, V>
    for DoubleProposeVote
{
    async fn recv_handler(&mut self, event: &HotShotEvent<TYPES>) -> Vec<HotShotEvent<TYPES>> {
        vec![event.clone()]
    }

    async fn send_handler(
        &mut self,
        event: &HotShotEvent<TYPES>,
        _public_key: &TYPES::SignatureKey,
        _private_key: &<TYPES::SignatureKey as SignatureKey>::PrivateKey,
        _upgrade_lock: &UpgradeLock<TYPES, V>,
        _consensus: OuterConsensus<TYPES>,
        _membership_coordinator: EpochMembershipCoordinator<TYPES>,
        _network: Arc<I::Network>,
    ) -> Vec<HotShotEvent<TYPES>> {
        match event {
            HotShotEvent::QuorumProposalSend(..) | HotShotEvent::QuorumVoteSend(_) => {
                vec![event.clone(), event.clone()]
            },
            _ => vec![event.clone()],
        }
    }
}

#[derive(Debug)]
/// An `EventHandlerState` that modifies justify_qc on `QuorumProposalSend` to that of a previous view to mock dishonest leader
pub struct DishonestLeader<TYPES: NodeType> {
    /// Store events from previous views
    pub validated_proposals: Vec<QuorumProposalWrapper<TYPES>>,
    /// How many times current node has been elected leader and sent proposal
    pub total_proposals_from_node: u64,
    /// Which proposals to be dishonest at
    pub dishonest_at_proposal_numbers: HashSet<u64>,
    /// How far back to look for a QC
    pub view_look_back: usize,
    /// Shared state of all view numbers we send bad proposal at
    pub dishonest_proposal_view_numbers: Arc<RwLock<HashSet<TYPES::View>>>,
}

/// Add method that will handle `QuorumProposalSend` events
/// If we have previous proposals stored and the total_proposals_from_node matches a value specified in dishonest_at_proposal_numbers
/// Then send out the event with the modified proposal that has an older QC
impl<TYPES: NodeType> DishonestLeader<TYPES> {
    /// When a leader is sending a proposal this method will mock a dishonest leader
    /// We accomplish this by looking back a number of specified views and using that cached proposals QC
    async fn handle_proposal_send_event(
        &self,
        event: &HotShotEvent<TYPES>,
        proposal: &Proposal<TYPES, QuorumProposalWrapper<TYPES>>,
        sender: &TYPES::SignatureKey,
    ) -> HotShotEvent<TYPES> {
        let length = self.validated_proposals.len();
        if !self
            .dishonest_at_proposal_numbers
            .contains(&self.total_proposals_from_node)
            || length == 0
        {
            return event.clone();
        }

        // Grab proposal from specified view look back
        let proposal_from_look_back = if length - 1 < self.view_look_back {
            // If look back is too far just take the first proposal
            self.validated_proposals[0].clone()
        } else {
            let index = (self.validated_proposals.len() - 1) - self.view_look_back;
            self.validated_proposals[index].clone()
        };

        // Create a dishonest proposal by using the old proposals qc
        let mut dishonest_proposal = proposal.clone();
        dishonest_proposal.data.proposal.justify_qc = proposal_from_look_back.proposal.justify_qc;

        // Save the view we sent the dishonest proposal on (used for coordination attacks with other byzantine replicas)
        let mut dishonest_proposal_sent = self.dishonest_proposal_view_numbers.write().await;
        dishonest_proposal_sent.insert(proposal.data.view_number());

        HotShotEvent::QuorumProposalSend(dishonest_proposal, sender.clone())
    }
}

#[async_trait]
impl<TYPES: NodeType, I: NodeImplementation<TYPES> + std::fmt::Debug, V: Versions>
    EventTransformerState<TYPES, I, V> for DishonestLeader<TYPES>
{
    async fn recv_handler(&mut self, event: &HotShotEvent<TYPES>) -> Vec<HotShotEvent<TYPES>> {
        vec![event.clone()]
    }

    async fn send_handler(
        &mut self,
        event: &HotShotEvent<TYPES>,
        _public_key: &TYPES::SignatureKey,
        _private_key: &<TYPES::SignatureKey as SignatureKey>::PrivateKey,
        _upgrade_lock: &UpgradeLock<TYPES, V>,
        _consensus: OuterConsensus<TYPES>,
        _membership_coordinator: EpochMembershipCoordinator<TYPES>,
        _network: Arc<I::Network>,
    ) -> Vec<HotShotEvent<TYPES>> {
        match event {
            HotShotEvent::QuorumProposalSend(proposal, sender) => {
                self.total_proposals_from_node += 1;
                return vec![
                    self.handle_proposal_send_event(event, proposal, sender)
                        .await,
                ];
            },
            HotShotEvent::QuorumProposalValidated(proposal, _) => {
                self.validated_proposals.push(proposal.data.clone());
            },
            _ => {},
        }
        vec![event.clone()]
    }
}

#[derive(Debug)]
/// An `EventHandlerState` that modifies view number on the certificate of `DacSend` event to that of a future view
pub struct DishonestDa {
    /// How many times current node has been elected leader and sent Da Cert
    pub total_da_certs_sent_from_node: u64,
    /// Which proposals to be dishonest at
    pub dishonest_at_da_cert_sent_numbers: HashSet<u64>,
    /// When leader how many times we will send DacSend and increment view number
    pub total_views_add_to_cert: u64,
}

#[async_trait]
impl<TYPES: NodeType, I: NodeImplementation<TYPES> + std::fmt::Debug, V: Versions>
    EventTransformerState<TYPES, I, V> for DishonestDa
{
    async fn recv_handler(&mut self, event: &HotShotEvent<TYPES>) -> Vec<HotShotEvent<TYPES>> {
        vec![event.clone()]
    }

    async fn send_handler(
        &mut self,
        event: &HotShotEvent<TYPES>,
        _public_key: &TYPES::SignatureKey,
        _private_key: &<TYPES::SignatureKey as SignatureKey>::PrivateKey,
        _upgrade_lock: &UpgradeLock<TYPES, V>,
        _consensus: OuterConsensus<TYPES>,
        _membership_coordinator: EpochMembershipCoordinator<TYPES>,
        _network: Arc<I::Network>,
    ) -> Vec<HotShotEvent<TYPES>> {
        if let HotShotEvent::DacSend(cert, sender) = event {
            self.total_da_certs_sent_from_node += 1;
            if self
                .dishonest_at_da_cert_sent_numbers
                .contains(&self.total_da_certs_sent_from_node)
            {
                let mut result = vec![HotShotEvent::DacSend(cert.clone(), sender.clone())];
                for i in 1..=self.total_views_add_to_cert {
                    let mut bad_cert = cert.clone();
                    bad_cert.view_number = cert.view_number + i;
                    result.push(HotShotEvent::DacSend(bad_cert, sender.clone()));
                }
                return result;
            }
        }
        vec![event.clone()]
    }
}

/// View delay configuration
#[derive(Debug)]
pub struct ViewDelay<TYPES: NodeType> {
    /// How many views the node will be delayed
    pub number_of_views_to_delay: u64,
    /// A map that is from view number to vector of events
    pub events_for_view: HashMap<TYPES::View, Vec<HotShotEvent<TYPES>>>,
    /// Specify which view number to stop delaying
    pub stop_view_delay_at_view_number: u64,
}

#[async_trait]
impl<TYPES: NodeType, I: NodeImplementation<TYPES> + std::fmt::Debug, V: Versions>
    EventTransformerState<TYPES, I, V> for ViewDelay<TYPES>
{
    async fn recv_handler(&mut self, event: &HotShotEvent<TYPES>) -> Vec<HotShotEvent<TYPES>> {
        let correct_event = vec![event.clone()];
        if let Some(view_number) = event.view_number() {
            if *view_number >= self.stop_view_delay_at_view_number {
                return correct_event;
            }

            // add current view or push event to the map if view number has been added
            let events_for_current_view = self.events_for_view.entry(view_number).or_default();
            events_for_current_view.push(event.clone());

            // ensure we are actually able to lookback enough views
            let view_diff = (*view_number).saturating_sub(self.number_of_views_to_delay);
            if view_diff > 0 {
                return match self
                    .events_for_view
                    .remove(&<TYPES as NodeType>::View::new(view_diff))
                {
                    Some(lookback_events) => lookback_events.clone(),
                    // we have already return all received events for this view
                    None => vec![],
                };
            }
        }

        correct_event
    }

    async fn send_handler(
        &mut self,
        event: &HotShotEvent<TYPES>,
        _public_key: &TYPES::SignatureKey,
        _private_key: &<TYPES::SignatureKey as SignatureKey>::PrivateKey,
        _upgrade_lock: &UpgradeLock<TYPES, V>,
        _consensus: OuterConsensus<TYPES>,
        _membership_coordinator: EpochMembershipCoordinator<TYPES>,
        _network: Arc<I::Network>,
    ) -> Vec<HotShotEvent<TYPES>> {
        vec![event.clone()]
    }
}

/// An `EventHandlerState` that modifies view number on the vote of `QuorumVoteSend` event to that of a future view and correctly signs the vote
pub struct DishonestVoting<TYPES: NodeType> {
    /// Number added to the original vote's view number
    pub view_increment: u64,
    /// A function passed to `NetworkEventTaskStateModifier` to modify `NetworkEventTaskState` behaviour.
    pub modifier: Arc<ModifierClosure<TYPES>>,
}

#[async_trait]
impl<TYPES: NodeType, I: NodeImplementation<TYPES> + std::fmt::Debug, V: Versions>
    EventTransformerState<TYPES, I, V> for DishonestVoting<TYPES>
{
    async fn recv_handler(&mut self, event: &HotShotEvent<TYPES>) -> Vec<HotShotEvent<TYPES>> {
        vec![event.clone()]
    }

    async fn send_handler(
        &mut self,
        event: &HotShotEvent<TYPES>,
        public_key: &TYPES::SignatureKey,
        private_key: &<TYPES::SignatureKey as SignatureKey>::PrivateKey,
        upgrade_lock: &UpgradeLock<TYPES, V>,
        _consensus: OuterConsensus<TYPES>,
        _membership_coordinator: EpochMembershipCoordinator<TYPES>,
        _network: Arc<I::Network>,
    ) -> Vec<HotShotEvent<TYPES>> {
        if let HotShotEvent::QuorumVoteSend(vote) = event {
            let new_view = vote.view_number + self.view_increment;
            let spoofed_vote = QuorumVote2::<TYPES>::create_signed_vote(
                vote.data.clone(),
                new_view,
                public_key,
                private_key,
                upgrade_lock,
            )
            .await
            .context("Failed to sign vote")
            .unwrap();
            tracing::debug!("Sending Quorum Vote for view: {new_view:?}");
            return vec![HotShotEvent::QuorumVoteSend(spoofed_vote)];
        }
        vec![event.clone()]
    }

    fn add_network_event_task(
        &self,
        handle: &mut SystemContextHandle<TYPES, I, V>,
        network: Arc<<I as NodeImplementation<TYPES>>::Network>,
    ) {
        let network_state: NetworkEventTaskState<_, V, _, _> = NetworkEventTaskState {
            network,
            view: TYPES::View::genesis(),
            epoch: None,
            membership_coordinator: handle.membership_coordinator.clone(),
            storage: handle.storage(),
            storage_metrics: handle.storage_metrics(),
            consensus: OuterConsensus::new(handle.consensus()),
            upgrade_lock: handle.hotshot.upgrade_lock.clone(),
            transmit_tasks: BTreeMap::new(),
            epoch_height: handle.epoch_height,
            id: handle.hotshot.id,
        };
        let modified_network_state = NetworkEventTaskStateModifier {
            network_event_task_state: network_state,
            modifier: Arc::clone(&self.modifier),
        };
        handle.add_task(modified_network_state);
    }
}

impl<TYPES: NodeType> std::fmt::Debug for DishonestVoting<TYPES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DishonestVoting")
            .field("view_increment", &self.view_increment)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
/// An `EventHandlerState` that will send a vote for a bad proposal
pub struct DishonestVoter<TYPES: NodeType> {
    /// Collect all votes the node sends
    pub votes_sent: Vec<QuorumVote2<TYPES>>,
    /// Shared state with views numbers that leaders were dishonest at
    pub dishonest_proposal_view_numbers: Arc<RwLock<HashSet<TYPES::View>>>,
}

#[async_trait]
impl<TYPES: NodeType, I: NodeImplementation<TYPES> + std::fmt::Debug, V: Versions>
    EventTransformerState<TYPES, I, V> for DishonestVoter<TYPES>
{
    async fn recv_handler(&mut self, event: &HotShotEvent<TYPES>) -> Vec<HotShotEvent<TYPES>> {
        vec![event.clone()]
    }

    async fn send_handler(
        &mut self,
        event: &HotShotEvent<TYPES>,
        public_key: &TYPES::SignatureKey,
        private_key: &<TYPES::SignatureKey as SignatureKey>::PrivateKey,
        upgrade_lock: &UpgradeLock<TYPES, V>,
        _consensus: OuterConsensus<TYPES>,
        _membership_coordinator: EpochMembershipCoordinator<TYPES>,
        _network: Arc<I::Network>,
    ) -> Vec<HotShotEvent<TYPES>> {
        match event {
            HotShotEvent::QuorumProposalRecv(proposal, _sender) => {
                // Check if view is a dishonest proposal, if true send a vote
                let dishonest_proposals = self.dishonest_proposal_view_numbers.read().await;
                if dishonest_proposals.contains(&proposal.data.view_number()) {
                    // Create a vote using data from most recent vote and the current event number
                    // We wont update internal consensus state for this Byzantine replica but we are at least
                    // Going to send a vote to the next honest leader
                    let vote = QuorumVote2::<TYPES>::create_signed_vote(
                        self.votes_sent.last().unwrap().data.clone(),
                        event.view_number().unwrap(),
                        public_key,
                        private_key,
                        upgrade_lock,
                    )
                    .await
                    .context("Failed to sign vote")
                    .unwrap();
                    return vec![HotShotEvent::QuorumVoteSend(vote)];
                }
            },
            HotShotEvent::TimeoutVoteSend(vote) => {
                // Check if this view was a dishonest proposal view, if true dont send timeout
                let dishonest_proposals = self.dishonest_proposal_view_numbers.read().await;
                if dishonest_proposals.contains(&vote.view_number) {
                    // We craft the vote upon `QuorumProposalRecv` and send out a vote.
                    // So, dont send the timeout to the next leader from this byzantine replica
                    return vec![];
                }
            },
            HotShotEvent::QuorumVoteSend(vote) => {
                self.votes_sent.push(vote.clone());
            },
            _ => {},
        }
        vec![event.clone()]
    }
}

/// Implements a byzantine behaviour which aims at splitting the honest nodes during view sync protocol
/// so that the honest nodes cannot view sync on their own.
///
/// Requirement: The scenario requires at least 4 dishonest nodes so total number of nodes need to be
/// at least 13.
///
/// Scenario:
/// 1. The first dishonest leader sends a proposal to only f + 1 honest nodes and f dishonest nodes
/// 2. The second dishonest leader sends a proposal to only f + 1 honest nodes.
/// 3. All dishonest nodes do not send timeout votes.
/// 4. The first dishonest relay sends a correctly formed precommit certificate to f + 1 honest nodes
///    and f dishonest nodes.
/// 5. The first dishonest relay sends a correctly formed commit certificate to only one honest node.
/// 6. The second dishonest relay behaves in the same way as the first dishonest relay.
#[derive(Debug)]
pub struct DishonestViewSyncRelay {
    pub dishonest_proposal_view_numbers: Vec<u64>,
    pub dishonest_vote_view_numbers: Vec<u64>,
    pub first_f_honest_nodes: Vec<u64>,
    pub second_f_honest_nodes: Vec<u64>,
    pub one_honest_node: u64,
    pub f_dishonest_nodes: Vec<u64>,
}

#[async_trait]
impl<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> EventTransformerState<TYPES, I, V>
    for DishonestViewSyncRelay
{
    async fn send_handler(
        &mut self,
        event: &HotShotEvent<TYPES>,
        _public_key: &TYPES::SignatureKey,
        _private_key: &<TYPES::SignatureKey as SignatureKey>::PrivateKey,
        upgrade_lock: &UpgradeLock<TYPES, V>,
        _consensus: OuterConsensus<TYPES>,
        membership_coordinator: EpochMembershipCoordinator<TYPES>,
        network: Arc<I::Network>,
    ) -> Vec<HotShotEvent<TYPES>> {
        match event {
            HotShotEvent::QuorumProposalSend(proposal, sender) => {
                let view_number = proposal.data.view_number();
                if !self.dishonest_proposal_view_numbers.contains(&view_number) {
                    return vec![event.clone()];
                }
                let message_kind = if upgrade_lock.epochs_enabled(view_number).await {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::General(
                        GeneralConsensusMessage::Proposal2(convert_proposal(proposal.clone())),
                    ))
                } else {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::General(
                        GeneralConsensusMessage::Proposal(convert_proposal(proposal.clone())),
                    ))
                };
                let message = Message {
                    sender: sender.clone(),
                    kind: message_kind,
                };
                let serialized_message = match upgrade_lock.serialize(&message).await {
                    Ok(serialized) => serialized,
                    Err(e) => {
                        panic!("Failed to serialize message: {e}");
                    },
                };
                let second_f_honest_it = self.second_f_honest_nodes.iter();
                let f_dishonest_it = self.f_dishonest_nodes.iter();
                let one_honest_it = once(&self.one_honest_node);
                let chained_it: Box<dyn Iterator<Item = &u64> + Send> =
                    if &*view_number == self.dishonest_proposal_view_numbers.first().unwrap() {
                        // The first dishonest proposal is sent to f + 1 honest nodes and f dishonest nodes
                        Box::new(second_f_honest_it.chain(one_honest_it.chain(f_dishonest_it)))
                    } else {
                        // All other dishonest proposals are sent to f + 1 honest nodes
                        Box::new(second_f_honest_it.chain(one_honest_it))
                    };
                for node_id in chained_it {
                    let dummy_view = TYPES::View::new(*node_id);
                    let Ok(node) = membership_coordinator
                        .membership()
                        .read()
                        .await
                        .leader(dummy_view, proposal.data.epoch())
                    else {
                        panic!(
                            "Failed to find leader for view {} and epoch {:?}",
                            dummy_view,
                            proposal.data.epoch()
                        );
                    };
                    let transmit_result = network
                        .direct_message(serialized_message.clone(), node.clone())
                        .await;
                    match transmit_result {
                        Ok(()) => tracing::info!(
                            "Sent proposal for view {} to node {}",
                            proposal.data.view_number(),
                            node_id
                        ),
                        Err(e) => panic!("Failed to send message task: {e:?}"),
                    }
                }
                vec![]
            },
            HotShotEvent::QuorumVoteSend(vote) => {
                if !self.dishonest_vote_view_numbers.contains(&vote.view_number) {
                    return vec![event.clone()];
                }
                vec![]
            },
            HotShotEvent::TimeoutVoteSend(vote) => {
                if !self.dishonest_vote_view_numbers.contains(&vote.view_number) {
                    return vec![event.clone()];
                }
                vec![]
            },
            HotShotEvent::ViewSyncPreCommitVoteSend(vote) => {
                if !self.dishonest_vote_view_numbers.contains(&vote.view_number) {
                    return vec![event.clone()];
                }
                vec![]
            },
            HotShotEvent::ViewSyncPreCommitCertificateSend(certificate, sender) => {
                let view_number = certificate.data.round;
                if !self.dishonest_proposal_view_numbers.contains(&view_number) {
                    return vec![event.clone()];
                }
                let message_kind = if upgrade_lock.epochs_enabled(view_number).await {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::General(
                        GeneralConsensusMessage::ViewSyncPreCommitCertificate2(certificate.clone()),
                    ))
                } else {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::General(
                        GeneralConsensusMessage::ViewSyncPreCommitCertificate(
                            certificate.clone().to_vsc(),
                        ),
                    ))
                };
                let message = Message {
                    sender: sender.clone(),
                    kind: message_kind,
                };
                let serialized_message = match upgrade_lock.serialize(&message).await {
                    Ok(serialized) => serialized,
                    Err(e) => {
                        panic!("Failed to serialize message: {e}");
                    },
                };
                let second_f_honest_it = self.second_f_honest_nodes.iter();
                let f_dishonest_it = self.f_dishonest_nodes.iter();
                let one_honest_it = once(&self.one_honest_node);
                // The pre-commit certificate is sent to f + 1 honest nodes and f dishonest nodes
                let chained_it: Box<dyn Iterator<Item = &u64> + Send> =
                    Box::new(second_f_honest_it.chain(one_honest_it.chain(f_dishonest_it)));
                for node_id in chained_it {
                    let dummy_view = TYPES::View::new(*node_id);
                    let Ok(node) = membership_coordinator
                        .membership()
                        .read()
                        .await
                        .leader(dummy_view, certificate.epoch())
                    else {
                        panic!(
                            "Failed to find leader for view {} and epoch {:?}",
                            dummy_view,
                            certificate.epoch()
                        );
                    };
                    let transmit_result = network
                        .direct_message(serialized_message.clone(), node.clone())
                        .await;
                    match transmit_result {
                        Ok(()) => tracing::info!(
                            "Sent ViewSyncPreCommitCertificate for view {} to node {}",
                            view_number,
                            node_id
                        ),
                        Err(e) => panic!("Failed to send message task: {e:?}"),
                    }
                }
                vec![]
            },
            HotShotEvent::ViewSyncCommitCertificateSend(certificate, sender) => {
                let view_number = certificate.data.round;
                if !self.dishonest_proposal_view_numbers.contains(&view_number) {
                    return vec![event.clone()];
                }
                let message_kind = if upgrade_lock.epochs_enabled(view_number).await {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::General(
                        GeneralConsensusMessage::ViewSyncCommitCertificate2(certificate.clone()),
                    ))
                } else {
                    MessageKind::<TYPES>::from_consensus_message(SequencingMessage::General(
                        GeneralConsensusMessage::ViewSyncCommitCertificate(
                            certificate.clone().to_vsc(),
                        ),
                    ))
                };
                let message = Message {
                    sender: sender.clone(),
                    kind: message_kind,
                };
                let serialized_message = match upgrade_lock.serialize(&message).await {
                    Ok(serialized) => serialized,
                    Err(e) => {
                        panic!("Failed to serialize message: {e}");
                    },
                };
                let one_honest_it = once(&self.one_honest_node);
                // The commit certificate is sent to 1 honest node
                let chained_it: Box<dyn Iterator<Item = &u64> + Send> = Box::new(one_honest_it);
                for node_id in chained_it {
                    let dummy_view = TYPES::View::new(*node_id);
                    let Ok(node) = membership_coordinator
                        .membership()
                        .read()
                        .await
                        .leader(dummy_view, certificate.epoch())
                    else {
                        panic!(
                            "Failed to find leader for view {} and epoch {:?}",
                            dummy_view,
                            certificate.epoch()
                        );
                    };
                    let transmit_result = network
                        .direct_message(serialized_message.clone(), node.clone())
                        .await;
                    match transmit_result {
                        Ok(()) => tracing::info!(
                            "Sent ViewSyncCommitCertificate for view {} to node {}",
                            view_number,
                            node_id
                        ),
                        Err(e) => panic!("Failed to send message task: {e:?}"),
                    }
                }
                vec![]
            },
            _ => vec![event.clone()],
        }
    }

    async fn recv_handler(&mut self, event: &HotShotEvent<TYPES>) -> Vec<HotShotEvent<TYPES>> {
        vec![event.clone()]
    }
}

#[derive(Debug)]
pub struct DishonestViewSyncWrongEpoch<TYPES: NodeType> {
    pub first_dishonest_view_number: u64,
    pub epoch_modifier: fn(TYPES::Epoch) -> TYPES::Epoch,
}

#[async_trait]
impl<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> EventTransformerState<TYPES, I, V>
    for DishonestViewSyncWrongEpoch<TYPES>
{
    async fn send_handler(
        &mut self,
        event: &HotShotEvent<TYPES>,
        public_key: &TYPES::SignatureKey,
        private_key: &<TYPES::SignatureKey as SignatureKey>::PrivateKey,
        upgrade_lock: &UpgradeLock<TYPES, V>,
        _consensus: OuterConsensus<TYPES>,
        _membership_coordinator: EpochMembershipCoordinator<TYPES>,
        _network: Arc<I::Network>,
    ) -> Vec<HotShotEvent<TYPES>> {
        match event {
            HotShotEvent::QuorumProposalSend(proposal, _) => {
                if self.first_dishonest_view_number > proposal.data.view_number().u64() {
                    return vec![event.clone()];
                }
                vec![]
            },
            HotShotEvent::QuorumVoteSend(vote) => {
                if self.first_dishonest_view_number > vote.view_number().u64() {
                    return vec![event.clone()];
                }
                vec![]
            },
            HotShotEvent::TimeoutVoteSend(vote) => {
                if self.first_dishonest_view_number > vote.view_number().u64() {
                    return vec![event.clone()];
                }
                vec![]
            },
            HotShotEvent::ViewSyncPreCommitVoteSend(vote) => {
                if self.first_dishonest_view_number > vote.view_number().u64() {
                    return vec![event.clone()];
                }
                let view_number = vote.data.round;
                let vote = if upgrade_lock.epochs_enabled(view_number).await {
                    ViewSyncPreCommitVote2::<TYPES>::create_signed_vote(
                        ViewSyncPreCommitData2 {
                            relay: 0,
                            round: view_number,
                            epoch: vote.data.epoch.map(self.epoch_modifier),
                        },
                        view_number,
                        public_key,
                        private_key,
                        upgrade_lock,
                    )
                    .await
                    .context("Failed to sign pre commit vote!")
                    .unwrap()
                } else {
                    let vote = ViewSyncPreCommitVote::<TYPES>::create_signed_vote(
                        ViewSyncPreCommitData {
                            relay: 0,
                            round: view_number,
                        },
                        view_number,
                        public_key,
                        private_key,
                        upgrade_lock,
                    )
                    .await
                    .context("Failed to sign pre commit vote!")
                    .unwrap();
                    vote.to_vote2()
                };
                vec![HotShotEvent::ViewSyncPreCommitVoteSend(vote)]
            },
            _ => vec![event.clone()],
        }
    }

    async fn recv_handler(&mut self, event: &HotShotEvent<TYPES>) -> Vec<HotShotEvent<TYPES>> {
        vec![event.clone()]
    }
}
