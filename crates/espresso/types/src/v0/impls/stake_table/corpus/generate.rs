//! Generated inputs, run through the node.
//!
//! Each generator draws inputs from a random number generator with a fixed seed, so
//! that every run writes the same cases, and records what the node makes of them as a
//! [`Case`]. Events are drawn from small pools of validators, keys and addresses, so
//! that they collide often.

use std::{
    collections::{BTreeMap, HashMap},
    num::NonZeroU64,
};

use alloy::{
    hex,
    primitives::{Address, FixedBytes, U256},
    rpc::types::Log,
};
use anyhow::{Context, Result, ensure};
use ark_serialize::CanonicalSerialize;
use hotshot_contract_adapter::{
    sol_types::{
        EdOnBN254PointSol, G2PointSol,
        StakeTableV3::{
            CommissionUpdated, ConsensusKeysUpdated, ConsensusKeysUpdatedV2, Delegated,
            P2pAddrUpdated, StakeTableV3Events, Undelegated, UndelegatedV2, ValidatorExit,
            ValidatorExitV2, ValidatorRegistered, ValidatorRegisteredV2, ValidatorRegisteredV3,
            X25519KeyUpdated,
        },
    },
    stake_table::StakeTableSolError,
};
use hotshot_types::{
    addr::NetAddr,
    data::EpochNumber,
    drb::election::{generate_stake_cdf, select_randomized_leader},
    signature_key::{BLSPubKey, SchnorrPubKey},
    stake_table::StakeTableEntry,
    traits::signature_key::SignatureKey,
    x25519,
};
use rand::{Rng, RngCore, SeedableRng, rngs::StdRng, seq::SliceRandom};
use versions::CANONICAL_STAKE_TABLE_ORDER_VERSION;

use super::{
    super::{
        compute_block_reward, parse_x25519_key, select_active_validator_set,
        sort_stake_table_events, testing::TestValidator,
    },
    case::{Case, Event, KeyUpdateKeys, RegistrationKeys, Selection, State, Validator},
};
use crate::{
    AuthenticatedValidatorMap, RewardDistributor, StakeTableState,
    v0_3::{
        AuthenticatedValidator, EventKey, ExpectedStakeTableError, Fetcher, MAX_VALIDATORS,
        RegisteredValidator, RewardAmount, StakeTableError, StakeTableEvent,
    },
};

/// Event sequences over a small pool, some ending in an event drawn for its outcome.
///
/// Fails if too few replays finish, or if some outcome never turned up.
pub fn replays() -> Result<Vec<Case>> {
    let mut rng = StdRng::seed_from_u64(1);
    let pool = Pool::new(&mut rng, 8);
    let targets = Pool::targets();
    let mut cases = Vec::new();
    let mut finished = 0;
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for round in 0..300 {
        let len = rng.gen_range(1..60);
        // Every other sequence ends in an event chosen for its outcome.
        let target = (round % 2 == 1).then(|| targets[(round / 2) % targets.len()]);
        let events = pool.sequence(&mut rng, len, target)?;
        let (case, _) = replay_from(
            format!("generated round {round}"),
            &events,
            Replayed::default(),
        )?;
        if let Case::Replay {
            outcomes, state, ..
        } = &case
        {
            for o in outcomes {
                *seen.entry(o.clone()).or_default() += 1;
            }
            finished += usize::from(state.is_some());
        }
        cases.push(case);
    }
    ensure!(finished > 100, "only {finished} generated replays finished");
    for target in targets.into_iter().chain(["applied"]) {
        ensure!(seen.contains_key(target), "never saw {target}: {seen:?}");
    }
    Ok(cases)
}

