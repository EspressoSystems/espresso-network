//! Everything the stake table contract knows about a single Ethereum address.
//!
//! An address can be a validator, a delegator, or both. State is derived entirely from event
//! logs, the same way the Espresso network itself builds the stake table. Every query is filtered
//! by the indexed address topic, which keeps the result set small enough to cover the whole
//! history of the contract in a single `eth_getLogs` call, and keeps the command within the
//! request rate limits of public RPC providers.
//!
//! Replaying events depends on `eth_getLogs` returning them in ascending
//! `(block number, log index)` order, which it does within a single query.
//!
//! The stake table V1 `Withdrawal` event named only the delegator, not the validator a claim
//! belonged to, so claims made before the V2 upgrade cannot be attributed: they leave both the
//! pending withdrawal and, for a validator exit claim, the stake itself overstated. V2 introduced
//! `WithdrawalClaimed` and `ValidatorExitClaimed`, which carry the validator. `approximate` marks
//! the affected results.

use std::{collections::BTreeMap, fmt::Display, future::Future};

use alloy::{
    primitives::{Address, B256, U256},
    providers::Provider,
    rpc::types::{Filter, Log},
    sol_types::{SolEvent as _, SolEventInterface as _},
};
use anyhow::{Context as _, Result};
use chrono::DateTime;
use hotshot_contract_adapter::sol_types::{
    EdOnBN254PointSol, G2PointSol,
    StakeTableV3::{
        self, CommissionUpdated, ConsensusKeysUpdated, ConsensusKeysUpdatedV2, Delegated,
        MetadataUriUpdated, P2pAddrUpdated, StakeTableV3Events, Undelegated, UndelegatedV2,
        ValidatorExit, ValidatorExitClaimed, ValidatorExitV2, ValidatorRegistered,
        ValidatorRegisteredV2, ValidatorRegisteredV3, Withdrawal, WithdrawalClaimed,
        X25519KeyUpdated,
    },
};
use hotshot_types::{addr::NetAddr, light_client::StateVerKey, signature_key::BLSPubKey, x25519};
use serde::Serialize;

use crate::{
    output::{Esp, output_success},
    parse::Commission,
};

/// Everything the stake table contract knows about one address at a given L1 block.
#[derive(Clone, Debug, Serialize)]
pub struct StakeTableEntry {
    pub address: Address,
    pub l1_block_number: u64,
    /// Registration and delegators, if the address ever registered as a validator.
    pub validator: Option<ValidatorEntry>,
    /// Stake this address has delegated to validators.
    pub delegations: Summary<Delegation>,
    /// Set when a pre-V2 withdrawal claim, which the events cannot attribute to a validator,
    /// may have left the amounts overstated.
    pub approximate: bool,
}

impl StakeTableEntry {
    /// Drop the per-counterparty lists, keeping only their totals.
    pub fn summarize(&mut self) {
        self.delegations.entries = None;
        if let Some(validator) = self.validator.as_mut() {
            validator.delegators.entries = None;
        }
    }
}

/// Totals over a set of stake positions, and optionally the positions themselves.
#[derive(Clone, Debug, Serialize)]
pub struct Summary<T> {
    pub count: usize,
    pub total_stake: Esp,
    pub pending_withdrawal_count: usize,
    pub total_pending_withdrawal: Esp,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entries: Option<Vec<T>>,
}

impl<T: Position> Summary<T> {
    fn new(entries: Vec<T>) -> Self {
        let pending = |entry: &T| entry.pending_withdrawal().map(|w| w.amount.0);
        Self {
            count: entries.len(),
            total_stake: entries.iter().map(|e| e.stake().0).sum::<U256>().into(),
            pending_withdrawal_count: entries.iter().filter_map(pending).count(),
            total_pending_withdrawal: entries.iter().filter_map(pending).sum::<U256>().into(),
            entries: Some(entries),
        }
    }
}

