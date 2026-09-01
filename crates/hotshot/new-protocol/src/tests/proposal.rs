use hotshot::types::{BLSPubKey, SignatureKey};
use hotshot_example_types::node_types::TestTypes;
use hotshot_types::data::EpochNumber;

use crate::{
    helpers::test_upgrade_lock,
    message::{Proposal, ProposalMessage},
    proposal::{ProposalValidator, ValidationError, epoch_matches_height},
    tests::common::utils::{TestData, mock_membership_with_num_nodes},
};

const EPOCH_HEIGHT: u64 = 10;

/// The last proposal of a short epoch-aware chain, with a correct epoch.
async fn epoch_aware_proposal(epoch_height: u64) -> Proposal<TestTypes> {
    let data = TestData::new_with_epoch_height(3, epoch_height).await;
    data.views.last().expect("a view").proposal.data.clone()
}

/// Blocks 1 to 21, so the chain crosses the epoch-1 and epoch-2 boundaries.
async fn chain_crossing_epoch_boundaries() -> Vec<Proposal<TestTypes>> {
    let data = TestData::new_with_epoch_height(21, EPOCH_HEIGHT).await;
    data.views.iter().map(|v| v.proposal.data.clone()).collect()
}

fn at_block(proposals: &[Proposal<TestTypes>], block_number: u64) -> Proposal<TestTypes> {
    proposals
        .iter()
        .find(|p| p.block_header.block_number == block_number)
        .unwrap_or_else(|| panic!("chain reaches block {block_number}"))
        .clone()
}

fn rejects(proposal: &Proposal<TestTypes>, epoch_height: u64, expected_epoch: u64) {
    match epoch_matches_height(proposal, epoch_height) {
        Err(ValidationError::EpochDoesNotMatchHeight {
            expected, claimed, ..
        }) => {
            assert_eq!(expected, EpochNumber::new(expected_epoch));
            assert_eq!(claimed, proposal.epoch);
        },
        other => panic!(
            "expected EpochDoesNotMatchHeight for block {} claiming epoch {}, got {other:?}",
            proposal.block_header.block_number, proposal.epoch,
        ),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn epoch_matching_its_block_number_is_accepted() {
    let proposal = epoch_aware_proposal(EPOCH_HEIGHT).await;
    assert!(epoch_matches_height(&proposal, EPOCH_HEIGHT).is_ok());
}

#[tokio::test(flavor = "multi_thread")]
async fn epoch_disagreeing_with_its_block_number_is_rejected() {
    let proposal = epoch_aware_proposal(EPOCH_HEIGHT).await;
    let correct = proposal.epoch;

    for claimed in [correct + 1, EpochNumber::new(correct.saturating_sub(1))] {
        let mut tampered = proposal.clone();
        tampered.epoch = claimed;
        rejects(&tampered, EPOCH_HEIGHT, *correct);
    }
}

/// Every block of a chain that crosses epoch boundaries carries the epoch its
/// height falls in, so the check never rejects an honest proposal — including
/// the last block of an epoch and the first block of the next one, which is
/// also where a proposal must carry `next_epoch_justify_qc`.
#[tokio::test(flavor = "multi_thread")]
async fn no_block_of_a_chain_crossing_epoch_boundaries_is_rejected() {
    let proposals = chain_crossing_epoch_boundaries().await;
    assert!(
        proposals
            .iter()
            .any(|p| p.block_header.block_number > 2 * EPOCH_HEIGHT),
        "chain must cross two epoch boundaries",
    );

    for proposal in &proposals {
        assert!(
            epoch_matches_height(proposal, EPOCH_HEIGHT).is_ok(),
            "block {} claims epoch {}",
            proposal.block_header.block_number,
            proposal.epoch,
        );
    }
}

/// The last block of an epoch belongs to that epoch, not to the next one. This
/// is the one height where the epoch is not `block_number / epoch_height + 1`,
/// and the height at which the proposer's own rule flips, so an off-by-one on
/// either side shows up here and nowhere else.
#[tokio::test(flavor = "multi_thread")]
async fn an_epoch_ends_with_the_block_that_is_a_multiple_of_its_height() {
    let proposals = chain_crossing_epoch_boundaries().await;

    let mut last_of_epoch = at_block(&proposals, EPOCH_HEIGHT);
    assert_eq!(last_of_epoch.epoch, EpochNumber::new(1));
    last_of_epoch.epoch = EpochNumber::new(2);
    rejects(&last_of_epoch, EPOCH_HEIGHT, 1);

    let mut first_of_epoch = at_block(&proposals, EPOCH_HEIGHT + 1);
    assert_eq!(first_of_epoch.epoch, EpochNumber::new(2));
    first_of_epoch.epoch = EpochNumber::new(1);
    rejects(&first_of_epoch, EPOCH_HEIGHT, 2);
}

/// Block zero is in epoch one rather than before epoch one, so it names an
/// epoch like any other height and is checked like any other height.
#[tokio::test(flavor = "multi_thread")]
async fn epoch_of_block_zero_is_checked() {
    let mut proposal = epoch_aware_proposal(EPOCH_HEIGHT).await;
    proposal.block_header.block_number = 0;

    proposal.epoch = EpochNumber::genesis();
    assert!(epoch_matches_height(&proposal, EPOCH_HEIGHT).is_ok());

    proposal.epoch = EpochNumber::new(u64::MAX);
    rejects(&proposal, EPOCH_HEIGHT, *EpochNumber::genesis());
}

/// With epochs disabled no block number names an epoch, so the check is inert.
#[tokio::test(flavor = "multi_thread")]
async fn epoch_unchecked_without_epoch_height() {
    let mut proposal = epoch_aware_proposal(0).await;
    proposal.epoch = EpochNumber::new(*proposal.epoch + 7);
    assert!(epoch_matches_height(&proposal, 0).is_ok());
}

/// The validator rejects a mismatching epoch before it looks a leader up in the
/// committee that field names.
///
/// The claimed epoch is zero, which precedes the membership's first epoch, so
/// no stake table resolves for it: were the check to run after
/// `Validator::signature`, the validator would report `NoMembershipForEpoch`
/// instead, having already reached the coordinator with an epoch the proposer
/// chose.
#[tokio::test(flavor = "multi_thread")]
async fn epoch_is_checked_before_the_leader_is_resolved() {
    let data = TestData::new_with_epoch_height(3, EPOCH_HEIGHT).await;
    let mut tampered = data.views.last().expect("a view").proposal.clone();
    tampered.data.epoch = EpochNumber::new(0);

    let (public_key, _) = BLSPubKey::generated_from_seed_indexed([0; 32], 0);
    let (membership, _storage, _client) =
        mock_membership_with_num_nodes(10, EPOCH_HEIGHT, public_key);
    let mut validator = ProposalValidator::new(membership, EPOCH_HEIGHT, test_upgrade_lock());

    validator.validate(ProposalMessage::unchecked(tampered));
    let result = validator.next().await.expect("a validation result");

    assert!(
        matches!(result, Err(ValidationError::EpochDoesNotMatchHeight { .. })),
        "expected EpochDoesNotMatchHeight, got {:?}",
        result.map(|_| ())
    );
}
