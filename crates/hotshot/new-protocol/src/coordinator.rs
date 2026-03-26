use std::{fmt, time::Duration};

use bon::Builder;
use futures::{FutureExt, future::BoxFuture};
use hotshot::traits::{NetworkError, NodeImplementation};
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    simple_certificate::{QuorumCertificate2, TimeoutCertificate2},
    simple_vote::{HasEpoch, QuorumVote2, TimeoutVote2},
    traits::{block_contents::BlockHeader, node_implementation::NodeType},
    vote::HasViewNumber,
};
use tokio::{select, time::sleep};
use tracing::{error, warn};

use crate::{
    consensus::{Consensus, ConsensusInput, ConsensusOutput},
    drb::DrbRequester,
    message::{
        Certificate2, CheckpointCertificate, CheckpointVote, ConsensusMessage, Message,
        MessageType, Vote2,
    },
    network::{Network, is_critical},
    outbox::Outbox,
    state::{StateManager, StateManagerOutput},
    vid::{VidDisperseRequest, VidDisperser, VidReconstructor, VidShareInput},
    vote::VoteCollector,
};

pub struct Timer {
    pub(crate) timer: BoxFuture<'static, ViewNumber>,
    duration: Duration,
}

impl Timer {
    pub fn new(duration: Duration) -> Self {
        Self {
            timer: sleep(duration)
                .map(|_| ViewNumber::genesis())
                .fuse()
                .boxed(),
            duration,
        }
    }

    pub fn reset(&mut self, view: ViewNumber) {
        self.timer = sleep(self.duration).map(move |_| view).fuse().boxed();
    }
}

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
    drb_requester: DrbRequester,
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
                    Err(err) if is_critical(&err) => {
                        return Err(CoordinatorError::critical(err).context("network receive"))
                    }
                    Err(err) => {
                        return Err(CoordinatorError::regular(err).context("network receive"))
                    }
                },
                view = &mut self.timer.timer => {
                    self.timer.reset(view + 1);
                    return Ok(ConsensusInput::Timeout(view))
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
                Some(checkpoint_cert) = self.checkpoint_collector.next() => {
                    self.gc(checkpoint_cert.view_number(), checkpoint_cert.epoch().unwrap());
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
                Some((_epoch, _drb_result)) = self.drb_requester.next() => {
                    todo!()
                }
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
            ConsensusOutput::RequestDRB(drb_input) => {
                self.drb_requester.request_drb(drb_input);
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
                    .map_err(|e| CoordinatorError::from(e).context("network broadcast"))?
            },
            ConsensusOutput::Certificate1Formed(_) => {}, // TODO
            ConsensusOutput::Certificate2Formed(_) => {}, // TODO
            ConsensusOutput::LeafDecided(_) => {},        // TODO
            ConsensusOutput::LockUpdated(_) => {},        // TODO
            ConsensusOutput::RequestBlockAndHeader(_) => {}, // TODO
            ConsensusOutput::RequestProposal(..) => {},   // TODO
            ConsensusOutput::SendProposal(..) => {},      // TODO
            ConsensusOutput::SendVote1(..) => {},         // TODO
            ConsensusOutput::SendVote2(..) => {},         // TODO
            ConsensusOutput::TimeoutCertificateReceived(..) => {}, // TODO
            ConsensusOutput::ViewSyncCertificateReceived(_) => {}, // TODO
            ConsensusOutput::ViewChanged(..) => {},       // TODO
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
        self.checkpoint_collector.gc(view);
        self.drb_requester.gc(epoch);
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{severity} coordinator error: {context}")]
pub struct CoordinatorError {
    pub severity: Severity,
    pub source: ErrorKind,
    pub context: &'static str,
}

impl CoordinatorError {
    pub fn regular<E: Into<ErrorKind>>(e: E) -> Self {
        Self {
            context: "",
            severity: Severity::Regular,
            source: e.into(),
        }
    }

    pub fn critical<E: Into<ErrorKind>>(e: E) -> Self {
        Self {
            context: "",
            severity: Severity::Critical,
            source: e.into(),
        }
    }

    pub fn unspecified() -> Self {
        Self {
            context: "",
            severity: Severity::Unspecified,
            source: ErrorKind::Unspecified,
        }
    }

    pub fn context(mut self, m: &'static str) -> Self {
        self.context = m;
        self
    }
}

#[derive(Debug, Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Severity {
    Unspecified,
    Regular,
    Critical,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unspecified => f.write_str("unspecified"),
            Self::Regular => f.write_str("regular"),
            Self::Critical => f.write_str("critical"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ErrorKind {
    #[error("network error: {0}")]
    Network(#[from] NetworkError),

    #[error("unspecified error")]
    Unspecified,

    #[error("coordinator has no inputs")]
    NoInput,
}

impl From<NetworkError> for CoordinatorError {
    fn from(e: NetworkError) -> Self {
        if is_critical(&e) {
            Self::critical(e)
        } else {
            Self::regular(e)
        }
    }
}
