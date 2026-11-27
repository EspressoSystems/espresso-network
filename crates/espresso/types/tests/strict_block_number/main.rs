//! Encoding of the QCs whose block number becomes required.
//!
//! `QuorumData2::block_number` (also inside `NextEpochQuorumData2`) becomes a required `u64`. The
//! encoding of a block number stays the same: a JSON number, and in bincode the `01` tag of `Some`
//! and the 8 bytes. An input without a block number (`null` in JSON, tag `00` in bincode) no longer
//! decodes. A QC with an epoch but no block number fits neither era, so the history types reject it
//! too.
//!
//! The inputs are QCs and leaves of the recorded `v2-v3` chain in `data/legacy-chains`, and the
//! quorum vote of `data/v6/new_protocol_messages.json`. `tests/strict_epoch` pins their encoding
//! with a block number. `vectors/*_block_number_none.hex` is the bincode of an input with the block
//! number `None`, which today's code wrote. `vectors/leaf_query_data_justify_qc_block_number_none`
//! is a leaf whose justify QC has no block number, with a QC that signs that leaf.
//!
//! Bincode is `bincode::serialize`, which consensus storage writes.

use std::{
    fmt::Debug,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use alloy::hex;
use espresso_types::SeqTypes;
use hotshot_query_service_types::availability::LeafQueryData;
use hotshot_types::{
    data::{
        Leaf2,
        history::{HistoricCertificatePair, HistoricLeaf, HistoricQuorumCertificate},
    },
    simple_certificate::{CertificatePair, NextEpochQuorumCertificate2, QuorumCertificate2},
    simple_vote::QuorumVote2,
};
use pretty_assertions::assert_eq;
use rstest::rstest;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};

/// The chain that upgraded from 0.2 to 0.3.
const UPGRADE_CHAIN: &str = "v2-v3";
/// The first leaf of the upgrade chain with an epoch justify QC. It has no next-epoch QC.
const FIRST_EPOCH_LEAF: usize = 333;
/// A leaf of the upgrade chain with a next-epoch justify QC.
const TRANSITION_LEAF: usize = 398;
/// A `LeafQueryData` whose leaf's justify QC has an epoch but no block number.
const MIXED_LEAF_QUERY_DATA: &str = "leaf_query_data_justify_qc_block_number_none";

#[rstest]
#[case::quorum_certificate2(
    qc(FIRST_EPOCH_LEAF),
    "/data/block_number",
    assert_null_rejected::<QuorumCertificate2<SeqTypes>>
)]
#[case::next_epoch_quorum_certificate2(
    next_epoch_qc(TRANSITION_LEAF),
    "/data/block_number",
    assert_null_rejected::<NextEpochQuorumCertificate2<SeqTypes>>
)]
#[case::leaf2_justify_qc(
    leaf(FIRST_EPOCH_LEAF),
    "/justify_qc/data/block_number",
    assert_null_rejected::<Leaf2<SeqTypes>>
)]
#[case::leaf2_next_epoch_justify_qc(
    leaf(TRANSITION_LEAF),
    "/next_epoch_justify_qc/data/block_number",
    assert_null_rejected::<Leaf2<SeqTypes>>
)]
#[case::certificate_pair_qc(
    pair_for_parent(FIRST_EPOCH_LEAF),
    "/qc/data/block_number",
    assert_null_rejected::<CertificatePair<SeqTypes>>
)]
#[case::certificate_pair_next_epoch_qc(
    pair_for_parent(TRANSITION_LEAF),
    "/next_epoch_qc/data/block_number",
    assert_null_rejected::<CertificatePair<SeqTypes>>
)]
#[case::quorum_vote2(
    v6_message("Vote1")["vote"].clone(),
    "/data/block_number",
    assert_null_rejected::<QuorumVote2<SeqTypes>>
)]
fn live_types_reject_null_block_number(
    #[case] base: Value,
    #[case] field: &str,
    #[case] check: fn(&Value, &str),
) {
    check(&base, field);
}

#[rstest]
#[case::quorum_certificate2(
    qc(FIRST_EPOCH_LEAF),
    "quorum_certificate2_block_number_none",
    assert_none_rejected::<QuorumCertificate2<SeqTypes>>
)]
#[case::next_epoch_quorum_certificate2(
    next_epoch_qc(TRANSITION_LEAF),
    "next_epoch_quorum_certificate2_block_number_none",
    assert_none_rejected::<NextEpochQuorumCertificate2<SeqTypes>>
)]
#[case::leaf2_justify_qc(
    leaf(FIRST_EPOCH_LEAF),
    "leaf2_justify_qc_block_number_none",
    assert_none_rejected::<Leaf2<SeqTypes>>
)]
#[case::leaf2_next_epoch_justify_qc(
    leaf(TRANSITION_LEAF),
    "leaf2_next_epoch_justify_qc_block_number_none",
    assert_none_rejected::<Leaf2<SeqTypes>>
)]
#[case::certificate_pair_qc(
    pair_for_parent(FIRST_EPOCH_LEAF),
    "certificate_pair_qc_block_number_none",
    assert_none_rejected::<CertificatePair<SeqTypes>>
)]
#[case::certificate_pair_next_epoch_qc(
    pair_for_parent(TRANSITION_LEAF),
    "certificate_pair_next_epoch_qc_block_number_none",
    assert_none_rejected::<CertificatePair<SeqTypes>>
)]
#[case::quorum_vote2(
    v6_message("Vote1")["vote"].clone(),
    "quorum_vote2_block_number_none",
    assert_none_rejected::<QuorumVote2<SeqTypes>>
)]
fn live_types_reject_none_block_number_in_bincode(
    #[case] base: Value,
    #[case] none: &str,
    #[case] check: fn(&Value, &str),
) {
    check(&base, none);
}

