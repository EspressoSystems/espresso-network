#![cfg(test)]
//! Reference data types.
//!
//! This module provides some reference instantiations of various data types which have an external,
//! language-independent interface (e.g. serialization or commitment scheme). Ports of the sequencer
//! to other languages, as well as downstream packages written in other languages, can use these
//! references objects and their known serializations and commitments to check that their
//! implementations are compatible with this reference implementation.
//!
//! The serialized form of each reference object, in both JSON and binary forms, is available in the
//! `data` directory of the repo for this crate. These files checked against the reference objects
//! in this module to prevent unintentional changes to the serialization format. Thus, these
//! serialized files can be used as test vectors for a port of the serialiation scheme to another
//! language or framework.
//!
//! To get the byte representation or U256 representation of a commitment for testing in other
//! packages, run the tests and look for "commitment bytes" or "commitment U256" in the logs.
//!
//! These tests may fail if you make a breaking change to a commitment scheme, serialization, etc.
//! If this happens, be sure you _want_ to break the API, and, if so, simply replace the relevant
//! constant in this module with the "actual" value that can be found in the logs of the failing
//! test.

use std::{fmt::Debug, path::Path, str::FromStr};

use alloy::primitives::U256;
use committable::Committable;
use hotshot_query_service::{
    availability::QueryablePayload, testing::mocks::MockVersions, VidCommon,
};
use hotshot_types::{
    data::vid_commitment,
    traits::{signature_key::BuilderSignatureKey, BlockPayload, EncodeBytes},
    vid::{advz::advz_scheme, avidm::init_avidm_param},
};
use jf_merkle_tree::MerkleTreeScheme;
use jf_vid::VidScheme;
use pretty_assertions::assert_eq;
use rand::{Rng, RngCore};
use sequencer_utils::{commitment_to_u256, test_utils::setup_test};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use tagged_base64::TaggedBase64;
use vbs::{
    version::{StaticVersion, StaticVersionType, Version},
    BinarySerializer,
};

use crate::{
    v0_1::{self, ADVZNsProof},
    v0_2, v0_3, ADVZNamespaceProofQueryData, FeeAccount, FeeInfo, Header, L1BlockInfo, NamespaceId,
    NamespaceProofQueryData, NsProof, NsTable, Payload, SeqTypes, Transaction, ValidatedState,
};

type V1Serializer = vbs::Serializer<StaticVersion<0, 1>>;
type V2Serializer = vbs::Serializer<StaticVersion<0, 2>>;
type V3Serializer = vbs::Serializer<StaticVersion<0, 3>>;
type V99Serializer = vbs::Serializer<StaticVersion<0, 99>>;

const REFERENCE_NAMESPACE_ID: u32 = 12648430;

async fn reference_payload() -> Payload {
    const NUM_NS_IDS: usize = 3;
    let ns_ids: [NamespaceId; NUM_NS_IDS] = [
        REFERENCE_NAMESPACE_ID.into(),
        314159265_u32.into(),
        2718281828_u32.into(),
    ];

    let mut rng = jf_utils::test_rng();
    let txs = {
        const NUM_TXS: usize = 20;
        let mut txs = Vec::with_capacity(NUM_TXS);
        for _ in 0..NUM_TXS {
            let ns_id = ns_ids[rng.gen_range(0..NUM_NS_IDS)];
            txs.push(reference_transaction(ns_id, &mut rng));
        }
        txs
    };

    Payload::from_transactions(txs, &Default::default(), &Default::default())
        .await
        .unwrap()
        .0
}

async fn reference_ns_proof_legacy() -> ADVZNamespaceProofQueryData {
    let payload = reference_payload().await;
    let ns_index = payload
        .ns_table()
        .find_ns_id(&(REFERENCE_NAMESPACE_ID.into()))
        .unwrap();
    let enc = payload.encode();
    let mut scheme = advz_scheme(10);
    let disperse = VidScheme::disperse(&mut scheme, &enc).unwrap();

    let proof = ADVZNsProof::new(&payload, &ns_index, &disperse.common);
    let transactions = proof
        .as_ref()
        .unwrap()
        .export_all_txs(&REFERENCE_NAMESPACE_ID.into());
    ADVZNamespaceProofQueryData {
        proof,
        transactions,
    }
}

async fn reference_ns_proof_enum_advz() -> NamespaceProofQueryData {
    let payload = reference_payload().await;
    let ns_index = payload
        .ns_table()
        .find_ns_id(&(REFERENCE_NAMESPACE_ID.into()))
        .unwrap();
    let enc = payload.encode();
    let mut scheme = advz_scheme(10);
    let disperse = VidScheme::disperse(&mut scheme, &enc).unwrap();

    let proof = NsProof::new(&payload, &ns_index, &VidCommon::V0(disperse.common));
    let transactions = proof
        .as_ref()
        .unwrap()
        .export_all_txs(&REFERENCE_NAMESPACE_ID.into());
    NamespaceProofQueryData {
        proof,
        transactions,
    }
}

