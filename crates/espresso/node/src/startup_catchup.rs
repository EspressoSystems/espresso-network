//! Startup stake-table catchup for new-protocol nodes.
//!
//! Cliquenet only connects to validators in the current epoch's stake table
//! window (`N-1`, `N`, `N+1`). On a fresh-join or cold-restart node, no
//! consensus messages can be received until those stake tables are populated,
//! so the existing reactive catchup (triggered by an unknown-epoch proposal)
//! never fires.
//!
//! [`bootstrap_epoch_window`] populates the window in two stages:
//!
//! 1. Skip: the L1 light-client contract vouches for a finalized header `F`. The epoch roots
//!    below `F` are fetched with Merkle proofs against `F` and installed directly.
//! 2. Walk: from the highest known epoch, one epoch at a time, until peers can no longer serve
//!    the next epoch root leaf, which is where the live network currently is.

use std::{ops::RangeInclusive, time::Duration};

use anyhow::{Context, bail, ensure};
use clap::ValueEnum;
use espresso_types::{Header, NodeState, SeqTypes};
use hotshot_contract_adapter::{sol_types::LightClientStateSol, u256_to_field};
use hotshot_query_service::node::BlockId;
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    light_client::CircuitField,
    traits::{
        block_contents::BlockHeader,
        election::Membership,
        metrics::{Gauge, Metrics},
    },
    utils::{epoch_from_block_number, root_block_in_epoch},
};
use light_client::client::Client;
use serde::Serialize;
use tokio::time::{Instant, sleep, timeout};

const TARGET: &str = "announce::bootstrap";

const INITIAL_BACKOFF: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(60);

/// Largest number of epochs between the anchor and the known window for which a failed skip still
/// lets the walk run at deadline.
const MAX_WALKABLE_GAP: u64 = 2;

/// Whether to skip the walk using the L1 anchor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum, Serialize)]
pub enum SkipMode {
    #[default]
    Auto,
    Off,
}

#[derive(Clone, Copy, Debug)]
pub struct BootstrapParams {
    /// Bound on a single `wait_for_stake_table` call during the walk.
    pub step_timeout: Duration,
    /// How long the skip may keep retrying before startup gives up.
    pub deadline: Duration,
    pub skip: SkipMode,
}

/// A finalized HotShot block as the L1 light-client contract commits to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct L1Anchor {
    pub height: u64,
    pub block_comm_root: CircuitField,
}

impl From<LightClientStateSol> for L1Anchor {
    fn from(state: LightClientStateSol) -> Self {
        Self {
            height: state.blockHeight,
            block_comm_root: u256_to_field(state.blockCommRoot),
        }
    }
}

pub struct BootstrapMetrics {
    window_epoch: Box<dyn Gauge>,
    l1_anchor_epoch: Box<dyn Gauge>,
}

impl BootstrapMetrics {
    pub fn new(metrics: &dyn Metrics) -> Self {
        Self {
            window_epoch: metrics.create_gauge("bootstrap_window_epoch".into(), None),
            l1_anchor_epoch: metrics.create_gauge("bootstrap_l1_anchor_epoch".into(), None),
        }
    }
}

/// The anchor at the L1 finalized block, or `None` when unavailable. A failed read must not make
/// startup worse than the walk alone.
pub async fn fetch_l1_anchor(node_state: &NodeState) -> Option<L1Anchor> {
    match node_state.finalized_light_client_state().await {
        Ok(state) => state.map(L1Anchor::from),
        Err(err) => {
            tracing::warn!(
                err = %format!("{err:#}"),
                "bootstrap: cannot read the L1 light client state; walking"
            );
            None
        },
    }
}

