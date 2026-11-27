//! Encoding of the QCs, leaves, votes and view change certificates whose epoch becomes required.
//!
//! `QuorumData2::epoch` (also inside `NextEpochQuorumData2`), `TimeoutData2::epoch` and
//! `ViewSyncFinalizeData2::epoch` become a required `EpochNumber`. The encoding of an epoch stays
//! the same: a JSON number, and in bincode the `01` tag of `Some` and the 8 bytes. An input
//! without an epoch (`null` in JSON, tag `00` in bincode) no longer decodes.
//!
//! A QC with no epoch but a block number fits neither era, so the history types reject it too. So
//! does a pair with a pre-epoch QC and a next-epoch QC. And the genesis built by `Leaf2::genesis`
//! and `QuorumCertificate2::genesis` is always the epoch genesis, whatever the version.
//!
//! Inputs and pins:
//!
//! * Leaves, QCs and pairs with epochs come from the recorded chains in `data/legacy-chains`. Their
//!   bincode is pinned by `tests/history_types/vectors`, which these tests read as well.
//! * Votes and certificates of the new protocol come from `data/v6/new_protocol_messages.json`,
//!   whose JSON and bincode the reference tests of `espresso-node` pin.
//! * No recording has view sync certificates or votes, or leaves with view change evidence. Today's
//!   code wrote those in `vectors/`, from leaf 5 of the `v3` chain, with commitments in
//!   `vectors/commitments.json`.
//! * `vectors/*_epoch_none.hex` is the bincode of an input with one epoch `None`, which today's code
//!   wrote. The other inputs without an epoch are edited from the JSON in the tests.
//!
//! Bincode is `bincode::serialize`, which consensus storage writes.

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
        Leaf2,
        history::{HistoricCertificatePair, HistoricLeaf, HistoricQuorumCertificate},
    },
    message::UpgradeLock,
    simple_certificate::{
        CertificatePair, NextEpochQuorumCertificate2, QuorumCertificate2, TimeoutCertificate2,
        ViewSyncFinalizeCertificate2,
    },
    simple_vote::{
        QuorumVote2, TimeoutData2, TimeoutVote2, VersionedVoteData, ViewSyncFinalizeData2,
        ViewSyncFinalizeVote2,
    },
    utils::bincode_opts,
};
use pretty_assertions::assert_eq;
use rstest::rstest;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use versions::{FEE_VERSION, NEW_PROTOCOL_VERSION, Upgrade, VERSION_0_1, Version, version};

/// The upgrade of the pre-epoch genesis vectors in `data/v1` to `data/v3`.
const LEGACY_UPGRADE: Upgrade = Upgrade::new(version(0, 1), version(0, 2));

/// The chain that upgraded from 0.2 to 0.3.
const UPGRADE_CHAIN: &str = "v2-v3";
/// A leaf of the upgrade chain with a pre-epoch justify QC.
const PRE_EPOCH_LEAF: usize = 5;
/// The first leaf of the upgrade chain with an epoch justify QC. It has no next-epoch QC.
const FIRST_EPOCH_LEAF: usize = 333;
/// A leaf of the upgrade chain with a next-epoch justify QC.
const TRANSITION_LEAF: usize = 398;

/// A `LeafQueryData` whose leaf's justify QC has a block number but no epoch.
const MIXED_LEAF_QUERY_DATA: &str = "leaf_query_data_justify_qc_epoch_none";

type Vsc2 = ViewSyncFinalizeCertificate2<SeqTypes>;
type Vsv2 = ViewSyncFinalizeVote2<SeqTypes>;

