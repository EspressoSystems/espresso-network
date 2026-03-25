use std::sync::Arc;

use hotshot::{
    traits::NodeImplementation,
    types::{SignatureKey, SystemContextHandle},
};
use hotshot_types::{
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    simple_certificate::{QuorumCertificate2, TimeoutCertificate2},
    simple_vote::{QuorumVote2, TimeoutVote2},
    traits::{
        block_contents::BlockHeader,
        node_implementation::NodeType,
        storage::{null_load_drb_progress_fn, null_store_drb_progress_fn},
    },
};

use crate::{
    Outbox,
    consensus::Consensus,
    drb::DrbRequestTask,
    events::*,
    io::network::Network,
    message::{Certificate2, ConsensusMessage, Vote2},
    validated_state::ValidatedStateManager,
    vid::{VidDisperseTask, VidReconstructionTask},
    vote::VoteCollectionTask,
};

// TODO: Use a builder pattern to construct the coordinator
pub(crate) struct Coordinator<T: NodeType, I: NodeImplementation<T>> {
    external_tx: async_broadcast::Sender<hotshot_types::event::Event<T>>,
    system_context: SystemContextHandle<T, I>,
    consensus: Consensus<T>,
    network: Network<T, I::Network>,
    state_manager: ValidatedStateManager<T>,
    vid_disperse_task: VidDisperseTask<T>,
    vid_reconstruction_task: VidReconstructionTask<T>,
    vote1_task: VoteCollectionTask<T, QuorumVote2<T>, QuorumCertificate2<T>>,
    vote2_task: VoteCollectionTask<T, Vote2<T>, Certificate2<T>>,
    timeout_vote_task: VoteCollectionTask<T, TimeoutVote2<T>, TimeoutCertificate2<T>>,
    drb_request_task: DrbRequestTask,
    membership_coordinator: EpochMembershipCoordinator<T>,
    outbox: Outbox<ConsensusOutput<T>>,
}

impl<T: NodeType, I: NodeImplementation<T>> Coordinator<T, I> {
    #[allow(clippy::too_many_arguments)]
    pub async fn new(
        external_tx: async_broadcast::Sender<hotshot_types::event::Event<T>>,
        system_context: SystemContextHandle<T, I>,
        membership_coordinator: EpochMembershipCoordinator<T>,
        public_key: T::SignatureKey,
        private_key: <T::SignatureKey as SignatureKey>::PrivateKey,
        instance_state: Arc<T::InstanceState>,
        upgrade_lock: UpgradeLock<T>,
        network: Network<T, I::Network>,
    ) -> Self {
        Self {
            external_tx,
            system_context,
            consensus: Consensus::new(membership_coordinator.clone(), public_key, private_key),
            network,
            state_manager: ValidatedStateManager::new(instance_state),
            vid_disperse_task: VidDisperseTask::new(membership_coordinator.clone()),
            vid_reconstruction_task: VidReconstructionTask::new(),
            vote1_task: VoteCollectionTask::new(
                membership_coordinator.clone(),
                upgrade_lock.clone(),
            ),
            vote2_task: VoteCollectionTask::new(
                membership_coordinator.clone(),
                upgrade_lock.clone(),
            ),
            timeout_vote_task: VoteCollectionTask::new(
                membership_coordinator.clone(),
                upgrade_lock.clone(),
            ),
            drb_request_task: DrbRequestTask::new(
                null_store_drb_progress_fn(),
                null_load_drb_progress_fn(),
            ),
            membership_coordinator,
            outbox: Outbox::new(),
        }
    }

    pub async fn run(mut self) {
        loop {
            tokio::select! {
                Ok(message) = self.network.recv_message() => {
                    match message {
                        ConsensusMessage::Proposal(proposal) => {
                            self.vid_reconstruction_task.handle_vid_share(VidShareInput {
                                share: proposal.vid_share.clone(),
                                metadata: Some(proposal.proposal.data.block_header.metadata().clone()),
                            });
                            self.process_input(ConsensusInput::Proposal(proposal)).await;

                        }
                        ConsensusMessage::Vote1(vote1) => {
                            self.vote1_task.accumulate_vote(vote1.vote).await;
                            self.vid_reconstruction_task.handle_vid_share(VidShareInput {
                                share: vote1.vid_share,
                                metadata: None,
                            });
                        }
                        ConsensusMessage::Vote2(vote2) => {
                            self.vote2_task.accumulate_vote(vote2).await;
                        }
                        ConsensusMessage::Certificate1(certificate1, _key) => {
                            self.process_input(ConsensusInput::Certificate1(certificate1)).await;
                        }
                        ConsensusMessage::Certificate2(certificate2, _key) => {
                            self.process_input(ConsensusInput::Certificate2(certificate2)).await;
                        }
                        ConsensusMessage::TimeoutVote(timeout_vote) => {
                            self.timeout_vote_task.accumulate_vote(timeout_vote).await;
                        }
                        ConsensusMessage::Transactions(transactions, view) => {
                            todo!()
                        }
                        ConsensusMessage::Checkpoint(view, epoch) => {
                            todo!()
                        }
                    }
                }
                Some(state_event) = self.state_manager.next() => {
                    if let Ok(input) = ConsensusInput::try_from(state_event) {
                        self.process_input(input).await;
                    }
                }
                Some(cert1) = self.vote1_task.next() => {
                    self.process_input(ConsensusInput::Certificate1(cert1)).await;
                }
                Some(cert2) = self.vote2_task.next() => {
                    self.process_input(ConsensusInput::Certificate2(cert2)).await;
                }
                Some(Ok((view, vid_commitment, vid_disperse))) = self.vid_disperse_task.next() => {
                    self.process_input(ConsensusInput::VidDisperseCreated(view, vid_disperse)).await;
                }
                Some(Ok((view, vid_commitment, payload))) = self.vid_reconstruction_task.next() => {
                    self.process_input(ConsensusInput::BlockReconstructed(view, vid_commitment)).await;
                }
                Some((_epoch, drb_result)) = self.drb_request_task.next() => {
                    todo!()
                }
            }
        }
    }

    async fn process_input(&mut self, input: ConsensusInput<T>) {
        self.consensus.apply(input, &mut self.outbox).await;
        self.process_outputs().await
    }

    async fn process_outputs(&mut self) {
        while let Some(output) = self.outbox.pop_front() {
            if let ConsensusOutput::Action(action) = &output {
                self.handle_action(action).await;
            }
        }
    }

    async fn handle_action(&mut self, action: &Action<T>) {
        match action {
            Action::SendProposal(..) => {},
            Action::SendVote1(..) => {},
            Action::SendVote2(..) => {},
            Action::RequestState(state_request) => {
                self.state_manager.request_state(state_request.clone());
            },
            Action::RequestBlockAndHeader(req) => {
                // TODO: add a block builder, and use it to build the block,
                // Then on block built, request the header
                todo!()
            },
            Action::RequestVidDisperse(view, epoch, block, metadata) => {
                self.vid_disperse_task
                    .request_vid_disperse(VidDisperseRequest {
                        view: *view,
                        epoch: *epoch,
                        block: block.clone(),
                        metadata: metadata.clone(),
                    });
            },
            Action::RequestProposal(_view, _commitment) => {},
            Action::RequestDRB(drb_input) => {
                self.drb_request_task.request_drb(drb_input.clone());
            },
            Action::Shutdown => {
                unreachable!()
            },
        }
    }
}
