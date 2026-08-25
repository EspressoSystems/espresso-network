//! Pins stake table event processing against real and synthetic event histories.
//!
//! `next_stake_table_hash` is consensus-validated, so a silent change between an L1 log and a
//! `StakeTableState` forks the network. `decaf`/`mainnet` are the histories that matter in
//! production; `synthetic` covers the variants they lack.

use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::PathBuf,
};

use alloy::rpc::types::Log;
use vbs::version::Version;
use versions::{EPOCH_REWARD_VERSION, EPOCH_VERSION, NEW_PROTOCOL_VERSION};

use super::*;
use crate::L1ClientOptions;

/// 0.3 and 0.5 select identically; 0.6 additionally requires x25519/p2p info.
const CORPORA: [&str; 3] = ["decaf", "mainnet", "synthetic"];
const VERSIONS: [Version; 3] = [EPOCH_VERSION, EPOCH_REWARD_VERSION, NEW_PROTOCOL_VERSION];

const LIVE_CORPORA: [&str; 2] = ["decaf", "mainnet"];

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

fn data_dir() -> PathBuf {
    PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../../../data")
}

fn load_logs(corpus: &str) -> Vec<Log> {
    let path = data_dir()
        .join("stake_table_history")
        .join(format!("{corpus}_logs.json"));
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {}: {e}", path.display()));
    serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("failed to parse fixture {}: {e}", path.display()))
}

/// The exhaustive `match` forces a new `StakeTableEvent` variant to fail compilation here, then
/// `stake_table_event_variants_are_covered` fails until a corpus exercises it.
fn variant_name(event: &StakeTableEvent) -> &'static str {
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

/// Kept in sync with `variant_name` by `stake_table_event_variants_are_covered`.
const ALL_VARIANT_NAMES: [&str; 13] = [
    "Register",
    "RegisterV2",
    "RegisterV3",
    "Deregister",
    "DeregisterV2",
    "Delegate",
    "Undelegate",
    "UndelegateV2",
    "KeyUpdate",
    "KeyUpdateV2",
    "CommissionUpdate",
    "X25519KeyUpdate",
    "P2pAddrUpdate",
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
    decoded_events: usize,
    stake_table_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    active_validators_commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    all_validators: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    active_validators: Option<usize>,
    /// Set at 0.6, where selection needs x25519/p2p info no real chain has registered yet.
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

fn summarize(corpus: &str, version: Version) -> Summary {
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
        event_variants: ALL_VARIANT_NAMES
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
        for version in VERSIONS {
            let summary = summarize(corpus, version);
            let name = format!(
                "stake_table_history_{corpus}_v{}_{}",
                version.major, version.minor
            );
            settings.bind(|| insta::assert_yaml_snapshot!(name, summary));
        }
    }
}

