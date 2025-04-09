#![cfg(test)]
//! Serialization compatibility tests for consensus messages.
//!
//! This test generates a test vector containing one variant of each type of consensus message,
//! instantiated with sequencer types. A serialized version of this vector is written to a file and
//! checked into the repo under `data/messages.json`. If the serialization of the generated test
//! vector does not match the committed file, the test fails, indicating a potential API
//! incompatibility.
//!
//! If this test fails and you intended to change the consensus API, you may simply replace the
//! serialized file as indicated in the test output. Note however that this may break compatibility
//! with older releases, and your pull request should explain why this is OK.
//!
//! If this test is failing and you did not intend to change the consensus API, figure out what
//! code changed caused the serialization change and revert it.

use std::path::Path;

use committable::Committable;
use espresso_types::{NodeState, PubKey, ValidatedState};
use hotshot_types::{
    data::{
        DaProposal, EpochNumber, QuorumProposal, UpgradeProposal, ViewChangeEvidence, ViewNumber,
    },
    message::{
        DaConsensusMessage, DataMessage, GeneralConsensusMessage, Message, MessageKind, Proposal,
        SequencingMessage,
    },
    simple_certificate::{DaCertificate, QuorumCertificate, UpgradeCertificate},
    simple_vote::{DaData, DaVote, QuorumData, QuorumVote, UpgradeProposalData, UpgradeVote},
    traits::{
        node_implementation::ConsensusTime, signature_key::SignatureKey, BlockPayload, EncodeBytes,
    },
};
use pretty_assertions::assert_eq;
use serde_json::Value;
use vbs::{
    version::{StaticVersion, StaticVersionType, Version},
    BinarySerializer,
};