/// A stake position held by one counterparty.
pub trait Position {
    fn stake(&self) -> Esp;
    fn pending_withdrawal(&self) -> Option<PendingWithdrawal>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidatorStatus {
    Active,
    Exited,
}

impl Display for ValidatorStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Active => "active",
            Self::Exited => "exited",
        })
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ValidatorEntry {
    pub status: ValidatorStatus,
    /// Stake currently delegated, excluding amounts marked for withdrawal.
    pub stake: Esp,
    #[serde(serialize_with = "serialize_display")]
    pub commission: Commission,
    /// Whether the registration signatures verified against the registered keys.
    pub authenticated: bool,
    pub consensus_public_key: Option<BLSPubKey>,
    pub state_public_key: Option<StateVerKey>,
    /// Serialized in the tagged form the CLI accepts, unlike the raw `x25519` serialization.
    #[serde(serialize_with = "serialize_display_opt")]
    pub x25519_public_key: Option<x25519::PublicKey>,
    pub p2p_addr: Option<NetAddr>,
    pub metadata_uri: Option<String>,
    pub registered_at_l1_block: u64,
    /// When delegators can claim their stake back. Unset for pre-V2 exits, which did not record
    /// the unlock time in the event.
    pub exit_unlocks_at: Option<u64>,
    pub delegators: Summary<Delegator>,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Delegator {
    pub address: Address,
    pub stake: Esp,
    pub pending_withdrawal: Option<PendingWithdrawal>,
}

impl Position for Delegator {
    fn stake(&self) -> Esp {
        self.stake
    }

    fn pending_withdrawal(&self) -> Option<PendingWithdrawal> {
        self.pending_withdrawal
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Delegation {
    pub validator: Address,
    pub stake: Esp,
    pub pending_withdrawal: Option<PendingWithdrawal>,
    pub validator_status: ValidatorStatus,
    /// When `stake` becomes claimable, set only if the validator exited.
    pub validator_exit_unlocks_at: Option<u64>,
}

impl Position for Delegation {
    fn stake(&self) -> Esp {
        self.stake
    }

    fn pending_withdrawal(&self) -> Option<PendingWithdrawal> {
        self.pending_withdrawal
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct PendingWithdrawal {
    pub amount: Esp,
    /// Unset for pre-V2 undelegations, which did not record the unlock time in the event.
    pub unlocks_at: Option<u64>,
}

fn serialize_display<T: Display, S: serde::Serializer>(
    value: &T,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_str(value)
}

fn serialize_display_opt<T: Display, S: serde::Serializer>(
    value: &Option<T>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match value {
        Some(value) => serializer.collect_str(value),
        None => serializer.serialize_none(),
    }
}

/// Fetch all stake table state for `address` as of `l1_block_number`.
pub async fn fetch_stake_table_entry(
    provider: &impl Provider,
    stake_table_address: Address,
    address: Address,
    l1_block_number: u64,
) -> Result<StakeTableEntry> {
    let contract = StakeTableV3::new(stake_table_address, provider);
    let from_block = contract
        .initializedAtBlock()
        .block(l1_block_number.into())
        .call()
        .await
        .context("failed to read the block the stake table was initialized at")?
        .to::<u64>();

    let fetch = |events: Vec<&'static str>, topic1: Option<Vec<B256>>, topic2: Option<B256>| {
        let mut filter = Filter::new()
            .address(stake_table_address)
            .events(events)
            .from_block(from_block)
            .to_block(l1_block_number);
        if let Some(topic) = topic1 {
            filter = filter.topic1(topic);
        }
        if let Some(topic) = topic2 {
            filter = filter.topic2(topic);
        }
        async move { get_logs_adaptively(provider, filter, from_block, l1_block_number).await }
    };

    let word = address.into_word();
    let (registration_logs, as_validator_logs, as_delegator_logs, withdrawal_logs) = futures_util::try_join!(
        fetch(registration_events(), Some(vec![word]), None),
        // `Delegated` and `Undelegated` index the delegator first, the validator second.
        fetch(stake_events(), None, Some(word)),
        fetch(stake_events(), Some(vec![word]), None),
        fetch(withdrawal_events(), Some(vec![word]), None),
    )?;

    // Every V2 claim also emits an attributable event, so only the surplus is unattributable.
    let attributable = as_delegator_logs
        .iter()
        .filter(|log| {
            matches!(
                log.topics().first().map(|t| t.as_slice()),
                Some(t) if t == WithdrawalClaimed::SIGNATURE_HASH.as_slice()
                    || t == ValidatorExitClaimed::SIGNATURE_HASH.as_slice()
            )
        })
        .count();
    let approximate = withdrawal_logs.len() > attributable;
    if approximate {
        tracing::warn!(
            "found {} pre-V2 withdrawal claims that the events cannot attribute to a validator; \
             stake and pending withdrawal amounts may be overstated",
            withdrawal_logs.len() - attributable,
        );
    }

    let positions = fold_positions(&as_delegator_logs, VALIDATOR_TOPIC)?;
    let exits = fetch_exits(&fetch, positions.keys().copied().collect()).await?;
    let delegations = Summary::new(
        positions
            .into_iter()
            .map(|(validator, position)| Delegation {
                validator,
                stake: position.stake.into(),
                pending_withdrawal: position.pending,
                validator_status: match exits.contains_key(&validator) {
                    true => ValidatorStatus::Exited,
                    false => ValidatorStatus::Active,
                },
                validator_exit_unlocks_at: exits.get(&validator).copied().flatten(),
            })
            .collect(),
    );

    let validator = fold_registration(&registration_logs)?
        .map(|registration| {
            registration.into_entry(fold_positions(&as_validator_logs, DELEGATOR_TOPIC)?)
        })
        .transpose()?;

    Ok(StakeTableEntry {
        address,
        l1_block_number,
        validator,
        delegations,
        approximate,
    })
}

/// Blocks per request when the range has to be split. Matches the default of the environment
/// variable that overrides it, which is the range capped providers usually allow.
const DEFAULT_BLOCK_RANGE: u64 = 10_000;

/// The block range the user pinned for their provider, if any.
///
/// Set means "my provider caps the range at this", so the single request is skipped rather than
/// spent on a rejection.
pub(crate) fn configured_block_range() -> Option<u64> {
    let value = std::env::var_os(BLOCK_RANGE_VAR)?;
    match value.to_str().and_then(|value| value.parse().ok()) {
        Some(range) if range > 0 => Some(range),
        _ => {
            tracing::warn!("ignoring {BLOCK_RANGE_VAR}: expected a positive integer");
            None
        },
    }
}

pub(crate) const BLOCK_RANGE_VAR: &str = "ESPRESSO_L1_EVENTS_MAX_BLOCK_RANGE";

/// Fetch a filter's logs in one request, splitting the range only if that is refused.
///
/// Providers pull in opposite directions: the gateways this CLI defaults to serve the whole range
/// at once but rate limit the hundreds of requests splitting needs, while others cap the range and
/// only work split. Neither attempt retries, so a provider that refuses the single request costs
/// one extra round trip rather than a retry loop.
async fn get_logs_adaptively(
    provider: &impl Provider,
    filter: Filter,
    from_block: u64,
    to_block: u64,
) -> Result<Vec<Log>> {
    let range = match configured_block_range() {
        Some(range) => range,
        None => match provider.get_logs(&filter).await {
            Ok(logs) => return Ok(logs),
            Err(err) => {
                tracing::info!(
                    %err,
                    "could not fetch events in one request, retrying in {DEFAULT_BLOCK_RANGE} \
                     block ranges"
                );
                DEFAULT_BLOCK_RANGE
            },
        },
    };

    let mut logs = vec![];
    for (from, to) in block_ranges(from_block, to_block, range) {
        logs.extend(
            provider
                .get_logs(&filter.clone().from_block(from).to_block(to))
                .await
                .with_context(|| {
                    format!(
                        "failed to fetch stake table events for blocks {from} to {to}; set \
                         {BLOCK_RANGE_VAR} lower if the provider caps the range"
                    )
                })?,
        );
    }
    Ok(logs)
}

/// Split an inclusive block range into consecutive chunks of at most `size` blocks.
///
/// `next` is an `Option` so that a chunk ending at `u64::MAX` terminates the iterator instead of
/// saturating back onto itself.
fn block_ranges(from: u64, to: u64, size: u64) -> impl Iterator<Item = (u64, u64)> {
    let mut next = Some(from);
    std::iter::from_fn(move || {
        let start = next.filter(|&start| start <= to)?;
        let end = start.saturating_add(size - 1).min(to);
        next = end.checked_add(1);
        Some((start, end))
    })
}

/// Events carrying a validator's own registration data, all indexed by the validator address.
fn registration_events() -> Vec<&'static str> {
    vec![
        ValidatorRegistered::SIGNATURE,
        ValidatorRegisteredV2::SIGNATURE,
        ValidatorRegisteredV3::SIGNATURE,
        ConsensusKeysUpdated::SIGNATURE,
        ConsensusKeysUpdatedV2::SIGNATURE,
        CommissionUpdated::SIGNATURE,
        MetadataUriUpdated::SIGNATURE,
        X25519KeyUpdated::SIGNATURE,
        P2pAddrUpdated::SIGNATURE,
        ValidatorExit::SIGNATURE,
        ValidatorExitV2::SIGNATURE,
    ]
}

/// Events that move stake, all indexed by delegator then validator.
fn stake_events() -> Vec<&'static str> {
    vec![
        Delegated::SIGNATURE,
        Undelegated::SIGNATURE,
        UndelegatedV2::SIGNATURE,
        WithdrawalClaimed::SIGNATURE,
        ValidatorExitClaimed::SIGNATURE,
    ]
}

/// The pre-V2 claim event, indexed by delegator only.
fn withdrawal_events() -> Vec<&'static str> {
    vec![Withdrawal::SIGNATURE]
}

const DELEGATOR_TOPIC: usize = 1;
const VALIDATOR_TOPIC: usize = 2;

/// Which of the given validators have exited, and when their stake unlocks.
async fn fetch_exits<F, Fut>(
    fetch: &F,
    validators: Vec<Address>,
) -> Result<BTreeMap<Address, Option<u64>>>
where
    F: Fn(Vec<&'static str>, Option<Vec<B256>>, Option<B256>) -> Fut,
    Fut: Future<Output = Result<Vec<Log>>>,
{
    if validators.is_empty() {
        return Ok(BTreeMap::new());
    }
    let topics = validators.iter().map(|v| v.into_word()).collect();
    let logs = fetch(
        vec![ValidatorExit::SIGNATURE, ValidatorExitV2::SIGNATURE],
        Some(topics),
        None,
    )
    .await?;

    let mut exits = BTreeMap::new();
    for log in &logs {
        match decode(log)? {
            StakeTableV3Events::ValidatorExit(e) => {
                exits.insert(e.validator, None);
            },
            StakeTableV3Events::ValidatorExitV2(e) => {
                exits.insert(e.validator, Some(e.unlocksAt.saturating_to()));
            },
            _ => tracing::warn!(topics = ?log.topics(), "ignoring unexpected event"),
        }
    }
    Ok(exits)
}

/// Folded stake one counterparty holds with, or in, the address being queried.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct FoldedPosition {
    stake: U256,
    pending: Option<PendingWithdrawal>,
}

/// Replay the stake events into a position per counterparty.
///
/// `counterparty_topic` selects the other side of each event: the validator when the queried
/// address is the delegator, the delegator when it is the validator.
/// The contract rejects an undelegation larger than the balance, so an underflow here means the
/// replay is missing events rather than that the stake is zero.
fn undelegate(stake: U256, amount: U256) -> Result<U256> {
    stake.checked_sub(amount).with_context(|| {
        format!("undelegation of {amount} exceeds the delegated {stake}; events are incomplete")
    })
}

fn fold_positions(
    logs: &[Log],
    counterparty_topic: usize,
) -> Result<BTreeMap<Address, FoldedPosition>> {
    let mut positions = BTreeMap::<Address, FoldedPosition>::new();
    for log in logs {
        let position = positions
            .entry(topic_address(log, counterparty_topic)?)
            .or_default();
        match decode(log)? {
            StakeTableV3Events::Delegated(e) => position.stake += e.amount,
            StakeTableV3Events::Undelegated(e) => {
                position.stake = undelegate(position.stake, e.amount)?;
                position.pending = Some(PendingWithdrawal {
                    amount: e.amount.into(),
                    unlocks_at: None,
                });
            },
            StakeTableV3Events::UndelegatedV2(e) => {
                position.stake = undelegate(position.stake, e.amount)?;
                position.pending = Some(PendingWithdrawal {
                    amount: e.amount.into(),
                    unlocks_at: Some(e.unlocksAt.saturating_to()),
                });
            },
            // The contract allows at most one outstanding undelegation per pair.
            StakeTableV3Events::WithdrawalClaimed(_) => position.pending = None,
            // The contract zeroes the whole delegation rather than subtracting.
            StakeTableV3Events::ValidatorExitClaimed(_) => position.stake = U256::ZERO,
            _ => tracing::warn!(topics = ?log.topics(), "ignoring unexpected event"),
        }
    }
    positions.retain(|_, position| !position.stake.is_zero() || position.pending.is_some());
    Ok(positions)
}

/// Registration state as of the last event.
#[derive(Debug, Default)]
struct Registration {
    commission: u16,
    authenticated: bool,
    consensus_public_key: Option<BLSPubKey>,
    state_public_key: Option<StateVerKey>,
    x25519_public_key: Option<x25519::PublicKey>,
    p2p_addr: Option<NetAddr>,
    metadata_uri: Option<String>,
    registered_at_l1_block: u64,
    exited: bool,
    exit_unlocks_at: Option<u64>,
}

impl Registration {
    fn into_entry(self, delegators: BTreeMap<Address, FoldedPosition>) -> Result<ValidatorEntry> {
        let stake = delegators
            .values()
            .fold(U256::ZERO, |total, position| total + position.stake);
        Ok(ValidatorEntry {
            status: match self.exited {
                true => ValidatorStatus::Exited,
                false => ValidatorStatus::Active,
            },
            stake: stake.into(),
            commission: self.commission.try_into()?,
            authenticated: self.authenticated,
            consensus_public_key: self.consensus_public_key,
            state_public_key: self.state_public_key,
            x25519_public_key: self.x25519_public_key,
            p2p_addr: self.p2p_addr,
            metadata_uri: self.metadata_uri,
            registered_at_l1_block: self.registered_at_l1_block,
            exit_unlocks_at: self.exit_unlocks_at,
            delegators: Summary::new(
                delegators
                    .into_iter()
                    .map(|(address, position)| Delegator {
                        address,
                        stake: position.stake.into(),
                        pending_withdrawal: position.pending,
                    })
                    .collect(),
            ),
        })
    }
}

/// Replay the address' registration events, or `None` if it never registered as a validator.
fn fold_registration(logs: &[Log]) -> Result<Option<Registration>> {
    let mut registration: Option<Registration> = None;
    for log in logs {
        let event = decode(log)?;
        // The contract rejects registering an address twice, so this only ever runs once. The
        // reset is defensive: a second registration must not inherit the first one's state.
        if let Some(block_number) = registration_block(&event, log)? {
            registration = Some(Registration {
                registered_at_l1_block: block_number,
                ..Default::default()
            });
        }
        let Some(current) = registration.as_mut() else {
            tracing::warn!(topics = ?log.topics(), "ignoring event before registration");
            continue;
        };
        apply_registration_event(current, event);
    }
    Ok(registration)
}

fn registration_block(event: &StakeTableV3Events, log: &Log) -> Result<Option<u64>> {
    let is_registration = matches!(
        event,
        StakeTableV3Events::ValidatorRegistered(_)
            | StakeTableV3Events::ValidatorRegisteredV2(_)
            | StakeTableV3Events::ValidatorRegisteredV3(_)
    );
    if !is_registration {
        return Ok(None);
    }
    let block_number = log
        .block_number
        .context("registration event log is missing a block number")?;
    Ok(Some(block_number))
}

fn apply_registration_event(current: &mut Registration, event: StakeTableV3Events) {
    match event {
        // V1 events carry no signatures, so keys count as authenticated iff they parse.
        StakeTableV3Events::ValidatorRegistered(e) => {
            set_parsed_keys(current, e.blsVk, e.schnorrVk);
            current.commission = e.commission;
        },
        // A key update whose keys do not parse or verify is skipped by the network, which keeps
        // serving the previous keys. Blanking them here would misreport that.
        StakeTableV3Events::ConsensusKeysUpdated(e) => {
            if let (Ok(bls), Ok(schnorr)) = (
                BLSPubKey::try_from(e.blsVK),
                StateVerKey::try_from(e.schnorrVK),
            ) {
                current.consensus_public_key = Some(bls);
                current.state_public_key = Some(schnorr);
                current.authenticated = true;
            }
        },
        StakeTableV3Events::ValidatorRegisteredV2(e) => {
            set_verified_keys(current, e.authenticate().ok(), e.blsVK, e.schnorrVK);
            current.commission = e.commission;
            current.metadata_uri = Some(e.metadataUri);
        },
        StakeTableV3Events::ConsensusKeysUpdatedV2(e) => {
            if let Ok((bls, schnorr)) = e.authenticate() {
                current.consensus_public_key = Some(bls);
                current.state_public_key = Some(schnorr);
                current.authenticated = true;
            }
        },
        StakeTableV3Events::ValidatorRegisteredV3(e) => {
            set_verified_keys(current, e.authenticate().ok(), e.blsVK, e.schnorrVK);
            current.commission = e.commission;
            current.metadata_uri = Some(e.metadataUri);
            current.x25519_public_key = parse_x25519_key(e.x25519Key.0);
            current.p2p_addr = e.p2pAddr.parse().ok();
        },
        StakeTableV3Events::CommissionUpdated(e) => current.commission = e.newCommission,
        StakeTableV3Events::MetadataUriUpdated(e) => current.metadata_uri = Some(e.metadataUri),
        StakeTableV3Events::X25519KeyUpdated(e) => {
            current.x25519_public_key = parse_x25519_key(e.x25519Key.0)
        },
        StakeTableV3Events::P2pAddrUpdated(e) => current.p2p_addr = e.p2pAddr.parse().ok(),
        StakeTableV3Events::ValidatorExit(_) => current.exited = true,
        StakeTableV3Events::ValidatorExitV2(e) => {
            current.exited = true;
            current.exit_unlocks_at = Some(e.unlocksAt.saturating_to());
        },
        _ => tracing::warn!("ignoring unexpected stake table event"),
    }
}

fn set_parsed_keys(current: &mut Registration, bls_vk: G2PointSol, schnorr_vk: EdOnBN254PointSol) {
    current.consensus_public_key = BLSPubKey::try_from(bls_vk).ok();
    current.state_public_key = StateVerKey::try_from(schnorr_vk).ok();
    current.authenticated =
        current.consensus_public_key.is_some() && current.state_public_key.is_some();
}

/// Prefer the keys returned by signature verification, falling back to the raw event keys.
fn set_verified_keys(
    current: &mut Registration,
    verified: Option<(BLSPubKey, StateVerKey)>,
    bls_vk: G2PointSol,
    schnorr_vk: EdOnBN254PointSol,
) {
    match verified {
        Some((bls, schnorr)) => {
            current.consensus_public_key = Some(bls);
            current.state_public_key = Some(schnorr);
            current.authenticated = true;
        },
        None => {
            set_parsed_keys(current, bls_vk, schnorr_vk);
            current.authenticated = false;
        },
    }
}

/// The contract rejects the zero key, which the Rust parser would otherwise accept.
fn parse_x25519_key(bytes: [u8; 32]) -> Option<x25519::PublicKey> {
    if bytes == [0u8; 32] {
        return None;
    }
    x25519::PublicKey::try_from(bytes.as_slice()).ok()
}

fn decode(log: &Log) -> Result<StakeTableV3Events> {
    StakeTableV3Events::decode_raw_log(log.topics(), &log.data().data)
        .context("failed to decode stake table event")
}

fn topic_address(log: &Log, topic: usize) -> Result<Address> {
    let word = log
        .topics()
        .get(topic)
        .with_context(|| format!("event log is missing topic {topic}"))?;
    Ok(Address::from_word(*word))
}

pub fn display_stake_table_entry(entry: &StakeTableEntry) {
    output_success(format!("Address: {}", entry.address));
    output_success(format!("L1 block: {}", entry.l1_block_number));
    if entry.approximate {
        output_success(
            "Note: pre-V2 withdrawal claims cannot be attributed to a validator, so the amounts \
             below may be overstated.",
        );
    }

    match &entry.validator {
        None => output_success("Validator: not registered"),
        Some(validator) => display_validator(validator),
    }

    output_success(format!("Delegations: {}", summary_line(&entry.delegations)));
    for delegation in entry.delegations.entries.iter().flatten() {
        let exit = match delegation.validator_status {
            ValidatorStatus::Active => String::new(),
            ValidatorStatus::Exited => format!(
                ", validator exited, claimable at {}",
                timestamp(delegation.validator_exit_unlocks_at)
            ),
        };
        output_success(format!(
            "- {}: {}{}{}",
            delegation.validator,
            delegation.stake,
            withdrawal_suffix(&delegation.pending_withdrawal),
            exit
        ));
    }
}

/// One-line totals, hinting at `--delegations` when the list is hidden.
fn summary_line<T>(summary: &Summary<T>) -> String {
    if summary.count == 0 {
        return "none".to_string();
    }
    let pending = match summary.pending_withdrawal_count {
        0 => String::new(),
        count => format!(
            ", {count} pending withdrawal(s) totalling {}",
            summary.total_pending_withdrawal
        ),
    };
    let hint = match summary.entries {
        Some(_) => ":",
        None => " (use --delegations to list)",
    };
    format!(
        "{}, {} total{pending}{hint}",
        summary.count, summary.total_stake
    )
}

fn display_validator(validator: &ValidatorEntry) {
    output_success("Validator:");
    output_success(format!("  Status: {}", validator.status));
    output_success(format!("  Stake: {}", validator.stake));
    output_success(format!("  Commission: {}", validator.commission));
    output_success(format!("  Authenticated: {}", validator.authenticated));
    output_success(format!(
        "  Consensus public key: {}",
        optional(&validator.consensus_public_key)
    ));
    output_success(format!(
        "  State public key: {}",
        optional(&validator.state_public_key)
    ));
    output_success(format!(
        "  x25519 public key: {}",
        optional(&validator.x25519_public_key)
    ));
    output_success(format!("  p2p address: {}", optional(&validator.p2p_addr)));
    output_success(format!(
        "  Metadata URI: {}",
        optional(&validator.metadata_uri)
    ));
    output_success(format!(
        "  Registered at L1 block: {}",
        validator.registered_at_l1_block
    ));
    if validator.status == ValidatorStatus::Exited {
        output_success(format!(
            "  Exit unlocks at: {}",
            timestamp(validator.exit_unlocks_at)
        ));
    }
    output_success(format!(
        "  Delegators: {}",
        summary_line(&validator.delegators)
    ));
    for delegator in validator.delegators.entries.iter().flatten() {
        output_success(format!(
            "  - {}: {}{}",
            delegator.address,
            delegator.stake,
            withdrawal_suffix(&delegator.pending_withdrawal)
        ));
    }
}

fn withdrawal_suffix(withdrawal: &Option<PendingWithdrawal>) -> String {
    match withdrawal {
        Some(withdrawal) => format!(
            ", pending withdrawal {} unlocking at {}",
            withdrawal.amount,
            timestamp(withdrawal.unlocks_at)
        ),
        None => String::new(),
    }
}

fn optional<T: Display>(value: &Option<T>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "not set".to_string(),
    }
}

