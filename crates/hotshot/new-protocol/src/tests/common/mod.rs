pub(crate) mod test_utils;

use hotshot::types::BLSPubKey;
use hotshot_example_types::node_types::TestTypes;
use hotshot_types::traits::signature_key::SignatureKey;

use crate::{
    Outbox,
    events::{Action, ConsensusOutput, Event},
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

pub(crate) fn has_proposal(outputs: &Outbox<ConsensusOutput<TestTypes>>) -> bool {
    outputs
        .iter()
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
