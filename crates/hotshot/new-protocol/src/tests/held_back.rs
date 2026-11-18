//! What consensus reports as held back agrees with what it releases.
//!
//! The trace recorder writes, for each action consensus holds back, the writes
//! storage has yet to confirm (`Consensus::held_back`). That list is computed
//! apart from the functions that release the action, so it can fall behind
//! them. A checker would then read an action still waiting as never released,
//! or the reverse. These tests confirm the listed writes one at a time: the
//! action must go out on the last, and not before.

use hotshot_example_types::node_types::TestTypes;
use hotshot_types::{
    data::ViewNumber, signature_key::BLSPubKey, traits::signature_key::SignatureKey,
};
use versions::TIMEOUT_EPOCH_VERSION;

use super::common::{
    assertions::{count_matching, is_proposal, is_vote1, is_vote2, node_index_for_key},
    utils::{ConsensusHarness, TestData, state_verified_input},
};
use crate::{
    consensus::{Consensus, ConsensusInput, ConsensusOutput, recorded::HeldKind},
    helpers::pin_test_version,
    outbox::Outbox,
    storage::ActionKind,
};

/// Confirm, one at a time, the writes consensus says its held-back action of
/// `kind` waits for. The action, which `sent` recognises, goes out on the last
/// of them and on none before, and is then no longer held back.
fn confirm_writes(
    consensus: &mut Consensus<TestTypes>,
    outbox: &mut Outbox<ConsensusOutput<TestTypes>>,
    kind: HeldKind,
    sent: fn(&ConsensusOutput<TestTypes>) -> bool,
) {
    let waits = consensus
        .held_back()
        .into_iter()
        .find(|held| held.kind == kind)
        .unwrap_or_else(|| panic!("no {kind:?} is held back"))
        .waits;
    assert!(!waits.is_empty(), "the {kind:?} is held back for no write");
    let before = count_matching(&*outbox, sent);
    for write in waits {
        assert_eq!(
            count_matching(&*outbox, sent),
            before,
            "the {kind:?} went out before {write:?} was confirmed"
        );
        assert!(
            consensus
                .held_back()
                .iter()
                .any(|held| held.kind == kind && held.waits.contains(&write)),
            "the {kind:?} no longer waits for {write:?}, which was not confirmed"
        );
        consensus.apply(ConsensusInput::Stored(write), outbox);
    }
    assert_eq!(
        count_matching(&*outbox, sent),
        before + 1,
        "the {kind:?} did not go out once every write it waited for was confirmed"
    );
    assert!(
        consensus.held_back().iter().all(|held| held.kind != kind),
        "the {kind:?} went out and is still held back"
    );
}

#[tokio::test]
async fn votes_go_out_on_the_last_write_they_wait_for() {
    pin_test_version(TIMEOUT_EPOCH_VERSION);
    let mut harness = ConsensusHarness::new(0).await;
    let test_data = TestData::new(3).await;
    let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;
    let first = &test_data.views[0];
    let consensus = &mut harness.consensus;
    let mut outbox = Outbox::new();

    let (proposal, share) = first.proposal_input_consensus(&node_key);
    consensus.apply(proposal, &mut outbox);
    consensus.apply(share, &mut outbox);
    consensus.apply(
        state_verified_input(&first.proposal.data, first.view_number),
        &mut outbox,
    );
    confirm_writes(consensus, &mut outbox, HeldKind::Vote1, is_vote1);

    consensus.apply(first.block_reconstructed_input(), &mut outbox);
    consensus.apply(first.cert1_input(), &mut outbox);
    confirm_writes(consensus, &mut outbox, HeldKind::Vote2, is_vote2);
}

#[tokio::test]
async fn timeout_vote_goes_out_on_the_write_it_waits_for() {
    pin_test_version(TIMEOUT_EPOCH_VERSION);
    let mut harness = ConsensusHarness::new(0).await;
    let consensus = &mut harness.consensus;
    let mut outbox = Outbox::new();

    consensus.apply(ConsensusInput::Timeout(ViewNumber::new(1)), &mut outbox);
    confirm_writes(consensus, &mut outbox, HeldKind::Timeout, |output| {
        matches!(output, ConsensusOutput::SendTimeoutVote(..))
    });
}

#[tokio::test]
async fn proposal_goes_out_on_the_last_write_it_waits_for() {
    pin_test_version(TIMEOUT_EPOCH_VERSION);
    let test_data = TestData::new(4).await;
    let leader = test_data.views[1].leader_public_key;
    let mut harness = ConsensusHarness::new(node_index_for_key(&leader)).await;
    // The harness confirms every other write as consensus asks for it.
    harness.defer_action(ViewNumber::new(2), ActionKind::Propose);

    harness
        .apply_pair(test_data.views[0].proposal_input_consensus(&leader))
        .await;
    harness.apply(test_data.views[0].cert1_input()).await;

    let mut outbox = Outbox::new();
    confirm_writes(
        &mut harness.consensus,
        &mut outbox,
        HeldKind::Proposal,
        is_proposal,
    );
}
