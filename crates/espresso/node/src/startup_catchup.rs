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
//! 1. Install: the L1 light-client contract vouches for a finalized header `F`. The epoch roots below
//!    `F` are fetched with Merkle proofs against `F` and installed directly. Failures to read the
//!    anchor or fetch the roots are retried until the deadline. A non-empty install range without
//!    state peers fails at once.
//! 2. Walk: from the highest known epoch, one epoch at a time. The walk targets the epoch the peers'
//!    chain tip needs, and a failure below it is retried until the deadline. Without a target, or
//!    past it, the first failure marks the live tip.

use std::{ops::RangeInclusive, time::Duration};

use anyhow::{Context, anyhow, ensure};
use espresso_types::{Header, SeqTypes};
use futures::future::{join_all, try_join_all};
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
use light_client::client::{Client, QueryServiceClient};
use tokio::time::{Instant, error::Elapsed, sleep, timeout, timeout_at};
use url::Url;

const TARGET: &str = "announce::bootstrap";

const INITIAL_BACKOFF: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(60);
/// Epochs past the anchor's window that the walk target may reach.
const TARGET_SLACK: u64 = 2;

/// A once-per-boot milestone: INFO on the announce target, or debug when the call is not the boot
/// one.
macro_rules! milestone {
    ($params:expr, $($arg:tt)+) => {
        if $params.boot {
            tracing::info!(target: TARGET, $($arg)+)
        } else {
            tracing::debug!($($arg)+)
        }
    };
}

#[derive(Clone, Copy, Debug)]
pub struct BootstrapParams {
    /// Bound on a single `wait_for_stake_table` call during the walk.
    pub step_timeout: Duration,
    /// How long the install, and separately the walk toward the peers' chain tip, may keep
    /// retrying before startup gives up.
    pub deadline: Duration,
    /// Set for the call at boot only. Mid-run, milestones log at debug and the gauges stay.
    pub boot: bool,
}

/// A finalized HotShot block that the node trusts without asking the peers: the L1 light-client
/// contract's finalized state.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TrustedAnchor {
    pub height: u64,
    pub block_comm_root: CircuitField,
}

impl TrustedAnchor {
    /// No finalized state: the install range is empty.
    pub(crate) fn genesis() -> Self {
        Self {
            height: 0,
            block_comm_root: CircuitField::from(0u64),
        }
    }
}

impl From<LightClientStateSol> for TrustedAnchor {
    fn from(state: LightClientStateSol) -> Self {
        Self {
            height: state.blockHeight,
            block_comm_root: u256_to_field(state.blockCommRoot),
        }
    }
}

pub(crate) struct BootstrapMetrics {
    window_epoch: Box<dyn Gauge>,
    anchor_epoch: Box<dyn Gauge>,
}

impl BootstrapMetrics {
    pub fn new(metrics: &dyn Metrics) -> Self {
        Self {
            window_epoch: metrics.create_gauge("bootstrap_window_epoch".into(), None),
            anchor_epoch: metrics.create_gauge("bootstrap_anchor_epoch".into(), None),
        }
    }
}