fn timestamp(seconds: Option<u64>) -> String {
    let Some(seconds) = seconds else {
        return "unknown".to_string();
    };
    match DateTime::from_timestamp(seconds as i64, 0) {
        Some(time) => time.to_rfc3339(),
        None => seconds.to_string(),
    }
}

#[cfg(test)]
mod test {
    use alloy::{
        primitives::{Address, LogData, U256, address, utils::parse_ether},
        rpc::types::Log,
        sol_types::SolEvent as _,
    };
    use anyhow::Result;
    use pretty_assertions::assert_eq;

    // A glob import here would pull in an `Option` that shadows the standard one.
    use super::{
        CommissionUpdated, ConsensusKeysUpdated, Delegated, Delegator, FoldedPosition,
        PendingWithdrawal, Registration, StakeTableV3Events, Summary, UndelegatedV2,
        VALIDATOR_TOPIC, ValidatorExitClaimed, ValidatorExitV2, WithdrawalClaimed,
        apply_registration_event, block_ranges, fold_positions, fold_registration,
        parse_x25519_key, timestamp,
    };

    const VALIDATOR: Address = address!("0x1111111111111111111111111111111111111111");
    const DELEGATOR: Address = address!("0x2222222222222222222222222222222222222222");

    /// Wrap an encoded event in a log, at an increasing index so the fold order is defined.
    fn log(index: u64, data: LogData) -> Log {
        Log {
            block_number: Some(index),
            log_index: Some(index),
            ..Log {
                inner: alloy::primitives::Log {
                    address: Address::ZERO,
                    data,
                },
                ..Default::default()
            }
        }
    }

