use hotshot::types::BLSPubKey;
use hotshot_example_types::node_types::TestTypes;
use hotshot_types::traits::signature_key::SignatureKey;

use crate::events::{Action, ConsensusOutput, Event};

pub(crate) fn has_vote1(events: &[ConsensusOutput<TestTypes>]) -> bool {
    events
        .iter()
        .any(|e| matches!(e, ConsensusOutput::Action(Action::SendVote1(_))))
}

pub(crate) fn has_cert1(events: &[ConsensusOutput<TestTypes>]) -> bool {
    events
        .iter()
        .any(|e| matches!(e, ConsensusOutput::Event(Event::Certificate1Formed(_))))
}
pub(crate) fn has_vote2(events: &[ConsensusOutput<TestTypes>]) -> bool {
    events
        .iter()
        .any(|e| matches!(e, ConsensusOutput::Action(Action::SendVote2(_))))
}

#[allow(dead_code)]
pub(crate) fn has_cert2(events: &[ConsensusOutput<TestTypes>]) -> bool {
    events
        .iter()
        .any(|e| matches!(e, ConsensusOutput::Event(Event::Certificate2Formed(_))))
}

pub(crate) fn has_leaf_decided(events: &[ConsensusOutput<TestTypes>]) -> bool {
    events
        .iter()
        .any(|e| matches!(e, ConsensusOutput::Event(Event::LeafDecided(_))))
}

pub(crate) fn has_request_state(events: &[ConsensusOutput<TestTypes>]) -> bool {
    events
        .iter()
        .any(|e| matches!(e, ConsensusOutput::Action(Action::RequestState(_))))
}

pub(crate) fn has_proposal(events: &[ConsensusOutput<TestTypes>]) -> bool {
    events
        .iter()
        .any(|e| matches!(e, ConsensusOutput::Action(Action::SendProposal(..))))
}

pub(crate) fn has_request_block_and_header(events: &[ConsensusOutput<TestTypes>]) -> bool {
    events
        .iter()
        .any(|e| matches!(e, ConsensusOutput::Action(Action::RequestBlockAndHeader(_))))
}

pub(crate) fn has_request_vid_disperse(events: &[ConsensusOutput<TestTypes>]) -> bool {
    events
        .iter()
        .any(|e| matches!(e, ConsensusOutput::Action(Action::RequestVidDisperse(..))))
}

pub(crate) fn has_vid_disperse(events: &[ConsensusOutput<TestTypes>]) -> bool {
    events
        .iter()
        .any(|e| matches!(e, ConsensusOutput::Event(Event::VidDisperseCreated(..))))
}

pub(crate) fn has_block_reconstructed(events: &[ConsensusOutput<TestTypes>]) -> bool {
    events
        .iter()
        .any(|e| matches!(e, ConsensusOutput::Event(Event::BlockReconstructed(..))))
}

pub(crate) fn count_vote1(events: &[ConsensusOutput<TestTypes>]) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, ConsensusOutput::Action(Action::SendVote1(_))))
        .count()
}

pub(crate) fn count_vote2(events: &[ConsensusOutput<TestTypes>]) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, ConsensusOutput::Action(Action::SendVote2(_))))
        .count()
}

pub(crate) fn has_timeout_cert(events: &[ConsensusOutput<TestTypes>]) -> bool {
    events.iter().any(|e| {
        matches!(
            e,
            ConsensusOutput::Event(Event::TimeoutCertificateReceived(_))
        )
    })
}

pub(crate) fn has_view_changed(events: &[ConsensusOutput<TestTypes>]) -> bool {
    events
        .iter()
        .any(|e| matches!(e, ConsensusOutput::Event(Event::ViewChanged(..))))
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
