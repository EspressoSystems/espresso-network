//! Reads from the store that wait for heights it does not have yet.
//!
//! A read of a height the [`Tip`] covers takes it from there and waits on the tip; only an older
//! height is read from the store.

use std::{future::Future, sync::Arc, time::Duration};

use chain_store::{ChainStore, Item, Lookup, Parts};
use espresso_api::{error::to_status, proto};
use espresso_types::{NamespaceId, NsPayload, Payload, SeqTypes};
use futures::stream::{self, BoxStream, StreamExt};
use hotshot_query_service_types::availability::{
    BlockQueryData, LeafQueryData, VidCommonQueryData,
};
use hotshot_types::{
    data::VidCommon,
    traits::block_contents::{BlockHeader, BlockPayload},
    vid::avidm_gf2::AvidmGf2Scheme,
};
use tokio::{
    sync::watch,
    time::{Instant, timeout_at},
};
use tonic::Status;

use crate::{
    request::BlockKey,
    tip::{Entry, Tip},
};

/// How long a single read waits for a height the store lacks.
const READ_TIMEOUT: Duration = Duration::from_millis(500);

#[derive(Clone, Debug)]
pub(crate) struct Reader {
    store: ChainStore,
    tip: watch::Receiver<Arc<Tip>>,
}

impl Reader {
    pub(crate) fn new(store: ChainStore, tip: watch::Receiver<Arc<Tip>>) -> Self {
        Self { store, tip }
    }

    pub(crate) fn store(&self) -> &ChainStore {
        &self.store
    }