    fn delegated(index: u64, amount: U256) -> Log {
        log(
            index,
            Delegated {
                delegator: DELEGATOR,
                validator: VALIDATOR,
                amount,
            }
            .encode_log_data(),
        )
    }

    fn undelegated_v2(index: u64, amount: U256, unlocks_at: u64) -> Log {
        log(
            index,
            UndelegatedV2 {
                delegator: DELEGATOR,
                validator: VALIDATOR,
                undelegationId: 0,
                amount,
                unlocksAt: U256::from(unlocks_at),
            }
            .encode_log_data(),
        )
    }

    fn withdrawal_claimed(index: u64, amount: U256) -> Log {
        log(
            index,
            WithdrawalClaimed {
                delegator: DELEGATOR,
                validator: VALIDATOR,
                undelegationId: 0,
                amount,
            }
            .encode_log_data(),
        )
    }

    fn exit_claimed(index: u64, amount: U256) -> Log {
        log(
            index,
            ValidatorExitClaimed {
                delegator: DELEGATOR,
                validator: VALIDATOR,
                amount,
            }
            .encode_log_data(),
        )
    }

    fn position(logs: &[Log]) -> Option<FoldedPosition> {
        fold_positions(logs, VALIDATOR_TOPIC)
            .expect("fold succeeds")
            .get(&VALIDATOR)
            .copied()
    }

