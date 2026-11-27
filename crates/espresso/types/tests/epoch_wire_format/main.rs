//! Encoding of the wire and storage types whose `epoch` and `target_epoch` become required.
//!
//! The golden tests pin today's JSON and bincode of the types that no `data/` vector pins yet.
//! They must keep passing once the fields are no longer `Option`s. The rejection tests check that
//! an input with a missing epoch (`null` in JSON, tag byte `00` in bincode) no longer decodes.
//!
//! `DaProposal2` is pinned by `data/v*/messages.*`, and `AvidmGf2DisperseShareFragment` by
//! `data/v{6,7}/new_protocol_messages.*`, so their rejection inputs come from those vectors.
//!
//! Bincode is `bincode::serialize`, which consensus storage writes. The network's
//! `versions::encode` writes the same bytes after a version prefix.

use std::{
    fmt::Debug,
    path::{Path, PathBuf},
};

use alloy::hex;
use bincode::Options;
use espresso_types::SeqTypes;
use hotshot_types::{
    data::{
        DaProposal2, QuorumProposal2, QuorumProposal2Legacy, QuorumProposalWrapper,
        QuorumProposalWrapperLegacy, VidDisperseShare,
        vid_disperse::{
            AvidMDisperseShare, AvidmGf2Disperse, AvidmGf2DisperseShare,
            AvidmGf2DisperseShareFragment,
        },
    },
    message::Proposal,
    utils::bincode_opts,
};
use pretty_assertions::assert_eq;
use rstest::rstest;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

type SignedVidShare = Proposal<SeqTypes, VidDisperseShare<SeqTypes>>;
type SignedDaProposal = Proposal<SeqTypes, DaProposal2<SeqTypes>>;
type SignedQuorumProposal = Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>>;
type SignedQuorumProposalLegacy = Proposal<SeqTypes, QuorumProposalWrapperLegacy<SeqTypes>>;

#[test]
fn signed_vid_share_v1_encoding_is_unchanged() {
    assert_golden::<SignedVidShare>("signed_vid_share_v1");
}

#[test]
fn signed_vid_share_v2_encoding_is_unchanged() {
    assert_golden::<SignedVidShare>("signed_vid_share_v2");
}

#[test]
fn avidm_gf2_disperse_encoding_is_unchanged() {
    assert_golden::<AvidmGf2Disperse<SeqTypes>>("avidm_gf2_disperse");
}

#[test]
fn signed_quorum_proposal_encoding_is_unchanged() {
    assert_golden::<SignedQuorumProposal>("signed_quorum_proposal");
}

#[test]
fn signed_quorum_proposal_legacy_encoding_is_unchanged() {
    assert_golden::<SignedQuorumProposalLegacy>("signed_quorum_proposal_legacy");
}

#[rstest]
#[case::epoch("/epoch")]
#[case::target_epoch("/target_epoch")]
fn avidm_disperse_share_rejects_null(#[case] field: &str) {
    let base = &golden_json("signed_vid_share_v1")["data"]["V1"];
    assert_null_rejected::<AvidMDisperseShare<SeqTypes>>(base, field);
}

#[rstest]
#[case::epoch("/epoch")]
#[case::target_epoch("/target_epoch")]
fn avidm_gf2_disperse_rejects_null(#[case] field: &str) {
    assert_null_rejected::<AvidmGf2Disperse<SeqTypes>>(&golden_json("avidm_gf2_disperse"), field);
}

#[rstest]
#[case::epoch("/epoch")]
#[case::target_epoch("/target_epoch")]
fn avidm_gf2_disperse_share_rejects_null(#[case] field: &str) {
    let base = &golden_json("signed_vid_share_v2")["data"]["V2"];
    assert_null_rejected::<AvidmGf2DisperseShare<SeqTypes>>(base, field);
}

#[rstest]
#[case::epoch("/epoch")]
#[case::target_epoch("/target_epoch")]
fn avidm_gf2_disperse_share_fragment_rejects_null(#[case] field: &str) {
    let base = &message_vector_entry("v7/new_protocol_messages.json", "VidShareFragment")["data"];
    assert_null_rejected::<AvidmGf2DisperseShareFragment<SeqTypes>>(base, field);
}

