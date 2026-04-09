use std::sync::LazyLock;

use alloy::primitives::{U256, utils::parse_ether};
use chrono::{Months, NaiveDate, NaiveDateTime, NaiveTime};
use espresso_types::{v0_1::ChainId, v0_3::RewardAmount};
use serde::Deserialize;

const SCHEDULE_TOML: &str = include_str!("../../../../../data/token-unlock-schedule.toml");

pub(crate) const MAINNET_CHAIN_ID: u64 = 1;

#[derive(Deserialize)]
struct UnlockEntry {
    month: u64,
    amount_esp: u64,
}

#[derive(Deserialize)]
struct UnlockSchedule {
    tge_date: String,
    unlocks: Vec<UnlockEntry>,
}

struct UnlockCliff {
    timestamp: u64,
    amount_wei: U256,
}

struct ParsedSchedule {
    /// Sorted by timestamp. Each entry is a calendar month boundary from TGE
    /// with the cumulative unlocked amount at that point.
    unlocks: Vec<UnlockCliff>,
}

static SCHEDULE: LazyLock<ParsedSchedule> = LazyLock::new(|| {
    let schedule: UnlockSchedule =
        toml::from_str(SCHEDULE_TOML).expect("valid token-unlock-schedule.toml");

    let tge_date =
        NaiveDate::parse_from_str(&schedule.tge_date, "%Y-%m-%d").expect("valid tge_date");

    assert!(
        schedule.unlocks.windows(2).all(|w| w[0].month < w[1].month),
        "unlock schedule must be sorted by month"
    );

    let unlocks = schedule
        .unlocks
        .iter()
        .map(|entry| {
            let date = tge_date
                .checked_add_months(Months::new(
                    u32::try_from(entry.month).expect("month fits in u32"),
                ))
                .expect("valid calendar month from TGE");
            let ts = NaiveDateTime::new(date, NaiveTime::MIN)
                .and_utc()
                .timestamp();
            assert!(ts >= 0, "unlock dates must be after unix epoch");
            let timestamp = ts as u64;
            let amount_wei = parse_ether(&entry.amount_esp.to_string()).unwrap();
            UnlockCliff {
                timestamp,
                amount_wei,
            }
        })
        .collect();

    ParsedSchedule { unlocks }
});

/// Unlocked token amount (in WEI) at the given unix timestamp.
///
/// Cliff-based unlock using calendar months from TGE: the amount stays at the
/// previous month's value until the next calendar month boundary is reached.
pub fn unlocked_amount_at(timestamp_secs: u64) -> U256 {
    SCHEDULE
        .unlocks
        .iter()
        .rev()
        .find(|cliff| timestamp_secs >= cliff.timestamp)
        .map(|cliff| cliff.amount_wei)
        .unwrap_or(U256::ZERO)
}

/// Computes token supply metrics from on-chain data and the unlock schedule.
///
/// - `total_issued          = initial_supply + reward_distributed`
/// - `circulating           = initial_supply + reward_distributed - locked`
/// - `circulating_ethereum  = total_supply_l1 - locked`
///
/// `locked` is the only mainnet/non-mainnet branching point:
/// - Mainnet: `locked = initial_supply - unlocked(now)`
/// - Non-mainnet: `locked = 0` (no unlock schedule)
pub struct SupplyCalculator {
    chain_id: U256,
    now_secs: u64,
    initial_supply: U256,
    total_supply_l1: U256,
    total_reward_distributed: U256,
}

impl SupplyCalculator {
    pub fn new(
        chain_id: ChainId,
        now_secs: u64,
        initial_supply: U256,
        total_supply_l1: U256,
        total_reward_distributed: Option<RewardAmount>,
    ) -> Self {
        Self {
            chain_id: chain_id.0,
            now_secs,
            initial_supply,
            total_supply_l1,
            total_reward_distributed: total_reward_distributed.map(|r| r.0).unwrap_or(U256::ZERO),
        }
    }

    /// Tokens still locked on L1 per the unlock schedule.
    /// Mainnet: `initial_supply - unlocked(now)`. Non-mainnet: `0`.
    fn locked(&self) -> U256 {
        if self.chain_id == U256::from(MAINNET_CHAIN_ID) {
            self.initial_supply
                .saturating_sub(unlocked_amount_at(self.now_secs))
        } else {
            U256::ZERO
        }
    }

