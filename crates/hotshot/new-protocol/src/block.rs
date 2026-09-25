use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    num::NonZeroUsize,
    panic::resume_unwind,
    sync::Arc,
    time::Duration,
};

use committable::{Commitment, Committable};
use hotshot::traits::{BlockPayload, ValidatedState as _};
use hotshot_types::{
    consensus::PayloadWithMetadata,
    data::{
        EpochNumber, Leaf2, VidCommitment, ViewNumber, vid_commitment,
        vid_disperse::vid_total_weight,
    },
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    traits::{
        EncodeBytes,
        block_contents::{BuilderFee, Transaction},
        node_implementation::NodeType,
        signature_key::BuilderSignatureKey,
    },
    utils::BuilderCommitment,
};
use tokio::{
    task::{AbortHandle, JoinSet, spawn_blocking},
    time::sleep,
};
use tracing::{error, warn};
use versions::Version;

use crate::{
    consensus::ConsensusInput,
    helpers::proposal_commitment,
    message::{DedupManifest, Proposal, TransactionMessage},
    state::HeaderRequest,
};

#[derive(Debug, thiserror::Error)]
pub enum BlockError {
    #[error("payload construction failed: {0}")]
    PayloadConstruction(String),

    #[error("stake table unavailable")]
    StakeTableUnavailable,

    #[error("builder signature failed")]
    BuilderSignature,

    #[error("block builder task cancelled")]
    Cancelled,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct BlockAndHeaderRequest<T: NodeType> {
    pub view: ViewNumber,
    pub epoch: EpochNumber,
    pub parent_proposal: Proposal<T>,
}

pub struct BlockBuilderOutput<T: NodeType> {
    pub view: ViewNumber,
    pub epoch: EpochNumber,
    pub payload: PayloadWithMetadata<T>,
    pub parent_proposal: Proposal<T>,
    pub builder_commitment: BuilderCommitment,
    pub builder_fee: BuilderFee<T>,
    pub payload_commitment: VidCommitment,
    pub manifest: DedupManifest<T>,
}

/// The message limit released nodes run with. No derived limit goes below it, so a chain with
/// smaller blocks (mainnet's 10mb, 10,000,000 bytes, included) keeps what every release accepts.
pub const MIN_MESSAGE_LIMIT: NonZeroUsize =
    NonZeroUsize::new(10 * 1024 * 1024).expect("10 MiB > 0");

/// Room above a full block for the envelope of the messages that carry one, such as a payload
/// response: version, sender key, enum tags, commitment and length prefixes.
const MESSAGE_HEADROOM: usize = 64 * 1024;

/// The message limit for a protocol version whose blocks are at most `max_block_size`: one block
/// plus its envelope, never below [`MIN_MESSAGE_LIMIT`].
pub fn message_limit(max_block_size: u64) -> NonZeroUsize {
    let block = usize::try_from(max_block_size).unwrap_or(usize::MAX);
    let limit = block.saturating_add(MESSAGE_HEADROOM);
    NonZeroUsize::new(limit).map_or(MIN_MESSAGE_LIMIT, |n| n.max(MIN_MESSAGE_LIMIT))
}

/// Room left in a forwarded message for everything but the transactions: version, sender key,
/// enum tags, view and length prefixes. Generous, since overshooting only shrinks the batch.
const FORWARD_ENVELOPE_BYTES: u64 = 4096;

pub struct BlockBuilderConfig {
    pub max_retry_bytes: u64,
    /// The chain's `max_block_size` per protocol version, from genesis; a version without an
    /// entry keeps the size of the version before it. The version running at a view bounds what
    /// a leader collects for its block, what a node forwards, and which submissions it accepts,
    /// so sizes change only once an upgrade has taken effect.
    pub block_sizes: BTreeMap<Version, u64>,
    pub ttl: u64,
    pub dedup_window_size: u64,
    pub empty_block_delay: Duration,
}

impl Default for BlockBuilderConfig {
    fn default() -> Self {
        Self {
            max_retry_bytes: 100 * 1024 * 1024,
            block_sizes: BTreeMap::from([(versions::version(0, 0), 2 * 1024 * 1024)]),
            ttl: 50,
            dedup_window_size: 10,
            empty_block_delay: Duration::from_millis(500),
        }
    }
}

/// Encoded bytes of transactions that fit in one message of `max_message_size`.
pub fn forward_budget(max_message_size: NonZeroUsize) -> u64 {
    (max_message_size.get() as u64).saturating_sub(FORWARD_ENVELOPE_BYTES)
}

struct RetryEntry<T: NodeType> {
    tx: T::Transaction,
    valid_until: ViewNumber,
    size: u64,
    /// Bytes on the wire, which can exceed `size`.
    encoded_size: u64,
}

pub struct BlockBuilder<T: NodeType> {
    instance: Arc<T::InstanceState>,
    membership: EpochMembershipCoordinator<T>,
    retry_pending: HashMap<Commitment<T::Transaction>, RetryEntry<T>>,
    retry_order: BTreeSet<(ViewNumber, Commitment<T::Transaction>)>,
    retry_total_bytes: u64,
    leader_buffer: HashMap<Commitment<T::Transaction>, T::Transaction>,
    leader_total_bytes: u64,
    dedups: BTreeMap<ViewNumber, HashSet<Commitment<T::Transaction>>>,
    config: BlockBuilderConfig,
    upgrade_lock: UpgradeLock<T>,
    current_view: ViewNumber,
    // Keyed by (view, parent_proposal commitment) so that two requests for
    // the same view but different parents (e.g. one from
    // `handle_proposal_with_vid_share` and one from
    // `handle_timeout_certificate`) don't dedup against each other.
    calculations: BTreeMap<(ViewNumber, Commitment<Leaf2<T>>), AbortHandle>,
    tasks: JoinSet<Result<BlockBuilderOutput<T>, BlockError>>,
}

impl<T: NodeType> BlockBuilder<T> {
    pub fn new(
        instance: Arc<T::InstanceState>,
        membership: EpochMembershipCoordinator<T>,
        config: BlockBuilderConfig,
        upgrade_lock: UpgradeLock<T>,
    ) -> Self {
        Self {
            instance,
            membership,
            config,
            upgrade_lock,
            retry_pending: HashMap::new(),
            retry_order: BTreeSet::new(),
            retry_total_bytes: 0,
            leader_buffer: HashMap::new(),
            leader_total_bytes: 0,
            dedups: BTreeMap::new(),
            current_view: ViewNumber::genesis(),
            calculations: BTreeMap::new(),
            tasks: JoinSet::new(),
        }
    }

