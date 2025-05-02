use std::{
    collections::{BTreeMap, HashMap, HashSet},
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
    consensus::{Consensus, OuterConsensus},
    data::QuorumProposalWrapper,
    message::{Proposal, UpgradeLock},
    simple_vote::QuorumVote2,
    traits::node_implementation::{ConsensusTime, NodeImplementation, NodeType, Versions},
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
        consensus: Arc<RwLock<Consensus<TYPES>>>,
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
        _consensus: Arc<RwLock<Consensus<TYPES>>>,
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
        _consensus: Arc<RwLock<Consensus<TYPES>>>,
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
        _consensus: Arc<RwLock<Consensus<TYPES>>>,
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
        _consensus: Arc<RwLock<Consensus<TYPES>>>,
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
        _consensus: Arc<RwLock<Consensus<TYPES>>>,
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
            storage: Arc::clone(&handle.storage()),
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
        _consensus: Arc<RwLock<Consensus<TYPES>>>,
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
