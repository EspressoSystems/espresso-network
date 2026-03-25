use std::time::Duration;

use bon::Builder;
use futures::{FutureExt, future::BoxFuture};
use hotshot::{traits::NodeImplementation, types::SystemContextHandle};
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
    Outbox,
    consensus::Consensus,
    drb::DrbRequester,
    events::*,
    io::network::{Network, is_critical},
    message::{
        Certificate2, CheckpointCertificate, CheckpointVote, ConsensusMessage, Message,
        MessageType, ProposalMessage, Vote2,
    },
    validated_state::ValidatedStateManager,
    vid::{VidDisperser, VidReconstructor},
    vote::VoteCollector,
};

pub struct Timer {
    pub(crate) timer: BoxFuture<'static, ViewNumber>,
    timeout_time: Duration,
}

impl Timer {
    pub fn new(timeout_time: Duration) -> Self {
        Self {
            timer: sleep(timeout_time)
                .map(|_| ViewNumber::genesis())
                .fuse()
                .boxed(),
            timeout_time,
        }
    }

    pub fn reset(&mut self, view_number: ViewNumber) {
        self.timer = sleep(self.timeout_time)
            .map(move |_| view_number)
            .fuse()
            .boxed();
    }
}

#[derive(Builder)]
pub(crate) struct Coordinator<T: NodeType, I: NodeImplementation<T>> {
    external_tx: async_broadcast::Sender<hotshot_types::event::Event<T>>,
    system_context: SystemContextHandle<T, I>,
    consensus: Consensus<T>,
    network: Network<T, I::Network>,
    state_manager: ValidatedStateManager<T>,
    vid_disperser: VidDisperser<T>,
    vid_reconstructor: VidReconstructor<T>,
    vote1_collector: VoteCollector<T, QuorumVote2<T>, QuorumCertificate2<T>>,
    vote2_collector: VoteCollector<T, Vote2<T>, Certificate2<T>>,
    timeout_collector: VoteCollector<T, TimeoutVote2<T>, TimeoutCertificate2<T>>,
    checkpoint_collector: VoteCollector<T, CheckpointVote<T>, CheckpointCertificate<T>>,
    drb_requester: DrbRequester,
    membership_coordinator: EpochMembershipCoordinator<T>,
    #[builder(default)]
    outbox: Outbox<ConsensusOutput<T>>,

    timer: Timer,
}

impl<T: NodeType, I: NodeImplementation<T>> Coordinator<T, I> {
    pub async fn run(mut self) {
        loop {
            select! {
                view_number = &mut self.timer.timer => {
                    self.evaluate(ConsensusInput::Timeout(view_number)).await;
                    self.timer.reset(view_number + 1);
                }
                message = self.network.receive() => match message {
                    Ok(m) => {
                        self.on_message(m).await
                    }
                    Err(err) if is_critical(&err) => {
                        error!(%err, "critical network error => exiting");
                        break
                    }
                    Err(err) => {
                        warn!(%err, "network error")
                    }
                },
                Some(state_event) = self.state_manager.next() => {
                    if let Ok(input) = ConsensusInput::try_from(state_event) {
                        self.evaluate(input).await;
                    }
                }
                Some(tcert) = self.timeout_collector.next() => {
                    self.evaluate(ConsensusInput::TimeoutCertificate(tcert)).await;
                }
                Some(cert1) = self.vote1_collector.next() => {
                    self.evaluate(ConsensusInput::Certificate1(cert1)).await;
                }
                Some(cert2) = self.vote2_collector.next() => {
                    self.evaluate(ConsensusInput::Certificate2(cert2)).await;
                }
                Some(checkpoint_cert) = self.checkpoint_collector.next() => {
                    self.gc(checkpoint_cert.view_number(), checkpoint_cert.epoch().unwrap());
                }
                Some(item) = self.vid_disperser.next() => match item {
                    Ok((view, _, disperse)) => {
                        self.evaluate(ConsensusInput::VidDisperseCreated(view, disperse)).await;
                    }
                    Err(err) => {
                        warn!(?err, "vid disperser error")
                    }
                },
                Some(item) = self.vid_reconstructor.next() => match item {
                    Ok((view, commitment, _)) => {
                        self.evaluate(ConsensusInput::BlockReconstructed(view, commitment)).await;
                    }
                    Err(err) => {
                        warn!(?err, "vid reconstructor error")
                    }
                },
                Some((_epoch, drb_result)) = self.drb_requester.next() => {
                    todo!()
                }
                else => {
                    error!("all coordinator inputs are closed => exiting");
                    break
                }
            }
        }
    }

    fn gc(&mut self, view_number: ViewNumber, epoch: EpochNumber) {
        self.consensus.gc(view_number, epoch);
        self.checkpoint_collector.gc(view_number);
        self.network.gc(view_number, epoch);
        self.state_manager.gc(view_number);
        self.vid_disperser.gc(view_number);
        self.vid_reconstructor.gc(view_number);
        self.vote1_collector.gc(view_number);
        self.vote2_collector.gc(view_number);
        self.timeout_collector.gc(view_number);
        self.checkpoint_collector.gc(view_number);
        self.drb_requester.gc(epoch);
    }

