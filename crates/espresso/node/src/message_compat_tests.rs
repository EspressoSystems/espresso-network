#![cfg(test)]
//! Serialization compatibility tests for consensus messages.
//!
//! These tests generate a test vector containing one variant of each type of consensus message,
//! instantiated with sequencer types. A serialized version of this vector is written to a file and
//! checked into the repo under `data/v{version}`. If the serialization of the generated test
//! vector does not match the committed file, the test fails, indicating a potential API
//! incompatibility.
//!
//! There is one vector per protocol version, and two kinds of vector: `messages` for the HotShot
//! `Message` envelope, and `new_protocol_messages` for the new protocol (fast finality) envelope
//! introduced at version 0.6. Binary vectors are produced with `versions::encode`, the encoder
//! the network itself uses (via `UpgradeLock::serialize`), so the committed bytes are the bytes
//! on the wire.
//!
//! If this test fails and you intended to change the consensus API, you may simply replace the
//! serialized file as indicated in the test output. Note however that this may break compatibility
//! with older releases, and your pull request should explain why this is OK.
//!
//! If this test is failing and you did not intend to change the consensus API, figure out what
//! code changed caused the serialization change and revert it.

use std::{fmt::Debug, path::Path};

use alloy::primitives::U256;
use bitvec::bitvec;
use committable::Committable;
use espresso_types::{
    EpochCommittees, Leaf, Leaf2, NewProposal, NodeState, Payload, PubKey, SeqTypes, Transaction,
    ValidatedState, v0_3::Fetcher,
};
use hotshot_contract_adapter::light_client::derive_signed_state_digest;
use hotshot_example_types::{node_types::TEST_VERSIONS, storage_types::TestStorage};
use hotshot_new_protocol::message::{
    BlockMessage, CatchupEvidence, Certificate1, Certificate2, ConsensusMessage, DedupManifest,
    EpochChangeMessage, FetchRequest, Message as NewProtocolMessage, MessageType,
    PayloadFetchMessage, PayloadFetchResponse, ProposalFetchMessage, ProposalMessage,
    TimeoutVoteMessage, TransactionMessage, Unchecked, Validated, Vote1,
};
use hotshot_types::{
    PeerConfig,
    data::{
        DaProposal, EpochNumber, QuorumProposal, UpgradeProposal, VidDisperse2, ViewChangeEvidence,
        ViewNumber,
        vid_disperse::{ADVZDisperse, AvidmGf2DisperseShareFragment, AvidmGf2NamespacePiece},
    },
    epoch_membership::EpochMembershipCoordinator,
    light_client::{StakeTableState, StateKeyPair},
    message::{
        DaConsensusMessage, DataMessage, GeneralConsensusMessage, Message, MessageKind, Proposal,
        SequencingMessage,
    },
    simple_certificate::{
        DaCertificate, LightClientStateUpdateCertificateV2, QuorumCertificate, SimpleCertificate,
        TimeoutCertificate, TimeoutCertificate2, UpgradeCertificate, ViewSyncCommitCertificate,
        ViewSyncFinalizeCertificate, ViewSyncPreCommitCertificate,
    },
    simple_vote::{
        DaData, DaVote, LightClientStateUpdateVote2, QuorumData, QuorumData2, QuorumVote,
        SimpleVote, TimeoutData, TimeoutData2, TimeoutVote, UpgradeProposalData, UpgradeVote,
        ViewSyncCommitData, ViewSyncCommitVote, ViewSyncFinalizeData, ViewSyncFinalizeVote,
        ViewSyncPreCommitData, ViewSyncPreCommitVote, Vote2Data,
    },
    traits::{
        BlockPayload, EncodeBytes,
        block_contents::BlockHeader,
        election::Membership,
        node_implementation::NodeType,
        signature_key::{LCV2StateSignatureKey, LCV3StateSignatureKey, SignatureKey},
    },
    vid::avidm_gf2::AvidmGf2Scheme,
};
use pretty_assertions::assert_eq;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use vbs::version::{StaticVersion, StaticVersionType, Version};

