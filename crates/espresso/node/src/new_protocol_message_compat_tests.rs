#![cfg(test)]
//! Serialization compatibility tests for new protocol (fast finality) messages.
//!
//! This test generates a test vector containing one variant of each type of new protocol message,
//! instantiated with sequencer types. A serialized version of this vector is written to a file and
//! checked into the repo under `data/v6/new_protocol_messages.json`. If the serialization of the
//! generated test vector does not match the committed file, the test fails, indicating a potential
//! API incompatibility.
//!
//! These messages are what nodes exchange over cliquenet once the network runs at
//! `NEW_PROTOCOL_VERSION` (0.6). The binary vector is produced with the same versioned
//! serialization the network uses, so the committed bytes are the bytes on the wire.
//!
//! Messages are serialized in their `Validated` form and parsed back in their `Unchecked` form,
//! matching how a node serializes what it sends and deserializes what it receives.
//!
//! If this test fails and you intended to change the consensus API, you may simply replace the
//! serialized file as indicated in the test output. Note however that this may break compatibility
//! with older releases, and your pull request should explain why this is OK.
//!
//! If this test is failing and you did not intend to change the consensus API, figure out what
//! code changed caused the serialization change and revert it.

use std::path::Path;

use pretty_assertions::assert_eq;
use serde_json::Value;
use vbs::{
    BinarySerializer,
    version::{StaticVersion, StaticVersionType, Version},
};

/// Serializer for `NEW_PROTOCOL_VERSION`, matching what nodes use on the wire.
type V6Serializer = vbs::Serializer<StaticVersion<0, 6>>;

/// One instance of every new protocol message variant.
///
/// Optional fields are populated wherever a variant allows it, so the vector pins the encoding of
/// the `Some` case rather than the cheaper `None` case.
#[cfg(feature = "testing")]
async fn reference_messages() -> Vec<
    hotshot_new_protocol::message::Message<
        espresso_types::SeqTypes,
        hotshot_new_protocol::message::Validated,
    >,
> {
    use committable::Committable;
    use espresso_types::{
        EpochCommittees, Leaf2, NewProposal, NodeState, Payload, PubKey, SeqTypes, Transaction,
        ValidatedState, v0_3::Fetcher,
    };
    use hotshot_contract_adapter::light_client::derive_signed_state_digest;
    use hotshot_example_types::storage_types::TestStorage;
    use hotshot_new_protocol::message::{
        BlockMessage, CatchupEvidence, Certificate1, Certificate2, ConsensusMessage, DedupManifest,
        EpochChangeMessage, Message, MessageType, ProposalFetchMessage, ProposalFetchRequest,
        ProposalMessage, TimeoutVoteMessage, TransactionMessage, Vote1,
    };
    use hotshot_types::{
        PeerConfig,
        data::{
            EpochNumber, VidDisperse2, ViewNumber,
            vid_disperse::{AvidmGf2DisperseShareFragment, AvidmGf2NamespacePiece},
        },
        epoch_membership::EpochMembershipCoordinator,
        light_client::{StakeTableState, StateKeyPair},
        message::Proposal as SignedProposal,
        simple_certificate::{
            LightClientStateUpdateCertificateV2, SimpleCertificate, TimeoutCertificate2,
            UpgradeCertificate,
        },
        simple_vote::{
            LightClientStateUpdateVote2, QuorumData2, SimpleVote, TimeoutData2,
            UpgradeProposalData, Vote2Data,
        },
        traits::{
            BlockPayload,
            block_contents::BlockHeader,
            election::Membership,
            node_implementation::NodeType,
            signature_key::{LCV2StateSignatureKey, LCV3StateSignatureKey, SignatureKey},
        },
        vid::avidm_gf2::AvidmGf2Scheme,
    };

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

    // Certificates carry no aggregated signature, as in the legacy message compat vector: an
    // assembled signature is not reproducible across runs.
    let quorum_data = QuorumData2::<SeqTypes> {
        leaf_commit,
        epoch: Some(epoch),
        block_number: Some(0),
    };
    let cert1: Certificate1<SeqTypes> = SimpleCertificate::new(
        quorum_data,
        quorum_data.commit(),
        view,
        None,
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
        None,
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
        None,
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
        Default::default(),
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
    let signed_proposal = SignedProposal::new(proposal.clone(), signature.clone());

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
        ConsensusMessage::VidShareFragment(SignedProposal::new(vid_fragment, signature.clone())),
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
                ProposalFetchRequest::new(view, sender, &priv_key).unwrap(),
            )),
            MessageType::ProposalFetch(ProposalFetchMessage::Response(Box::new(signed_proposal))),
            // External payloads bypass this envelope on the wire, so this entry pins only the
            // encoding of the variant itself.
            MessageType::External(vec![1, 2, 3]),
        ]);

    message_types
        .map(|message_type| Message {
            sender,
            message_type,
        })
        .collect()
}

