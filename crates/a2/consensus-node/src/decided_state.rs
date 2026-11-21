//! The decided state in the consensus store.
//!
//! Consensus keeps what it needs of the `ValidatedState` of its anchor, so that a restarted node
//! validates the next blocks without fetching state. Under the new protocol the fee tree and the
//! v1 reward tree never change, so their roots in the header are enough. The block tree needs
//! only its frontier to take the next block. The v2 reward tree changes at the last block of an
//! epoch, and paying out rewards needs all of it, so every account is kept.
//!
//! Each part is checked against the anchor's header when it is loaded, and a part that does not
//! match is left out, to be fetched as for a node without the store.

use std::sync::LazyLock;

use alloy::primitives::Address;
use anyhow::Context;
use app_tables::{AppWrite, TableName};
use committable::{Commitment, Committable};
use consensus::{
    state::StateEntry,
    storage::{DecidedState, DecidedWrites},
    store::StateChange,
};
use espresso_types::{
    BlockMerkleTree, ChainConfig, Leaf2, SeqTypes, ValidatedState,
    v0_3::RewardAmount,
    v0_4::{
        REWARD_MERKLE_TREE_V2_HEIGHT, RewardAccountV2, RewardMerkleCommitmentV2, RewardMerkleTreeV2,
    },
};
use hotshot_types::traits::states::ValidatedState as _;
use jf_merkle_tree_compat::{ForgetableMerkleTreeScheme, MerkleTreeScheme};

use crate::store::NodeStore;

type BlocksFrontier = <BlockMerkleTree as MerkleTreeScheme>::MembershipProof;

/// Fixed keys to the parts of the decided state other than the reward accounts.
static STATE: LazyLock<TableName> = LazyLock::new(|| TableName::of("decided-state"));

/// Reward account address to its balance in the v2 reward tree.
static REWARD_ACCOUNTS: LazyLock<TableName> = LazyLock::new(|| TableName::of("reward-accounts"));

/// The frontier of the block tree.
const FRONTIER: &[u8] = b"frontier";

/// The resolved chain config.
const CHAIN_CONFIG: &[u8] = b"chain-config";

/// The root of the v2 reward tree that [`REWARD_ACCOUNTS`] holds.
const REWARD_ROOT: &[u8] = b"reward-root";

/// The state at `anchor`, with the parts the store holds for it.
pub fn load(store: &NodeStore, anchor: &Leaf2) -> anyhow::Result<ValidatedState> {
    let header = anchor.block_header();
    let mut state = ValidatedState::from_header(header);

    if let Some(frontier) = get::<BlocksFrontier>(store, FRONTIER)?
        && let Some(last) = state.block_merkle_tree.num_leaves().checked_sub(1)
        && let Some(elem) = frontier.elem()
        && let Err(err) = state.block_merkle_tree.remember(last, *elem, &frontier)
    {
        tracing::info!(%err, "the stored block frontier is not the anchor's");
    }

    if let Some(config) = get::<ChainConfig>(store, CHAIN_CONFIG)?
        && config.commit() == state.chain_config.commit()
    {
        state.chain_config = config.into();
    }

    let root = header.reward_merkle_tree_root().right();
    if root.is_some() && get::<RewardMerkleCommitmentV2>(store, REWARD_ROOT)? == root {
        let accounts = store
            .app_entries(&REWARD_ACCOUNTS, &[], None)?
            .into_iter()
            .map(|(key, value)| {
                let account = RewardAccountV2(Address::try_from(&key[..])?);
                let balance: RewardAmount = bincode::deserialize(&value)?;
                anyhow::Ok((account, balance))
            })
            .collect::<anyhow::Result<Vec<_>>>()
            .context("reading the reward accounts")?;
        let tree = RewardMerkleTreeV2::from_kv_set(REWARD_MERKLE_TREE_V2_HEIGHT, accounts)
            .context("rebuilding the reward tree")?;
        if Some(tree.commitment()) == root {
            state.reward_merkle_tree_v2 = tree;
        } else {
            tracing::warn!("the stored reward accounts do not match their root");
        }
    }
    Ok(state)
}