/// Populate the membership with stake tables for every epoch up through `N+1` (where `N` is the
/// current epoch). Returns `N`.
///
/// `fetch_anchor` failures are retried until the deadline, like those of the install.
///
/// Preconditions: `reload_stake` should have run before this: it populates the membership from
/// local persistence so the walk skips epochs we already know.
pub(crate) async fn bootstrap_epoch_window<C: Client, Fut>(
    coordinator: &EpochMembershipCoordinator<SeqTypes>,
    epoch_height: u64,
    fetch_anchor: impl Fn() -> Fut,
    peers: &[(Url, C)],
    params: &BootstrapParams,
    metrics: &BootstrapMetrics,
) -> anyhow::Result<EpochNumber>
where
    Fut: Future<Output = anyhow::Result<TrustedAnchor>>,
{
    if epoch_height == 0 {
        // Pre-epoch chain: epochs aren't enabled yet, the non-epoch
        // committee path is what gets used.
        return Ok(EpochNumber::genesis());
    }

    let started = Instant::now();
    let first_epoch = coordinator
        .membership()
        .first_epoch()
        .context("first_epoch not seeded; genesis stake table missing")?;

    let deadline = Instant::now() + params.deadline;
    let anchor = retry(deadline, "reading the anchor", 1, |_, remaining| {
        let read = timeout(params.step_timeout.min(remaining), fetch_anchor());
        async move { read.await.context("attempt timed out")? }
    })
    .await?;
    milestone!(params, finalized_height = anchor.height, "anchor read");
    if params.boot {
        let anchor_epoch = epoch_from_block_number(anchor.height, epoch_height);
        metrics.anchor_epoch.set(anchor_epoch as usize);
    }

    let range = install_range(anchor.height, epoch_height, first_epoch);
    if let Some(range) = &range {
        ensure!(
            !peers.is_empty(),
            "finalized height {} needs a state peer serving the light-client module",
            anchor.height
        );
        install_window(
            coordinator,
            peers,
            &anchor,
            range,
            epoch_height,
            deadline,
            params,
        )
        .await?;
    }

    // An anchor with an empty range vouches for nothing, so it does not cap the target.
    let anchor = range.is_some().then_some(&anchor);
    let target = walk_target(peers, anchor, epoch_height, params.step_timeout).await;
    let current = walk_to_tip(coordinator, first_epoch, target, params).await?;
    if params.boot {
        metrics.window_epoch.set(current.u64() as usize);
    }
    milestone!(
        params,
        current_epoch = %current,
        duration_ms = started.elapsed().as_millis() as u64,
        "window installed",
    );
    Ok(current)
}

pub(crate) fn query_clients(urls: &[Url]) -> Vec<(Url, QueryServiceClient)> {
    urls.iter()
        .map(|url| (url.clone(), QueryServiceClient::new(url.clone())))
        .collect()
}

/// Install the `range` of epochs the `anchor` determines. Epochs already stored are a no-op.
async fn install_window<C: Client>(
    coordinator: &EpochMembershipCoordinator<SeqTypes>,
    peers: &[(Url, C)],
    anchor: &TrustedAnchor,
    range: &RangeInclusive<EpochNumber>,
    epoch_height: u64,
    deadline: Instant,
    params: &BootstrapParams,
) -> anyhow::Result<()> {
    milestone!(
        params,
        finalized_height = anchor.height,
        epochs = ?(range.start().u64()..=range.end().u64()),
        highest_known = ?coordinator.membership().highest_known_epoch().map(|e| e.u64()),
        "installing the anchor window",
    );

    let fetch = fetch_epoch_roots(peers, anchor, range, epoch_height, deadline, params);
    let headers = fetch.await?;
    // The install can fetch a chain config or L1 events without a bound of its own. Cancelling it
    // leaves at most a lone stake table, which the next boot installs again.
    let install = async {
        for header in headers {
            let height = header.height();
            coordinator
                .add_epoch_root(header)
                .await
                .map_err(|err| anyhow!("installing the epoch root at {height}: {err}"))?;
        }
        anyhow::Ok(())
    };
    timeout_at(deadline, install)
        .await
        .context("bootstrap deadline exceeded while installing the epoch roots")?
}

/// The epochs whose stake tables the install covers: `n-3 ..= n+1` for `n = epoch(F)`, clamped to
/// start after the two epochs seeded from genesis.
///
/// `block_reward(n)` and `block_reward(n+1)` need the stake table of three epochs earlier, and the
/// rest is the window. `None` when `F` is before the first epoch's end, so there is no root to
/// prove. Every root in the range is below `F`: the root of `e-2` lies in epoch `e-2 < n`.
fn install_range(
    anchor_height: u64,
    epoch_height: u64,
    first_epoch: EpochNumber,
) -> Option<RangeInclusive<EpochNumber>> {
    let n = epoch_from_block_number(anchor_height, epoch_height);
    let start = n.saturating_sub(3).max(*first_epoch + 2);
    (start <= n + 1).then(|| EpochNumber::new(start)..=EpochNumber::new(n + 1))
}

