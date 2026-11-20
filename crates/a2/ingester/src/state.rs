//! The state of the chain at recent heights, for consensus nodes catching up through the API
//! instances.
//!
//! The ingester follows its store in height order, as a consensus node follows the chain: it
//! applies each block's header to the state of its parent and checks the result against the roots
//! the header commits to, and it hands each epoch root and each DRB result to membership, which
//! needs both to know the leaders whose rewards the state pays at the end of an epoch.
//!
//! For each of the newest [`KEPT_HEIGHTS`] heights it writes what the state holds besides its
//! header's roots into the store's [`Tables`]: the frontier of the block tree, the chain config,
//! and the reward balances, these only at the heights where they changed. The fee tree does not
//! change under the new protocol; it is written once, from genesis, as are the chain configs the
//! genesis file knows. API instances answer catchup requests from these rows, and a restarted
//! ingester resumes from the newest one instead of applying the chain from genesis again.
//!
//! A follower without a row of its own, in a new deployment or one whose store was pruned past it,
//! takes the state at the newest stored height from the catchup service of another API service,
//! and checks it against the stored header as it checks its own rows. Membership then gets what
//! the skipped blocks would have given it. Only if no peer has the state and the store starts at
//! genesis does the follower apply the chain from there.

use std::{
    collections::{BTreeSet, VecDeque},
    time::Duration,
};

use anyhow::{Context, bail, ensure};
use catchup_api::proto::{
    ChainConfigRequest, HeightRequest, catchup_service_client::CatchupServiceClient,
};
use chain_api::{BlocksFrontier, RewardSet, StateRow, Tables};
use chain_store::ChainStore;
use committable::{Commitment, Committable};
use either::Either;
use espresso_types::{
    ChainConfig, Leaf2, NodeState, ValidatedState,
    v0_4::{REWARD_MERKLE_TREE_V2_HEIGHT, RewardMerkleTreeV2},
};
use hotshot_types::{
    data::EpochNumber,
    traits::{
        election::{Membership as _, MembershipSnapshot as _},
        states::ValidatedState as _,
    },
    utils::{is_epoch_root, is_transition_block, root_block_in_epoch, transition_block_for_epoch},
};
use jf_merkle_tree_compat::{ForgetableMerkleTreeScheme, MerkleTreeScheme};
use membership::null_catchup::NullStateCatchup;
use serde::de::DeserializeOwned;
use tokio::time::sleep;

use crate::{
    membership::Membership,
    peers::{Peer, Peers},
};

/// How many of the newest heights the state is kept for.
pub const KEPT_HEIGHTS: usize = 256;

/// How long to wait before applying a block again that could not be applied, for instance because
/// membership does not have the stake table of its epoch yet.
const RETRY_DELAY: Duration = Duration::from_secs(1);

/// How large an answer of a peer's catchup service may be; reward balances are the largest.
const MAX_ANSWER_SIZE: usize = 256 << 20;

type RewardRoot = <RewardMerkleTreeV2 as MerkleTreeScheme>::Commitment;

/// Applies the blocks of the store in height order, and stores what the newest states hold.
pub struct ChainFollower {
    store: ChainStore,
    tables: Tables,
    node: NodeState,

    /// The API services to take the state from when the follower has no row of its own.
    peers: Peers,

    /// The heights of the kept rows, oldest first, each with the height its reward balances are
    /// stored under.
    kept: VecDeque<(u64, u64)>,

    /// The root of the reward tree of the newest row, and the height its balances are stored
    /// under.
    rewards: Option<(RewardRoot, u64)>,

    /// The chain configs stored since the follower started.
    configs: BTreeSet<Commitment<ChainConfig>>,
}

impl ChainFollower {
    pub fn new(store: ChainStore, node: NodeState, peers: Peers) -> Self {
        Self {
            tables: Tables::new(store.clone()),
            store,
            node,
            peers,
            kept: VecDeque::new(),
            rewards: None,
            configs: BTreeSet::new(),
        }
    }

