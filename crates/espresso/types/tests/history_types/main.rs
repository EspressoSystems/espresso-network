//! The history types, which hold leaves and QCs from before and after the epoch upgrade.
//!
//! They must encode exactly like today's `Leaf2`, `QuorumCertificate2` and `CertificatePair`, and
//! have the same commitments, so stored data and other nodes stay compatible. The era comes from
//! the fields: a QC is pre-epoch when its data has neither an epoch nor a block number, a leaf
//! when its justify QC is.
//!
//! The inputs are the chains recorded in `data/legacy-chains`, kept as JSON, and the `data/v3`
//! leaf vector. `Leaf2` and `QuorumCertificate2` will require an epoch, so the tests never decode
//! pre-epoch data as them. The bincode of pre-epoch data is pinned by `vectors/*.hex` instead,
//! which today's `Leaf2`, `QuorumCertificate2` and `CertificatePair` wrote from the recorded JSON
//! with `bincode::serialize`.
//!
//! In the `v2-v3` chain, leaves 0 to 332 have a pre-epoch justify QC, and QCs 0 to 331 are
//! pre-epoch. Leaf 332 is the first leaf with `with_epoch` set; its own QC has an epoch.

use std::{
    fmt::Debug,
    path::{Path, PathBuf},
};

use alloy::hex;
use bincode::Options;
use committable::{Commitment, Committable};
use espresso_types::{Header, Payload, SeqTypes};
use hotshot_types::{
    PeerConfig,
    data::{
        Leaf2, VidCommitment, ViewNumber,
        history::{
            HistoricCertificatePair, HistoricLeaf, HistoricQuorumCertificate, PreEpochLeaf,
            PreEpochQuorumCertificate, PreEpochQuorumData,
        },
    },
    message::UpgradeLock,
    simple_certificate::{CertificatePair, QuorumCertificate2, UpgradeCertificate},
    simple_vote::VersionedVoteData,
    stake_table::{StakeTableEntries, supermajority_threshold},
    traits::signature_key::StakeTableEntryType,
    utils::bincode_opts,
    vote::Certificate,
};
use pretty_assertions::{assert_eq, assert_ne};
use rstest::rstest;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use versions::{FEE_VERSION, Upgrade, version};

/// The chain that upgraded from 0.2 to 0.3, so it holds both eras.
const UPGRADE_CHAIN: &str = "v2-v3";
/// The last pre-epoch QC of the upgrade chain.
const LAST_PRE_EPOCH_QC: usize = 331;
/// The last leaf of the upgrade chain with a pre-epoch justify QC, and the first with
/// `with_epoch` set.
const BOUNDARY_LEAF: usize = 332;
/// The first leaf of the upgrade chain with an epoch justify QC.
const FIRST_EPOCH_LEAF: usize = 333;
/// A leaf of the upgrade chain with a next-epoch justify QC and a next DRB result.
const TRANSITION_LEAF: usize = 398;

#[test]
fn historic_qc_without_epoch_and_block_number_is_pre_epoch() {
    let chain = Chain::load(UPGRADE_CHAIN);
    for i in 0..=BOUNDARY_LEAF {
        let qc: HistoricQuorumCertificate<SeqTypes> = from_json(&chain.leaf(i)["justify_qc"]);
        assert_eq!(qc.era(), Era::PreEpoch, "justify QC of leaf {i}");
    }
}

#[rstest]
#[case::upgrade_chain(UPGRADE_CHAIN, BOUNDARY_LEAF)]
#[case::v3("v3", 0)]
#[case::v4("v4", 0)]
#[case::v5("v5", 0)]
fn historic_qc_with_epoch_is_epoch(#[case] name: &str, #[case] first_qc_with_epoch: usize) {
    let chain = Chain::load(name);
    for i in first_qc_with_epoch..chain.len() {
        let qc: HistoricQuorumCertificate<SeqTypes> = from_json(chain.qc(i));
        let expected: QuorumCertificate2<SeqTypes> = from_json(chain.qc(i));
        assert_eq!(
            qc,
            HistoricQuorumCertificate::Epoch(expected),
            "{name}: QC {i}"
        );
    }
}

/// Such a QC is malformed. It must not decode as pre-epoch, which would drop the field.
#[rstest]
#[case::epoch_only(Some(2), None)]
#[case::block_number_only(None, Some(5))]
fn historic_qc_with_epoch_or_block_number_is_not_pre_epoch(
    #[case] epoch: Option<u64>,
    #[case] block_number: Option<u64>,
) {
    let chain = Chain::load(UPGRADE_CHAIN);
    let input = set_epoch_fields(chain.qc(LAST_PRE_EPOCH_QC), "/data", epoch, block_number);
    let decoded = try_from_json::<HistoricQuorumCertificate<SeqTypes>>(&input);
    assert!(
        !matches!(decoded, Ok(HistoricQuorumCertificate::PreEpoch(_))),
        "decoded as pre-epoch: {decoded:?}"
    );
}

#[test]
fn historic_leaf_with_pre_epoch_justify_qc_is_pre_epoch() {
    let chain = Chain::load(UPGRADE_CHAIN);
    for i in 0..=BOUNDARY_LEAF {
        let leaf: HistoricLeaf<SeqTypes> = from_json(chain.leaf(i));
        assert_eq!(leaf.era(), Era::PreEpoch, "leaf {i}");
    }
}

/// The era comes from the justify QC, not from the `with_epoch` flag.
#[test]
fn boundary_leaf_with_epoch_flag_is_pre_epoch() {
    let chain = Chain::load(UPGRADE_CHAIN);
    let json = chain.leaf(BOUNDARY_LEAF);
    assert_eq!(
        json["with_epoch"],
        json!(true),
        "the boundary leaf sets with_epoch"
    );

    let leaf: HistoricLeaf<SeqTypes> = from_json(json);
    assert_eq!(leaf.era(), Era::PreEpoch);
}

