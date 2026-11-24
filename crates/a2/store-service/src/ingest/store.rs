//! The [`ChainStore`] as the sink of the decide log.

use async_trait::async_trait;
use bytes::Bytes;
use chain_api::{SentState, Tables};
use chain_store::{ChainStore, Item, Parts};
use consensus_api::{discovery::SignedServiceRecord, proto::State};
use espresso_types::{Leaf2, SeqTypes};
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
            // The payload comes through the payload offers.
            payload: None,
            vid_common: block.vid_common,
            cert2: block.cert2,
            state_cert: block.state_cert,
            certs_known: true,
            // Computed, and the payload uploaded, when it is written.
            payload_info: None,
            payload_index: None,
            payload_stored: false,
        };
        self.put(&item, &parts).await
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
        // The info is written with the payload, so this need not read the payload.
        let Some(summary) = self.summary(height).await? else {
            return Ok(false);
        };
        Ok(summary.parts()?.payload_info.is_some())
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

/// Stores the state after `height` as consensus sent it, for the follower. A state that does
/// not decode is left out, and the follower applies the block instead.
async fn put_state(store: &ChainStore, height: u64, state: &State) -> anyhow::Result<()> {
    let decoded = (|| {
        let frontier = match state.frontier.is_empty() {
            true => None,
            false => Some(bincode::deserialize(&state.frontier)?),
        };
        anyhow::Ok(SentState {
            frontier,
            chain_config: bincode::deserialize(&state.chain_config)?,
            reward_changes: bincode::deserialize(&state.reward_changes)?,
        })
    })();
    match decoded {
        Ok(sent) => {
            Tables::new(store.clone())
                .put_sent_state(height, &sent)
                .await
        },
        Err(err) => {
            tracing::warn!(height, "ignoring a state that does not decode: {err:#}");
            Ok(())
        },
    }
}

#[cfg(test)]
mod tests {
    use chain_api::RewardSet;
    use chain_store::mem::MemBackend;
    use espresso_types::{NodeState, ValidatedState};
    use hotshot_types::simple_certificate::QuorumCertificate2;
    use versions::{NEW_PROTOCOL_VERSION, Upgrade};

    use super::*;

    /// A block reaches the store as an item, and ingesting resumes after it.
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
            state: None,
        };
        let store = ChainStore::new(MemBackend::default());
        assert_eq!(store.newest_leaf().await.unwrap(), None);

        Sink::append(&store, block.clone()).await.unwrap();
        assert_eq!(store.newest_leaf().await.unwrap(), Some(block.leaf.clone()));
        let item = store.item(0).await.unwrap().unwrap();
        assert_eq!(item.leaf, block.leaf);
        assert_eq!(item.cert1, block.cert1);
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
        let changes: RewardSet = vec![(Default::default(), 7u64.into())];
        let block = Block {
            leaf: Leaf2::genesis(&state, &instance, NEW_PROTOCOL_VERSION).await,
            cert1: QuorumCertificate2::genesis(&state, &instance, upgrade).await,
            cert2: None,
            vid_common: None,
            state_cert: None,
            state: Some(State {
                frontier: Bytes::new(),
                chain_config: bincode::serialize(&instance.chain_config).unwrap().into(),
                reward_changes: bincode::serialize(&changes).unwrap().into(),
            }),
        };
        let store = ChainStore::new(MemBackend::default());
        let tables = Tables::new(store.clone());

        Sink::append(&store, block).await.unwrap();
        let sent = tables
            .sent_state(0)
            .await
            .unwrap()
            .expect("the state of height 0");
        assert_eq!(
            sent,
            SentState {
                frontier: None,
                chain_config: instance.chain_config,
                reward_changes: changes,
            }
        );

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