    /// Follows the store, once it holds a height, from the newest stored row, else from a peer's
    /// state, else from genesis. Returns when the store's changes end, or fails if the store no
    /// longer holds the heights to start from.
    pub async fn run(mut self) -> anyhow::Result<()> {
        self.store_genesis().await?;
        let mut changes = self.store.changes();
        // Where to start depends on what the store holds: an empty store gives no height to take
        // the state of from a peer, and would leave only genesis.
        while self.store.newest().await?.is_none() {
            if changes.changed().await.is_err() {
                return Ok(());
            }
        }
        let (mut parent, mut height) = match self.start().await? {
            Some((leaf, state)) => {
                let height = leaf.height();
                tracing::info!(height, "following the chain after this height");
                (Some((leaf, state)), height + 1)
            },
            None => (None, 0),
        };
        loop {
            changes.borrow_and_update();
            let item = match self.store.item(height).await {
                Ok(Some(item)) => item,
                Ok(None) => {
                    let floor = self.store.floor().await?;
                    if floor > height {
                        bail!("the store pruned height {height} before it was applied");
                    }
                    // Not stored yet; the decide log or the gap filler stores it.
                    if changes.changed().await.is_err() {
                        return Ok(());
                    }
                    continue;
                },
                Err(err) => {
                    tracing::warn!(height, "reading the store failed: {err:#}");
                    sleep(RETRY_DELAY).await;
                    continue;
                },
            };
            let state = match &parent {
                None => Ok(ValidatedState::genesis(&self.node).0),
                Some((parent_leaf, parent_state)) => {
                    apply(&self.node, parent_state, parent_leaf, &item.leaf).await
                },
            };
            let stored = match state {
                Ok(state) => match track_epoch(&self.node, &item.leaf).await {
                    Ok(()) => self.store_state(height, &state).await.map(|()| state),
                    Err(err) => Err(err.context("membership did not take the block")),
                },
                Err(err) => Err(err.context("applying the block failed")),
            };
            match stored {
                Ok(state) => {
                    parent = Some((item.leaf, state));
                    height += 1;
                },
                Err(err) => {
                    tracing::warn!(height, "{err:#}");
                    sleep(RETRY_DELAY).await;
                },
            }
        }
    }

    /// Stores what does not change, or changes only by upgrade: the fee accounts and the chain
    /// configs of the genesis file.
    async fn store_genesis(&mut self) -> anyhow::Result<()> {
        let fees = ValidatedState::genesis(&self.node).0.fee_merkle_tree;
        let accounts: Vec<_> = fees
            .iter()
            .map(|(account, amount)| (*account, *amount))
            .collect();
        ensure!(
            accounts.len() as u64 == fees.num_leaves(),
            "the genesis fee tree is missing accounts"
        );
        self.tables.put_fee_accounts(&accounts).await?;
        let upgrades = self
            .node
            .upgrades
            .values()
            .filter_map(|upgrade| upgrade.upgrade_type.chain_config());
        let configs: Vec<_> = [self.node.chain_config, self.node.genesis_chain_config]
            .into_iter()
            .chain(upgrades)
            .collect();
        for config in configs {
            self.store_chain_config(&config).await?;
        }
        Ok(())
    }

    async fn store_chain_config(&mut self, config: &ChainConfig) -> anyhow::Result<()> {
        let commitment = config.commit();
        if !self.configs.contains(&commitment) {
            self.tables.put_chain_config(config).await?;
            self.configs.insert(commitment);
        }
        Ok(())
    }

    /// The leaf and state to continue after: the newest row's, else one from a peer, or none to
    /// start at genesis. A failed read is an error, to be tried again; a row that does not check
    /// out is ignored.
    async fn start(&mut self) -> anyhow::Result<Option<(Leaf2, ValidatedState)>> {
        let rows = self.tables.states().await?;
        if let Some((height, row)) = rows.last().cloned() {
            let item = self.store.item(height).await?;
            let balances = self.tables.rewards(row.rewards_at).await?;
            let resumed = item
                .with_context(|| format!("height {height} is not stored"))
                .and_then(|item| {
                    let state = self.rebuild(&item.leaf, &row, balances)?;
                    Ok((item.leaf, state))
                });
            match resumed {
                Ok((leaf, state)) => {
                    self.kept = rows.iter().map(|(h, row)| (*h, row.rewards_at)).collect();
                    self.rewards = Some((state.reward_merkle_tree_v2.commitment(), row.rewards_at));
                    return Ok(Some((leaf, state)));
                },
                Err(err) => tracing::warn!("ignoring the stored state: {err:#}"),
            }
        }
        match self.bootstrap().await {
            Ok(Some(start)) => return Ok(Some(start)),
            Ok(None) => {},
            Err(err) => tracing::warn!("taking the state from a peer failed: {err:#}"),
        }
        let floor = self.store.floor().await?;
        if floor > 0 {
            bail!(
                "the store keeps heights from {floor} on and no usable state; it cannot be \
                 followed from genesis"
            );
        }
        Ok(None)
    }