/// Stake tables with more eligible validators than `MAX_VALIDATORS`, with tied stakes,
/// small stakes below the minimum, and validators that are not candidates, so that every
/// rule of the selection decides; and the leader schedule of each committee.
///
/// Fails if no committee was cut off at `MAX_VALIDATORS` among tied stakes.
pub fn selections() -> Result<Vec<Case>> {
    let mut rng = StdRng::seed_from_u64(6);
    let validators: Vec<_> = (0..250)
        .map(|_| TestValidator::from_rng(&mut rng))
        .collect();
    // Enough usable addresses that more candidates than `MAX_VALIDATORS` remain.
    let addrs = [
        "1.2.3.4:9000",
        "example.com:80",
        "10.0.0.1:80",
        "node.example.org:443",
        "[::ffff:1.2.3.4]:80",
        "not an address",
    ];
    let registrations: Vec<StakeTableEvent> = validators
        .iter()
        .enumerate()
        .map(|(i, v)| match i % 10 {
            // Registered without a network key: never a candidate.
            0 => StakeTableEvent::RegisterV2(ValidatorRegisteredV2::from(v)),
            // A signature over another account: never authenticated.
            1 => {
                let mut reg = ValidatorRegisteredV2::from(v);
                reg.account = random_address(&mut rng);
                StakeTableEvent::RegisterV2(reg)
            },
            _ => {
                let v = v
                    .clone()
                    .with_x25519_key(x25519_key(i as u64))
                    .with_p2p_addr(addrs[i % addrs.len()]);
                StakeTableEvent::RegisterV3(ValidatorRegisteredV3::from(&v))
            },
        })
        .collect();
    let accounts: Vec<Address> = registrations
        .iter()
        .map(|e| match e {
            StakeTableEvent::RegisterV2(r) => r.account,
            StakeTableEvent::RegisterV3(r) => r.account,
            _ => unreachable!(),
        })
        .collect();
    let (_, base) = replay_from("registrations".into(), &registrations, Replayed::default())?;
    let base = base.context("the registrations replay")?;
    let delegator = random_address(&mut rng);
    let mut cases = Vec::new();
    let mut cut_at_tie = 0;
    for round in 0..60 {
        let mut events = registrations.clone();
        // Few distinct stakes, so ties are common. Every third round they are close
        // enough that all candidates are eligible, more than fit in a committee;
        // otherwise a few large ones make the minimum stake leave some out.
        let stakes: &[u64] = match round % 3 {
            0 => &[1, 2, 3, 5, 8, 1000, 5000],
            1 => &[1000, 2000, 3000],
            _ => &[1, 2, 3, 5, 8, 1000, 5000, 1_000_000],
        };
        for &account in &accounts {
            if rng.gen_bool(0.1) {
                continue;
            }
            let amount = U256::from(*stakes.choose(&mut rng).unwrap());
            events.push(StakeTableEvent::Delegate(Delegated {
                delegator,
                validator: account,
                amount,
            }));
        }
        for &account in accounts.choose_multiple(&mut rng, 5) {
            events.push(StakeTableEvent::Deregister(ValidatorExit {
                validator: account,
            }));
        }
        let (case, state) = replay_from(format!("selection round {round}"), &events, base.clone())?;
        let state = state.context("a selection round's replay")?.state;
        cases.push(case);
        if let Ok(active) =
            select_active_validator_set(state.validators(), CANONICAL_STAKE_TABLE_ORDER_VERSION)
        {
            cut_at_tie += usize::from(cut_among_ties(&state, &active));
            let committee: Vec<_> = active
                .values()
                .map(|v| (*v.stake_table_key(), v.stake))
                .collect();
            cases.push(schedule_case(&committee, random_drb(&mut rng), 200));
        }
    }
    ensure!(cut_at_tie > 0, "no committee was cut off among tied stakes");
    Ok(cases)
}

/// Whether the committee `active` was cut off at `MAX_VALIDATORS` with a candidate left
/// out whose stake equals the smallest a member holds.
fn cut_among_ties(state: &StakeTableState, active: &AuthenticatedValidatorMap) -> bool {
    let Some(smallest) = active.values().map(|v| v.stake).min() else {
        return false;
    };
    let candidate = |v: &RegisteredValidator<BLSPubKey>| {
        AuthenticatedValidator::try_from(v).is_ok_and(|v| {
            v.ensure_eligible(CANONICAL_STAKE_TABLE_ORDER_VERSION)
                .is_ok()
        })
    };
    active.len() == MAX_VALIDATORS
        && state
            .validators()
            .iter()
            .any(|(a, v)| !active.contains_key(a) && v.stake == smallest && candidate(v))
}

/// Block rewards, including inputs around a hundredth of the supply staked, where the
/// rate stops growing, and inputs the node rejects.
pub fn rewards() -> Vec<Case> {
    let mut rng = StdRng::seed_from_u64(2);
    let mut inputs = vec![
        (U256::ZERO, U256::ZERO, 2000),
        (U256::from(1), U256::from(2), 2000),
        (U256::from(100), U256::from(1), 2000),
        (U256::from(100), U256::from(1), 0),
        (U256::from(100), U256::from(100), 86_400_000 * 365 + 1),
        (U256::from(100), U256::from(0), 2000),
        // Half the supply staked at 2000 ms a block makes the reward exactly 0.5, 1.5
        // and 2.5: the node rounds a half to the even neighbour.
        (U256::from(525_600_000u64), U256::from(262_800_000u64), 2000),
        (
            U256::from(1_576_800_000u64),
            U256::from(788_400_000u64),
            2000,
        ),
        (
            U256::from(2_628_000_000u64),
            U256::from(1_314_000_000u64),
            2000,
        ),
    ];
    let esp = U256::from(10).pow(U256::from(18));
    for supply in [
        U256::from(100),
        U256::from(12_345_678) * esp,
        U256::from(10_000_000_000u64) * esp,
    ] {
        let hundredth = supply / U256::from(100);
        for stake in [hundredth - U256::ONE, hundredth, hundredth + U256::ONE] {
            inputs.push((supply, stake, 2000));
            inputs.push((supply + U256::from(37), stake, 1999));
        }
    }
    for _ in 0..2000 {
        let supply =
            U256::from(rng.gen_range(1u64..10_000_000_000)) * esp + U256::from(rng.next_u64());
        let stake = match rng.gen_range(0..4) {
            0 => supply / U256::from(rng.gen_range(1u64..1000)),
            1 => supply / U256::from(rng.gen_range(90u64..110)),
            2 => U256::from(rng.next_u64()),
            _ => supply,
        };
        let avg_ms = match rng.gen_range(0..3) {
            0 => 2000,
            1 => rng.gen_range(1..20_000),
            _ => rng.gen_range(1..u64::MAX),
        };
        inputs.push((supply, stake, avg_ms));
    }
    inputs
        .into_iter()
        .map(|(supply, stake, avg_ms)| Case::Reward {
            supply: supply.to_string(),
            stake: stake.to_string(),
            avg_ms,
            reward: compute_block_reward(EpochNumber::new(1), supply, stake, avg_ms)
                .ok()
                .map(|r| r.0.to_string()),
        })
        .collect()
}