#[rstest]
#[case::v1_epoch("signed_vid_share_v1", "/V1/epoch")]
#[case::v1_target_epoch("signed_vid_share_v1", "/V1/target_epoch")]
#[case::v2_epoch("signed_vid_share_v2", "/V2/epoch")]
#[case::v2_target_epoch("signed_vid_share_v2", "/V2/target_epoch")]
fn vid_disperse_share_rejects_null(#[case] golden: &str, #[case] field: &str) {
    assert_null_rejected::<VidDisperseShare<SeqTypes>>(&golden_json(golden)["data"], field);
}

#[test]
fn da_proposal2_rejects_null_epoch() {
    let base = &message_vector_entry("v5/messages.json", "DaProposal2")["data"];
    assert_null_rejected::<DaProposal2<SeqTypes>>(base, "/epoch");
}

#[test]
fn quorum_proposal2_rejects_null_epoch() {
    let base = &golden_json("signed_quorum_proposal")["data"]["proposal"];
    assert_null_rejected::<QuorumProposal2<SeqTypes>>(base, "/epoch");
}

#[test]
fn quorum_proposal2_legacy_rejects_null_epoch() {
    let base = &golden_json("signed_quorum_proposal_legacy")["data"]["proposal"];
    assert_null_rejected::<QuorumProposal2Legacy<SeqTypes>>(base, "/epoch");
}

#[test]
fn quorum_proposal_wrapper_rejects_null_epoch() {
    let base = &golden_json("signed_quorum_proposal")["data"];
    assert_null_rejected::<QuorumProposalWrapper<SeqTypes>>(base, "/proposal/epoch");
}

#[test]
fn quorum_proposal_wrapper_legacy_rejects_null_epoch() {
    let base = &golden_json("signed_quorum_proposal_legacy")["data"];
    assert_null_rejected::<QuorumProposalWrapperLegacy<SeqTypes>>(base, "/proposal/epoch");
}

#[test]
fn signed_quorum_proposal_rejects_null_epoch() {
    let base = &golden_json("signed_quorum_proposal");
    assert_null_rejected::<SignedQuorumProposal>(base, "/data/proposal/epoch");
}

#[test]
fn signed_quorum_proposal_legacy_rejects_null_epoch() {
    let base = &golden_json("signed_quorum_proposal_legacy");
    assert_null_rejected::<SignedQuorumProposalLegacy>(base, "/data/proposal/epoch");
}

#[test]
fn signed_vid_share_v1_rejects_none_epoch_in_bincode() {
    let base = &golden_json("signed_vid_share_v1");
    assert_none_rejected::<SignedVidShare>(base, "signed_vid_share_v1_epoch_none");
}

#[test]
fn signed_vid_share_v2_rejects_none_target_epoch_in_bincode() {
    let base = &golden_json("signed_vid_share_v2");
    assert_none_rejected::<SignedVidShare>(base, "signed_vid_share_v2_target_epoch_none");
}

#[test]
fn signed_da_proposal2_rejects_none_epoch_in_bincode() {
    let base = &message_vector_entry("v5/messages.json", "DaProposal2");
    assert_none_rejected::<SignedDaProposal>(base, "signed_da_proposal2_epoch_none");
}

#[test]
fn signed_quorum_proposal_rejects_none_epoch_in_bincode() {
    let base = &golden_json("signed_quorum_proposal");
    assert_none_rejected::<SignedQuorumProposal>(base, "signed_quorum_proposal_epoch_none");
}

#[test]
fn signed_quorum_proposal_legacy_rejects_none_epoch_in_bincode() {
    let base = &golden_json("signed_quorum_proposal_legacy");
    let none = "signed_quorum_proposal_legacy_epoch_none";
    assert_none_rejected::<SignedQuorumProposalLegacy>(base, none);
}