#[rstest]
#[case::upgrade_chain(UPGRADE_CHAIN, FIRST_EPOCH_LEAF)]
#[case::v3("v3", 0)]
#[case::v4("v4", 0)]
#[case::v5("v5", 0)]
fn historic_leaf_with_epoch_justify_qc_is_epoch(#[case] name: &str, #[case] first: usize) {
    let chain = Chain::load(name);
    for i in first..chain.len() {
        let leaf: HistoricLeaf<SeqTypes> = from_json(chain.leaf(i));
        let expected: Leaf2<SeqTypes> = from_json(chain.leaf(i));
        assert_eq!(leaf, HistoricLeaf::Epoch(expected), "{name}: leaf {i}");
    }
}

#[test]
fn historic_pair_with_pre_epoch_qc_and_no_next_epoch_qc_is_pre_epoch() {
    let chain = Chain::load(UPGRADE_CHAIN);
    for i in 0..=BOUNDARY_LEAF {
        let pair: HistoricCertificatePair<SeqTypes> = from_json(&chain.pair_for_parent(i));
        assert_eq!(pair.era(), Era::PreEpoch, "pair for the parent of leaf {i}");
    }
}

#[rstest]
#[case::upgrade_chain(UPGRADE_CHAIN, FIRST_EPOCH_LEAF)]
#[case::v3("v3", 0)]
#[case::v4("v4", 0)]
#[case::v5("v5", 0)]
fn historic_pair_with_epoch_qc_is_epoch(#[case] name: &str, #[case] first: usize) {
    let chain = Chain::load(name);
    for i in first..chain.len() {
        let json = chain.pair_for_parent(i);
        let pair: HistoricCertificatePair<SeqTypes> = from_json(&json);
        let expected: CertificatePair<SeqTypes> = from_json(&json);
        assert_eq!(
            pair,
            HistoricCertificatePair::Epoch(expected),
            "{name}: pair for the parent of leaf {i}"
        );
    }
}

/// A pre-epoch pair never has a next-epoch QC, so decoding this one as pre-epoch would drop it.
#[test]
fn historic_pair_with_next_epoch_qc_is_not_pre_epoch() {
    let chain = Chain::load(UPGRADE_CHAIN);
    let mut input = chain.pair_for_parent(5);
    input["next_epoch_qc"] = chain.leaf(TRANSITION_LEAF)["next_epoch_justify_qc"].clone();
    assert!(input["next_epoch_qc"].is_object());

    let decoded = try_from_json::<HistoricCertificatePair<SeqTypes>>(&input);
    assert!(
        !matches!(decoded, Ok(HistoricCertificatePair::PreEpoch(_))),
        "decoded as pre-epoch: {decoded:?}"
    );
}

#[rstest]
#[case::upgrade_chain(UPGRADE_CHAIN)]
#[case::v3("v3")]
#[case::v4("v4")]
#[case::v5("v5")]
fn historic_leaf_json_round_trips(#[case] name: &str) {
    let chain = Chain::load(name);
    for i in 0..chain.len() {
        assert_json_round_trip::<HistoricLeaf<SeqTypes>>(
            chain.leaf(i),
            &format!("{name} leaf {i}"),
        );
    }
}

#[rstest]
#[case::pre_epoch(5)]
#[case::epoch(FIRST_EPOCH_LEAF)]
fn historic_leaf_json_with_block_payload_round_trips(#[case] i: usize) {
    let chain = Chain::load(UPGRADE_CHAIN);
    let json = chain.leaf_with_payload(i);
    assert_json_round_trip::<HistoricLeaf<SeqTypes>>(&json, &format!("leaf {i} with payload"));
}

#[rstest]
#[case::upgrade_chain(UPGRADE_CHAIN)]
#[case::v3("v3")]
#[case::v4("v4")]
#[case::v5("v5")]
fn historic_qc_json_round_trips(#[case] name: &str) {
    let chain = Chain::load(name);
    assert_json_round_trip::<HistoricQuorumCertificate<SeqTypes>>(
        &chain.leaf(0)["justify_qc"],
        &format!("{name} justify QC of leaf 0"),
    );
    for i in 0..chain.len() {
        assert_json_round_trip::<HistoricQuorumCertificate<SeqTypes>>(
            chain.qc(i),
            &format!("{name} QC {i}"),
        );
    }
}

#[rstest]
#[case::upgrade_chain(UPGRADE_CHAIN)]
#[case::v3("v3")]
#[case::v4("v4")]
#[case::v5("v5")]
fn historic_pair_json_round_trips(#[case] name: &str) {
    let chain = Chain::load(name);
    for i in 0..chain.len() {
        assert_json_round_trip::<HistoricCertificatePair<SeqTypes>>(
            &chain.pair_for_parent(i),
            &format!("{name} pair for the parent of leaf {i}"),
        );
    }
}

#[test]
fn pre_epoch_leaf_json_round_trips() {
    let chain = Chain::load(UPGRADE_CHAIN);
    for i in 0..=BOUNDARY_LEAF {
        assert_json_round_trip::<PreEpochLeaf<SeqTypes>>(chain.leaf(i), &format!("leaf {i}"));
    }
    let with_payload = chain.leaf_with_payload(5);
    assert_json_round_trip::<PreEpochLeaf<SeqTypes>>(&with_payload, "leaf 5 with payload");
}

#[test]
fn pre_epoch_qc_json_round_trips() {
    let chain = Chain::load(UPGRADE_CHAIN);
    for i in 0..=BOUNDARY_LEAF {
        assert_json_round_trip::<PreEpochQuorumCertificate<SeqTypes>>(
            &chain.leaf(i)["justify_qc"],
            &format!("justify QC of leaf {i}"),
        );
    }
}

#[rstest]
#[case::genesis(0, false, Era::PreEpoch)]
#[case::first(1, false, Era::PreEpoch)]
#[case::with_payload(5, true, Era::PreEpoch)]
#[case::with_upgrade_certificate(LAST_PRE_EPOCH_QC, false, Era::PreEpoch)]
#[case::boundary(BOUNDARY_LEAF, false, Era::PreEpoch)]
#[case::first_epoch(FIRST_EPOCH_LEAF, false, Era::Epoch)]
#[case::first_epoch_with_payload(FIRST_EPOCH_LEAF, true, Era::Epoch)]
#[case::transition(TRANSITION_LEAF, false, Era::Epoch)]
fn historic_leaf_bincode_matches_leaf2(
    #[case] i: usize,
    #[case] with_payload: bool,
    #[case] era: Era,
) {
    let chain = Chain::load(UPGRADE_CHAIN);
    let (json, golden) = chain.leaf_and_golden(i, with_payload);
    let leaf: HistoricLeaf<SeqTypes> = assert_bincode_golden(&json, &golden);
    assert_eq!(leaf.era(), era);
}

