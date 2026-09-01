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
        match epoch_matches_height(&tampered, EPOCH_HEIGHT) {
            Err(ValidationError::EpochDoesNotMatchHeight {
                expected, proposal, ..
            }) => {
                assert_eq!(expected, correct);
                assert_eq!(proposal, claimed);
            },
            other => panic!("expected EpochDoesNotMatchHeight for {claimed}, got {other:?}"),
        }
    }
}

/// With epochs disabled no block number names an epoch, so the check is inert.
#[tokio::test(flavor = "multi_thread")]
async fn epoch_unchecked_without_epoch_height() {
    let mut proposal = epoch_aware_proposal(0).await;
    proposal.epoch = EpochNumber::new(*proposal.epoch + 7);
    assert!(epoch_matches_height(&proposal, 0).is_ok());
}

/// A mismatching epoch is rejected by the validator *there* — before the leader
/// is looked up in the committee that field names. The tampered proposal keeps
/// its original signature, so any later check would report a different error.
#[tokio::test(flavor = "multi_thread")]
async fn epoch_is_checked_before_the_leader_is_resolved() {
    let data = TestData::new_with_epoch_height(3, EPOCH_HEIGHT).await;
    let signed = data.views.last().expect("a view").proposal.clone();

    let mut tampered = signed.clone();
    tampered.data.epoch += 1;

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
