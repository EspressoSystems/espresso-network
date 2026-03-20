pub(crate) mod test_utils;

use hotshot::{traits::BlockPayload, types::BLSPubKey};
use hotshot_example_types::{
    block_types::{TestBlockHeader, TestBlockPayload, TestMetadata},
    node_types::{TEST_VERSIONS, TestTypes},
};
use hotshot_types::{
    data::{Leaf2, QuorumProposalWrapper, VidDisperse, vid_commitment},
    traits::{EncodeBytes, signature_key::SignatureKey},
};

use crate::{
    Consensus, Outbox,
    events::{Action, ConsensusInput, ConsensusOutput, Event},
    helpers::upgrade_lock,
    tests::test_utils::mock_membership,
};

// ── Event assertion helpers ──

/// Check if received outputs contain a Vote1 action.
pub(crate) fn has_vote1(outputs: &Outbox<ConsensusOutput<TestTypes>>) -> bool {
    outputs
        .iter()
        .any(|e| matches!(e, ConsensusOutput::Action(Action::SendVote1(_))))
}

/// Check if received outputs contain a Vote2 action.
pub(crate) fn has_vote2(outputs: &Outbox<ConsensusOutput<TestTypes>>) -> bool {
    outputs
        .iter()
        .any(|e| matches!(e, ConsensusOutput::Action(Action::SendVote2(_))))
}

/// Check if received outputs contain a LeafDecided update.
pub(crate) fn has_leaf_decided(outputs: &Outbox<ConsensusOutput<TestTypes>>) -> bool {
    outputs
        .iter()
        .any(|e| matches!(e, ConsensusOutput::Event(Event::LeafDecided(_))))
}

/// Check if received outputs contain a RequestState action.
pub(crate) fn has_request_state(outputs: &Outbox<ConsensusOutput<TestTypes>>) -> bool {
    outputs
        .iter()
        .any(|e| matches!(e, ConsensusOutput::Action(Action::RequestState(_))))
}

/// Count how many Vote1 actions are in the outputs.
pub(crate) fn count_vote1(outputs: &Outbox<ConsensusOutput<TestTypes>>) -> usize {
    outputs
        .iter()
        .filter(|e| matches!(e, ConsensusOutput::Action(Action::SendVote1(_))))
        .count()
}

/// Count how many Vote2 actions are in the outputs.
pub(crate) fn count_vote2(outputs: &Outbox<ConsensusOutput<TestTypes>>) -> usize {
    outputs
        .iter()
        .filter(|e| matches!(e, ConsensusOutput::Action(Action::SendVote2(_))))
        .count()
}

pub(crate) fn has_proposal<'a, I>(outputs: I) -> bool
where
    I: IntoIterator<Item = &'a ConsensusOutput<TestTypes>>,
{
    outputs
        .into_iter()
        .any(|e| matches!(e, ConsensusOutput::Action(Action::SendProposal(..))))
}

pub(crate) fn has_request_block_and_header(outputs: &Outbox<ConsensusOutput<TestTypes>>) -> bool {
    outputs
        .iter()
        .any(|e| matches!(e, ConsensusOutput::Action(Action::RequestBlockAndHeader(_))))
}

/// Find the node index (0..10) for a given public key.
pub(crate) fn node_index_for_key(key: &BLSPubKey) -> u64 {
    for i in 0..10 {
        let (pk, _) = BLSPubKey::generated_from_seed_indexed([0; 32], i);
        if pk == *key {
            return i;
        }
    }
    panic!("Key not found in test keys (indices 0..10)");
}

pub(crate) async fn map_block_requests(
    outbox: &mut Outbox<ConsensusOutput<TestTypes>>,
) -> (
    Vec<ConsensusInput<TestTypes>>,
    Vec<ConsensusOutput<TestTypes>>,
) {
    let membership = mock_membership().await;
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();

    while let Some(output) = outbox.pop_front() {
        match output {
            ConsensusOutput::Action(Action::RequestBlockAndHeader(req)) => {
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
                let wrapper = QuorumProposalWrapper::<TestTypes> {
                    proposal: req.parent_proposal.clone(),
                };
                let parent_leaf = Leaf2::from_quorum_proposal(&wrapper);
                let header = TestBlockHeader::new(
                    &parent_leaf,
                    payload_commitment,
                    builder_commitment,
                    metadata,
                    TEST_VERSIONS.test.base,
                );
                inputs.push(ConsensusInput::HeaderCreated(req.view, header));
                inputs.push(ConsensusInput::BlockBuilt(
                    req.view, req.epoch, block, metadata,
                ));
            },
            ConsensusOutput::Action(Action::RequestVidDisperse {
                view,
                epoch,
                block,
                metadata,
            }) => {
                let vid_disperse = VidDisperse::calculate_vid_disperse(
                    &block,
                    &membership,
                    view,
                    Some(epoch),
                    Some(epoch),
                    &metadata,
                    &upgrade_lock(),
                )
                .await
                .unwrap();
                let VidDisperse::V2(vid) = vid_disperse.disperse else {
                    panic!("Expected V2 VID disperse");
                };
                inputs.push(ConsensusInput::VidDisperseCreated(view, vid));
            },
            other => outputs.push(other),
        }
    }
    (inputs, outputs)
}