    /// The leaf and state at the newest stored height, from the first peer that has the state,
    /// checked against the stored header. Membership is first given what the blocks before would
    /// have given it, and the state is then stored as the follower's first row. `None` if no peer
    /// has the state.
    async fn bootstrap(&mut self) -> anyhow::Result<Option<(Leaf2, ValidatedState)>> {
        let Some(height) = self.store.newest().await? else {
            return Ok(None);
        };
        let leaf = self
            .store
            .item(height)
            .await?
            .with_context(|| format!("height {height} is not stored"))?
            .leaf;
        let membership = Membership::new(&self.node)?;
        self.peers
            .refresh(membership.epoch_of(height), &membership)
            .await;
        let mut state = None;
        for peer in self.peers.shuffled() {
            match self.state_from(&peer, &leaf).await {
                Ok(taken) => {
                    tracing::info!(%peer, height, "took the state from a peer");
                    state = Some(taken);
                    break;
                },
                Err(err) => tracing::debug!(%peer, height, "no state from the peer: {err:#}"),
            }
        }
        let Some(state) = state else {
            return Ok(None);
        };
        self.prepare_membership(height, &membership).await?;
        self.store_state(height, &state).await?;
        Ok(Some((leaf, state)))
    }

    /// The state after `leaf` as `peer`'s catchup service has it, checked against the leaf's
    /// header. The chain config comes from the store if it holds it.
    async fn state_from(&self, peer: &Peer, leaf: &Leaf2) -> anyhow::Result<ValidatedState> {
        let height = leaf.height();
        let header = leaf.block_header();
        let mut client = CatchupServiceClient::new(peer.channel.clone())
            .max_decoding_message_size(MAX_ANSWER_SIZE);
        let frontier: Option<BlocksFrontier> = match ValidatedState::from_header(header)
            .block_merkle_tree
            .num_leaves()
        {
            0 => None,
            _ => Some(decoded(
                client
                    .get_blocks_frontier(HeightRequest { height })
                    .await?
                    .into_inner()
                    .data,
            )?),
        };
        let balances: Option<RewardSet> = match header.reward_merkle_tree_root() {
            Either::Right(_) => Some(decoded(
                client
                    .get_reward_balances_v2(HeightRequest { height })
                    .await?
                    .into_inner()
                    .data,
            )?),
            Either::Left(_) => None,
        };
        let commitment = header.chain_config().commit().to_string();
        let chain_config = match self.tables.chain_config(&commitment).await? {
            Some(config) => config,
            None => decoded(
                client
                    .get_chain_config(ChainConfigRequest { commitment })
                    .await?
                    .into_inner()
                    .data,
            )?,
        };
        let row = StateRow {
            frontier,
            chain_config,
            rewards_at: height,
        };
        self.rebuild(leaf, &row, balances)
    }