/// Today such a QC decodes as an epoch QC.
#[rstest]
#[case::historic_qc(
    qc(FIRST_EPOCH_LEAF),
    "/data",
    assert_null_rejected::<HistoricQuorumCertificate<SeqTypes>>
)]
#[case::historic_leaf_justify_qc(
    leaf(FIRST_EPOCH_LEAF),
    "/justify_qc/data",
    assert_null_rejected::<HistoricLeaf<SeqTypes>>
)]
#[case::historic_pair(
    pair_for_parent(FIRST_EPOCH_LEAF),
    "/qc/data",
    assert_null_rejected::<HistoricCertificatePair<SeqTypes>>
)]
#[case::leaf_query_data_qc(
    leaf_query_data(FIRST_EPOCH_LEAF),
    "/qc/data",
    assert_null_rejected::<LeafQueryData<SeqTypes>>
)]
fn historic_types_reject_qc_with_epoch_but_no_block_number(
    #[case] base: Value,
    #[case] data: &str,
    #[case] check: fn(&Value, &str),
) {
    assert_has_epoch_and_block_number(&base, data);
    check(&base, &format!("{data}/block_number"));
}

#[rstest]
#[case::historic_qc(
    qc(FIRST_EPOCH_LEAF),
    "/data",
    "quorum_certificate2_block_number_none",
    assert_none_rejected::<HistoricQuorumCertificate<SeqTypes>>
)]
#[case::historic_leaf_justify_qc(
    leaf(FIRST_EPOCH_LEAF),
    "/justify_qc/data",
    "leaf2_justify_qc_block_number_none",
    assert_none_rejected::<HistoricLeaf<SeqTypes>>
)]
#[case::historic_pair(
    pair_for_parent(FIRST_EPOCH_LEAF),
    "/qc/data",
    "certificate_pair_qc_block_number_none",
    assert_none_rejected::<HistoricCertificatePair<SeqTypes>>
)]
#[case::leaf_query_data_qc(
    leaf_query_data(FIRST_EPOCH_LEAF),
    "/qc/data",
    "leaf_query_data_qc_block_number_none",
    assert_none_rejected::<LeafQueryData<SeqTypes>>
)]
fn historic_types_reject_qc_with_epoch_but_no_block_number_in_bincode(
    #[case] base: Value,
    #[case] data: &str,
    #[case] none: &str,
    #[case] check: fn(&Value, &str),
) {
    assert_has_epoch_and_block_number(&base, data);
    check(&base, none);
}

/// The leaf's justify QC has an epoch but no block number, and the QC signs the leaf with that
/// justify QC. Today the leaf decodes as an epoch leaf.
#[test]
fn leaf_query_data_rejects_justify_qc_with_epoch_but_no_block_number() {
    let json = own_json(MIXED_LEAF_QUERY_DATA);
    let data = &json["leaf"]["justify_qc"]["data"];
    assert!(
        data["epoch"].is_u64() && data["block_number"].is_null(),
        "{data}"
    );

    let decoded = try_from_json::<LeafQueryData<SeqTypes>>(&json);
    assert!(decoded.is_err(), "JSON decoded: {decoded:?}");
}

/// The bincode of the input of `leaf_query_data_rejects_justify_qc_with_epoch_but_no_block_number`.
#[test]
fn leaf_query_data_rejects_justify_qc_with_epoch_but_no_block_number_in_bincode() {
    let decoded = bincode::deserialize::<LeafQueryData<SeqTypes>>(&own_hex(MIXED_LEAF_QUERY_DATA));
    assert!(decoded.is_err(), "bincode decoded: {decoded:?}");
}

/// A recorded chain, as the JSON of `data/legacy-chains/{name}/blocks.json`.
struct Chain(Vec<Value>);

impl Chain {
    fn load(name: &str) -> Self {
        let path = data_dir()
            .join("legacy-chains")
            .join(name)
            .join("blocks.json");
        let Value::Array(items) = read_json(&path) else {
            panic!("{name}: blocks.json is not an array");
        };
        Self(items)
    }

    /// Leaf `i`, which has no block payload.
    fn leaf(&self, i: usize) -> &Value {
        &self.0[i]["leaf"]["leaf"]
    }

    /// The QC that signs leaf `i`.
    fn qc(&self, i: usize) -> &Value {
        &self.0[i]["leaf"]["qc"]
    }

