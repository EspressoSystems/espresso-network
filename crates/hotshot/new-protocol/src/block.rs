use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    sync::Arc,
};

use committable::{Commitment, Committable};
use hotshot::traits::BlockPayload;
use hotshot_types::{
    data::{
        EpochNumber, QuorumProposal2, VidCommitment, ViewNumber, vid_commitment,
        vid_disperse::vid_total_weight,
    },
    epoch_membership::EpochMembershipCoordinator,
    traits::{
        EncodeBytes,
        block_contents::{BuilderFee, Transaction},
        node_implementation::NodeType,
        signature_key::BuilderSignatureKey,
    },
    utils::BuilderCommitment,
};
use tokio::task::{AbortHandle, JoinSet};

use crate::{
    consensus::ConsensusInput,
    helpers::upgrade_lock,
    message::{DedupManifest, TransactionMessage},
    state::HeaderRequest,
};

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct BlockAndHeaderRequest<T: NodeType> {
    pub view: ViewNumber,
    pub epoch: EpochNumber,
    pub parent_proposal: QuorumProposal2<T>,
}

pub struct BlockBuilderOutput<T: NodeType> {
    pub view: ViewNumber,
    pub epoch: EpochNumber,
    pub payload: T::BlockPayload,
    pub metadata: <T::BlockPayload as BlockPayload<T>>::Metadata,
    pub parent_proposal: QuorumProposal2<T>,
    pub builder_commitment: BuilderCommitment,
    pub builder_fee: BuilderFee<T>,
    pub payload_commitment: VidCommitment,
    pub manifest: DedupManifest<T>,
}

pub struct BlockBuilderConfig {
    pub max_retry_bytes: usize,
    pub max_leader_bytes: usize,
    pub ttl: u64,
    pub num_forward_leaders: usize,
    pub dedup_window_size: usize,
}

impl Default for BlockBuilderConfig {
    fn default() -> Self {
        Self {
            max_retry_bytes: 100 * 1024 * 1024,
            max_leader_bytes: 2 * 1024 * 1024,
            ttl: 50,
            num_forward_leaders: 3,
            dedup_window_size: 10,
        }
    }
}

struct RetryEntry<T: NodeType> {
    tx: T::Transaction,
    valid_until: ViewNumber,
    size: u64,
}

pub struct BlockBuilder<T: NodeType> {
    instance: Arc<T::InstanceState>,
    retry_pending: HashMap<Commitment<T::Transaction>, RetryEntry<T>>,
    retry_total_bytes: u64,
    leader_buffer: HashMap<Commitment<T::Transaction>, T::Transaction>,
    leader_total_bytes: u64,
    dedup_set: HashSet<Commitment<T::Transaction>>,
    dedup_views: VecDeque<(ViewNumber, Vec<Commitment<T::Transaction>>)>,
    config: BlockBuilderConfig,
    current_view: ViewNumber,
    calculations: BTreeMap<ViewNumber, AbortHandle>,
    tasks: JoinSet<Result<BlockBuilderOutput<T>, ()>>,
}

impl<T: NodeType> BlockBuilder<T> {
    pub fn new(instance: Arc<T::InstanceState>, config: BlockBuilderConfig) -> Self {
        Self {
            instance,
            config,
            retry_pending: HashMap::new(),
            retry_total_bytes: 0,
            leader_buffer: HashMap::new(),
            leader_total_bytes: 0,
            dedup_set: HashSet::new(),
            dedup_views: VecDeque::new(),
            current_view: ViewNumber::genesis(),
            calculations: BTreeMap::new(),
            tasks: JoinSet::new(),
        }
    }

    pub fn request_block(
        &mut self,
        request: BlockAndHeaderRequest<T>,
        membership_coordinator: EpochMembershipCoordinator<T>,
    ) {
        let view = request.view;
        if self.calculations.contains_key(&view) {
            return;
        }
        let Ok(version) = upgrade_lock::<T>().version(view) else {
            tracing::warn!(%view, "unsupported version for block building");
            return;
        };
        let epoch = request.epoch;
        let (txs, manifest) = self.drain(view);
        let instance = self.instance.clone();
        let handle = self.tasks.spawn(async move {
            let (payload, metadata) =
                T::BlockPayload::from_transactions(txs, &T::ValidatedState::default(), &instance)
                    .await
                    .map_err(|_| ())?;

            let total_weight = {
                let target_mem = membership_coordinator
                    .stake_table_for_epoch(Some(epoch))
                    .await
                    .map_err(|_| ())?;
                vid_total_weight::<T>(&target_mem.stake_table().await, Some(epoch))
            };
            let payload_commitment =
                vid_commitment(&payload.encode(), &metadata.encode(), total_weight, version);

            let builder_commitment = payload.builder_commitment(&metadata);
            let (builder_key, builder_private_key) =
                T::BuilderSignatureKey::generated_from_seed_indexed([0u8; 32], 0);
            let builder_fee = BuilderFee {
                fee_amount: 0,
                fee_account: builder_key,
                fee_signature: T::BuilderSignatureKey::sign_builder_message(
                    &builder_private_key,
                    builder_commitment.as_ref(),
                )
                .map_err(|_| ())?,
            };
            Ok(BlockBuilderOutput {
                view,
                epoch,
                payload,
                metadata,
                parent_proposal: request.parent_proposal,
                builder_commitment,
                builder_fee,
                payload_commitment,
                manifest,
            })
        });
        self.calculations.insert(view, handle);
    }