#[cfg(feature = "testing")]
#[tokio::test(flavor = "multi_thread")]
async fn test_v6_new_protocol_message_compat() {
    use espresso_types::SeqTypes;
    use hotshot_new_protocol::message::{Message, Unchecked};

    let messages = reference_messages().await;
    // A node parses what it receives as `Unchecked`, so that is what the committed vectors are
    // compared against.
    let unchecked: Vec<Message<SeqTypes, Unchecked>> = messages
        .iter()
        .cloned()
        .map(Message::into_unchecked)
        .collect();

    let data_dir =
        Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../../../data/v6");

    // Ensure the current serialization implementation generates the same JSON as the committed
    // reference.
    let expected_bytes = std::fs::read(data_dir.join("new_protocol_messages.json")).unwrap();
    let expected: Value = serde_json::from_slice(&expected_bytes).unwrap();
    let actual = serde_json::to_value(&messages).unwrap();
    if actual != expected {
        let actual_pretty = serde_json::to_string_pretty(&actual).unwrap();
        let expected_pretty = serde_json::to_string_pretty(&expected).unwrap();

        // Write the actual output to a file to make it easier to compare with/replace the expected
        // file if the serialization change was actually intended.
        let actual_path = data_dir.join("new_protocol_messages-actual.json");
        std::fs::write(&actual_path, actual_pretty.as_bytes()).unwrap();

        // Fail the test with an assertion that outputs a nice diff between the prettified JSON
        // objects.
        assert_eq!(
            expected_pretty,
            actual_pretty,
            r#"
    Serialized messages do not match expected JSON. The actual serialization has been written to {}.
    If you intended to make a breaking change to the API you may replace the reference JSON file
    /data/v6/new_protocol_messages.json with /data/v6/new_protocol_messages-actual.json. Otherwise,
    revert your changes which have caused a change in the serialization of new protocol messages.
    "#,
            actual_path.display()
        );
    }

    // Ensure the current `Message` type can be parsed from the committed reference JSON.
    let parsed: Vec<Message<SeqTypes, Unchecked>> = serde_json::from_value(expected).unwrap();
    assert_eq!(parsed, unchecked);

    // Ensure the current serialization implementation generates the same binary output as the
    // committed reference.
    let expected = std::fs::read(data_dir.join("new_protocol_messages.bin")).unwrap();
    let actual = V6Serializer::serialize(&messages).unwrap();
    if actual != expected {
        // Write the actual output to a file to make it easier to compare with/replace the expected
        // file if the serialization change was actually intended.
        let actual_path = data_dir.join("new_protocol_messages-actual.bin");
        std::fs::write(&actual_path, &actual).unwrap();

        // Fail the test with an assertion that outputs a diff.
        assert_eq!(
            expected,
            actual,
            r#"
    Serialized messages do not match expected binary. The actual serialization has been written to
    {}. If you intended to make a breaking change to the API you may replace the reference binary
    file /data/v6/new_protocol_messages.bin with /data/v6/new_protocol_messages-actual.bin.
    Otherwise, revert your changes which have caused a change in the serialization of new protocol
    messages.
    "#,
            actual_path.display()
        );
    }

    // The committed bytes are the bytes on the wire, which are prefixed with the protocol version.
    assert_eq!(expected[..4], [0, 0, 6, 0]);

    // Ensure the current `Message` type can be parsed from the committed reference binary.
    let parsed: Vec<Message<SeqTypes, Unchecked>> = V6Serializer::deserialize(&expected).unwrap();
    assert_eq!(parsed, unchecked);
}
