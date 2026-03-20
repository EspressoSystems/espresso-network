pub mod testing {
    use std::sync::Arc;

    use hotshot::traits::{BlockPayload, ValidatedState};
    use hotshot_example_types::{
        block_types::{TestBlockHeader, TestBlockPayload, TestMetadata},
        node_types::{TEST_VERSIONS, TestTypes},
        state_types::TestValidatedState,
    };
    use hotshot_types::{
        data::{
            Leaf2, QuorumProposal2, QuorumProposalWrapper, VidCommitment, VidDisperse,
            vid_commitment,
        },
        epoch_membership::EpochMembershipCoordinator,
        traits::{EncodeBytes, block_contents::BuilderFee, signature_key::BuilderSignatureKey},
        utils::BuilderCommitment,
    };

    use crate::{
        events::*,
        helpers::{proposal_commitment, upgrade_lock},
    };

    /// A mock block with its derived commitments and metadata.
    struct MockBlock {
        block: TestBlockPayload,
        metadata: TestMetadata,
        payload_commitment: VidCommitment,
        builder_commitment: BuilderCommitment,
    }

    impl MockBlock {
        fn new() -> Self {
            let block = TestBlockPayload::genesis();
            let metadata = TestMetadata {
                num_transactions: 0,
            };
            let payload_commitment = vid_commitment(
                &block.encode(),
                &metadata.encode(),
                10,
                TEST_VERSIONS.test.base,
            );
            let builder_commitment =
                <TestBlockPayload as BlockPayload<TestTypes>>::builder_commitment(
                    &block, &metadata,
                );
            Self {
                block,
                metadata,
                payload_commitment,
                builder_commitment,
            }
        }
    }

    fn mock_builder_fee() -> BuilderFee<TestTypes> {
        let (builder_key, builder_private_key) =
            <hotshot_types::signature_key::BuilderKey as BuilderSignatureKey>::generated_from_seed_indexed([0; 32], 0);
        let builder_signature =
            <hotshot_types::signature_key::BuilderKey as BuilderSignatureKey>::sign_builder_message(
                &builder_private_key,
                &[0u8],
            )
            .unwrap();
        BuilderFee {
            fee_amount: 0,
            fee_account: builder_key,
            fee_signature: builder_signature,
        }
    }

    fn state_verified_event(
        proposal: &QuorumProposal2<TestTypes>,
        view: hotshot_types::data::ViewNumber,
    ) -> ConsensusEvent<TestTypes> {
        let commitment = proposal_commitment(proposal);
        let state =
            <TestValidatedState as ValidatedState<TestTypes>>::from_header(&proposal.block_header);
        ConsensusEvent::StateVerified(StateResponse {
            view,
            commitment,
            state: Arc::new(state),
        })
    }

    /// MockCoordinator is for testing the various different modules the coordinator will
    /// coordinate.  It will send back appropriate responses for actions it receives.
    /// It will also store the events it receives for verification.
    ///
    /// When `state_tx` is `Some`, state and header requests are forwarded to a
    /// `ValidatedStateManager` instead of being handled inline. Responses from
    /// the state manager come back through the shared `event_rx` channel as
    /// `Update` variants and are forwarded to consensus.
    pub struct MockCoordinator {
        pub event_rx: tokio::sync::mpsc::Receiver<Event<TestTypes>>,
        pub consensus_tx: tokio::sync::mpsc::Sender<ConsensusEvent<TestTypes>>,
        pub state_tx: Option<tokio::sync::mpsc::Sender<StateEvent<TestTypes>>>,
        pub membership_coordinator: EpochMembershipCoordinator<TestTypes>,
        pub received_events: Vec<Event<TestTypes>>,
    }
    impl MockCoordinator {
        pub async fn run(mut self) -> Vec<Event<TestTypes>> {
            while let Some(event) = self.event_rx.recv().await {
                if matches!(event, Event::Action(Action::Shutdown)) {
                    // Signal consensus to shut down before stopping
                    let _ = self.consensus_tx.send(ConsensusEvent::Shutdown).await;
                    break;
                }
                self.handle_event(event).await;
            }
            self.received_events
        }
        async fn handle_event(&mut self, event: Event<TestTypes>) {
            match event {
                Event::Action(ref action) => self.handle_action(action).await,
                Event::Update(ref update) => {
                    if let Ok(consensus_event) = ConsensusEvent::try_from(update.clone()) {
                        self.consensus_tx.send(consensus_event).await.unwrap();
                    }
                },
            }
            self.received_events.push(event);
        }

        async fn handle_action(&self, action: &Action<TestTypes>) {
            match action {
                Action::SendMessage(_message) => {},
                Action::RequestState(state_request) => {
                    if let Some(state_tx) = &self.state_tx {
                        state_tx
                            .send(StateEvent::RequestState(state_request.clone()))
                            .await
                            .unwrap();
                    } else {
                        self.consensus_tx
                            .send(state_verified_event(
                                &state_request.proposal,
                                state_request.view,
                            ))
                            .await
                            .unwrap();
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
                                metadata: mock_block.metadata.clone(),
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
                            mock_block.metadata.clone(),
                            TEST_VERSIONS.test.base,
                        );
                        self.consensus_tx
                            .send(ConsensusEvent::HeaderCreated(req.view, header))
                            .await
                            .unwrap();
                    }

                    self.consensus_tx
                        .send(ConsensusEvent::BlockBuilt(
                            req.view,
                            req.epoch,
                            mock_block.block,
                            mock_block.metadata,
                        ))
                        .await
                        .unwrap();
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
                    self.consensus_tx
                        .send(ConsensusEvent::VidDisperseCreated(*view, vid))
                        .await
                        .unwrap();
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