/// Writes the decided state of each decided leaf, as [`DecidedState`] for consensus.
pub struct DecidedStateWriter {
    /// The height of the latest leaf written for, and the root of its reward tree.
    height: u64,
    leaf_root: Option<RewardMerkleCommitmentV2>,

    /// The root the stored reward accounts make up.
    stored_root: Option<RewardMerkleCommitmentV2>,

    /// The commitment of the stored chain config.
    chain_config: Option<Commitment<ChainConfig>>,

    /// The height of the latest leaf whose state its decide log entry got.
    state_sent: Option<u64>,
}

impl DecidedStateWriter {
    /// Continues from what `store` holds, with `anchor` the latest decided leaf.
    pub fn new(store: &NodeStore, anchor: &Leaf2) -> anyhow::Result<Self> {
        Ok(Self {
            height: anchor.height(),
            leaf_root: anchor.block_header().reward_merkle_tree_root().right(),
            stored_root: get(store, REWARD_ROOT)?,
            chain_config: get::<ChainConfig>(store, CHAIN_CONFIG)?.map(|c| c.commit()),
            state_sent: None,
        })
    }
}

impl DecidedState<SeqTypes> for DecidedStateWriter {
    /// Writes the frontier and the chain config whenever the leaf's state holds them. The
    /// reward accounts follow the leaf's delta when they hold the parent's tree, and are written
    /// in full when the leaf's tree is complete. Otherwise they stay behind, and [`load`]
    /// leaves them out until a complete tree catches them up.
    ///
    /// The decide log entry gets the leaf's frontier and chain config, and every reward balance
    /// unless they are the parent's, whenever the leaf's state holds them all.
    fn decided(&mut self, leaf: &Leaf2, entry: Option<&StateEntry<SeqTypes>>) -> DecidedWrites {
        let height = leaf.height();
        // A decide that fills a gap trails the anchor this state has moved past.
        if height <= self.height {
            return DecidedWrites::default();
        }
        let parent_root = (height == self.height + 1)
            .then_some(self.leaf_root)
            .flatten();
        let root = leaf.block_header().reward_merkle_tree_root().right();
        // Balances left out stand for the parent's, which only a parent whose entry got its
        // state gives the reader.
        let rewards_unchanged = self.state_sent == Some(self.height)
            && height == self.height + 1
            && self.leaf_root == root;
        self.height = height;
        self.leaf_root = root;
        let state = entry.and_then(|e| state_change(leaf, &e.state, rewards_unchanged));
        if state.is_some() {
            self.state_sent = Some(height);
        }
        DecidedWrites {
            writes: self.writes(entry, height, parent_root, root),
            state,
        }
    }
}

impl DecidedStateWriter {
    /// The writes to the decided state for the leaf at `height`, whose state is `entry`'s, with
    /// reward root `root` and its parent's `parent_root`, if known.
    fn writes(
        &mut self,
        entry: Option<&StateEntry<SeqTypes>>,
        height: u64,
        parent_root: Option<RewardMerkleCommitmentV2>,
        root: Option<RewardMerkleCommitmentV2>,
    ) -> Vec<AppWrite> {
        let state = entry.map(|e| &*e.state);
        let frontier = state.and_then(|s| {
            let last = s.block_merkle_tree.num_leaves().checked_sub(1)?;
            Some(s.block_merkle_tree.lookup(last).expect_ok().ok()?.1)
        });
        let config = state
            .and_then(|s| s.chain_config.resolve())
            .filter(|c| Some(c.commit()) != self.chain_config);
        let tree = state
            .map(|s| &s.reward_merkle_tree_v2)
            .filter(|t| root.is_some() && root != self.stored_root && Some(t.commitment()) == root);
        let changed = tree.and_then(|tree| {
            let delta = entry?.delta.as_ref()?;
            if self.stored_root.is_none() || self.stored_root != parent_root {
                return None;
            }
            delta
                .rewards_delta
                .iter()
                .map(|account| Some((*account, tree.lookup(*account).expect_ok().ok()?.0)))
                .collect::<Option<Vec<_>>>()
        });
        let complete =
            tree.filter(|t| changed.is_none() && t.iter().count() as u64 == t.num_leaves());

        let writes = (|| {
            let mut writes = Vec::new();
            if let Some(frontier) = &frontier {
                writes.push(put(FRONTIER, frontier)?);
            }
            if let Some(config) = &config {
                writes.push(put(CHAIN_CONFIG, config)?);
            }
            if let Some(changed) = &changed {
                for (account, balance) in changed {
                    writes.push(put_account(account, balance)?);
                }
            } else if let Some(tree) = complete {
                writes.push(AppWrite::Clear {
                    table: REWARD_ACCOUNTS.clone(),
                });
                for (account, balance) in tree.iter() {
                    writes.push(put_account(account, balance)?);
                }
            }
            if let Some(root) = root
                && (changed.is_some() || complete.is_some())
            {
                writes.push(put(REWARD_ROOT, &root)?);
            }
            anyhow::Ok(writes)
        })();
        match writes {
            Ok(writes) => {
                if let Some(config) = config {
                    self.chain_config = Some(config.commit());
                }
                if changed.is_some() || complete.is_some() {
                    self.stored_root = root;
                }
                writes
            },
            Err(err) => {
                tracing::error!(%err, height, "failed to encode the decided state");
                Vec::new()
            },
        }
    }
}

