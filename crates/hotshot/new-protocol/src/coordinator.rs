use bon::Builder;
use hotshot::{traits::NodeImplementation, types::SystemContextHandle};
use hotshot_types::{
    epoch_membership::EpochMembershipCoordinator,
    simple_certificate::{QuorumCertificate2, TimeoutCertificate2},
    simple_vote::{QuorumVote2, TimeoutVote2},
    traits::{block_contents::BlockHeader, node_implementation::NodeType},
};
use tokio::select;
use tracing::{error, warn};

use crate::{
    Outbox,
    block::BlockBuilder,
    consensus::Consensus,
    drb::DrbRequester,
    events::*,
    io::network::{Network, is_critical},
    message::{BlockMessage, Certificate2, ConsensusMessage, Message, MessageType, Vote2},
    state::StateManager,
    vid::{VidDisperser, VidReconstructor},
    vote::VoteCollector,
};

#[derive(Builder)]
pub(crate) struct Coordinator<T: NodeType, I: NodeImplementation<T>> {
    external_tx: async_broadcast::Sender<hotshot_types::event::Event<T>>,
    system_context: SystemContextHandle<T, I>,
    consensus: Consensus<T>,
    network: Network<T, I::Network>,
    state_manager: StateManager<T>,
    vid_disperser: VidDisperser<T>,
    vid_reconstructor: VidReconstructor<T>,
    vote1_collector: VoteCollector<T, QuorumVote2<T>, QuorumCertificate2<T>>,
    vote2_collector: VoteCollector<T, Vote2<T>, Certificate2<T>>,
    timeout_collector: VoteCollector<T, TimeoutVote2<T>, TimeoutCertificate2<T>>,
    drb_requester: DrbRequester,
    membership_coordinator: EpochMembershipCoordinator<T>,
    #[builder(default)]
    outbox: Outbox<ConsensusOutput<T>>,
    #[builder(default)]
    block_builder: BlockBuilder<T>,
}

impl<T: NodeType, I: NodeImplementation<T>> Coordinator<T, I> {
    pub async fn run(mut self) {
        loop {
            select! {
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
                Some(item) = self.vid_disperser.next() => match item {
                    Ok((view, _, disperse)) => {
                        self.evaluate(ConsensusInput::VidDisperseCreated(view, disperse)).await;
                    }
                    Err(err) => {
                        warn!(?err, "vid disperser error")
                    }
                },
                Some(item) = self.vid_reconstructor.next() => match item {
                    Ok((view, commitment, payload)) => {
                        self.block_builder.on_block_reconstructed(view, payload);
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
                ConsensusMessage::Checkpoint(view, epoch) => {
                    todo!()
                },
            },
            MessageType::Block(msg) => match msg {
                BlockMessage::Transactions(tx_msg) => {
                    self.block_builder.on_transactions(tx_msg);
                },
                BlockMessage::DedupManifest(manifest) => {
                    self.block_builder.on_dedup_manifest(manifest);
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
            Action::SendProposal(..) => {},
            Action::SendVote1(..) => {},
            Action::SendVote2(..) => {},
            Action::RequestState(state_request) => {
                self.state_manager.request_state(state_request);
            },
            Action::RequestBlockAndHeader(req) => {
                let (_txns, _manifest) = self.block_builder.drain(req.view);
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
            Action::SendTransactions(_view, _txns) => {},
            Action::SendDedupManifest(_view, _manifest) => {},
        }
    }

    fn handle_event(&mut self, event: Event<T>) {
        error!("TODO")
    }
}
