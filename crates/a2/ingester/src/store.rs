//! The [`ChainStore`] as the ingester's sink.

use async_trait::async_trait;
use chain_store::{ChainStore, Item, Parts};
use espresso_types::{Leaf2, SeqTypes};

use crate::{Block, Sink};

#[async_trait]
impl Sink<SeqTypes> for ChainStore {
    async fn append(&self, block: Block<SeqTypes>) -> anyhow::Result<()> {
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
}

#[cfg(test)]
mod tests {
    use chain_store::mem::MemBackend;
    use espresso_types::{NodeState, ValidatedState};
    use hotshot_types::simple_certificate::QuorumCertificate2;
    use versions::{NEW_PROTOCOL_VERSION, Upgrade};

    use super::*;

    /// A block reaches the store as an item and a payload, and the ingester resumes after it.
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
}