#[rstest]
#[case::genesis(0, false)]
#[case::first(1, false)]
#[case::with_payload(5, true)]
#[case::with_upgrade_certificate(LAST_PRE_EPOCH_QC, false)]
#[case::boundary(BOUNDARY_LEAF, false)]
fn pre_epoch_leaf_bincode_matches_leaf2(#[case] i: usize, #[case] with_payload: bool) {
    let chain = Chain::load(UPGRADE_CHAIN);
    let (json, golden) = chain.leaf_and_golden(i, with_payload);
    assert_bincode_golden::<PreEpochLeaf<SeqTypes>>(&json, &golden);
}

#[rstest]
#[case::genesis(0, Era::PreEpoch)]
#[case::first(1, Era::PreEpoch)]
#[case::last_pre_epoch(LAST_PRE_EPOCH_QC, Era::PreEpoch)]
#[case::first_epoch(BOUNDARY_LEAF, Era::Epoch)]
#[case::second_epoch(FIRST_EPOCH_LEAF, Era::Epoch)]
#[case::transition(TRANSITION_LEAF, Era::Epoch)]
fn historic_qc_bincode_matches_quorum_certificate2(#[case] i: usize, #[case] era: Era) {
    let chain = Chain::load(UPGRADE_CHAIN);
    let qc: HistoricQuorumCertificate<SeqTypes> =
        assert_bincode_golden(chain.qc(i), &format!("qc_{i}"));
    assert_eq!(qc.era(), era);
}

#[rstest]
#[case::genesis(0)]
#[case::first(1)]
#[case::last_pre_epoch(LAST_PRE_EPOCH_QC)]
fn pre_epoch_qc_bincode_matches_quorum_certificate2(#[case] i: usize) {
    let chain = Chain::load(UPGRADE_CHAIN);
    assert_bincode_golden::<PreEpochQuorumCertificate<SeqTypes>>(chain.qc(i), &format!("qc_{i}"));
}

#[rstest]
#[case::genesis(0, Era::PreEpoch)]
#[case::first(1, Era::PreEpoch)]
#[case::pre_epoch(LAST_PRE_EPOCH_QC, Era::PreEpoch)]
#[case::boundary(BOUNDARY_LEAF, Era::PreEpoch)]
#[case::first_epoch(FIRST_EPOCH_LEAF, Era::Epoch)]
#[case::transition(TRANSITION_LEAF, Era::Epoch)]
fn historic_pair_bincode_matches_certificate_pair(#[case] i: usize, #[case] era: Era) {
    let chain = Chain::load(UPGRADE_CHAIN);
    let golden = format!("pair_for_parent_of_{i}");
    let pair: HistoricCertificatePair<SeqTypes> =
        assert_bincode_golden(&chain.pair_for_parent(i), &golden);
    assert_eq!(pair.era(), era);
}

#[rstest]
#[case::v3("v3")]
#[case::v4("v4")]
#[case::v5("v5")]
fn historic_types_encode_epoch_chains_like_today_types(#[case] name: &str) {
    let chain = Chain::load(name);
    for i in 0..chain.len() {
        let leaf: Leaf2<SeqTypes> = from_json(chain.leaf(i));
        let qc: QuorumCertificate2<SeqTypes> = from_json(chain.qc(i));
        let pair = CertificatePair::for_parent(&leaf);
        let what = format!("{name} item {i}");
        assert_bincode_like(&leaf, HistoricLeaf::Epoch(leaf.clone()), &what);
        assert_bincode_like(&qc, HistoricQuorumCertificate::Epoch(qc.clone()), &what);
        assert_bincode_like(&pair, HistoricCertificatePair::Epoch(pair.clone()), &what);
    }
}

#[test]
fn pre_epoch_types_read_the_v3_leaf_query_data_vector() {
    assert_reads_v3_leaf_vector::<PreEpochLeaf<SeqTypes>, PreEpochQuorumCertificate<SeqTypes>>();
}

#[test]
fn historic_types_read_the_v3_leaf_query_data_vector() {
    let (leaf, qc) =
        assert_reads_v3_leaf_vector::<HistoricLeaf<SeqTypes>, HistoricQuorumCertificate<SeqTypes>>(
        );
    assert_eq!(leaf.era(), Era::PreEpoch);
    assert_eq!(qc.era(), Era::PreEpoch);
}

#[rstest]
#[case::epoch(Some(2), None)]
#[case::block_number(None, Some(5))]
#[case::both(Some(2), Some(5))]
fn pre_epoch_quorum_data_rejects_epoch_or_block_number(
    #[case] epoch: Option<u64>,
    #[case] block_number: Option<u64>,
) {
    let chain = Chain::load(UPGRADE_CHAIN);
    let data = &chain.qc(LAST_PRE_EPOCH_QC)["data"];
    // The unchanged data must decode, so the error below comes from the change alone.
    from_json::<PreEpochQuorumData<SeqTypes>>(data);

    let input = set_epoch_fields(data, "", epoch, block_number);
    let decoded = try_from_json::<PreEpochQuorumData<SeqTypes>>(&input);
    assert!(decoded.is_err(), "decoded {input}");
}

#[rstest]
#[case::epoch(Some(2), None)]
#[case::block_number(None, Some(5))]
#[case::both(Some(2), Some(5))]
fn pre_epoch_qc_rejects_epoch_or_block_number(
    #[case] epoch: Option<u64>,
    #[case] block_number: Option<u64>,
) {
    let chain = Chain::load(UPGRADE_CHAIN);
    let qc = chain.qc(LAST_PRE_EPOCH_QC);
    from_json::<PreEpochQuorumCertificate<SeqTypes>>(qc);

    let input = set_epoch_fields(qc, "/data", epoch, block_number);
    let decoded = try_from_json::<PreEpochQuorumCertificate<SeqTypes>>(&input);
    assert!(decoded.is_err(), "decoded {input}");
}