/// Real history has no V3/fast-finality events yet, which is why `synthetic` exists.
#[test]
fn stake_table_event_variants_are_covered() {
    let mut seen = HashSet::new();
    for corpus in CORPORA {
        let events = events_from_logs(load_logs(corpus)).unwrap();
        seen.extend(events.iter().map(|(_, e)| variant_name(e)));
    }

    let missing: Vec<_> = ALL_VARIANT_NAMES
        .iter()
        .filter(|n| !seen.contains(*n))
        .collect();
    assert!(
        missing.is_empty(),
        "no corpus in data/stake_table_history/ exercises these StakeTableEvent variants: \
         {missing:?}. Extend the synthetic corpus (see `regenerate_synthetic_corpus`)."
    );

    let unknown: Vec<_> = seen
        .iter()
        .filter(|n| !ALL_VARIANT_NAMES.contains(n))
        .collect();
    assert!(
        unknown.is_empty(),
        "ALL_VARIANT_NAMES is stale: {unknown:?}"
    );
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
        ALL_VARIANT_NAMES.len(),
        "the eth_getLogs filter and the StakeTableEvent variants have diverged: {} signatures for \
         {} variants",
        STAKE_TABLE_EVENT_SIGNATURES.len(),
        ALL_VARIANT_NAMES.len(),
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
    for corpus in LIVE_CORPORA {
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

/// Regenerates `data/stake_table_history/synthetic_logs.json`.
///
/// Real history has no `RegisterV3`, `X25519KeyUpdate`, `P2pAddrUpdate`, or `KeyUpdate`.
///
/// ```text
/// cargo test -p espresso-types --lib regenerate_synthetic_corpus -- --ignored
/// ```
///
/// A fatal [`StakeTableError`] aborts the whole history, so this corpus covers only the
/// silent-degradation paths and fatal ones stay in the unit tests.
#[test]
#[ignore = "regenerates a checked-in fixture"]
fn regenerate_synthetic_corpus() {
    use alloy::{
        primitives::{FixedBytes, U256},
        sol_types::SolEvent,
    };
    use rand::SeedableRng;

    use super::testing::TestValidator;

    // The identically named helpers in the sibling `tests` module are private to it.
    fn zero_g2() -> hotshot_contract_adapter::sol_types::G2PointSol {
        hotshot_contract_adapter::sol_types::G2PointSol {
            x0: U256::ZERO,
            x1: U256::ZERO,
            y0: U256::ZERO,
            y1: U256::ZERO,
        }
    }
    fn zero_g1() -> hotshot_contract_adapter::sol_types::G1PointSol {
        hotshot_contract_adapter::sol_types::G1PointSol {
            x: U256::ZERO,
            y: U256::ZERO,
        }
    }

    // Fixed seed: the fixture must be byte-reproducible on any machine.
    let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(0xE5B7_0000_5A1E_7AB1);
    let contract = Address::repeat_byte(0x11);
    let v: Vec<TestValidator> = (0..7)
        .map(|_| TestValidator::random_with(&mut rng))
        .collect();
    let delegator: Vec<Address> = (0..6)
        .map(|i| Address::repeat_byte(0xD0 + i as u8))
        .collect();
    let stake = |n: u64| U256::from(n) * U256::from(10u64).pow(U256::from(18));

    // Registers unauthenticated, so it never enters the active set however much stake it holds.
    let mut unparsable_bls = ValidatorRegistered::from(&v[4]);
    unparsable_bls.blsVk = zero_g2();

    // `authenticate` fails, but the parsed keys are kept.
    let mut bad_sig = ValidatorRegisteredV2::from(&v[5]);
    bad_sig.blsSig = zero_g1().into();

    // Reuses v[0]'s Schnorr key. Tolerated: the V1 contract did not enforce uniqueness.
    let mut dup_schnorr = ValidatorRegistered::from(&v[6]);
    dup_schnorr.schnorrVk = v[0].schnorr_vk;

    let events: Vec<alloy::primitives::LogData> = vec![
        ValidatorRegistered::from(&v[0]).encode_log_data(),
        Delegated {
            delegator: delegator[0],
            validator: v[0].account,
            amount: stake(500),
        }
        .encode_log_data(),
        ConsensusKeysUpdated::from(&v[0].randomize_keys_with(&mut rng)).encode_log_data(),
        ValidatorRegistered::from(&v[1]).encode_log_data(),
        Delegated {
            delegator: delegator[1],
            validator: v[1].account,
            amount: stake(300),
        }
        .encode_log_data(),
        Undelegated {
            delegator: delegator[1],
            validator: v[1].account,
            amount: stake(100),
        }
        .encode_log_data(),
        ValidatorRegisteredV2::from(&v[2]).encode_log_data(),
        Delegated {
            delegator: delegator[2],
            validator: v[2].account,
            amount: stake(700),
        }
        .encode_log_data(),
        ConsensusKeysUpdatedV2::from(&v[2].randomize_keys_with(&mut rng)).encode_log_data(),
        CommissionUpdated {
            validator: v[2].account,
            timestamp: U256::from(1_700_000_000u64),
            oldCommission: v[2].commission,
            newCommission: COMMISSION_BASIS_POINTS,
        }
        .encode_log_data(),
        UndelegatedV2 {
            delegator: delegator[2],
            validator: v[2].account,
            undelegationId: 1,
            amount: stake(200),
            unlocksAt: U256::from(1_700_086_400u64),
        }
        .encode_log_data(),
        // The only validator eligible at 0.6.
        ValidatorRegisteredV3::from(&v[3]).encode_log_data(),
        Delegated {
            delegator: delegator[3],
            validator: v[3].account,
            amount: stake(900),
        }
        .encode_log_data(),
        X25519KeyUpdated {
            validator: v[3].account,
            x25519Key: FixedBytes([0x42u8; 32]),
        }
        .encode_log_data(),
        P2pAddrUpdated {
            validator: v[3].account,
            p2pAddr: "validator-3.example.com:5000".to_string(),
        }
        .encode_log_data(),
        // Both degrade to `None` without an error, silently dropping the validator at 0.6.
        X25519KeyUpdated {
            validator: v[0].account,
            x25519Key: FixedBytes([0u8; 32]),
        }
        .encode_log_data(),
        P2pAddrUpdated {
            validator: v[0].account,
            p2pAddr: "not a socket address".to_string(),
        }
        .encode_log_data(),
        unparsable_bls.encode_log_data(),
        Delegated {
            delegator: delegator[4],
            validator: v[4].account,
            amount: stake(1_000),
        }
        .encode_log_data(),
        bad_sig.encode_log_data(),
        Delegated {
            delegator: delegator[5],
            validator: v[5].account,
            amount: stake(1_100),
        }
        .encode_log_data(),
        dup_schnorr.encode_log_data(),
        ValidatorExit {
            validator: v[1].account,
        }
        .encode_log_data(),
        ValidatorExitV2 {
            validator: v[2].account,
            unlocksAt: U256::from(1_700_172_800u64),
        }
        .encode_log_data(),
    ];

    // One event per block; the real-history corpora already cover multiple logs per block.
    let logs: Vec<Log> = events
        .iter()
        .enumerate()
        .map(|(i, event)| Log {
            inner: alloy::primitives::Log {
                address: contract,
                data: event.clone(),
            },
            block_hash: None,
            block_number: Some(1_000 + i as u64),
            block_timestamp: None,
            transaction_hash: None,
            transaction_index: None,
            log_index: Some(i as u64),
            removed: false,
        })
        .collect();

    // Fail loudly rather than write a fixture the pin test cannot load.
    let decoded = events_from_logs(logs.clone()).expect("synthetic logs round-trip through decode");
    assert_eq!(
        decoded.len(),
        logs.len(),
        "an event was dropped in decode; every synthetic event should survive validation"
    );
    ValidatorSet::from_l1_events(decoded.iter().map(|(_, e)| e.clone()), NEW_PROTOCOL_VERSION)
        .expect("synthetic history applies without a fatal error");

    let covered: HashSet<_> = decoded.iter().map(|(_, e)| variant_name(e)).collect();
    let uncovered: Vec<_> = ALL_VARIANT_NAMES
        .iter()
        .filter(|n| !covered.contains(*n))
        .collect();
    assert!(
        uncovered.is_empty(),
        "the synthetic corpus is meant to cover every variant; missing {uncovered:?}"
    );

    let path = data_dir()
        .join("stake_table_history")
        .join("synthetic_logs.json");
    let body = logs
        .iter()
        .map(|l| serde_json::to_string(l).unwrap())
        .collect::<Vec<_>>()
        .join(",\n");
    fs::write(&path, format!("[\n{body}\n]\n")).unwrap();
    println!("wrote {} logs to {}", logs.len(), path.display());
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