    /// Process an incoming network message.
    async fn on_message(&mut self, msg: Message<T>) {
        match msg.message_type {
            MessageType::Consensus(msg) => match msg {
                ConsensusMessage::Proposal(proposal) => {
                    self.vid_reconstructor.handle_vid_share(VidShareInput {
                        share: proposal.vid_share.clone(),
                        metadata: Some(proposal.proposal.data.block_header.metadata().clone()),
                    });
                    self.evaluate(ConsensusInput::Proposal(proposal)).await;
                },
                ConsensusMessage::Vote1(vote1) => {
                    self.vote1_collector.accumulate_vote(vote1.vote).await;
                    self.vid_reconstructor.handle_vid_share(VidShareInput {
                        share: vote1.vid_share,
                        metadata: None,
                    });
                },
                ConsensusMessage::Vote2(vote2) => {
                    self.vote2_collector.accumulate_vote(vote2).await;
                },
                ConsensusMessage::Certificate1(certificate1, _key) => {
                    self.evaluate(ConsensusInput::Certificate1(certificate1))
                        .await;
                },
                ConsensusMessage::Certificate2(certificate2, _key) => {
                    self.evaluate(ConsensusInput::Certificate2(certificate2))
                        .await;
                },
                ConsensusMessage::TimeoutVote(timeout_vote) => {
                    self.timeout_collector.accumulate_vote(timeout_vote).await;
                },
                ConsensusMessage::Transactions(transactions, view) => {
                    todo!()
                },
                ConsensusMessage::Checkpoint(checkpoint) => {
                    self.checkpoint_collector.accumulate_vote(checkpoint).await;
                },
            },
            MessageType::ViewSync(_) => todo!(),
            MessageType::External(_) => todo!(),
        }
    }

    async fn evaluate(&mut self, input: ConsensusInput<T>) {
        self.consensus.apply(input, &mut self.outbox).await;
        while let Some(output) = self.outbox.pop_front() {
            match output {
                ConsensusOutput::Action(a) => self.handle_action(a).await,
                ConsensusOutput::Event(e) => self.handle_event(e),
            }
        }
    }

    async fn handle_action(&mut self, action: Action<T>) {
        match action {
            Action::SendProposal(proposal, vid_disperse) => {
                for vid_share in vid_disperse.to_shares() {
                    let recipient_key = vid_share.recipient_key.clone();
                    let message = Message {
                        sender: self.system_context.public_key(),
                        message_type: MessageType::Consensus(ConsensusMessage::Proposal(
                            ProposalMessage {
                                proposal: proposal.clone(),
                                vid_share,
                            },
                        )),
                    };
                    let _ = self
                        .network
                        .unicast(recipient_key, message)
                        .await
                        .inspect_err(|e| warn!(%e, "failed to send proposal to recipient"));
                }
            },
            Action::SendVote1(vote1) => {
                let message = Message {
                    sender: self.system_context.public_key(),
                    message_type: MessageType::Consensus(ConsensusMessage::Vote1(vote1)),
                };
                let _ = self
                    .network
                    .broadcast(message)
                    .await
                    .inspect_err(|e| warn!(%e, "failed to send vote1"));
            },
            Action::SendVote2(vote2) => {
                let message = Message {
                    sender: self.system_context.public_key(),
                    message_type: MessageType::Consensus(ConsensusMessage::Vote2(vote2)),
                };
                let _ = self
                    .network
                    .broadcast(message)
                    .await
                    .inspect_err(|e| warn!(%e, "failed to send vote2"));
            },
            Action::SendTimeoutVote(timeout_vote) => {
                let message = Message {
                    sender: self.system_context.public_key(),
                    message_type: MessageType::Consensus(ConsensusMessage::TimeoutVote(
                        timeout_vote,
                    )),
                };
                let _ = self
                    .network
                    .broadcast(message)
                    .await
                    .inspect_err(|e| warn!(%e, "failed to send timeout vote"));
            },
            Action::SendCheckpointVote(checkpoint_vote) => {
                let message = Message {
                    sender: self.system_context.public_key(),
                    message_type: MessageType::Consensus(ConsensusMessage::Checkpoint(
                        checkpoint_vote,
                    )),
                };
                let _ = self
                    .network
                    .broadcast(message)
                    .await
                    .inspect_err(|e| warn!(%e, "failed to send checkpoint vote"));
            },
            Action::RequestState(state_request) => {
                self.state_manager.request_state(state_request);
            },
            Action::RequestBlockAndHeader(req) => {
                // TODO: add a block builder, and use it to build the block,
                // Then on block built, request the header
                todo!()
            },
            Action::RequestVidDisperse(view, epoch, block, metadata) => {
                self.vid_disperser.request_vid_disperse(VidDisperseRequest {
                    view,
                    epoch,
                    block,
                    metadata,
                });
            },
            Action::RequestProposal(_view, _commitment) => {},
            Action::RequestDRB(drb_input) => {
                self.drb_requester.request_drb(drb_input);
            },
        }
    }

    fn handle_event(&mut self, event: Event<T>) {
        match event {
            Event::ViewChanged(view_number, _epoch) => {
                self.timer.reset(view_number);
            },
            Event::LeafDecided(leaves) => {},

            _ => error!("TODO"),
        }
    }
}