/// The parts of `state`, the state after `leaf`, for `leaf`'s decide log entry: the frontier, the
/// chain config, and every reward balance unless `rewards_unchanged`. `None` if the state lacks
/// any of them, or they do not match the leaf's header.
fn state_change(
    leaf: &Leaf2,
    state: &ValidatedState,
    rewards_unchanged: bool,
) -> Option<StateChange> {
    let header = leaf.block_header();
    let frontier = match state.block_merkle_tree.num_leaves().checked_sub(1) {
        Some(last) => {
            let (_, frontier) = state.block_merkle_tree.lookup(last).expect_ok().ok()?;
            bincode::serialize(&frontier).ok()?
        },
        None => Vec::new(),
    };
    let config = state
        .chain_config
        .resolve()
        .filter(|c| c.commit() == header.chain_config().commit())?;
    let rewards = match rewards_unchanged {
        true => None,
        false => {
            let tree = &state.reward_merkle_tree_v2;
            let root = header.reward_merkle_tree_root().right();
            if root.is_some_and(|root| tree.commitment() != root) {
                return None;
            }
            let balances: Vec<(RewardAccountV2, RewardAmount)> = tree
                .iter()
                .map(|(account, amount)| (*account, *amount))
                .collect();
            if balances.len() as u64 != tree.num_leaves() {
                return None;
            }
            Some(bincode::serialize(&balances).ok()?)
        },
    };
    Some(StateChange {
        frontier,
        chain_config: bincode::serialize(&config).ok()?,
        rewards,
    })
}

/// The value under `key`, or `None` if it is missing or no longer decodes, as after a release that
/// changed its type: it is then rebuilt from later decides, and must not stop the node.
fn get<V: serde::de::DeserializeOwned>(store: &NodeStore, key: &[u8]) -> anyhow::Result<Option<V>> {
    let Some(bytes) = store.app_get(&STATE, key)? else {
        return Ok(None);
    };
    Ok(bincode::deserialize(&bytes)
        .inspect_err(|err| {
            let key = String::from_utf8_lossy(key);
            tracing::warn!(%err, %key, "ignoring a decided state row that does not decode");
        })
        .ok())
}

fn put(key: &[u8], value: &impl serde::Serialize) -> anyhow::Result<AppWrite> {
    Ok(AppWrite::Put {
        table: STATE.clone(),
        key: key.to_vec().into(),
        value: bincode::serialize(value)?.into(),
    })
}

