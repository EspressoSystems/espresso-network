//! The [`ChainStore`] as the sink of the decide log.

use async_trait::async_trait;
use bytes::Bytes;
use chain_api::{RewardSet, Tables};
use chain_store::{ChainStore, Item, Parts};
use consensus_api::{discovery::SignedServiceRecord, proto::State};
use espresso_types::{ChainConfig, Leaf2, SeqTypes};
use hotshot_types::{PeerConfig, data::EpochNumber};
use membership::RETAINED_EPOCHS;

use super::{Block, Sink};

#[async_trait]
impl Sink<SeqTypes> for ChainStore {
    async fn append(&self, block: Block<SeqTypes>) -> anyhow::Result<()> {
        // The state goes first, so that the follower, which wakes on the item, finds it there.
        if let Some(state) = &block.state {
            put_state(self, block.leaf.height(), state).await?;
        }
        let item = Item {
            leaf: block.leaf,
            cert1: block.cert1,
        };
        let parts = Parts {
            payload: block.payload.map(Into::into),
            vid_common: block.vid_common,
            cert2: block.cert2,
            state_cert: block.state_cert,
            certs_known: true,
        };
        self.put(&item, &parts).await
    }

    async fn add_payload(&self, height: u64, payload: Vec<u8>) -> anyhow::Result<()> {
        let parts = Parts {
            payload: Some(payload.into()),
            ..Default::default()
        };
        self.put_parts(height, &parts).await
    }

    async fn newest_leaf(&self) -> anyhow::Result<Option<Leaf2>> {
        let Some(height) = self.newest().await? else {
            return Ok(None);
        };
        Ok(self.item(height).await?.map(|item| item.leaf))
    }

    async fn leaf(&self, height: u64) -> anyhow::Result<Option<Leaf2>> {
        Ok(self.item(height).await?.map(|item| item.leaf))
    }

    async fn has_payload(&self, height: u64) -> anyhow::Result<bool> {
        Ok(self.payload(height).await?.is_some())
    }

    async fn put_stake_table(&self, epoch: u64, table: Bytes) -> anyhow::Result<()> {
        let table: Vec<PeerConfig<SeqTypes>> = match bincode::deserialize(&table) {
            Ok(table) => table,
            Err(err) => {
                tracing::warn!(epoch, "ignoring a stake table that does not decode: {err}");
                return Ok(());
            },
        };
        Tables::new(self.clone())
            .put_stake_table(EpochNumber::new(epoch), &table)
            .await
    }

    async fn put_service_records(&self, epoch: u64, records: Vec<Bytes>) -> anyhow::Result<()> {
        let records: Vec<SignedServiceRecord<SeqTypes>> = records
            .iter()
            .filter_map(|bytes| {
                bincode::deserialize(bytes)
                    .inspect_err(|err| {
                        tracing::warn!(epoch, "ignoring a record that does not decode: {err}")
                    })
                    .ok()
            })
            .collect();
        let keep_from = EpochNumber::new(epoch.saturating_sub(RETAINED_EPOCHS));
        Tables::new(self.clone())
            .put_service_records(EpochNumber::new(epoch), &records, keep_from)
            .await
    }
}

