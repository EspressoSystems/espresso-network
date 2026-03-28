pub mod error;
pub(crate) mod timer;

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
    consensus::{Consensus, ConsensusInput, ConsensusOutput},
    coordinator::{
        error::{CoordinatorError, ErrorKind, Severity},
        timer::Timer,
    },
    epoch::{EpochManager, EpochRootResult},
    message::{
        Certificate2, CheckpointCertificate, CheckpointVote, ConsensusMessage, Message,
        MessageType, ProposalMessage, Vote2,
    },
    network::Network,
    outbox::Outbox,
    state::{StateManager, StateManagerOutput},
    vid::{VidDisperseRequest, VidDisperser, VidReconstructor, VidShareInput},
    vote::VoteCollector,
};

#[derive(Builder)]
pub struct Coordinator<T: NodeType, I: NodeImplementation<T>> {
    _membership_coordinator: EpochMembershipCoordinator<T>,
    consensus: Consensus<T>,
    network: Network<T, I::Network>,
    state_manager: StateManager<T>,
    vid_disperser: VidDisperser<T>,
    vid_reconstructor: VidReconstructor<T>,
    vote1_collector: VoteCollector<T, QuorumVote2<T>, QuorumCertificate2<T>>,
    vote2_collector: VoteCollector<T, Vote2<T>, Certificate2<T>>,
    timeout_collector: VoteCollector<T, TimeoutVote2<T>, TimeoutCertificate2<T>>,
    checkpoint_collector: VoteCollector<T, CheckpointVote<T>, CheckpointCertificate<T>>,
    epoch_manager: EpochManager<T>,
    #[builder(default)]
    outbox: Outbox<ConsensusOutput<T>>,
    public_key: T::SignatureKey,
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
                    let input = ConsensusInput::Timeout(self.timer.view());
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
                Some(cert1) = self.vote1_collector.next() => {
                    return Ok(ConsensusInput::Certificate1(cert1))
                }
                Some(cert2) = self.vote2_collector.next() => {
                    return Ok(ConsensusInput::Certificate2(cert2))
                }
                Some(cert) = self.checkpoint_collector.next() => {
                    let Some(epoch) = cert.epoch() else {
                        let msg = format!("missing epoch in view {}", cert.view_number());
                        return Err(CoordinatorError::critical(msg).context("gc certificate"))
                    };
                    self.gc(cert.view_number(), epoch);
                }
                Some(item) = self.vid_disperser.next() => match item {
                    Ok((view, _, disperse)) => {
                        return Ok(ConsensusInput::VidDisperseCreated(view, disperse))
                    }
                    Err(()) => {
                        return Err(CoordinatorError::unspecified().context("vid disperse"))
                    }
                },
                Some(item) = self.vid_reconstructor.next() => match item {
                    Ok((view, commitment, _)) => {
                        return Ok(ConsensusInput::BlockReconstructed(view, commitment))
                    }
                    Err(()) => {
                        return Err(CoordinatorError::unspecified().context("vid reconstruction"))
                    }
                },
                Some(result) = self.epoch_manager.next() => match result {
                    Ok(EpochRootResult::DrbResult(epoch, drb_result)) => {
                        return Ok(ConsensusInput::DrbResult(epoch, drb_result))
                    }
                    Ok(EpochRootResult::RootAdded(epoch)) => {}
                    Err(_) => {
                        return Err(CoordinatorError::unspecified().context("epoch root"))
                    }
                },
                else => {
                    return Err(CoordinatorError::critical(ErrorKind::NoInput))
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

    pub async fn on_network_message(&mut self, msg: Message<T>) -> Option<ConsensusInput<T>> {
        match msg.message_type {
            MessageType::Consensus(msg) => match msg {
                ConsensusMessage::Proposal(proposal) => {
                    self.vid_reconstructor.handle_vid_share(VidShareInput {
                        share: proposal.vid_share.clone(),
                        metadata: Some(proposal.proposal.data.block_header.metadata().clone()),
                    });
                    Some(ConsensusInput::Proposal(proposal))
                },
                ConsensusMessage::Vote1(vote1) => {
                    self.vote1_collector.accumulate_vote(vote1.vote).await;
                    self.vid_reconstructor.handle_vid_share(VidShareInput {
                        share: vote1.vid_share,
                        metadata: None,
                    });
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
                ConsensusMessage::Transactions(..) => {
                    todo!()
                },
                ConsensusMessage::Checkpoint(checkpoint) => {
                    self.checkpoint_collector.accumulate_vote(checkpoint).await;
                    None
                },
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
            ConsensusOutput::RequestBlockAndHeader(_) => {}, // TODO
            ConsensusOutput::RequestProposal(..) => {}, // TODO
            ConsensusOutput::SendProposal(proposal, vid_disperse) => {
                // TODO: This may be done async in network so we do not spend
                // too much time here in this loop.
                for vid_share in vid_disperse.to_shares() {
                    let recipient_key = vid_share.recipient_key.clone();
                    let message = Message {
                        sender: self.public_key.clone(),
                        message_type: MessageType::Consensus(ConsensusMessage::Proposal(
                            ProposalMessage {
                                proposal: proposal.clone(),
                                vid_share,
                            },
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
            ConsensusOutput::ViewChanged(view, _) => {
                self.timer.reset_with(view);
            },
        }
        Ok(())
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
    }
}