/// Validators paid a block reward, with commissions above the maximum and products
/// that overflow among them.
///
/// Fails if no payout failed.
pub fn payouts() -> Result<Vec<Case>> {
    let mut rng = StdRng::seed_from_u64(6);
    let accounts: Vec<Address> = (0..8u8).map(|i| Address::repeat_byte(i + 1)).collect();
    let esp = U256::from(10).pow(U256::from(18));
    let mut cases = Vec::new();
    let mut errors = 0;
    for _ in 0..2000 {
        let account = accounts[rng.gen_range(0..accounts.len())];
        let mut delegators = HashMap::new();
        for _ in 0..rng.gen_range(0..6) {
            let amount = match rng.gen_range(0..3) {
                0 => U256::from(rng.gen_range(1u64..1_000_000)),
                1 => U256::from(rng.gen_range(1u64..10_000_000)) * esp,
                _ => U256::from_limbs([rng.next_u64(), rng.next_u64(), rng.next_u64(), 1 << 60]),
            };
            delegators.insert(accounts[rng.gen_range(0..accounts.len())], amount);
        }
        let stake = delegators.values().fold(U256::ZERO, |s, a| s + a);
        let commission = match rng.gen_range(0..5) {
            0 => 0,
            1 => 10_000,
            2 => rng.gen_range(10_001..=u16::MAX),
            _ => rng.gen_range(0..=10_000),
        };
        let reward = match rng.gen_range(0..4) {
            0 => U256::ZERO,
            1 => U256::from(rng.gen_range(1u64..1_000_000)),
            2 => U256::from(rng.gen_range(1u64..1000)) * esp,
            _ => U256::from_limbs([
                rng.next_u64(),
                rng.next_u64(),
                rng.next_u64(),
                rng.next_u64(),
            ]),
        };
        let mut v = RegisteredValidator::<BLSPubKey>::mock();
        v.account = account;
        v.commission = commission;
        v.stake = stake;
        v.delegators = delegators;
        let validator = AuthenticatedValidator::try_from(v.clone())
            .context("the mock validator is authenticated")?;
        let credits =
            RewardDistributor::new(validator, RewardAmount(reward), RewardAmount::default())
                .compute_rewards()
                .ok()
                .map(|c| {
                    sorted(
                        c.all_rewards()
                            .into_iter()
                            .map(|(a, amount)| (address(&a), amount.0.to_string())),
                    )
                });
        errors += usize::from(credits.is_none());
        cases.push(Case::Payout {
            account: address(&v.account),
            commission: v.commission,
            stake: v.stake.to_string(),
            delegators: sorted(
                v.delegators
                    .iter()
                    .map(|(d, a)| (address(d), a.to_string())),
            ),
            reward: reward.to_string(),
            credits,
        });
    }
    ensure!(errors > 0, "no payout failed");
    Ok(cases)
}

/// Log keys in random order, with repeated blocks.
pub fn orders() -> Result<Vec<Case>> {
    let mut rng = StdRng::seed_from_u64(3);
    (0..200)
        .map(|_| {
            let n = rng.gen_range(0..200);
            let mut keys: Vec<EventKey> = (0..n)
                .map(|_| (rng.gen_range(0..50), rng.gen_range(0..20)))
                .collect();
            keys.sort();
            keys.dedup();
            keys.shuffle(&mut rng);
            order_case(keys)
        })
        .collect()
}

/// Block ranges split into chunks, including an empty range and ranges and sizes at the
/// `u64` bounds.
pub fn chunks() -> Vec<Case> {
    let mut rng = StdRng::seed_from_u64(4);
    let mut inputs = vec![
        (5, 4, 10),
        (5, 5, 1),
        (0, 0, 1),
        (7, 100, 1),
        (0, 100, 100),
        (2, 100, u64::MAX),
        (u64::MAX - 4, u64::MAX, 2),
        (u64::MAX - 5, u64::MAX - 1, 4),
    ];
    for _ in 0..500 {
        let first: u64 = rng.gen_range(0..1000);
        let last = (first + rng.gen_range(0..1000)).saturating_sub(10);
        inputs.push((first, last, rng.gen_range(1..300)));
    }
    inputs
        .into_iter()
        .map(|(first, last, size)| Case::Chunks {
            first,
            last,
            size,
            chunks: {
                let size = NonZeroU64::new(size).expect("sizes are drawn from 1 up");
                Fetcher::block_range_chunks(first, last, size).collect()
            },
        })
        .collect()
}

