//! Pins stake table event processing against the histories decaf and mainnet actually emitted.
//!
//! `next_stake_table_hash` is consensus-validated, so a silent change between an L1 log and a
//! `StakeTableState` forks the network.
//!
//! Every snapshot here is a fork signal: a diff means a node replaying that chain from genesis
//! would now compute a different `stake_table_hash`, and nothing but a fixture refresh should
//! produce one. What the pin does *not* constrain is recorded in [`REACHABILITY`], which is where
//! a refactor should look first.

use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::PathBuf,
};

use alloy::rpc::types::Log;
use vbs::version::Version;
use versions::parse_version;

use super::*;
use crate::L1ClientOptions;

/// The chains whose replay `next_stake_table_hash` depends on.
const CORPORA: [&str; 2] = ["decaf", "mainnet"];

/// The protocol version a corpus is summarized at.
///
/// Only [`select_active_validator_set`] reads the version, and only to decide whether x25519/p2p
/// info is required. Pinning a real history at a version its network never ran states nothing
/// about that network, so each live corpus uses `base_version` from its genesis file and a
/// network upgrade moves the snapshot. `test_select_version_boundary` covers the version boundary
/// itself.
fn corpus_version(network: &str) -> Version {
    let path = data_dir().join("genesis").join(format!("{network}.toml"));
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read genesis {}: {e}", path.display()));
    let genesis: toml::Value = toml::from_str(&raw)
        .unwrap_or_else(|e| panic!("failed to parse genesis {}: {e}", path.display()));
    let version = genesis["base_version"]
        .as_str()
        .expect("base_version is a string");
    parse_version(version).expect("base_version parses")
}

/// A single terminal hash says *that* something changed; the ladder narrows it to a window.
///
/// Fixed, not derived from corpus length, so appending events leaves every existing checkpoint at
/// the same index with the same hash. An append-only fixture update is then a purely additive
/// snapshot diff, and any changed history line is visibly wrong. `regenerate_history_fixtures`
/// depends on this.
const LADDER_STRIDE: usize = 32;

/// The short retry budget keeps a rate-limited or dead endpoint from stalling for 20 minutes.
fn connect_archive_l1(url: &str) -> L1Client {
    L1ClientOptions {
        l1_events_max_retry_duration: Duration::from_secs(120),
        l1_events_max_block_range: 45_000,
        l1_retry_delay: Duration::from_secs(2),
        ..Default::default()
    }
    .connect(vec![url.parse().expect("archive RPC URL parses")])
    .expect("unable to construct l1 client")
}

pub(super) fn data_dir() -> PathBuf {
    PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../../../data")
}

pub(super) fn load_logs(corpus: &str) -> Vec<Log> {
    let path = data_dir()
        .join("stake_table_history")
        .join(format!("{corpus}_logs.json"));
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {}: {e}", path.display()));
    let mut logs: Vec<Log> = serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("failed to parse fixture {}: {e}", path.display()));
    drop_block_timestamps(&mut logs);
    logs
}

/// `blockTimestamp` is optional in an `eth_getLogs` response and endpoints disagree on returning
/// it, so a fixture carries whichever answer its RPC happened to give. Nothing between a log and
/// a `StakeTableState` reads it. Dropping it on both load and fetch makes the fixture, and the
/// prefix check in [`regenerate_history_fixtures`], independent of that.
fn drop_block_timestamps(logs: &mut [Log]) {
    for log in logs {
        log.block_timestamp = None;
    }
}

/// The exhaustive `match` forces a new `StakeTableEvent` variant to fail compilation here, then
/// `stake_table_event_variants_are_covered` fails until a corpus exercises it.
pub(super) fn variant_name(event: &StakeTableEvent) -> &'static str {
    match event {
        StakeTableEvent::Register(_) => "Register",
        StakeTableEvent::RegisterV2(_) => "RegisterV2",
        StakeTableEvent::RegisterV3(_) => "RegisterV3",
        StakeTableEvent::Deregister(_) => "Deregister",
        StakeTableEvent::DeregisterV2(_) => "DeregisterV2",
        StakeTableEvent::Delegate(_) => "Delegate",
        StakeTableEvent::Undelegate(_) => "Undelegate",
        StakeTableEvent::UndelegateV2(_) => "UndelegateV2",
        StakeTableEvent::KeyUpdate(_) => "KeyUpdate",
        StakeTableEvent::KeyUpdateV2(_) => "KeyUpdateV2",
        StakeTableEvent::CommissionUpdate(_) => "CommissionUpdate",
        StakeTableEvent::X25519KeyUpdate(_) => "X25519KeyUpdate",
        StakeTableEvent::P2pAddrUpdate(_) => "P2pAddrUpdate",
    }
}