    /// Hands membership what the blocks up to `height` would have given it, had the follower
    /// applied them: the epoch roots of the stored epochs up to `height`'s, which give the stake
    /// tables up to two epochs later, and the DRB results of `height`'s epoch and the next. A DRB
    /// result comes from the transition block before its epoch, or, where the store no longer
    /// holds that block, is computed from the epoch root, which takes as long as the DRB demands.
    async fn prepare_membership(&self, height: u64, membership: &Membership) -> anyhow::Result<()> {
        let epoch_height = membership.epoch_height();
        let epoch = *membership.epoch_of(height);
        for e in epoch.saturating_sub(2)..=epoch {
            let root = root_block_in_epoch(e, epoch_height);
            if root <= height
                && let Some(item) = self.store.item(root).await?
            {
                track_epoch(&self.node, &item.leaf).await?;
            }
        }
        // The first two epochs run on the genesis committee and its DRB result.
        let first = *membership.first_epoch();
        for e in (epoch.max(first + 2))..=epoch + 1 {
            let transition = transition_block_for_epoch(e - 1, epoch_height);
            let coordinator = &self.node.coordinator;
            let known = coordinator
                .membership()
                .snapshot(EpochNumber::new(e))
                .is_some_and(|snapshot| snapshot.has_drb());
            if transition > height || known {
                continue;
            }
            match self.store.item(transition).await? {
                Some(item) if item.leaf.next_drb_result.is_some() => {
                    track_epoch(&self.node, &item.leaf).await?;
                },
                _ => {
                    let root = root_block_in_epoch(e - 2, epoch_height);
                    let item = self.store.item(root).await?.with_context(|| {
                        format!(
                            "the store holds neither the transition block {transition} nor the \
                             epoch root {root} that give the DRB result of epoch {e}"
                        )
                    })?;
                    tracing::info!(
                        epoch = e,
                        "computing the DRB result the store does not hold"
                    );
                    let drb = coordinator
                        .compute_drb_result(EpochNumber::new(e), item.leaf)
                        .await
                        .map_err(|err| anyhow::anyhow!("computing the DRB result: {err}"))?;
                    coordinator.supply_drb(EpochNumber::new(e), drb);
                },
            }
        }
        Ok(())
    }

    /// The state after `leaf` from its stored `row` and reward `balances`, checked against the
    /// leaf's header.
    fn rebuild(
        &self,
        leaf: &Leaf2,
        row: &StateRow,
        balances: Option<chain_api::RewardSet>,
    ) -> anyhow::Result<ValidatedState> {
        let header = leaf.block_header();
        let mut state = ValidatedState::from_header(header);

        if let Some(last) = state.block_merkle_tree.num_leaves().checked_sub(1) {
            let frontier = row
                .frontier
                .clone()
                .context("no frontier of the block tree")?;
            let elem = *frontier.elem().context("a frontier without its element")?;
            state
                .block_merkle_tree
                .remember(last, elem, &frontier)
                .context("the frontier does not prove against the block root")?;
        }

        ensure!(
            row.chain_config.commit() == header.chain_config().commit(),
            "the chain config is not the header's"
        );
        state.chain_config = row.chain_config.into();

        // The fee tree does not change under the new protocol.
        let genesis = ValidatedState::genesis(&self.node).0;
        ensure!(
            genesis.fee_merkle_tree.commitment() == header.fee_merkle_tree_root(),
            "the genesis fee tree is not the header's"
        );
        state.fee_merkle_tree = genesis.fee_merkle_tree;

        if let Either::Right(root) = header.reward_merkle_tree_root() {
            let balances = balances
                .with_context(|| format!("no reward balances at height {}", row.rewards_at))?;
            let tree = RewardMerkleTreeV2::from_kv_set(REWARD_MERKLE_TREE_V2_HEIGHT, balances)
                .context("rebuilding the reward tree")?;
            ensure!(
                tree.commitment() == root,
                "the reward tree is not the header's"
            );
            state.reward_merkle_tree_v2 = tree;
        }
        Ok(state)
    }

    /// Stores what `state`, the state after `height`, holds besides its header's roots, and
    /// forgets the rows of heights no longer among the newest kept.
    async fn store_state(&mut self, height: u64, state: &ValidatedState) -> anyhow::Result<()> {
        let frontier = match state.block_merkle_tree.num_leaves().checked_sub(1) {
            Some(last) => {
                let (_, frontier) = state
                    .block_merkle_tree
                    .lookup(last)
                    .expect_ok()
                    .map_err(|_| anyhow::anyhow!("the block tree forgot its frontier"))?;
                Some(frontier)
            },
            None => None,
        };
        let chain_config = state
            .chain_config
            .resolve()
            .context("the chain config is not resolved")?;
        self.store_chain_config(&chain_config).await?;

        let tree = &state.reward_merkle_tree_v2;
        let root = tree.commitment();
        let (rewards, rewards_at) = match self.rewards {
            Some((stored, at)) if stored == root => (None, at),
            _ => {
                let balances: Vec<_> = tree
                    .iter()
                    .map(|(account, amount)| (*account, *amount))
                    .collect();
                ensure!(
                    balances.len() as u64 == tree.num_leaves(),
                    "the reward tree is missing accounts"
                );
                (Some(balances), height)
            },
        };

        let mut kept = self.kept.clone();
        kept.push_back((height, rewards_at));
        while kept.len() > KEPT_HEIGHTS {
            kept.pop_front();
        }
        let (keep_from, keep_rewards_from) = *kept.front().expect("one was just added");
        let row = StateRow {
            frontier,
            chain_config,
            rewards_at,
        };
        self.tables
            .put_state(height, &row, rewards.as_ref(), keep_from, keep_rewards_from)
            .await?;
        (self.kept, self.rewards) = (kept, Some((root, rewards_at)));
        Ok(())
    }
}