/// Committees with stakes from tiny to large, each with the leaders of many views.
pub fn schedules() -> Vec<Case> {
    let mut rng = StdRng::seed_from_u64(5);
    (0..50u8)
        .map(|round| {
            let n = rng.gen_range(1..60);
            let committee: Vec<_> = (0..n)
                .map(|i| {
                    let (key, _) = BLSPubKey::generated_from_seed_indexed([round; 32], i);
                    let stake = match rng.gen_range(0..3) {
                        0 => U256::from(rng.gen_range(1u64..10)),
                        1 => U256::from(rng.next_u64()),
                        _ => U256::from(rng.next_u64()) * U256::from(rng.next_u64()),
                    };
                    (key, stake)
                })
                .collect();
            schedule_case(&committee, random_drb(&mut rng), 300)
        })
        .collect()
}

/// A replay so far: the node's state and the outcome of each event.
#[derive(Clone, Default)]
struct Replayed {
    state: StakeTableState,
    outcomes: Vec<String>,
}

/// Replay the events of `events` after those `so_far` covers, in the node: each
/// outcome, the state if the replay finished, and the active validators under the
/// canonical order. Also the replay, if it finished.
fn replay_from(
    name: String,
    events: &[StakeTableEvent],
    so_far: Replayed,
) -> Result<(Case, Option<Replayed>)> {
    let Replayed {
        mut state,
        mut outcomes,
    } = so_far;
    let mut finished = true;
    for event in &events[outcomes.len()..] {
        let result = state.apply_event(event.clone());
        outcomes.push(
            outcome(&result).with_context(|| format!("{name}: an error Lean does not model"))?,
        );
        if result.is_err() {
            finished = false;
            break;
        }
    }
    let selections = if finished {
        selections_of(&name, &state)?
    } else {
        Vec::new()
    };
    let case = Case::Replay {
        events: events.iter().map(abstract_event).collect(),
        outcomes: outcomes.clone(),
        state: finished.then(|| node_state(&state)),
        selections,
        name,
    };
    Ok((case, finished.then_some(Replayed { state, outcomes })))
}

/// The active validators under the canonical order.
fn selections_of(name: &str, state: &StakeTableState) -> Result<Vec<Selection>> {
    let usable: Vec<String> = state
        .validators()
        .values()
        .filter_map(|v| v.p2p_addr.as_ref())
        .filter(|a| a.validate().is_ok())
        .map(|a| a.to_string())
        .collect();
    let version = CANONICAL_STAKE_TABLE_ORDER_VERSION;
    let active = match select_active_validator_set(state.validators(), version) {
        Ok(active) => Some(active.keys().map(address).collect()),
        Err(StakeTableError::NoValidValidators) => None,
        Err(err) => anyhow::bail!("{name}: selection at {version}: {err}"),
    };
    Ok(vec![Selection { usable, active }])
}

/// Sort logs with these keys in the node.
fn order_case(keys: Vec<EventKey>) -> Result<Case> {
    let logs: Vec<_> = keys
        .iter()
        .enumerate()
        .map(|(i, &(block, index))| {
            let event = StakeTableV3Events::Delegated(Delegated {
                delegator: Address::ZERO,
                validator: Address::ZERO,
                amount: U256::from(i),
            });
            let log = Log {
                block_number: Some(block),
                log_index: Some(index),
                ..Default::default()
            };
            (event, log)
        })
        .collect();
    let order = sort_stake_table_events(logs)?
        .into_iter()
        .map(|(_, e)| match e {
            StakeTableEvent::Delegate(d) => Ok(d.amount.to::<usize>()),
            e => anyhow::bail!("sorting returned another event: {e:?}"),
        })
        .collect::<Result<_>>()?;
    Ok(Case::Order { keys, order })
}

/// The schedule order of a committee and the leaders of `views` views.
///
/// A view's hash is the one the node's `select_randomized_leader` draws the leader from.
fn schedule_case(committee: &[(BLSPubKey, U256)], drb: [u8; 32], views: u64) -> Case {
    let position = |key: &BLSPubKey| {
        committee
            .iter()
            .position(|(k, _)| k == key)
            .expect("an entry of the committee")
    };
    let entries: Vec<_> = committee
        .iter()
        .map(|(k, s)| StakeTableEntry {
            stake_key: *k,
            stake_amount: *s,
        })
        .collect();
    let randomized = generate_stake_cdf(entries, drb);
    let order: Vec<usize> = randomized
        .entries()
        .map(|e| position(&e.stake_key))
        .collect();
    let hashes = (0..views)
        .map(|view| randomized.view_hash(view).to_string())
        .collect();
    let leaders = (0..views)
        .map(|view| position(&select_randomized_leader(&randomized, view).stake_key))
        .collect();
    Case::Schedule {
        drb: hex::encode_prefixed(drb),
        entries: committee
            .iter()
            .map(|(k, s)| (hex::encode_prefixed(k.to_bytes()), s.to_string()))
            .collect(),
        order,
        hashes,
        leaders,
    }
}

