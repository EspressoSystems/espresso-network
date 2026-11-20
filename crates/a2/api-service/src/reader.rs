//! Reads from the store that wait for heights it does not have yet.

use std::{future::Future, sync::Arc, time::Duration};

use chain_store::{ChainStore, Item, Lookup, Parts};
use espresso_api::error::to_status;
use espresso_types::{NamespaceId, NamespaceProofQueryData, NsProof, Payload, SeqTypes};
use futures::stream::{self, BoxStream, StreamExt};
use hotshot_query_service_types::availability::{
    BlockQueryData, LeafQueryData, VidCommonQueryData,
};
use hotshot_types::traits::block_contents::{BlockHeader, BlockPayload};
use tokio::{
    task::spawn_blocking,
    time::{Instant, timeout_at},
};
use tonic::Status;

use crate::request::BlockKey;

/// How long a single read waits for a height the store lacks.
const READ_TIMEOUT: Duration = Duration::from_millis(500);

#[derive(Clone, Debug)]
pub(crate) struct Reader {
    store: ChainStore,
}

impl Reader {
    pub(crate) fn new(store: ChainStore) -> Self {
        Self { store }
    }

    pub(crate) fn store(&self) -> &ChainStore {
        &self.store
    }

    /// The height of the block `key` names. A height is taken as given; a commitment the store
    /// does not know is not found.
    pub(crate) async fn height(&self, key: &BlockKey) -> Result<u64, Status> {
        let lookup = match key {
            BlockKey::Height(height) => return Ok(*height),
            BlockKey::Block(commit) => Lookup::Block(*commit),
            BlockKey::Payload(commit) => Lookup::Payload(*commit),
            BlockKey::Leaf(commit) => Lookup::Leaf(*commit),
        };
        self.store
            .height_of(&lookup)
            .await
            .map_err(to_status)?
            .ok_or_else(|| Status::not_found(format!("no block with {key:?}")))
    }

    pub(crate) async fn item(&self, height: u64, wait: Wait) -> Result<Item, Status> {
        self.wait(
            height,
            wait,
            || async move { self.store.item(height).await },
        )
        .await
    }

    pub(crate) async fn leaf(
        &self,
        height: u64,
        wait: Wait,
    ) -> Result<LeafQueryData<SeqTypes>, Status> {
        leaf(self.item(height, wait).await?)
    }

    pub(crate) async fn block(
        &self,
        height: u64,
        wait: Wait,
    ) -> Result<BlockQueryData<SeqTypes>, Status> {
        let (item, parts) = self
            .with_parts(height, wait, |p| p.payload.is_some())
            .await?;
        Ok(block(
            &item,
            parts.payload.as_deref().expect("waited for the payload"),
        ))
    }

    pub(crate) async fn vid_common(
        &self,
        height: u64,
        wait: Wait,
    ) -> Result<VidCommonQueryData<SeqTypes>, Status> {
        let (item, parts) = self
            .with_parts(height, wait, |p| p.vid_common.is_some())
            .await?;
        let common = parts.vid_common.expect("waited for the VID common data");
        Ok(VidCommonQueryData::new(
            item.leaf.block_header().clone(),
            common,
        ))
    }

    /// The proof of `namespace` in the block at `height`, empty if the block has no such
    /// namespace.
    pub(crate) async fn namespace_proof(
        &self,
        height: u64,
        namespace: NamespaceId,
        wait: Wait,
    ) -> Result<NamespaceProofQueryData, Status> {
        let (item, parts) = self
            .with_parts(height, wait, |p| {
                p.payload.is_some() && p.vid_common.is_some()
            })
            .await?;
        let block = block(
            &item,
            parts.payload.as_deref().expect("waited for the payload"),
        );
        let common = parts.vid_common.expect("waited for the VID common data");
        let none = NamespaceProofQueryData {
            proof: None,
            transactions: Vec::new(),
        };
        let Some(index) = block.payload().ns_table().find_ns_id(&namespace) else {
            return Ok(none);
        };
        // Proving encodes the whole payload.
        let proof = spawn_blocking(move || NsProof::new(block.payload(), &index, &common))
            .await
            .map_err(|err| Status::internal(format!("proving the namespace failed: {err}")))?;
        let Some(proof) = proof else {
            return Ok(none);
        };
        Ok(NamespaceProofQueryData {
            transactions: proof.export_all_txs(&namespace),
            proof: Some(proof),
        })
    }

    /// The values `read` gives for the heights from `from` on, in order, each once the store has
    /// it. The stream ends after the first error, so a subscriber cannot miss a height unawares.
    pub(crate) fn stream<T, F, Fut>(
        &self,
        from: u64,
        read: F,
    ) -> BoxStream<'static, Result<T, Status>>
    where
        T: Send + 'static,
        F: Fn(Reader, u64) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<T, Status>> + Send + 'static,
    {
        let reader = self.clone();
        let read = Arc::new(read);
        stream::unfold(Some(from), move |height| {
            let (reader, read) = (reader.clone(), read.clone());
            async move {
                let height = height?;
                let value = read(reader, height).await;
                let next = value.is_ok().then_some(height + 1);
                Some((value, next))
            }
        })
        .boxed()
    }

    /// The item of `height` with its parts, read together, once the parts are as `complete`
    /// requires.
    async fn with_parts(
        &self,
        height: u64,
        wait: Wait,
        complete: impl Fn(&Parts) -> bool,
    ) -> Result<(Item, Parts), Status> {
        let complete = &complete;
        self.wait(height, wait, || async move {
            let Some(record) = self.store.record(height).await? else {
                return Ok(None);
            };
            let parts = record.parts()?;
            Ok(complete(&parts).then_some((record.item()?, parts)))
        })
        .await
    }

    /// The value `read` gives at `height`, waiting as `wait` says while it gives none.
    async fn wait<T, F, Fut>(&self, height: u64, wait: Wait, read: F) -> Result<T, Status>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = anyhow::Result<Option<T>>>,
    {
        let deadline = match wait {
            Wait::Briefly => Some(Instant::now() + READ_TIMEOUT),
            Wait::Forever => None,
        };
        let mut changes = self.store.changes();
        loop {
            // Marked seen before reading, so a write after the read is not missed.
            changes.borrow_and_update();
            if let Some(value) = read().await.map_err(to_status)? {
                return Ok(value);
            }
            if height < self.store.floor().await.map_err(to_status)? {
                return Err(Status::not_found(format!("height {height} is pruned")));
            }
            let changed = match deadline {
                Some(deadline) => timeout_at(deadline, changes.changed())
                    .await
                    .map_err(|_| Status::not_found(format!("height {height} is not available")))?,
                None => changes.changed().await,
            };
            changed.map_err(|_| Status::unavailable("the store is closed"))?;
        }
    }
}

/// How long a read waits for a height the store lacks.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Wait {
    /// A moment, for a single request.
    Briefly,

    /// Until the store has it, for a stream.
    Forever,
}

fn leaf(item: Item) -> Result<LeafQueryData<SeqTypes>, Status> {
    LeafQueryData::new(item.leaf, item.cert1)
        .map_err(|err| Status::internal(format!("stored certificate does not match: {err:#}")))
}

fn block(item: &Item, payload: &[u8]) -> BlockQueryData<SeqTypes> {
    let header = item.leaf.block_header().clone();
    let payload = Payload::from_bytes(payload, header.metadata());
    BlockQueryData::new(header, payload)
}