#[rstest]
#[case::leaf2(leaf(FIRST_EPOCH_LEAF), history_hex("leaf_333"), assert_golden::<Leaf2<SeqTypes>>)]
#[case::leaf2_with_next_epoch_justify_qc(
    leaf(TRANSITION_LEAF),
    history_hex("leaf_398"),
    assert_golden::<Leaf2<SeqTypes>>
)]
#[case::leaf2_with_timeout_evidence(
    own_json("leaf2_timeout_evidence"),
    own_hex("leaf2_timeout_evidence"),
    assert_golden::<Leaf2<SeqTypes>>
)]
#[case::leaf2_with_view_sync_evidence(
    own_json("leaf2_view_sync_evidence"),
    own_hex("leaf2_view_sync_evidence"),
    assert_golden::<Leaf2<SeqTypes>>
)]
#[case::leaf2_genesis(
    own_json("leaf2_genesis"),
    own_hex("leaf2_genesis"),
    assert_golden::<Leaf2<SeqTypes>>
)]
#[case::quorum_certificate2(
    qc(FIRST_EPOCH_LEAF),
    history_hex("qc_333"),
    assert_golden::<QuorumCertificate2<SeqTypes>>
)]
#[case::quorum_certificate2_of_transition_leaf(
    qc(TRANSITION_LEAF),
    history_hex("qc_398"),
    assert_golden::<QuorumCertificate2<SeqTypes>>
)]
#[case::quorum_certificate2_genesis(
    own_json("quorum_certificate2_genesis"),
    own_hex("quorum_certificate2_genesis"),
    assert_golden::<QuorumCertificate2<SeqTypes>>
)]
#[case::next_epoch_quorum_certificate2(
    next_epoch_qc(TRANSITION_LEAF),
    own_hex("next_epoch_quorum_certificate2"),
    assert_golden::<NextEpochQuorumCertificate2<SeqTypes>>
)]
#[case::certificate_pair(
    pair_for_parent(FIRST_EPOCH_LEAF),
    history_hex("pair_for_parent_of_333"),
    assert_golden::<CertificatePair<SeqTypes>>
)]
#[case::certificate_pair_with_next_epoch_qc(
    pair_for_parent(TRANSITION_LEAF),
    history_hex("pair_for_parent_of_398"),
    assert_golden::<CertificatePair<SeqTypes>>
)]
#[case::view_sync_finalize_certificate2(
    own_json("view_sync_finalize_certificate2"),
    own_hex("view_sync_finalize_certificate2"),
    assert_golden::<Vsc2>
)]
#[case::view_sync_finalize_vote2(
    own_json("view_sync_finalize_vote2"),
    own_hex("view_sync_finalize_vote2"),
    assert_golden::<Vsv2>
)]
fn epoch_encoding_is_unchanged(
    #[case] json: Value,
    #[case] bytes: Vec<u8>,
    #[case] check: fn(&Value, &[u8]),
) {
    check(&json, &bytes);
}

/// The bincode of these is pinned inside the messages of `data/v6/new_protocol_messages.bin`.
#[rstest]
#[case::quorum_certificate2(v6_message("HighQc"), assert_json_golden::<QuorumCertificate2<SeqTypes>>)]
#[case::timeout_certificate2(
    v6_message("TimeoutCertificate"),
    assert_json_golden::<TimeoutCertificate2<SeqTypes>>
)]
#[case::quorum_vote2(v6_message("Vote1")["vote"].clone(), assert_json_golden::<QuorumVote2<SeqTypes>>)]
#[case::timeout_vote2(
    v6_message("TimeoutVote")["vote"].clone(),
    assert_json_golden::<TimeoutVote2<SeqTypes>>
)]
fn epoch_json_of_v6_messages_is_unchanged(#[case] json: Value, #[case] check: fn(&Value)) {
    check(&json);
}

/// A leaf's QC signs the leaf's commitment, and the next leaf holds it as its parent. The leaf
/// commitment covers the commitments of its justify QCs.
#[rstest]
#[case::upgrade_chain(UPGRADE_CHAIN, FIRST_EPOCH_LEAF)]
#[case::v3("v3", 0)]
#[case::v4("v4", 0)]
#[case::v5("v5", 0)]
fn leaf2_commit_matches_the_recorded_chain(#[case] name: &str, #[case] first: usize) {
    let chain = Chain::load(name);
    for i in first..chain.len() {
        let leaf: Leaf2<SeqTypes> = from_json(chain.leaf(i));
        assert_eq!(
            leaf.commit(),
            commitment(&chain.qc(i)["data"]["leaf_commit"]),
            "{name}: leaf {i}"
        );
        if i + 1 < chain.len() {
            let parent = commitment(&chain.leaf(i + 1)["parent_commitment"]);
            assert_eq!(leaf.commit(), parent, "{name}: parent of leaf {}", i + 1);
        }
    }
}

