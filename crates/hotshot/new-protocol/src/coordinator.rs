pub mod error;
pub mod timer;

use bon::Builder;
use hotshot::traits::NodeImplementation;
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    simple_certificate::{QuorumCertificate2, TimeoutCertificate2},
    simple_vote::{HasEpoch, QuorumVote2, TimeoutVote2},
    traits::{block_contents::BlockHeader, node_implementation::NodeType},
    vote::HasViewNumber,
};
use tokio::select;
use tracing::{error, warn};

use crate::{
    block::BlockBuilder,
    consensus::{Consensus, ConsensusInput, ConsensusOutput},
    coordinator::{
        error::{CoordinatorError, ErrorSource, Severity},
        timer::Timer,
    },
    epoch::{EpochManager, EpochRootResult},
    logging::KeyPrefix,
    message::{
        self, BlockMessage, Certificate2, CheckpointCertificate, CheckpointVote, ConsensusMessage,
        Message, MessageType, ProposalMessage, TimeoutOneHonest, TransactionMessage, Unchecked,
        Vote2,
    },
    network::Network,
    outbox::Outbox,
    proposal::ProposalValidator,
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
    vid_disperser: VidDisperser<T>,
    vid_reconstructor: VidReconstructor<T>,
    vote1_collector: VoteCollector<T, QuorumVote2<T>, QuorumCertificate2<T>>,
    vote2_collector: VoteCollector<T, Vote2<T>, Certificate2<T>>,
    timeout_collector: VoteCollector<T, TimeoutVote2<T>, TimeoutCertificate2<T>>,
    timeout_one_honest_collector: VoteCollector<T, TimeoutVote2<T>, TimeoutOneHonest<T>>,
    checkpoint_collector: VoteCollector<T, CheckpointVote<T>, CheckpointCertificate<T>>,
    epoch_manager: EpochManager<T>,
    block_builder: BlockBuilder<T>,
    proposal_validator: ProposalValidator<T>,
    #[builder(default)]
    outbox: Outbox<ConsensusOutput<T>>,
    public_key: T::SignatureKey,
    #[builder(default = KeyPrefix::from(&public_key))]
    node_id: KeyPrefix,
    timer: Timer,
}

impl<T: NodeType, I: NodeImplementation<T>> Coordinator<T, I> {
    /// Convenience method to run the coordinator event loop.
    ///
    /// Combines `next_consensus_input`, `apply_consensus` and outbox processing.
    pub async fn run(mut self) {
        loop {
            match self.next_consensus_input().await {
                Ok(input) => self.apply_consensus(input).await,
                Err(err) if err.severity == Severity::Critical => {
                    error!(%err, "while awaiting next consensus input");
                    return;
                },
                Err(err) => {
                    warn!(%err, "while awaiting next consensus input");
                },
            }
            while let Some(output) = self.outbox.pop_front() {
                if let Err(err) = self.process_consensus_output(output).await {
                    if err.severity == Severity::Critical {
                        error!(%err, "while processing consensus output");
                        return;
                    } else {
                        warn!(%err, "while processing consensus output");
                    }
                }
            }
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
                    let input = ConsensusInput::Timeout(self.timer.view(), self.timer.epoch());
                    // Timer is only reset so we can resend the timeout vote
                    // This isn't strictly necessary for the protocol, but it's a good idea to
                    // resend the timeout vote to avoid a situation where the network is stuck
                    // view because we fail to form a timeout certificate.
                    self.timer.reset();
                    return Ok(input)
                }
                Some(output) = self.state_manager.next() => {
                    if let Some(input) = self.on_state_manager_output(output).await {
                        return Ok(input)
                    }
                }
                Some(tcert) = self.timeout_collector.next() => {
                    return Ok(ConsensusInput::TimeoutCertificate(tcert))
                }
                Some(out) = self.timeout_one_honest_collector.next() => {
                    let Some(epoch) = out.data.epoch else {
                        let msg = format!("missing epoch in view {}", out.view_number());
                        return Err(CoordinatorError::regular(msg).context("gc timeout one honest"))
                    };
                    return Ok(ConsensusInput::TimeoutOneHonest(out.view_number(), epoch))
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
                ConsensusMessage::TimeoutVote(timeout_msg) => {
                    self.timeout_collector
                        .accumulate_vote(timeout_msg.vote.clone())
                        .await;
                    self.timeout_one_honest_collector
                        .accumulate_vote(timeout_msg.vote)
                        .await;
                    None
                },
                ConsensusMessage::TimeoutCertificate(tc) => {
                    Some(ConsensusInput::TimeoutCertificate(tc))
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
            MessageType::External(_) => todo!(),
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
            ConsensusOutput::LeafDecided(leaves) => {
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
                            ProposalMessage::validated(proposal.clone(), vid_share),
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
            ConsensusOutput::SendTimeoutVote(vote, lock) => {
                let message = Message {
                    sender: self.public_key.clone(),
                    message_type: MessageType::Consensus(ConsensusMessage::TimeoutVote(
                        message::TimeoutVoteMessage { vote, lock },
                    )),
                };
                self.network
                    .broadcast(message)
                    .await
                    .map_err(|e| CoordinatorError::from(e).context("broadcast timeout vote"))?
            },
            ConsensusOutput::SendTimeoutCertificate(tc, view, epoch) => {
                if let Some(leader) = self.leader(view, epoch).await {
                    let message = Message {
                        sender: self.public_key.clone(),
                        message_type: MessageType::Consensus(ConsensusMessage::TimeoutCertificate(
                            tc,
                        )),
                    };
                    self.network
                        .unicast(leader, message)
                        .await
                        .map_err(|e| CoordinatorError::from(e).context("timeout certificate"))?;
                }
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
            ConsensusOutput::ViewChanged(view, epoch) => {
                self.timer.reset_with_epoch(view, epoch);
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
        self.checkpoint_collector.gc(view, epoch);
        self.network.gc(view, epoch);
        self.state_manager.gc(view);
        self.vid_disperser.gc(view);
        self.vid_reconstructor.gc(view);
        self.vote1_collector.gc(view, epoch);
        self.vote2_collector.gc(view, epoch);
        self.timeout_collector.gc(view, epoch);
        self.timeout_one_honest_collector.gc(view, epoch);
        self.epoch_manager.gc(epoch);
        self.block_builder.gc(view);
    }

    pub fn node_id(&self) -> &KeyPrefix {
        &self.node_id
    }
}