#[test]
fn pre_epoch_qc_rejects_a_recorded_epoch_qc() {
    let chain = Chain::load(UPGRADE_CHAIN);
    let json = chain.qc(BOUNDARY_LEAF);
    assert!(
        json["data"]["epoch"].is_u64(),
        "QC {BOUNDARY_LEAF} has an epoch"
    );

    let decoded_json = try_from_json::<PreEpochQuorumCertificate<SeqTypes>>(json);
    assert!(decoded_json.is_err(), "JSON decoded");
    let bytes = read_hex(&format!("qc_{BOUNDARY_LEAF}"));
    let decoded_bincode = bincode_opts().deserialize::<PreEpochQuorumCertificate<SeqTypes>>(&bytes);
    assert!(decoded_bincode.is_err(), "bincode decoded");
}

#[rstest]
#[case::epoch(Some(1), None)]
#[case::block_number(None, Some(5))]
#[case::both(Some(1), Some(5))]
fn pre_epoch_leaf_rejects_justify_qc_with_epoch_or_block_number(
    #[case] epoch: Option<u64>,
    #[case] block_number: Option<u64>,
) {
    let chain = Chain::load(UPGRADE_CHAIN);
    let leaf = chain.leaf(5);
    from_json::<PreEpochLeaf<SeqTypes>>(leaf);

    let input = set_epoch_fields(leaf, "/justify_qc/data", epoch, block_number);
    let decoded = try_from_json::<PreEpochLeaf<SeqTypes>>(&input);
    assert!(
        decoded.is_err(),
        "decoded a leaf whose justify QC is {}",
        input["justify_qc"]
    );
}

#[test]
fn pre_epoch_leaf_rejects_a_recorded_epoch_leaf() {
    let chain = Chain::load(UPGRADE_CHAIN);
    let json = chain.leaf(FIRST_EPOCH_LEAF);
    assert!(json["justify_qc"]["data"]["epoch"].is_u64());

    let decoded_json = try_from_json::<PreEpochLeaf<SeqTypes>>(json);
    assert!(decoded_json.is_err(), "JSON decoded");
    let bytes = read_hex(&format!("leaf_{FIRST_EPOCH_LEAF}"));
    let decoded_bincode = bincode_opts().deserialize::<PreEpochLeaf<SeqTypes>>(&bytes);
    assert!(decoded_bincode.is_err(), "bincode decoded");
}

/// A leaf's QC signs the leaf's commitment, and the next leaf holds it as its parent. The leaf
/// commitment covers the justify QC's commitment, so this also checks the QC commitments.
#[rstest]
#[case::upgrade_chain(UPGRADE_CHAIN)]
#[case::v3("v3")]
#[case::v4("v4")]
#[case::v5("v5")]
fn historic_leaf_commit_matches_the_recorded_chain(#[case] name: &str) {
    let chain = Chain::load(name);
    for i in 0..chain.len() {
        let leaf: HistoricLeaf<SeqTypes> = from_json(chain.leaf(i));
        assert_commit_matches_chain(&chain, i, leaf.commit(), name);
    }
}

#[test]
fn pre_epoch_leaf_commit_matches_the_recorded_chain() {
    let chain = Chain::load(UPGRADE_CHAIN);
    for i in 0..=BOUNDARY_LEAF {
        let leaf: PreEpochLeaf<SeqTypes> = from_json(chain.leaf(i));
        assert_commit_matches_chain(&chain, i, leaf.commit(), UPGRADE_CHAIN);
    }
}

#[test]
fn pre_epoch_leaf_commit_ignores_the_block_payload() {
    let chain = Chain::load(UPGRADE_CHAIN);
    let leaf: PreEpochLeaf<SeqTypes> = from_json(&chain.leaf_with_payload(5));
    assert!(leaf.block_payload().is_some());
    assert_eq!(leaf.commit(), chain.signed_leaf_commit(5));
}

#[test]
fn pre_epoch_leaf_commit_matches_the_qc_of_the_v3_vector() {
    let vector = read_json(&data_dir().join("v3/leaf_query_data.json"));
    let leaf: PreEpochLeaf<SeqTypes> = from_json(&vector["leaf"]);
    assert_eq!(
        leaf.commit(),
        commitment(&vector["qc"]["data"]["leaf_commit"])
    );
}

/// A signed QC stores the commitment of the vote its signers signed, which covers the data's
/// commitment and the view.
#[test]
fn pre_epoch_quorum_data_commit_matches_the_recorded_vote_commitments() {
    let chain = Chain::load(UPGRADE_CHAIN);
    let upgrade_lock = upgrade_lock();
    for i in 0..=LAST_PRE_EPOCH_QC {
        let qc: PreEpochQuorumCertificate<SeqTypes> = from_json(chain.qc(i));
        let vote = VersionedVoteData::new(qc.data, qc.view_number, &upgrade_lock).unwrap();
        assert_eq!(raw(vote.commit()), raw(qc.vote_commitment()), "QC {i}");
    }
}

/// The genesis leaf's justify QC stores the commitment of its data as its vote commitment.
#[test]
fn pre_epoch_quorum_data_commit_matches_the_genesis_qc() {
    let chain = Chain::load(UPGRADE_CHAIN);
    let qc: PreEpochQuorumCertificate<SeqTypes> = from_json(&chain.leaf(0)["justify_qc"]);
    assert_eq!(raw(qc.data.commit()), raw(qc.vote_commitment()));
}

/// The pre-epoch QCs were signed by the genesis stake table, from the chain's network config.
#[test]
fn pre_epoch_qc_signatures_verify_against_the_genesis_stake_table() {
    let chain = Chain::load(UPGRADE_CHAIN);
    let stake_table = genesis_stake_table(UPGRADE_CHAIN).0;
    let threshold = supermajority_threshold(stake_table.iter().map(|entry| entry.stake()).sum());
    let upgrade_lock = upgrade_lock();
    // QC 0 signs the genesis leaf and has no signatures.
    for i in 1..=LAST_PRE_EPOCH_QC {
        let qc: PreEpochQuorumCertificate<SeqTypes> = from_json(chain.qc(i));
        qc.is_valid_cert(&stake_table, threshold, &upgrade_lock)
            .unwrap_or_else(|err| panic!("QC {i} does not verify: {err:?}"));
    }
}