async fn reference_ns_proof_enum_avidm() -> NamespaceProofQueryData {
    let payload = reference_payload().await;
    let ns_index = payload
        .ns_table()
        .find_ns_id(&(REFERENCE_NAMESPACE_ID.into()))
        .unwrap();

    let avid_m_param = init_avidm_param(10).unwrap();
    let proof = NsProof::new(&payload, &ns_index, &VidCommon::V1(avid_m_param));
    let transactions = proof
        .as_ref()
        .unwrap()
        .export_all_txs(&REFERENCE_NAMESPACE_ID.into());
    NamespaceProofQueryData {
        proof,
        transactions,
    }
}

async fn reference_ns_table() -> NsTable {
    reference_payload().await.ns_table().clone()
}

const REFERENCE_NS_TABLE_COMMITMENT: &str = "NSTABLE~tMW0-hGn0563bgYgvsO9r95f2AUiTD_2tvjDOuGRwNA5";

fn reference_l1_block() -> L1BlockInfo {
    L1BlockInfo {
        number: 123,
        timestamp: U256::from(0x456),
        hash: "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
            .parse()
            .unwrap(),
    }
}

const REFERENCE_L1_BLOCK_COMMITMENT: &str = "L1BLOCK~4HpzluLK2Isz3RdPNvNrDAyQcWOF2c9JeLZzVNLmfpQ9";

fn reference_chain_config() -> crate::v0_99::ChainConfig {
    crate::v0_99::ChainConfig {
        chain_id: 0x8a19.into(),
        max_block_size: 10240.into(),
        base_fee: 0.into(),
        fee_contract: Some(Default::default()),
        fee_recipient: Default::default(),
        bid_recipient: Some(Default::default()),
        stake_table_contract: Some(Default::default()),
    }
}

const REFERENCE_V1_CHAIN_CONFIG_COMMITMENT: &str =
    "CHAIN_CONFIG~L6HmMktJbvnEGgpmRrsiYvQmIBstSj9UtDM7eNFFqYFO";

const REFERENCE_V2_CHAIN_CONFIG_COMMITMENT: &str =
    "CHAIN_CONFIG~L6HmMktJbvnEGgpmRrsiYvQmIBstSj9UtDM7eNFFqYFO";

const REFERENCE_V3_CHAIN_CONFIG_COMMITMENT: &str =
    "CHAIN_CONFIG~eGc90bEB8zFN4GTo2nForM7pox7r4OiHd2LrtgotiNMO";

const REFERENCE_V99_CHAIN_CONFIG_COMMITMENT: &str =
    "CHAIN_CONFIG~ucfYQZSMbWCUHdtwYMc6vsw-4jDmlu3hi2lGDBxCRpI-";

fn reference_fee_info() -> FeeInfo {
    FeeInfo::new(
        FeeAccount::from_str("0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266").unwrap(),
        0,
    )
}

const REFERENCE_FEE_INFO_COMMITMENT: &str = "FEE_INFO~xCCeTjJClBtwtOUrnAmT65LNTQGceuyjSJHUFfX6VRXR";

async fn reference_header(version: Version) -> Header {
    let builder_key = FeeAccount::generated_from_seed_indexed(Default::default(), 0).1;
    let fee_info = reference_fee_info();
    let payload = reference_payload().await;
    let ns_table = payload.ns_table().clone();
    let payload_commitment =
        vid_commitment::<MockVersions>(&payload.encode(), &ns_table.encode(), 1, version);
    let builder_commitment = payload.builder_commitment(&ns_table);
    let builder_signature =
        FeeAccount::sign_fee(&builder_key, fee_info.amount().as_u64().unwrap(), &ns_table).unwrap();

    let state = ValidatedState::default();

    Header::create(
        reference_chain_config(),
        42,
        789,
        124,
        Some(reference_l1_block()),
        payload_commitment,
        builder_commitment,
        ns_table,
        state.fee_merkle_tree.commitment(),
        state.block_merkle_tree.commitment(),
        Some(state.reward_merkle_tree.commitment()),
        vec![fee_info],
        vec![builder_signature],
        version,
    )
}