    pub async fn next(&mut self) -> Option<Result<BlockBuilderOutput<T>, ()>> {
        loop {
            match self.tasks.join_next().await {
                Some(Ok(result)) => return Some(result),
                Some(Err(_)) => continue,
                None => return None,
            }
        }
    }

    pub fn gc(&mut self, view_number: ViewNumber) {
        let keep = self.calculations.split_off(&view_number);
        for handle in self.calculations.values_mut() {
            handle.abort();
        }
        self.calculations = keep;
    }

    pub fn on_submit_transaction(&mut self, tx: T::Transaction) {
        self.handle_submit(tx);
    }

    pub fn on_transactions(&mut self, msg: TransactionMessage<T>) {
        for tx in msg.transactions {
            self.handle_tx(tx);
        }
    }

    pub fn on_dedup_manifest(&mut self, manifest: DedupManifest<T>) {
        self.handle_dedup_manifest(manifest);
    }

    pub fn on_view_changed(
        &mut self,
        view: ViewNumber,
        _epoch: EpochNumber,
    ) -> Vec<T::Transaction> {
        self.current_view = view;

        let expired: Vec<_> = self
            .retry_pending
            .iter()
            .filter(|(_, entry)| view > entry.valid_until)
            .map(|(hash, _)| *hash)
            .collect();
        for hash in expired {
            if let Some(entry) = self.retry_pending.remove(&hash) {
                self.retry_total_bytes -= entry.size;
            }
        }

        self.retry_pending
            .values()
            .map(|entry| entry.tx.clone())
            .collect()
    }

    pub fn on_block_reconstructed(
        &mut self,
        _view: ViewNumber,
        payload: T::BlockPayload,
        metadata: <T::BlockPayload as BlockPayload<T>>::Metadata,
    ) {
        for hash in payload.transaction_commitments(&metadata) {
            if let Some(entry) = self.retry_pending.remove(&hash) {
                self.retry_total_bytes -= entry.size;
            }
        }
    }

    pub fn drain(&mut self, view: ViewNumber) -> (Vec<T::Transaction>, DedupManifest<T>) {
        let (hashes, txs): (Vec<_>, Vec<_>) = self.leader_buffer.drain().unzip();
        self.leader_total_bytes = 0;
        (txs, DedupManifest { view, hashes })
    }

    fn handle_submit(&mut self, tx: T::Transaction) {
        let hash = tx.commit();

        if self.retry_pending.contains_key(&hash) {
            return;
        }

        let size = tx.minimum_block_size();
        if self.retry_total_bytes + size > self.config.max_retry_bytes as u64 {
            tracing::warn!("Retry buffer full, rejecting transaction {hash}");
            return;
        }

        let valid_until = ViewNumber::new(self.current_view.u64() + self.config.ttl);

        self.retry_total_bytes += size;
        self.retry_pending.insert(
            hash,
            RetryEntry {
                tx,
                valid_until,
                size,
            },
        );
    }

    fn handle_tx(&mut self, tx: T::Transaction) -> bool {
        let hash = tx.commit();

        if self.dedup_set.contains(&hash) {
            return false;
        }

        if self.leader_buffer.contains_key(&hash) {
            return false;
        }

        let size = tx.minimum_block_size();
        if self.leader_total_bytes + size > self.config.max_leader_bytes as u64 {
            return false;
        }

        self.leader_total_bytes += size;
        self.leader_buffer.insert(hash, tx);
        true
    }

    fn handle_dedup_manifest(&mut self, manifest: DedupManifest<T>) {
        let hashes = manifest.hashes.clone();
        self.dedup_set.extend(hashes.iter().copied());
        self.dedup_views.push_back((manifest.view, hashes));

        while let Some((oldest_view, _)) = self.dedup_views.front() {
            if self.current_view.u64().saturating_sub(oldest_view.u64())
                > self.config.dedup_window_size as u64
            {
                if let Some((_, old_hashes)) = self.dedup_views.pop_front() {
                    for hash in &old_hashes {
                        self.dedup_set.remove(hash);
                    }
                }
            } else {
                break;
            }
        }

        for hash in &manifest.hashes {
            if let Some(tx) = self.leader_buffer.remove(hash) {
                self.leader_total_bytes -= tx.minimum_block_size();
            }
        }
    }
}