/// Every name [`variant_name`] can return. Adding a `StakeTableEvent` variant breaks its
/// exhaustive match, then this array's length, so the two cannot drift apart silently.
fn all_variant_names() -> [&'static str; REACHABILITY.len()] {
    REACHABILITY.map(|(name, _)| name)
}

/// What a change to a variant's processing can break.
///
/// `stake_table_history_pin` freezes the processing of every log decaf and mainnet have emitted:
/// a node syncing from genesis replays them and must reach the same `stake_table_hash`. Which
/// variants that actually constrains is not visible from the code, so it is recorded here and
/// checked against the fixtures and the contract by [`event_reachability_matches_history`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Reachability {
    /// In the corpora, and the V3 contract still emits it. The fixtures are a lower bound on the
    /// shapes this code will see, so a refactor also has to hold for inputs no fixture contains.
    Live,
    /// In the corpora, but unreachable on the V3 contract: the V1/V2 entry point reverts or was
    /// overridden to emit the V2 event. The fixtures therefore hold every log of this variant
    /// that will ever exist, so a refactor that keeps the pin green cannot break it.
    Historical,
    /// Absent from both corpora and unreachable on the V3 contract. Nothing replays it and
    /// nothing can emit it, so its processing may change or be deleted without forking a chain.
    /// Nothing here pins it.
    Dead,
}

/// Unreachability is the deprecated entry points: `registerValidator` and `updateConsensusKeys`
/// revert (`StakeTableV2.sol:902`, `StakeTableV2.sol:914`), `registerValidatorV2` reverts
/// (`StakeTableV3.sol:208`), and `undelegate`/`deregisterValidator` are overridden to emit the V2
/// events (`StakeTableV2.sol:555`, `StakeTableV2.sol:591`).
pub(super) const REACHABILITY: [(&str, Reachability); 13] = [
    ("Register", Reachability::Historical),
    ("RegisterV2", Reachability::Historical),
    ("RegisterV3", Reachability::Live),
    ("Deregister", Reachability::Historical),
    ("DeregisterV2", Reachability::Live),
    ("Delegate", Reachability::Live),
    ("Undelegate", Reachability::Historical),
    ("UndelegateV2", Reachability::Live),
    ("KeyUpdate", Reachability::Dead),
    ("KeyUpdateV2", Reachability::Live),
    ("CommissionUpdate", Reachability::Live),
    ("X25519KeyUpdate", Reachability::Live),
    ("P2pAddrUpdate", Reachability::Live),
];

/// One checkpoint in the hash ladder.
#[derive(serde::Serialize)]
struct Checkpoint {
    event: usize,
    block: u64,
    log_index: u64,
    variant: &'static str,
    commit: String,
}

#[derive(serde::Serialize)]
struct Summary {
    corpus: String,
    protocol_version: String,
    logs: usize,
    /// Below `logs` when a log decodes but fails validation. decaf carries one
    /// `ConsensusKeysUpdatedV2` (block 11310213) whose signature does not authenticate, so this
    /// pins the silent-drop path against a real event.
    decoded_events: usize,
    stake_table_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    active_validators_commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    all_validators: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    active_validators: Option<usize>,
    /// Selection returning no eligible validator is a live network halting, so this being set on
    /// a live corpus is a failure the snapshot records rather than hides.
    #[serde(skip_serializing_if = "Option::is_none")]
    selection_error: Option<String>,
    event_variants: BTreeMap<&'static str, usize>,
    ladder_stride: usize,
    hash_ladder: Vec<Checkpoint>,
}