#[rstest]
#[case::genesis(0)]
#[case::first(1)]
#[case::with_upgrade_certificate(LAST_PRE_EPOCH_QC)]
#[case::boundary(BOUNDARY_LEAF)]
fn pre_epoch_leaf_accessors_read_the_encoding(#[case] i: usize) {
    let chain = Chain::load(UPGRADE_CHAIN);
    let json = chain.leaf(i);
    let leaf: PreEpochLeaf<SeqTypes> = from_json(json);
    let expected = LeafFields::of(json);

    assert_eq!(leaf.view_number(), expected.view_number);
    assert_eq!(leaf.height(), expected.height);
    assert_eq!(leaf.block_header(), &expected.block_header);
    assert_eq!(leaf.parent_commitment(), expected.parent_commitment);
    assert_eq!(
        leaf.upgrade_certificate(),
        expected.upgrade_certificate.as_ref()
    );
    assert_eq!(leaf.payload_commitment(), expected.payload_commitment);
    let justify_qc: PreEpochQuorumCertificate<SeqTypes> = from_json(&json["justify_qc"]);
    assert_eq!(leaf.justify_qc(), &justify_qc);
}

#[rstest]
#[case::genesis(0)]
#[case::with_upgrade_certificate(LAST_PRE_EPOCH_QC)]
#[case::boundary(BOUNDARY_LEAF)]
#[case::first_epoch(FIRST_EPOCH_LEAF)]
#[case::epoch_with_upgrade_certificate(FIRST_EPOCH_LEAF + 1)]
#[case::transition(TRANSITION_LEAF)]
fn historic_leaf_accessors_read_the_encoding(#[case] i: usize) {
    let chain = Chain::load(UPGRADE_CHAIN);
    let json = chain.leaf(i);
    let leaf: HistoricLeaf<SeqTypes> = from_json(json);
    let expected = LeafFields::of(json);

    assert_eq!(leaf.view_number(), expected.view_number);
    assert_eq!(leaf.height(), expected.height);
    assert_eq!(leaf.block_header(), &expected.block_header);
    assert_eq!(leaf.parent_commitment(), expected.parent_commitment);
    assert_eq!(
        leaf.upgrade_certificate(),
        expected.upgrade_certificate.as_ref()
    );
    assert_eq!(leaf.payload_commitment(), expected.payload_commitment);
}

#[rstest]
#[case::genesis(0)]
#[case::boundary(BOUNDARY_LEAF)]
fn historic_leaf_justify_qc_of_pre_epoch_leaf_is_pre_epoch(#[case] i: usize) {
    let chain = Chain::load(UPGRADE_CHAIN);
    let leaf: HistoricLeaf<SeqTypes> = from_json(chain.leaf(i));
    let expected: PreEpochQuorumCertificate<SeqTypes> = from_json(&chain.leaf(i)["justify_qc"]);
    assert_eq!(
        leaf.justify_qc(),
        HistoricQuorumCertificate::PreEpoch(expected)
    );
}

#[rstest]
#[case::first_epoch(FIRST_EPOCH_LEAF)]
#[case::transition(TRANSITION_LEAF)]
fn historic_leaf_justify_qc_of_epoch_leaf_is_epoch(#[case] i: usize) {
    let chain = Chain::load(UPGRADE_CHAIN);
    let leaf: Leaf2<SeqTypes> = from_json(chain.leaf(i));
    let expected = HistoricQuorumCertificate::Epoch(leaf.justify_qc().clone());
    assert_eq!(HistoricLeaf::Epoch(leaf).justify_qc(), expected);
}

#[test]
fn historic_leaf_as_leaf2_is_none_for_pre_epoch_leaf() {
    let chain = Chain::load(UPGRADE_CHAIN);
    let leaf: HistoricLeaf<SeqTypes> = from_json(chain.leaf(BOUNDARY_LEAF));
    assert!(leaf.as_leaf2().is_none());
    assert!(leaf.into_leaf2().is_none());
}

#[test]
fn historic_leaf_as_leaf2_is_the_leaf_for_epoch_leaf() {
    let chain = Chain::load(UPGRADE_CHAIN);
    let expected: Leaf2<SeqTypes> = from_json(chain.leaf(FIRST_EPOCH_LEAF));
    let leaf = HistoricLeaf::Epoch(expected.clone());
    assert_eq!(leaf.as_leaf2(), Some(&expected));
    assert_eq!(leaf.into_leaf2(), Some(expected));
}

#[test]
fn pre_epoch_leaf_block_payload_can_be_read_and_taken() {
    let chain = Chain::load(UPGRADE_CHAIN);
    let payload: Payload = from_json(chain.payload(5));
    let mut leaf: PreEpochLeaf<SeqTypes> = from_json(&chain.leaf_with_payload(5));

    assert_eq!(leaf.block_payload(), Some(payload.clone()));
    assert_eq!(leaf.unfill_block_payload(), Some(payload));
    assert_eq!(leaf.block_payload(), None);
    assert_eq!(leaf.unfill_block_payload(), None);
    assert_eq!(serde_json::to_value(&leaf).unwrap(), *chain.leaf(5));
}

#[rstest]
#[case::pre_epoch(5)]
#[case::epoch(FIRST_EPOCH_LEAF)]
fn historic_leaf_block_payload_can_be_read_and_taken(#[case] i: usize) {
    let chain = Chain::load(UPGRADE_CHAIN);
    let payload: Payload = from_json(chain.payload(i));
    let mut leaf: HistoricLeaf<SeqTypes> = from_json(&chain.leaf_with_payload(i));

    assert_eq!(leaf.block_payload(), Some(payload.clone()));
    assert_eq!(leaf.unfill_block_payload(), Some(payload));
    assert_eq!(leaf.block_payload(), None);
    assert_eq!(leaf.unfill_block_payload(), None);
    assert_eq!(serde_json::to_value(&leaf).unwrap(), *chain.leaf(i));
}

