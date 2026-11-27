//! Constructors of the pre-epoch history types, and the pre-epoch leaves they must keep reading.
//!
//! `Leaf2` and `QuorumCertificate2` will require an epoch. So the pre-epoch genesis and the
//! conversions from the v1 `Leaf` and `QuorumCertificate` move to `PreEpochLeaf` and
//! `PreEpochQuorumCertificate`. These must build exactly what today's `Leaf2::genesis`,
//! `QuorumCertificate2::genesis`, `Leaf2::from` and `QuorumCertificate::to_qc2` build for a chain
//! before epochs: the same JSON, bincode and commitments.
//!
//! The expected values are recorded data, not today's functions, which will change:
//!
//! * The genesis leaf and QC are those of `data/v3/leaf_query_data.json`, which the reference test
//!   `test_leaf_query_data_v3` builds from the same default states. `vectors/genesis_*.hex` is
//!   their bincode, written by today's `Leaf2::genesis` and `QuorumCertificate2::genesis`.
//! * The v1 inputs are `data/v1/leaf_query_data_legacy.json` and pre-epoch leaves and QCs of the
//!   recorded `v2-v3` chain, without the fields v1 does not have. Today's `Leaf2::from` and
//!   `to_qc2` turn them back into exactly the recorded JSON, the bincode in
//!   `tests/history_types/vectors` and, for the v1 genesis, the genesis vectors above.
//!
//! Pre-epoch leaves can carry view change evidence whose epoch is `null`, which older code wrote.
//! `vectors/*_evidence*` hold such leaves, which today's `Leaf2` wrote from leaves 5 and 332 of
//! the `v2-v3` chain. Leaf 332 sets `with_epoch`, so only its commitment covers the evidence.

