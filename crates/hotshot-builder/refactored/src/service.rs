use std::{
    fmt::Display,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

pub use async_broadcast::{broadcast, RecvError, TryRecvError};
use async_lock::RwLock;
use async_trait::async_trait;
use committable::Commitment;
use futures::{
    future::BoxFuture,
    stream::{FuturesOrdered, FuturesUnordered, StreamExt},
    Stream, TryStreamExt,
};
use hotshot::types::Event;
use hotshot_builder_api::{
    v0_1::{
        block_info::{AvailableBlockData, AvailableBlockInfo},
        builder::{
            define_api, submit_api, BuildError, Error as BuilderApiError, TransactionStatus,
        },
        data_source::{AcceptsTxnSubmits, BuilderDataSource},
    },
    v0_2::block_info::AvailableBlockHeaderInputV1,
};
use hotshot_builder_shared::{
    block::{BlockId, BuilderStateId, ReceivedTransaction, TransactionSource},
    coordinator::{BuilderStateCoordinator, BuilderStateLookup},
    error::Error,
    state::BuilderState,
    utils::BuilderKeys,
};
use hotshot_types::{
    data::VidCommitment,
    event::EventType,
    traits::{
        block_contents::{BlockPayload, Transaction},
        node_implementation::{ConsensusTime, NodeType},
        signature_key::{BuilderSignatureKey, SignatureKey},
        EncodeBytes,
    },
    utils::BuilderCommitment,
};
use tagged_base64::TaggedBase64;
use tide_disco::{app::AppError, method::ReadState, App};
use tokio::{
    spawn,
    task::JoinHandle,
    time::{sleep, timeout},
};
use tracing::{error, info, instrument, trace, warn};
use vbs::version::StaticVersion;

use crate::{
    block_size_limits::BlockSizeLimits,
    block_store::{BlockInfo, BlockStore},
};

/// Proportion of overall allotted time to wait for optimal builder state
/// to appear before resorting to highest view builder state
const BUILDER_STATE_EXACT_MATCH_DIVISOR: u32 = 2;

/// This constant governs duration of sleep in various retry loops
/// in the API. If we're re-trying something with a timeout of `X`,
/// we will sleep for `X / RETRY_LOOP_RESOLUTION` between attempts.
const RETRY_LOOP_RESOLUTION: u32 = 10;

/// We will not increment max block value if we aren't able to serve a response
/// with a margin below [`GlobalState::max_api_waiting_time`]
/// more than [`GlobalState::max_api_waiting_time`] / `VID_RESPONSE_TARGET_MARGIN_DIVISOR`
const VID_RESPONSE_TARGET_MARGIN_DIVISOR: u32 = 10;

/// [`ALLOW_EMPTY_BLOCK_PERIOD`] is a constant that is used to determine the
/// number of future views that we will allow building empty blocks for.
///
/// This value governs the ability for the Builder to prioritize finalizing
/// transactions by producing empty blocks rather than avoiding the creation
/// of them, following the proposal that contains transactions.
pub(crate) const ALLOW_EMPTY_BLOCK_PERIOD: u64 = 3;

/// Configuration to initialize the builder
#[derive(Debug, Clone)]
pub struct BuilderConfig<Types: NodeType> {
    /// Keys that this builder will use to sign responses
    pub builder_keys: BuilderKeys<Types>,
    /// Maximum time allotted for the builder to respond to an API call.
    /// If the response isn't ready by this time, an error will be returned
    /// to the caller.
    pub max_api_waiting_time: Duration,
    /// Interval at which the builder will optimistically increment its maximum
    /// allowed block size in case it becomes lower than the protocol maximum.
    pub max_block_size_increment_period: Duration,
    /// Time the builder will wait for new transactions before answering an
    /// `available_blocks` API call if the builder doesn't have any transactions at the moment
    /// of the call. Should be less than [`Self::max_api_waiting_time`]
    pub maximize_txn_capture_timeout: Duration,
    /// (Approximate) duration over which included transaction hashes will be stored
    /// by the builder for deduplication of incoming transactions.
    pub txn_garbage_collect_duration: Duration,
    /// Channel capacity for incoming transactions for a single builder state.
    pub txn_channel_capacity: usize,
    /// Capacity of cache storing information for transaction status API
    pub tx_status_cache_capacity: usize,
    /// Base fee; the sequencing fee for a block is calculated as block size × base fee
    pub base_fee: u64,
}

#[cfg(test)]
impl<Types: NodeType> BuilderConfig<Types> {
    pub(crate) fn test() -> Self {
        use hotshot_builder_shared::testing::constants::*;
        Self {
            builder_keys:
                <Types::BuilderSignatureKey as BuilderSignatureKey>::generated_from_seed_indexed(
                    [0u8; 32], 42,
                ),
            max_api_waiting_time: TEST_API_TIMEOUT,
            max_block_size_increment_period: TEST_MAX_BLOCK_SIZE_INCREMENT_PERIOD,
            maximize_txn_capture_timeout: TEST_MAXIMIZE_TX_CAPTURE_TIMEOUT,
            txn_garbage_collect_duration: TEST_INCLUDED_TX_GC_PERIOD,
            txn_channel_capacity: TEST_CHANNEL_BUFFER_SIZE,
            tx_status_cache_capacity: TEST_TX_STATUS_CACHE_CAPACITY,
            base_fee: TEST_BASE_FEE,
        }
    }
}

pub struct GlobalState<Types: NodeType> {
    /// Underlying coordinator, responsible for builder state lifecycle
    pub(crate) coordinator: Arc<BuilderStateCoordinator<Types>>,
    /// Keys that this builder will use to sign responses
    pub(crate) builder_keys: BuilderKeys<Types>,
    /// Stores blocks built by this builder
    pub(crate) block_store: RwLock<BlockStore<Types>>,
    /// Limits on block size. See [`BlockSizeLimits`] documentation for more details.
    pub(crate) block_size_limits: BlockSizeLimits,
    /// Number of DA nodes used in VID computation
    pub(crate) num_nodes: AtomicUsize,
    /// Instance state, used to construct new blocks
    pub(crate) instance_state: Types::InstanceState,
    /// See [`BuilderConfig::max_api_waiting_time`]
    pub(crate) max_api_waiting_time: Duration,
    /// See [`BuilderConfig::maximize_txn_capture_timeout`]
    pub(crate) maximize_txn_capture_timeout: Duration,
    /// See [`BuilderConfig::base_fee`]
    pub(crate) base_fee: u64,
}

impl<Types: NodeType> GlobalState<Types>
where
    for<'a> <<Types::SignatureKey as SignatureKey>::PureAssembledSignatureType as TryFrom<
        &'a TaggedBase64,
    >>::Error: Display,
    for<'a> <Types::SignatureKey as TryFrom<&'a TaggedBase64>>::Error: Display,
{
    pub fn new(
        config: BuilderConfig<Types>,
        instance_state: Types::InstanceState,
        protocol_max_block_size: u64,
        num_nodes: usize,
    ) -> Arc<Self> {
        Arc::new(Self {
            coordinator: Arc::new(BuilderStateCoordinator::new(
                config.txn_channel_capacity,
                config.txn_garbage_collect_duration,
                config.tx_status_cache_capacity,
            )),
            block_store: RwLock::new(BlockStore::new()),
            block_size_limits: BlockSizeLimits::new(
                protocol_max_block_size,
                config.max_block_size_increment_period,
            ),
            num_nodes: num_nodes.into(),
            builder_keys: config.builder_keys,
            max_api_waiting_time: config.max_api_waiting_time,
            maximize_txn_capture_timeout: config.maximize_txn_capture_timeout,
            instance_state,
            base_fee: config.base_fee,
        })
    }

    /// Spawns an event loop handling HotShot events from the provided stream.
    /// Returns a handle for the spawned task.
    pub fn start_event_loop(
        self: Arc<Self>,
        event_stream: impl Stream<Item = Event<Types>> + Unpin + Send + 'static,
    ) -> JoinHandle<anyhow::Result<()>> {
        spawn(self.event_loop(event_stream))
    }

    /// Internal implementation of the event loop, drives the underlying coordinator
    /// and runs hooks
    async fn event_loop(
        self: Arc<Self>,
        mut event_stream: impl Stream<Item = Event<Types>> + Unpin + Send + 'static,
    ) -> anyhow::Result<()> {
        loop {
            let Some(event) = event_stream.next().await else {
                anyhow::bail!("Event stream ended");
            };

            match event.event {
                EventType::Error { error } => {
                    error!("Error event in HotShot: {error:?}");
                },
                EventType::Transactions { transactions } => {
                    let this = Arc::clone(&self);
                    spawn(async move {
                        transactions
                            .into_iter()
                            .map(|txn| {
                                this.handle_transaction(ReceivedTransaction::new(
                                    txn,
                                    TransactionSource::Public,
                                ))
                            })
                            .collect::<FuturesUnordered<_>>()
                            .collect::<Vec<_>>()
                            .await;
                    });
                },
                EventType::Decide { leaf_chain, .. } => {
                    let prune_cutoff = leaf_chain[0].leaf.view_number();

                    let coordinator = Arc::clone(&self.coordinator);
                    spawn(async move { coordinator.handle_decide(leaf_chain).await });

                    let this = Arc::clone(&self);
                    spawn(async move { this.block_store.write().await.prune(prune_cutoff) });
                },
                EventType::DaProposal { proposal, .. } => {
                    let coordinator = Arc::clone(&self.coordinator);
                    spawn(async move { coordinator.handle_da_proposal(proposal.data).await });
                },
                EventType::QuorumProposal { proposal, .. } => {
                    let coordinator = Arc::clone(&self.coordinator);
                    spawn(async move { coordinator.handle_quorum_proposal(proposal.data).await });
                },
                _ => {},
            }
        }
    }

    /// Consumes `self` and returns a `tide_disco` [`App`] with builder and private mempool APIs registered
    pub fn into_app(
        self: Arc<Self>,
    ) -> Result<App<ProxyGlobalState<Types>, BuilderApiError>, AppError> {
        let proxy = ProxyGlobalState(self);
        let builder_api = define_api::<ProxyGlobalState<Types>, Types>(&Default::default())?;

        // TODO: Replace StaticVersion with proper constant when added in HotShot
        let private_mempool_api =
            submit_api::<ProxyGlobalState<Types>, Types, StaticVersion<0, 1>>(&Default::default())?;

        let mut app: App<ProxyGlobalState<Types>, BuilderApiError> = App::with_state(proxy);

        app.register_module(hotshot_types::constants::LEGACY_BUILDER_MODULE, builder_api)?;

        app.register_module("txn_submit", private_mempool_api)?;

        Ok(app)
    }

    async fn handle_transaction(&self, tx: ReceivedTransaction<Types>) -> Result<(), Error<Types>> {
        let len = tx.transaction.minimum_block_size();
        let max_tx_len = self.block_size_limits.max_block_size();
        if len > max_tx_len {
            tracing::warn!(%tx.commit, %len, %max_tx_len, "Transaction too big");
            let error = Error::TxTooBig { len, max_tx_len };
            self.coordinator.update_txn_status(
                &tx.commit,
                TransactionStatus::Rejected {
                    reason: error.to_string(),
                },
            );
            return Err(error);
        }
        self.coordinator.handle_transaction(tx).await
    }

    async fn wait_for_builder_state(
        &self,
        state_id: &BuilderStateId<Types>,
        check_period: Duration,
    ) -> Result<Arc<BuilderState<Types>>, Error<Types>> {
        loop {
            match self.coordinator.lookup_builder_state(state_id).await {
                BuilderStateLookup::Found(builder) => break Ok(builder),
                BuilderStateLookup::Decided => {
                    return Err(Error::AlreadyDecided);
                },
                BuilderStateLookup::NotFound => {
                    sleep(check_period).await;
                },
            };
        }
    }

    /// Build a block with provided builder state
    ///
    /// Returns None if there are no transactions to include
    /// and we aren't prioritizing finalization for this builder state
    pub(crate) async fn build_block(
        &self,
        builder_state: Arc<BuilderState<Types>>,
    ) -> Result<Option<BlockInfo<Types>>, Error<Types>> {
        let timeout_after = Instant::now() + self.maximize_txn_capture_timeout;
        let sleep_interval = self.maximize_txn_capture_timeout / RETRY_LOOP_RESOLUTION;

        while Instant::now() <= timeout_after {
            let queue_populated = builder_state.collect_txns(timeout_after).await;

            if queue_populated || Instant::now() + sleep_interval > timeout_after {
                // we don't have time for another iteration
                break;
            }

            sleep(sleep_interval).await
        }

        // If the parent block had transactions included and [`ALLOW_EMPTY_BLOCK_PERIOD`] views has not
        // passed since, we will allow building empty blocks. This is done to allow for faster finalization
        // of previous blocks that have had transactions included in them.
        let should_prioritize_finalization = builder_state.parent_block_references.tx_count != 0
            && builder_state
                .parent_block_references
                .last_nonempty_view
                .map(|nonempty_view| {
                    nonempty_view.saturating_sub(*builder_state.parent_block_references.view_number)
                        < ALLOW_EMPTY_BLOCK_PERIOD
                })
                .unwrap_or(false);

        let builder: &Arc<BuilderState<Types>> = &builder_state;
        let max_block_size = self.block_size_limits.max_block_size();

        let transactions_to_include = {
            let txn_queue = builder.txn_queue.read().await;
            if txn_queue.is_empty() && !should_prioritize_finalization {
                // Don't build an empty block
                return Ok(None);
            }
            txn_queue
                .iter()
                .scan(0, |total_size, tx| {
                    let prev_size = *total_size;
                    *total_size += tx.min_block_size;
                    // We will include one transaction over our target block length
                    // if it's the first transaction in queue, otherwise we'd have a possible failure
                    // state where a single transaction larger than target block state is stuck in
                    // queue and we just build empty blocks forever
                    if *total_size >= max_block_size && prev_size != 0 {
                        None
                    } else {
                        // Note: we're going to map from ReceivedTransaction to
                        // Transaction it contains later, so we can just clone
                        // the Arc here to reduce the time we hold the lock
                        Some(Arc::clone(tx))
                    }
                })
                .collect::<Vec<_>>()
        };

        let (payload, metadata) =
            match <Types::BlockPayload as BlockPayload<Types>>::from_transactions(
                transactions_to_include
                    .into_iter()
                    .map(|tx| tx.transaction.clone()),
                &builder.validated_state,
                &self.instance_state,
            )
            .await
            {
                Ok((payload, metadata)) => (payload, metadata),
                Err(error) => {
                    warn!(?error, "Failed to build block payload");
                    return Err(Error::BuildBlock(error));
                },
            };

        // count the number of txns
        let actual_txn_count = payload.num_transactions(&metadata);
        let truncated = actual_txn_count == 0;

        // Payload is empty despite us checking that tx_queue isn't empty earlier.
        //
        // This means that the block was truncated due to *sequencer* block length
        // limits, which are different from our `max_block_size`. There's no good way
        // for us to check for this in advance, so we detect transactions too big for
        // the sequencer indirectly, by observing that we passed some transactions
        // to `<Types::BlockPayload as BlockPayload<Types>>::from_transactions`, but
        // it returned an empty block.
        // Thus we deduce that the first transaction in our queue is too big to *ever*
        // be included, because it alone goes over sequencer's block size limit.
        if truncated {
            builder.txn_queue.write().await.pop_front();
            if !should_prioritize_finalization {
                return Ok(None);
            }
        }

        let encoded_txns: Vec<u8> = payload.encode().to_vec();
        let block_size: u64 = encoded_txns.len() as u64;
        let offered_fee: u64 = self.base_fee * block_size;

        info!(
            builder_id = %builder.id(),
            txn_count = actual_txn_count,
            block_size,
            "Built a block",
        );

        Ok(Some(BlockInfo {
            block_payload: payload,
            block_size,
            metadata,
            offered_fee,
            truncated,
        }))
    }

    #[instrument(skip_all,
        fields(state_id = %state_id)
    )]
    pub(crate) async fn available_blocks_implementation(
        &self,
        state_id: BuilderStateId<Types>,
    ) -> Result<Vec<AvailableBlockInfo<Types>>, Error<Types>> {
        let start = Instant::now();

        let check_period = self.max_api_waiting_time / RETRY_LOOP_RESOLUTION;
        let time_to_wait_for_matching_builder =
            self.max_api_waiting_time / BUILDER_STATE_EXACT_MATCH_DIVISOR;

        let builder = match timeout(
            time_to_wait_for_matching_builder,
            self.wait_for_builder_state(&state_id, check_period),
        )
        .await
        {
            Ok(Ok(builder)) => Some(builder),
            Err(_) => {
                // Timeout waiting for ideal state, get the highest view builder instead
                warn!("Couldn't find the ideal builder state");
                self.coordinator.highest_view_builder().await
            },
            Ok(Err(e)) => {
                // State already decided
                let lowest_view = self.coordinator.lowest_view().await;
                warn!(
                    ?lowest_view,
                    "get_available_blocks request for decided view"
                );
                return Err(e);
            },
        };

        let Some(builder) = builder else {
            if let Some(cached_block) = self.block_store.read().await.get_cached(&state_id) {
                return Ok(vec![cached_block.signed_response(&self.builder_keys)?]);
            } else {
                return Err(Error::NotFound);
            };
        };

        let build_block_timeout = self
            .max_api_waiting_time
            .saturating_sub(start.elapsed())
            .div_f32(1.1);
        match timeout(build_block_timeout, self.build_block(builder))
            .await
            .map_err(|_| Error::ApiTimeout)
        {
            // Success
            Ok(Ok(Some(info))) => {
                let block_id = BlockId {
                    hash: info.block_payload.builder_commitment(&info.metadata),
                    view: state_id.parent_view,
                };

                let response = info.signed_response(&self.builder_keys)?;

                {
                    let mut mutable_state = self.block_store.write().await;
                    mutable_state.update(state_id, block_id, info);
                }

                Ok(vec![response])
            },
            // Success, but no block: we don't have transactions and aren't prioritizing finalization
            Ok(Ok(None)) => Ok(vec![]),
            // Error building block, try to respond with a cached one as last-ditch attempt
            Ok(Err(e)) | Err(e) => {
                if let Some(cached_block) = self.block_store.read().await.get_cached(&state_id) {
                    Ok(vec![cached_block.signed_response(&self.builder_keys)?])
                } else {
                    Err(e)
                }
            },
        }
    }

    #[instrument(skip_all,
        fields(block_id = %block_id)
    )]
    pub(crate) async fn claim_block_implementation(
        &self,
        block_id: BlockId<Types>,
    ) -> Result<AvailableBlockData<Types>, Error<Types>> {
        let (block_payload, metadata) = {
            // We store this read lock guard separately to make it explicit
            // that this will end up holding a lock for the duration of this
            // closure.
            //
            // Additionally, we clone the properties from the block_info that
            // end up being cloned if found anyway.  Since we know this already
            // we can perform the clone here to avoid holding the lock for
            // longer than needed.
            let mutable_state_read = self.block_store.read().await;
            let block_info = mutable_state_read
                .get_block(&block_id)
                .ok_or(Error::NotFound)?;

            (
                block_info.block_payload.clone(),
                block_info.metadata.clone(),
            )
        };

        let (pub_key, sign_key) = self.builder_keys.clone();

        // sign over the builder commitment, as the proposer can computer it based on provide block_payload
        // and the metadata
        let response_block_hash = block_payload.builder_commitment(&metadata);
        let signature_over_builder_commitment =
            <Types as NodeType>::BuilderSignatureKey::sign_builder_message(
                &sign_key,
                response_block_hash.as_ref(),
            )
            .map_err(Error::Signing)?;

        let block_data = AvailableBlockData::<Types> {
            block_payload,
            metadata,
            signature: signature_over_builder_commitment,
            sender: pub_key,
        };

        info!("Sending Claim Block data for {block_id}");
        Ok(block_data)
    }

    #[instrument(skip_all,
        fields(block_id = %block_id)
    )]
    pub(crate) async fn claim_block_header_input_implementation(
        &self,
        block_id: BlockId<Types>,
    ) -> Result<(bool, AvailableBlockHeaderInputV1<Types>), Error<Types>> {
        let metadata;
        let offered_fee;
        let truncated;
        {
            // We store this read lock guard separately to make it explicit
            // that this will end up holding a lock for the duration of this
            // closure.
            //
            // Additionally, we clone the properties from the block_info that
            // end up being cloned if found anyway.  Since we know this already
            // we can perform the clone here to avoid holding the lock for
            // longer than needed.
            let mutable_state_read_lock_guard = self.block_store.read().await;
            let block_info = mutable_state_read_lock_guard
                .get_block(&block_id)
                .ok_or(Error::NotFound)?;

            metadata = block_info.metadata.clone();
            offered_fee = block_info.offered_fee;
            truncated = block_info.truncated;
        };

        // TODO Add precompute back.

        let signature_over_fee_info =
            Types::BuilderSignatureKey::sign_fee(&self.builder_keys.1, offered_fee, &metadata)
                .map_err(Error::Signing)?;

        let response = AvailableBlockHeaderInputV1::<Types> {
            fee_signature: signature_over_fee_info,
            sender: self.builder_keys.0.clone(),
        };
        info!("Sending Claim Block Header Input response");
        Ok((truncated, response))
    }
}