    #[test]
    fn test_delegations_accumulate() -> Result<()> {
        let folded = position(&[
            delegated(1, parse_ether("1")?),
            delegated(2, parse_ether("2")?),
        ])
        .expect("position");
        assert_eq!(folded.stake, parse_ether("3")?);
        assert_eq!(folded.pending, None);
        Ok(())
    }

    #[test]
    fn test_undelegation_moves_stake_to_pending() -> Result<()> {
        let folded = position(&[
            delegated(1, parse_ether("3")?),
            undelegated_v2(2, parse_ether("1")?, 42),
        ])
        .expect("position");
        assert_eq!(folded.stake, parse_ether("2")?);
        assert_eq!(
            folded.pending,
            Some(PendingWithdrawal {
                amount: parse_ether("1")?.into(),
                unlocks_at: Some(42),
            })
        );
        Ok(())
    }

    #[test]
    fn test_claiming_a_withdrawal_clears_pending() -> Result<()> {
        let folded = position(&[
            delegated(1, parse_ether("3")?),
            undelegated_v2(2, parse_ether("1")?, 42),
            withdrawal_claimed(3, parse_ether("1")?),
        ])
        .expect("position");
        assert_eq!(folded.stake, parse_ether("2")?);
        assert_eq!(folded.pending, None);
        Ok(())
    }