/// Hash after every `LADDER_STRIDE`th event, and after the last.
///
/// Replays through `apply_event` rather than `validators_from_l1_events` so a checkpoint can be
/// taken between events; the error handling mirrors that function.
fn hash_ladder(events: &[(EventKey, StakeTableEvent)]) -> (usize, Vec<Checkpoint>) {
    // Corpora smaller than one stride get a checkpoint per event; they are hand-built and never
    // grow on their own.
    let stride = if events.len() <= LADDER_STRIDE {
        1
    } else {
        LADDER_STRIDE
    };
    let mut state = StakeTableState::default();
    let mut ladder = vec![];

    for (idx, ((block, log_index), event)) in events.iter().enumerate() {
        match state.apply_event(event.clone()) {
            Ok(Ok(())) | Ok(Err(_)) => {},
            Err(err) => panic!("fatal error applying event {idx} ({event:?}): {err}"),
        }
        if idx % stride == stride - 1 || idx == events.len() - 1 {
            ladder.push(Checkpoint {
                event: idx,
                block: *block,
                log_index: *log_index,
                variant: variant_name(event),
                commit: state.commit().to_string(),
            });
        }
    }
    (stride, ladder)
}

fn summarize(corpus: &str) -> Summary {
    let version = corpus_version(corpus);
    let logs = load_logs(corpus);
    let log_count = logs.len();
    let events = events_from_logs(logs).expect("fixture logs decode, validate, and sort");

    let (all_validators, hash) = validators_from_l1_events(events.iter().map(|(_, e)| e.clone()))
        .expect("fixture history applies without a fatal error");
    let selected = select_active_validator_set(&all_validators, version);

    let active_commit = |active: &AuthenticatedValidatorMap| {
        StakeTableState::new(
            to_registered_validator_map(active),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        )
        .commit()
        .to_string()
    };

    let (stride, ladder) = hash_ladder(&events);
    Summary {
        corpus: corpus.into(),
        protocol_version: version.to_string(),
        logs: log_count,
        decoded_events: events.len(),
        stake_table_hash: hash.to_string(),
        active_validators_commit: selected.as_ref().ok().map(active_commit),
        all_validators: selected.as_ref().ok().map(|_| all_validators.len()),
        active_validators: selected.as_ref().ok().map(|a| a.len()),
        selection_error: selected.as_ref().err().map(|e| e.to_string()),
        event_variants: all_variant_names()
            .into_iter()
            .map(|name| {
                (
                    name,
                    events
                        .iter()
                        .filter(|(_, e)| variant_name(e) == name)
                        .count(),
                )
            })
            .filter(|(_, count)| *count > 0)
            .collect(),
        ladder_stride: stride,
        hash_ladder: ladder,
    }
}

/// The pin. Any change to decode, validate, sort, apply, commit, or selection moves a snapshot.
#[test]
fn stake_table_history_pin() {
    let mut settings = insta::Settings::clone_current();
    settings.set_snapshot_path(data_dir().join("insta_snapshots"));
    settings.set_prepend_module_to_snapshot(false);

    for corpus in CORPORA {
        let summary = summarize(corpus);
        let name = format!("stake_table_history_{corpus}");
        settings.bind(|| insta::assert_yaml_snapshot!(name, summary));
    }
}

/// Keeps [`REACHABILITY`] honest, so a refactor can trust it.
///
/// The observed half is derived from the live corpora rather than declared, so a `Dead` variant
/// that turns up in real history, or a `Live`/`Historical` one that is missing, fails here
/// instead of quietly telling someone a frozen path is free to change.
///
/// The unreachable half cannot be derived from logs (absence of a log is not proof the contract
/// cannot emit it), so it rests on the deprecated overrides cited on [`REACHABILITY`]. A contract
/// upgrade that re-enables one of those entry points has to move a variant back to `Live` by
/// hand.
#[test]
fn event_reachability_matches_history() {
    let declared: HashSet<&str> = REACHABILITY.iter().map(|(name, _)| *name).collect();

    let observed: HashSet<&'static str> = CORPORA
        .iter()
        .flat_map(|corpus| events_from_logs(load_logs(corpus)).unwrap())
        .map(|(_, event)| variant_name(&event))
        .collect();

    let unknown: Vec<_> = observed.difference(&declared).collect();
    assert!(
        unknown.is_empty(),
        "REACHABILITY does not classify {unknown:?}, which a live corpus emits"
    );

    for (name, reachability) in REACHABILITY {
        match reachability {
            Reachability::Live | Reachability::Historical => assert!(
                observed.contains(name),
                "{name} is declared {reachability:?}, so decaf or mainnet has emitted it, but \
                 neither corpus contains one. Either the fixtures are wrong or {name} is Dead and \
                 its processing is free to change."
            ),
            Reachability::Dead => assert!(
                !observed.contains(name),
                "{name} is declared Dead, so nothing replays it and its processing is free to \
                 change, but a live corpus now contains one. Reclassify it before anyone \
                 refactors on that assumption."
            ),
        }
    }
}