#[rstest]
#[case::genesis(0)]
#[case::last_pre_epoch(LAST_PRE_EPOCH_QC)]
#[case::first_epoch(BOUNDARY_LEAF)]
#[case::transition(TRANSITION_LEAF)]
fn historic_qc_accessors_read_the_encoding(#[case] i: usize) {
    let chain = Chain::load(UPGRADE_CHAIN);
    let json = chain.qc(i);
    let qc: HistoricQuorumCertificate<SeqTypes> = from_json(json);
    assert_eq!(qc.view_number(), view_number(&json["view_number"]));
    assert_eq!(qc.leaf_commit(), commitment(&json["data"]["leaf_commit"]));
}

#[test]
fn historic_qc_as_qc2_is_none_for_pre_epoch_qc() {
    let chain = Chain::load(UPGRADE_CHAIN);
    let qc: HistoricQuorumCertificate<SeqTypes> = from_json(chain.qc(LAST_PRE_EPOCH_QC));
    assert!(qc.as_qc2().is_none());
    assert!(qc.into_qc2().is_none());
}

#[test]
fn historic_qc_as_qc2_is_the_qc_for_epoch_qc() {
    let chain = Chain::load(UPGRADE_CHAIN);
    let expected: QuorumCertificate2<SeqTypes> = from_json(chain.qc(BOUNDARY_LEAF));
    let qc = HistoricQuorumCertificate::Epoch(expected.clone());
    assert_eq!(qc.as_qc2(), Some(&expected));
    assert_eq!(qc.into_qc2(), Some(expected));
}

#[rstest]
#[case::genesis(0)]
#[case::first(1)]
#[case::boundary(BOUNDARY_LEAF)]
fn historic_pair_for_parent_of_pre_epoch_leaf_is_its_justify_qc(#[case] i: usize) {
    let chain = Chain::load(UPGRADE_CHAIN);
    let leaf: HistoricLeaf<SeqTypes> = from_json(chain.leaf(i));
    let expected: PreEpochQuorumCertificate<SeqTypes> = from_json(&chain.leaf(i)["justify_qc"]);
    assert_eq!(
        HistoricCertificatePair::for_parent(&leaf),
        HistoricCertificatePair::PreEpoch(expected)
    );
}

#[rstest]
#[case::first_epoch(UPGRADE_CHAIN, FIRST_EPOCH_LEAF)]
#[case::transition(UPGRADE_CHAIN, TRANSITION_LEAF)]
#[case::v3_transition("v3", 9)]
fn historic_pair_for_parent_of_epoch_leaf_has_its_justify_qcs(
    #[case] name: &str,
    #[case] i: usize,
) {
    let chain = Chain::load(name);
    let leaf: Leaf2<SeqTypes> = from_json(chain.leaf(i));
    let expected = CertificatePair::new(
        leaf.justify_qc().clone(),
        leaf.next_epoch_justify_qc().cloned(),
    );
    assert_eq!(
        HistoricCertificatePair::for_parent(&HistoricLeaf::Epoch(leaf)),
        HistoricCertificatePair::Epoch(expected)
    );
}

#[rstest]
#[case::genesis(0)]
#[case::boundary(BOUNDARY_LEAF)]
#[case::first_epoch(FIRST_EPOCH_LEAF)]
#[case::transition(TRANSITION_LEAF)]
fn historic_pair_accessors_read_the_encoding(#[case] i: usize) {
    let chain = Chain::load(UPGRADE_CHAIN);
    let json = chain.pair_for_parent(i);
    let pair: HistoricCertificatePair<SeqTypes> = from_json(&json);
    assert_eq!(pair.view_number(), view_number(&json["qc"]["view_number"]));
    assert_eq!(
        pair.leaf_commit(),
        commitment(&json["qc"]["data"]["leaf_commit"])
    );
}

#[test]
fn historic_pair_as_certificate_pair_is_none_for_pre_epoch_pair() {
    let chain = Chain::load(UPGRADE_CHAIN);
    let pair: HistoricCertificatePair<SeqTypes> = from_json(&chain.pair_for_parent(BOUNDARY_LEAF));
    assert!(pair.as_certificate_pair().is_none());
}

#[test]
fn historic_pair_as_certificate_pair_is_the_pair_for_epoch_pair() {
    let chain = Chain::load(UPGRADE_CHAIN);
    let expected: CertificatePair<SeqTypes> = from_json(&chain.pair_for_parent(TRANSITION_LEAF));
    let pair = HistoricCertificatePair::Epoch(expected.clone());
    assert_eq!(pair.as_certificate_pair(), Some(&expected));
}

#[test]
fn historic_qc_from_epoch_qc_is_epoch() {
    let chain = Chain::load(UPGRADE_CHAIN);
    for i in [BOUNDARY_LEAF, FIRST_EPOCH_LEAF, TRANSITION_LEAF] {
        let qc: QuorumCertificate2<SeqTypes> = from_json(chain.qc(i));
        assert_eq!(
            HistoricQuorumCertificate::from(qc.clone()),
            HistoricQuorumCertificate::Epoch(qc),
            "QC {i}"
        );
    }
}

#[test]
fn historic_qc_from_pre_epoch_qc_is_pre_epoch() {
    let chain = Chain::load(UPGRADE_CHAIN);
    let qc: PreEpochQuorumCertificate<SeqTypes> = from_json(chain.qc(LAST_PRE_EPOCH_QC));
    assert_eq!(
        HistoricQuorumCertificate::from(qc.clone()),
        HistoricQuorumCertificate::PreEpoch(qc)
    );
}

#[test]
fn historic_leaf_from_epoch_leaf_is_epoch() {
    let chain = Chain::load(UPGRADE_CHAIN);
    for i in [FIRST_EPOCH_LEAF, TRANSITION_LEAF] {
        let leaf: Leaf2<SeqTypes> = from_json(chain.leaf(i));
        assert_eq!(
            HistoricLeaf::from(leaf.clone()),
            HistoricLeaf::Epoch(leaf),
            "leaf {i}"
        );
    }
}

#[test]
fn historic_leaf_from_pre_epoch_leaf_is_pre_epoch() {
    let chain = Chain::load(UPGRADE_CHAIN);
    let leaf: PreEpochLeaf<SeqTypes> = from_json(chain.leaf(BOUNDARY_LEAF));
    assert_eq!(
        HistoricLeaf::from(leaf.clone()),
        HistoricLeaf::PreEpoch(leaf)
    );
}

