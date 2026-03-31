use hotshot::types::BLSPubKey;
use hotshot_example_types::node_types::TestTypes;
use hotshot_types::traits::signature_key::SignatureKey;

use crate::consensus::{ConsensusInput, ConsensusOutput};

pub(crate) fn has_vote1<'a, I>(outputs: I) -> bool
where
    I: IntoIterator<Item = &'a ConsensusOutput<TestTypes>>,
{
    outputs
        .into_iter()
        .any(|e| matches!(e, ConsensusOutput::SendVote1(_)))
}

pub(crate) fn has_vote2<'a, I>(outputs: I) -> bool
where
    I: IntoIterator<Item = &'a ConsensusOutput<TestTypes>>,
{
    outputs
        .into_iter()
        .any(|e| matches!(e, ConsensusOutput::SendVote2(_)))
}

pub(crate) fn has_cert1<'a, I>(inputs: I) -> bool
where
    I: IntoIterator<Item = &'a ConsensusInput<TestTypes>>,
{
    inputs
        .into_iter()
        .any(|e| matches!(e, ConsensusInput::Certificate1(_)))
}

pub(crate) fn has_cert2<'a, I>(inputs: I) -> bool
where
    I: IntoIterator<Item = &'a ConsensusInput<TestTypes>>,
{
    inputs
        .into_iter()
        .any(|e| matches!(e, ConsensusInput::Certificate2(_)))
}

pub(crate) fn has_leaf_decided<'a, I>(outputs: I) -> bool
where
    I: IntoIterator<Item = &'a ConsensusOutput<TestTypes>>,
{
    outputs
        .into_iter()
        .any(|e| matches!(e, ConsensusOutput::LeafDecided(_)))
}

pub(crate) fn count_leaf_decided<'a, I>(outputs: I) -> usize
where
    I: IntoIterator<Item = &'a ConsensusOutput<TestTypes>>,
{
    outputs
        .into_iter()
        .filter(|e| matches!(e, ConsensusOutput::LeafDecided(_)))
        .count()
}

pub(crate) fn has_request_state<'a, I>(outputs: I) -> bool
where
    I: IntoIterator<Item = &'a ConsensusOutput<TestTypes>>,
{
    outputs
        .into_iter()
        .any(|e| matches!(e, ConsensusOutput::RequestState(_)))
}

pub(crate) fn has_proposal<'a, I>(outputs: I) -> bool
where
    I: IntoIterator<Item = &'a ConsensusOutput<TestTypes>>,
{
    outputs
        .into_iter()
        .any(|e| matches!(e, ConsensusOutput::SendProposal(..)))
}

pub(crate) fn has_request_block_and_header<'a, I>(outputs: I) -> bool
where
    I: IntoIterator<Item = &'a ConsensusOutput<TestTypes>>,
{
    outputs
        .into_iter()
        .any(|e| matches!(e, ConsensusOutput::RequestBlockAndHeader(_)))
}

pub(crate) fn has_request_vid_disperse<'a, I>(outputs: I) -> bool
where
    I: IntoIterator<Item = &'a ConsensusOutput<TestTypes>>,
{
    outputs
        .into_iter()
        .any(|e| matches!(e, ConsensusOutput::RequestVidDisperse { .. }))
}

pub(crate) fn has_vid_disperse<'a, I>(inputs: I) -> bool
where
    I: IntoIterator<Item = &'a ConsensusInput<TestTypes>>,
{
    inputs
        .into_iter()
        .any(|e| matches!(e, ConsensusInput::VidDisperseCreated(..)))
}

pub(crate) fn has_block_reconstructed<'a, I>(inputs: I) -> bool
where
    I: IntoIterator<Item = &'a ConsensusInput<TestTypes>>,
{
    inputs
        .into_iter()
        .any(|e| matches!(e, ConsensusInput::BlockReconstructed(..)))
}

pub(crate) fn has_state_validated<'a, I>(inputs: I) -> bool
where
    I: IntoIterator<Item = &'a ConsensusInput<TestTypes>>,
{
    inputs
        .into_iter()
        .any(|e| matches!(e, ConsensusInput::StateValidated(..)))
}

pub(crate) fn count_vote1<'a, I>(outputs: I) -> usize
where
    I: IntoIterator<Item = &'a ConsensusOutput<TestTypes>>,
{
    outputs
        .into_iter()
        .filter(|e| matches!(e, ConsensusOutput::SendVote1(_)))
        .count()
}

pub(crate) fn count_vote2<'a, I>(outputs: I) -> usize
where
    I: IntoIterator<Item = &'a ConsensusOutput<TestTypes>>,
{
    outputs
        .into_iter()
        .filter(|e| matches!(e, ConsensusOutput::SendVote2(_)))
        .count()
}

pub(crate) fn count_state_requests<'a, I>(outputs: I) -> usize
where
    I: IntoIterator<Item = &'a ConsensusOutput<TestTypes>>,
{
    outputs
        .into_iter()
        .filter(|e| matches!(e, ConsensusOutput::RequestState(_)))
        .count()
}

pub(crate) fn has_timeout<'a, I>(inputs: I) -> bool
where
    I: IntoIterator<Item = &'a ConsensusInput<TestTypes>>,
{
    inputs
        .into_iter()
        .any(|e| matches!(e, ConsensusInput::Timeout(_)))
}

pub(crate) fn has_timeout_cert<'a, I>(inputs: I) -> bool
where
    I: IntoIterator<Item = &'a ConsensusInput<TestTypes>>,
{
    inputs
        .into_iter()
        .any(|e| matches!(e, ConsensusInput::TimeoutCertificate(_)))
}

pub(crate) fn has_view_changed<'a, I>(outputs: I) -> bool
where
    I: IntoIterator<Item = &'a ConsensusOutput<TestTypes>>,
{
    outputs
        .into_iter()
        .any(|e| matches!(e, ConsensusOutput::ViewChanged(..)))
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