/// The fixtures cannot pin the sort: `eth_getLogs` already returns chain order, so a weaker key
/// looks identical on them. Two events in one block applied in the wrong order change the hash.
#[test]
fn events_from_logs_sorts_by_block_and_log_index() {
    // decaf has multiple logs in one block, which is what makes log_index matter.
    let logs = load_logs("decaf");
    let expected = events_from_logs(logs.clone()).unwrap();
    let keys: Vec<EventKey> = expected.iter().map(|(k, _)| *k).collect();
    assert!(
        keys.windows(2).all(|w| w[0] < w[1]),
        "fixture keys are not strictly increasing"
    );
    assert!(
        keys.windows(2).any(|w| w[0].0 == w[1].0),
        "fixture has no block with two logs, so this test would not exercise log_index"
    );

    // Destroys chain order without needing an RNG.
    let mut rotated = logs.clone();
    rotated.rotate_left(logs.len() / 3);
    let mut reversed = logs;
    reversed.reverse();

    for (label, shuffled) in [("rotated", rotated), ("reversed", reversed)] {
        let actual = events_from_logs(shuffled).unwrap();
        assert_eq!(
            actual.iter().map(|(k, _)| *k).collect::<Vec<_>>(),
            keys,
            "{label} logs did not sort back into (block, log_index) order"
        );
        assert_eq!(
            actual, expected,
            "{label} logs produced a different event stream"
        );
    }
}

/// The fixtures bypass the `eth_getLogs` filter, so a signature dropped from it would go
/// unnoticed until the variant was never fetched in production.
#[test]
fn stake_table_event_signatures_cover_all_variants() {
    assert_eq!(
        STAKE_TABLE_EVENT_SIGNATURES.len(),
        REACHABILITY.len(),
        "the eth_getLogs filter and the StakeTableEvent variants have diverged: {} signatures for \
         {} variants",
        STAKE_TABLE_EVENT_SIGNATURES.len(),
        REACHABILITY.len(),
    );

    let unique: HashSet<_> = STAKE_TABLE_EVENT_SIGNATURES.iter().collect();
    assert_eq!(
        unique.len(),
        STAKE_TABLE_EVENT_SIGNATURES.len(),
        "duplicate signature in STAKE_TABLE_EVENT_SIGNATURES"
    );
}