#[test]
fn historic_pair_from_epoch_certificate_pair_is_epoch() {
    let chain = Chain::load(UPGRADE_CHAIN);
    for i in [FIRST_EPOCH_LEAF, TRANSITION_LEAF] {
        let pair: CertificatePair<SeqTypes> = from_json(&chain.pair_for_parent(i));
        assert_eq!(
            HistoricCertificatePair::from(pair.clone()),
            HistoricCertificatePair::Epoch(pair),
            "pair for the parent of leaf {i}"
        );
    }
}

#[test]
fn historic_pair_from_pre_epoch_qc_is_pre_epoch() {
    let chain = Chain::load(UPGRADE_CHAIN);
    let qc: PreEpochQuorumCertificate<SeqTypes> = from_json(chain.qc(LAST_PRE_EPOCH_QC));
    let pair = HistoricCertificatePair::from(HistoricQuorumCertificate::PreEpoch(qc.clone()));
    assert_eq!(pair, HistoricCertificatePair::PreEpoch(qc));
}

#[test]
fn historic_pair_from_epoch_qc_has_no_next_epoch_qc() {
    let chain = Chain::load(UPGRADE_CHAIN);
    // The QC of a transition leaf, which has a next-epoch QC in the next leaf.
    let qc: QuorumCertificate2<SeqTypes> = from_json(chain.qc(TRANSITION_LEAF));
    let pair = HistoricCertificatePair::from(HistoricQuorumCertificate::Epoch(qc.clone()));
    assert_eq!(
        pair,
        HistoricCertificatePair::Epoch(CertificatePair::new(qc, None))
    );
}

#[test]
fn pre_epoch_leaf_equality_ignores_the_block_payload() {
    let chain = Chain::load(UPGRADE_CHAIN);
    let without: PreEpochLeaf<SeqTypes> = from_json(chain.leaf(5));
    let with: PreEpochLeaf<SeqTypes> = from_json(&chain.leaf_with_payload(5));
    assert!(with.block_payload().is_some());
    assert_eq!(with, without);
    assert_eq!(without, with);
}

