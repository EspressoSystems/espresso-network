use std::collections::{HashMap, HashSet, VecDeque};

use committable::{Commitment, Committable};
use hotshot::traits::BlockPayload;
use hotshot_types::{
    data::ViewNumber,
    traits::{block_contents::Transaction, node_implementation::NodeType},
};

use crate::{
    events::{Action, BlockEvent},
    helpers::Outbox,
    message::DedupManifest,
};

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

struct RetryEntry<TYPES: NodeType> {
    tx: TYPES::Transaction,
    valid_until: ViewNumber,
    size: u64,
}

pub(crate) struct BlockBuilder<TYPES: NodeType> {
    retry_pending: HashMap<Commitment<TYPES::Transaction>, RetryEntry<TYPES>>,
    retry_total_bytes: u64,
    leader_buffer: HashMap<Commitment<TYPES::Transaction>, TYPES::Transaction>,
    leader_total_bytes: u64,
    dedup_set: HashSet<Commitment<TYPES::Transaction>>,
    dedup_views: VecDeque<(ViewNumber, Vec<Commitment<TYPES::Transaction>>)>,
    config: BlockBuilderConfig,
    current_view: ViewNumber,
}

impl<TYPES: NodeType> BlockBuilder<TYPES> {
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

    pub fn apply(&mut self, event: BlockEvent<TYPES>, outbox: &mut Outbox<Action<TYPES>>) {
        match event {
            BlockEvent::SubmitTransaction(tx) => {
                self.handle_submit(tx);
            },
            BlockEvent::TransactionsReceived(txs, _view) => {
                self.handle_transactions_received(txs);
            },
            BlockEvent::DedupManifestReceived(manifest) => {
                self.handle_dedup_manifest(manifest);
            },
            BlockEvent::ViewChanged(view, _epoch) => {
                self.handle_view_changed(view, outbox);
            },
            BlockEvent::BlockReconstructed(_view, payload) => {
                self.handle_block_reconstructed(payload);
            },
        }
    }

    pub fn drain(&mut self) -> Vec<TYPES::Transaction> {
        let txs: Vec<_> = self.leader_buffer.drain().map(|(_, tx)| tx).collect();
        self.leader_total_bytes = 0;
        txs
    }

    fn handle_submit(&mut self, tx: TYPES::Transaction) {
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

    fn handle_view_changed(&mut self, new_view: ViewNumber, outbox: &mut Outbox<Action<TYPES>>) {
        self.current_view = new_view;

        let expired: Vec<_> = self
            .retry_pending
            .iter()
            .filter(|(_, entry)| new_view > entry.valid_until)
            .map(|(hash, _)| *hash)
            .collect();
        for hash in expired {
            if let Some(entry) = self.retry_pending.remove(&hash) {
                self.retry_total_bytes -= entry.size;
            }
        }

        let txs_to_forward: Vec<_> = self
            .retry_pending
            .values()
            .map(|entry| entry.tx.clone())
            .collect();

        if !txs_to_forward.is_empty() {
            outbox.push_back(Action::ForwardTransactions(txs_to_forward, new_view));
        }

    }

    fn handle_transactions_received(&mut self, txs: Vec<TYPES::Transaction>) {
        for tx in txs {
            self.leader_receive_tx(tx);
        }
    }

    fn leader_receive_tx(&mut self, tx: TYPES::Transaction) -> bool {
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

    fn handle_dedup_manifest(&mut self, manifest: DedupManifest<TYPES>) {
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

    fn handle_block_reconstructed(&mut self, payload: TYPES::BlockPayload) {
        let (_, metadata) = TYPES::BlockPayload::empty();
        for tx in payload.transactions(&metadata) {
            let hash = tx.commit();
            if let Some(entry) = self.retry_pending.remove(&hash) {
                self.retry_total_bytes -= entry.size;
            }
        }
    }
}