fn put_account(account: &RewardAccountV2, balance: &RewardAmount) -> anyhow::Result<AppWrite> {
    Ok(AppWrite::Put {
        table: REWARD_ACCOUNTS.clone(),
        key: account.0.to_vec().into(),
        value: bincode::serialize(balance)?.into(),
    })
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, sync::Arc};

    use alloy::primitives::U256;
    use espresso_types::{NodeState, v0_4::Delta};
    use jf_merkle_tree_compat::{AppendableMerkleTreeScheme, UniversalMerkleTreeScheme};
    use versions::NEW_PROTOCOL_VERSION;

    use super::*;

    const A: RewardAccountV2 = RewardAccountV2(Address::repeat_byte(0xa));
    const B: RewardAccountV2 = RewardAccountV2(Address::repeat_byte(0xb));

    /// The writer follows the reward accounts through full trees and deltas, and a restart at the
    /// latest decided leaf gets back its frontier, chain config and reward tree.
    #[tokio::test]
    async fn restores_the_state_of_the_latest_decide() {
        let (_dir, store) = store().await;
        let (leaf0, s0) = genesis().await;
        let mut writer = DecidedStateWriter::new(&store, &leaf0).unwrap();

        let s1 = next(&s0, &leaf0, &[(A, 1), (B, 2)]);
        let leaf1 = child(&leaf0, &s1);
        let writes = writer
            .decided(&leaf1, Some(&entry(&leaf1, &s1, Some(&[A, B]))))
            .writes;
        store.app_write(&writes).await.unwrap();
        assert_restored(&store, &leaf1, &s1);

        let s2 = next(&s1, &leaf1, &[(A, 5)]);
        let leaf2 = child(&leaf1, &s2);
        let writes = writer
            .decided(&leaf2, Some(&entry(&leaf2, &s2, Some(&[A]))))
            .writes;
        assert_eq!(
            account_writes(&writes),
            1,
            "only the changed account is written"
        );
        store.app_write(&writes).await.unwrap();
        assert_restored(&store, &leaf2, &s2);
    }

    /// Without the state of a decided leaf the accounts fall behind and are not restored, until
    /// a leaf with a complete tree brings them up to date. An incomplete tree does not.
    #[tokio::test]
    async fn catches_up_on_a_complete_tree() {
        let (_dir, store) = store().await;
        let (leaf0, s0) = genesis().await;
        let mut writer = DecidedStateWriter::new(&store, &leaf0).unwrap();
        let s1 = next(&s0, &leaf0, &[(A, 1), (B, 2)]);
        let leaf1 = child(&leaf0, &s1);
        store
            .app_write(
                &writer
                    .decided(&leaf1, Some(&entry(&leaf1, &s1, None)))
                    .writes,
            )
            .await
            .unwrap();

        let s2 = next(&s1, &leaf1, &[(A, 3)]);
        let leaf2 = child(&leaf1, &s2);
        store
            .app_write(&writer.decided(&leaf2, None).writes)
            .await
            .unwrap();
        let restored = load(&store, &leaf2).unwrap();
        assert!(
            restored
                .reward_merkle_tree_v2
                .lookup(A)
                .expect_ok()
                .is_err()
        );

        let mut s3 = next(&s2, &leaf2, &[(B, 4)]);
        s3.reward_merkle_tree_v2.forget(A).expect_ok().unwrap();
        let leaf3 = child(&leaf2, &s3);
        let writes = writer
            .decided(&leaf3, Some(&entry(&leaf3, &s3, Some(&[B]))))
            .writes;
        assert_eq!(account_writes(&writes), 0);
        store.app_write(&writes).await.unwrap();

        let s4 = next(&next(&s2, &leaf2, &[(B, 4)]), &leaf3, &[(B, 6)]);
        let leaf4 = child(&leaf3, &s4);
        store
            .app_write(
                &writer
                    .decided(&leaf4, Some(&entry(&leaf4, &s4, Some(&[B]))))
                    .writes,
            )
            .await
            .unwrap();
        assert_restored(&store, &leaf4, &s4);
    }

    /// A decide that fills a gap below the latest decided leaf writes nothing.
    #[tokio::test]
    async fn ignores_a_decide_that_fills_a_gap() {
        let (_dir, store) = store().await;
        let (leaf0, s0) = genesis().await;
        let s1 = next(&s0, &leaf0, &[(A, 1)]);
        let leaf1 = child(&leaf0, &s1);
        let mut writer = DecidedStateWriter::new(&store, &leaf1).unwrap();
        assert!(
            writer
                .decided(&leaf1, Some(&entry(&leaf1, &s1, None)))
                .writes
                .is_empty()
        );
    }

    /// The decide log entry of a leaf with its state gets the frontier, the chain config and
    /// every reward balance, these only where they differ from the parent's; one without its
    /// state gets none.
    #[tokio::test]
    async fn gives_the_decide_log_the_state_of_each_leaf() {
        let (_dir, store) = store().await;
        let (leaf0, s0) = genesis().await;
        let mut writer = DecidedStateWriter::new(&store, &leaf0).unwrap();
        let balances = |state: &ValidatedState| -> Vec<(RewardAccountV2, RewardAmount)> {
            let tree = &state.reward_merkle_tree_v2;
            tree.iter().map(|(a, b)| (*a, *b)).collect()
        };

        let s1 = next(&s0, &leaf0, &[(A, 1), (B, 2)]);
        let leaf1 = child(&leaf0, &s1);
        let change = writer
            .decided(&leaf1, Some(&entry(&leaf1, &s1, Some(&[A, B]))))
            .state
            .expect("the state of a validated leaf");
        let config: ChainConfig = bincode::deserialize(&change.chain_config).unwrap();
        assert_eq!(
            config.commit(),
            leaf1.block_header().chain_config().commit()
        );
        let frontier: BlocksFrontier = bincode::deserialize(&change.frontier).unwrap();
        let last = s1.block_merkle_tree.num_leaves() - 1;
        assert_eq!(
            frontier,
            s1.block_merkle_tree.lookup(last).expect_ok().unwrap().1
        );
        let rewards: Vec<(RewardAccountV2, RewardAmount)> =
            bincode::deserialize(&change.rewards.expect("changed rewards")).unwrap();
        assert_eq!(rewards, balances(&s1));

        let s2 = next(&s1, &leaf1, &[]);
        let leaf2 = child(&leaf1, &s2);
        let change = writer
            .decided(&leaf2, Some(&entry(&leaf2, &s2, Some(&[]))))
            .state
            .expect("the state of a validated leaf");
        assert_eq!(change.rewards, None, "the parent's rewards");

        let s3 = next(&s2, &leaf2, &[(A, 7)]);
        let leaf3 = child(&leaf2, &s3);
        assert_eq!(writer.decided(&leaf3, None).state, None);

        // After a leaf without its state, the rewards are sent in full again.
        let s4 = next(&s3, &leaf3, &[]);
        let leaf4 = child(&leaf3, &s4);
        let change = writer
            .decided(&leaf4, Some(&entry(&leaf4, &s4, Some(&[]))))
            .state
            .expect("the state of a validated leaf");
        let rewards: Vec<(RewardAccountV2, RewardAmount)> =
            bincode::deserialize(&change.rewards.expect("rewards in full")).unwrap();
        assert_eq!(rewards, balances(&s4));
    }

    /// Parts stored for another leaf are left out of the state of an anchor they do not match.
    #[tokio::test]
    async fn leaves_out_what_does_not_match_the_anchor() {
        let (_dir, store) = store().await;
        let (leaf0, s0) = genesis().await;
        let mut writer = DecidedStateWriter::new(&store, &leaf0).unwrap();
        let s1 = next(&s0, &leaf0, &[(A, 1)]);
        let leaf1 = child(&leaf0, &s1);
        store
            .app_write(
                &writer
                    .decided(&leaf1, Some(&entry(&leaf1, &s1, None)))
                    .writes,
            )
            .await
            .unwrap();

        let s2 = next(&s1, &leaf1, &[(A, 2)]);
        let leaf2 = child(&leaf1, &s2);
        let restored = load(&store, &leaf2).unwrap();
        let tree = &restored.block_merkle_tree;
        assert!(tree.lookup(tree.num_leaves() - 1).expect_ok().is_err());
        assert!(
            restored
                .reward_merkle_tree_v2
                .lookup(A)
                .expect_ok()
                .is_err()
        );
    }

    async fn store() -> (tempfile::TempDir, NodeStore) {
        let dir = tempfile::tempdir().unwrap();
        let (store, _) = crate::store::open(dir.path().join("store")).await.unwrap();
        (dir, store)
    }

    async fn genesis() -> (Leaf2, ValidatedState) {
        let instance = NodeState::mock().with_current_version(NEW_PROTOCOL_VERSION);
        let state = ValidatedState {
            chain_config: instance.chain_config.into(),
            ..Default::default()
        };
        let leaf = Leaf2::genesis(&state, &instance, NEW_PROTOCOL_VERSION).await;
        (leaf, state)
    }

    /// The state after `parent`, with `balances` set.
    fn next(
        state: &ValidatedState,
        parent: &Leaf2,
        balances: &[(RewardAccountV2, u64)],
    ) -> ValidatedState {
        let mut state = state.clone();
        state
            .block_merkle_tree
            .push(parent.block_header().commit())
            .unwrap();
        for (account, balance) in balances {
            state
                .reward_merkle_tree_v2
                .update(*account, RewardAmount(U256::from(*balance)))
                .unwrap();
        }
        state
    }

    /// A child of `parent` whose header commits to `state`.
    fn child(parent: &Leaf2, state: &ValidatedState) -> Leaf2 {
        let mut leaf = parent.clone();
        let header = leaf.block_header_mut();
        *header.height_mut() += 1;
        *header.block_merkle_tree_root_mut() = state.block_merkle_tree.commitment();
        // The header's fields are private to espresso-types, so the root is set in its encoding.
        let mut encoded = serde_json::to_value(&*header).unwrap();
        let root = serde_json::to_value(state.reward_merkle_tree_v2.commitment()).unwrap();
        assert!(
            set_field(&mut encoded, "reward_merkle_tree_root", root),
            "expected a header with a v2 reward tree"
        );
        *header = serde_json::from_value(encoded).unwrap();
        leaf
    }

    /// Sets every field named `name` in `value`, returning whether there was one.
    fn set_field(value: &mut serde_json::Value, name: &str, to: serde_json::Value) -> bool {
        match value {
            serde_json::Value::Object(fields) => {
                let mut found = false;
                for (key, field) in fields.iter_mut() {
                    if key == name {
                        *field = to.clone();
                        found = true;
                    } else {
                        found |= set_field(field, name, to.clone());
                    }
                }
                found
            },
            serde_json::Value::Array(items) => items.iter_mut().fold(false, |found, item| {
                set_field(item, name, to.clone()) | found
            }),
            _ => false,
        }
    }

    fn entry(
        leaf: &Leaf2,
        state: &ValidatedState,
        changed: Option<&[RewardAccountV2]>,
    ) -> StateEntry<SeqTypes> {
        StateEntry {
            state: Arc::new(state.clone()),
            delta: changed.map(|accounts| {
                Arc::new(Delta {
                    fees_delta: HashSet::new(),
                    rewards_delta: accounts.iter().copied().collect(),
                })
            }),
            leaf: leaf.clone(),
        }
    }

    fn account_writes(writes: &[AppWrite]) -> usize {
        writes
            .iter()
            .filter(|w| matches!(w, AppWrite::Put { table, .. } if *table == *REWARD_ACCOUNTS))
            .count()
    }

    fn assert_restored(store: &NodeStore, anchor: &Leaf2, expected: &ValidatedState) {
        let restored = load(store, anchor).unwrap();
        let tree = &restored.block_merkle_tree;
        assert!(tree.lookup(tree.num_leaves() - 1).expect_ok().is_ok());
        assert!(restored.chain_config.resolve().is_some());
        let mut accounts: Vec<_> = restored
            .reward_merkle_tree_v2
            .iter()
            .map(|(a, b)| (*a, *b))
            .collect();
        let mut want: Vec<_> = expected
            .reward_merkle_tree_v2
            .iter()
            .map(|(a, b)| (*a, *b))
            .collect();
        accounts.sort();
        want.sort();
        assert_eq!(accounts, want);
    }
}