/// Each case replaces one field of the boundary leaf with the field of another leaf.
#[rstest]
#[case::view_number("view_number", 5)]
#[case::justify_qc("justify_qc", 5)]
#[case::parent_commitment("parent_commitment", 5)]
#[case::block_header("block_header", 5)]
#[case::upgrade_certificate("upgrade_certificate", 5)]
#[case::with_epoch("with_epoch", 5)]
#[case::next_drb_result("next_drb_result", TRANSITION_LEAF)]
fn pre_epoch_leaf_equality_compares_the_other_fields(#[case] field: &str, #[case] donor: usize) {
    let chain = Chain::load(UPGRADE_CHAIN);
    let base = chain.leaf(BOUNDARY_LEAF);
    let mut changed = base.clone();
    changed[field] = chain.leaf(donor)[field].clone();
    assert_ne!(
        changed[field], base[field],
        "the donor's {field} is the same"
    );

    let base: PreEpochLeaf<SeqTypes> = from_json(base);
    let changed: PreEpochLeaf<SeqTypes> = from_json(&changed);
    assert_ne!(base, changed);
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

    /// The block payload of leaf `i`.
    fn payload(&self, i: usize) -> &Value {
        &self.0[i]["block"]["payload"]
    }

    /// Leaf `i` with its block payload.
    fn leaf_with_payload(&self, i: usize) -> Value {
        let mut leaf = self.leaf(i).clone();
        leaf["block_payload"] = self.payload(i).clone();
        leaf
    }

    /// The QCs for the parent of leaf `i`, as `CertificatePair::for_parent` takes them from it.
    fn pair_for_parent(&self, i: usize) -> Value {
        let leaf = self.leaf(i);
        json!({
            "qc": leaf["justify_qc"],
            "next_epoch_qc": leaf["next_epoch_justify_qc"],
        })
    }

    /// The leaf commitment that the QC of leaf `i` signs.
    fn signed_leaf_commit(&self, i: usize) -> Commitment<Leaf2<SeqTypes>> {
        commitment(&self.qc(i)["data"]["leaf_commit"])
    }

    /// The JSON of leaf `i` and the name of its bincode vector.
    fn leaf_and_golden(&self, i: usize, with_payload: bool) -> (Value, String) {
        if with_payload {
            (self.leaf_with_payload(i), format!("leaf_{i}_with_payload"))
        } else {
            (self.leaf(i).clone(), format!("leaf_{i}"))
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Era {
    PreEpoch,
    Epoch,
}

trait HasEra {
    fn era(&self) -> Era;
}

impl HasEra for HistoricLeaf<SeqTypes> {
    fn era(&self) -> Era {
        match self {
            Self::Epoch(_) => Era::Epoch,
            Self::PreEpoch(_) => Era::PreEpoch,
        }
    }
}

impl HasEra for HistoricQuorumCertificate<SeqTypes> {
    fn era(&self) -> Era {
        match self {
            Self::Epoch(_) => Era::Epoch,
            Self::PreEpoch(_) => Era::PreEpoch,
        }
    }
}

impl HasEra for HistoricCertificatePair<SeqTypes> {
    fn era(&self) -> Era {
        match self {
            Self::Epoch(_) => Era::Epoch,
            Self::PreEpoch(_) => Era::PreEpoch,
        }
    }
}

/// The fields of a leaf as its JSON holds them, decoded without the leaf types.
struct LeafFields {
    view_number: ViewNumber,
    height: u64,
    block_header: Header,
    parent_commitment: Commitment<Leaf2<SeqTypes>>,
    upgrade_certificate: Option<UpgradeCertificate<SeqTypes>>,
    payload_commitment: VidCommitment,
}

impl LeafFields {
    fn of(leaf: &Value) -> Self {
        let block_header: Header = from_json(&leaf["block_header"]);
        Self {
            view_number: view_number(&leaf["view_number"]),
            height: block_header.height(),
            payload_commitment: block_header.payload_commitment(),
            block_header,
            parent_commitment: commitment(&leaf["parent_commitment"]),
            upgrade_certificate: from_json(&leaf["upgrade_certificate"]),
        }
    }
}

/// Checks that `commit`, the commitment of leaf `i`, is what the leaf's QC signs and what the
/// next leaf holds as its parent.
fn assert_commit_matches_chain(
    chain: &Chain,
    i: usize,
    commit: Commitment<Leaf2<SeqTypes>>,
    name: &str,
) {
    assert_eq!(
        commit,
        chain.signed_leaf_commit(i),
        "{name}: leaf {i} is not what its QC signs"
    );
    if i + 1 < chain.len() {
        let parent = commitment(&chain.leaf(i + 1)["parent_commitment"]);
        assert_eq!(
            commit,
            parent,
            "{name}: leaf {i} is not the parent of leaf {}",
            i + 1
        );
    }
}

/// Decodes `json` as `T` and checks that it encodes back to the same JSON.
fn assert_json_round_trip<T: Serialize + DeserializeOwned>(json: &Value, name: &str) -> T {
    let value: T = from_json(json);
    assert_eq!(
        serde_json::to_value(&value).unwrap(),
        *json,
        "{name}: JSON changed"
    );
    value
}

/// Checks that the value `json` decodes to encodes to the bytes of `vectors/{golden}.hex`, and
/// that those bytes decode to an equal value, which encodes to the same bytes. Returns the value
/// decoded from the bytes.
fn assert_bincode_golden<T>(json: &Value, golden: &str) -> T
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    let bytes = read_hex(golden);
    let value: T = from_json(json);
    assert_eq!(
        hex::encode(bincode::serialize(&value).unwrap()),
        hex::encode(&bytes),
        "{golden}: the value from JSON encodes differently"
    );
    // `bincode_opts` rejects trailing bytes, so the decoder must read exactly the golden bytes.
    let decoded: T = bincode_opts()
        .deserialize(&bytes)
        .unwrap_or_else(|err| panic!("{golden}: bincode does not decode: {err}"));
    assert_eq!(decoded, value, "{golden}: bincode decodes to another value");
    assert_eq!(
        hex::encode(bincode::serialize(&decoded).unwrap()),
        hex::encode(&bytes),
        "{golden}: the decoded value encodes differently"
    );
    decoded
}

/// Checks that the bincode of `today` decodes to `historic`, which encodes to the same bytes.
fn assert_bincode_like<T, H>(today: &T, historic: H, name: &str)
where
    T: Serialize,
    H: Serialize + DeserializeOwned + PartialEq + Debug,
{
    let bytes = bincode::serialize(today).unwrap();
    let decoded: H = bincode_opts()
        .deserialize(&bytes)
        .unwrap_or_else(|err| panic!("{name}: bincode does not decode: {err}"));
    assert_eq!(
        decoded, historic,
        "{name}: bincode decodes to another value"
    );
    assert_eq!(
        hex::encode(bincode::serialize(&historic).unwrap()),
        hex::encode(&bytes),
        "{name}: encoding differs"
    );
}

/// Reads the leaf and QC of `data/v3/leaf_query_data.{json,bin}` as `L` and `Q`, and checks
/// that they encode back to the same JSON and bytes. The `.bin` is a 0.3 version prefix and the
/// bincode of the leaf and then the QC.
fn assert_reads_v3_leaf_vector<L, Q>() -> (L, Q)
where
    L: Serialize + DeserializeOwned,
    Q: Serialize + DeserializeOwned,
{
    let json = read_json(&data_dir().join("v3/leaf_query_data.json"));
    assert_json_round_trip::<L>(&json["leaf"], "v3 leaf");
    assert_json_round_trip::<Q>(&json["qc"], "v3 QC");

    let bytes = std::fs::read(data_dir().join("v3/leaf_query_data.bin")).unwrap();
    let (prefix, value): (_, (L, Q)) = versions::decode(&bytes).expect("v3 bincode decodes");
    assert_eq!(prefix, version(0, 3));
    let encoded = versions::encode(prefix, &value).unwrap();
    assert_eq!(
        hex::encode(encoded),
        hex::encode(&bytes),
        "v3 bincode changed"
    );
    value
}

/// `json` with the `epoch` and `block_number` under the JSON pointer `data` set, `null` for
/// `None`.
fn set_epoch_fields(
    json: &Value,
    data: &str,
    epoch: Option<u64>,
    block_number: Option<u64>,
) -> Value {
    let mut changed = json.clone();
    let fields = changed.pointer_mut(data).expect("the data exists");
    fields["epoch"] = json!(epoch);
    fields["block_number"] = json!(block_number);
    changed
}

/// The stake table the chain started with. Before epochs, every QC was signed by it.
fn genesis_stake_table(name: &str) -> StakeTableEntries<SeqTypes> {
    let path = data_dir()
        .join("legacy-chains")
        .join(name)
        .join("network-config.json");
    let config = read_json(&path);
    let peers: Vec<PeerConfig<SeqTypes>> = from_json(&config["config"]["known_nodes_with_stake"]);
    peers.into()
}

/// Neither the vote commitment nor the signature check depends on the version.
fn upgrade_lock() -> UpgradeLock<SeqTypes> {
    UpgradeLock::new(Upgrade::trivial(FEE_VERSION))
}

fn raw<T: Committable>(commitment: Commitment<T>) -> [u8; 32] {
    commitment.into()
}

fn commitment(json: &Value) -> Commitment<Leaf2<SeqTypes>> {
    from_json(json)
}

fn view_number(json: &Value) -> ViewNumber {
    ViewNumber::new(json.as_u64().expect("a view number"))
}

fn from_json<T: DeserializeOwned>(json: &Value) -> T {
    try_from_json(json).unwrap_or_else(|err| panic!("JSON does not decode: {err}"))
}

fn try_from_json<T: DeserializeOwned>(json: &Value) -> serde_json::Result<T> {
    serde_json::from_str(&json.to_string())
}

fn read_hex(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/history_types/vectors")
        .join(format!("{name}.hex"));
    let text = std::fs::read_to_string(&path).expect("hex vector exists");
    hex::decode(text.trim()).expect("hex vector is hex")
}

fn data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data")
}

fn read_json(path: &Path) -> Value {
    let text = std::fs::read_to_string(path).expect("vector exists");
    serde_json::from_str(&text).expect("vector is JSON")
}