/// Extends `data/stake_table_history/{decaf,mainnet}_logs.json` and `manifest.json` to the
/// current finalized block.
///
/// ```text
/// cargo test -p espresso-types --lib regenerate_history_fixtures -- --ignored --nocapture
/// ```
///
/// History only ever grows, so the existing fixture must be a prefix of what the chain now
/// returns. Anything else is a reorg or a redeployed contract, and this aborts rather than
/// rewrite it: the scheduled job then fails instead of opening a PR that quietly restates
/// history. Combined with a ladder whose checkpoints do not shift (see [`LADDER_STRIDE`]), an
/// honest update is a purely additive snapshot diff.
///
/// A node that has pruned logs for part of the range answers with an empty result rather than an
/// error, which the `first_event_block` assertion below turns into a failure.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "regenerates checked-in fixtures from an archive L1 RPC"]
async fn regenerate_history_fixtures() {
    use alloy::{eips::BlockNumberOrTag, providers::Provider, rpc::types::Filter};

    let mut manifests: BTreeMap<String, CorpusManifest> = BTreeMap::new();
    for corpus in CORPORA {
        let url = archive_rpc(corpus);
        let mut manifest = load_manifest(corpus);
        let previous = load_logs(corpus);

        let l1 = connect_archive_l1(&url);

        // Finalized, not head: the fixture must describe blocks that cannot be reorged away.
        let finalized = l1
            .provider
            .get_block_by_number(BlockNumberOrTag::Finalized)
            .await
            .expect("fetch finalized block")
            .expect("finalized block exists")
            .header
            .number;
        assert!(
            finalized >= manifest.to_block,
            "{corpus}: finalized block {finalized} is behind the fixture's to_block {}",
            manifest.to_block
        );
        manifest.to_block = finalized;

        // The production filter, so the fixture holds exactly the logs a node would see.
        let mut logs: Vec<Log> = vec![];
        let mut from = manifest.from_block;
        while from <= manifest.to_block {
            let to = (from + 44_999).min(manifest.to_block);
            let filter = Filter::new()
                .events(STAKE_TABLE_EVENT_SIGNATURES)
                .address(manifest.stake_table_contract)
                .from_block(from)
                .to_block(to);
            let provider = l1.provider.clone();
            logs.extend(
                retry(
                    Duration::from_secs(2),
                    Duration::from_secs(120),
                    "fixture regeneration get_logs",
                    move || {
                        let provider = provider.clone();
                        let filter = filter.clone();
                        Box::pin(async move { provider.get_logs(&filter).await })
                    },
                )
                .await,
            );
            from = to + 1;
        }

        drop_block_timestamps(&mut logs);
        logs.sort_by_key(|l| (l.block_number, l.log_index));
        let first = logs.first().and_then(|l| l.block_number);
        assert_eq!(
            first,
            Some(manifest.first_event_block),
            "{corpus}: {url} does not serve the start of the range (first event {first:?}, \
             expected {}). It is pruning history; use a real archive node rather than committing \
             truncated history.",
            manifest.first_event_block,
        );

        assert!(
            logs.len() >= previous.len() && logs[..previous.len()] == previous[..],
            "{corpus}: the chain no longer returns the checked-in fixture as a prefix ({} logs \
             now, {} before). History does not change, so this is a reorg or a redeployed \
             contract and needs a human.",
            logs.len(),
            previous.len(),
        );

        manifest.log_count = logs.len();
        manifest.last_event_block = logs
            .last()
            .and_then(|l| l.block_number)
            .expect("fixture has at least one log");

        // One log per line, so an update reviews as "N logs appended".
        let body = logs
            .iter()
            .map(|l| serde_json::to_string(l).unwrap())
            .collect::<Vec<_>>()
            .join(",\n");
        let path = data_dir()
            .join("stake_table_history")
            .join(format!("{corpus}_logs.json"));
        fs::write(&path, format!("[\n{body}\n]\n")).unwrap();
        println!(
            "{corpus}: {} logs ({} new) through block {}",
            logs.len(),
            logs.len() - previous.len(),
            manifest.to_block,
        );
        manifests.insert(corpus.to_string(), manifest);
    }

    let mut json = serde_json::to_string_pretty(&manifests).unwrap();
    json.push('\n');
    fs::write(manifest_path(), json).unwrap();
}

/// Needs log retention back to the fixture's first block, not historical state: the only calls
/// here are `eth_getLogs` and `eth_getBlockByNumber`. Most public endpoints gate log queries that
/// old; Tenderly does not, and needs no API key. It rate-limits bursts (`-32005`), hence
/// [`retry`].
fn archive_rpc(corpus: &str) -> String {
    let (env, default) = match corpus {
        "decaf" => (
            "ESPRESSO_L1_ARCHIVE_RPC_SEPOLIA",
            "https://sepolia.gateway.tenderly.co",
        ),
        "mainnet" => (
            "ESPRESSO_L1_ARCHIVE_RPC_MAINNET",
            "https://mainnet.gateway.tenderly.co",
        ),
        other => panic!("no archive RPC configured for corpus {other}"),
    };
    std::env::var(env).unwrap_or_else(|_| default.to_string())
}

/// Manifest entry for one corpus, from `data/stake_table_history/manifest.json`.
///
/// Fields are declared alphabetically so a rewritten manifest diffs cleanly against the
/// checked-in one.
#[derive(serde::Serialize, serde::Deserialize)]
struct CorpusManifest {
    chain_id: u64,
    first_event_block: u64,
    from_block: u64,
    last_event_block: u64,
    log_count: usize,
    network: String,
    stake_table_contract: Address,
    to_block: u64,
}

fn manifest_path() -> PathBuf {
    data_dir().join("stake_table_history").join("manifest.json")
}

fn load_manifest(corpus: &str) -> CorpusManifest {
    let path = manifest_path();
    let raw = fs::read_to_string(&path).unwrap();
    let all: BTreeMap<String, CorpusManifest> = serde_json::from_str(&raw).unwrap();
    all.into_iter()
        .find(|(k, _)| k == corpus)
        .unwrap_or_else(|| panic!("manifest has no entry for {corpus}"))
        .1
}