/// Generated events over a fixed set of validators, keys and addresses.
struct Pool {
    validators: Vec<TestValidator>,
    rekeys: Vec<TestValidator>,
    delegators: Vec<Address>,
    x25519: Vec<[u8; 32]>,
    addrs: Vec<&'static str>,
}

impl Pool {
    fn new(rng: &mut StdRng, n: usize) -> Self {
        let validators: Vec<_> = (0..n).map(|_| TestValidator::from_rng(rng)).collect();
        let rekeys = validators
            .iter()
            .flat_map(|v| {
                [(), ()].map(|_| TestValidator::keys_from_rng(rng, v.account, v.commission))
            })
            .collect();
        let mut x25519: Vec<[u8; 32]> = (0..n as u64).map(x25519_key).collect();
        x25519.push([0; 32]);
        Self {
            validators,
            rekeys,
            delegators: (0..4).map(|_| random_address(rng)).collect(),
            x25519,
            addrs: vec![
                "1.2.3.4:9000",
                "example.com:80",
                "[::1]:9977",
                "[::ffff:1.2.3.4]:80",
                "",
                "not an address",
                "a..b:1",
            ],
        }
    }

    /// The outcomes, other than applying, that the node gives for some event. A
    /// delegation that overflows is left out: the node panics on it.
    fn targets() -> Vec<&'static str> {
        vec![
            "skipped:schnorrKeyAlreadyUsed",
            "skipped:invalidBlsKey",
            "skipped:invalidSchnorrKey",
            "fatal:alreadyRegistered",
            "fatal:validatorNotFound",
            "fatal:delegatorNotFound",
            "fatal:blsKeyAlreadyUsed",
            "fatal:validatorAlreadyExited",
            "fatal:zeroDelegatorStake",
            "fatal:insufficientStake",
            "fatal:authenticationFailed",
            "fatal:invalidCommission",
            "fatal:x25519KeyAlreadyUsed",
        ]
    }

    /// A sequence of up to `len` events the contract could emit: an event that would
    /// be fatal is drawn again. With a `target`, one more event follows, drawn until
    /// it has that outcome, if one turns up.
    ///
    /// Each draw is tried on a copy of the node's state so far.
    fn sequence(
        &self,
        rng: &mut StdRng,
        len: usize,
        target: Option<&str>,
    ) -> Result<Vec<StakeTableEvent>> {
        let mut events = Vec::new();
        let mut state = StakeTableState::default();
        let try_event = |state: &StakeTableState, event: &StakeTableEvent| {
            let mut next = state.clone();
            let result = next.apply_event(event.clone());
            (outcome(&result), next)
        };
        for _ in 0..len * 50 {
            if events.len() == len {
                break;
            }
            let event = self.event(rng);
            if let (Some(o), next) = try_event(&state, &event)
                && !o.starts_with("fatal")
            {
                state = next;
                events.push(event);
            }
        }
        if let Some(target) = target {
            for _ in 0..2000 {
                let event = self.event(rng);
                if try_event(&state, &event).0.as_deref() == Some(target) {
                    events.push(event);
                    break;
                }
            }
        }
        Ok(events)
    }

    /// An event drawn from the pool: a registration, an exit or a change of stake, a key
    /// update, or a change to a validator's commission or network details.
    fn event(&self, rng: &mut StdRng) -> StakeTableEvent {
        match rng.gen_range(0..20) {
            0..=5 => self.registration(rng),
            6..=12 => self.exit_or_stake(rng),
            13..=15 => self.key_update(rng),
            _ => self.validator_update(rng),
        }
    }

    /// A registration as each version of the contract emits it (`StakeTable`, `StakeTableV2`,
    /// `StakeTableV3`), some with another validator's keys
    /// or a signature over another account.
    fn registration(&self, rng: &mut StdRng) -> StakeTableEvent {
        let v = self.validators.choose(rng).unwrap();
        let other = self.validators.choose(rng).unwrap();
        match rng.gen_range(0..6) {
            0 => StakeTableEvent::Register(ValidatorRegistered::from(v)),
            1 => {
                // The consensus key of one validator with the light client key of another.
                let mut reg = ValidatorRegistered::from(self.rekeys.choose(rng).unwrap());
                reg.schnorrVk = other.schnorr_vk;
                StakeTableEvent::Register(reg)
            },
            2 => StakeTableEvent::RegisterV2(ValidatorRegisteredV2::from(v)),
            3 => {
                let mut reg = ValidatorRegisteredV2::from(v);
                reg.account = other.account;
                StakeTableEvent::RegisterV2(reg)
            },
            _ => {
                let mut reg = ValidatorRegisteredV3::from(v);
                reg.x25519Key = FixedBytes(*self.x25519.choose(rng).unwrap());
                reg.p2pAddr = self.addrs.choose(rng).unwrap().to_string();
                StakeTableEvent::RegisterV3(reg)
            },
        }
    }

    /// An exit, a delegation or a withdrawal, mostly about a registered validator.
    fn exit_or_stake(&self, rng: &mut StdRng) -> StakeTableEvent {
        let validator = self.account(rng);
        let delegator = *self.delegators.choose(rng).unwrap();
        let amount = match rng.gen_range(0..20) {
            0 => U256::ZERO,
            1 => U256::from(rng.next_u64()) << 64,
            _ => U256::from(rng.gen_range(1u64..1000)),
        };
        let withdrawn = amount / U256::from(rng.gen_range(1u64..4));
        match rng.gen_range(0..7) {
            0 => StakeTableEvent::Deregister(ValidatorExit { validator }),
            1 => StakeTableEvent::DeregisterV2(ValidatorExitV2 {
                validator,
                unlocksAt: U256::ZERO,
            }),
            2..=4 => StakeTableEvent::Delegate(Delegated {
                delegator,
                validator,
                amount,
            }),
            5 => StakeTableEvent::Undelegate(Undelegated {
                delegator,
                validator,
                amount: withdrawn,
            }),
            _ => StakeTableEvent::UndelegateV2(UndelegatedV2 {
                delegator,
                validator,
                undelegationId: 0,
                amount: withdrawn,
                unlocksAt: U256::ZERO,
            }),
        }
    }

    /// A key update as `StakeTable` or `StakeTableV2` emits it, some with keys that do not decode or a
    /// signature over another account.
    fn key_update(&self, rng: &mut StdRng) -> StakeTableEvent {
        let rekey = self.rekeys.choose(rng).unwrap();
        // Points not on their curves: keys that do not decode.
        let bad_bls = G2PointSol {
            x0: U256::from(1),
            x1: U256::from(2),
            y0: U256::from(3),
            y1: U256::from(4),
        };
        let bad_schnorr = EdOnBN254PointSol {
            x: U256::from(5),
            y: U256::from(6),
        };
        match rng.gen_range(0..3) {
            0 => StakeTableEvent::KeyUpdate(ConsensusKeysUpdated::from(rekey)),
            1 => {
                let mut update = ConsensusKeysUpdated::from(rekey);
                if rng.gen_bool(0.5) {
                    update.blsVK = bad_bls;
                } else {
                    update.schnorrVK = bad_schnorr;
                }
                StakeTableEvent::KeyUpdate(update)
            },
            _ => {
                let mut update = ConsensusKeysUpdatedV2::from(rekey);
                match rng.gen_range(0..4) {
                    0 => update.account = self.validators.choose(rng).unwrap().account,
                    1 => update.blsVK = bad_bls,
                    2 => update.schnorrVK = bad_schnorr,
                    _ => {},
                }
                StakeTableEvent::KeyUpdateV2(update)
            },
        }
    }

    /// A change of commission, network key or network address.
    fn validator_update(&self, rng: &mut StdRng) -> StakeTableEvent {
        let validator = self.account(rng);
        match rng.gen_range(0..4) {
            0 => StakeTableEvent::CommissionUpdate(CommissionUpdated {
                validator,
                timestamp: U256::ZERO,
                oldCommission: 0,
                newCommission: rng.gen_range(0..10_500),
            }),
            1 => StakeTableEvent::X25519KeyUpdate(X25519KeyUpdated {
                validator,
                x25519Key: FixedBytes(*self.x25519.choose(rng).unwrap()),
            }),
            _ => StakeTableEvent::P2pAddrUpdate(P2pAddrUpdated {
                validator,
                p2pAddr: self.addrs.choose(rng).unwrap().to_string(),
            }),
        }
    }

    /// The account of a validator in the pool.
    fn account(&self, rng: &mut StdRng) -> Address {
        self.validators.choose(rng).unwrap().account
    }
}