/// A signed QC stores the commitment of the vote its signers signed, which covers the commitment
/// of the data, with its epoch and block number.
#[rstest]
#[case::upgrade_chain(UPGRADE_CHAIN, FIRST_EPOCH_LEAF - 1)]
#[case::v3("v3", 0)]
#[case::v4("v4", 0)]
#[case::v5("v5", 0)]
fn quorum_data2_commit_matches_the_recorded_vote_commitments(
    #[case] name: &str,
    #[case] first: usize,
) {
    let chain = Chain::load(name);
    let upgrade_lock = upgrade_lock();
    for i in first..chain.len() {
        let qc: QuorumCertificate2<SeqTypes> = from_json(chain.qc(i));
        let vote = VersionedVoteData::new(qc.data, qc.view_number, &upgrade_lock).unwrap();
        assert_eq!(
            raw(vote.commit()),
            raw(qc.vote_commitment()),
            "{name}: QC {i}"
        );

        let leaf: HistoricLeaf<SeqTypes> = from_json(chain.leaf(i));
        if let Some(qc) = leaf.as_leaf2().and_then(Leaf2::next_epoch_justify_qc) {
            let vote =
                VersionedVoteData::new(qc.data.clone(), qc.view_number, &upgrade_lock).unwrap();
            assert_eq!(
                raw(vote.commit()),
                raw(qc.vote_commitment()),
                "{name}: next-epoch justify QC of leaf {i}"
            );
        }
    }
}

#[rstest]
#[case::leaf2_with_timeout_evidence(
    own_json("leaf2_timeout_evidence"),
    "leaf2_timeout_evidence",
    commit_of::<Leaf2<SeqTypes>>
)]
#[case::leaf2_with_view_sync_evidence(
    own_json("leaf2_view_sync_evidence"),
    "leaf2_view_sync_evidence",
    commit_of::<Leaf2<SeqTypes>>
)]
#[case::leaf2_genesis(own_json("leaf2_genesis"), "leaf2_genesis", commit_of::<Leaf2<SeqTypes>>)]
#[case::view_sync_finalize_certificate2(
    own_json("view_sync_finalize_certificate2"),
    "view_sync_finalize_certificate2",
    commit_of::<Vsc2>
)]
#[case::view_sync_finalize_data2(
    own_json("view_sync_finalize_certificate2")["data"].clone(),
    "view_sync_finalize_data2",
    commit_of::<ViewSyncFinalizeData2>
)]
#[case::timeout_certificate2(
    v6_message("TimeoutCertificate"),
    "v6_timeout_certificate2",
    commit_of::<TimeoutCertificate2<SeqTypes>>
)]
#[case::timeout_data2(
    v6_message("TimeoutCertificate")["data"].clone(),
    "v6_timeout_data2",
    commit_of::<TimeoutData2>
)]
fn epoch_commitment_is_unchanged(
    #[case] json: Value,
    #[case] name: &str,
    #[case] commit: fn(&Value) -> [u8; 32],
) {
    assert_eq!(hex::encode(commit(&json)), golden_commitment(name));
}