/// Populate the membership with stake tables for every epoch up through `N+1` (where `N` is the
/// current epoch). Returns `N`.
///
/// Preconditions: `reload_stake` should have run before this: it populates the membership from
/// local persistence so the walk skips epochs we already know.
pub async fn bootstrap_epoch_window<C: Client>(
    coordinator: &EpochMembershipCoordinator<SeqTypes>,
    epoch_height: u64,
    anchor: Option<L1Anchor>,
    peers: &[C],
    params: &BootstrapParams,
    metrics: &BootstrapMetrics,
) -> anyhow::Result<EpochNumber> {
    if epoch_height == 0 {
        // Pre-epoch chain: epochs aren't enabled yet, the non-epoch
        // committee path is what gets used.
        return Ok(EpochNumber::genesis());
    }

    let started = Instant::now();
    let membership = coordinator.membership();
    let first_epoch = membership
        .first_epoch()
        .context("first_epoch not seeded; genesis stake table missing")?;
    let known = membership.highest_known_epoch();
    if let Some(anchor) = &anchor {
        let epoch = epoch_from_block_number(anchor.height, epoch_height);
        metrics.l1_anchor_epoch.set(epoch as usize);
    }

    let skipped = match plan_skip(
        params.skip,
        anchor,
        peers.len(),
        epoch_height,
        first_epoch,
        known,
    ) {
        SkipPlan::Run(anchor, range) => {
            tracing::info!(
                target: TARGET,
                finalized_height = anchor.height,
                epochs = ?(range.start().u64()..=range.end().u64()),
                highest_known = ?known.map(|e| e.u64()),
                "skipping to L1 anchor",
            );
            skip_with_retry(
                coordinator,
                peers,
                anchor,
                &range,
                epoch_height,
                params,
                known,
            )
            .await?;
            true
        },
        SkipPlan::Walk(reason) => {
            tracing::info!(target: TARGET, reason, "skip disabled or not applicable; walking");
            false
        },
    };

    let current = walk_to_tip(coordinator, first_epoch, params.step_timeout).await?;
    metrics.window_epoch.set(current.u64() as usize);
    tracing::info!(
        target: TARGET,
        current_epoch = %current,
        duration_ms = started.elapsed().as_millis() as u64,
        skipped,
        "window installed",
    );
    Ok(current)
}

/// The epochs whose stake tables the skip installs: `n-3 ..= n+1` for `n = epoch(F)`.
///
/// `block_reward(n)` and `block_reward(n+1)` need the stake table of three epochs earlier, and the
/// rest is the window. `None` when `n-3` is within the epochs seeded from genesis, where the walk
/// is short anyway.
pub fn install_range(
    anchor_height: u64,
    epoch_height: u64,
    first_epoch: EpochNumber,
) -> Option<RangeInclusive<EpochNumber>> {
    if epoch_height == 0 {
        return None;
    }
    let n = epoch_from_block_number(anchor_height, epoch_height);
    let start = n.checked_sub(3).filter(|start| *start > *first_epoch + 1)?;
    Some(EpochNumber::new(start)..=EpochNumber::new(n + 1))
}

/// Whether the known stake tables fall short of the install range.
pub fn should_skip(
    highest_known: Option<EpochNumber>,
    range: &RangeInclusive<EpochNumber>,
    mode: SkipMode,
) -> bool {
    mode == SkipMode::Auto && highest_known.is_none_or(|highest| highest < *range.start())
}

/// Accept `header` only if it is the block the contract finalized.
///
/// The view number does not enter `block_comm_root`, so any view serves.
pub fn verify_anchor_header(header: &Header, anchor: &L1Anchor) -> anyhow::Result<()> {
    ensure!(
        header.height() == anchor.height,
        "anchor header has height {}, expected {}",
        header.height(),
        anchor.height
    );
    let state = header.get_light_client_state(ViewNumber::genesis())?;
    ensure!(
        state.block_comm_root == anchor.block_comm_root,
        "anchor header at height {} does not match the L1 block commitment root",
        anchor.height
    );
    Ok(())
}