#[derive(derive_more::Deref, derive_more::DerefMut)]
#[deref(forward)]
#[deref_mut(forward)]
pub struct ProxyGlobalState<Types: NodeType>(pub Arc<GlobalState<Types>>);

/*
Handling Builder API responses
*/
#[async_trait]
impl<Types: NodeType> BuilderDataSource<Types> for ProxyGlobalState<Types>
where
    for<'a> <<Types::SignatureKey as SignatureKey>::PureAssembledSignatureType as TryFrom<
        &'a TaggedBase64,
    >>::Error: Display,
    for<'a> <Types::SignatureKey as TryFrom<&'a TaggedBase64>>::Error: Display,
{
    #[tracing::instrument(skip_all)]
    async fn available_blocks(
        &self,
        parent_block: &VidCommitment,
        parent_view: u64,
        sender: Types::SignatureKey,
        signature: &<Types::SignatureKey as SignatureKey>::PureAssembledSignatureType,
    ) -> Result<Vec<AvailableBlockInfo<Types>>, BuildError> {
        // verify the signature
        if !sender.validate(signature, parent_block.as_ref()) {
            warn!("Signature validation failed");
            return Err(Error::<Types>::SignatureValidation.into());
        }

        let state_id = BuilderStateId {
            parent_commitment: *parent_block,
            parent_view: Types::View::new(parent_view),
        };

        trace!("Requesting available blocks");

        let available_blocks = timeout(
            self.max_api_waiting_time,
            self.available_blocks_implementation(state_id),
        )
        .await
        .map_err(|_| Error::<Types>::ApiTimeout)??;

        Ok(available_blocks)
    }

    #[tracing::instrument(skip_all)]
    async fn claim_block(
        &self,
        block_hash: &BuilderCommitment,
        view_number: u64,
        sender: Types::SignatureKey,
        signature: &<<Types as NodeType>::SignatureKey as SignatureKey>::PureAssembledSignatureType,
    ) -> Result<AvailableBlockData<Types>, BuildError> {
        // verify the signature
        if !sender.validate(signature, block_hash.as_ref()) {
            warn!("Signature validation failed");
            return Err(Error::<Types>::SignatureValidation.into());
        }

        let block_id = BlockId {
            hash: block_hash.clone(),
            view: Types::View::new(view_number),
        };

        trace!("Processing claim block request");

        let block = timeout(
            self.max_api_waiting_time,
            self.claim_block_implementation(block_id),
        )
        .await
        .map_err(|_| Error::<Types>::ApiTimeout)??;

        Ok(block)
    }

    #[tracing::instrument(skip_all)]
    async fn claim_block_with_num_nodes(
        &self,
        block_hash: &BuilderCommitment,
        view_number: u64,
        sender: <Types as NodeType>::SignatureKey,
        signature: &<<Types as NodeType>::SignatureKey as SignatureKey>::PureAssembledSignatureType,
        num_nodes: usize,
    ) -> Result<AvailableBlockData<Types>, BuildError> {
        // Update the stored `num_nodes` with the given value, which will be used for VID computation.
        trace!(
            new_num_nodes = num_nodes,
            old_num_nodes = self.num_nodes.load(Ordering::Relaxed),
            "Updating num_nodes"
        );

        self.num_nodes.store(num_nodes, Ordering::Relaxed);

        self.claim_block(block_hash, view_number, sender, signature)
            .await
    }

    #[tracing::instrument(skip_all)]
    async fn claim_block_header_input(
        &self,
        block_hash: &BuilderCommitment,
        view_number: u64,
        sender: Types::SignatureKey,
        signature: &<<Types as NodeType>::SignatureKey as SignatureKey>::PureAssembledSignatureType,
    ) -> Result<AvailableBlockHeaderInputV1<Types>, BuildError> {
        let start = Instant::now();
        // verify the signature
        if !sender.validate(signature, block_hash.as_ref()) {
            warn!("Signature validation failed in claim_block_header_input");
            return Err(Error::<Types>::SignatureValidation.into());
        }

        let block_id = BlockId {
            hash: block_hash.clone(),
            view: Types::View::new(view_number),
        };

        trace!("Processing claim_block_header_input request");

        let (truncated, info) = timeout(
            self.max_api_waiting_time,
            self.claim_block_header_input_implementation(block_id),
        )
        .await
        .inspect_err(|_| {
            // we can't keep up with this block size, reduce max block size
            self.block_size_limits.decrement_block_size();
        })
        .map_err(|_| Error::<Types>::ApiTimeout)??;

        if self.max_api_waiting_time.saturating_sub(start.elapsed())
            > self.max_api_waiting_time / VID_RESPONSE_TARGET_MARGIN_DIVISOR
        {
            // Increase max block size
            self.block_size_limits.try_increment_block_size(truncated);
        }

        Ok(info)
    }

    /// Returns the public key of the builder
    #[tracing::instrument(skip_all)]
    async fn builder_address(
        &self,
    ) -> Result<<Types as NodeType>::BuilderSignatureKey, BuildError> {
        Ok(self.builder_keys.0.clone())
    }
}