    pub fn request_block(&mut self, request: BlockAndHeaderRequest<T>) {
        let view = request.view;
        let parent_commitment = proposal_commitment(&request.parent_proposal);
        if self.calculations.contains_key(&(view, parent_commitment)) {
            return;
        }
        let Ok(version) = self.upgrade_lock.version(view) else {
            warn!(%view, "unsupported version");
            return;
        };
        let epoch = request.epoch;
        let buffer = std::mem::take(&mut self.leader_buffer);
        self.leader_total_bytes = 0;
        let instance = self.instance.clone();
        let membership = self.membership.clone();

        let empty_block_delay = self.config.empty_block_delay;

        let handle = self.tasks.spawn(async move {
            // Without this an idle network produces empty blocks as fast as consensus can run
            // them, flooding the coordinator's event queue.
            if buffer.is_empty() {
                sleep(empty_block_delay).await;
            }
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

            let total_weight = {
                let target_mem = membership
                    .stake_table_for_epoch(Some(epoch))
                    .map_err(|_| BlockError::StakeTableUnavailable)?;
                vid_total_weight(target_mem.stake_table(), Some(epoch))
            };
            let commitments = spawn_blocking(move || {
                let payload_bytes = payload.payload.encode();
                let metadata_bytes = payload.metadata.encode();
                // The two commitments are independent, and neither can be split:
                // `vid_commitment` erasure-codes the payload (parallel over
                // namespaces internally) and `builder_commitment` is a serial
                // SHA-256 over every transaction. Running them sequentially made the
                // leader pay both in turn on the path that gates its proposal, so
                // the hash rides alongside the erasure code instead, occupying one
                // worker for its duration rather than adding its full wall time.
                let (payload_commitment, builder_commitment) = rayon::join(
                    || {
                        vid_commitment(
                            payload_bytes.as_ref(),
                            metadata_bytes.as_ref(),
                            total_weight,
                            version,
                        )
                    },
                    || payload.payload.builder_commitment(&payload.metadata),
                );
                let block_size = payload_bytes.len() as u64;
                (payload, block_size, payload_commitment, builder_commitment)
            });
            let (payload, block_size, payload_commitment, builder_commitment) =
                match commitments.await {
                    Ok(out) => out,
                    Err(e) if e.is_panic() => resume_unwind(e.into_panic()),
                    Err(_) => return Err(BlockError::Cancelled),
                };
            let (builder_key, builder_private_key) =
                T::BuilderSignatureKey::generated_from_seed_indexed([0u8; 32], 0);
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
                manifest,
            })
        });
        self.calculations.insert((view, parent_commitment), handle);
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
        self.calculations.retain(|(view, _), handle| {
            if *view < view_number {
                handle.abort();
                false
            } else {
                true
            }
        });
    }

    pub fn outstanding_transactions(&self) -> (usize, usize) {
        (self.retry_pending.len(), self.retry_total_bytes as usize)
    }

    pub fn on_submit_transaction(&mut self, tx: T::Transaction) {
        let hash = tx.commit();

        if self.retry_pending.contains_key(&hash) {
            return;
        }

        let size = tx.minimum_block_size();
        let encoded_size = bincode::serialized_size(&tx).expect("transactions serialize");
        let max_bytes = self.block_size(self.current_view);
        if size > max_bytes || encoded_size > forward_budget(message_limit(max_bytes)) {
            warn!(%hash, %size, "transaction can never be included, rejecting");
            return;
        }
        if self.retry_total_bytes + size > self.config.max_retry_bytes {
            warn!("retry buffer full, rejecting {hash}");
            return;
        }

        let valid_until = self.current_view + self.config.ttl;

        self.retry_total_bytes += size;
        self.retry_order.insert((valid_until, hash));
        self.retry_pending.insert(
            hash,
            RetryEntry {
                tx,
                valid_until,
                size,
                encoded_size,
            },
        );
    }

    pub fn on_transactions(&mut self, msg: TransactionMessage<T>) {
        let max_bytes = self.block_size(msg.view);
        for tx in msg.transactions {
            let hash = tx.commit();

            if self.dedups.values().any(|hs| hs.contains(&hash)) {
                continue;
            }

            if self.leader_buffer.contains_key(&hash) {
                continue;
            }

            let size = tx.minimum_block_size();
            if self.leader_total_bytes + size > max_bytes {
                continue;
            }

            self.leader_total_bytes += size;
            self.leader_buffer.insert(hash, tx);
        }
    }

    pub fn on_dedup_manifest(&mut self, manifest: DedupManifest<T>) {
        let DedupManifest { view, hashes, .. } = manifest;
        self.mark_included(view, hashes);
    }

    /// Returns pending transactions for the next leader, oldest first, within one block and one
    /// message.
    pub fn on_view_changed(&mut self, view: ViewNumber) -> Vec<T::Transaction> {
        self.current_view = view;
        while let Some(&(valid_until, hash)) = self.retry_order.first() {
            if valid_until >= view {
                break;
            }
            self.remove_pending(&hash);
        }

        let max_bytes = self.block_size(view + 1);
        let max_encoded = forward_budget(message_limit(max_bytes));
        let mut batch = Vec::new();
        let mut unfit = Vec::new();
        let (mut bytes, mut encoded) = (0u64, 0u64);
        for (_, hash) in &self.retry_order {
            let entry = &self.retry_pending[hash];
            if entry.size > max_bytes || entry.encoded_size > max_encoded {
                unfit.push(*hash);
                continue;
            }
            if bytes + entry.size > max_bytes || encoded + entry.encoded_size > max_encoded {
                continue;
            }
            bytes += entry.size;
            encoded += entry.encoded_size;
            batch.push(entry.tx.clone());
        }
        for hash in &unfit {
            warn!(%hash, "pending transaction no longer fits a block, dropping");
            self.remove_pending(hash);
        }
        batch
    }

    fn remove_pending(&mut self, hash: &Commitment<T::Transaction>) {
        if let Some(entry) = self.retry_pending.remove(hash) {
            self.retry_order.remove(&(entry.valid_until, *hash));
            self.retry_total_bytes -= entry.size;
        }
    }

    /// The block size of the protocol version running at `view`.
    fn block_size(&self, view: ViewNumber) -> u64 {
        let version = self.upgrade_lock.version_infallible(view);
        *self
            .config
            .block_sizes
            .range(..=version)
            .next_back()
            .expect("block sizes start at or below the running version")
            .1
    }

    /// Call for every block this node proposes or reconstructs, so it stops forwarding the
    /// block's transactions and drops copies that reach it later.
    pub fn on_block_reconstructed(
        &mut self,
        view: ViewNumber,
        tx_commitments: Vec<Commitment<T::Transaction>>,
    ) {
        for hash in &tx_commitments {
            self.remove_pending(hash);
        }
        self.mark_included(view, tx_commitments);
    }

    fn mark_included(&mut self, view: ViewNumber, hashes: Vec<Commitment<T::Transaction>>) {
        for hash in &hashes {
            if let Some(tx) = self.leader_buffer.remove(hash) {
                self.leader_total_bytes -= tx.minimum_block_size();
            }
        }

        let lower_bound: ViewNumber = self
            .current_view
            .saturating_sub(self.config.dedup_window_size)
            .into();

        if view >= lower_bound {
            self.dedups.entry(view).or_default().extend(hashes);
        }

        self.dedups = self.dedups.split_off(&lower_bound);
    }

    #[cfg(test)]
    pub(crate) fn drain(
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

impl<T: NodeType> From<&BlockBuilderOutput<T>> for HeaderRequest<T> {
    fn from(output: &BlockBuilderOutput<T>) -> Self {
        HeaderRequest {
            view: output.view,
            epoch: output.epoch,
            parent_proposal: output.parent_proposal.clone(),
            payload_commitment: output.payload_commitment,
            builder_commitment: output.builder_commitment.clone(),
            metadata: output.payload.metadata.clone(),
            builder_fee: output.builder_fee.clone(),
        }
    }
}

impl<T: NodeType> From<BlockBuilderOutput<T>> for ConsensusInput<T> {
    fn from(output: BlockBuilderOutput<T>) -> Self {
        ConsensusInput::BlockBuilt {
            view: output.view,
            epoch: output.epoch,
            payload: output.payload.payload,
            metadata: output.payload.metadata,
            payload_commitment: output.payload_commitment,
        }
    }
}