    /// Leaf `i` and its QC, as the query service serves them.
    fn leaf_query_data(&self, i: usize) -> &Value {
        &self.0[i]["leaf"]
    }
}

fn upgrade_chain() -> &'static Chain {
    static CHAIN: LazyLock<Chain> = LazyLock::new(|| Chain::load(UPGRADE_CHAIN));
    &CHAIN
}

fn leaf(i: usize) -> Value {
    upgrade_chain().leaf(i).clone()
}

fn qc(i: usize) -> Value {
    upgrade_chain().qc(i).clone()
}

fn next_epoch_qc(i: usize) -> Value {
    let qc = upgrade_chain().leaf(i)["next_epoch_justify_qc"].clone();
    assert!(qc.is_object(), "leaf {i} has no next-epoch justify QC");
    qc
}

fn leaf_query_data(i: usize) -> Value {
    upgrade_chain().leaf_query_data(i).clone()
}

/// The QCs for the parent of leaf `i`, as `CertificatePair::for_parent` takes them from it.
fn pair_for_parent(i: usize) -> Value {
    let leaf = upgrade_chain().leaf(i);
    json!({
        "qc": leaf["justify_qc"],
        "next_epoch_qc": leaf["next_epoch_justify_qc"],
    })
}

/// The first message of kind `kind` in `data/v6/new_protocol_messages.json`.
fn v6_message(kind: &str) -> Value {
    let messages = read_json(&data_dir().join("v6/new_protocol_messages.json"));
    let Value::Array(messages) = messages else {
        panic!("the messages are not an array");
    };
    messages
        .into_iter()
        .find_map(|message| message["message_type"]["Consensus"].get(kind).cloned())
        .unwrap_or_else(|| panic!("no {kind} message"))
}

/// Checks that the QC data at the JSON pointer `data` has an epoch and a block number, so that
/// setting the block number to `null` makes a QC that fits neither era.
fn assert_has_epoch_and_block_number(json: &Value, data: &str) {
    let data = json.pointer(data).expect("the QC data exists");
    assert!(
        data["epoch"].is_u64() && data["block_number"].is_u64(),
        "{data}"
    );
}

/// Checks that `base` decodes as `T`, but no longer once the block number at the JSON pointer
/// `field` is `null`.
fn assert_null_rejected<T: DeserializeOwned + Debug>(base: &Value, field: &str) {
    // The unchanged input must decode, so the error below comes from the `null` alone.
    from_json::<T>(base);

    let mut input = base.clone();
    let value = input.pointer_mut(field).expect("field exists");
    assert!(value.is_u64(), "{field} is not a number: {value}");
    *value = Value::Null;
    let decoded = try_from_json::<T>(&input);
    assert!(
        decoded.is_err(),
        "decoding succeeded with a null {field}: {decoded:?}"
    );
}

/// Checks that `vectors/{none}.hex`, the bincode of `base` with the block number set to `None`,
/// does not decode as `T`.
fn assert_none_rejected<T: Serialize + DeserializeOwned + Debug>(base: &Value, none: &str) {
    let value: T = from_json(base);
    let none_bytes = own_hex(none);
    assert_only_one_option_is_none(&bincode::serialize(&value).unwrap(), &none_bytes);

    let decoded = bincode::deserialize::<T>(&none_bytes);
    assert!(
        decoded.is_err(),
        "decoding succeeded with a None block number in {none}: {decoded:?}"
    );
}

/// Checks that `none` is `some` with one `Some(u64)` (tag `01` and 8 bytes) written as `None`
/// (tag `00`), so that an error when decoding `none` comes from that field alone.
fn assert_only_one_option_is_none(some: &[u8], none: &[u8]) {
    let at = some.iter().zip(none).position(|(a, b)| a != b);
    let at = at.expect("the encodings differ");
    assert_eq!(
        (some[at], none[at]),
        (1, 0),
        "the first difference is not an Option tag"
    );
    assert_eq!(
        hex::encode(&some[at + 9..]),
        hex::encode(&none[at + 1..]),
        "the encodings differ after the field"
    );
}

fn from_json<T: DeserializeOwned>(json: &Value) -> T {
    try_from_json(json).unwrap_or_else(|err| panic!("JSON does not decode: {err}"))
}

fn try_from_json<T: DeserializeOwned>(json: &Value) -> serde_json::Result<T> {
    serde_json::from_str(&json.to_string())
}

fn own_json(name: &str) -> Value {
    read_json(&vectors_dir().join(format!("{name}.json")))
}

fn own_hex(name: &str) -> Vec<u8> {
    let path = vectors_dir().join(format!("{name}.hex"));
    let text = std::fs::read_to_string(path).expect("hex vector exists");
    hex::decode(text.trim()).expect("hex vector is hex")
}

fn vectors_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/strict_block_number/vectors")
}

fn data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data")
}

fn read_json(path: &Path) -> Value {
    let text = std::fs::read_to_string(path).expect("vector exists");
    serde_json::from_str(&text).expect("vector is JSON")
}