#[rstest]
#[case::quorum_certificate2(
    qc(FIRST_EPOCH_LEAF),
    "/data/epoch",
    assert_null_rejected::<QuorumCertificate2<SeqTypes>>
)]
#[case::next_epoch_quorum_certificate2(
    next_epoch_qc(TRANSITION_LEAF),
    "/data/epoch",
    assert_null_rejected::<NextEpochQuorumCertificate2<SeqTypes>>
)]
#[case::leaf2_justify_qc(
    leaf(FIRST_EPOCH_LEAF),
    "/justify_qc/data/epoch",
    assert_null_rejected::<Leaf2<SeqTypes>>
)]
#[case::leaf2_next_epoch_justify_qc(
    leaf(TRANSITION_LEAF),
    "/next_epoch_justify_qc/data/epoch",
    assert_null_rejected::<Leaf2<SeqTypes>>
)]
#[case::leaf2_timeout_evidence(
    own_json("leaf2_timeout_evidence"),
    "/view_change_evidence/Timeout/data/epoch",
    assert_null_rejected::<Leaf2<SeqTypes>>
)]
#[case::leaf2_view_sync_evidence(
    own_json("leaf2_view_sync_evidence"),
    "/view_change_evidence/ViewSync/data/epoch",
    assert_null_rejected::<Leaf2<SeqTypes>>
)]
#[case::certificate_pair_qc(
    pair_for_parent(FIRST_EPOCH_LEAF),
    "/qc/data/epoch",
    assert_null_rejected::<CertificatePair<SeqTypes>>
)]
#[case::certificate_pair_next_epoch_qc(
    pair_for_parent(TRANSITION_LEAF),
    "/next_epoch_qc/data/epoch",
    assert_null_rejected::<CertificatePair<SeqTypes>>
)]
#[case::timeout_certificate2(
    v6_message("TimeoutCertificate"),
    "/data/epoch",
    assert_null_rejected::<TimeoutCertificate2<SeqTypes>>
)]
#[case::view_sync_finalize_certificate2(
    own_json("view_sync_finalize_certificate2"),
    "/data/epoch",
    assert_null_rejected::<Vsc2>
)]
#[case::quorum_vote2(
    v6_message("Vote1")["vote"].clone(),
    "/data/epoch",
    assert_null_rejected::<QuorumVote2<SeqTypes>>
)]
#[case::timeout_vote2(
    v6_message("TimeoutVote")["vote"].clone(),
    "/data/epoch",
    assert_null_rejected::<TimeoutVote2<SeqTypes>>
)]
#[case::view_sync_finalize_vote2(
    own_json("view_sync_finalize_vote2"),
    "/data/epoch",
    assert_null_rejected::<Vsv2>
)]
fn live_types_reject_null_epoch(
    #[case] base: Value,
    #[case] field: &str,
    #[case] check: fn(&Value, &str),
) {
    check(&base, field);
}