/// An outcome as the corpus names it: `applied`, `skipped:<error>` or `fatal:<error>`,
/// or none for an error the Lean implementation does not model.
fn outcome(
    result: &Result<Result<(), ExpectedStakeTableError>, StakeTableError>,
) -> Option<String> {
    let name = match result {
        Ok(Ok(())) => return Some("applied".into()),
        Ok(Err(err)) => match err {
            ExpectedStakeTableError::SchnorrKeyAlreadyUsed(_) => "skipped:schnorrKeyAlreadyUsed",
            ExpectedStakeTableError::InvalidBlsKey => "skipped:invalidBlsKey",
            ExpectedStakeTableError::InvalidSchnorrKey => "skipped:invalidSchnorrKey",
        },
        Err(err) => match err {
            StakeTableError::AlreadyRegistered(_) => "fatal:alreadyRegistered",
            StakeTableError::ValidatorNotFound(_) => "fatal:validatorNotFound",
            StakeTableError::DelegatorNotFound(_) => "fatal:delegatorNotFound",
            StakeTableError::BlsKeyAlreadyUsed(_) => "fatal:blsKeyAlreadyUsed",
            StakeTableError::ValidatorAlreadyExited(_) => "fatal:validatorAlreadyExited",
            StakeTableError::ZeroDelegatorStake(_) => "fatal:zeroDelegatorStake",
            StakeTableError::InsufficientStake => "fatal:insufficientStake",
            StakeTableError::AuthenticationFailed(_) => "fatal:authenticationFailed",
            StakeTableError::InvalidCommission(..) => "fatal:invalidCommission",
            StakeTableError::X25519KeyAlreadyUsed(_) => "fatal:x25519KeyAlreadyUsed",
            _ => return None,
        },
    };
    Some(name.into())
}

