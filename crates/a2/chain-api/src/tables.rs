//! The tables of the chain store that the API instances serve besides the chain.
//!
//! The storage instances write them from what consensus sends with the decide log, and repair
//! writes what it fills; API instances only read them, and trust them as they trust the blocks. Epochs and heights are keys as big-endian `u64`, so they sort in
//! order and older ones are deleted by a key range.

use std::sync::LazyLock;

use anyhow::Context;
use bytes::Bytes;
use chain_store::{AppWrite, ChainStore, TableName};
use committable::Committable;
use consensus_api::discovery::SignedServiceRecord;
use espresso_types::{
    BlockMerkleTree, ChainConfig, FeeAccount, FeeAmount, SeqTypes, v0_3::RewardAmount,
    v0_4::RewardAccountV2,
};
use hotshot_types::{PeerConfig, data::EpochNumber};
use jf_merkle_tree_compat::MerkleTreeScheme;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// Height to the [`StateRow`] of the state after it, for the newest heights.
static STATES: LazyLock<TableName> = LazyLock::new(|| TableName::of("catchup-states"));

/// Height to the reward balances of the state after it, written only where they changed.
static REWARDS: LazyLock<TableName> = LazyLock::new(|| TableName::of("catchup-rewards"));

/// Height to the [`SentState`] consensus sent for it, until the follower makes a [`StateRow`] of
/// it.
static SENT_STATES: LazyLock<TableName> = LazyLock::new(|| TableName::of("catchup-sent-states"));

/// The commitment of a chain config, as a string, to the chain config.
static CHAIN_CONFIGS: LazyLock<TableName> = LazyLock::new(|| TableName::of("chain-configs"));

/// The accounts of the fee tree, which does not change under the new protocol.
static FEES: LazyLock<TableName> = LazyLock::new(|| TableName::of("fee-accounts"));
const FEES_KEY: &[u8] = b"genesis";

/// Epoch to its stake table.
static STAKE_TABLES: LazyLock<TableName> = LazyLock::new(|| TableName::of("stake-tables"));

/// Epoch to the valid discovery records of the epoch, as the consensus node received them.
static SERVICE_RECORDS: LazyLock<TableName> = LazyLock::new(|| TableName::of("service-records"));

/// How many of the newest heights the state is kept for.
pub const KEPT_STATES: u64 = 256;

pub type BlocksFrontier = <BlockMerkleTree as MerkleTreeScheme>::MembershipProof;

/// Reward balances by account: every balance of a state, or those a block changed.
pub type RewardSet = Vec<(RewardAccountV2, RewardAmount)>;

/// What a state holds besides the roots its block's header commits to, and besides the fee tree,
/// which does not change.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateRow {
    /// The frontier of the block tree, unless the tree is empty.
    pub frontier: Option<BlocksFrontier>,

    pub chain_config: ChainConfig,

    /// The height under which [`Tables::rewards`] finds the reward balances of the state: the
    /// first height whose state had them.
    pub rewards_at: u64,
}

/// The state after a block as consensus sent it: what a [`StateRow`] holds, but with the reward
/// balances the block changed instead of all of them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SentState {
    pub frontier: Option<BlocksFrontier>,
    pub chain_config: ChainConfig,
    pub reward_changes: RewardSet,
}

/// The tables of a chain store that the API instances serve besides the chain.
#[derive(Clone, Debug)]
pub struct Tables {
    store: ChainStore,
}

impl Tables {
    pub fn new(store: ChainStore) -> Self {
        Self { store }
    }

    /// What the state after `height` holds besides its header's roots, if it is among the newest
    /// heights kept.
    pub async fn state(&self, height: u64) -> anyhow::Result<Option<StateRow>> {
        self.get(&STATES, &height.to_be_bytes()).await
    }

    /// Every kept [`StateRow`], with its height, in height order.
    pub async fn states(&self) -> anyhow::Result<Vec<(u64, StateRow)>> {
        self.store
            .app_entries(&STATES, &[], None)
            .await?
            .into_iter()
            .map(|(key, value)| Ok((u64_of(&key)?, decode(&value)?)))
            .collect()
    }

    /// The reward balances first written for the state after `at`.
    pub async fn rewards(&self, at: u64) -> anyhow::Result<Option<RewardSet>> {
        self.get(&REWARDS, &at.to_be_bytes()).await
    }

    /// The state consensus sent for `height`, if the follower has not passed it yet.
    pub async fn sent_state(&self, height: u64) -> anyhow::Result<Option<SentState>> {
        self.get(&SENT_STATES, &height.to_be_bytes()).await
    }

    /// The chain config whose commitment, as a string, is `commitment`.
    pub async fn chain_config(&self, commitment: &str) -> anyhow::Result<Option<ChainConfig>> {
        self.get(&CHAIN_CONFIGS, commitment.as_bytes()).await
    }