    /// Circulating supply across Espresso + Ethereum.
    /// `= initial_supply + reward_distributed - locked`
    pub fn circulating_supply(&self) -> U256 {
        self.initial_supply
            .saturating_add(self.total_reward_distributed)
            .saturating_sub(self.locked())
    }

    /// Circulating supply on Ethereum L1 only.
    /// `= total_supply_l1 - locked`
    pub fn circulating_supply_ethereum(&self) -> U256 {
        self.total_supply_l1.saturating_sub(self.locked())
    }

    /// Total issued supply: `initial_supply + total_reward_distributed`.
    pub fn total_issued_supply(&self) -> U256 {
        self.initial_supply
            .saturating_add(self.total_reward_distributed)
    }

    /// Total rewards distributed by consensus.
    pub fn total_reward_distributed(&self) -> U256 {
        self.total_reward_distributed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn month_boundary(n: usize) -> u64 {
        SCHEDULE.unlocks[n].timestamp
    }

    fn tge_timestamp() -> u64 {
        month_boundary(0)
    }

    #[test]
    fn test_before_tge() {
        assert_eq!(unlocked_amount_at(0), U256::ZERO);
        assert_eq!(unlocked_amount_at(tge_timestamp() - 1), U256::ZERO);
    }

    #[test]
    fn test_at_tge() {
        let expected = parse_ether("520550000").unwrap();
        assert_eq!(unlocked_amount_at(tge_timestamp()), expected);
    }

    #[test]
    fn test_after_last_month() {
        let far_future = tge_timestamp() + 10 * 365 * 86400; // ~10 years
        let expected = parse_ether("3590000000").unwrap();
        assert_eq!(unlocked_amount_at(far_future), expected);
    }

    #[test]
    fn test_cliff_mid_month_equals_start_of_month() {
        // Mid-month should return the same amount as the start of the month (cliff behavior)
        let half_month = (month_boundary(0) + month_boundary(1)) / 2;
        let result = unlocked_amount_at(half_month);
        let expected = parse_ether("520550000").unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_cliff_just_before_next_month() {
        // One second before month 1 boundary should still return month 0 amount
        let just_before_month_1 = month_boundary(1) - 1;
        let expected = parse_ether("520550000").unwrap();
        assert_eq!(unlocked_amount_at(just_before_month_1), expected);
    }

    #[test]
    fn test_at_exact_month_boundary() {
        let expected = parse_ether("540400476").unwrap();
        assert_eq!(unlocked_amount_at(month_boundary(1)), expected);

        let expected = parse_ether("1198597668").unwrap();
        assert_eq!(unlocked_amount_at(month_boundary(12)), expected);
    }

    #[test]
    fn test_at_month_72() {
        let expected = parse_ether("3590000000").unwrap();
        assert_eq!(unlocked_amount_at(month_boundary(72)), expected);
    }

    #[test]
    fn test_schedule_is_monotonically_increasing() {
        for window in SCHEDULE.unlocks.windows(2) {
            assert!(window[1].amount_wei >= window[0].amount_wei);
            assert!(window[1].timestamp > window[0].timestamp);
        }
    }

    #[test]
    fn test_schedule_has_expected_entries() {
        assert_eq!(SCHEDULE.unlocks.len(), 73);
    }

    #[test]
    fn test_tge_date_parses() {
        use chrono::{Datelike, TimeZone, Utc};
        let dt = Utc
            .timestamp_opt(SCHEDULE.unlocks[0].timestamp as i64, 0)
            .single()
            .unwrap();
        assert_eq!(dt.year(), 2026);
        assert_eq!(dt.month(), 2);
        assert_eq!(dt.day(), 12);
    }

    // --- SupplyCalculator tests ---

    fn mainnet_id() -> ChainId {
        ChainId(U256::from(MAINNET_CHAIN_ID))
    }

    fn testnet_id() -> ChainId {
        ChainId(U256::from(35353u64))
    }

    fn post_tge_time() -> u64 {
        month_boundary(6)
    }

    /// Mainnet initial supply (3.59B tokens) -- must be >= final unlock schedule amount.
    fn mainnet_initial_supply() -> U256 {
        parse_ether("3590000000").unwrap()
    }

    #[test]
    fn test_locked_mainnet() {
        let now = post_tge_time();
        let initial_supply = mainnet_initial_supply();
        let total_supply_l1 = initial_supply + parse_ether("50").unwrap();
        let calc = SupplyCalculator::new(mainnet_id(), now, initial_supply, total_supply_l1, None);

        let unlocked = unlocked_amount_at(now);
        assert_eq!(calc.locked(), initial_supply - unlocked);
    }

    #[test]
    fn test_locked_non_mainnet() {
        let now = post_tge_time();
        let calc = SupplyCalculator::new(
            testnet_id(),
            now,
            parse_ether("1000").unwrap(),
            parse_ether("1000").unwrap(),
            None,
        );
        assert_eq!(calc.locked(), U256::ZERO);
    }

    #[test]
    fn test_locked_before_tge() {
        // Before TGE, nothing is unlocked so locked = entire initial supply
        let before_tge = tge_timestamp() - 1;
        let initial_supply = mainnet_initial_supply();
        let calc = SupplyCalculator::new(
            mainnet_id(),
            before_tge,
            initial_supply,
            initial_supply,
            None,
        );
        assert_eq!(calc.locked(), initial_supply);
    }

    #[test]
    fn test_locked_after_full_vest() {
        // After month 72, everything is unlocked so locked = 0
        // (assuming initial_supply <= final unlock amount)
        let far_future = tge_timestamp() + 100 * 365 * 86400;
        let initial_supply = mainnet_initial_supply();
        let calc = SupplyCalculator::new(
            mainnet_id(),
            far_future,
            initial_supply,
            initial_supply,
            None,
        );
        assert_eq!(calc.locked(), U256::ZERO);
    }

    #[test]
    fn test_supply_calculator_mainnet_circulating() {
        let now = post_tge_time();
        let initial_supply = mainnet_initial_supply();
        let total_supply_l1 = initial_supply + parse_ether("50").unwrap();
        let reward = Some(RewardAmount(parse_ether("100").unwrap()));
        let calc =
            SupplyCalculator::new(mainnet_id(), now, initial_supply, total_supply_l1, reward);

        // circulating = initial_supply + reward - locked = unlocked + reward
        let unlocked = unlocked_amount_at(now);
        assert_eq!(
            calc.circulating_supply(),
            unlocked + parse_ether("100").unwrap()
        );
    }

    #[test]
    fn test_supply_calculator_mainnet_ethereum() {
        let now = post_tge_time();
        let initial_supply = mainnet_initial_supply();
        let claimed = parse_ether("50").unwrap();
        let total_supply_l1 = initial_supply + claimed;
        let calc = SupplyCalculator::new(mainnet_id(), now, initial_supply, total_supply_l1, None);

        // circulating_ethereum = total_supply_l1 - locked = total_supply_l1 - (initial - unlocked)
        //                      = unlocked + claimed
        let unlocked = unlocked_amount_at(now);
        assert_eq!(calc.circulating_supply_ethereum(), unlocked + claimed);
    }

    #[test]
    fn test_supply_calculator_non_mainnet_circulating() {
        let now = post_tge_time();
        let reward = Some(RewardAmount(parse_ether("200").unwrap()));
        // locked=0: initial_supply + reward = 1000 + 200 = 1200
        let calc = SupplyCalculator::new(
            testnet_id(),
            now,
            parse_ether("1000").unwrap(),
            parse_ether("1000").unwrap(),
            reward,
        );

        assert_eq!(calc.circulating_supply(), parse_ether("1200").unwrap());
    }

    #[test]
    fn test_supply_calculator_non_mainnet_ethereum() {
        let now = post_tge_time();
        let calc = SupplyCalculator::new(
            testnet_id(),
            now,
            parse_ether("5000").unwrap(),
            parse_ether("5000").unwrap(),
            Some(RewardAmount(parse_ether("10").unwrap())),
        );

        // locked=0: total_supply_l1
        assert_eq!(
            calc.circulating_supply_ethereum(),
            parse_ether("5000").unwrap()
        );
    }

    #[test]
    fn test_supply_calculator_invariant() {
        // circulating - circulating_ethereum = initial_supply + reward - total_supply_l1
        // (locked cancels out in subtraction)
        for chain_id in [mainnet_id(), testnet_id()] {
            let now = post_tge_time();
            let initial_supply = parse_ether("10000").unwrap();
            let reward = parse_ether("500").unwrap();
            let total_supply_l1 = parse_ether("10200").unwrap();
            let calc = SupplyCalculator::new(
                chain_id,
                now,
                initial_supply,
                total_supply_l1,
                Some(RewardAmount(reward)),
            );

            // circulating = initial + reward - locked
            // ethereum    = total_supply_l1 - locked
            // diff        = initial + reward - total_supply_l1
            assert_eq!(
                calc.circulating_supply() - calc.circulating_supply_ethereum(),
                initial_supply + reward - total_supply_l1,
            );
        }
    }

    #[test]
    fn test_supply_calculator_zero_rewards() {
        let now = post_tge_time();
        let initial_supply = mainnet_initial_supply();
        let calc = SupplyCalculator::new(mainnet_id(), now, initial_supply, initial_supply, None);

        let unlocked = unlocked_amount_at(now);
        assert_eq!(calc.circulating_supply(), unlocked);
    }

    #[test]
    fn test_supply_calculator_zero_claimed() {
        let now = post_tge_time();
        let initial_supply = parse_ether("1000").unwrap();
        let reward = Some(RewardAmount(parse_ether("100").unwrap()));
        let calc = SupplyCalculator::new(testnet_id(), now, initial_supply, initial_supply, reward);
        assert_eq!(calc.circulating_supply(), parse_ether("1100").unwrap());
    }

    #[test]
    fn test_supply_calculator_no_underflow() {
        // Before TGE: locked = initial_supply, so circulating = 0 via saturating_sub
        let before_tge = tge_timestamp() - 1;
        let calc = SupplyCalculator::new(
            mainnet_id(),
            before_tge,
            parse_ether("100").unwrap(),
            parse_ether("100").unwrap(),
            None,
        );
        assert_eq!(calc.circulating_supply(), U256::ZERO);
    }

    #[test]
    fn test_locked_saturates_when_initial_less_than_unlocked() {
        // If initial_supply < unlocked(now), locked saturates to 0 (not negative)
        let now = post_tge_time();
        let small_initial = parse_ether("100").unwrap();
        let calc = SupplyCalculator::new(mainnet_id(), now, small_initial, small_initial, None);
        assert_eq!(calc.locked(), U256::ZERO);
        // circulating = initial + reward - locked = 100 + 0 - 0 = 100
        assert_eq!(calc.circulating_supply(), small_initial);
    }

    #[test]
    fn test_total_issued_supply_with_rewards() {
        let now = post_tge_time();
        let initial_supply = mainnet_initial_supply();
        let reward = Some(RewardAmount(parse_ether("100").unwrap()));
        let calc = SupplyCalculator::new(mainnet_id(), now, initial_supply, initial_supply, reward);
        assert_eq!(
            calc.total_issued_supply(),
            initial_supply + parse_ether("100").unwrap()
        );
    }

    #[test]
    fn test_total_issued_supply_zero_rewards() {
        let now = post_tge_time();
        let initial_supply = mainnet_initial_supply();
        let calc = SupplyCalculator::new(mainnet_id(), now, initial_supply, initial_supply, None);
        assert_eq!(calc.total_issued_supply(), initial_supply);
    }

    #[test]
    fn test_total_issued_supply_chain_invariant() {
        let now = post_tge_time();
        let initial_supply = parse_ether("10000").unwrap();
        let reward = Some(RewardAmount(parse_ether("500").unwrap()));
        let total_supply_l1 = parse_ether("10200").unwrap();

        let mainnet =
            SupplyCalculator::new(mainnet_id(), now, initial_supply, total_supply_l1, reward);
        let testnet =
            SupplyCalculator::new(testnet_id(), now, initial_supply, total_supply_l1, reward);

        assert_eq!(mainnet.total_issued_supply(), testnet.total_issued_supply());
    }

    #[test]
    fn test_total_reward_distributed() {
        let now = post_tge_time();
        let initial_supply = mainnet_initial_supply();
        let reward = Some(RewardAmount(parse_ether("200").unwrap()));
        let calc = SupplyCalculator::new(mainnet_id(), now, initial_supply, initial_supply, reward);
        assert_eq!(calc.total_reward_distributed(), parse_ether("200").unwrap());
    }

    #[test]
    fn test_total_reward_distributed_zero() {
        let now = post_tge_time();
        let initial_supply = mainnet_initial_supply();
        let calc = SupplyCalculator::new(mainnet_id(), now, initial_supply, initial_supply, None);
        assert_eq!(calc.total_reward_distributed(), U256::ZERO);
    }
}
