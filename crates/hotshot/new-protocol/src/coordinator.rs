pub mod error;
pub(crate) mod timer;

use std::{sync::Arc, time::Duration};

use bon::Builder;
use hotshot::traits::NodeImplementation;
use hotshot_types::{
    data::{EpochNumber, Leaf2, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    simple_certificate::{QuorumCertificate2, TimeoutCertificate2},
    simple_vote::{HasEpoch, QuorumVote2, TimeoutVote2},
    traits::{
        block_contents::BlockHeader, node_implementation::NodeType, signature_key::SignatureKey,
    },
    vote::HasViewNumber,
};
use tokio::{select, sync::mpsc};
use tracing::warn;

use crate::{
    block::{BlockBuilder, BlockBuilderConfig},
    consensus::{Consensus, ConsensusInput, ConsensusOutput},
    coordinator::{
        error::{CoordinatorError, ErrorSource, Severity},
        timer::Timer,
    },
    epoch::{EpochManager, EpochRootResult},
    helpers::upgrade_lock,
    message::{
        BlockMessage, Certificate2, CheckpointCertificate, CheckpointVote, ConsensusMessage,
        Message, MessageType, ProposalMessage, TransactionMessage, Unchecked, Vote2,
    },
    network::Network,
    outbox::Outbox,
    proposal::ProposalValidator,
    query::CoordinatorQuery,
    state::{HeaderRequest, StateManager, StateManagerOutput},
    vid::{VidDisperseRequest, VidDisperser, VidReconstructor},
    vote::VoteCollector,
};

#[derive(Builder)]
pub struct Coordinator<T: NodeType, I: NodeImplementation<T>> {
    membership_coordinator: EpochMembershipCoordinator<T>,
    consensus: Consensus<T>,
    network: Network<T, I::Network>,
    state_manager: StateManager<T>,
    query_rx: mpsc::Receiver<CoordinatorQuery<T>>,
    vid_disperser: VidDisperser<T>,
    vid_reconstructor: VidReconstructor<T>,
    vote1_collector: VoteCollector<T, QuorumVote2<T>, QuorumCertificate2<T>>,
    vote2_collector: VoteCollector<T, Vote2<T>, Certificate2<T>>,
    timeout_collector: VoteCollector<T, TimeoutVote2<T>, TimeoutCertificate2<T>>,
    checkpoint_collector: VoteCollector<T, CheckpointVote<T>, CheckpointCertificate<T>>,
    epoch_manager: EpochManager<T>,
    block_builder: BlockBuilder<T>,
    proposal_validator: ProposalValidator<T>,
    #[builder(default)]
    outbox: Outbox<ConsensusOutput<T>>,
    public_key: T::SignatureKey,
    timer: Timer,
}

impl<T: NodeType, I: NodeImplementation<T>> Coordinator<T, I> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        membership_coordinator: EpochMembershipCoordinator<T>,
        network: I::Network,
        instance_state: Arc<T::InstanceState>,
        public_key: T::SignatureKey,
        private_key: <T::SignatureKey as SignatureKey>::PrivateKey,
        genesis_leaf: Leaf2<T>,
        epoch_height: u64,
        timeout_duration: Duration,
    ) -> (Self, mpsc::Sender<CoordinatorQuery<T>>) {
        let consensus = Consensus::new(
            membership_coordinator.clone(),
            public_key.clone(),
            private_key,
            genesis_leaf,
            epoch_height,
        );
        let state_manager = StateManager::new(instance_state.clone());
        let (query_tx, query_rx) = mpsc::channel(256);

        let lock = upgrade_lock();
        let coordinator = Self::builder()
            .consensus(consensus)
            .network(Network::new(
                network,
                membership_coordinator.clone(),
                lock.clone(),
            ))
            .state_manager(state_manager)
            .query_rx(query_rx)
            .vid_disperser(VidDisperser::new(membership_coordinator.clone()))
            .vid_reconstructor(VidReconstructor::new())
            .vote1_collector(VoteCollector::new(
                membership_coordinator.clone(),
                lock.clone(),
            ))
            .vote2_collector(VoteCollector::new(
                membership_coordinator.clone(),
                lock.clone(),
            ))
            .timeout_collector(VoteCollector::new(
                membership_coordinator.clone(),
                lock.clone(),
            ))
            .checkpoint_collector(VoteCollector::new(membership_coordinator.clone(), lock))
            .epoch_manager(EpochManager::new(
                epoch_height,
                membership_coordinator.clone(),
            ))
            .block_builder(BlockBuilder::new(
                instance_state,
                membership_coordinator.clone(),
                BlockBuilderConfig::default(),
            ))
            .proposal_validator(ProposalValidator::new(membership_coordinator.clone()))
            .membership_coordinator(membership_coordinator)
            .timer(Timer::new(timeout_duration, ViewNumber::genesis()))
            .public_key(public_key)
            .build();

        (coordinator, query_tx)
    }

    fn handle_query(&mut self, query: CoordinatorQuery<T>) {
        match query {
            CoordinatorQuery::CurView(tx) => {
                let _ = tx.send(self.consensus.cur_view());
            },
            CoordinatorQuery::CurEpoch(tx) => {
                let _ = tx.send(self.consensus.cur_epoch());
            },
            CoordinatorQuery::DecidedLeaf(tx) => {
                let _ = tx.send(self.consensus.last_decided_leaf().clone());
            },
            CoordinatorQuery::DecidedState(tx) => {
                let view = self.consensus.last_decided_leaf().view_number();
                let state = self
                    .state_manager
                    .get_state(&view)
                    .expect("no state for decided leaf");
                let _ = tx.send(state);
            },
            CoordinatorQuery::UndecidedLeaves(tx) => {
                let _ = tx.send(self.consensus.undecided_leaves());
            },
            CoordinatorQuery::GetState { view, respond } => {
                let _ = respond.send(self.state_manager.get_state(&view));
            },
            CoordinatorQuery::GetStateAndDelta { view, respond } => {
                let _ = respond.send(self.state_manager.get_state_and_delta(&view));
            },
        }
    }

    pub async fn next_consensus_input(&mut self) -> Result<ConsensusInput<T>, CoordinatorError> {
        loop {
            select! {
                message = self.network.receive() => match message {
                    Ok(m) => {
                        if let Some(input) = self.on_network_message(m).await {
                            return Ok(input)
                        }
                    }
                    Err(e) => {
                        return Err(CoordinatorError::from(e).context("network receive"))
                    }
                },
                () = &mut self.timer => {
                    let input = ConsensusInput::Timeout(self.timer.view());
                    self.timer.reset();
                    return Ok(input)
                }
                Some(output) = self.state_manager.next() => {
                    if let Some(input) = self.on_state_manager_output(output).await {
                        return Ok(input)
                    }
                }
                Some(query) = self.query_rx.recv() => {
                    self.handle_query(query);
                }
                Some(tcert) = self.timeout_collector.next() => {
                    return Ok(ConsensusInput::TimeoutCertificate(tcert))
                }
                Some(cert1) = self.vote1_collector.next() => {
                    return Ok(ConsensusInput::Certificate1(cert1))
                }
                Some(cert2) = self.vote2_collector.next() => {
                    return Ok(ConsensusInput::Certificate2(cert2))
                }
                Some(item) = self.proposal_validator.next() => match item {
                    Ok(p) => {
                        let s = p.vid_share.clone();
                        let m = p.proposal.data.block_header.metadata().clone();
                        self.vid_reconstructor.handle_vid_share(s, m);
                        self.outbox.push_back(ConsensusOutput::ProposalReceived {
                            proposal: p.proposal.clone(),
                            sender: p.sender.clone(),
                        });
                        return Ok(ConsensusInput::Proposal(p))
                    }
                    Err(e) => {
                        return Err(CoordinatorError::regular(e).context("proposal validation"))
                    }
                },
                Some(cert) = self.checkpoint_collector.next() => {
                    let Some(epoch) = cert.epoch() else {
                        let msg = format!("missing epoch in view {}", cert.view_number());
                        return Err(CoordinatorError::critical(msg).context("gc certificate"))
                    };
                    self.gc(cert.view_number(), epoch);
                }
                Some(item) = self.block_builder.next() => match item {
                    Ok(block) => {
                        self.state_manager.request_header(HeaderRequest::from(&block));
                        let next_view = block.view + 1;
                        let epoch = block.epoch;
                        let manifest = block.manifest.clone();
                        self.unicast_to_leader(
                            next_view,
                            epoch,
                            BlockMessage::DedupManifest(manifest),
                        )
                        .await?;
                        return Ok(block.into())
                    }
                    Err(err) => {
                        return Err(CoordinatorError::regular(err).context("block building"))
                    }
                },
                Some(item) = self.vid_disperser.next() => match item {
                    Ok(out) => {
                        return Ok(ConsensusInput::VidDisperseCreated(out.view, out.disperse))
                    }
                    Err(()) => {
                        return Err(CoordinatorError::unspecified().context("vid disperse"))
                    }
                },
                Some(item) = self.vid_reconstructor.next() => match item {
                    Ok(out) => {
                        self.block_builder.on_block_reconstructed(out.tx_commitments);
                        return Ok(ConsensusInput::BlockReconstructed(out.view, out.payload_commitment))
                    }
                    Err(()) => {
                        return Err(CoordinatorError::unspecified().context("vid reconstruction"))
                    }
                },
                Some(result) = self.epoch_manager.next() => match result {
                    Ok(EpochRootResult::DrbResult(epoch, drb_result)) => {
                        return Ok(ConsensusInput::DrbResult(epoch, drb_result))
                    }
                    Ok(EpochRootResult::RootAdded(_epoch)) => {}
                    Err(err) => {
                        return Err(CoordinatorError::regular(err))
                    }
                },
                else => {
                    return Err(CoordinatorError::critical(ErrorSource::NoInput))
                }
            }
        }
    }

    pub async fn apply_consensus(&mut self, input: ConsensusInput<T>) {
        self.consensus.apply(input, &mut self.outbox).await;
    }

    pub fn outbox(&self) -> &Outbox<ConsensusOutput<T>> {
        &self.outbox
    }

    pub fn outbox_mut(&mut self) -> &mut Outbox<ConsensusOutput<T>> {
        &mut self.outbox
    }

    pub async fn on_state_manager_output(
        &mut self,
        output: StateManagerOutput<T>,
    ) -> Option<ConsensusInput<T>> {
        match output {
            StateManagerOutput::State {
                response,
                validated: true,
            } => Some(ConsensusInput::StateValidated(response)),
            StateManagerOutput::State {
                response,
                validated: false,
            } => Some(ConsensusInput::StateValidationFailed(response)),
            StateManagerOutput::Header {
                response,
                header: Some(hdr),
            } => Some(ConsensusInput::HeaderCreated(response.view, hdr)),
            StateManagerOutput::Header {
                response: _,
                header: None,
            } => {
                todo!()
            },
        }
    }

    pub async fn on_network_message(
        &mut self,
        message: Message<T, Unchecked>,
    ) -> Option<ConsensusInput<T>> {
        match message.message_type {
            MessageType::Consensus(msg) => match msg {
                ConsensusMessage::Proposal(p) => {
                    if self.consensus.wants_proposal(&p) {
                        self.proposal_validator.validate(p);
                    }
                    None
                },
                ConsensusMessage::Vote1(vote1) => {
                    self.vote1_collector.accumulate_vote(vote1.vote).await;
                    self.vid_reconstructor
                        .handle_vid_share(vote1.vid_share, None);
                    None
                },
                ConsensusMessage::Vote2(vote2) => {
                    self.vote2_collector.accumulate_vote(vote2).await;
                    None
                },
                ConsensusMessage::Certificate1(certificate1, _key) => {
                    Some(ConsensusInput::Certificate1(certificate1))
                },
                ConsensusMessage::Certificate2(certificate2, _key) => {
                    Some(ConsensusInput::Certificate2(certificate2))
                },
                ConsensusMessage::TimeoutVote(timeout_vote) => {
                    self.timeout_collector.accumulate_vote(timeout_vote).await;
                    None
                },
                ConsensusMessage::EpochChange(epoch_change) => {
                    Some(ConsensusInput::EpochChange(epoch_change))
                },
                ConsensusMessage::Checkpoint(checkpoint) => {
                    self.checkpoint_collector.accumulate_vote(checkpoint).await;
                    None
                },
            },
            MessageType::Block(msg) => {
                match msg {
                    BlockMessage::Transactions(msg) => self.block_builder.on_transactions(msg),
                    BlockMessage::DedupManifest(manifest) => {
                        if let Some(view_leader) = self.leader(manifest.view, manifest.epoch).await
                            && view_leader == message.sender
                        {
                            self.block_builder.on_dedup_manifest(manifest)
                        }
                    },
                }
                None
            },
            MessageType::ViewSync(_) => todo!(),
            MessageType::External(_data) => {
                // self.outbox
                //     .push_back(ConsensusOutput::ExternalMessageReceived {
                //         sender: message.sender,
                //         data,
                //     });
                None
            },
        }
    }

    pub async fn process_consensus_output(
        &mut self,
        output: ConsensusOutput<T>,
    ) -> Result<(), CoordinatorError> {
        match output {
            ConsensusOutput::RequestState(state_request) => {
                self.state_manager.request_state(state_request);
            },
            ConsensusOutput::RequestVidDisperse {
                view,
                epoch,
                payload,
                metadata,
            } => {
                self.vid_disperser.request_vid_disperse(VidDisperseRequest {
                    view,
                    epoch,
                    block: payload,
                    metadata,
                });
            },
            ConsensusOutput::RequestDrbResult(epoch) => {
                self.epoch_manager.request_drb_result(epoch);
            },
            ConsensusOutput::SendCheckpointVote(checkpoint_vote) => {
                let message = Message {
                    sender: self.public_key.clone(),
                    message_type: MessageType::Consensus(ConsensusMessage::Checkpoint(
                        checkpoint_vote,
                    )),
                };
                self.network
                    .broadcast(message)
                    .await
                    .map_err(|e| CoordinatorError::from(e).context("broadcast checkpoint vote"))?
            },
            ConsensusOutput::Certificate1Formed(_) => {}, // TODO
            ConsensusOutput::Certificate2Formed(_) => {}, // TODO
            ConsensusOutput::LeafDecided { leaves, .. } => {
                for leaf in leaves {
                    self.epoch_manager.handle_leaf_decided(leaf);
                }
            },
            ConsensusOutput::LockUpdated(_) => {}, // TODO
            ConsensusOutput::RequestBlockAndHeader(request) => {
                self.block_builder.request_block(request);
            },
            ConsensusOutput::RequestProposal(..) => {}, // TODO
            ConsensusOutput::SendProposal(proposal, vid_disperse) => {
                // TODO: This may be done async in network so we do not spend
                // too much time here in this loop.
                for vid_share in vid_disperse.to_shares() {
                    let recipient_key = vid_share.recipient_key.clone();
                    let message = Message {
                        sender: self.public_key.clone(),
                        message_type: MessageType::Consensus(ConsensusMessage::Proposal(
                            ProposalMessage::validated(
                                self.public_key.clone(),
                                proposal.clone(),
                                vid_share,
                            ),
                        )),
                    };
                    if let Err(err) = self.network.unicast(recipient_key, message).await {
                        let err = CoordinatorError::from(err).context("vid share unicast");
                        if err.severity == Severity::Critical {
                            return Err(err);
                        } else {
                            warn!(%err, "network error while sending vid share")
                        }
                    }
                }
            },
            ConsensusOutput::SendTimeoutVote(vote) => {
                let message = Message {
                    sender: self.public_key.clone(),
                    message_type: MessageType::Consensus(ConsensusMessage::TimeoutVote(vote)),
                };
                self.network
                    .broadcast(message)
                    .await
                    .map_err(|e| CoordinatorError::from(e).context("broadcast timeout vote"))?
            },
            ConsensusOutput::SendVote1(vote1) => {
                let message = Message {
                    sender: self.public_key.clone(),
                    message_type: MessageType::Consensus(ConsensusMessage::Vote1(vote1)),
                };
                self.network
                    .broadcast(message)
                    .await
                    .map_err(|e| CoordinatorError::from(e).context("broadcast vote1"))?
            },
            ConsensusOutput::SendVote2(vote2) => {
                let message = Message {
                    sender: self.public_key.clone(),
                    message_type: MessageType::Consensus(ConsensusMessage::Vote2(vote2)),
                };
                self.network
                    .broadcast(message)
                    .await
                    .map_err(|e| CoordinatorError::from(e).context("broadcast vote2"))?
            },
            ConsensusOutput::SendEpochChange(epoch_change) => {
                let message = Message {
                    sender: self.public_key.clone(),
                    message_type: MessageType::Consensus(ConsensusMessage::EpochChange(
                        epoch_change,
                    )),
                };
                self.network
                    .broadcast(message)
                    .await
                    .map_err(|e| CoordinatorError::from(e).context("broadcast epoch change"))?
            },
            ConsensusOutput::SendCertificate1(cert1) => {
                let message = Message {
                    sender: self.public_key.clone(),
                    message_type: MessageType::Consensus(ConsensusMessage::Certificate1(
                        cert1,
                        self.public_key.clone(),
                    )),
                };
                self.network
                    .broadcast(message)
                    .await
                    .map_err(|e| CoordinatorError::from(e).context("broadcast certificate1"))?
            },
            ConsensusOutput::SendCertificate2(cert2) => {
                let message = Message {
                    sender: self.public_key.clone(),
                    message_type: MessageType::Consensus(ConsensusMessage::Certificate2(
                        cert2,
                        self.public_key.clone(),
                    )),
                };
                self.network
                    .broadcast(message)
                    .await
                    .map_err(|e| CoordinatorError::from(e).context("broadcast certificate2"))?
            },
            ConsensusOutput::ProposalReceived { .. } => {},
            ConsensusOutput::ExternalMessageReceived { .. } => {},
            ConsensusOutput::ViewChanged(view, epoch) => {
                self.consensus.set_view(view, epoch);
                self.timer.reset_with(view);
                let txns = self.block_builder.on_view_changed(view, epoch);
                if !txns.is_empty() {
                    let next_view = view + 1;
                    self.unicast_to_leader(
                        next_view,
                        epoch,
                        BlockMessage::Transactions(TransactionMessage {
                            view: next_view,
                            transactions: txns,
                        }),
                    )
                    .await
                    .map_err(|e| e.context("unicast transactions"))?;
                }
            },
        }
        Ok(())
    }

    async fn unicast_to_leader(
        &mut self,
        view: ViewNumber,
        epoch: EpochNumber,
        msg: BlockMessage<T>,
    ) -> Result<(), CoordinatorError> {
        let Some(leader) = self.leader(view, epoch).await else {
            warn!(%view, %epoch, "failed to resolve leader for unicast");
            return Ok(());
        };
        let message = Message {
            sender: self.public_key.clone(),
            message_type: MessageType::Block(msg),
        };
        self.network
            .unicast(leader, message)
            .await
            .map_err(|e| CoordinatorError::from(e).context("leader unicast"))
    }

    async fn leader(&self, view: ViewNumber, epoch: EpochNumber) -> Option<T::SignatureKey> {
        let membership = self
            .membership_coordinator
            .membership_for_epoch(Some(epoch))
            .await
            .ok()?;
        membership.leader(view).await.ok()
    }

    fn gc(&mut self, view: ViewNumber, epoch: EpochNumber) {
        self.consensus.gc(view, epoch);
        self.checkpoint_collector.gc(view);
        self.network.gc(view, epoch);
        self.state_manager.gc(view);
        self.vid_disperser.gc(view);
        self.vid_reconstructor.gc(view);
        self.vote1_collector.gc(view);
        self.vote2_collector.gc(view);
        self.timeout_collector.gc(view);
        self.epoch_manager.gc(epoch);
        self.block_builder.gc(view);
    }
}