impl<T: NodeType> From<&BlockBuilderOutput<T>> for HeaderRequest<T> {
    fn from(output: &BlockBuilderOutput<T>) -> Self {
        HeaderRequest {
            view: output.view,
            epoch: output.epoch,
            parent_proposal: output.parent_proposal.clone(),
            payload_commitment: output.payload_commitment,
            builder_commitment: output.builder_commitment.clone(),
            metadata: output.metadata.clone(),
            builder_fee: output.builder_fee.clone(),
        }
    }
}

impl<T: NodeType> From<BlockBuilderOutput<T>> for ConsensusInput<T> {
    fn from(output: BlockBuilderOutput<T>) -> Self {
        ConsensusInput::BlockBuilt {
            view: output.view,
            epoch: output.epoch,
            payload: output.payload,
            metadata: output.metadata,
        }
    }
}

#[cfg(test)]
mod tests {
    use committable::Committable;
    use hotshot_example_types::{
        block_types::{TestBlockPayload, TestMetadata, TestTransaction},
        node_types::TestTypes,
        state_types::TestInstanceState,
    };
    use hotshot_types::data::{EpochNumber, ViewNumber};

    use super::*;

    fn tx(n: u8) -> TestTransaction {
        TestTransaction::new(vec![n])
    }

    fn view(n: u64) -> ViewNumber {
        ViewNumber::new(n)
    }

    fn tx_msg(v: ViewNumber, transactions: Vec<TestTransaction>) -> TransactionMessage<TestTypes> {
        TransactionMessage {
            view: v,
            transactions,
        }
    }

    fn epoch() -> EpochNumber {
        EpochNumber::genesis()
    }

    fn small_config() -> BlockBuilderConfig {
        BlockBuilderConfig {
            max_retry_bytes: 1024,
            max_leader_bytes: 512,
            ttl: 5,
            num_forward_leaders: 3,
            dedup_window_size: 3,
        }
    }

    fn builder() -> BlockBuilder<TestTypes> {
        BlockBuilder::new(Arc::new(TestInstanceState::default()), small_config())
    }

    #[test]
    fn test_retry_buffer() {
        let mut b = builder();
        let t1 = tx(1);
        let t2 = tx(2);
        b.on_submit_transaction(t1.clone());
        b.on_submit_transaction(t2.clone());

        // t1 reconstructed and should be removed from retry
        b.on_block_reconstructed(
            view(1),
            TestBlockPayload {
                transactions: vec![t1],
            },
            TestMetadata {
                num_transactions: 1,
            },
        );

        let forwarded = b.on_view_changed(view(1), epoch());
        assert_eq!(
            forwarded,
            vec![t2],
            "only unconfirmed tx should be forwarded"
        );

        // past ttl
        let forwarded = b.on_view_changed(view(6), epoch());
        assert!(forwarded.is_empty(), "tx past ttl should expire");
    }

    #[test]
    fn test_leader_buffer_drain() {
        let mut b = builder();
        b.on_transactions(tx_msg(view(1), vec![tx(1), tx(2)]));
        let (mut txns, manifest) = b.drain(view(1));
        txns.sort_by_key(|t| t.bytes().clone());
        assert_eq!(txns.len(), 2, "both transactions should be drained");
        assert_eq!(manifest.hashes.len(), 2, "manifest should have one hash per tx");

        // buffer is cleared after drain
        let (txns2, manifest2) = b.drain(view(2));
        assert!(txns2.is_empty(), "second drain should be empty");
        assert!(manifest2.hashes.is_empty(), "second drain manifest should have no hashes");
    }

    #[test]
    fn test_dedup_window() {
        let mut b: BlockBuilder<TestTypes> = BlockBuilder::new(
            Arc::new(TestInstanceState::default()),
            BlockBuilderConfig {
                dedup_window_size: 2,
                ..small_config()
            },
        );
        let t = tx(1);

        b.on_dedup_manifest(DedupManifest {
            view: view(1),
            hashes: vec![t.commit()],
        });
        b.on_transactions(tx_msg(view(1), vec![t.clone()]));
        let (txns, _) = b.drain(view(1));
        assert!(
            txns.is_empty(),
            "tx should be blocked while in the dedup window"
        );

        // Advance past the threshold: current_view - view(1) > window_size(2)
        b.on_view_changed(view(4), epoch());
        b.on_dedup_manifest(DedupManifest {
            view: view(4),
            hashes: vec![],
        });

        b.on_transactions(tx_msg(view(4), vec![t.clone()]));
        let (txns, _) = b.drain(view(4));
        assert_eq!(
            txns.len(),
            1,
            "tx should be accepted after dedup window eviction"
        );
    }
}