/// Fetch and verify the epoch roots of `range` from the peers in rotation, backing off after each
/// full pass, until `deadline`. One attempt is bounded by `step_timeout`, so a slow peer does not
/// starve the others.
async fn fetch_epoch_roots<C: Client>(
    peers: &[(Url, C)],
    anchor: &TrustedAnchor,
    range: &RangeInclusive<EpochNumber>,
    epoch_height: u64,
    deadline: Instant,
    params: &BootstrapParams,
) -> anyhow::Result<Vec<Header>> {
    debug_assert!(!peers.is_empty());
    retry(
        deadline,
        "installing the anchor window",
        peers.len(),
        |attempt, remaining| async move {
            let (url, peer) = &peers[attempt % peers.len()];
            let fetch = fetch_roots_from(peer, anchor, range, epoch_height);
            let fetch = timeout(params.step_timeout.min(remaining), fetch);
            fetch
                .await
                .context("attempt timed out")?
                .with_context(|| format!("peer {url}"))
        },
    )
    .await
}

/// Run `attempt` with its index and the time left until it succeeds, backing off after every
/// `per_pass` failures. Errors at `deadline`.
async fn retry<T, Fut: Future<Output = anyhow::Result<T>>>(
    deadline: Instant,
    what: &str,
    per_pass: usize,
    mut attempt: impl FnMut(usize, Duration) -> Fut,
) -> anyhow::Result<T> {
    let mut backoff = INITIAL_BACKOFF;
    let mut count = 0usize;
    let mut last_err = String::from("none");
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        ensure!(
            !remaining.is_zero(),
            "bootstrap deadline exceeded after {count} attempts while {what}, last error: \
             {last_err}"
        );

        match attempt(count, remaining).await {
            Ok(value) => return Ok(value),
            Err(err) => last_err = format!("{err:#}"),
        }
        count += 1;
        if count.is_power_of_two() {
            tracing::warn!(
                attempt = count,
                err = last_err,
                "bootstrap: {what} failed; retrying"
            );
        }

        if count.is_multiple_of(per_pass) {
            let remaining = deadline.saturating_duration_since(Instant::now());
            sleep(backoff.min(remaining)).await;
            backoff = (backoff * 2).min(MAX_BACKOFF);
        }
    }
}

/// Fetch the epoch roots of `range` from one `peer`, in ascending order. The proofs are fetched
/// concurrently, once the anchor header is verified.
///
/// Every header is verified against header `F` of the anchor, which is itself verified against
/// the anchor's commitment, so a lying peer fails here and nothing it cannot prove gets installed.
async fn fetch_roots_from<C: Client>(
    peer: &C,
    anchor: &TrustedAnchor,
    range: &RangeInclusive<EpochNumber>,
    epoch_height: u64,
) -> anyhow::Result<Vec<Header>> {
    let header = peer
        .header(anchor.height)
        .await
        .context("fetching the anchor header")?;
    verify_anchor_header(&header, anchor)?;
    let root = header.block_merkle_tree_root();

    try_join_all(
        (range.start().u64()..=range.end().u64()).map(|epoch| async move {
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
            Ok(header)
        }),
    )
    .await
}

/// Accept `header` only if it is the block the anchor commits to.
///
/// The view number does not enter `block_comm_root`, so any view serves.
fn verify_anchor_header(header: &Header, anchor: &TrustedAnchor) -> anyhow::Result<()> {
    ensure!(
        header.height() == anchor.height,
        "anchor header has height {}, expected {}",
        header.height(),
        anchor.height
    );
    let state = header.get_light_client_state(ViewNumber::genesis())?;
    ensure!(
        state.block_comm_root == anchor.block_comm_root,
        "anchor header at height {} does not match the block commitment root",
        anchor.height
    );
    Ok(())
}

/// The epoch whose stake table the window needs for the chain tip the peers report, or `None` when
/// no peer answers within `step_timeout`.
async fn walk_target<C: Client>(
    peers: &[(Url, C)],
    anchor: Option<&TrustedAnchor>,
    epoch_height: u64,
    step_timeout: Duration,
) -> Option<EpochNumber> {
    let heights = join_all(
        peers
            .iter()
            .map(|(_, peer)| timeout(step_timeout, peer.block_height())),
    )
    .await;
    let heights: Vec<u64> = heights
        .into_iter()
        .filter_map(|height| height.ok()?.ok())
        .collect();
    if heights.is_empty() && !peers.is_empty() {
        tracing::warn!(
            "bootstrap: no state peer reported a block height; walking without a target"
        );
    }
    target_epoch(heights, anchor, epoch_height)
}