    /// The accounts of the fee tree.
    pub async fn fee_accounts(&self) -> anyhow::Result<Option<Vec<(FeeAccount, FeeAmount)>>> {
        self.get(&FEES, FEES_KEY).await
    }

    /// The stake table of `epoch`.
    pub async fn stake_table(
        &self,
        epoch: EpochNumber,
    ) -> anyhow::Result<Option<Vec<PeerConfig<SeqTypes>>>> {
        self.get(&STAKE_TABLES, &epoch.to_be_bytes()).await
    }

    /// The discovery records of `epoch`, or of the latest epoch with records if `epoch` is
    /// `None`, with the epoch they are for.
    pub async fn service_records(
        &self,
        epoch: Option<EpochNumber>,
    ) -> anyhow::Result<Option<(EpochNumber, Vec<SignedServiceRecord<SeqTypes>>)>> {
        let entry = match epoch {
            Some(epoch) => self
                .store
                .app_get(&SERVICE_RECORDS, &epoch.to_be_bytes())
                .await?
                .map(|value| (Bytes::copy_from_slice(&epoch.to_be_bytes()), value)),
            None => self
                .store
                .app_entries(&SERVICE_RECORDS, &[], None)
                .await?
                .pop(),
        };
        let Some((key, value)) = entry else {
            return Ok(None);
        };
        Ok(Some((EpochNumber::new(u64_of(&key)?), decode(&value)?)))
    }

    /// Stores what the state after `height` holds besides its header's roots: `frontier`,
    /// `chain_config`, and `rewards`, every reward balance, or `None` if they are those of the
    /// state after `height - 1`. Forgets the sent states up to `height`, which the follower has
    /// passed, the rows of heights no longer among the newest [`KEPT_STATES`], and the reward
    /// balances no kept row refers to.
    ///
    /// Returns whether it stored the row: without `rewards` it needs the row of `height - 1`.
    pub async fn put_state(
        &self,
        height: u64,
        frontier: Option<BlocksFrontier>,
        chain_config: ChainConfig,
        rewards: Option<&RewardSet>,
    ) -> anyhow::Result<bool> {
        let rewards_at = match rewards {
            Some(_) => height,
            None => {
                let parent = match height.checked_sub(1) {
                    Some(parent) => self.state(parent).await?,
                    None => None,
                };
                let Some(parent) = parent else {
                    return Ok(false);
                };
                parent.rewards_at
            },
        };
        let keep_from = (height + 1).saturating_sub(KEPT_STATES);
        // Reward balances before those of the oldest kept row are no longer referred to. Without
        // that row, they are kept until a later write finds it.
        let keep_rewards_from = match self.state(keep_from).await? {
            Some(oldest) => oldest.rewards_at.min(rewards_at),
            None => 0,
        };
        let row = StateRow {
            frontier,
            chain_config,
            rewards_at,
        };
        let mut writes = vec![
            put(&STATES, &height.to_be_bytes(), &row)?,
            put(
                &CHAIN_CONFIGS,
                chain_config.commit().to_string().as_bytes(),
                &chain_config,
            )?,
        ];
        if let Some(rewards) = rewards {
            writes.push(put(&REWARDS, &height.to_be_bytes(), rewards)?);
        }
        writes.extend([
            delete_before(&SENT_STATES, height + 1),
            delete_before(&STATES, keep_from),
            delete_before(&REWARDS, keep_rewards_from),
        ]);
        self.store.app_write(writes).await?;
        Ok(true)
    }

    /// Stores the state consensus sent for `height`, for the follower, and forgets those of
    /// heights no longer among the newest [`KEPT_STATES`].
    pub async fn put_sent_state(&self, height: u64, state: &SentState) -> anyhow::Result<()> {
        let keep_from = (height + 1).saturating_sub(KEPT_STATES);
        self.store
            .app_write(vec![
                put(&SENT_STATES, &height.to_be_bytes(), state)?,
                delete_before(&SENT_STATES, keep_from),
            ])
            .await
    }

    pub async fn put_chain_config(&self, config: &ChainConfig) -> anyhow::Result<()> {
        let key = config.commit().to_string();
        self.store
            .app_write(vec![put(&CHAIN_CONFIGS, key.as_bytes(), config)?])
            .await
    }

    pub async fn put_fee_accounts(
        &self,
        accounts: &[(FeeAccount, FeeAmount)],
    ) -> anyhow::Result<()> {
        self.store
            .app_write(vec![put(&FEES, FEES_KEY, &accounts)?])
            .await
    }

    pub async fn put_stake_table(
        &self,
        epoch: EpochNumber,
        table: &[PeerConfig<SeqTypes>],
    ) -> anyhow::Result<()> {
        self.store
            .app_write(vec![put(&STAKE_TABLES, &epoch.to_be_bytes(), &table)?])
            .await
    }