/// Fetch the epoch roots of `range` from one `peer` and install them in ascending order.
///
/// Every header is verified against header `F` of the anchor, which is itself verified against
/// the contract's commitment, so a lying peer fails here and installs nothing it cannot prove.
pub async fn skip_to_l1_anchor<C: Client>(
    coordinator: &EpochMembershipCoordinator<SeqTypes>,
    peer: &C,
    anchor: &L1Anchor,
    range: &RangeInclusive<EpochNumber>,
    epoch_height: u64,
) -> anyhow::Result<()> {
    let header = peer
        .header(anchor.height)
        .await
        .context("fetching the anchor header")?;
    verify_anchor_header(&header, anchor)?;
    let root = header.block_merkle_tree_root();

    for epoch in range.start().u64()..=range.end().u64() {
        // The root of epoch `e - 2` derives the stake table of epoch `e`.
        let height = root_block_in_epoch(epoch - 2, epoch_height);
        let proof = peer
            .header_proof(anchor.height, BlockId::Number(height as usize))
            .await
            .with_context(|| format!("fetching the epoch root at height {height}"))?;
        let header = proof
            .verify(root)
            .with_context(|| format!("verifying the epoch root at height {height}"))?;
        ensure!(
            header.height() == height,
            "peer returned height {} for the epoch root at {height}",
            header.height()
        );
        coordinator
            .add_epoch_root(header)
            .await
            .map_err(|err| anyhow::anyhow!("installing the epoch root at {height}: {err}"))?;
    }
    Ok(())
}

enum SkipPlan {
    Run(L1Anchor, RangeInclusive<EpochNumber>),
    Walk(&'static str),
}

fn plan_skip(
    mode: SkipMode,
    anchor: Option<L1Anchor>,
    peer_count: usize,
    epoch_height: u64,
    first_epoch: EpochNumber,
    known: Option<EpochNumber>,
) -> SkipPlan {
    if mode == SkipMode::Off {
        return SkipPlan::Walk("off");
    }
    let Some(anchor) = anchor else {
        return SkipPlan::Walk("no light client anchor");
    };
    if peer_count == 0 {
        return SkipPlan::Walk("no state peers");
    }
    let Some(range) = install_range(anchor.height, epoch_height, first_epoch) else {
        return SkipPlan::Walk("anchor too early for epochs");
    };
    if !should_skip(known, &range, mode) {
        return SkipPlan::Walk("near tip");
    }
    SkipPlan::Run(anchor, range)
}

/// Try the peers in rotation, backing off after each full pass, until the deadline.
///
/// At the deadline, a node still far from the anchor gives up; one near it falls back to the walk.
async fn skip_with_retry<C: Client>(
    coordinator: &EpochMembershipCoordinator<SeqTypes>,
    peers: &[C],
    anchor: L1Anchor,
    range: &RangeInclusive<EpochNumber>,
    epoch_height: u64,
    params: &BootstrapParams,
    known: Option<EpochNumber>,
) -> anyhow::Result<()> {
    let deadline = Instant::now() + params.deadline;
    let mut backoff = INITIAL_BACKOFF;
    let mut attempt = 0usize;
    loop {
        let peer = attempt % peers.len();
        let remaining = deadline.saturating_duration_since(Instant::now());
        let result = timeout(
            remaining,
            skip_to_l1_anchor(coordinator, &peers[peer], &anchor, range, epoch_height),
        )
        .await;
        let err = match result {
            Ok(Ok(())) => return Ok(()),
            Ok(Err(err)) => format!("{err:#}"),
            Err(_) => "attempt timed out".into(),
        };
        attempt += 1;
        if attempt.is_power_of_two() {
            tracing::warn!(
                attempt,
                peer,
                err,
                "bootstrap: skip attempt failed; retrying"
            );
        }

        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return deadline_exceeded(range, known, params.deadline);
        }
        if attempt.is_multiple_of(peers.len()) {
            sleep(backoff.min(remaining)).await;
            backoff = (backoff * 2).min(MAX_BACKOFF);
        }
    }
}

fn deadline_exceeded(
    range: &RangeInclusive<EpochNumber>,
    known: Option<EpochNumber>,
    deadline: Duration,
) -> anyhow::Result<()> {
    let n = range.end().u64() - 1;
    let gap = n.saturating_sub(known.map_or(0, |known| known.u64()));
    if gap > MAX_WALKABLE_GAP {
        tracing::error!(?deadline, gap, "bootstrap: deadline exceeded");
        bail!("bootstrap skip did not finish within {deadline:?}; {gap} epochs behind the anchor");
    }
    tracing::warn!(
        ?deadline,
        gap,
        "bootstrap: deadline exceeded near the tip; walking"
    );
    Ok(())
}

