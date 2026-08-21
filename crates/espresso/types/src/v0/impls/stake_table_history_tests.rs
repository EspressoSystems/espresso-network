//! Pins stake table event processing against real and synthetic event histories.
//!
//! `next_stake_table_hash` is a consensus-validated header field (see
//! `ValidatedState::validate_next_stake_table_hash`), so any silent change to the pipeline that
//! turns L1 logs into a `StakeTableState` forks the network. These tests drive the whole pipeline
//! -- decode, validate, sort, apply, commit, select -- from raw logs checked into
//! `data/stake_table_history/` and pin the results with `insta`.
//!
//! The corpora are complementary: `decaf`/`mainnet` are the histories that actually matter in
//! production, and `synthetic` covers the event variants those histories do not contain yet.

use std::{collections::HashSet, fs, path::PathBuf};

use alloy::rpc::types::Log;
use vbs::version::Version;
use versions::{EPOCH_REWARD_VERSION, EPOCH_VERSION, NEW_PROTOCOL_VERSION};

use super::*;
use crate::L1ClientOptions;

/// Corpora under `data/stake_table_history/`, pinned at every protocol version whose selection
/// rules differ. 0.3 and 0.5 select identically; 0.6 additionally requires x25519/p2p info.
const CORPORA: [&str; 3] = ["decaf", "mainnet", "synthetic"];
const VERSIONS: [Version; 3] = [EPOCH_VERSION, EPOCH_REWARD_VERSION, NEW_PROTOCOL_VERSION];

/// Number of hash checkpoints in the incremental ladder. A single terminal hash says *that*
/// something changed; the ladder narrows it to a window of events.
const LADDER_CHECKPOINTS: usize = 48;

fn data_dir() -> PathBuf {
    PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../../../data")
}

pub(crate) fn load_logs(corpus: &str) -> Vec<Log> {
    let path = data_dir()
        .join("stake_table_history")
        .join(format!("{corpus}_logs.json"));
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {}: {e}", path.display()));
    serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("failed to parse fixture {}: {e}", path.display()))
}

/// Stable display name for each event variant.
///
/// The exhaustive `match` is the forcing function: adding a `StakeTableEvent` variant fails to
/// compile here, and then `stake_table_event_variants_are_covered` fails until a corpus exercises
/// it.
pub(crate) fn variant_name(event: &StakeTableEvent) -> &'static str {
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

/// Every variant `variant_name` knows about. Kept in sync with it by
/// `stake_table_event_variants_are_covered`.
pub(crate) const ALL_VARIANT_NAMES: [&str; 13] = [
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

/// Hash after each event, sampled to at most `LADDER_CHECKPOINTS` lines.
///
/// Replays through `apply_event` directly rather than `validators_from_l1_events` so a checkpoint
/// can be taken between events; the error handling mirrors that function exactly.
fn hash_ladder(events: &[(EventKey, StakeTableEvent)]) -> String {
    let stride = events.len().div_ceil(LADDER_CHECKPOINTS).max(1);
    let mut state = StakeTableState::default();
    let mut out = format!("(every {stride} event(s), and always the last)\n");

    for (idx, ((block, log_index), event)) in events.iter().enumerate() {
        match state.apply_event(event.clone()) {
            Ok(Ok(())) | Ok(Err(_)) => {},
            Err(err) => panic!("fatal error applying event {idx} ({event:?}): {err}"),
        }
        if idx % stride == stride - 1 || idx == events.len() - 1 {
            out += &format!(
                "{idx:5}  block {block:>9}  log {log_index:>4}  {:<17} {}\n",
                variant_name(event),
                state.commit()
            );
        }
    }
    out
}

fn summarize(corpus: &str, version: Version) -> String {
    let logs = load_logs(corpus);
    let log_count = logs.len();
    let events = events_from_logs(logs).expect("fixture logs decode, validate, and sort");

    // At `NEW_PROTOCOL_VERSION`, selection requires x25519/p2p info, which no validator on a
    // real chain has registered yet -- so this legitimately fails with `NoValidValidators`.
    // Pinning the error is the point: it records that the network is not yet upgradeable at 0.6,
    // and the snapshot moves the moment the first V3 registration lands.
    let (hash, selection) = {
        let (all_validators, hash) =
            validators_from_l1_events(events.iter().map(|(_, e)| e.clone()))
                .expect("fixture history applies without a fatal error");
        let selection = select_active_validator_set(&all_validators, version)
            .map(|active| (all_validators.len(), active));
        (hash, selection)
    };

    // Variant histogram in a fixed order, so the snapshot does not churn on map iteration order.
    let mut histogram = String::new();
    for name in ALL_VARIANT_NAMES {
        let count = events
            .iter()
            .filter(|(_, e)| variant_name(e) == name)
            .count();
        if count > 0 {
            histogram += &format!("  {count:5}  {name}\n");
        }
    }

    let mut summary = String::new();
    summary += &format!("corpus: {corpus}\n");
    summary += &format!("protocol_version: {version}\n");
    summary += &format!("logs: {log_count}\n");
    summary += &format!("decoded_events: {}\n", events.len());
    summary += &format!("stake_table_hash: {hash}\n");

    match &selection {
        Ok((all, active)) => {
            let active_commit = StakeTableState::new(
                to_registered_validator_map(active),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
            )
            .commit();
            summary += &format!("active_validators_commit: {active_commit}\n");
            summary += &format!("all_validators: {all}\n");
            summary += &format!("active_validators: {}\n", active.len());
        },
        Err(e) => {
            summary += &format!("active_validator_selection_error: {e}\n");
        },
    }

    summary += &format!("\nevent_variants:\n{histogram}");
    summary += &format!("\nhash_ladder:\n{}", hash_ladder(&events));
    summary
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
            settings.bind(|| insta::assert_snapshot!(name, summary));
        }
    }
}