use std::{
    fmt::Debug,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use alloy::hex;
use bincode::Options;
use committable::{Commitment, Committable};
use espresso_types::{NodeState, SeqTypes, ValidatedState};
use hotshot_query_service_types::availability::LeafQueryData;
use hotshot_types::{
    data::{
        Leaf, Leaf2, ViewNumber,
        history::{
            HistoricLeaf, HistoricQuorumCertificate, PreEpochLeaf, PreEpochQuorumCertificate,
        },
    },
    message::UpgradeLock,
    simple_certificate::QuorumCertificate,
    simple_vote::VersionedVoteData,
    utils::bincode_opts,
};
use pretty_assertions::assert_eq;
use rstest::rstest;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use versions::{EPOCH_VERSION, FEE_VERSION, NEW_PROTOCOL_VERSION, Upgrade, version};

/// The upgrade of the `data/v1` to `data/v3` genesis vectors, as in `reference_tests.rs`.
const LEGACY_UPGRADE: Upgrade = Upgrade::new(version(0, 1), version(0, 2));
/// A chain at 0.2 that upgrades to epochs. Its genesis is still pre-epoch.
const FEE_TO_EPOCH_UPGRADE: Upgrade = Upgrade::new(FEE_VERSION, EPOCH_VERSION);

/// The chain that upgraded from 0.2 to 0.3. Leaves 0 to 332 have a pre-epoch justify QC.
const UPGRADE_CHAIN: &str = "v2-v3";
/// The last pre-epoch QC of the upgrade chain. Its leaf has an upgrade certificate.
const LAST_PRE_EPOCH_QC: usize = 331;
/// The first leaf of the upgrade chain with `with_epoch` set. Its justify QC is pre-epoch.
const BOUNDARY_LEAF: usize = 332;

#[tokio::test]
async fn pre_epoch_leaf_genesis_encodes_like_the_recorded_genesis_leaf() {
    let leaf = pre_epoch_genesis_leaf().await;
    assert_encodes_as(&leaf, &v3_genesis()["leaf"], &read_hex("genesis_leaf"));
}

#[tokio::test]
async fn pre_epoch_leaf_genesis_commit_is_the_recorded_genesis_commit() {
    let leaf = pre_epoch_genesis_leaf().await;
    assert_eq!(
        leaf.commit(),
        commitment(&v3_genesis()["qc"]["data"]["leaf_commit"])
    );
}

#[tokio::test]
async fn pre_epoch_leaf_genesis_has_the_genesis_fields() {
    let leaf = pre_epoch_genesis_leaf().await;
    assert_eq!(leaf.view_number(), ViewNumber::genesis());
    assert_eq!(leaf.height(), 0);
    assert!(!leaf.with_epoch());
    assert!(leaf.block_payload().is_some());
    assert_eq!(leaf.justify_qc().view_number, ViewNumber::genesis());
}

#[tokio::test]
async fn pre_epoch_qc_genesis_encodes_like_the_recorded_genesis_qc() {
    let qc = pre_epoch_genesis_qc().await;
    assert_encodes_as(&qc, &v3_genesis()["qc"], &read_hex("genesis_qc"));
}

/// The data commitment is what the signers of a pre-epoch QC signed, so it must not change.
#[tokio::test]
async fn pre_epoch_qc_genesis_keeps_its_commitments() {
    let qc = pre_epoch_genesis_qc().await;
    assert_eq!(qc.view_number, ViewNumber::genesis());
    assert_eq!(
        hex::encode(raw(qc.data.commit())),
        golden_commitment("genesis_qc_data")
    );
    let vote = VersionedVoteData::new(qc.data, qc.view_number, &upgrade_lock()).unwrap();
    assert_eq!(raw(qc.vote_commitment()), raw(vote.commit()));
}

#[tokio::test]
async fn pre_epoch_qc_genesis_signs_the_pre_epoch_genesis_leaf() {
    let qc = pre_epoch_genesis_qc().await;
    let leaf = pre_epoch_genesis_leaf().await;
    assert_eq!(qc.data.leaf_commit, leaf.commit());
}

#[rstest]
#[case::genesis(V1Case::Genesis)]
#[case::first(V1Case::Chain(1))]
#[case::with_payload(V1Case::ChainWithPayload(5))]
#[case::with_upgrade_certificate(V1Case::Chain(LAST_PRE_EPOCH_QC))]
fn pre_epoch_leaf_from_v1_leaf_encodes_like_today_leaf2_from(#[case] case: V1Case) {
    let leaf = PreEpochLeaf::from(case.v1_leaf());
    assert_encodes_as(&leaf, &case.expected_leaf(), &case.expected_leaf_bincode());
}

#[rstest]
#[case::genesis(V1Case::Genesis)]
#[case::first(V1Case::Chain(1))]
#[case::with_payload(V1Case::ChainWithPayload(5))]
#[case::with_upgrade_certificate(V1Case::Chain(LAST_PRE_EPOCH_QC))]
fn pre_epoch_leaf_from_v1_leaf_keeps_the_commitment(#[case] case: V1Case) {
    let leaf = PreEpochLeaf::from(case.v1_leaf());
    assert_eq!(leaf.commit(), case.expected_leaf_commit());
}

#[rstest]
#[case::genesis(V1Case::Genesis)]
#[case::first(V1Case::Chain(1))]
#[case::with_payload(V1Case::ChainWithPayload(5))]
#[case::with_upgrade_certificate(V1Case::Chain(LAST_PRE_EPOCH_QC))]
fn pre_epoch_leaf_from_v1_leaf_keeps_the_fields(#[case] case: V1Case) {
    let v1 = case.v1_leaf();
    let leaf = PreEpochLeaf::from(v1.clone());

    assert_eq!(leaf.view_number(), v1.view_number());
    assert_eq!(leaf.height(), v1.height());
    assert_eq!(leaf.block_header(), v1.block_header());
    assert_eq!(raw(leaf.parent_commitment()), raw(v1.parent_commitment()));
    assert_eq!(
        leaf.upgrade_certificate(),
        v1.upgrade_certificate().as_ref()
    );
    assert_eq!(leaf.block_payload(), v1.block_payload());
    assert_eq!(leaf.payload_commitment(), v1.payload_commitment());
    assert_eq!(leaf.justify_qc().view_number, v1.justify_qc().view_number);
    assert_eq!(
        raw(leaf.justify_qc().data.leaf_commit),
        raw(v1.justify_qc().data.leaf_commit)
    );
    assert!(!leaf.with_epoch());
    assert!(leaf.next_epoch_justify_qc().is_none());
    assert!(leaf.view_change_evidence().is_none());
    assert!(leaf.next_drb_result().is_none());
}

#[rstest]
#[case::genesis(V1Case::Genesis)]
#[case::first(V1Case::Chain(1))]
#[case::last_pre_epoch(V1Case::Chain(LAST_PRE_EPOCH_QC))]
fn pre_epoch_qc_from_v1_qc_encodes_like_today_to_qc2(#[case] case: V1Case) {
    let qc = PreEpochQuorumCertificate::from(case.v1_qc());
    assert_encodes_as(&qc, &case.expected_qc(), &case.expected_qc_bincode());
}

/// The signatures over a v1 QC still verify only if the commitments stay the same.
#[rstest]
#[case::genesis(V1Case::Genesis)]
#[case::first(V1Case::Chain(1))]
#[case::last_pre_epoch(V1Case::Chain(LAST_PRE_EPOCH_QC))]
fn pre_epoch_qc_from_v1_qc_keeps_the_fields(#[case] case: V1Case) {
    let v1 = case.v1_qc();
    let qc = PreEpochQuorumCertificate::from(v1.clone());

    assert_eq!(raw(qc.data.leaf_commit), raw(v1.data.leaf_commit));
    assert_eq!(qc.view_number, v1.view_number);
    assert_eq!(qc.signatures, v1.signatures);
    assert_eq!(raw(qc.vote_commitment()), raw(v1.vote_commitment()));
    assert_eq!(raw(qc.data.commit()), raw(v1.data.commit()));
}

/// Base 0.2 builds the same genesis as base 0.1: the header does not depend on the version.
#[rstest]
#[case::v0_1(LEGACY_UPGRADE)]
#[case::v0_2(FEE_TO_EPOCH_UPGRADE)]
#[tokio::test]
async fn leaf_query_data_genesis_before_epochs_is_pre_epoch(#[case] upgrade: Upgrade) {
    let genesis =
        LeafQueryData::<SeqTypes>::genesis(&validated_state(), &instance_state(), upgrade).await;
    assert!(
        matches!(genesis.leaf(), HistoricLeaf::PreEpoch(_)),
        "leaf: {:?}",
        genesis.leaf()
    );
    assert!(
        matches!(genesis.qc(), HistoricQuorumCertificate::PreEpoch(_)),
        "QC: {:?}",
        genesis.qc()
    );
    assert_eq!(serde_json::to_value(&genesis).unwrap(), v3_genesis());
    let bytes = std::fs::read(data_dir().join("v3/leaf_query_data.bin")).unwrap();
    assert_eq!(
        hex::encode(versions::encode(version(0, 3), &genesis).unwrap()),
        hex::encode(bytes)
    );

    assert_eq!(
        genesis.leaf(),
        &HistoricLeaf::PreEpoch(pre_epoch_genesis_leaf().await)
    );
    assert_eq!(
        genesis.qc(),
        &HistoricQuorumCertificate::PreEpoch(pre_epoch_genesis_qc().await)
    );
}

#[rstest]
#[case::v0_3(Upgrade::trivial(EPOCH_VERSION))]
#[case::v0_6(Upgrade::trivial(NEW_PROTOCOL_VERSION))]
#[tokio::test]
async fn leaf_query_data_genesis_with_epochs_is_epoch(#[case] upgrade: Upgrade) {
    let genesis =
        LeafQueryData::<SeqTypes>::genesis(&validated_state(), &instance_state(), upgrade).await;
    assert!(
        matches!(genesis.leaf(), HistoricLeaf::Epoch(_)),
        "leaf: {:?}",
        genesis.leaf()
    );
    assert!(
        matches!(genesis.qc(), HistoricQuorumCertificate::Epoch(_)),
        "QC: {:?}",
        genesis.qc()
    );
    let name = "epoch_genesis_leaf_query_data";
    assert_encodes_as(
        &genesis,
        &read_json(&vector_path(name, "json")),
        &read_hex(name),
    );
}

#[rstest]
#[case::timeout(Evidence::Timeout, "pre_epoch_leaf")]
#[case::view_sync(Evidence::ViewSync, "pre_epoch_leaf")]
#[case::boundary_timeout(Evidence::Timeout, "boundary_leaf")]
#[case::boundary_view_sync(Evidence::ViewSync, "boundary_leaf")]
fn pre_epoch_leaf_reads_evidence_without_epoch(#[case] evidence: Evidence, #[case] leaf: &str) {
    let name = evidence.vector(leaf);
    let (json, bytes) = evidence_leaf_vectors(&name, "", evidence);

    let leaf: PreEpochLeaf<SeqTypes> = assert_round_trips(&json, &bytes, &name);
    assert_eq!(hex::encode(raw(leaf.commit())), golden_commitment(&name));
}

#[rstest]
#[case::timeout(Evidence::Timeout, "pre_epoch_leaf")]
#[case::view_sync(Evidence::ViewSync, "pre_epoch_leaf")]
#[case::boundary_timeout(Evidence::Timeout, "boundary_leaf")]
#[case::boundary_view_sync(Evidence::ViewSync, "boundary_leaf")]
fn historic_leaf_reads_evidence_without_epoch(#[case] evidence: Evidence, #[case] leaf: &str) {
    let name = evidence.vector(leaf);
    let (json, bytes) = evidence_leaf_vectors(&name, "", evidence);

    let leaf: HistoricLeaf<SeqTypes> = assert_round_trips(&json, &bytes, &name);
    assert!(
        matches!(leaf, HistoricLeaf::PreEpoch(_)),
        "{name}: {leaf:?}"
    );
    assert_eq!(hex::encode(raw(leaf.commit())), golden_commitment(&name));
}

#[rstest]
#[case::timeout(Evidence::Timeout, "pre_epoch_leaf")]
#[case::view_sync(Evidence::ViewSync, "pre_epoch_leaf")]
#[case::boundary_timeout(Evidence::Timeout, "boundary_leaf")]
#[case::boundary_view_sync(Evidence::ViewSync, "boundary_leaf")]
fn leaf_query_data_reads_evidence_without_epoch(#[case] evidence: Evidence, #[case] leaf: &str) {
    let name = evidence.vector(leaf);
    let vector = format!("{name}_leaf_query_data");
    let (json, bytes) = evidence_leaf_vectors(&vector, "/leaf", evidence);

    let data: LeafQueryData<SeqTypes> = assert_round_trips(&json, &bytes, &vector);
    assert!(
        matches!(data.leaf(), HistoricLeaf::PreEpoch(_)),
        "{vector}: {:?}",
        data.leaf()
    );
    assert_eq!(hex::encode(raw(data.hash())), golden_commitment(&name));
}

/// A v1 leaf and its QC, and what today's `Leaf2::from` and `to_qc2` make of them.
#[derive(Clone, Copy, Debug)]
enum V1Case {
    /// The genesis of `data/v1/leaf_query_data_legacy.json`.
    Genesis,
    /// Leaf `i` of the upgrade chain and the QC on it.
    Chain(usize),
    /// Leaf `i` of the upgrade chain with its block payload.
    ChainWithPayload(usize),
}

impl V1Case {
    fn v1_leaf(self) -> Leaf<SeqTypes> {
        let json = match self {
            Self::Genesis => v1_genesis()["leaf"].clone(),
            Self::Chain(i) => v1_leaf_json(chain().leaf(i)),
            Self::ChainWithPayload(i) => v1_leaf_json(&chain().leaf_with_payload(i)),
        };
        from_json(&json)
    }

    fn v1_qc(self) -> QuorumCertificate<SeqTypes> {
        let json = match self {
            Self::Genesis => v1_genesis()["qc"].clone(),
            Self::Chain(i) | Self::ChainWithPayload(i) => v1_qc_json(chain().qc(i)),
        };
        from_json(&json)
    }

    fn expected_leaf(self) -> Value {
        match self {
            Self::Genesis => v3_genesis()["leaf"].clone(),
            Self::Chain(i) => chain().leaf(i).clone(),
            Self::ChainWithPayload(i) => chain().leaf_with_payload(i),
        }
    }

    fn expected_leaf_bincode(self) -> Vec<u8> {
        match self {
            Self::Genesis => read_hex("genesis_leaf"),
            Self::Chain(i) => read_history_types_hex(&format!("leaf_{i}")),
            Self::ChainWithPayload(i) => read_history_types_hex(&format!("leaf_{i}_with_payload")),
        }
    }

    /// The commitment the recorded QC on the expected leaf signs.
    fn expected_leaf_commit(self) -> Commitment<Leaf2<SeqTypes>> {
        commitment(&self.expected_qc()["data"]["leaf_commit"])
    }

    /// The recorded QC: the QC on the expected leaf.
    fn expected_qc(self) -> Value {
        match self {
            Self::Genesis => v3_genesis()["qc"].clone(),
            Self::Chain(i) | Self::ChainWithPayload(i) => chain().qc(i).clone(),
        }
    }

    fn expected_qc_bincode(self) -> Vec<u8> {
        match self {
            Self::Genesis => read_hex("genesis_qc"),
            Self::Chain(i) | Self::ChainWithPayload(i) => {
                read_history_types_hex(&format!("qc_{i}"))
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Evidence {
    Timeout,
    ViewSync,
}

impl Evidence {
    fn vector(self, leaf: &str) -> String {
        match self {
            Self::Timeout => format!("{leaf}_timeout_evidence"),
            Self::ViewSync => format!("{leaf}_view_sync_evidence"),
        }
    }

    /// The JSON pointer of the evidence's epoch, under the leaf.
    fn epoch_pointer(self) -> &'static str {
        match self {
            Self::Timeout => "/view_change_evidence/Timeout/data/epoch",
            Self::ViewSync => "/view_change_evidence/ViewSync/data/epoch",
        }
    }
}

/// The JSON and bincode of `vectors/{name}`, after checking that the evidence of the leaf at the
/// JSON pointer `leaf` has a `null` epoch, and that the leaf's justify QC is pre-epoch.
fn evidence_leaf_vectors(name: &str, leaf: &str, evidence: Evidence) -> (Value, Vec<u8>) {
    let json = read_json(&vector_path(name, "json"));
    let epoch = json.pointer(&format!("{leaf}{}", evidence.epoch_pointer()));
    assert_eq!(epoch, Some(&Value::Null), "{name}: evidence epoch");
    let justify = &json.pointer(leaf).unwrap()["justify_qc"]["data"];
    assert_eq!(
        (&justify["epoch"], &justify["block_number"]),
        (&Value::Null, &Value::Null),
        "{name}: justify QC"
    );
    (json, read_hex(name))
}

/// `leaf2`, the JSON of a pre-epoch `Leaf2`, as the JSON of the v1 `Leaf` that today's
/// `Leaf2::from` turns into it.
fn v1_leaf_json(leaf2: &Value) -> Value {
    let mut leaf = leaf2.clone();
    let fields = leaf.as_object_mut().unwrap();
    for field in [
        "next_epoch_justify_qc",
        "view_change_evidence",
        "next_drb_result",
    ] {
        assert_eq!(fields.remove(field), Some(Value::Null), "{field}");
    }
    assert_eq!(fields.remove("with_epoch"), Some(json!(false)));
    leaf["justify_qc"] = v1_qc_json(&leaf2["justify_qc"]);
    leaf
}

/// `qc2`, the JSON of a pre-epoch `QuorumCertificate2`, as the JSON of the v1
/// `QuorumCertificate` that today's `to_qc2` turns into it.
fn v1_qc_json(qc2: &Value) -> Value {
    let mut qc = qc2.clone();
    let data = qc["data"].as_object_mut().unwrap();
    for field in ["epoch", "block_number"] {
        assert_eq!(data.remove(field), Some(Value::Null), "{field}");
    }
    qc
}

async fn pre_epoch_genesis_leaf() -> PreEpochLeaf<SeqTypes> {
    PreEpochLeaf::genesis(&validated_state(), &instance_state(), LEGACY_UPGRADE.base).await
}

async fn pre_epoch_genesis_qc() -> PreEpochQuorumCertificate<SeqTypes> {
    PreEpochQuorumCertificate::genesis(&validated_state(), &instance_state(), LEGACY_UPGRADE.base)
        .await
}

/// The states of the reference test `test_leaf_query_data_v3`.
fn validated_state() -> ValidatedState {
    ValidatedState::default()
}

fn instance_state() -> NodeState {
    NodeState::default()
}

/// The recorded `v2-v3` chain, as the JSON of `data/legacy-chains/v2-v3/blocks.json`.
struct Chain(Vec<Value>);

fn chain() -> &'static Chain {
    static CHAIN: LazyLock<Chain> = LazyLock::new(Chain::load);
    &CHAIN
}

impl Chain {
    fn load() -> Self {
        let path = data_dir()
            .join("legacy-chains")
            .join(UPGRADE_CHAIN)
            .join("blocks.json");
        let Value::Array(items) = read_json(&path) else {
            panic!("blocks.json is not an array");
        };
        assert!(items.len() > BOUNDARY_LEAF);
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

    /// Leaf `i` with its block payload.
    fn leaf_with_payload(&self, i: usize) -> Value {
        let mut leaf = self.leaf(i).clone();
        leaf["block_payload"] = self.0[i]["block"]["payload"].clone();
        leaf
    }
}

/// Checks that `value` encodes to `json` and `bytes`, and that both decode to an equal value.
fn assert_encodes_as<T>(value: &T, json: &Value, bytes: &[u8])
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    assert_eq!(&serde_json::to_value(value).unwrap(), json, "JSON differs");
    assert_eq!(
        hex::encode(bincode::serialize(value).unwrap()),
        hex::encode(bytes),
        "bincode differs"
    );
    assert_eq!(
        &from_json::<T>(json),
        value,
        "JSON decodes to another value"
    );
    let decoded: T = bincode_opts().deserialize(bytes).expect("bincode decodes");
    assert_eq!(&decoded, value, "bincode decodes to another value");
}

/// Decodes `json` and `bytes` as `T`, checks that they give equal values which encode back to
/// the same JSON and bytes, and returns the value.
fn assert_round_trips<T>(json: &Value, bytes: &[u8], name: &str) -> T
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    let from_json: T = try_from_json(json).unwrap_or_else(|err| panic!("{name}: JSON: {err}"));
    assert_eq!(
        &serde_json::to_value(&from_json).unwrap(),
        json,
        "{name}: JSON changed"
    );
    // `bincode_opts` rejects trailing bytes, so the decoder must read exactly the vector.
    let from_bincode: T = bincode_opts()
        .deserialize(bytes)
        .unwrap_or_else(|err| panic!("{name}: bincode: {err}"));
    assert_eq!(
        hex::encode(bincode::serialize(&from_bincode).unwrap()),
        hex::encode(bytes),
        "{name}: bincode changed"
    );
    assert_eq!(from_json, from_bincode, "{name}: JSON and bincode differ");
    from_bincode
}

/// Neither the vote commitment nor the encoding depends on the version.
fn upgrade_lock() -> UpgradeLock<SeqTypes> {
    UpgradeLock::new(Upgrade::trivial(FEE_VERSION))
}

fn raw<T: Committable>(commitment: Commitment<T>) -> [u8; 32] {
    commitment.into()
}

fn commitment(json: &Value) -> Commitment<Leaf2<SeqTypes>> {
    from_json(json)
}

/// A commitment from `vectors/commitments.json`, which today's code wrote, as hex.
fn golden_commitment(name: &str) -> String {
    let commitments = read_json(&vector_path("commitments", "json"));
    commitments[name]
        .as_str()
        .unwrap_or_else(|| panic!("no commitment {name}"))
        .to_string()
}

fn v1_genesis() -> Value {
    read_json(&data_dir().join("v1/leaf_query_data_legacy.json"))
}

fn v3_genesis() -> Value {
    read_json(&data_dir().join("v3/leaf_query_data.json"))
}

fn from_json<T: DeserializeOwned>(json: &Value) -> T {
    try_from_json(json).unwrap_or_else(|err| panic!("JSON does not decode: {err}"))
}

fn try_from_json<T: DeserializeOwned>(json: &Value) -> serde_json::Result<T> {
    serde_json::from_str(&json.to_string())
}

fn vector_path(name: &str, extension: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/pre_epoch_constructors/vectors")
        .join(format!("{name}.{extension}"))
}

fn read_hex(name: &str) -> Vec<u8> {
    read_hex_file(&vector_path(name, "hex"))
}

/// A bincode vector of `tests/history_types`, which pins the recorded `v2-v3` chain.
fn read_history_types_hex(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/history_types/vectors")
        .join(format!("{name}.hex"));
    read_hex_file(&path)
}

fn read_hex_file(path: &Path) -> Vec<u8> {
    let text = std::fs::read_to_string(path).expect("hex vector exists");
    hex::decode(text.trim()).expect("hex vector is hex")
}

fn data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data")
}

fn read_json(path: &Path) -> Value {
    let text = std::fs::read_to_string(path).expect("vector exists");
    serde_json::from_str(&text).expect("vector is JSON")
}
