use hotshot::types::BLSPubKey;
use hotshot_example_types::node_types::TestTypes;
use hotshot_types::traits::signature_key::SignatureKey;

use crate::consensus::{ConsensusInput, ConsensusOutput};

pub(crate) fn is_vote1(output: &ConsensusOutput<TestTypes>) -> bool {
    matches!(output, ConsensusOutput::SendVote1(_))
}

pub(crate) fn is_vote2(output: &ConsensusOutput<TestTypes>) -> bool {
    matches!(output, ConsensusOutput::SendVote2(_))
}

pub(crate) fn is_leaf_decided(output: &ConsensusOutput<TestTypes>) -> bool {
    matches!(output, ConsensusOutput::LeafDecided(_))
}

pub(crate) fn is_request_state(output: &ConsensusOutput<TestTypes>) -> bool {
    matches!(output, ConsensusOutput::RequestState(_))
}

pub(crate) fn is_proposal(output: &ConsensusOutput<TestTypes>) -> bool {
    matches!(output, ConsensusOutput::SendProposal(..))
}

pub(crate) fn is_request_block_and_header(output: &ConsensusOutput<TestTypes>) -> bool {
    matches!(output, ConsensusOutput::RequestBlockAndHeader(_))
}

pub(crate) fn is_request_vid_disperse(output: &ConsensusOutput<TestTypes>) -> bool {
    matches!(output, ConsensusOutput::RequestVidDisperse { .. })
}

pub(crate) fn is_view_changed(output: &ConsensusOutput<TestTypes>) -> bool {
    matches!(output, ConsensusOutput::ViewChanged(..))
}

pub(crate) fn is_send_epoch_change(output: &ConsensusOutput<TestTypes>) -> bool {
    matches!(output, ConsensusOutput::SendEpochChange(..))
}

pub(crate) fn is_cert1(input: &ConsensusInput<TestTypes>) -> bool {
    matches!(input, ConsensusInput::Certificate1(_))
}

pub(crate) fn is_cert2(input: &ConsensusInput<TestTypes>) -> bool {
    matches!(input, ConsensusInput::Certificate2(_))
}

pub(crate) fn is_vid_disperse(input: &ConsensusInput<TestTypes>) -> bool {
    matches!(input, ConsensusInput::VidDisperseCreated(..))
}

pub(crate) fn is_block_reconstructed(input: &ConsensusInput<TestTypes>) -> bool {
    matches!(input, ConsensusInput::BlockReconstructed(..))
}

pub(crate) fn is_state_validated(input: &ConsensusInput<TestTypes>) -> bool {
    matches!(input, ConsensusInput::StateValidated(..))
}

pub(crate) fn is_timeout(input: &ConsensusInput<TestTypes>) -> bool {
    matches!(input, ConsensusInput::Timeout(..))
}

pub(crate) fn is_timeout_cert(input: &ConsensusInput<TestTypes>) -> bool {
    matches!(input, ConsensusInput::TimeoutCertificate(_))
}

pub(crate) fn is_drb_result(input: &ConsensusInput<TestTypes>) -> bool {
    matches!(input, ConsensusInput::DrbResult(..))
}

pub(crate) fn is_block_built(input: &ConsensusInput<TestTypes>) -> bool {
    matches!(input, ConsensusInput::BlockBuilt { .. })
}

pub(crate) fn is_header_created(input: &ConsensusInput<TestTypes>) -> bool {
    matches!(input, ConsensusInput::HeaderCreated(..))
}

pub(crate) fn any<'a, I, P, A>(items: I, pred: P) -> bool
where
    I: IntoIterator<Item = &'a A>,
    P: Fn(&A) -> bool,
    A: 'a,
{
    items.into_iter().any(pred)
}

pub(crate) fn count_matching<'a, I, P, A>(items: I, pred: P) -> usize
where
    I: IntoIterator<Item = &'a A>,
    P: Fn(&A) -> bool,
    A: 'a,
{
    items.into_iter().filter(|it| pred(it)).count()
}

pub(crate) fn has_epoch_change<'a, I>(outputs: I) -> bool
where
    I: IntoIterator<Item = &'a ConsensusOutput<TestTypes>>,
{
    outputs
        .into_iter()
        .any(|e| matches!(e, ConsensusOutput::SendEpochChange(_)))
}

pub(crate) fn has_request_drb_for_epoch<'a, I>(
    outputs: I,
    epoch: hotshot_types::data::EpochNumber,
) -> bool
where
    I: IntoIterator<Item = &'a ConsensusOutput<TestTypes>>,
{
    outputs
        .into_iter()
        .any(|e| matches!(e, ConsensusOutput::RequestDrbResult(e) if *e == epoch))
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