/// Every `StakeTableEvent` variant is exercised by at least one corpus.
///
/// Real history does not contain the V3/fast-finality events yet, which is why the `synthetic`
/// corpus exists. Together the corpora must cover all of `variant_name`.
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

/// `events_from_logs` orders by `(block_number, log_index)` whatever order the logs arrive in.
///
/// The history fixtures cannot pin this: `eth_getLogs` already returns logs in chain order and
/// `sort_by_key` is stable, so a weaker sort key -- by block alone -- produces identical output
/// for them. Feeding the same logs shuffled is what actually exercises the comparator, and
/// ordering is consensus-critical: two events in one block applied in the wrong order can produce
/// a different stake table hash.
#[test]
fn events_from_logs_sorts_by_block_and_log_index() {
    // decaf has multiple logs in a single block, which is what makes log_index matter.
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

    // A fixed rotation and a reversal: both destroy chain order without needing an RNG.
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

/// `STAKE_TABLE_EVENT_SIGNATURES` -- the `eth_getLogs` filter -- must admit every log that can
/// decode into a `StakeTableEvent`.
///
/// The fixtures cannot catch a signature dropped from the filter, because they bypass the filter
/// entirely. This closes that gap: a variant reachable by `events_from_logs` but absent from the
/// filter would never be fetched in production.
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

/// Regenerates `data/stake_table_history/{decaf,mainnet}_logs.json` and `manifest.json`.
///
/// ```text
/// export ESPRESSO_L1_ARCHIVE_RPC_SEPOLIA=... ESPRESSO_L1_ARCHIVE_RPC_MAINNET=...
/// cargo test -p espresso-types --lib regenerate_history_fixtures -- --ignored --nocapture
/// ```
///
/// Requires *archive* endpoints. A pruning node answers `eth_getLogs` for a range it no longer
/// holds with an empty result rather than an error, which would commit silently truncated
/// history; the `first_event_block` assertion below is what turns that into a loud failure.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "regenerates checked-in fixtures from an archive L1 RPC"]
async fn regenerate_history_fixtures() {
    use alloy::{providers::Provider, rpc::types::Filter};

    for corpus in ["decaf", "mainnet"] {
        let env = archive_rpc_env(corpus);
        let url = std::env::var(env).unwrap_or_else(|_| panic!("{env} is not set"));
        let manifest = load_manifest(corpus);

        let l1 = L1ClientOptions {
            l1_events_max_retry_duration: Duration::from_secs(120),
            l1_events_max_block_range: 45_000,
            l1_retry_delay: Duration::from_secs(2),
            ..Default::default()
        }
        .connect(vec![url.parse().expect("archive RPC URL parses")])
        .expect("unable to construct l1 client");

        // Mirrors the production filter exactly, so the fixture holds precisely the logs a node
        // would see.
        let mut logs: Vec<Log> = vec![];
        let mut from = manifest.from_block;
        while from <= manifest.to_block {
            let to = (from + 44_999).min(manifest.to_block);
            let filter = Filter::new()
                .events(STAKE_TABLE_EVENT_SIGNATURES)
                .address(manifest.stake_table_contract)
                .from_block(from)
                .to_block(to);
            logs.extend(l1.provider.get_logs(&filter).await.expect("get_logs"));
            from = to + 1;
        }

        logs.sort_by_key(|l| (l.block_number, l.log_index));
        let first = logs.first().and_then(|l| l.block_number);
        assert_eq!(
            first,
            Some(manifest.first_event_block),
            "{corpus}: {env} does not serve the start of the range (first event {first:?}, \
             expected {}). It is pruning history; use a real archive node rather than committing \
             truncated history.",
            manifest.first_event_block,
        );

        // One log per line: compact enough to keep the repo small, still line-diffable, so a
        // regenerated fixture reviews as "N logs appended" rather than one opaque blob.
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
            "{corpus}: wrote {} logs ({}..{}) to {}",
            logs.len(),
            manifest.from_block,
            manifest.to_block,
            path.display()
        );
        println!("  update manifest.json log_count to {}", logs.len());
    }
}