    /// The contract zeroes the delegation, so the whole position goes away.
    #[test]
    fn test_claiming_a_validator_exit_empties_the_position() -> Result<()> {
        let logs = [
            delegated(1, parse_ether("3")?),
            exit_claimed(2, parse_ether("3")?),
        ];
        assert_eq!(position(&logs), None);
        Ok(())
    }

    /// A fully withdrawn position is dropped rather than reported as zero.
    #[test]
    fn test_fully_undelegated_and_claimed_position_is_dropped() -> Result<()> {
        let logs = [
            delegated(1, parse_ether("1")?),
            undelegated_v2(2, parse_ether("1")?, 42),
            withdrawal_claimed(3, parse_ether("1")?),
        ];
        assert_eq!(position(&logs), None);
        Ok(())
    }

    /// An undelegation the contract would have rejected means events are missing, not zero stake.
    #[test]
    fn test_undelegating_more_than_delegated_is_an_error() -> Result<()> {
        let logs = [
            delegated(1, parse_ether("1")?),
            undelegated_v2(2, parse_ether("2")?, 42),
        ];
        let err = fold_positions(&logs, VALIDATOR_TOPIC).expect_err("fold fails");
        assert!(
            err.to_string().contains("events are incomplete"),
            "unexpected error: {err}"
        );
        Ok(())
    }