const REFERENCE_V1_HEADER_COMMITMENT: &str = "BLOCK~dh1KpdvvxSvnnPpOi2yI3DOg8h6ltr2Kv13iRzbQvtN2";
const REFERENCE_V2_HEADER_COMMITMENT: &str = "BLOCK~V0GJjL19nCrlm9n1zZ6gaOKEekSMCT6uR5P-h7Gi6UJR";
const REFERENCE_V3_HEADER_COMMITMENT: &str = "BLOCK~jcrvSlMuQnR2bK6QtraQ4RhlP_F3-v_vae5Zml0rtPbl";
const REFERENCE_V99_HEADER_COMMITMENT: &str = "BLOCK~3mQrgngtwuDmhzShcxbajQBcY6tuhKYqxcwzCkQDkoIa";

fn reference_transaction<R>(ns_id: NamespaceId, rng: &mut R) -> Transaction
where
    R: RngCore,
{
    let mut tx_payload = vec![0u8; 256];
    rng.fill_bytes(&mut tx_payload);
    Transaction::new(ns_id, tx_payload)
}

const REFERENCE_TRANSACTION_COMMITMENT: &str = "TX~EikfLslj3g6sIWRZYpN6ZuU1gadN77AHXmRA56yNnPrQ";

async fn reference_tx_index() -> <Payload as QueryablePayload<SeqTypes>>::TransactionIndex {
    let payload = reference_payload().await;
    payload.iter(payload.ns_table()).last().unwrap()
}

fn reference_test_without_committable<T: Serialize + DeserializeOwned + Eq + Debug>(
    version: &str,
    name: &str,
    reference: &T,
) {
    setup_test();

    // Load the expected serialization from the repo.
    let data_dir = Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("../data")
        .join(version);

    let file_path = data_dir.join(format!("{name}.json"));

    let expected_bytes =
        std::fs::read(&file_path).unwrap_or_else(|_| panic!("file {} exists", file_path.display()));
    let expected: Value = serde_json::from_slice(&expected_bytes).unwrap();

    // Check that the reference object matches the expected serialized form.
    let actual = serde_json::to_value(reference).unwrap();

    if actual != expected {
        let actual_pretty = serde_json::to_string_pretty(&actual).unwrap();
        let expected_pretty = serde_json::to_string_pretty(&expected).unwrap();

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
Serialized {name} does not match expected JSON. The actual serialization has been written to {}. If
you intended to make a breaking change to the API, you may replace the reference JSON file in
/data/{name}.json with /data/{name}-actual.json. Otherwise, revert your changes which have caused a
change in the serialization of this data structure.
"#,
            actual_path.display()
        );
    }

    // Check that we can deserialize from the reference JSON object.
    let parsed: T = serde_json::from_value(expected).unwrap();
    assert_eq!(
        *reference,
        parsed,
        "Reference object commitment does not match commitment of parsed JSON. This is indicative of
        inconsistency or non-determinism in the commitment scheme.",
    );

    // Check that the reference object matches the expected binary form.
    let file_path = data_dir.join(format!("{name}.bin"));
    let expected =
        std::fs::read(&file_path).unwrap_or_else(|_| panic!("file {} exists", file_path.display()));
    // todo (ab) : cleanup
    let actual = match version {
        "v1" => V1Serializer::serialize(&reference).unwrap(),
        "v2" => V2Serializer::serialize(&reference).unwrap(),
        "v3" => V3Serializer::serialize(&reference).unwrap(),
        "v99" => V99Serializer::serialize(&reference).unwrap(),
        _ => panic!("invalid version"),
    };
    if actual != expected {
        // Write the actual output to a file to make it easier to compare with/replace the expected
        // file if the serialization change was actually intended.
        let actual_path = data_dir.join(format!("{name}-actual.bin"));
        std::fs::write(&actual_path, &actual).unwrap();

        // Fail the test with an assertion that outputs a diff.
        // Use TaggedBase64 for compact console output.
        assert_eq!(
            TaggedBase64::encode_raw(&expected),
            TaggedBase64::encode_raw(&actual),
            r#"
Serialized {name} does not match expected binary file. The actual serialization has been written to
{}. If you intended to make a breaking change to the API, you may replace the reference file in
/data/{name}.bin with /data/{name}-actual.bin. Otherwise, revert your changes which have caused a
change in the serialization of this data structure.
"#,
            actual_path.display()
        );
    }

    // Check that we can deserialize from the reference binary object.
    // todo: (ab) cleanup
    let parsed: T = match version {
        "v1" => V1Serializer::deserialize(&expected).unwrap(),
        "v2" => V2Serializer::deserialize(&expected).unwrap(),
        "v3" => V3Serializer::deserialize(&expected).unwrap(),
        "v99" => V99Serializer::deserialize(&expected).unwrap(),
        _ => panic!("invalid version"),
    };

    assert_eq!(
        *reference, parsed,
        "Reference object commitment does not match commitment of parsed binary object. This is
        indicative of inconsistency or non-determinism in the commitment scheme.",
    );
}