#[rstest]
#[case::quorum_certificate2(
    qc(FIRST_EPOCH_LEAF),
    "quorum_certificate2_epoch_none",
    assert_none_rejected::<QuorumCertificate2<SeqTypes>>
)]
#[case::next_epoch_quorum_certificate2(
    next_epoch_qc(TRANSITION_LEAF),
    "next_epoch_quorum_certificate2_epoch_none",
    assert_none_rejected::<NextEpochQuorumCertificate2<SeqTypes>>
)]
#[case::leaf2_justify_qc(
    leaf(FIRST_EPOCH_LEAF),
    "leaf2_justify_qc_epoch_none",
    assert_none_rejected::<Leaf2<SeqTypes>>
)]
#[case::leaf2_next_epoch_justify_qc(
    leaf(TRANSITION_LEAF),
    "leaf2_next_epoch_justify_qc_epoch_none",
    assert_none_rejected::<Leaf2<SeqTypes>>
)]
#[case::leaf2_timeout_evidence(
    own_json("leaf2_timeout_evidence"),
    "leaf2_timeout_evidence_epoch_none",
    assert_none_rejected::<Leaf2<SeqTypes>>
)]
#[case::leaf2_view_sync_evidence(
    own_json("leaf2_view_sync_evidence"),
    "leaf2_view_sync_evidence_epoch_none",
    assert_none_rejected::<Leaf2<SeqTypes>>
)]
#[case::certificate_pair_qc(
    pair_for_parent(FIRST_EPOCH_LEAF),
    "certificate_pair_qc_epoch_none",
    assert_none_rejected::<CertificatePair<SeqTypes>>
)]
#[case::certificate_pair_next_epoch_qc(
    pair_for_parent(TRANSITION_LEAF),
    "certificate_pair_next_epoch_qc_epoch_none",
    assert_none_rejected::<CertificatePair<SeqTypes>>
)]
#[case::timeout_certificate2(
    v6_message("TimeoutCertificate"),
    "timeout_certificate2_epoch_none",
    assert_none_rejected::<TimeoutCertificate2<SeqTypes>>
)]
#[case::view_sync_finalize_certificate2(
    own_json("view_sync_finalize_certificate2"),
    "view_sync_finalize_certificate2_epoch_none",
    assert_none_rejected::<Vsc2>
)]
#[case::quorum_vote2(
    v6_message("Vote1")["vote"].clone(),
    "quorum_vote2_epoch_none",
    assert_none_rejected::<QuorumVote2<SeqTypes>>
)]
#[case::timeout_vote2(
    v6_message("TimeoutVote")["vote"].clone(),
    "timeout_vote2_epoch_none",
    assert_none_rejected::<TimeoutVote2<SeqTypes>>
)]
#[case::view_sync_finalize_vote2(
    own_json("view_sync_finalize_vote2"),
    "view_sync_finalize_vote2_epoch_none",
    assert_none_rejected::<Vsv2>
)]
fn live_types_reject_none_epoch_in_bincode(
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
fn historic_types_reject_qc_with_block_number_but_no_epoch(
    #[case] base: Value,
    #[case] data: &str,
    #[case] check: fn(&Value, &str),
) {
    assert_has_block_number(&base, data);
    check(&base, &format!("{data}/epoch"));
}

#[rstest]
#[case::historic_qc(
    qc(FIRST_EPOCH_LEAF),
    "/data",
    "quorum_certificate2_epoch_none",
    assert_none_rejected::<HistoricQuorumCertificate<SeqTypes>>
)]
#[case::historic_leaf_justify_qc(
    leaf(FIRST_EPOCH_LEAF),
    "/justify_qc/data",
    "leaf2_justify_qc_epoch_none",
    assert_none_rejected::<HistoricLeaf<SeqTypes>>
)]
#[case::historic_pair(
    pair_for_parent(FIRST_EPOCH_LEAF),
    "/qc/data",
    "certificate_pair_qc_epoch_none",
    assert_none_rejected::<HistoricCertificatePair<SeqTypes>>
)]
#[case::leaf_query_data_qc(
    leaf_query_data(FIRST_EPOCH_LEAF),
    "/qc/data",
    "leaf_query_data_qc_epoch_none",
    assert_none_rejected::<LeafQueryData<SeqTypes>>
)]
fn historic_types_reject_qc_with_block_number_but_no_epoch_in_bincode(
    #[case] base: Value,
    #[case] data: &str,
    #[case] none: &str,
    #[case] check: fn(&Value, &str),
) {
    assert_has_block_number(&base, data);
    check(&base, none);
}

/// The leaf's justify QC has no epoch but a block number, and the QC signs the leaf with that
/// justify QC. Today the leaf decodes as an epoch leaf.
#[test]
fn leaf_query_data_rejects_justify_qc_with_block_number_but_no_epoch() {
    let json = own_json(MIXED_LEAF_QUERY_DATA);
    let data = &json["leaf"]["justify_qc"]["data"];
    assert!(
        data["epoch"].is_null() && data["block_number"].is_u64(),
        "{data}"
    );

    let decoded = try_from_json::<LeafQueryData<SeqTypes>>(&json);
    assert!(decoded.is_err(), "JSON decoded: {decoded:?}");
}

/// The bincode of the input of `leaf_query_data_rejects_justify_qc_with_block_number_but_no_epoch`.
#[test]
fn leaf_query_data_rejects_justify_qc_with_block_number_but_no_epoch_in_bincode() {
    let decoded = bincode::deserialize::<LeafQueryData<SeqTypes>>(&own_hex(MIXED_LEAF_QUERY_DATA));
    assert!(decoded.is_err(), "bincode decoded: {decoded:?}");
}