/// `epoch(tip) + 1` for the median of the reported chain heights, which the epoch root of
/// `epoch(tip) - 1` derives. With an anchor, at most `TARGET_SLACK` epochs past its window, so a
/// few lying peers cannot hold the walk. The median of an even count is the lower one.
fn target_epoch(
    mut heights: Vec<u64>,
    anchor: Option<&TrustedAnchor>,
    epoch_height: u64,
) -> Option<EpochNumber> {
    heights.sort_unstable();
    let tip = heights
        .get(heights.len().checked_sub(1)? / 2)?
        .checked_sub(1)?;
    let target = epoch_from_block_number(tip, epoch_height) + 1;
    let cap = anchor
        .map(|anchor| epoch_from_block_number(anchor.height, epoch_height) + 1 + TARGET_SLACK);
    Some(EpochNumber::new(cap.map_or(target, |cap| target.min(cap))))
}

/// Walk forward from the highest known epoch. Each step drives `add_epoch_root` through the
/// existing catchup machinery, persisting the new stake table.
///
/// Steps below the `target` are retried until the deadline. From the target up, the first step
/// that peers cannot serve marks the live tip.
async fn walk_to_tip(
    coordinator: &EpochMembershipCoordinator<SeqTypes>,
    first_epoch: EpochNumber,
    target: Option<EpochNumber>,
    params: &BootstrapParams,
) -> anyhow::Result<EpochNumber> {
    let membership = coordinator.membership();
    // Catchup resumes from the latest consecutive pair of stake tables we
    // hold, so a gap near the tip is bridged by the step that runs into it
    // rather than having to be found here.
    let mut highest = membership.highest_known_epoch().unwrap_or(first_epoch + 1);

    milestone!(
        params,
        %first_epoch,
        starting_from = %highest,
        target = ?target.map(|e| e.u64()),
        "bootstrap: walking forward",
    );

    let deadline = Instant::now() + params.deadline;
    let step_timeout = params.step_timeout;
    loop {
        let next = highest + 1;
        let derive = move |_, bound: Duration| async move {
            let step = timeout(
                step_timeout.min(bound),
                coordinator.wait_for_stake_table(next),
            );
            step.await
                .context(format!("timed out after {}s", step_timeout.as_secs()))?
                .map(drop)
                .map_err(|err| anyhow!("{err:#}"))
        };
        let below_target = target.is_some_and(|target| highest < target);
        let step = if below_target {
            let what = format!("deriving epoch {next} below the chain tip's epoch");
            retry(deadline, &what, 1, derive).await
        } else {
            derive(0, step_timeout).await
        };
        match step {
            Ok(()) => {
                tracing::debug!(%next, "bootstrap: derived stake table");
                highest = next;
            },
            Err(err) if below_target => return Err(err),
            Err(err) if err.is::<Elapsed>() => {
                tracing::warn!(%next, %err, "bootstrap: catchup timed out; proceeding with a possibly stale epoch window");
                break;
            },
            Err(err) => {
                milestone!(params, %next, %err, "bootstrap: no further epoch to derive; at the live tip");
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
    for epoch in [current, highest] {
        prepare_drb(coordinator, epoch, params.step_timeout).await;
    }
    Ok(current)
}

/// Make the DRB of `epoch` available before consensus starts. A failure only delays it: consensus
/// retries through reactive catchup.
async fn prepare_drb(
    coordinator: &EpochMembershipCoordinator<SeqTypes>,
    epoch: EpochNumber,
    step_timeout: Duration,
) {
    // A missing DRB starts a catchup that ends in `drb_ready`; its error is the start notice.
    if coordinator.membership_for_epoch(Some(epoch)).is_ok() {
        return;
    }
    match timeout(step_timeout, coordinator.wait_for_catchup(epoch)).await {
        Ok(Ok(_)) => tracing::debug!(%epoch, "bootstrap: DRB ready"),
        Ok(Err(err)) => {
            tracing::warn!(%epoch, %err, "bootstrap: DRB not ready; consensus will catch up")
        },
        Err(_) => tracing::warn!(%epoch, "bootstrap: DRB timed out; consensus will catch up"),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };

    use alloy::primitives::{Address, U256};
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
        boot: true,
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
    #[case::start_is_seeded(100, 100 * 4, Some(3..=5))]
    #[case::start_clamped(100, 100 * 5, Some(3..=6))]
    #[case::start_unclamped(100, 100 * 6, Some(3..=7))]
    #[case::window_only(100, 100 * 2, Some(3..=3))]
    #[case::pre_pos(100, 50, None)]
    #[case::genesis(100, 0, None)]
    fn install_range_cases(
        #[case] epoch_height: u64,
        #[case] anchor_height: u64,
        #[case] expected: Option<RangeInclusive<u64>>,
    ) {
        // `first_epoch = 1` seeds epochs 1 and 2.
        let range = install_range(anchor_height, epoch_height, EpochNumber::new(1)).map(epochs);
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

    /// Test chain of `ANCHOR_HEIGHT + 1` blocks and the anchor over it.
    ///
    /// Peers report that height, so the walk targets epoch `N + 1`. `mock_block_height(0)` leaves
    /// the walk without a target.
    async fn chain() -> (TestClient, TrustedAnchor) {
        let client = TestClient::with_epoch_height(EPOCH_HEIGHT);
        let header = client.header(ANCHOR_HEIGHT).await.unwrap();
        let anchor = TrustedAnchor {
            height: header.height(),
            block_comm_root: header
                .get_light_client_state(ViewNumber::genesis())
                .unwrap()
                .block_comm_root,
        };
        (client, anchor)
    }

    #[tokio::test]
    async fn anchor_header_verification() {
        let (client, anchor) = chain().await;
        let header = client.header(ANCHOR_HEIGHT).await.unwrap();
        verify_anchor_header(&header, &anchor).unwrap();

        let wrong_root = TrustedAnchor {
            block_comm_root: anchor.block_comm_root + CircuitField::from(1u64),
            ..anchor
        };
        let err = verify_anchor_header(&header, &wrong_root).unwrap_err();
        assert!(format!("{err:#}").contains("commitment root"), "{err:#}");

        let wrong_height = TrustedAnchor {
            height: ANCHOR_HEIGHT + 1,
            ..anchor
        };
        let err = verify_anchor_header(&header, &wrong_height).unwrap_err();
        assert!(format!("{err:#}").contains("height"), "{err:#}");
    }

    /// Stands in for the node's database: it holds a stake table and DRB result for each epoch the
    /// test lets it answer for, with the hash a verified epoch root header commits to.
    #[derive(Clone, Debug, Default)]
    struct SeededStore(Arc<std::sync::Mutex<HashMap<u64, StakeTuple>>>);

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
            Self(Arc::new(std::sync::Mutex::new(tables)))
        }
    }

    #[async_trait]
    impl MembershipPersistence for SeededStore {
        async fn load_stake(&self, epoch: EpochNumber) -> anyhow::Result<Option<StakeTuple>> {
            Ok(self.0.lock().unwrap().get(&epoch.u64()).cloned())
        }

        async fn load_latest_stake(
            &self,
            _limit: u64,
        ) -> anyhow::Result<Option<Vec<IndexedStake>>> {
            Ok(None)
        }

        async fn load_drb_result(&self, epoch: EpochNumber) -> anyhow::Result<Option<DrbResult>> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .contains_key(&epoch.u64())
                .then_some([0; 32]))
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
                    U256::from(100),
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

    fn peer_list(clients: &[TestClient]) -> Vec<(Url, TestClient)> {
        let url: Url = "http://peer.invalid".parse().unwrap();
        clients.iter().map(|c| (url.clone(), c.clone())).collect()
    }

    async fn bootstrap(
        coordinator: &EpochMembershipCoordinator<SeqTypes>,
        anchor: Option<TrustedAnchor>,
        peers: &[TestClient],
        params: BootstrapParams,
    ) -> anyhow::Result<EpochNumber> {
        let metrics = BootstrapMetrics::new(&NoMetrics);
        bootstrap_epoch_window(
            coordinator,
            EPOCH_HEIGHT,
            || async { anchor.context("no anchor") },
            &peer_list(peers),
            &params,
            &metrics,
        )
        .await
    }

    fn has_table(coordinator: &EpochMembershipCoordinator<SeqTypes>, epoch: u64) -> bool {
        coordinator
            .membership()
            .snapshot(EpochNumber::new(epoch))
            .is_some()
    }

    #[test_log::test(tokio::test)]
    async fn installs_window() {
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
    async fn peer_with_wrong_chain_is_rotated_past() {
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
    async fn invalid_proof_at_first_epoch_fails() {
        let (client, anchor) = chain().await;
        let range = install_range(anchor.height, EPOCH_HEIGHT, EpochNumber::new(FIRST_EPOCH));
        let range = range.unwrap();
        let first_root = root_block_in_epoch(range.start().u64() - 2, EPOCH_HEIGHT);
        client.return_invalid_proof(first_root as usize).await;

        let err = fetch_roots_from(&client, &anchor, &range, EPOCH_HEIGHT)
            .await
            .unwrap_err();

        assert!(
            format!("{err:#}").contains("verifying the epoch root"),
            "{err:#}"
        );
    }

    /// A verification failure is retried, not read as the tip, and the retry installs the window.
    #[test_log::test(tokio::test)]
    async fn invalid_proof_is_retried() {
        let (client, anchor) = chain().await;
        let coordinator = fresh_coordinator(&client).await;
        let root = root_block_in_epoch(N - 1, EPOCH_HEIGHT) as usize;
        client.return_invalid_proof(root).await;
        let heal = client.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            heal.remember_leaf(root).await;
        });

        let current = bootstrap(
            &coordinator,
            Some(anchor),
            &[client.clone(), client],
            PARAMS,
        )
        .await
        .unwrap();

        assert_eq!(current, EpochNumber::new(N));
        assert!(has_table(&coordinator, N + 1));
    }

    #[test_log::test(tokio::test)]
    async fn wrong_anchor_root_fails() {
        let (client, anchor) = chain().await;
        let range = install_range(anchor.height, EPOCH_HEIGHT, EpochNumber::new(FIRST_EPOCH));
        let anchor = TrustedAnchor {
            block_comm_root: anchor.block_comm_root + CircuitField::from(1u64),
            ..anchor
        };

        let err = fetch_roots_from(&client, &anchor, &range.unwrap(), EPOCH_HEIGHT)
            .await
            .unwrap_err();

        assert!(format!("{err:#}").contains("commitment root"), "{err:#}");
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
    async fn anchor_header_unavailable_aborts_at_the_deadline() {
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

        assert!(format!("{err:#}").contains("deadline exceeded"), "{err:#}");
    }

    /// Storage already holding the whole range is installed again: `add_epoch_root` returns early
    /// for it, but the roots are still fetched and verified.
    #[test_log::test(tokio::test)]
    async fn restart_reinstalls_the_window() {
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
        let before = client.header_requests().await;

        let current = bootstrap(
            &coordinator,
            Some(anchor),
            std::slice::from_ref(&client),
            PARAMS,
        )
        .await
        .unwrap();

        assert_eq!(current, EpochNumber::new(N));
        for epoch in (N - 4)..=(N + 1) {
            assert!(has_table(&coordinator, epoch), "epoch {epoch}");
        }
        assert!(client.header_requests().await > before);
    }

    /// A failure below the epoch the peers' chain tip needs is retried, not read as the tip.
    #[test_log::test(tokio::test)]
    async fn walk_retries_failures_below_the_target() {
        let (client, _) = chain().await;
        let store = SeededStore::for_chain(&client, (N - 4)..=(N + 1)).await;
        let withheld = store.0.lock().unwrap().remove(&(N - 1)).unwrap();
        let coordinator = coordinator(store.clone());
        for epoch in [N - 4, N - 3] {
            coordinator
                .membership()
                .load_stake_table(EpochNumber::new(epoch))
                .await;
        }
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            store.0.lock().unwrap().insert(N - 1, withheld);
        });

        let current = bootstrap(
            &coordinator,
            Some(TrustedAnchor::genesis()),
            &[client],
            PARAMS,
        )
        .await
        .unwrap();

        assert_eq!(current, EpochNumber::new(N));
        assert!(has_table(&coordinator, N + 1));
    }

    #[test_log::test(tokio::test)]
    async fn walk_below_the_target_fails_at_the_deadline() {
        let (client, _) = chain().await;
        let store = SeededStore::for_chain(&client, (N - 4)..=(N + 1)).await;
        store.0.lock().unwrap().remove(&(N - 1));
        let coordinator = coordinator(store);
        for epoch in [N - 4, N - 3] {
            coordinator
                .membership()
                .load_stake_table(EpochNumber::new(epoch))
                .await;
        }
        let params = BootstrapParams {
            deadline: Duration::from_millis(300),
            ..PARAMS
        };

        let err = bootstrap(
            &coordinator,
            Some(TrustedAnchor::genesis()),
            &[client],
            params,
        )
        .await
        .unwrap_err();

        assert!(format!("{err:#}").contains("deadline exceeded"), "{err:#}");
    }

    /// A bad proof after the first good ones fails the whole fetch.
    #[test_log::test(tokio::test)]
    async fn invalid_proof_mid_range_fails() {
        let (client, anchor) = chain().await;
        let range = install_range(anchor.height, EPOCH_HEIGHT, EpochNumber::new(FIRST_EPOCH));
        let range = range.unwrap();
        client
            .return_invalid_proof(root_block_in_epoch(N - 3, EPOCH_HEIGHT) as usize)
            .await;

        let err = fetch_roots_from(&client, &anchor, &range, EPOCH_HEIGHT)
            .await
            .unwrap_err();

        assert!(
            format!("{err:#}").contains("verifying the epoch root"),
            "{err:#}"
        );
    }

    /// A restart after an interrupted install holds a lone table at the start of the range, and the
    /// install completes it.
    #[test_log::test(tokio::test)]
    async fn partial_install_is_completed() {
        let (client, anchor) = chain().await;
        let coordinator = fresh_coordinator(&client).await;
        let first_root = client
            .header(root_block_in_epoch(N - 5, EPOCH_HEIGHT))
            .await;
        coordinator
            .add_epoch_root(first_root.unwrap())
            .await
            .unwrap();
        assert!(has_table(&coordinator, N - 3));
        assert!(!has_table(&coordinator, N - 4));
        let before = client.header_requests().await;

        let current = bootstrap(
            &coordinator,
            Some(anchor),
            std::slice::from_ref(&client),
            PARAMS,
        )
        .await
        .unwrap();

        assert_eq!(current, EpochNumber::new(N));
        for epoch in (N - 3)..=(N + 1) {
            assert!(has_table(&coordinator, epoch), "epoch {epoch}");
        }
        assert!(client.header_requests().await > before);
    }

    /// When the prover lags, the chain runs past the anchor's window and the walk continues there.
    #[test_log::test(tokio::test)]
    async fn walk_continues_above_the_window() {
        let (client, anchor) = chain().await;
        let coordinator = coordinator(SeededStore::for_chain(&client, (N - 3)..=(N + 2)).await);

        let current = bootstrap(&coordinator, Some(anchor), &[client], PARAMS)
            .await
            .unwrap();

        assert_eq!(current, EpochNumber::new(N + 1));
        assert!(has_table(&coordinator, N + 2));
    }

    /// The follower's mid-run call installs the window without boot milestones.
    #[test_log::test(tokio::test)]
    async fn mid_run_installs_window() {
        let (client, anchor) = chain().await;
        let coordinator = fresh_coordinator(&client).await;
        let params = BootstrapParams {
            boot: false,
            deadline: Duration::from_secs(2),
            ..PARAMS
        };

        let current = bootstrap(&coordinator, Some(anchor), &[client], params)
            .await
            .unwrap();

        assert_eq!(current, EpochNumber::new(N));
        for epoch in (N - 3)..=(N + 1) {
            assert!(has_table(&coordinator, epoch), "epoch {epoch}");
        }
    }

    /// A non-empty install range cannot be fetched without peers, so startup fails at once.
    #[test_log::test(tokio::test)]
    async fn install_without_peers_fails_immediately() {
        let (client, anchor) = chain().await;
        let coordinator = fresh_coordinator(&client).await;
        let started = Instant::now();

        let err = bootstrap(&coordinator, Some(anchor), &[], PARAMS)
            .await
            .unwrap_err();

        assert!(format!("{err:#}").contains("state peer"), "{err:#}");
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    /// Without finalized state there is nothing to install, so no peers is fine and the walk runs.
    #[test_log::test(tokio::test)]
    async fn genesis_anchor_without_peers_walks() {
        let (client, _) = chain().await;
        let coordinator = fresh_coordinator(&client).await;

        let current = bootstrap(&coordinator, Some(TrustedAnchor::genesis()), &[], PARAMS)
            .await
            .unwrap();

        assert_eq!(current, EpochNumber::new(FIRST_EPOCH));
        assert!(!has_table(&coordinator, N - 3));
    }

    /// An anchor read that keeps failing is retried until the deadline, not read as "walk".
    #[test_log::test(tokio::test)]
    async fn anchor_failure_aborts_at_the_deadline() {
        let (client, _) = chain().await;
        let coordinator = fresh_coordinator(&client).await;
        let params = BootstrapParams {
            deadline: Duration::from_millis(300),
            ..PARAMS
        };

        let err = bootstrap(&coordinator, None, &[client], params)
            .await
            .unwrap_err();

        let err = format!("{err:#}");
        assert!(err.contains("deadline exceeded"), "{err}");
        assert!(err.contains("no anchor"), "{err}");
    }

    /// A failed anchor read is retried and the install then runs.
    #[test_log::test(tokio::test)]
    async fn anchor_failure_is_retried() {
        let (client, anchor) = chain().await;
        let coordinator = fresh_coordinator(&client).await;
        let reads = AtomicUsize::new(0);
        let metrics = BootstrapMetrics::new(&NoMetrics);

        let current = bootstrap_epoch_window(
            &coordinator,
            EPOCH_HEIGHT,
            || async {
                ensure!(reads.fetch_add(1, Ordering::Relaxed) > 0, "L1 not ready");
                Ok(anchor)
            },
            &peer_list(&[client]),
            &PARAMS,
            &metrics,
        )
        .await
        .unwrap();

        assert_eq!(reads.load(Ordering::Relaxed), 2);
        assert_eq!(current, EpochNumber::new(N));
        assert!(has_table(&coordinator, N + 1));
    }

    #[test_log::test(tokio::test)]
    async fn pre_pos_anchor_walks() {
        let (client, _) = chain().await;
        client.mock_block_height(0).await;
        let coordinator = fresh_coordinator(&client).await;
        let early = client.header(EPOCH_HEIGHT).await.unwrap();
        let anchor = TrustedAnchor {
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

    #[rstest]
    #[case::one_peer(&[78], false, Some(9))]
    #[case::no_peer(&[], false, None)]
    #[case::empty_chain(&[0], false, None)]
    #[case::median_ignores_one_liar(&[78, 78, 10_000], false, Some(9))]
    #[case::even_takes_the_lower(&[78, 10_000], false, Some(9))]
    #[case::unanchored_liar_alone(&[10_000], false, Some(1001))]
    #[case::anchored_liar_alone(&[10_000], true, Some(N + 1 + TARGET_SLACK))]
    #[case::anchored_liars_in_majority(&[78, 10_000, 10_000], true, Some(N + 1 + TARGET_SLACK))]
    #[case::anchored_honest(&[78], true, Some(9))]
    fn target_epoch_cases(
        #[case] heights: &[u64],
        #[case] anchored: bool,
        #[case] expected: Option<u64>,
    ) {
        let anchor = TrustedAnchor {
            height: ANCHOR_HEIGHT,
            block_comm_root: CircuitField::from(0u64),
        };
        let target = target_epoch(heights.to_vec(), anchored.then_some(&anchor), EPOCH_HEIGHT);
        assert_eq!(target.map(|e| e.u64()), expected);
    }

    /// A peer that reports a huge chain cannot hold the walk past the anchor's slack.
    #[test_log::test(tokio::test)]
    async fn huge_peer_height_is_clamped_by_the_anchor() {
        let (client, anchor) = chain().await;
        client.mock_block_height(10_000).await;
        let coordinator =
            coordinator(SeededStore::for_chain(&client, (N - 3)..=(N + 1 + TARGET_SLACK)).await);

        let current = bootstrap(&coordinator, Some(anchor), &[client], PARAMS)
            .await
            .unwrap();

        assert_eq!(current, EpochNumber::new(N + TARGET_SLACK));
    }
}