/// Compare `messages` against the vectors committed at `data/v{minor}/{name}.{json,bin}`.
///
/// `messages` is what gets serialized, while `expected` is what the committed vectors must parse
/// back into. The two are the same value for most messages, but differ for new protocol messages,
/// which are sent in their `Validated` form and received in their `Unchecked` form.
fn check_reference_messages<Ver, S, D>(name: &str, messages: &S, expected: &D)
where
    Ver: StaticVersionType,
    S: Serialize,
    D: DeserializeOwned + PartialEq + Debug,
{
    let data_dir = Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("../../../data")
        .join(format!("v{}", Ver::VERSION.minor));

    // Ensure the current serialization implementation generates the same JSON as the committed
    // reference.
    let json_path = data_dir.join(format!("{name}.json"));
    let reference_json = std::fs::read(&json_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", json_path.display()));
    let reference_json: Value = serde_json::from_slice(&reference_json).unwrap();
    let actual = serde_json::to_value(messages).unwrap();
    if actual != reference_json {
        let actual_pretty = serde_json::to_string_pretty(&actual).unwrap();
        let expected_pretty = serde_json::to_string_pretty(&reference_json).unwrap();

        // Write the actual output to a file to make it easier to compare with/replace the expected
        // file if the serialization change was actually intended.
        let actual_path = data_dir.join(format!("{name}-actual.json"));
        std::fs::write(&actual_path, actual_pretty.as_bytes()).unwrap();

        // Fail the test with an assertion that outputs a nice diff between the prettified JSON
        // objects.
        assert_eq!(
            expected_pretty,
            actual_pretty,
            r#"
    Serialized messages do not match expected JSON. The actual serialization has been written to {}.
    If you intended to make a breaking change to the API you may replace the reference JSON file
    with it. Otherwise, revert your changes which have caused a change in the serialization of
    consensus messages.
    "#,
            actual_path.display()
        );
    }

    // Ensure the committed reference JSON can be parsed by the current message types.
    let parsed: D = serde_json::from_value(reference_json).unwrap();
    assert_eq!(&parsed, expected);

    // Ensure the wire encoder generates the same binary output as the committed reference. This
    // is the same function the network serializes messages with (via `UpgradeLock::serialize`),
    // so the committed bytes are the bytes on the wire, version prefix included.
    let bin_path = data_dir.join(format!("{name}.bin"));
    let reference_bin = std::fs::read(&bin_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", bin_path.display()));
    let actual = versions::encode(Ver::VERSION, messages).unwrap();
    if actual != reference_bin {
        // Write the actual output to a file to make it easier to compare with/replace the expected
        // file if the serialization change was actually intended.
        let actual_path = data_dir.join(format!("{name}-actual.bin"));
        std::fs::write(&actual_path, &actual).unwrap();

        // Fail the test with an assertion that outputs a diff.
        assert_eq!(
            reference_bin,
            actual,
            r#"
    Serialized messages do not match expected binary. The actual serialization has been written to
    {}. If you intended to make a breaking change to the API you may replace the reference binary
    file with it. Otherwise, revert your changes which have caused a change in the serialization of
    consensus messages.
    "#,
            actual_path.display()
        );
    }

    // Ensure the committed reference binary can be parsed by the wire decoder, and that its
    // version prefix is the version under test.
    let (version, parsed): (Version, D) = versions::decode(&reference_bin).unwrap();
    assert_eq!(version, Ver::VERSION);
    assert_eq!(&parsed, expected);
}

#[cfg(feature = "testing")]
async fn test_message_compat<Ver: StaticVersionType>(_ver: Ver) {
    let (sender, priv_key) = PubKey::generated_from_seed_indexed(Default::default(), 0);
    let signature = PubKey::sign(&priv_key, &[]).unwrap();
    let committee = vec![PeerConfig::test_default()]; /* one committee member, necessary to generate a VID share */
    let storage = TestStorage::default();
    let epoch_height = 10;

    let membership = EpochMembershipCoordinator::new(
        EpochCommittees::new_stake(
            committee.clone(),
            committee,
            None,
            Fetcher::mock(),
            epoch_height,
        ),
        epoch_height,
        &storage,
    );

    EpochMembershipCoordinator::<SeqTypes>::membership(&membership)
        .set_first_epoch(1.into(), [0u8; 32]);

    let upgrade_data = UpgradeProposalData {
        old_version: Version { major: 0, minor: 1 },
        new_version: Version { major: 1, minor: 0 },
        decide_by: ViewNumber::genesis(),
        new_version_hash: Default::default(),
        old_version_last_view: ViewNumber::genesis(),
        new_version_first_view: ViewNumber::genesis(),
    };

    let node_state = NodeState::mock()
        .with_current_version(Ver::VERSION)
        .with_genesis_version(Ver::VERSION);
    let leaf = Leaf::genesis(
        &ValidatedState::default(),
        &node_state,
        TEST_VERSIONS.test.base,
    )
    .await;
    let block_header = leaf.block_header().clone();
    let transaction = Transaction::new(1_u32.into(), vec![1, 2, 3]);
    let (payload, metadata) = Payload::from_transactions(
        [transaction.clone()],
        &ValidatedState::default(),
        &node_state,
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
                justify_qc: QuorumCertificate::genesis(
                    &ValidatedState::default(),
                    &node_state,
                    TEST_VERSIONS.test,
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
            data: ADVZDisperse::calculate_vid_disperse(
                &payload,
                &membership,
                ViewNumber::genesis(),
                Some(EpochNumber::genesis()),
                Some(EpochNumber::new(1)),
            )
            .await
            .unwrap()
            .0
            .to_shares()
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

    check_reference_messages::<Ver, _, _>("messages", &messages, &messages);
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

#[tokio::test(flavor = "multi_thread")]
async fn test_v4_message_compat() {
    test_message_compat(StaticVersion::<0, 4> {}).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_v5_message_compat() {
    test_message_compat(StaticVersion::<0, 5> {}).await;
}

/// Assemble an aggregated QC signature over `commitment` the way the vote accumulator does, from
/// a one-member stake table whose only signer is `signer`. BLS signing and signature aggregation
/// are deterministic, so the assembled signature is reproducible across runs.
fn assemble_qc_signature(
    signer: &PubKey,
    priv_key: &<PubKey as SignatureKey>::PrivateKey,
    commitment: &[u8],
) -> <PubKey as SignatureKey>::QcType {
    let partial = PubKey::sign(priv_key, commitment).unwrap();
    let stake_entries = vec![signer.stake_table_entry(U256::from(1))];
    let params = <PubKey as SignatureKey>::public_parameter(&stake_entries, U256::from(1));
    let signers = bitvec![1; stake_entries.len()];
    <PubKey as SignatureKey>::assemble(&params, signers.as_bitslice(), &[partial])
}

/// One instance of every new protocol message variant.
///
/// Optional fields are populated wherever a variant allows it, so the vector pins the encoding of
/// the `Some` case rather than the cheaper `None` case.
#[cfg(feature = "testing")]
async fn reference_new_protocol_messages() -> Vec<NewProtocolMessage<SeqTypes, Validated>> {
    let (sender, priv_key) = PubKey::generated_from_seed_indexed(Default::default(), 0);
    let signature = PubKey::sign(&priv_key, &[]).unwrap();
    let view = ViewNumber::genesis();
    let epoch = EpochNumber::new(1);
    let epoch_height = 10;

    // One committee member, necessary to generate a VID share.
    let committee = vec![PeerConfig::test_default()];
    let storage = TestStorage::default();
    let membership = EpochMembershipCoordinator::new(
        EpochCommittees::new_stake(
            committee.clone(),
            committee,
            None,
            Fetcher::mock(),
            epoch_height,
        ),
        epoch_height,
        &storage,
    );
    EpochMembershipCoordinator::<SeqTypes>::membership(&membership)
        .set_first_epoch(1.into(), [0u8; 32]);

    let version = <StaticVersion<0, 6> as StaticVersionType>::VERSION;
    let node_state = NodeState::mock()
        .with_current_version(version)
        .with_genesis_version(version);
    let leaf = Leaf2::genesis(&ValidatedState::default(), &node_state, version).await;
    let block_header = leaf.block_header().clone();
    let leaf_commit = leaf.commit();

    let transaction = Transaction::new(1_u32.into(), vec![1, 2, 3]);
    let (payload, metadata) = Payload::from_transactions(
        [transaction.clone()],
        &ValidatedState::default(),
        &node_state,
    )
    .await
    .unwrap();

    // Unlike the legacy message compat vector, certificates carry a real aggregated signature, so
    // the vector pins the `Some((signature, signers))` encoding rather than the `None` case.
    let quorum_data = QuorumData2::<SeqTypes> {
        leaf_commit,
        epoch: Some(epoch),
        block_number: Some(0),
    };
    let cert1: Certificate1<SeqTypes> = SimpleCertificate::new(
        quorum_data,
        quorum_data.commit(),
        view,
        Some(assemble_qc_signature(
            &sender,
            &priv_key,
            quorum_data.commit().as_ref(),
        )),
        Default::default(),
    );
    let vote2_data = Vote2Data::<SeqTypes> {
        leaf_commit,
        epoch,
        block_number: 0,
    };
    let cert2: Certificate2<SeqTypes> = SimpleCertificate::new(
        vote2_data.clone(),
        vote2_data.commit(),
        view,
        Some(assemble_qc_signature(
            &sender,
            &priv_key,
            vote2_data.commit().as_ref(),
        )),
        Default::default(),
    );
    let timeout_data = TimeoutData2 {
        view,
        epoch: Some(epoch),
    };
    let timeout_cert: TimeoutCertificate2<SeqTypes> = SimpleCertificate::new(
        timeout_data.clone(),
        timeout_data.commit(),
        view,
        Some(assemble_qc_signature(
            &sender,
            &priv_key,
            timeout_data.commit().as_ref(),
        )),
        Default::default(),
    );
    let upgrade_data = UpgradeProposalData {
        old_version: Version { major: 0, minor: 1 },
        new_version: Version { major: 1, minor: 0 },
        decide_by: view,
        new_version_hash: Default::default(),
        old_version_last_view: view,
        new_version_first_view: view,
    };
    let upgrade_cert = UpgradeCertificate::new(
        upgrade_data.clone(),
        upgrade_data.commit(),
        view,
        Some(assemble_qc_signature(
            &sender,
            &priv_key,
            upgrade_data.commit().as_ref(),
        )),
        Default::default(),
    );

    let proposal = NewProposal::<SeqTypes> {
        block_header: block_header.clone(),
        view_number: view,
        epoch,
        justify_qc: cert1.clone(),
        next_epoch_justify_qc: Some(cert2.clone()),
        upgrade_certificate: Some(upgrade_cert),
        view_change_evidence: Some(timeout_cert.clone()),
        next_drb_result: Some([1u8; 32]),
        state_cert: Some(LightClientStateUpdateCertificateV2::<SeqTypes>::genesis()),
    };
    let signed_proposal = Proposal::new(proposal.clone(), signature.clone());

    // Schnorr signing derives its nonce from the signing key and message, so a real state vote is
    // reproducible across runs.
    let state_key_pair = StateKeyPair::generate_from_seed_indexed([0u8; 32], 0);
    let state_sign_key = state_key_pair.sign_key_ref();
    let next_stake_table_state = StakeTableState::default();
    let light_client_state =
        <_ as BlockHeader<SeqTypes>>::get_light_client_state(&block_header, view).unwrap();
    let auth_root = <_ as BlockHeader<SeqTypes>>::auth_root(&block_header).unwrap();
    let v2_signature =
        <<SeqTypes as NodeType>::StateSignatureKey as LCV2StateSignatureKey>::sign_state(
            state_sign_key,
            &light_client_state,
            &next_stake_table_state,
        )
        .unwrap();
    let signed_state_digest =
        derive_signed_state_digest(&light_client_state, &next_stake_table_state, &auth_root);
    let state_signature =
        <<SeqTypes as NodeType>::StateSignatureKey as LCV3StateSignatureKey>::sign_state(
            state_sign_key,
            signed_state_digest,
        )
        .unwrap();
    let state_vote = LightClientStateUpdateVote2::<SeqTypes> {
        epoch,
        light_client_state,
        next_stake_table_state,
        signature: state_signature,
        v2_signature,
        auth_root,
        signed_state_digest,
    };

    // A full VID share, and one fragment of it, produced the way the disperser produces them.
    let vid_share = VidDisperse2::<SeqTypes>::calculate_vid_disperse(
        &payload,
        &membership,
        view,
        Some(epoch),
        Some(epoch),
        &metadata,
    )
    .await
    .unwrap()
    .0
    .to_shares()
    .remove(0);
    let params =
        VidDisperse2::<SeqTypes>::disperse_params(&payload, &membership, Some(epoch), &metadata)
            .unwrap();
    let dispersal = AvidmGf2Scheme::ns_disperse_one(
        &params.param,
        &params.weights,
        &params.payload[params.ns_table[0].clone()],
        0,
    )
    .unwrap();
    let payload_commitment = vid_share.payload_commitment;
    let vid_fragment = AvidmGf2DisperseShareFragment::<SeqTypes> {
        view_number: view,
        epoch: Some(epoch),
        target_epoch: Some(epoch),
        payload_commitment: vid_share.payload_commitment,
        recipient_key: params.recipients[0],
        param: params.param.clone(),
        num_namespaces: params.ns_table.len(),
        namespaces: vec![AvidmGf2NamespacePiece {
            ns_index: dispersal.ns_index,
            ns_payload_byte_len: dispersal.payload_byte_len,
            ns_commit: dispersal.commit,
            ns_share: dispersal.shares[0].clone(),
        }],
    };

    let consensus_messages = vec![
        ConsensusMessage::Proposal(ProposalMessage::validated(signed_proposal.clone())),
        ConsensusMessage::Vote1(Vote1 {
            vote: SimpleVote {
                signature: (sender, signature.clone()),
                data: quorum_data,
                view_number: view,
            },
            state_vote: Some(state_vote),
        }),
        ConsensusMessage::Vote2(SimpleVote {
            signature: (sender, signature.clone()),
            data: vote2_data,
            view_number: view,
        }),
        ConsensusMessage::Certificate1(cert1.clone(), sender),
        ConsensusMessage::Certificate2(cert2.clone(), sender),
        // Both `CatchupEvidence` variants, since a timeout vote carries either one.
        ConsensusMessage::TimeoutVote(TimeoutVoteMessage {
            vote: SimpleVote {
                signature: (sender, signature.clone()),
                data: timeout_data.clone(),
                view_number: view,
            },
            evidence: Some(CatchupEvidence::Qc(cert1.clone())),
        }),
        ConsensusMessage::TimeoutVote(TimeoutVoteMessage {
            vote: SimpleVote {
                signature: (sender, signature.clone()),
                data: timeout_data,
                view_number: view,
            },
            evidence: Some(CatchupEvidence::Tc(timeout_cert.clone())),
        }),
        ConsensusMessage::TimeoutCertificate(timeout_cert),
        ConsensusMessage::EpochChange(EpochChangeMessage::validated(
            cert1.clone(),
            cert2,
            proposal,
        )),
        ConsensusMessage::VidShareFragment(Proposal::new(vid_fragment, signature.clone())),
        ConsensusMessage::VidShareBroadcast(vid_share),
        ConsensusMessage::HighQc(cert1),
    ];

    let message_types = consensus_messages
        .into_iter()
        .map(MessageType::Consensus)
        .chain([
            MessageType::Block(BlockMessage::Transactions(TransactionMessage {
                view,
                transactions: vec![transaction.clone()],
            })),
            MessageType::Block(BlockMessage::DedupManifest(DedupManifest {
                view,
                epoch,
                hashes: vec![transaction.commit()],
            })),
            MessageType::ProposalFetch(ProposalFetchMessage::Request(
                FetchRequest::new(view, sender, &priv_key).unwrap(),
            )),
            MessageType::ProposalFetch(ProposalFetchMessage::Response(Box::new(signed_proposal))),
            // External payloads bypass this envelope on the wire, so this entry pins only the
            // encoding of the variant itself.
            MessageType::External(vec![1, 2, 3]),
            MessageType::PayloadFetch(PayloadFetchMessage::Request(
                FetchRequest::new(view, sender, &priv_key).unwrap(),
            )),
            MessageType::PayloadFetch(PayloadFetchMessage::Response(PayloadFetchResponse {
                view,
                payload_commitment,
                payload: payload.encode().to_vec(),
            })),
            MessageType::ShareFetch(FetchRequest::new(view, sender, &priv_key).unwrap()),
        ]);

    message_types
        .map(|message_type| NewProtocolMessage {
            sender,
            message_type,
        })
        .collect()
}

#[cfg(feature = "testing")]
#[tokio::test(flavor = "multi_thread")]
async fn test_v6_new_protocol_message_compat() {
    let messages = reference_new_protocol_messages().await;
    // A node parses what it receives as `Unchecked`, so that is the form the committed vectors are
    // compared against.
    let unchecked: Vec<NewProtocolMessage<SeqTypes, Unchecked>> = messages
        .iter()
        .cloned()
        .map(NewProtocolMessage::into_unchecked)
        .collect();

    check_reference_messages::<StaticVersion<0, 6>, _, _>(
        "new_protocol_messages",
        &messages,
        &unchecked,
    );
}