/// A valid network key, distinct for each `i`.
fn x25519_key(i: u64) -> [u8; 32] {
    x25519::Keypair::generated_from_seed_indexed([3; 32], i)
        .unwrap()
        .public_key()
        .as_slice()
        .try_into()
        .unwrap()
}

fn random_drb(rng: &mut StdRng) -> [u8; 32] {
    let mut drb = [0; 32];
    rng.fill_bytes(&mut drb);
    drb
}

fn bls_hex(key: &BLSPubKey) -> String {
    hex::encode_prefixed(key.to_bytes())
}

fn schnorr_hex(key: &SchnorrPubKey) -> String {
    let mut bytes = vec![];
    key.serialize_with_mode(&mut bytes, ark_serialize::Compress::Yes)
        .expect("serializing to a vector does not fail");
    hex::encode_prefixed(bytes)
}

fn x25519_hex(key: &x25519::PublicKey) -> String {
    hex::encode_prefixed(key.as_slice())
}

fn address(a: &Address) -> String {
    hex::encode_prefixed(a.as_slice())
}

/// The network key the node takes from these bytes, if any.
fn x25519_of(bytes: [u8; 32]) -> Option<String> {
    parse_x25519_key(bytes).ok().as_ref().map(x25519_hex)
}

fn random_address(rng: &mut StdRng) -> Address {
    let mut bytes = [0u8; 20];
    rng.fill_bytes(&mut bytes);
    Address::from(bytes)
}

/// Sorted, so that the order a hash map gave does not change the corpus.
fn sorted(pairs: impl Iterator<Item = (String, String)>) -> Vec<(String, String)> {
    let mut pairs: Vec<_> = pairs.collect();
    pairs.sort();
    pairs
}

fn sorted_strings(strings: impl Iterator<Item = String>) -> Vec<String> {
    let mut strings: Vec<_> = strings.collect();
    strings.sort();
    strings
}

fn parse_p2p_addr(addr: &str) -> Option<String> {
    addr.parse::<NetAddr>().ok().map(|a| a.to_string())
}

/// `authenticate`, remembered per signed content: generated events reuse signatures,
/// and checking one costs a pairing.
fn authenticated<T: std::fmt::Debug>(
    signed: &T,
    authenticate: impl FnOnce() -> Result<(BLSPubKey, SchnorrPubKey), StakeTableSolError>,
) -> Result<(BLSPubKey, SchnorrPubKey), StakeTableSolError> {
    use std::cell::RefCell;

    type Verdict = Result<(BLSPubKey, SchnorrPubKey), String>;
    thread_local! {
        static SEEN: RefCell<HashMap<String, Verdict>> = RefCell::new(HashMap::new());
    }
    let key = format!("{signed:?}");
    if let Some(v) = SEEN.with(|s| s.borrow().get(&key).cloned()) {
        return v.map_err(|e| match e.as_str() {
            "InvalidBlsKey" => StakeTableSolError::InvalidBlsKey,
            "InvalidSchnorrKey" => StakeTableSolError::InvalidSchnorrKey,
            _ => StakeTableSolError::InvalidBlsSignature,
        });
    }
    let result = authenticate();
    let verdict = match &result {
        Ok(keys) => Ok(keys.clone()),
        Err(StakeTableSolError::InvalidBlsKey) => Err("InvalidBlsKey".into()),
        Err(StakeTableSolError::InvalidSchnorrKey) => Err("InvalidSchnorrKey".into()),
        Err(_) => Err("Failed".into()),
    };
    SEEN.with(|s| s.borrow_mut().insert(key, verdict));
    result
}