/// Decodes `vectors/{name}.json`, and checks that it encodes back to the same JSON and to the
/// bytes of `vectors/{name}.hex`, which decode back to the same value.
fn assert_golden<T>(name: &str)
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    let json = golden_json(name);
    let bytes = read_hex(name);
    let value: T = serde_json::from_str(&json.to_string()).expect("golden JSON decodes");

    assert_eq!(
        serde_json::to_value(&value).unwrap(),
        json,
        "{name}: JSON changed"
    );
    let encoded = bincode::serialize(&value).unwrap();
    assert_eq!(
        hex::encode(encoded),
        hex::encode(&bytes),
        "{name}: bincode changed"
    );
    // `bincode_opts` rejects trailing bytes, so the decoder must read exactly the golden bytes.
    let decoded: T = bincode_opts()
        .deserialize(&bytes)
        .expect("golden bincode decodes");
    assert_eq!(decoded, value, "{name}: bincode decodes to another value");
}

/// Checks that `base` decodes as `T`, but no longer once the epoch at the JSON pointer `field`
/// is `null`.
fn assert_null_rejected<T: DeserializeOwned>(base: &Value, field: &str) {
    // The unchanged input must decode, so the error below comes from the `null` alone.
    serde_json::from_str::<T>(&base.to_string()).expect("input without the null decodes");

    let mut input = base.clone();
    let epoch = input.pointer_mut(field).expect("field exists");
    assert!(epoch.is_u64(), "{field} is not an epoch number: {epoch}");
    *epoch = Value::Null;
    let decoded = serde_json::from_str::<T>(&input.to_string());
    assert!(decoded.is_err(), "decoding succeeded with a null {field}");
}

/// Checks that `vectors/{none}.hex`, the bincode of `base` with one epoch set to `None`, does
/// not decode as `T`.
fn assert_none_rejected<T: Serialize + DeserializeOwned>(base: &Value, none: &str) {
    let value: T = serde_json::from_str(&base.to_string()).expect("base JSON decodes");
    let none_bytes = read_hex(none);
    assert_only_epoch_is_none(&bincode::serialize(&value).unwrap(), &none_bytes);

    let decoded = bincode::deserialize::<T>(&none_bytes);
    assert!(
        decoded.is_err(),
        "decoding succeeded with a None epoch in {none}"
    );
}

/// Checks that `none` is `some` with one `Some(epoch)` (tag `01` and 8 bytes) written as `None`
/// (tag `00`), so that an error when decoding `none` comes from that epoch alone.
fn assert_only_epoch_is_none(some: &[u8], none: &[u8]) {
    let at = some.iter().zip(none).position(|(a, b)| a != b);
    let at = at.expect("the encodings differ");
    assert_eq!(
        (some[at], none[at]),
        (1, 0),
        "the first difference is not an Option tag"
    );
    assert_eq!(
        &some[at + 9..],
        &none[at + 1..],
        "the encodings differ after the epoch"
    );
}

fn golden_json(name: &str) -> Value {
    read_json(&vector_path(&format!("{name}.json")))
}

fn read_hex(name: &str) -> Vec<u8> {
    let text = std::fs::read_to_string(vector_path(&format!("{name}.hex")));
    hex::decode(text.expect("hex vector exists").trim()).expect("hex vector is hex")
}

fn vector_path(file: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/epoch_wire_format/vectors")
        .join(file)
}

/// The value under the first `key` in the committed message vector `data/{file}`.
fn message_vector_entry(file: &str, key: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../data")
        .join(file);
    let json = read_json(&path);
    find_key(&json, key)
        .expect("message vector has the key")
        .clone()
}

fn find_key<'a>(json: &'a Value, key: &str) -> Option<&'a Value> {
    match json {
        Value::Object(map) => map
            .get(key)
            .or_else(|| map.values().find_map(|value| find_key(value, key))),
        Value::Array(items) => items.iter().find_map(|value| find_key(value, key)),
        _ => None,
    }
}

fn read_json(path: &Path) -> Value {
    let text = std::fs::read_to_string(path).expect("vector exists");
    serde_json::from_str(&text).expect("vector is JSON")
}