    #[test]
    fn test_no_registration_events_means_not_a_validator() {
        assert!(fold_registration(&[]).expect("fold succeeds").is_none());
    }

    #[test]
    fn test_registration_folds_the_latest_commission() -> Result<()> {
        let mut registration = Registration::default();
        apply_registration_event(
            &mut registration,
            StakeTableV3Events::CommissionUpdated(CommissionUpdated {
                validator: VALIDATOR,
                timestamp: U256::ZERO,
                oldCommission: 100,
                newCommission: 250,
            }),
        );
        assert_eq!(registration.commission, 250);
        Ok(())
    }

    /// The network keeps serving the old keys when an update fails to verify, so this must too.
    #[test]
    fn test_unverifiable_key_update_keeps_the_previous_keys() {
        let mut registration = Registration {
            authenticated: true,
            ..Default::default()
        };
        apply_registration_event(
            &mut registration,
            StakeTableV3Events::ConsensusKeysUpdated(ConsensusKeysUpdated {
                account: VALIDATOR,
                blsVK: Default::default(),
                schnorrVK: Default::default(),
            }),
        );
        assert_eq!(registration.consensus_public_key, None);
        assert!(registration.authenticated);
    }

    #[test]
    fn test_exit_records_the_unlock_time() {
        let mut registration = Registration::default();
        apply_registration_event(
            &mut registration,
            StakeTableV3Events::ValidatorExitV2(ValidatorExitV2 {
                validator: VALIDATOR,
                unlocksAt: U256::from(1234),
            }),
        );
        assert!(registration.exited);
        assert_eq!(registration.exit_unlocks_at, Some(1234));
    }