    /// Stores the records of `epoch` and forgets those of the epochs before `keep_from`.
    pub async fn put_service_records(
        &self,
        epoch: EpochNumber,
        records: &[SignedServiceRecord<SeqTypes>],
        keep_from: EpochNumber,
    ) -> anyhow::Result<()> {
        self.store
            .app_write(vec![
                put(&SERVICE_RECORDS, &epoch.to_be_bytes(), &records)?,
                delete_before(&SERVICE_RECORDS, *keep_from),
            ])
            .await
    }

    async fn get<V: DeserializeOwned>(
        &self,
        table: &TableName,
        key: &[u8],
    ) -> anyhow::Result<Option<V>> {
        self.store
            .app_get(table, key)
            .await?
            .map(|value| decode(&value).with_context(|| format!("decoding a row of {table}")))
            .transpose()
    }
}

fn put<V: Serialize + ?Sized>(
    table: &TableName,
    key: &[u8],
    value: &V,
) -> anyhow::Result<AppWrite> {
    Ok(AppWrite::Put {
        table: table.clone(),
        key: Bytes::copy_from_slice(key),
        value: bincode::serialize(value)?.into(),
    })
}

fn delete_before(table: &TableName, key: u64) -> AppWrite {
    AppWrite::DeleteBefore {
        table: table.clone(),
        key: Bytes::copy_from_slice(&key.to_be_bytes()),
    }
}

fn decode<V: DeserializeOwned>(bytes: &[u8]) -> anyhow::Result<V> {
    Ok(bincode::deserialize(bytes)?)
}

fn u64_of(key: &[u8]) -> anyhow::Result<u64> {
    Ok(u64::from_be_bytes(
        key.try_into().context("a key that is not a u64")?,
    ))
}

#[cfg(test)]
mod tests {
    use chain_store::mem::MemBackend;
    use espresso_types::v0_4::RewardAccountV2;

    use super::*;

    fn rewards(n: u8) -> RewardSet {
        vec![(RewardAccountV2::default(), RewardAmount::from(u64::from(n)))]
    }

    /// A row without balances takes its parent's, a row whose parent is missing is not stored,
    /// and rows and balances no kept row refers to are forgotten.
    #[tokio::test]
    async fn keeps_the_newest_states_and_the_balances_they_refer_to() {
        let tables = Tables::new(ChainStore::new(MemBackend::default()));
        let config = ChainConfig::default();

        assert!(
            tables
                .put_state(0, None, config, Some(&rewards(0)))
                .await
                .unwrap()
        );
        for height in 1..KEPT_STATES {
            assert!(tables.put_state(height, None, config, None).await.unwrap());
        }
        let row = tables.state(KEPT_STATES - 1).await.unwrap().unwrap();
        assert_eq!(row.rewards_at, 0, "the balances of height 0");
        assert!(
            !tables
                .put_state(KEPT_STATES + 5, None, config, None)
                .await
                .unwrap(),
            "no parent to take the balances from"
        );

        // The rewards change, and height 0 leaves the kept window, but rows still refer to its
        // balances.
        let at = KEPT_STATES;
        assert!(
            tables
                .put_state(at, None, config, Some(&rewards(1)))
                .await
                .unwrap()
        );
        assert_eq!(tables.state(0).await.unwrap(), None);
        assert_eq!(tables.rewards(0).await.unwrap(), Some(rewards(0)));

        // Once no kept row refers to them, they go.
        for height in at + 1..at + KEPT_STATES {
            assert!(tables.put_state(height, None, config, None).await.unwrap());
        }
        assert_eq!(tables.rewards(0).await.unwrap(), None);
        assert_eq!(tables.rewards(at).await.unwrap(), Some(rewards(1)));
        assert_eq!(tables.states().await.unwrap().len() as u64, KEPT_STATES);
        let commitment = config.commit().to_string();
        assert_eq!(
            tables.chain_config(&commitment).await.unwrap(),
            Some(config)
        );
    }

    /// A sent state stays until a row at or after its height is written, or until it leaves the
    /// kept window.
    #[tokio::test]
    async fn keeps_sent_states_until_the_follower_passes_them() {
        let tables = Tables::new(ChainStore::new(MemBackend::default()));
        let config = ChainConfig::default();
        let sent = SentState {
            frontier: None,
            chain_config: config,
            reward_changes: rewards(1),
        };
        for height in 1..=3 {
            tables.put_sent_state(height, &sent).await.unwrap();
        }
        assert!(
            tables
                .put_state(2, None, config, Some(&rewards(0)))
                .await
                .unwrap()
        );
        assert_eq!(tables.sent_state(1).await.unwrap(), None);
        assert_eq!(tables.sent_state(2).await.unwrap(), None);
        assert_eq!(tables.sent_state(3).await.unwrap(), Some(sent.clone()));

        tables.put_sent_state(3 + KEPT_STATES, &sent).await.unwrap();
        assert_eq!(tables.sent_state(3).await.unwrap(), None);
    }
}
