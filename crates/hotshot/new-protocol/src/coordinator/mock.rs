pub mod testing {
    use hotshot::traits::BlockPayload;
    use hotshot_example_types::{
        block_types::{TestBlockHeader, TestBlockPayload, TestMetadata},
        node_types::{TEST_VERSIONS, TestTypes},
        state_types::TestInstanceState,
    };
    use hotshot_types::{
        data::{Leaf2, QuorumProposalWrapper, VidDisperse, vid_commitment},
        epoch_membership::EpochMembershipCoordinator,
        traits::EncodeBytes,
    };

    use crate::{
        events::*,
        helpers::{proposal_commitment, upgrade_lock},
    };

    pub struct MockCoordinator {
        pub event_rx: tokio::sync::mpsc::Receiver<Event<TestTypes>>,
        pub consensus_tx: tokio::sync::mpsc::Sender<ConsensusEvent<TestTypes>>,
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
        // TODO: For now I'm only handling consensus related actions
        async fn handle_action(&self, action: &Action<TestTypes>) {
            match action {
                Action::SendMessage(message) => {},
                Action::RequestState(state_request) => {
                    let commitment = proposal_commitment(&state_request.proposal);
                    self.consensus_tx
                        .send(ConsensusEvent::StateVerified(StateResponse {
                            view: state_request.view,
                            commitment,
                        }))
                        .await
                        .unwrap();
                },
                Action::RequestBlockAndHeader(block_and_header_request) => {
                    let block = TestBlockPayload::genesis();
                    let state = TestInstanceState::default();
                    let wrapper = QuorumProposalWrapper::<TestTypes> {
                        proposal: block_and_header_request.parent_proposal.clone(),
                    };
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
                    let parent_leaf = Leaf2::from_quorum_proposal(&wrapper);
                    let header = TestBlockHeader::new(
                        &parent_leaf,
                        payload_commitment,
                        builder_commitment,
                        metadata,
                        TEST_VERSIONS.test.base,
                    );
                    self.consensus_tx
                        .send(ConsensusEvent::HeaderCreated(
                            block_and_header_request.view,
                            header,
                        ))
                        .await
                        .unwrap();
                    self.consensus_tx
                        .send(ConsensusEvent::BlockBuilt(
                            block_and_header_request.view,
                            block,
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
                Action::RequestProposal(view, commitment) => {},
                Action::RequestDRB(drb_input) => {},
                Action::Shutdown => {
                    unreachable!()
                },
            }
        }
        async fn handle_update(&mut self, update: &Update<TestTypes>) {
            // TODO: Implement
        }
    }
}
