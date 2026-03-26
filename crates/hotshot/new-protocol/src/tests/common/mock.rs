pub mod testing {
    use hotshot_example_types::{
        block_types::TestBlockHeader,
        node_types::{TEST_VERSIONS, TestTypes},
    };
    use hotshot_types::{
        data::{Leaf2, QuorumProposalWrapper, VidDisperse},
        epoch_membership::EpochMembershipCoordinator,
        simple_vote::QuorumVote2,
    };
    use tokio::select;

    use crate::{
        Outbox,
        consensus::Consensus,
        drb::DrbRequester,
        events::*,
        helpers::upgrade_lock,
        message::{Certificate1, Certificate2, ConsensusMessage, Vote2},
        state::StateManager,
        tests::common::utils::{MockBlock, PendingIfNone, mock_builder_fee, state_verified_input},
        vid::{VidDisperser, VidReconstructor},
        vote::VoteCollector,
    };

    /// MockCoordinator is for testing the various different modules the coordinator will
    /// coordinate. It will send back appropriate responses for actions it receives.
    /// It will also store the events it receives for verification.
    ///
    /// When `state_manager` is `Some`, state and header requests are forwarded to
    /// the `StateManager`. Its completions are polled via `next()` and
    /// fed back as `ConsensusInput`.
    pub struct MockCoordinator {
        pub consensus: Consensus<TestTypes>,
        pub input_rx: tokio::sync::mpsc::Receiver<ConsensusOutput<TestTypes>>,
        pub shutdown_rx: tokio::sync::oneshot::Receiver<()>,
        pub state_manager: Option<StateManager<TestTypes>>,
        pub vote1_task:
            Option<VoteCollector<TestTypes, QuorumVote2<TestTypes>, Certificate1<TestTypes>>>,
        pub vote2_task: Option<VoteCollector<TestTypes, Vote2<TestTypes>, Certificate2<TestTypes>>>,
        pub vid_disperse_task: Option<VidDisperser<TestTypes>>,
        pub vid_reconstruction_task: Option<VidReconstructor<TestTypes>>,
        pub drb_request_task: Option<DrbRequester>,
        pub membership_coordinator: EpochMembershipCoordinator<TestTypes>,
        pub outbox: Outbox<ConsensusOutput<TestTypes>>,
        pub received_events: Vec<ConsensusOutput<TestTypes>>,
    }

    impl MockCoordinator {
        pub async fn run(mut self) -> Vec<ConsensusOutput<TestTypes>> {
            loop {
                select! {
                    Some(input) = self.input_rx.recv() => {
                        if let ConsensusOutput::Event(event) = &input
                            && let Ok(consensus_input) = ConsensusInput::try_from(event.clone()) {
                                self.process_input(consensus_input).await;
                        }

                        match input {
                            ConsensusOutput::Event(Event::MessageReceived(ConsensusMessage::Vote1(vote1))) => {
                                if let Some(vote1_task) = &mut self.vote1_task {
                                    vote1_task.accumulate_vote(vote1.vote).await;
                                }
                                if let Some(vid_reconstruction_task) = &mut self.vid_reconstruction_task {
                                    vid_reconstruction_task.handle_vid_share(VidShareInput {
                                        share: vote1.vid_share,
                                        metadata: None,
                                    });
                                }
                            }
                            ConsensusOutput::Event(Event::MessageReceived(ConsensusMessage::Vote2(vote2))) => {
                                if let Some(vote2_task) = &mut self.vote2_task {
                                    vote2_task.accumulate_vote(vote2).await;
                                }
                            }
                            ConsensusOutput::Event(Event::MessageReceived(ConsensusMessage::Proposal(proposal))) => {
                                if let Some(vid_reconstruction_task) = &mut self.vid_reconstruction_task {
                                    vid_reconstruction_task.handle_vid_share(VidShareInput {
                                        share: proposal.vid_share,
                                        metadata: Some(proposal.proposal.data.block_header.metadata),
                                    });
                                }
                            }
                            _ => {},
                        }
                    }
                    Some(event) = PendingIfNone(self.state_manager.as_mut().map(|sm| sm.next())) => {
                        self.received_events.push(ConsensusOutput::Event(event.clone()));
                        if let Ok(input) = ConsensusInput::try_from(event) {
                            self.process_input(input).await;
                        }
                    }
                    Some(cert1) = PendingIfNone(self.vote1_task.as_mut().map(|task| task.next())) => {
                        self.received_events.push(ConsensusOutput::Event(Event::Certificate1Formed(cert1.clone())));
                        self.process_input(ConsensusInput::Certificate1(cert1)).await;
                    }
                    Some(cert2) = PendingIfNone(self.vote2_task.as_mut().map(|task| task.next())) => {
                        self.received_events.push(ConsensusOutput::Event(Event::Certificate2Formed(cert2.clone())));
                        self.process_input(ConsensusInput::Certificate2(cert2)).await;
                    }
                    Some(Ok((view, vid_commitment, vid_disperse))) = PendingIfNone(self.vid_disperse_task.as_mut().map(|task| task.next())) => {
                        self.received_events.push(ConsensusOutput::Event(Event::VidDisperseCreated(vid_commitment, vid_disperse.clone())));
                        self.process_input(ConsensusInput::VidDisperseCreated(view, vid_disperse)).await;
                    }
                    Some(Ok((view, vid_commitment, payload))) = PendingIfNone(self.vid_reconstruction_task.as_mut().map(|task| task.next())) => {
                        self.received_events.push(ConsensusOutput::Event(Event::BlockReconstructed(view, payload, vid_commitment)));
                        self.process_input(ConsensusInput::BlockReconstructed(view, vid_commitment)).await;
                    }
                    Some((_epoch, drb_result)) = PendingIfNone(self.drb_request_task.as_mut().map(|task| task.next())) => {
                        self.received_events.push(ConsensusOutput::Event(Event::DrbCalculated(drb_result)));
                    }
                    _ = &mut self.shutdown_rx => break,
                    else => break,
                }
            }
            self.received_events
        }

        async fn process_input(&mut self, input: ConsensusInput<TestTypes>) {
            self.consensus.apply(input, &mut self.outbox).await;
            self.process_outputs().await
        }

        async fn process_outputs(&mut self) {
            while let Some(output) = self.outbox.pop_front() {
                if let ConsensusOutput::Action(action) = &output {
                    self.handle_action(action).await;
                }
                self.received_events.push(output);
            }
        }

        async fn handle_action(&mut self, action: &Action<TestTypes>) {
            match action {
                Action::SendProposal(..) => {},
                Action::SendVote1(..) => {},
                Action::SendVote2(..) => {},
                Action::RequestState(state_request) => {
                    if let Some(sm) = &mut self.state_manager {
                        sm.request_state(state_request.clone());
                    } else {
                        let input =
                            state_verified_input(&state_request.proposal, state_request.view);
                        self.consensus.apply(input, &mut self.outbox).await;
                    }
                },
                Action::RequestBlockAndHeader(req) => {
                    let mock_block = MockBlock::new();

                    if let Some(sm) = &mut self.state_manager {
                        sm.request_header(HeaderRequest {
                            view: req.view,
                            epoch: req.epoch,
                            parent_proposal: req.parent_proposal.clone(),
                            payload_commitment: mock_block.payload_commitment,
                            builder_commitment: mock_block.builder_commitment,
                            metadata: mock_block.metadata,
                            builder_fee: mock_builder_fee(),
                        });
                    } else {
                        let wrapper = QuorumProposalWrapper::<TestTypes> {
                            proposal: req.parent_proposal.clone(),
                        };
                        let parent_leaf = Leaf2::from_quorum_proposal(&wrapper);
                        let header = TestBlockHeader::new(
                            &parent_leaf,
                            mock_block.payload_commitment,
                            mock_block.builder_commitment,
                            mock_block.metadata,
                            TEST_VERSIONS.test.base,
                        );
                        self.consensus
                            .apply(
                                ConsensusInput::HeaderCreated(req.view, header),
                                &mut self.outbox,
                            )
                            .await;
                    }

                    self.consensus
                        .apply(
                            ConsensusInput::BlockBuilt(
                                req.view,
                                req.epoch,
                                mock_block.block,
                                mock_block.metadata,
                            ),
                            &mut self.outbox,
                        )
                        .await;
                },
                Action::RequestVidDisperse(view, epoch, block, metadata) => {
                    if let Some(vid_disperse_task) = &mut self.vid_disperse_task {
                        vid_disperse_task.request_vid_disperse(VidDisperseRequest {
                            view: *view,
                            epoch: *epoch,
                            block: block.clone(),
                            metadata: *metadata,
                        });
                    } else {
                        let vid_disperse = VidDisperse::calculate_vid_disperse(
                            block,
                            &self.membership_coordinator,
                            *view,
                            Some(*epoch),
                            Some(*epoch),
                            metadata,
                            &upgrade_lock(),
                        )
                        .await
                        .unwrap();

                        let VidDisperse::V2(vid) = vid_disperse.disperse else {
                            panic!("VidDisperse is not a V2");
                        };
                        self.consensus
                            .apply(
                                ConsensusInput::VidDisperseCreated(*view, vid),
                                &mut self.outbox,
                            )
                            .await;
                    }
                },
                Action::RequestProposal(_view, _commitment) => {},
                Action::RequestDRB(drb_input) => {
                    if let Some(drb_request_task) = &mut self.drb_request_task {
                        drb_request_task.request_drb(drb_input.clone());
                    }
                },
            }
        }
    }
}
