use std::collections::{HashMap, HashSet, VecDeque};

use committable::{Commitment, Committable};
use hotshot::traits::BlockPayload;
use hotshot_types::{
    data::{EpochNumber, QuorumProposal2, ViewNumber},
    traits::{block_contents::Transaction, node_implementation::NodeType},
};

use crate::message::{DedupManifest, TransactionMessage};

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct BlockAndHeaderRequest<T: NodeType> {
    pub view: ViewNumber,
    pub epoch: EpochNumber,
    pub parent_proposal: QuorumProposal2<T>,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct BlockRequest<T: NodeType> {
    pub view: ViewNumber,
    pub parent_proposal: QuorumProposal2<T>,
    pub epoch: EpochNumber,
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
    retry_pending: HashMap<Commitment<T::Transaction>, RetryEntry<T>>,
    retry_total_bytes: u64,
    leader_buffer: HashMap<Commitment<T::Transaction>, T::Transaction>,
    leader_total_bytes: u64,
    dedup_set: HashSet<Commitment<T::Transaction>>,
    dedup_views: VecDeque<(ViewNumber, Vec<Commitment<T::Transaction>>)>,
    config: BlockBuilderConfig,
    current_view: ViewNumber,
}

impl<T: NodeType> Default for BlockBuilder<T> {
    fn default() -> Self {
        Self::new(BlockBuilderConfig::default())
    }
}

impl<T: NodeType> BlockBuilder<T> {
    pub fn new(config: BlockBuilderConfig) -> Self {
        Self {
            retry_pending: HashMap::new(),
            retry_total_bytes: 0,
            leader_buffer: HashMap::new(),
            leader_total_bytes: 0,
            dedup_set: HashSet::new(),
            dedup_views: VecDeque::new(),
            config,
            current_view: ViewNumber::genesis(),
        }
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

    /// Returns transactions and an optional dedup manifest to broadcast.
    pub fn drain(&mut self, view: ViewNumber) -> (Vec<T::Transaction>, Option<DedupManifest<T>>) {
        let (hashes, txs): (Vec<_>, Vec<_>) = self.leader_buffer.drain().unzip();
        self.leader_total_bytes = 0;

        let manifest = if !txs.is_empty() {
            Some(DedupManifest { view, hashes })
        } else {
            None
        };

        (txs, manifest)
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

#[cfg(test)]
mod tests {
    use committable::Committable;
    use hotshot_example_types::{
        block_types::{TestBlockPayload, TestMetadata, TestTransaction},
        node_types::TestTypes,
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
        BlockBuilder::new(small_config())
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
        assert!(
            manifest.is_some(),
            "non-empty drain should produce a dedup manifest"
        );
        let m = manifest.unwrap();
        assert_eq!(m.hashes.len(), 2, "manifest should have one hash per tx");

        // buffer is cleared after drain
        let (txns2, manifest2) = b.drain(view(2));
        assert!(txns2.is_empty(), "second drain should be empty");
        assert!(
            manifest2.is_none(),
            "second drain should produce no manifest"
        );
    }

    #[test]
    fn test_dedup_window() {
        let mut b: BlockBuilder<TestTypes> = BlockBuilder::new(BlockBuilderConfig {
            dedup_window_size: 2,
            ..small_config()
        });
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