#[cfg(feature = "testing")]
async fn test_message_compat<Ver: StaticVersionType>(_ver: Ver) {
    use std::sync::Arc;

    use async_lock::RwLock;
    use espresso_types::{
        v0_3::StakeTableFetcher, EpochCommittees, Leaf, Payload, SeqTypes, Transaction,
    };
    use hotshot_example_types::node_types::TestVersions;
    use hotshot_types::{
        data::vid_disperse::{ADVZDisperse, ADVZDisperseShare},
        epoch_membership::EpochMembershipCoordinator,
        simple_certificate::{
            TimeoutCertificate, ViewSyncCommitCertificate, ViewSyncFinalizeCertificate,
            ViewSyncPreCommitCertificate,
        },
        simple_vote::{
            TimeoutData, TimeoutVote, ViewSyncCommitData, ViewSyncCommitVote, ViewSyncFinalizeData,
            ViewSyncFinalizeVote, ViewSyncPreCommitData, ViewSyncPreCommitVote,
        },
        PeerConfig,
    };

    let (sender, priv_key) = PubKey::generated_from_seed_indexed(Default::default(), 0);
    let signature = PubKey::sign(&priv_key, &[]).unwrap();
    let committee = vec![PeerConfig::default()]; /* one committee member, necessary to generate a VID share */

    let membership = EpochMembershipCoordinator::new(
        Arc::new(RwLock::new(EpochCommittees::new_stake(
            committee.clone(),
            committee,
            StakeTableFetcher::mock(),
        ))),
        None,
        10,
    );
    let upgrade_data = UpgradeProposalData {
        old_version: Version { major: 0, minor: 1 },
        new_version: Version { major: 1, minor: 0 },
        decide_by: ViewNumber::genesis(),
        new_version_hash: Default::default(),
        old_version_last_view: ViewNumber::genesis(),
        new_version_first_view: ViewNumber::genesis(),
    };
    let leaf = Leaf::genesis::<TestVersions>(
        &ValidatedState::default(),
        &NodeState::mock().with_current_version(Ver::VERSION),
    )
    .await;
    let block_header = leaf.block_header().clone();
    let transaction = Transaction::new(1_u32.into(), vec![1, 2, 3]);
    let (payload, metadata) = Payload::from_transactions(
        [transaction.clone()],
        &ValidatedState::default(),
        &NodeState::mock(),
    )
    .await
    .unwrap();
    let view_sync_pre_commit_data = ViewSyncPreCommitData {
        relay: 0,
        round: ViewNumber::genesis(),
    };
    let view_sync_commit_data = ViewSyncCommitData {
        relay: 0,
        round: ViewNumber::genesis(),
    };
    let view_sync_finalize_data = ViewSyncFinalizeData {
        relay: 0,
        round: ViewNumber::genesis(),
    };
    let timeout_data = TimeoutData {
        view: ViewNumber::genesis(),
    };
    let da_data = DaData {
        payload_commit: block_header.payload_commitment(),
    };

    let consensus_messages = [
        GeneralConsensusMessage::Proposal(Proposal {
            data: QuorumProposal {
                block_header: block_header.clone(),
                view_number: ViewNumber::genesis(),
                justify_qc: QuorumCertificate::genesis::<TestVersions>(
                    &ValidatedState::default(),
                    &NodeState::mock(),
                )
                .await,
                upgrade_certificate: Some(UpgradeCertificate::new(
                    upgrade_data.clone(),
                    upgrade_data.commit(),
                    ViewNumber::genesis(),
                    Default::default(),
                    Default::default(),
                )),
                proposal_certificate: Some(ViewChangeEvidence::Timeout(TimeoutCertificate::new(
                    timeout_data.clone(),
                    timeout_data.commit(),
                    ViewNumber::genesis(),
                    Default::default(),
                    Default::default(),
                ))),
            },
            signature: signature.clone(),
            _pd: Default::default(),
        }),
        GeneralConsensusMessage::Vote(QuorumVote {
            signature: (sender, signature.clone()),
            data: QuorumData {
                leaf_commit: <Leaf as Committable>::commit(&leaf),
            },
            view_number: ViewNumber::genesis(),
        }),
        GeneralConsensusMessage::ViewSyncPreCommitVote(ViewSyncPreCommitVote {
            signature: (sender, signature.clone()),
            data: view_sync_pre_commit_data.clone(),
            view_number: ViewNumber::genesis(),
        }),
        GeneralConsensusMessage::ViewSyncCommitVote(ViewSyncCommitVote {
            signature: (sender, signature.clone()),
            data: view_sync_commit_data.clone(),
            view_number: ViewNumber::genesis(),
        }),
        GeneralConsensusMessage::ViewSyncFinalizeVote(ViewSyncFinalizeVote {
            signature: (sender, signature.clone()),
            data: view_sync_finalize_data.clone(),
            view_number: ViewNumber::genesis(),
        }),
        GeneralConsensusMessage::ViewSyncPreCommitCertificate(ViewSyncPreCommitCertificate::new(
            view_sync_pre_commit_data.clone(),
            view_sync_pre_commit_data.commit(),
            ViewNumber::genesis(),
            Default::default(),
            Default::default(),
        )),
        GeneralConsensusMessage::ViewSyncCommitCertificate(ViewSyncCommitCertificate::new(
            view_sync_commit_data.clone(),
            view_sync_commit_data.commit(),
            ViewNumber::genesis(),
            Default::default(),
            Default::default(),
        )),
        GeneralConsensusMessage::ViewSyncFinalizeCertificate(ViewSyncFinalizeCertificate::new(
            view_sync_finalize_data.clone(),
            view_sync_finalize_data.commit(),
            ViewNumber::genesis(),
            Default::default(),
            Default::default(),
        )),
        GeneralConsensusMessage::TimeoutVote(TimeoutVote {
            signature: (sender, signature.clone()),
            data: TimeoutData {
                view: ViewNumber::genesis(),
            },
            view_number: ViewNumber::genesis(),
        }),
        GeneralConsensusMessage::UpgradeProposal(Proposal {
            data: UpgradeProposal {
                upgrade_proposal: upgrade_data.clone(),
                view_number: ViewNumber::genesis(),
            },
            signature: signature.clone(),
            _pd: Default::default(),
        }),
        GeneralConsensusMessage::UpgradeVote(UpgradeVote {
            signature: (sender, signature.clone()),
            data: upgrade_data,
            view_number: ViewNumber::genesis(),
        }),
    ];
    let da_messages = [
        DaConsensusMessage::DaProposal(Proposal {
            data: DaProposal {
                encoded_transactions: payload.encode(),
                metadata,
                view_number: ViewNumber::genesis(),
            },
            signature: signature.clone(),
            _pd: Default::default(),
        }),
        DaConsensusMessage::DaVote(DaVote {
            signature: (sender, signature.clone()),
            data: da_data.clone(),
            view_number: ViewNumber::genesis(),
        }),
        DaConsensusMessage::DaCertificate(DaCertificate::new(
            da_data.clone(),
            da_data.commit(),
            ViewNumber::genesis(),
            Default::default(),
            Default::default(),
        )),
        DaConsensusMessage::VidDisperseMsg(Proposal {
            data: ADVZDisperseShare::from_advz_disperse(
                ADVZDisperse::calculate_vid_disperse(
                    &payload,
                    &membership,
                    ViewNumber::genesis(),
                    Some(EpochNumber::genesis()),
                    Some(EpochNumber::new(1)),
                )
                .await
                .unwrap(),
            )
            .remove(0),
            signature: signature.clone(),
            _pd: Default::default(),
        }),
    ];
    let data_messages = [DataMessage::SubmitTransaction(
        transaction,
        ViewNumber::genesis(),
    )];

    let seq_messages = consensus_messages
        .into_iter()
        .map(SequencingMessage::General)
        .chain(da_messages.into_iter().map(SequencingMessage::Da));
    let messages = seq_messages
        .map(MessageKind::Consensus)
        .chain(data_messages.into_iter().map(MessageKind::Data))
        .map(|kind| Message { kind, sender })
        .collect::<Vec<Message<SeqTypes>>>();

    let version_sub_dir = format!("v{}", Ver::VERSION.minor);
    // Load the expected serialization from the repo.
    let data_dir = Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("../data")
        .join(version_sub_dir);
    let expected_bytes = std::fs::read(data_dir.join("messages.json")).unwrap();
    let expected: Value = serde_json::from_slice(&expected_bytes).unwrap();

    // Ensure the current serialization implementation generates the same JSON as the committed
    // reference.
    let actual = serde_json::to_value(&messages).unwrap();
    if actual != expected {
        let actual_pretty = serde_json::to_string_pretty(&actual).unwrap();
        let expected_pretty = serde_json::to_string_pretty(&expected).unwrap();

        // Write the actual output to a file to make it easier to compare with/replace the expected
        // file if the serialization change was actually intended.
        let actual_path = data_dir.join("messages-actual.json");
        std::fs::write(&actual_path, actual_pretty.as_bytes()).unwrap();

        // Fail the test with an assertion that outputs a nice diff between the prettified JSON
        // objects.
        assert_eq!(
            expected_pretty,
            actual_pretty,
            r#"
    Serialized messages do not match expected JSON. The actual serialization has been written to {}.
    If you intended to make a breaking change to the API you may replace the reference JSON file
    /data/messages.json with /data/messages-actual.json. Otherwise, revert your changes which have
    caused a change in the serialization of HotShot messages.
    "#,
            actual_path.display()
        );
    }

    // Ensure the current `Message` type can be parsed from the committed reference JSON.
    let parsed: Vec<Message<SeqTypes>> = serde_json::from_value(expected).unwrap();
    assert_eq!(parsed, messages);

    // Ensure the current serialization implementation generates the same binary output as the
    // committed reference.
    let expected = std::fs::read(data_dir.join("messages.bin")).unwrap();
    let actual = vbs::Serializer::<Ver>::serialize(&messages).unwrap();
    if actual != expected {
        // Write the actual output to a file to make it easier to compare with/replace the expected
        // file if the serialization change was actually intended.
        let actual_path = data_dir.join("messages-actual.bin");
        std::fs::write(&actual_path, &actual).unwrap();

        // Fail the test with an assertion that outputs a diff.
        assert_eq!(
            expected,
            actual,
            r#"
    Serialized messages do not match expected binary. The actual serialization has been written to
    {}. If you intended to make a breaking change to the API you may replace the reference binary
    file /data/messages.bin with /data/messages-actual.bin. Otherwise, revert your changes which
    have caused a change in the serialization of HotShot messages.
    "#,
            actual_path.display()
        );
    }

    // Ensure the current `Message` type can be parsed from the committed reference binary.
    let parsed: Vec<Message<SeqTypes>> = vbs::Serializer::<Ver>::deserialize(&expected).unwrap();
    assert_eq!(parsed, messages);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_v1_message_compat() {
    test_message_compat(StaticVersion::<0, 1> {}).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_v2_message_compat() {
    test_message_compat(StaticVersion::<0, 2> {}).await;
}
#[tokio::test(flavor = "multi_thread")]
async fn test_v3_message_compat() {
    test_message_compat(StaticVersion::<0, 3> {}).await;
}