fn reference_test<T: Committable + Serialize + DeserializeOwned + Eq + Debug>(
    version: &str,
    name: &str,
    reference: T,
    commitment: &str,
) {
    setup_test();

    reference_test_without_committable(version, name, &reference);

    // Print information about the commitment that might be useful in generating tests for other
    // languages.
    let actual = reference.commit();
    let bytes: &[u8] = actual.as_ref();
    let u256 = commitment_to_u256(actual);
    tracing::info!("actual commitment: {}", actual);
    tracing::info!("commitment bytes: {:?}", bytes);
    tracing::info!("commitment U256: {}", u256);

    // Check that the reference object matches the expected serialized object.
    let expected = commitment.parse().unwrap();
    assert_eq!(
        actual, expected,
        r#"
Commitment of serialized object does not match commitment of reference object. If you intended to
make a breaking change to the API, you may replace the reference commitment constant in the
reference_tests module with the "actual" commitment below. Otherwise, revert your changes which
have caused a change to the commitment scheme.

Expected: {expected}
Actual: {actual}
"#
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_reference_payload() {
    reference_test_without_committable("v1", "payload", &reference_payload().await);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_reference_tx_index() {
    reference_test_without_committable("v1", "tx_index", &reference_tx_index().await);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_reference_ns_table() {
    reference_test(
        "v1",
        "ns_table",
        reference_ns_table().await,
        REFERENCE_NS_TABLE_COMMITMENT,
    );
}

#[test]
fn test_reference_l1_block() {
    reference_test(
        "v1",
        "l1_block",
        reference_l1_block(),
        REFERENCE_L1_BLOCK_COMMITMENT,
    );
}

#[test]
fn test_reference_v1_chain_config() {
    reference_test(
        "v1",
        "chain_config",
        v0_1::ChainConfig::from(reference_chain_config()),
        REFERENCE_V1_CHAIN_CONFIG_COMMITMENT,
    );
}

#[test]
fn test_reference_v2_chain_config() {
    reference_test(
        "v2",
        "chain_config",
        v0_2::ChainConfig::from(reference_chain_config()),
        REFERENCE_V2_CHAIN_CONFIG_COMMITMENT,
    );
}

#[test]
fn test_reference_v3_chain_config() {
    reference_test(
        "v3",
        "chain_config",
        v0_3::ChainConfig::from(reference_chain_config()),
        REFERENCE_V3_CHAIN_CONFIG_COMMITMENT,
    );
}

#[test]
fn test_reference_v99_chain_config() {
    reference_test(
        "v99",
        "chain_config",
        reference_chain_config(),
        REFERENCE_V99_CHAIN_CONFIG_COMMITMENT,
    );
}

#[test]
fn test_reference_fee_info() {
    reference_test(
        "v1",
        "fee_info",
        reference_fee_info(),
        REFERENCE_FEE_INFO_COMMITMENT,
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_reference_header_v1() {
    reference_test(
        "v1",
        "header",
        reference_header(StaticVersion::<0, 1>::version()).await,
        REFERENCE_V1_HEADER_COMMITMENT,
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_reference_header_v2() {
    reference_test(
        "v2",
        "header",
        reference_header(StaticVersion::<0, 2>::version()).await,
        REFERENCE_V2_HEADER_COMMITMENT,
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_reference_header_v3() {
    reference_test(
        "v3",
        "header",
        reference_header(StaticVersion::<0, 3>::version()).await,
        REFERENCE_V3_HEADER_COMMITMENT,
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_reference_header_v99() {
    reference_test(
        "v99",
        "header",
        reference_header(StaticVersion::<0, 99>::version()).await,
        REFERENCE_V99_HEADER_COMMITMENT,
    );
}
#[test]
fn test_reference_transaction() {
    reference_test(
        "v1",
        "transaction",
        reference_transaction(12648430_u32.into(), &mut jf_utils::test_rng()),
        REFERENCE_TRANSACTION_COMMITMENT,
    );
}

// "legacy" refers to the proof type used before it was an enum.
#[tokio::test(flavor = "multi_thread")]
async fn test_reference_ns_proof_legacy() {
    reference_test_without_committable("v1", "ns_proof_legacy", &reference_ns_proof_legacy().await);
}

// "V0" does not refer to the version scheme used in this crate but to the NSProof::VO variant.
#[tokio::test(flavor = "multi_thread")]
async fn test_reference_ns_proof_enum_advz() {
    reference_test_without_committable("v3", "ns_proof_V0", &reference_ns_proof_enum_advz().await);
}

// "V1" does not refer to the version scheme used in this crate but to the NSProof::V1 variant.
#[tokio::test(flavor = "multi_thread")]
async fn test_reference_ns_proof_enum_avidm() {
    reference_test_without_committable("v3", "ns_proof_V1", &reference_ns_proof_enum_avidm().await);
}
