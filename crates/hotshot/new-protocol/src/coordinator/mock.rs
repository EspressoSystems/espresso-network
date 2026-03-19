pub mod testing {
    use std::sync::Arc;

    use hotshot::traits::{BlockPayload, ValidatedState};
    use hotshot_example_types::{
        block_types::{TestBlockHeader, TestBlockPayload, TestMetadata},
        node_types::{TEST_VERSIONS, TestTypes},
        state_types::TestValidatedState,
    };
    use hotshot_types::{
        data::{Leaf2, QuorumProposalWrapper, VidDisperse, vid_commitment},
        epoch_membership::EpochMembershipCoordinator,
        traits::{EncodeBytes, block_contents::BuilderFee, signature_key::BuilderSignatureKey},
    };

    use crate::{
        events::*,
        helpers::{proposal_commitment, upgrade_lock},
    };

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
                    break;
                }
                self.handle_event(event).await;
            }
            self.received_events
        }
        async fn handle_event(&mut self, event: Event<TestTypes>) {
            match &event {
                Event::Action(action) => self.handle_action(action).await,
                Event::Update(update) => self.handle_update(update).await,
            }
            self.received_events.push(event);
        }

        async fn handle_action(&self, action: &Action<TestTypes>) {
            match action {
                Action::SendMessage(_message) => {},
                Action::RequestState(state_request) => {
                    if let Some(state_tx) = &self.state_tx {
                        // Forward to the real ValidatedStateManager
                        state_tx
                            .send(StateEvent::RequestState(state_request.clone()))
                            .await
                            .unwrap();
                    } else {
                        // Inline mock: immediately respond with a fake state
                        let commitment = proposal_commitment(&state_request.proposal);
                        let state = <TestValidatedState as ValidatedState<TestTypes>>::from_header(
                            &state_request.proposal.block_header,
                        );
                        self.consensus_tx
                            .send(ConsensusEvent::StateVerified(StateResponse {
                                view: state_request.view,
                                commitment,
                                state: Arc::new(state),
                            }))
                            .await
                            .unwrap();
                    }
                },
                Action::RequestBlockAndHeader(block_and_header_request) => {
                    let block = TestBlockPayload::genesis();
                    let metadata = TestMetadata {
                        num_transactions: 0,
                    };

                    if let Some(state_tx) = &self.state_tx {
                        // Forward header creation to the ValidatedStateManager
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
                        let (builder_key, builder_private_key) =
                            <hotshot_types::signature_key::BuilderKey as BuilderSignatureKey>::generated_from_seed_indexed([0; 32], 0);
                        let builder_signature =
                            <hotshot_types::signature_key::BuilderKey as BuilderSignatureKey>::sign_builder_message(
                                &builder_private_key,
                                &[0u8],
                            )
                            .unwrap();
                        state_tx
                            .send(StateEvent::RequestHeader(HeaderRequest {
                                view: block_and_header_request.view,
                                epoch: block_and_header_request.epoch,
                                parent_proposal: block_and_header_request.parent_proposal.clone(),
                                payload_commitment,
                                builder_commitment,
                                metadata: metadata.clone(),
                                builder_fee: BuilderFee {
                                    fee_amount: 0,
                                    fee_account: builder_key,
                                    fee_signature: builder_signature,
                                },
                            }))
                            .await
                            .unwrap();
                    } else {
                        // Inline mock: create header directly
                        let wrapper = QuorumProposalWrapper::<TestTypes> {
                            proposal: block_and_header_request.parent_proposal.clone(),
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
                        let parent_leaf = Leaf2::from_quorum_proposal(&wrapper);
                        let header = TestBlockHeader::new(
                            &parent_leaf,
                            payload_commitment,
                            builder_commitment,
                            metadata.clone(),
                            TEST_VERSIONS.test.base,
                        );
                        self.consensus_tx
                            .send(ConsensusEvent::HeaderCreated(
                                block_and_header_request.view,
                                header,
                            ))
                            .await
                            .unwrap();
                    }

                    // Always send the block to consensus
                    self.consensus_tx
                        .send(ConsensusEvent::BlockBuilt(
                            block_and_header_request.view,
                            block_and_header_request.epoch,
                            block,
                            metadata,
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

        async fn handle_update(&self, update: &Update<TestTypes>) {
            match update {
                Update::StateVerified(state_request) => {
                    // State manager verified a state — forward to consensus
                    let commitment = proposal_commitment(&state_request.proposal);
                    let state = <TestValidatedState as ValidatedState<TestTypes>>::from_header(
                        &state_request.proposal.block_header,
                    );
                    self.consensus_tx
                        .send(ConsensusEvent::StateVerified(StateResponse {
                            view: state_request.view,
                            commitment,
                            state: Arc::new(state),
                        }))
                        .await
                        .unwrap();
                },
                Update::HeaderCreated(view, header) => {
                    // State manager created a header — forward to consensus
                    self.consensus_tx
                        .send(ConsensusEvent::HeaderCreated(*view, header.clone()))
                        .await
                        .unwrap();
                },
                _ => {},
            }
        }
    }
}