    #[test]
    fn test_zero_x25519_key_is_rejected() {
        assert_eq!(parse_x25519_key([0u8; 32]), None);
        assert!(parse_x25519_key([1u8; 32]).is_some());
    }

    #[test]
    fn test_summary_totals_pending_withdrawals() -> Result<()> {
        let summary = Summary::new(vec![
            Delegator {
                address: DELEGATOR,
                stake: parse_ether("1")?.into(),
                pending_withdrawal: None,
            },
            Delegator {
                address: VALIDATOR,
                stake: parse_ether("2")?.into(),
                pending_withdrawal: Some(PendingWithdrawal {
                    amount: parse_ether("3")?.into(),
                    unlocks_at: None,
                }),
            },
        ]);
        assert_eq!(summary.count, 2);
        assert_eq!(summary.total_stake.0, parse_ether("3")?);
        assert_eq!(summary.pending_withdrawal_count, 1);
        assert_eq!(summary.total_pending_withdrawal.0, parse_ether("3")?);
        Ok(())
    }

    #[test]
    fn test_block_ranges_cover_the_whole_span_without_gaps() {
        assert_eq!(block_ranges(0, 9, 10).collect::<Vec<_>>(), vec![(0, 9)]);
        assert_eq!(block_ranges(5, 5, 10).collect::<Vec<_>>(), vec![(5, 5)]);
        assert_eq!(
            block_ranges(0, 10, 4).collect::<Vec<_>>(),
            vec![(0, 3), (4, 7), (8, 10)]
        );
        assert_eq!(
            block_ranges(7, 9, 1).collect::<Vec<_>>(),
            vec![(7, 7), (8, 8), (9, 9)]
        );
        // An exhausted range yields nothing rather than looping.
        assert_eq!(block_ranges(10, 9, 10).count(), 0);
        assert_eq!(block_ranges(u64::MAX - 1, u64::MAX, 10).count(), 1);
    }

    #[test]
    fn test_missing_timestamp_renders_as_unknown() {
        assert_eq!(timestamp(None), "unknown");
        assert_eq!(timestamp(Some(0)), "1970-01-01T00:00:00+00:00");
    }
}