/// A pre-epoch QC never has a next-epoch QC with it. Today such a pair decodes as an epoch pair.
#[test]
fn historic_pair_rejects_pre_epoch_qc_with_next_epoch_qc() {
    let decoded = try_from_json::<HistoricCertificatePair<SeqTypes>>(&pre_epoch_pair_json());
    assert!(decoded.is_err(), "JSON decoded: {decoded:?}");
}

#[test]
fn historic_pair_rejects_pre_epoch_qc_with_next_epoch_qc_in_bincode() {
    let json = pre_epoch_pair_json();
    let bytes = own_hex("pre_epoch_pair_with_next_epoch_qc");
    assert_eq!(
        hex::encode(&bytes),
        hex::encode(pair_bincode(&json["qc"], &json["next_epoch_qc"])),
        "the vector is not the pair"
    );

    let decoded = bincode::deserialize::<HistoricCertificatePair<SeqTypes>>(&bytes);
    assert!(decoded.is_err(), "bincode decoded: {decoded:?}");
}

/// `version` used to decide the genesis era. A version below 0.3 now builds the epoch genesis too.
#[rstest]
#[case::v0_1(VERSION_0_1)]
#[case::v0_6(NEW_PROTOCOL_VERSION)]
#[tokio::test]
async fn leaf2_genesis_is_the_epoch_genesis(#[case] version: Version) {
    let genesis = Leaf2::<SeqTypes>::genesis(&validated_state(), &instance_state(), version).await;
    let json = serde_json::to_value(&genesis).unwrap();
    assert_eq!(
        json["justify_qc"]["data"]["epoch"],
        json!(1),
        "justify QC epoch"
    );
    assert_eq!(
        json["justify_qc"]["data"]["block_number"],
        json!(0),
        "justify QC block number"
    );
    assert_eq!(json["with_epoch"], json!(true));
    assert_eq!(
        hex::encode(raw(genesis.commit())),
        golden_commitment("leaf2_genesis")
    );
    assert_eq!(json, own_json("leaf2_genesis"));
    assert_eq!(
        hex::encode(bincode::serialize(&genesis).unwrap()),
        hex::encode(own_hex("leaf2_genesis"))
    );
}

/// `upgrade` used to decide the genesis era. A base below 0.3 now builds the epoch genesis too.
#[rstest]
#[case::v0_1(LEGACY_UPGRADE)]
#[case::v0_6(Upgrade::trivial(NEW_PROTOCOL_VERSION))]
#[tokio::test]
async fn quorum_certificate2_genesis_is_the_epoch_genesis(#[case] upgrade: Upgrade) {
    let genesis =
        QuorumCertificate2::<SeqTypes>::genesis(&validated_state(), &instance_state(), upgrade)
            .await;
    let json = serde_json::to_value(&genesis).unwrap();
    assert_eq!(json["data"]["epoch"], json!(1), "epoch");
    assert_eq!(json["data"]["block_number"], json!(0), "block number");
    assert_eq!(
        hex::encode(raw(genesis.data.leaf_commit)),
        golden_commitment("leaf2_genesis")
    );
    assert_eq!(json, own_json("quorum_certificate2_genesis"));
    assert_eq!(
        hex::encode(bincode::serialize(&genesis).unwrap()),
        hex::encode(own_hex("quorum_certificate2_genesis"))
    );
}

/// The genesis states of the reference tests.
fn validated_state() -> ValidatedState {
    ValidatedState::default()
}