/// The state after `leaf`, its parent's being `parent_state`, checked against the roots its header
/// commits to. Of the block tree it keeps only the frontier, which is all the next block needs.
async fn apply(
    node: &NodeState,
    parent_state: &ValidatedState,
    parent_leaf: &Leaf2,
    leaf: &Leaf2,
) -> anyhow::Result<ValidatedState> {
    let header = leaf.block_header();
    let mut parent_state = parent_state.clone();
    // A block of a newer version runs under its own chain config; the genesis file need not
    // record every upgrade the chain went through.
    if header.version() > parent_leaf.block_header().version() {
        parent_state.chain_config = header.chain_config();
    }
    let (mut state, ..) = parent_state
        .apply_header(
            node,
            &NullStateCatchup::default(),
            parent_leaf,
            header,
            header.version(),
            leaf.view_number(),
        )
        .await?;
    let height = header.height();
    ensure!(
        state.chain_config.commit() == header.chain_config().commit(),
        "the chain config after block {height} does not match its header"
    );
    ensure!(
        state.block_merkle_tree.commitment() == header.block_merkle_tree_root(),
        "the block tree after block {height} does not match its header"
    );
    ensure!(
        state.fee_merkle_tree.commitment() == header.fee_merkle_tree_root(),
        "the fee tree after block {height} does not match its header"
    );
    let rewards_match = match header.reward_merkle_tree_root() {
        Either::Left(root) => state.reward_merkle_tree_v1.commitment() == root,
        Either::Right(root) => state.reward_merkle_tree_v2.commitment() == root,
    };
    ensure!(
        rewards_match,
        "the reward tree after block {height} does not match its header"
    );
    if let Some(before_last) = state.block_merkle_tree.num_leaves().checked_sub(2) {
        // Only the leaf just appended can still be unforgotten before it.
        let _ = state.block_merkle_tree.forget(before_last);
    }
    Ok(state)
}

fn decoded<V: DeserializeOwned>(bytes: Vec<u8>) -> anyhow::Result<V> {
    Ok(bincode::deserialize(&bytes)?)
}