#[async_trait]
impl<Types: NodeType> AcceptsTxnSubmits<Types> for ProxyGlobalState<Types>
where
    for<'a> <<Types::SignatureKey as SignatureKey>::PureAssembledSignatureType as TryFrom<
        &'a TaggedBase64,
    >>::Error: Display,
    for<'a> <Types::SignatureKey as TryFrom<&'a TaggedBase64>>::Error: Display,
{
    async fn submit_txns(
        &self,
        txns: Vec<<Types as NodeType>::Transaction>,
    ) -> Result<Vec<Commitment<<Types as NodeType>::Transaction>>, BuildError> {
        tracing::warn!(
            "BlockBuilder: submit_txns called with transactions: {:?}",
            txns
        );
        txns.into_iter()
            .map(|txn| ReceivedTransaction::new(txn, TransactionSource::Private))
            .map(|txn| async {
                let commit = txn.commit;
                self.0.handle_transaction(txn).await?;
                Ok(commit)
            })
            .collect::<FuturesOrdered<_>>()
            .try_collect()
            .await
    }

    async fn txn_status(
        &self,
        txn_hash: Commitment<<Types as NodeType>::Transaction>,
    ) -> Result<TransactionStatus, BuildError> {
        Ok(self.coordinator.tx_status(&txn_hash))
    }
}

#[async_trait]
impl<Types: NodeType> ReadState for ProxyGlobalState<Types> {
    type State = ProxyGlobalState<Types>;

    async fn read<T>(
        &self,
        op: impl Send + for<'a> FnOnce(&'a Self::State) -> BoxFuture<'a, T> + 'async_trait,
    ) -> T {
        op(self).await
    }
}
