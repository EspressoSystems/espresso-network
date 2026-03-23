pub mod testing {
    use hotshot_example_types::{
        block_types::TestBlockHeader,
        node_types::{TEST_VERSIONS, TestTypes},
    };
    use hotshot_types::{
        data::{Leaf2, QuorumProposalWrapper, VidDisperse},
        epoch_membership::EpochMembershipCoordinator,
    };

    use crate::{
        Outbox,
        consensus::Consensus,
        events::*,
        helpers::upgrade_lock,
        tests::common::utils::{MockBlock, mock_builder_fee, state_verified_input},
    };

    /// MockCoordinator is for testing the various different modules the coordinator will
    /// coordinate.  It will send back appropriate responses for actions it receives.
    /// It will also store the events it receives for verification.
    ///
    /// When `state_tx` is `Some`, state and header requests are forwarded to a
    /// `ValidatedStateManager` instead of being handled inline. Responses from
    /// the state manager come back through the bridge task as `ConsensusInput`
    /// variants and are forwarded to consensus.
    pub struct MockCoordinator {
        pub consensus: Consensus<TestTypes>,
        pub input_rx: tokio::sync::mpsc::Receiver<ConsensusInput<TestTypes>>,
        pub shutdown_rx: tokio::sync::oneshot::Receiver<()>,
        pub state_tx: Option<tokio::sync::mpsc::Sender<StateEvent<TestTypes>>>,
        pub membership_coordinator: EpochMembershipCoordinator<TestTypes>,
        pub outbox: Outbox<ConsensusOutput<TestTypes>>,
        pub received_events: Vec<ConsensusOutput<TestTypes>>,
    }
    impl MockCoordinator {
        pub async fn run(mut self) -> Vec<ConsensusOutput<TestTypes>> {
            loop {
                tokio::select! {
                    Some(input) = self.input_rx.recv() => {
                        self.process_input(input).await;
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
                    if let Some(state_tx) = &self.state_tx {
                        state_tx
                            .send(StateEvent::RequestState(state_request.clone()))
                            .await
                            .unwrap();
                    } else {
                        let input =
                            state_verified_input(&state_request.proposal, state_request.view);
                        self.consensus.apply(input, &mut self.outbox).await;
                    }
                },
                Action::RequestBlockAndHeader(req) => {
                    let mock_block = MockBlock::new();

                    if let Some(state_tx) = &self.state_tx {
                        state_tx
                            .send(StateEvent::RequestHeader(HeaderRequest {
                                view: req.view,
                                epoch: req.epoch,
                                parent_proposal: req.parent_proposal.clone(),
                                payload_commitment: mock_block.payload_commitment,
                                builder_commitment: mock_block.builder_commitment,
                                metadata: mock_block.metadata,
                                builder_fee: mock_builder_fee(),
                            }))
                            .await
                            .unwrap();
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
                },
                Action::RequestProposal(_view, _commitment) => {},
                Action::RequestDRB(_drb_input) => {},
                Action::Shutdown => {
                    unreachable!()
                },
            }
        }
    }
}