/// Regenerates `data/stake_table_history/synthetic_logs.json`.
///
/// Real decaf/mainnet history does not contain the V3 / fast-finality events (`RegisterV3`,
/// `X25519KeyUpdate`, `P2pAddrUpdate`) or `KeyUpdate`, so without this corpus most of
/// `apply_event` is unpinned. Run with:
///
/// ```text
/// cargo test -p espresso-types --lib regenerate_synthetic_corpus -- --ignored
/// ```
///
/// Every event here must apply cleanly or produce a *tolerated* error: a fatal
/// [`StakeTableError`] aborts the whole history, so fatal paths stay in the unit tests above.
/// What this corpus does cover is the silent-degradation paths, which no commitment would
/// otherwise pin: unauthenticated registration, an unparsable BLS key, a rejected x25519 key, an
/// unparsable p2p address, and a duplicate Schnorr key.
#[test]
#[ignore = "regenerates a checked-in fixture"]
fn regenerate_synthetic_corpus() {
    use alloy::{
        primitives::{FixedBytes, U256},
        sol_types::SolEvent,
    };
    use rand::SeedableRng;

    use super::testing::TestValidator;

    // Local copies: the identically named helpers live in the sibling `tests` module.
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

    // A validator whose BLS key bytes do not parse: registers, but unauthenticated, so it can
    // never enter the active set however much stake it accrues.
    let mut unparsable_bls = ValidatorRegistered::from(&v[4]);
    unparsable_bls.blsVk = zero_g2();

    // A V2 registration whose signatures do not match the account: `authenticate` fails, the
    // parsed keys are still kept, and the validator is recorded as unauthenticated.
    let mut bad_sig = ValidatorRegisteredV2::from(&v[5]);
    bad_sig.blsSig = zero_g1().into();

    // Reuses v[0]'s Schnorr key. The V1 contract did not enforce uniqueness, so this is a
    // *tolerated* error: the event is skipped and the history continues.
    let mut dup_schnorr = ValidatorRegistered::from(&v[6]);
    dup_schnorr.schnorrVk = v[0].schnorr_vk;

    let events: Vec<alloy::primitives::LogData> = vec![
        // --- V1 surface: register, delegate, undelegate, rotate keys, exit ---
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
        // --- V2 surface: authenticated register, key rotation, commission, undelegate ---
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
        // --- V3 / fast-finality surface: the only validator eligible at 0.6 ---
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
        // A rejected x25519 key and an unparsable p2p address both degrade to `None` without an
        // error, which silently drops the validator from the 0.6 active set.
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
        // --- degradation and tolerated-error paths ---
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
        // --- exits, both shapes ---
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

    // One event per block, so the ladder reads cleanly; `sort_stake_table_events` orders by
    // (block, log_index) and is already exercised by the real-history corpora, which have
    // multiple logs per block.
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

/// Environment variable holding the archive RPC URL for a corpus.
fn archive_rpc_env(corpus: &str) -> &'static str {
    match corpus {
        "decaf" => "ESPRESSO_L1_ARCHIVE_RPC_SEPOLIA",
        "mainnet" => "ESPRESSO_L1_ARCHIVE_RPC_MAINNET",
        other => panic!("no archive RPC configured for corpus {other}"),
    }
}

/// Manifest entry for one corpus, from `data/stake_table_history/manifest.json`.
#[derive(serde::Deserialize)]
struct CorpusManifest {
    stake_table_contract: Address,
    from_block: u64,
    to_block: u64,
    log_count: usize,
    first_event_block: u64,
}

fn load_manifest(corpus: &str) -> CorpusManifest {
    let path = data_dir().join("stake_table_history").join("manifest.json");
    let raw = fs::read_to_string(&path).unwrap();
    let all: std::collections::HashMap<String, CorpusManifest> =
        serde_json::from_str(&raw).unwrap();
    all.into_iter()
        .find(|(k, _)| k == corpus)
        .unwrap_or_else(|| panic!("manifest has no entry for {corpus}"))
        .1
}

/// Checks the checked-in fixtures still describe the live chains.
///
/// Networked, so it runs on a schedule rather than per PR (see
/// `.github/workflows/stake-table-history.yml`). It asserts three things:
///
/// 1. the RPC actually serves the fixture's block range -- see below, this is not a given;
/// 2. re-fetching that range reproduces the fixture exactly;
/// 3. the history *past* the fixture still applies without a fatal error, so a validator syncing
///    from scratch today would not get stuck.
///
/// (1) is not paranoia. `ethereum-sepolia.publicnode.com`, which the previous version of this
/// test used, prunes old logs and answers `eth_getLogs` for pruned ranges with an empty result
/// rather than an error. Against it, decaf history silently starts ~2.8M blocks late and the
/// stake table hash is quietly wrong. Any fixture regeneration must fail loudly on such an
/// endpoint instead of committing truncated history, which is what the `first_event_block`
/// assertion is for.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires an archive L1 RPC; runs on a schedule, not per PR"]
async fn stake_table_history_is_current() {
    for corpus in ["decaf", "mainnet"] {
        let env = archive_rpc_env(corpus);
        let Ok(url) = std::env::var(env) else {
            panic!("{env} is not set; this test needs an archive RPC for {corpus}");
        };
        let manifest = load_manifest(corpus);

        let l1 = L1ClientOptions {
            l1_events_max_retry_duration: Duration::from_secs(120),
            l1_events_max_block_range: 45_000,
            l1_retry_delay: Duration::from_secs(2),
            ..Default::default()
        }
        .connect(vec![url.parse().expect("archive RPC URL parses")])
        .expect("unable to construct l1 client");

        // (1) + (2): the fixture range, re-fetched.
        let refetched = Fetcher::fetch_events_from_contract(
            l1.clone(),
            manifest.stake_table_contract,
            Some(manifest.from_block),
            manifest.to_block,
        )
        .await
        .expect("re-fetching the fixture range succeeds");

        let earliest = refetched.first().map(|((block, _), _)| *block);
        assert_eq!(
            earliest,
            Some(manifest.first_event_block),
            "{corpus}: the RPC at {env} does not serve the start of the fixture range (first \
             event at {earliest:?}, expected {}). It is pruning history and returning empty \
             results instead of erroring -- use a real archive node.",
            manifest.first_event_block,
        );

        let fixture = events_from_logs(load_logs(corpus)).unwrap();
        assert_eq!(
            refetched.len(),
            manifest.log_count,
            "{corpus}: live history has {} events in the fixture range, the manifest records {}",
            refetched.len(),
            manifest.log_count,
        );
        assert_eq!(
            refetched, fixture,
            "{corpus}: live history in the fixture range no longer matches the checked-in \
             fixture. If this is a legitimate reorg or contract change, regenerate with `just \
             regen-stake-table-fixtures`."
        );

        // (3): everything since, applied on top.
        let head = l1.provider.get_block_number().await.unwrap();
        let full = Fetcher::fetch_events_from_contract(
            l1,
            manifest.stake_table_contract,
            Some(manifest.from_block),
            head,
        )
        .await
        .expect("fetching current history succeeds");

        let current =
            ValidatorSet::from_l1_events(full.iter().map(|(_, e)| e.clone()), NEW_PROTOCOL_VERSION)
                .unwrap_or_else(|e| {
                    panic!(
                        "{corpus}: current L1 history no longer applies cleanly ({e}). A node \
                         syncing from scratch would fail here."
                    )
                });

        tracing::info!(
            corpus,
            head,
            fixture_events = fixture.len(),
            current_events = full.len(),
            stake_table_hash = %current.stake_table_hash().unwrap(),
            "history is current"
        );
    }
}