/// Walk forward from the highest known epoch until peers can no longer serve the next epoch root
/// leaf. Each step drives `add_epoch_root` through the existing catchup machinery, persisting the
/// new stake table.
async fn walk_to_tip(
    coordinator: &EpochMembershipCoordinator<SeqTypes>,
    first_epoch: EpochNumber,
    step_timeout: Duration,
) -> anyhow::Result<EpochNumber> {
    let membership = coordinator.membership();
    // Catchup resumes from the latest consecutive pair of stake tables we
    // hold, so a gap near the tip is bridged by the step that runs into it
    // rather than having to be found here.
    let mut highest = membership.highest_known_epoch().unwrap_or(first_epoch + 1);

    tracing::info!(
        %first_epoch,
        starting_from = %highest,
        "bootstrap_epoch_window: walking forward",
    );

    loop {
        let target = highest + 1;
        match timeout(step_timeout, coordinator.wait_for_stake_table(target)).await {
            Ok(Ok(_)) => {
                tracing::info!(%target, "bootstrap_epoch_window: derived stake table");
                highest = target;
            },
            Ok(Err(err)) => {
                tracing::info!(
                    target: TARGET,
                    %target,
                    %err,
                    "bootstrap_epoch_window: catchup failed; treating as live tip",
                );
                break;
            },
            Err(_) => {
                tracing::warn!(
                    %target,
                    timeout_secs = step_timeout.as_secs(),
                    "bootstrap_epoch_window: catchup timed out; proceeding with a possibly \
                     stale epoch window",
                );
                break;
            },
        }
    }

    // `highest` corresponds to N+1 (the leaf at root_block_in_epoch(N-1) is
    // the last finalized one peers can serve). So current epoch N = highest - 1.
    let current = EpochNumber::new(highest.saturating_sub(1));

    ensure!(
        membership.snapshot(current).is_some(),
        "missing stake table for current epoch {current} after bootstrap"
    );
    ensure!(
        membership.snapshot(highest).is_some(),
        "missing stake table for next epoch {highest} after bootstrap"
    );
    Ok(current)
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use alloy::primitives::Address;
    use async_lock::Mutex as AsyncMutex;
    use async_trait::async_trait;
    use espresso_types::{
        AuthenticatedValidatorMap, ChainConfig, EpochCommittees, L1Client, StakeTableHash,
        mock::MockStateCatchup,
        traits::{EventsPersistenceRead, MembershipPersistence, StakeTuple},
        v0_3::{
            EventKey, Fetcher, IndexedStake, RegisteredValidator, RewardAmount, StakeTableEvent,
        },
    };
    use hotshot_example_types::storage_types::TestStorage;
    use hotshot_types::{
        PeerConfig, ValidatorConfig, drb::DrbResult, signature_key::BLSPubKey,
        traits::metrics::NoMetrics,
    };
    use indexmap::IndexMap;
    use light_client::testing::{TestClient, random_validator};
    use rstest::rstest;

    use super::*;

    const EPOCH_HEIGHT: u64 = 10;
    const FIRST_EPOCH: u64 = 1;
    /// Epoch of the anchor: the install range is `N-3 ..= N+1`.
    const N: u64 = 8;
    /// In epoch `N`, past its root block (75).
    const ANCHOR_HEIGHT: u64 = 77;

    const PARAMS: BootstrapParams = BootstrapParams {
        step_timeout: Duration::from_secs(5),
        deadline: Duration::from_secs(30),
        skip: SkipMode::Auto,
    };

    fn epochs(range: RangeInclusive<EpochNumber>) -> RangeInclusive<u64> {
        range.start().u64()..=range.end().u64()
    }

    #[rstest]
    #[case::decaf_anchor_at_root(3000, 3000 * 1000 - 5, Some(997..=1001))]
    #[case::decaf_after_root(3000, 3000 * 1000 - 4, Some(997..=1001))]
    #[case::decaf_epoch_end(3000, 3000 * 1000, Some(997..=1001))]
    #[case::decaf_next_epoch(3000, 3000 * 1000 + 1, Some(998..=1002))]
    #[case::mainnet_anchor_at_root(40000, 40000 * 100 - 5, Some(97..=101))]
    #[case::mainnet_next_epoch(40000, 40000 * 100 + 1, Some(98..=102))]
    #[case::start_is_seeded(100, 100 * 4, None)]
    #[case::start_after_seeded(100, 100 * 5, Some(2..=6))]
    #[case::pre_pos(100, 50, None)]
    #[case::genesis(100, 0, None)]
    #[case::no_epochs(0, 1000, None)]
    fn install_range_cases(
        #[case] epoch_height: u64,
        #[case] anchor_height: u64,
        #[case] expected: Option<RangeInclusive<u64>>,
    ) {
        // `first_epoch = 0` seeds epochs 0 and 1.
        let range = install_range(anchor_height, epoch_height, EpochNumber::new(0)).map(epochs);
        assert_eq!(range, expected);
    }

    /// `block_reward(E)` needs the stake table of `E - 3`, and `E` ranges over `n` and `n + 1`.
    #[test]
    fn install_range_covers_reward_inputs() {
        let range = install_range(ANCHOR_HEIGHT, EPOCH_HEIGHT, EpochNumber::new(FIRST_EPOCH));
        let range = epochs(range.unwrap());
        assert!(range.contains(&(N - 3)));
        assert_eq!(*range.end(), N + 1);
    }

    #[test]
    fn install_range_roots_precede_the_anchor() {
        let range = install_range(ANCHOR_HEIGHT, EPOCH_HEIGHT, EpochNumber::new(FIRST_EPOCH));
        for epoch in epochs(range.unwrap()) {
            assert!(root_block_in_epoch(epoch - 2, EPOCH_HEIGHT) < ANCHOR_HEIGHT);
        }
    }

    #[rstest]
    #[case::empty(None, SkipMode::Auto, true)]
    #[case::below(Some(4), SkipMode::Auto, true)]
    #[case::at_start(Some(5), SkipMode::Auto, false)]
    #[case::inside(Some(7), SkipMode::Auto, false)]
    #[case::above(Some(12), SkipMode::Auto, false)]
    #[case::off(None, SkipMode::Off, false)]
    fn should_skip_cases(
        #[case] highest: Option<u64>,
        #[case] mode: SkipMode,
        #[case] expected: bool,
    ) {
        let range = EpochNumber::new(5)..=EpochNumber::new(9);
        assert_eq!(
            should_skip(highest.map(EpochNumber::new), &range, mode),
            expected
        );
    }

    #[test]
    fn deadline_policy() {
        let range = EpochNumber::new(5)..=EpochNumber::new(9);
        let at =
            |known: u64| deadline_exceeded(&range, Some(EpochNumber::new(known)), PARAMS.deadline);
        assert!(at(4).is_err());
        assert!(at(5).is_err());
        assert!(at(6).is_ok(), "gap of 2 walks");
        assert!(at(8).is_ok());
        assert!(deadline_exceeded(&range, None, PARAMS.deadline).is_err());
    }

    /// Test chain of `ANCHOR_HEIGHT + 1` blocks and the anchor over it.
    async fn chain() -> (TestClient, L1Anchor) {
        let client = TestClient::with_epoch_height(EPOCH_HEIGHT);
        let header = client.header(ANCHOR_HEIGHT).await.unwrap();
        let block_comm_root = header
            .get_light_client_state(ViewNumber::genesis())
            .unwrap()
            .block_comm_root;
        let anchor = L1Anchor {
            height: ANCHOR_HEIGHT,
            block_comm_root,
        };
        (client, anchor)
    }

    #[tokio::test]
    async fn anchor_header_verification() {
        let (client, anchor) = chain().await;
        let header = client.header(ANCHOR_HEIGHT).await.unwrap();
        verify_anchor_header(&header, &anchor).unwrap();

        let wrong_root = L1Anchor {
            block_comm_root: anchor.block_comm_root + CircuitField::from(1u64),
            ..anchor
        };
        let err = verify_anchor_header(&header, &wrong_root).unwrap_err();
        assert!(format!("{err:#}").contains("commitment root"), "{err:#}");

        let wrong_height = L1Anchor {
            height: ANCHOR_HEIGHT + 1,
            ..anchor
        };
        let err = verify_anchor_header(&header, &wrong_height).unwrap_err();
        assert!(format!("{err:#}").contains("height"), "{err:#}");
    }

    /// Stands in for the node's database: it holds a stake table for each epoch the test lets it
    /// answer for, with the hash a verified epoch root header commits to.
    #[derive(Debug, Default)]
    struct SeededStore(HashMap<u64, StakeTuple>);

    impl SeededStore {
        async fn for_chain(client: &TestClient, epochs: RangeInclusive<u64>) -> Self {
            let validators: AuthenticatedValidatorMap = (0..4)
                .map(|_| random_validator())
                .map(|validator| (validator.account, validator))
                .collect();
            let mut tables = HashMap::new();
            for epoch in epochs {
                // The root of epoch `e - 1` commits to the stake table of epoch `e`.
                let root = root_block_in_epoch(epoch - 1, EPOCH_HEIGHT);
                let hash = client.header(root).await.unwrap().next_stake_table_hash();
                tables.insert(
                    epoch,
                    (validators.clone(), Some(RewardAmount::default()), hash),
                );
            }
            Self(tables)
        }
    }

    #[async_trait]
    impl MembershipPersistence for SeededStore {
        async fn load_stake(&self, epoch: EpochNumber) -> anyhow::Result<Option<StakeTuple>> {
            Ok(self.0.get(&epoch.u64()).cloned())
        }

        async fn load_latest_stake(
            &self,
            _limit: u64,
        ) -> anyhow::Result<Option<Vec<IndexedStake>>> {
            Ok(None)
        }

        async fn load_drb_result(&self, _epoch: EpochNumber) -> anyhow::Result<Option<DrbResult>> {
            Ok(None)
        }

        async fn load_epoch_root(&self, _epoch: EpochNumber) -> anyhow::Result<Option<Header>> {
            Ok(None)
        }

        async fn store_epoch_root(&self, _: EpochNumber, _: Header) -> anyhow::Result<()> {
            Ok(())
        }

        async fn store_stake(
            &self,
            _: EpochNumber,
            _: AuthenticatedValidatorMap,
            _: Option<RewardAmount>,
            _: Option<StakeTableHash>,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn store_events(
            &self,
            _: u64,
            _: Vec<(EventKey, StakeTableEvent)>,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn load_events(
            &self,
            _: u64,
            _: u64,
        ) -> anyhow::Result<(
            Option<EventsPersistenceRead>,
            Vec<(EventKey, StakeTableEvent)>,
        )> {
            Ok((None, vec![]))
        }

        async fn delete_stake_tables(&self) -> anyhow::Result<()> {
            Ok(())
        }

        async fn store_all_validators(
            &self,
            _: EpochNumber,
            _: IndexMap<Address, RegisteredValidator<BLSPubKey>>,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn load_all_validators(
            &self,
            _: EpochNumber,
            _: u64,
            _: u64,
        ) -> anyhow::Result<Vec<RegisteredValidator<BLSPubKey>>> {
            Ok(vec![])
        }
    }

    /// A real `EpochCommittees` and coordinator over `store`, seeded like a fresh node. No peer
    /// serves epoch roots, so the walk ends at the first epoch above the installed ones.
    fn coordinator(store: SeededStore) -> EpochMembershipCoordinator<SeqTypes> {
        let fetcher = Fetcher::new(
            Arc::new(MockStateCatchup::default()),
            Arc::new(AsyncMutex::new(store)),
            L1Client::new(vec!["http://localhost:3331".parse().unwrap()]).unwrap(),
            ChainConfig::default(),
        );
        let peers: Vec<PeerConfig<SeqTypes>> = (0..4)
            .map(|i| {
                ValidatorConfig::<SeqTypes>::generated_from_seed_indexed(
                    [42u8; 32],
                    i,
                    alloy::primitives::U256::from(100),
                    true,
                )
                .public_config()
            })
            .collect();
        let committees =
            EpochCommittees::new_stake(peers.clone(), peers, None, fetcher, EPOCH_HEIGHT);
        committees.set_first_epoch(EpochNumber::new(FIRST_EPOCH), [0u8; 32]);
        EpochMembershipCoordinator::new(
            committees,
            EPOCH_HEIGHT,
            &TestStorage::<SeqTypes>::default(),
        )
    }

    async fn fresh_coordinator(client: &TestClient) -> EpochMembershipCoordinator<SeqTypes> {
        coordinator(SeededStore::for_chain(client, (N - 3)..=(N + 1)).await)
    }

    async fn bootstrap(
        coordinator: &EpochMembershipCoordinator<SeqTypes>,
        anchor: Option<L1Anchor>,
        peers: &[TestClient],
        params: BootstrapParams,
    ) -> anyhow::Result<EpochNumber> {
        let metrics = BootstrapMetrics::new(&NoMetrics);
        bootstrap_epoch_window(coordinator, EPOCH_HEIGHT, anchor, peers, &params, &metrics).await
    }

    fn has_table(coordinator: &EpochMembershipCoordinator<SeqTypes>, epoch: u64) -> bool {
        coordinator
            .membership()
            .snapshot(EpochNumber::new(epoch))
            .is_some()
    }

    #[test_log::test(tokio::test)]
    async fn skip_installs_window() {
        let (client, anchor) = chain().await;
        let coordinator = fresh_coordinator(&client).await;

        let current = bootstrap(&coordinator, Some(anchor), &[client], PARAMS)
            .await
            .unwrap();

        // The residual walk finds no root above the anchor, as when the prover is not lagging.
        assert_eq!(current, EpochNumber::new(N));
        for epoch in (N - 3)..=(N + 1) {
            assert!(has_table(&coordinator, epoch), "epoch {epoch}");
        }
        assert_eq!(
            coordinator.membership().highest_known_epoch(),
            Some(EpochNumber::new(N + 1))
        );
    }

    #[test_log::test(tokio::test)]
    async fn peer_with_wrong_chain_is_skipped() {
        let (client, anchor) = chain().await;
        let (other, _) = chain().await;
        let coordinator = fresh_coordinator(&client).await;

        let current = bootstrap(&coordinator, Some(anchor), &[other, client], PARAMS)
            .await
            .unwrap();

        assert_eq!(current, EpochNumber::new(N));
        assert!(has_table(&coordinator, N + 1));
    }

    #[test_log::test(tokio::test)]
    async fn invalid_proof_installs_nothing() {
        let (client, anchor) = chain().await;
        let coordinator = fresh_coordinator(&client).await;
        let range = install_range(anchor.height, EPOCH_HEIGHT, EpochNumber::new(FIRST_EPOCH));
        let range = range.unwrap();
        let bad_root = root_block_in_epoch(range.end().u64() - 2, EPOCH_HEIGHT);
        client.return_invalid_proof(bad_root as usize).await;

        let err = skip_to_l1_anchor(&coordinator, &client, &anchor, &range, EPOCH_HEIGHT)
            .await
            .unwrap_err();

        assert!(
            format!("{err:#}").contains("verifying the epoch root"),
            "{err:#}"
        );
        assert!(!has_table(&coordinator, N + 1));
    }

    #[test_log::test(tokio::test)]
    async fn wrong_anchor_root_installs_nothing() {
        let (client, anchor) = chain().await;
        let coordinator = fresh_coordinator(&client).await;
        let range = install_range(anchor.height, EPOCH_HEIGHT, EpochNumber::new(FIRST_EPOCH));
        let anchor = L1Anchor {
            block_comm_root: anchor.block_comm_root + CircuitField::from(1u64),
            ..anchor
        };

        let err = skip_to_l1_anchor(
            &coordinator,
            &client,
            &anchor,
            &range.unwrap(),
            EPOCH_HEIGHT,
        )
        .await
        .unwrap_err();

        assert!(format!("{err:#}").contains("commitment root"), "{err:#}");
        assert!(!has_table(&coordinator, N - 3));
    }

    /// An epoch root the peer cannot serve at first must be retried, not read as the tip.
    #[test_log::test(tokio::test)]
    async fn transient_failure_is_retried() {
        let (client, anchor) = chain().await;
        let coordinator = fresh_coordinator(&client).await;
        let flaky_root = root_block_in_epoch(N - 1, EPOCH_HEIGHT) as usize;
        client.forget_leaf(flaky_root).await;
        let heal = client.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            heal.remember_leaf(flaky_root).await;
        });

        let current = bootstrap(&coordinator, Some(anchor), &[client], PARAMS)
            .await
            .unwrap();

        assert_eq!(current, EpochNumber::new(N));
        assert!(has_table(&coordinator, N + 1));
    }

    #[test_log::test(tokio::test)]
    async fn deadline_aborts_when_far_from_the_anchor() {
        let (client, anchor) = chain().await;
        let coordinator = fresh_coordinator(&client).await;
        client.forget_leaf(ANCHOR_HEIGHT as usize).await;

        let params = BootstrapParams {
            deadline: Duration::from_millis(300),
            ..PARAMS
        };
        let err = bootstrap(&coordinator, Some(anchor), &[client], params)
            .await
            .unwrap_err();

        assert!(format!("{err:#}").contains("did not finish"), "{err:#}");
    }

    /// Storage already inside the install range walks as before, without contacting any peer.
    #[test_log::test(tokio::test)]
    async fn restart_inside_range_walks() {
        let (client, anchor) = chain().await;
        let coordinator = coordinator(SeededStore::for_chain(&client, (N - 4)..=(N + 1)).await);
        for epoch in [N - 4, N - 3] {
            assert!(
                coordinator
                    .membership()
                    .load_stake_table(EpochNumber::new(epoch))
                    .await
            );
        }
        client.forget_leaf(ANCHOR_HEIGHT as usize).await;

        let current = bootstrap(&coordinator, Some(anchor), &[client], PARAMS)
            .await
            .unwrap();

        assert_eq!(current, EpochNumber::new(N - 4));
        assert!(!has_table(&coordinator, N + 1));
    }

    #[rstest]
    #[case::skip_off(SkipMode::Off, true)]
    #[case::no_anchor(SkipMode::Auto, false)]
    #[test_log::test(tokio::test)]
    async fn walks_without_skip(#[case] skip: SkipMode, #[case] with_anchor: bool) {
        let (client, anchor) = chain().await;
        let coordinator = fresh_coordinator(&client).await;
        // Any contact with the peer would install tables.
        let params = BootstrapParams { skip, ..PARAMS };

        let current = bootstrap(
            &coordinator,
            with_anchor.then_some(anchor),
            &[client],
            params,
        )
        .await
        .unwrap();

        assert_eq!(current, EpochNumber::new(FIRST_EPOCH));
        assert!(!has_table(&coordinator, N - 3));
    }

    #[test_log::test(tokio::test)]
    async fn pre_pos_anchor_walks() {
        let (client, _) = chain().await;
        let coordinator = fresh_coordinator(&client).await;
        let early = client.header(EPOCH_HEIGHT).await.unwrap();
        let anchor = L1Anchor {
            height: early.height(),
            block_comm_root: early
                .get_light_client_state(ViewNumber::genesis())
                .unwrap()
                .block_comm_root,
        };

        let current = bootstrap(&coordinator, Some(anchor), &[client], PARAMS)
            .await
            .unwrap();

        assert_eq!(current, EpochNumber::new(FIRST_EPOCH));
        assert!(!has_table(&coordinator, N - 3));
    }
}