/// Stores the state after `height` as consensus sent it. A state that does not decode, or whose
/// rewards are its parent's while the store lacks the parent's state, is left to the follower.
async fn put_state(store: &ChainStore, height: u64, state: &State) -> anyhow::Result<()> {
    let decoded = (|| {
        let frontier = match state.frontier.is_empty() {
            true => None,
            false => Some(bincode::deserialize(&state.frontier)?),
        };
        let chain_config: ChainConfig = bincode::deserialize(&state.chain_config)?;
        let rewards: Option<RewardSet> = state
            .rewards
            .as_ref()
            .map(|rewards| bincode::deserialize(rewards))
            .transpose()?;
        anyhow::Ok((frontier, chain_config, rewards))
    })();
    let (frontier, chain_config, rewards) = match decoded {
        Ok(decoded) => decoded,
        Err(err) => {
            tracing::warn!(height, "ignoring a state that does not decode: {err:#}");
            return Ok(());
        },
    };
    let stored = Tables::new(store.clone())
        .put_state(height, frontier, chain_config, rewards.as_ref())
        .await?;
    if !stored {
        tracing::debug!(height, "no state of the parent to take the rewards from");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use chain_store::mem::MemBackend;
    use espresso_types::{NodeState, ValidatedState};
    use hotshot_types::simple_certificate::QuorumCertificate2;
    use versions::{NEW_PROTOCOL_VERSION, Upgrade};

    use super::*;

    /// A block reaches the store as an item and a payload, and ingesting resumes after it.
    #[tokio::test]
    async fn stores_the_block_and_resumes_after_it() {
        let instance = NodeState::mock().with_current_version(NEW_PROTOCOL_VERSION);
        let state = ValidatedState {
            chain_config: instance.chain_config.into(),
            ..Default::default()
        };
        let upgrade = Upgrade::trivial(NEW_PROTOCOL_VERSION);
        let block = Block {
            leaf: Leaf2::genesis(&state, &instance, NEW_PROTOCOL_VERSION).await,
            cert1: QuorumCertificate2::genesis(&state, &instance, upgrade).await,
            cert2: None,
            vid_common: None,
            state_cert: None,
            payload: Some(b"payload".to_vec()),
            state: None,
        };
        let store = ChainStore::new(MemBackend::default());
        assert_eq!(store.newest_leaf().await.unwrap(), None);

        Sink::append(&store, block.clone()).await.unwrap();
        assert_eq!(store.newest_leaf().await.unwrap(), Some(block.leaf.clone()));
        let item = store.item(0).await.unwrap().unwrap();
        assert_eq!(item.leaf, block.leaf);
        assert_eq!(item.cert1, block.cert1);
        assert_eq!(
            store.payload(0).await.unwrap().as_deref(),
            Some(&b"payload"[..])
        );
    }

    /// The state consensus sent with a block, and the stake tables and records of the stream,
    /// reach the store's tables; a record that does not decode is left out.
    #[tokio::test]
    async fn stores_the_state_stake_tables_and_records() {
        let instance = NodeState::mock().with_current_version(NEW_PROTOCOL_VERSION);
        let state = ValidatedState {
            chain_config: instance.chain_config.into(),
            ..Default::default()
        };
        let upgrade = Upgrade::trivial(NEW_PROTOCOL_VERSION);
        let rewards: RewardSet = Vec::new();
        let block = Block {
            leaf: Leaf2::genesis(&state, &instance, NEW_PROTOCOL_VERSION).await,
            cert1: QuorumCertificate2::genesis(&state, &instance, upgrade).await,
            cert2: None,
            vid_common: None,
            state_cert: None,
            payload: None,
            state: Some(State {
                frontier: Bytes::new(),
                chain_config: bincode::serialize(&instance.chain_config).unwrap().into(),
                rewards: Some(bincode::serialize(&rewards).unwrap().into()),
            }),
        };
        let store = ChainStore::new(MemBackend::default());
        let tables = Tables::new(store.clone());

        Sink::append(&store, block).await.unwrap();
        let row = tables
            .state(0)
            .await
            .unwrap()
            .expect("the state of height 0");
        assert_eq!(row.chain_config, instance.chain_config);
        assert_eq!(row.frontier, None);
        assert_eq!(tables.rewards(0).await.unwrap(), Some(rewards));

        let table: Vec<PeerConfig<SeqTypes>> = Vec::new();
        Sink::put_stake_table(&store, 4, bincode::serialize(&table).unwrap().into())
            .await
            .unwrap();
        assert_eq!(
            tables.stake_table(EpochNumber::new(4)).await.unwrap(),
            Some(table)
        );
        Sink::put_service_records(&store, 4, vec![Bytes::from_static(b"not a record")])
            .await
            .unwrap();
        let (epoch, records) = tables.service_records(None).await.unwrap().unwrap();
        assert_eq!((*epoch, records.len()), (4, 0));
    }
}