    /// The newest stored height, from the tip once the follower has read the store.
    pub(crate) async fn newest(&self) -> Result<Option<u64>, Status> {
        let tip = self.tip.borrow().clone();
        match tip.read {
            true => Ok(tip.newest),
            false => self.store.newest().await.map_err(to_status),
        }
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
        Ok(self.entry(height, wait, |_| true).await?.item.clone())
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
    ) -> Result<proto::NamespaceProofResponse, Status> {
        let entry = self
            .entry(height, wait, |p| {
                p.payload.is_some() && p.vid_common.is_some()
            })
            .await?;
        let payload = entry
            .parts
            .payload
            .as_deref()
            .expect("waited for the payload");
        let Some(VidCommon::V2(common)) = &entry.parts.vid_common else {
            return Err(Status::internal("the VID common data is not AvidmGf2's"));
        };
        let ns_table = entry.item.leaf.block_header().ns_table();
        let index = ns_table
            .iter()
            .position(|index| ns_table.read_ns_id_unchecked(&index) == namespace);
        let Some(index) = index.filter(|&index| common.ns_lens.get(index) > Some(&0)) else {
            return Ok(proto::NamespaceProofResponse::default());
        };
        // The proof holds a copy of the namespace's bytes, and the transactions a second one.
        let proof = AvidmGf2Scheme::namespace_proof(common, payload, index)
            .map_err(|err| Status::internal(format!("proving the namespace failed: {err}")))?;
        let transactions = NsPayload::from_bytes_slice(&proof.ns_payload)
            .export_all_txs(&namespace)
            .into_iter()
            .map(|tx| proto::Transaction {
                namespace: namespace.0,
                payload: tx.into_payload().into(),
            })
            .collect();
        let proof = proto::NsProofPayload {
            ns_index: proof.ns_index as u64,
            ns_proof: proof.ns_proof.to_string(),
            ns_payload: proof.ns_payload.into(),
        };
        Ok(proto::NamespaceProofResponse {
            proof: Some(proto::NsProof {
                proof: Some(proto::ns_proof::Proof::V2(proof)),
            }),
            transactions,
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
        let entry = self.entry(height, wait, complete).await?;
        Ok((entry.item.clone(), entry.parts.clone()))
    }

    /// The stored `height` once its parts are as `complete` requires, waiting as `wait` says
    /// while they are not.
    async fn entry(
        &self,
        height: u64,
        wait: Wait,
        complete: impl Fn(&Parts) -> bool,
    ) -> Result<Arc<Entry>, Status> {
        let deadline = match wait {
            Wait::Briefly => Some(Instant::now() + READ_TIMEOUT),
            Wait::Forever => None,
        };
        let mut tips = self.tip.clone();
        loop {
            // Marked seen before reading, so a newer tip after the read is not missed.
            let tip = tips.borrow_and_update().clone();
            if height < tip.floor {
                return Err(Status::not_found(format!("height {height} is pruned")));
            }
            match (tip.start(), tip.newest) {
                (Some(start), _) if height >= start => {
                    if let Some(entry) = tip.heights.get(&height)
                        && complete(&entry.parts)
                    {
                        return Ok(entry.clone());
                    }
                },
                // Older than the tip: read from the store, which can have filled a gap since.
                (_, Some(newest)) if height <= newest => {
                    if let Some(entry) = self.stored(height, &complete).await? {
                        return Ok(entry);
                    }
                },
                _ => {},
            }
            let changed = match deadline {
                Some(deadline) => timeout_at(deadline, tips.changed())
                    .await
                    .map_err(|_| Status::not_found(format!("height {height} is not available")))?,
                None => tips.changed().await,
            };
            changed.map_err(|_| Status::unavailable("the store is closed"))?;
        }
    }

    /// The stored `height` from the store itself, if its parts are as `complete` requires.
    async fn stored(
        &self,
        height: u64,
        complete: &impl Fn(&Parts) -> bool,
    ) -> Result<Option<Arc<Entry>>, Status> {
        let Some(record) = self.store.record(height).await.map_err(to_status)? else {
            return Ok(None);
        };
        let parts = record.parts().map_err(to_status)?;
        if !complete(&parts) {
            return Ok(None);
        }
        let item = record.item().map_err(to_status)?;
        Ok(Some(Arc::new(Entry { item, parts })))
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

#[cfg(test)]
mod tests {
    use chain_store::mem::MemBackend;
    use espresso_types::{
        Leaf2, NamespaceProofQueryData, NodeState, NsProof, Transaction, ValidatedState,
    };
    use hotshot_types::{
        data::ns_table::parse_ns_table, simple_certificate::QuorumCertificate2,
        traits::EncodeBytes, vid::avidm_gf2::AvidmGf2Param,
    };
    use tokio::task::JoinSet;
    use versions::{NEW_PROTOCOL_VERSION, Upgrade};

    use super::*;
    use crate::tip;

    /// The proof built from the stored bytes is the one the generic `NsProof` gives, for each
    /// namespace of a block with several, and empty for a namespace the block lacks.
    #[tokio::test]
    async fn namespace_proofs_match_the_generic_proof() {
        let instance = NodeState::mock().with_current_version(NEW_PROTOCOL_VERSION);
        let state = ValidatedState {
            chain_config: instance.chain_config.into(),
            ..Default::default()
        };
        let transactions: Vec<_> = [(7u32, 3), (9, 1), (11, 2)]
            .into_iter()
            .flat_map(|(namespace, count)| {
                (0..count).map(move |i| {
                    Transaction::new(NamespaceId::from(namespace), vec![i as u8; 10 + i])
                })
            })
            .collect();
        let (payload, ns_table) =
            Payload::from_transactions_sync(&transactions, instance.chain_config).unwrap();
        let bytes = payload.encode().to_vec();
        let ranges = parse_ns_table(bytes.len(), &ns_table.encode());
        let param = AvidmGf2Param::new(5, 10).unwrap();
        let (_, common) = AvidmGf2Scheme::commit(&param, &bytes, ranges).unwrap();
        let common = VidCommon::V2(common);
        let mut leaf = Leaf2::genesis(&state, &instance, NEW_PROTOCOL_VERSION).await;
        *leaf.block_header_mut().ns_table_mut() = ns_table.clone();
        let cert1 =
            QuorumCertificate2::genesis(&state, &instance, Upgrade::trivial(NEW_PROTOCOL_VERSION))
                .await;

        let store = ChainStore::new(MemBackend::default());
        let parts = Parts {
            payload: Some(bytes.into()),
            vid_common: Some(common.clone()),
            ..Default::default()
        };
        store.put(&Item { leaf, cert1 }, &parts).await.unwrap();
        let (follower, tips) = tip::follower(store.clone());
        let mut tasks = JoinSet::new();
        tasks.spawn(follower.run());
        let reader = Reader::new(store, tips);

        for namespace in [7u32, 9, 11, 13].map(NamespaceId::from) {
            let expected = match ns_table.find_ns_id(&namespace) {
                Some(index) => {
                    let proof = NsProof::new(&payload, &index, &common).unwrap();
                    let data = NamespaceProofQueryData {
                        transactions: proof.export_all_txs(&namespace),
                        proof: Some(proof),
                    };
                    proto::NamespaceProofResponse::try_from(&data).unwrap()
                },
                None => proto::NamespaceProofResponse::default(),
            };
            let proof = reader
                .namespace_proof(0, namespace, Wait::Forever)
                .await
                .unwrap();
            assert_eq!(proof, expected, "namespace {namespace:?}");
        }
    }
}