/// An event with its keys decoded and checked as `apply_event` decodes and checks them.
fn abstract_event(event: &StakeTableEvent) -> Event {
    match event {
        StakeTableEvent::Register(reg) => Event::Register {
            account: address(&reg.account),
            bls: decoded_bls(reg.blsVk),
            schnorr: decoded_schnorr(reg.schnorrVk),
            commission: reg.commission,
        },
        StakeTableEvent::RegisterV2(reg) => Event::RegisterV2 {
            account: address(&reg.account),
            keys: registration_keys(
                &(
                    reg.account,
                    reg.blsVK,
                    reg.schnorrVK,
                    &reg.blsSig,
                    &reg.schnorrSig,
                ),
                reg.blsVK,
                reg.schnorrVK,
                || reg.authenticate(),
            ),
            commission: reg.commission,
        },
        StakeTableEvent::RegisterV3(reg) => Event::RegisterV3 {
            account: address(&reg.account),
            keys: registration_keys(
                &(
                    reg.account,
                    reg.blsVK,
                    reg.schnorrVK,
                    &reg.blsSig,
                    &reg.schnorrSig,
                ),
                reg.blsVK,
                reg.schnorrVK,
                || reg.authenticate(),
            ),
            commission: reg.commission,
            x25519: x25519_of(reg.x25519Key.0),
            p2p_addr: parse_p2p_addr(&reg.p2pAddr),
        },
        StakeTableEvent::Deregister(ValidatorExit { validator })
        | StakeTableEvent::DeregisterV2(ValidatorExitV2 { validator, .. }) => Event::Deregister {
            validator: address(validator),
        },
        StakeTableEvent::Delegate(d) => Event::Delegate {
            delegator: address(&d.delegator),
            validator: address(&d.validator),
            amount: d.amount.to_string(),
        },
        StakeTableEvent::Undelegate(Undelegated {
            delegator,
            validator,
            amount,
        })
        | StakeTableEvent::UndelegateV2(UndelegatedV2 {
            delegator,
            validator,
            amount,
            ..
        }) => Event::Undelegate {
            delegator: address(delegator),
            validator: address(validator),
            amount: amount.to_string(),
        },
        StakeTableEvent::KeyUpdate(update) => Event::KeyUpdate {
            validator: address(&update.account),
            bls: decoded_bls(update.blsVK),
            schnorr: decoded_schnorr(update.schnorrVK),
        },
        StakeTableEvent::KeyUpdateV2(update) => Event::KeyUpdateV2 {
            validator: address(&update.account),
            keys: key_update_keys(update),
        },
        StakeTableEvent::CommissionUpdate(update) => Event::CommissionUpdate {
            validator: address(&update.validator),
            commission: update.newCommission,
        },
        StakeTableEvent::X25519KeyUpdate(update) => Event::X25519Update {
            validator: address(&update.validator),
            key: x25519_of(update.x25519Key.0),
        },
        StakeTableEvent::P2pAddrUpdate(update) => Event::P2pAddrUpdate {
            validator: address(&update.validator),
            addr: parse_p2p_addr(&update.p2pAddr),
        },
    }
}

fn decoded_bls(key: G2PointSol) -> Option<String> {
    BLSPubKey::try_from(key).ok().as_ref().map(bls_hex)
}

fn decoded_schnorr(key: EdOnBN254PointSol) -> Option<String> {
    SchnorrPubKey::try_from(key).ok().as_ref().map(schnorr_hex)
}

/// The keys of a registration emitted by `StakeTableV2` or later: verified, or the
/// ones that decode.
fn registration_keys(
    signed: &impl std::fmt::Debug,
    bls: G2PointSol,
    schnorr: EdOnBN254PointSol,
    authenticate: impl FnOnce() -> Result<(BLSPubKey, SchnorrPubKey), StakeTableSolError>,
) -> RegistrationKeys {
    match authenticated(signed, authenticate) {
        Ok((bls, schnorr)) => RegistrationKeys::Verified {
            bls: bls_hex(&bls),
            schnorr: schnorr_hex(&schnorr),
        },
        Err(_) => RegistrationKeys::Unverified {
            bls: decoded_bls(bls),
            schnorr: decoded_schnorr(schnorr),
        },
    }
}

/// The verdict on the keys of a key update emitted by `StakeTableV2` or later.
fn key_update_keys(update: &ConsensusKeysUpdatedV2) -> KeyUpdateKeys {
    let signed = (
        update.account,
        update.blsVK,
        update.schnorrVK,
        &update.blsSig,
        &update.schnorrSig,
    );
    match authenticated(&signed, || update.authenticate()) {
        Ok((bls, schnorr)) => KeyUpdateKeys::Verified {
            bls: bls_hex(&bls),
            schnorr: schnorr_hex(&schnorr),
        },
        Err(StakeTableSolError::InvalidBlsKey) => KeyUpdateKeys::InvalidBls,
        Err(StakeTableSolError::InvalidSchnorrKey) => KeyUpdateKeys::InvalidSchnorr,
        Err(_) => KeyUpdateKeys::Failed,
    }
}

/// The node's state in the corpus's form.
fn node_state(state: &StakeTableState) -> State {
    let validator = |v: &RegisteredValidator<BLSPubKey>| Validator {
        account: address(&v.account),
        bls: v.stake_table_key.as_ref().map(bls_hex),
        schnorr: v.state_ver_key.as_ref().map(schnorr_hex),
        x25519: v.x25519_key.as_ref().map(x25519_hex),
        p2p_addr: v.p2p_addr.as_ref().map(|a| a.to_string()),
        stake: v.stake.to_string(),
        commission: v.commission,
        authenticated: v.authenticated,
        delegators: sorted(
            v.delegators
                .iter()
                .map(|(d, n)| (address(d), n.to_string())),
        ),
    };
    State {
        validators: state.validators().values().map(validator).collect(),
        used_bls: sorted_strings(state.used_bls_keys().iter().map(bls_hex)),
        used_schnorr: sorted_strings(state.used_schnorr_keys().iter().map(schnorr_hex)),
        used_x25519: sorted_strings(state.used_x25519_keys().iter().map(x25519_hex)),
        exits: sorted_strings(state.validator_exits().iter().map(address)),
    }
}
