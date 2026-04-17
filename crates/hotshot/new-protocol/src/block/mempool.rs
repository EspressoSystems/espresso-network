use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    sync::Arc,
};

use committable::{Commitment, Committable};
use hotshot::traits::{BlockPayload, ValidatedState as _};
use hotshot_types::{
    consensus::PayloadWithMetadata,
    data::{EpochNumber, VidCommitment, VidDisperse2, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    traits::{
        EncodeBytes,
        block_contents::{BuilderFee, Transaction},
        node_implementation::NodeType,
        signature_key::BuilderSignatureKey,
    },
};
use tokio::task::{AbortHandle, JoinSet};
use tracing::{error, warn};

use super::{
    BlockAndHeaderRequest, BlockBuilder, BlockBuilderConfig, BlockBuilderOutput, BlockError,
};
use crate::{
    helpers::upgrade_lock,
    message::{DedupManifest, TransactionMessage},
};

struct RetryEntry<T: NodeType> {
    tx: T::Transaction,
    valid_until: ViewNumber,
    size: u64,
}

pub struct MempoolBuilder<T: NodeType> {
    instance: Arc<T::InstanceState>,
    membership: EpochMembershipCoordinator<T>,
    retry_pending: HashMap<Commitment<T::Transaction>, RetryEntry<T>>,
    retry_total_bytes: u64,
    leader_buffer: HashMap<Commitment<T::Transaction>, T::Transaction>,
    leader_total_bytes: u64,
    dedup_set: HashSet<Commitment<T::Transaction>>,
    dedup_views: VecDeque<(ViewNumber, Vec<Commitment<T::Transaction>>)>,
    config: BlockBuilderConfig,
    current_view: ViewNumber,
    calculations: BTreeMap<ViewNumber, AbortHandle>,
    tasks: JoinSet<Result<BlockBuilderOutput<T>, BlockError>>,
}

impl<T: NodeType> MempoolBuilder<T> {
    pub fn new(
        instance: Arc<T::InstanceState>,
        membership: EpochMembershipCoordinator<T>,
        config: BlockBuilderConfig,
    ) -> Self {
        Self {
            instance,
            membership,
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

    pub fn request_block(&mut self, request: BlockAndHeaderRequest<T>) {
        let view = request.view;
        if self.calculations.contains_key(&view) {
            return;
        }
        let Ok(_version) = upgrade_lock::<T>().version(view) else {
            warn!(%view, "unsupported version");
            return;
        };
        let epoch = request.epoch;
        let buffer = std::mem::take(&mut self.leader_buffer);
        self.leader_total_bytes = 0;
        let instance = self.instance.clone();
        let membership = self.membership.clone();

        let handle = self.tasks.spawn(async move {
            let (hashes, txs): (Vec<_>, Vec<_>) = buffer.into_iter().unzip();
            let manifest = DedupManifest {
                view,
                epoch,
                hashes,
            };

            let validated_state =
                T::ValidatedState::from_header(&request.parent_proposal.block_header);
            let (payload, metadata) =
                T::BlockPayload::from_transactions(txs, &validated_state, &instance)
                    .await
                    .map_err(|e| BlockError::PayloadConstruction(e.to_string()))?;
            let payload: PayloadWithMetadata<T> = PayloadWithMetadata { payload, metadata };

            // Compute full VID disperse (commitment + shares in one pass).
            let (vid_disperse, _duration) = VidDisperse2::calculate_vid_disperse(
                &payload.payload,
                &membership,
                view,
                Some(epoch),
                Some(epoch),
                &payload.metadata,
            )
            .await
            .map_err(|_| BlockError::StakeTableUnavailable)?;

            let payload_commitment = VidCommitment::V2(vid_disperse.payload_commitment);
            let builder_commitment = payload.payload.builder_commitment(&payload.metadata);
            let payload_bytes = payload.payload.encode();
            let (builder_key, builder_private_key) =
                T::BuilderSignatureKey::generated_from_seed_indexed([0u8; 32], 0);
            let block_size = payload_bytes.len() as u64;
            let offered_fee = block_size;
            let builder_fee = BuilderFee {
                fee_amount: offered_fee,
                fee_account: builder_key,
                fee_signature: T::BuilderSignatureKey::sign_fee(
                    &builder_private_key,
                    offered_fee,
                    &payload.metadata,
                )
                .map_err(|_| BlockError::BuilderSignature)?,
            };
            Ok(BlockBuilderOutput {
                view,
                epoch,
                payload,
                parent_proposal: request.parent_proposal,
                builder_commitment,
                builder_fee,
                payload_commitment,
                vid_disperse,
                manifest,
            })
        });
        self.calculations.insert(view, handle);
    }

    pub async fn next(&mut self) -> Option<Result<BlockBuilderOutput<T>, BlockError>> {
        loop {
            match self.tasks.join_next().await {
                Some(Ok(result)) => return Some(result),
                Some(Err(err)) => {
                    if err.is_panic() {
                        error!(%err, "block builder task panicked");
                    }
                },
                None => return None,
            }
        }
    }

    pub fn gc(&mut self, view_number: ViewNumber) {
        let keep = self.calculations.split_off(&view_number);
        for handle in self.calculations.values() {
            handle.abort();
        }
        self.calculations = keep;
    }

    pub fn on_submit_transaction(&mut self, tx: T::Transaction) {
        let hash = tx.commit();

        if self.retry_pending.contains_key(&hash) {
            return;
        }

        let size = tx.minimum_block_size();
        if self.retry_total_bytes + size > self.config.max_retry_bytes {
            warn!("retry buffer full, rejecting {hash}");
            return;
        }

        let valid_until = self.current_view + self.config.ttl;

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

    pub fn on_transactions(&mut self, msg: TransactionMessage<T>) {
        for tx in msg.transactions {
            let hash = tx.commit();

            if self.dedup_set.contains(&hash) {
                continue;
            }

            if self.leader_buffer.contains_key(&hash) {
                continue;
            }

            let size = tx.minimum_block_size();
            if self.leader_total_bytes + size > self.config.max_leader_bytes {
                continue;
            }

            self.leader_total_bytes += size;
            self.leader_buffer.insert(hash, tx);
        }
    }

    pub fn on_dedup_manifest(&mut self, manifest: DedupManifest<T>) {
        let DedupManifest { view, hashes, .. } = manifest;

        for hash in &hashes {
            if let Some(tx) = self.leader_buffer.remove(hash) {
                self.leader_total_bytes -= tx.minimum_block_size();
            }
        }

        self.dedup_set.extend(hashes.iter().copied());
        self.dedup_views.push_back((view, hashes));

        let current = self.current_view.u64();
        let window = self.config.dedup_window_size;

        while let Some((view, hashes)) = self.dedup_views.pop_front() {
            if current.saturating_sub(view.u64()) <= window {
                self.dedup_views.push_front((view, hashes));
                break;
            }
            for hash in &hashes {
                self.dedup_set.remove(hash);
            }
        }
    }

    pub fn on_view_changed(
        &mut self,
        view: ViewNumber,
        _epoch: EpochNumber,
    ) -> Vec<T::Transaction> {
        self.current_view = view;

        let mut expired_bytes = 0u64;
        self.retry_pending.retain(|_, entry| {
            if view > entry.valid_until {
                expired_bytes += entry.size;
                false
            } else {
                true
            }
        });
        self.retry_total_bytes -= expired_bytes;

        self.retry_pending
            .values()
            .map(|entry| entry.tx.clone())
            .collect()
    }

    pub fn on_block_reconstructed(&mut self, tx_commitments: Vec<Commitment<T::Transaction>>) {
        for hash in tx_commitments {
            if let Some(entry) = self.retry_pending.remove(&hash) {
                self.retry_total_bytes = self.retry_total_bytes.saturating_sub(entry.size);
            }
        }
    }

    pub fn drain(
        &mut self,
        view: ViewNumber,
        epoch: EpochNumber,
    ) -> (Vec<T::Transaction>, DedupManifest<T>) {
        let (hashes, txs) = self.leader_buffer.drain().unzip();
        self.leader_total_bytes = 0;

        let manifest = DedupManifest {
            view,
            epoch,
            hashes,
        };

        (txs, manifest)
    }
}

impl<T: NodeType> BlockBuilder<T> for MempoolBuilder<T> {
    fn request_block(&mut self, request: BlockAndHeaderRequest<T>) {
        self.request_block(request);
    }

    async fn next(&mut self) -> Option<Result<BlockBuilderOutput<T>, BlockError>> {
        self.next().await
    }

    fn on_transactions(&mut self, msg: TransactionMessage<T>) {
        self.on_transactions(msg);
    }

    fn on_dedup_manifest(&mut self, manifest: DedupManifest<T>) {
        self.on_dedup_manifest(manifest);
    }

    fn on_view_changed(&mut self, view: ViewNumber, epoch: EpochNumber) -> Vec<T::Transaction> {
        self.on_view_changed(view, epoch)
    }

    fn on_block_reconstructed(&mut self, tx_commitments: Vec<Commitment<T::Transaction>>) {
        self.on_block_reconstructed(tx_commitments);
    }

    fn gc(&mut self, view: ViewNumber) {
        self.gc(view);
    }
}