fn instance_state() -> NodeState {
    NodeState::default()
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

    fn len(&self) -> usize {
        self.0.len()
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

/// A pair of the pre-epoch justify QC of a leaf and the next-epoch justify QC of another.
fn pre_epoch_pair_json() -> Value {
    let pre_epoch_qc = leaf(PRE_EPOCH_LEAF)["justify_qc"].clone();
    let data = &pre_epoch_qc["data"];
    assert!(
        data["epoch"].is_null() && data["block_number"].is_null(),
        "{data}"
    );
    json!({ "qc": pre_epoch_qc, "next_epoch_qc": next_epoch_qc(TRANSITION_LEAF) })
}

/// The bincode of a `CertificatePair` of `qc`, a pre-epoch QC, and `next_epoch_qc`: the QC, then
/// the `Some` tag and the next-epoch QC.
fn pair_bincode(qc: &Value, next_epoch_qc: &Value) -> Vec<u8> {
    let qc: HistoricQuorumCertificate<SeqTypes> = from_json(qc);
    assert!(matches!(qc, HistoricQuorumCertificate::PreEpoch(_)));
    let next_epoch_qc: NextEpochQuorumCertificate2<SeqTypes> = from_json(next_epoch_qc);
    let mut bytes = bincode::serialize(&qc).unwrap();
    bytes.push(1);
    bytes.extend(bincode::serialize(&next_epoch_qc).unwrap());
    bytes
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
/// setting the epoch to `null` makes a QC that fits neither era.
fn assert_has_block_number(json: &Value, data: &str) {
    let data = json.pointer(data).expect("the QC data exists");
    assert!(
        data["epoch"].is_u64() && data["block_number"].is_u64(),
        "{data}"
    );
}

/// Decodes `json`, and checks that it encodes back to the same JSON and to `bytes`, which decode
/// back to the same value.
fn assert_golden<T>(json: &Value, bytes: &[u8])
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    assert_json_golden::<T>(json);
    let value: T = from_json(json);
    assert_eq!(
        hex::encode(bincode::serialize(&value).unwrap()),
        hex::encode(bytes),
        "bincode changed"
    );
    // `bincode_opts` rejects trailing bytes, so the decoder must read exactly the golden bytes.
    let decoded: T = bincode_opts()
        .deserialize(bytes)
        .expect("golden bincode decodes");
    assert_eq!(decoded, value, "bincode decodes to another value");
}

/// Decodes `json`, and checks that it encodes back to the same JSON.
fn assert_json_golden<T: Serialize + DeserializeOwned>(json: &Value) {
    let value: T = from_json(json);
    assert_eq!(serde_json::to_value(&value).unwrap(), *json, "JSON changed");
}

/// Checks that `base` decodes as `T`, but no longer once the epoch at the JSON pointer `field` is
/// `null`.
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

/// Checks that `vectors/{none}.hex`, the bincode of `base` with one epoch set to `None`, does not
/// decode as `T`.
fn assert_none_rejected<T: Serialize + DeserializeOwned + Debug>(base: &Value, none: &str) {
    let value: T = from_json(base);
    let none_bytes = own_hex(none);
    assert_only_one_option_is_none(&bincode::serialize(&value).unwrap(), &none_bytes);

    let decoded = bincode::deserialize::<T>(&none_bytes);
    assert!(
        decoded.is_err(),
        "decoding succeeded with a None epoch in {none}: {decoded:?}"
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

fn commit_of<T: Committable + DeserializeOwned>(json: &Value) -> [u8; 32] {
    raw(from_json::<T>(json).commit())
}

/// A commitment from `vectors/commitments.json`, which today's code wrote, as hex.
fn golden_commitment(name: &str) -> String {
    let commitments = own_json("commitments");
    commitments[name]
        .as_str()
        .unwrap_or_else(|| panic!("no commitment {name}"))
        .to_string()
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

fn from_json<T: DeserializeOwned>(json: &Value) -> T {
    try_from_json(json).unwrap_or_else(|err| panic!("JSON does not decode: {err}"))
}

fn try_from_json<T: DeserializeOwned>(json: &Value) -> serde_json::Result<T> {
    serde_json::from_str(&json.to_string())
}

fn own_json(name: &str) -> Value {
    read_json(&vectors_dir("strict_epoch").join(format!("{name}.json")))
}

fn own_hex(name: &str) -> Vec<u8> {
    read_hex(&vectors_dir("strict_epoch").join(format!("{name}.hex")))
}

/// A bincode vector of `tests/history_types`, which pins the recorded `v2-v3` chain.
fn history_hex(name: &str) -> Vec<u8> {
    read_hex(&vectors_dir("history_types").join(format!("{name}.hex")))
}

fn vectors_dir(test: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(test)
        .join("vectors")
}

fn read_hex(path: &Path) -> Vec<u8> {
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