/// Hands membership what `leaf` tells it: an epoch root gives the stake table of the epoch two
/// later, and a transition block the DRB result of the next epoch.
async fn track_epoch(node: &NodeState, leaf: &Leaf2) -> anyhow::Result<()> {
    let epoch_height = node.epoch_height.unwrap_or_default();
    let height = leaf.height();
    let Some(epoch) = leaf.epoch(epoch_height) else {
        return Ok(());
    };
    let membership = &node.coordinator;
    let has_stake_table = |epoch| {
        use hotshot_types::traits::election::Membership as _;
        membership.membership().snapshot(epoch).is_some()
    };
    if is_epoch_root(height, epoch_height) && !has_stake_table(epoch + 2) {
        membership
            .add_epoch_root(leaf.block_header().clone())
            .await
            .map_err(|err| anyhow::anyhow!("{err}"))
            .with_context(|| format!("deriving the stake table of epoch {}", epoch + 2))?;
    }
    if is_transition_block(height, epoch_height)
        && let Some(drb) = leaf.next_drb_result
    {
        membership.supply_drb(epoch + 1, drb);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use chain_store::{Item, Parts, mem::MemBackend};
    use hotshot_types::simple_certificate::QuorumCertificate2;
    use jf_merkle_tree_compat::AppendableMerkleTreeScheme;
    use versions::{NEW_PROTOCOL_VERSION, Upgrade};

    use super::*;

    /// A follower of `store` without peers to take the state from.
    fn follower(store: ChainStore, node: NodeState) -> ChainFollower {
        let peers = Peers::new(Tables::new(store.clone()), Vec::new(), None);
        ChainFollower::new(store, node, peers)
    }

    /// A store holding the genesis block, and the state after it.
    async fn genesis(node: &NodeState) -> (ChainStore, ValidatedState) {
        let state = ValidatedState::genesis(node).0;
        let leaf = Leaf2::genesis(&state, node, NEW_PROTOCOL_VERSION).await;
        stored(leaf, state, node).await
    }

    /// A store holding the genesis block as if a block came before it, so that the block tree
    /// after it is not empty, and that state.
    async fn after_a_block(node: &NodeState) -> (ChainStore, ValidatedState) {
        let mut state = ValidatedState::genesis(node).0;
        let mut leaf = Leaf2::genesis(&state, node, NEW_PROTOCOL_VERSION).await;
        state
            .block_merkle_tree
            .push(leaf.block_header().commit())
            .unwrap();
        *leaf.block_header_mut().block_merkle_tree_root_mut() =
            state.block_merkle_tree.commitment();
        stored(leaf, state, node).await
    }

    async fn stored(
        leaf: Leaf2,
        state: ValidatedState,
        node: &NodeState,
    ) -> (ChainStore, ValidatedState) {
        let upgrade = Upgrade::trivial(NEW_PROTOCOL_VERSION);
        let item = Item {
            leaf,
            cert1: QuorumCertificate2::genesis(&state, node, upgrade).await,
        };
        let store = ChainStore::new(MemBackend::default());
        store.put(&item, &Parts::default()).await.unwrap();
        (store, state)
    }

    /// A restarted follower resumes from the newest row with the state it stored.
    #[tokio::test]
    async fn resumes_from_the_newest_row() {
        let node = NodeState::mock().with_current_version(NEW_PROTOCOL_VERSION);
        for (store, state) in [genesis(&node).await, after_a_block(&node).await] {
            resumes(store, state, node.clone()).await;
        }
    }

    async fn resumes(store: ChainStore, state: ValidatedState, node: NodeState) {
        let mut first = follower(store.clone(), node.clone());
        assert!(first.start().await.unwrap().is_none());

        first.store_state(0, &state).await.unwrap();
        let mut restarted = follower(store, node);
        let (leaf, resumed) = restarted.start().await.unwrap().expect("a row");
        assert_eq!(leaf.height(), 0);
        assert_eq!(
            resumed.block_merkle_tree.commitment(),
            state.block_merkle_tree.commitment()
        );
        assert_eq!(
            resumed.reward_merkle_tree_v2.commitment(),
            state.reward_merkle_tree_v2.commitment()
        );
        assert_eq!(resumed.chain_config.commit(), state.chain_config.commit());
        assert_eq!(
            resumed.fee_merkle_tree.num_leaves(),
            state.fee_merkle_tree.num_leaves()
        );
    }

    /// Reward balances are written only where they change, and rows and balances older than the
    /// newest kept heights are forgotten.
    #[tokio::test]
    async fn keeps_the_newest_rows_and_shares_unchanged_rewards() {
        let node = NodeState::mock().with_current_version(NEW_PROTOCOL_VERSION);
        let (store, state) = genesis(&node).await;
        let tables = Tables::new(store.clone());
        let mut follower = follower(store, node);
        let last = KEPT_HEIGHTS as u64 + 1;
        for height in 0..=last {
            follower.store_state(height, &state).await.unwrap();
        }

        let rows = tables.states().await.unwrap();
        assert_eq!(rows.len(), KEPT_HEIGHTS);
        assert_eq!(rows.first().unwrap().0, 2);
        assert!(rows.iter().all(|(_, row)| row.rewards_at == 0));
        assert!(tables.rewards(0).await.unwrap().is_some());
        assert!(tables.rewards(1).await.unwrap().is_none());
        assert_eq!(
            tables
                .chain_config(&state.chain_config.commit().to_string())
                .await
                .unwrap()
                .map(|config| config.commit()),
            Some(state.chain_config.commit())
        );
    }

    /// Serves the catchup service of an API instance over `store`, returning its URL.
    async fn serve_catchup(store: &ChainStore) -> url::Url {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap())
            .parse()
            .unwrap();
        let service = api_service::ApiService::new(store.clone(), "127.0.0.1:1".parse().unwrap())
            .with_catchup(api_service::catchup::Catchup::new(store.clone(), 10));
        tokio::spawn(service.serve(listener));
        url
    }

    /// A follower without a row takes the state at the newest stored height from a peer's
    /// catchup service, checks it against the stored header, and stores it as its first row.
    #[tokio::test]
    async fn takes_the_state_from_a_peer() {
        let node = NodeState::mock()
            .with_current_version(NEW_PROTOCOL_VERSION)
            .with_epoch_height(10);
        let (peer_store, state) = after_a_block(&node).await;
        follower(peer_store.clone(), node.clone())
            .store_state(0, &state)
            .await
            .unwrap();
        let url = serve_catchup(&peer_store).await;

        let store = ChainStore::new(MemBackend::default());
        let item = peer_store.item(0).await.unwrap().unwrap();
        store.put(&item, &Parts::default()).await.unwrap();
        let peers = Peers::new(Tables::new(store.clone()), vec![url], None);
        let mut follower = ChainFollower::new(store.clone(), node, peers);

        let (leaf, taken) = follower.start().await.unwrap().expect("the peer's state");
        assert_eq!(leaf.height(), 0);
        assert_eq!(
            taken.block_merkle_tree.commitment(),
            state.block_merkle_tree.commitment()
        );
        assert_eq!(
            taken.reward_merkle_tree_v2.commitment(),
            state.reward_merkle_tree_v2.commitment()
        );
        assert!(Tables::new(store).state(0).await.unwrap().is_some());
    }

    /// A peer's state that does not match the stored header is not taken; with a store that
    /// starts at genesis, the follower then starts there.
    #[tokio::test]
    async fn refuses_a_peer_state_that_does_not_match() {
        let node = NodeState::mock()
            .with_current_version(NEW_PROTOCOL_VERSION)
            .with_epoch_height(10);
        let (peer_store, peer_state) = genesis(&node).await;
        follower(peer_store.clone(), node.clone())
            .store_state(0, &peer_state)
            .await
            .unwrap();
        let url = serve_catchup(&peer_store).await;
        // A different block at the same height, whose block tree is not empty.
        let (store, _) = after_a_block(&node).await;
        let peers = Peers::new(Tables::new(store.clone()), vec![url], None);
        let mut follower = ChainFollower::new(store.clone(), node, peers);

        assert!(
            follower.start().await.unwrap().is_none(),
            "starts at genesis"
        );
        assert!(Tables::new(store).state(0).await.unwrap().is_none());
    }

    /// A follower that the store prunes past stops instead of waiting for a height it will not
    /// get.
    #[tokio::test]
    async fn stops_when_pruned_past() {
        let node = NodeState::mock().with_current_version(NEW_PROTOCOL_VERSION);
        let (store, _) = genesis(&node).await;
        let tables = Tables::new(store.clone());
        let mut run = tokio::spawn(follower(store.clone(), node).run());
        tokio::time::timeout(Duration::from_secs(10), async {
            while tables.state(0).await.unwrap().is_none() {
                sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("genesis is applied");
        assert!(tables.fee_accounts().await.unwrap().is_some());
        store.prune_below(2).await.unwrap();
        tokio::time::timeout(Duration::from_secs(10), &mut run)
            .await
            .expect("the follower stops")
            .unwrap()
            .expect_err("it was pruned past");
    }

    /// A store pruned past genesis, without a row, cannot be followed.
    #[tokio::test]
    async fn refuses_a_pruned_store_without_a_row() {
        let node = NodeState::mock().with_current_version(NEW_PROTOCOL_VERSION);
        let (store, _) = genesis(&node).await;
        store.prune_below(1).await.unwrap();
        let mut pruned = follower(store, node);
        pruned.start().await.expect_err("nothing to start from");
    }
}
