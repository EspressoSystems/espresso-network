use std::{
    collections::HashMap,
    fmt::Display,
    num::NonZeroUsize,
    sync::Arc,
    time::{Duration, Instant},
};

pub use async_broadcast::{broadcast, RecvError, TryRecvError};
use async_broadcast::{Sender as BroadcastSender, TrySendError};
use async_lock::RwLock;
use async_trait::async_trait;
use committable::{Commitment, Committable};
use futures::{future::BoxFuture, stream::StreamExt, Stream};
use hotshot::types::Event;
use hotshot_builder_api::{
    v0_1::{
        block_info::{AvailableBlockData, AvailableBlockHeaderInputV1, AvailableBlockInfo},
        builder::BuildError,
        data_source::{AcceptsTxnSubmits, BuilderDataSource},
    },
    v0_2::builder::TransactionStatus,
};
use hotshot_types::{
    data::{DaProposal2, Leaf2, QuorumProposalWrapper, VidCommitment},
    event::EventType,
    message::Proposal,
    traits::{
        block_contents::{BlockPayload, Transaction},
        node_implementation::{ConsensusTime, NodeType},
        signature_key::{BuilderSignatureKey, SignatureKey},
    },
    utils::BuilderCommitment,
};
use lru::LruCache;
use marketplace_builder_shared::block::{BlockId, BuilderStateId, ParentBlockReferences};
use sha2::{Digest, Sha256};
use tagged_base64::TaggedBase64;
use tide_disco::method::ReadState;
use tokio::{
    sync::{mpsc::unbounded_channel, oneshot},
    time::{sleep, timeout},
};
use vbs::version::StaticVersionType;

use crate::builder_state::{
    BuildBlockInfo, DaProposalMessage, DecideMessage, MessageType, QuorumProposalMessage,
    RequestMessage, ResponseMessage, TransactionSource, TriggerStatus,
};

// It holds all the necessary information for a block
#[derive(Debug)]
pub struct BlockInfo<Types: NodeType> {
    pub block_payload: Types::BlockPayload,
    pub metadata: <<Types as NodeType>::BlockPayload as BlockPayload<Types>>::Metadata,
    pub vid_trigger: Arc<RwLock<Option<oneshot::Sender<TriggerStatus>>>>,
    // TODO Add precompute back.
    pub offered_fee: u64,
    // Could we have included more transactions with this block, but chose not to?
    pub truncated: bool,
}

/// [`ReceivedTransaction`] represents receipt information concerning a received
/// [`NodeType::Transaction`].
#[derive(Debug)]
pub struct ReceivedTransaction<Types: NodeType> {
    // the transaction
    pub tx: Types::Transaction,
    // transaction's hash
    pub commit: Commitment<Types::Transaction>,
    // transaction's estimated length
    pub len: u64,
    // transaction's source
    pub source: TransactionSource,
    // received time
    pub time_in: Instant,
}

/// Adjustable limits for block size ceiled by
/// maximum block size allowed by the protocol
#[derive(Debug, Clone)]
pub struct BlockSizeLimits {
    // maximum block size allowed by the protocol
    pub protocol_max_block_size: u64,
    // estimated maximum block size we can build in time
    pub max_block_size: u64,
    pub increment_period: Duration,
    pub last_block_size_increment: Instant,
}

impl BlockSizeLimits {
    /// Never go lower than 10 kilobytes
    pub const MAX_BLOCK_SIZE_FLOOR: u64 = 10_000;
    /// When adjusting max block size, it will be decremented or incremented
    /// by current value / `MAX_BLOCK_SIZE_CHANGE_DIVISOR`
    pub const MAX_BLOCK_SIZE_CHANGE_DIVISOR: u64 = 10;

    pub fn new(protocol_max_block_size: u64, increment_period: Duration) -> Self {
        Self {
            protocol_max_block_size,
            max_block_size: protocol_max_block_size,
            increment_period,
            last_block_size_increment: Instant::now(),
        }
    }

    /// If increment period has elapsed or `force` flag is set,
    /// increment [`Self::max_block_size`] by current value * [`Self::MAX_BLOCK_SIZE_CHANGE_DIVISOR`]
    /// with [`Self::protocol_max_block_size`] as a ceiling
    pub fn try_increment_block_size(&mut self, force: bool) {
        if force || self.last_block_size_increment.elapsed() >= self.increment_period {
            self.max_block_size = std::cmp::min(
                self.max_block_size
                    + self
                        .max_block_size
                        .div_ceil(Self::MAX_BLOCK_SIZE_CHANGE_DIVISOR),
                self.protocol_max_block_size,
            );
            self.last_block_size_increment = Instant::now();
        }
    }

    /// Decrement [`Self::max_block_size`] by current value * [`Self::MAX_BLOCK_SIZE_CHANGE_DIVISOR`]
    /// with [`Self::MAX_BLOCK_SIZE_FLOOR`] as a floor
    pub fn decrement_block_size(&mut self) {
        self.max_block_size = std::cmp::max(
            self.max_block_size
                - self
                    .max_block_size
                    .div_ceil(Self::MAX_BLOCK_SIZE_CHANGE_DIVISOR),
            Self::MAX_BLOCK_SIZE_FLOOR,
        );
    }
}

/// [`GlobalState`] represents the internalized state of the Builder service as
/// represented from its public facing API.
#[allow(clippy::type_complexity)]
#[derive(Debug)]
pub struct GlobalState<Types: NodeType> {
    // data store for the blocks
    pub blocks: lru::LruCache<BlockId<Types>, BlockInfo<Types>>,

    // registered builder states
    pub spawned_builder_states: HashMap<
        BuilderStateId<Types>,
        (
            // This is provided as an Option for convenience with initialization.
            // When we build the initial state, we don't necessarily want to
            // have to generate a valid ParentBlockReferences object.  As doing
            // such would require a bit of setup.  Additionally it would
            // result in the call signature to `GlobalState::new` changing.
            // However for every subsequent BuilderState, we expect this value
            // to be populated.
            Option<ParentBlockReferences<Types>>,
            BroadcastSender<MessageType<Types>>,
        ),
    >,

    // builder state -> last built block , it is used to respond the client
    // if the req channel times out during get_available_blocks
    pub builder_state_to_last_built_block: HashMap<BuilderStateId<Types>, ResponseMessage>,

    // sending a transaction from the hotshot/private mempool to the builder states
    // NOTE: Currently, we don't differentiate between the transactions from the hotshot and the private mempool
    pub tx_sender: BroadcastSender<Arc<ReceivedTransaction<Types>>>,

    // last garbage collected view number
    pub last_garbage_collected_view_num: Types::View,

    // highest view running builder task
    pub highest_view_num_builder_id: BuilderStateId<Types>,

    pub block_size_limits: BlockSizeLimits,

    // A mapping from transaction hash to its status
    pub tx_status: RwLock<lru::LruCache<Commitment<Types::Transaction>, TransactionStatus>>,

    /// Number of nodes.
    ///
    /// Initial value may be updated by the `claim_block_with_num_nodes` endpoint.
    pub num_nodes: usize,
}

/// `GetChannelForMatchingBuilderError` is an error enum that represents the
/// class of possible errors that can be returned when calling
/// `get_channel_for_matching_builder_or_highest_view_builder` on a
/// `GlobalState`.  These errors are used for internal representations for
/// consistency and testing, and do not leak beyond the `GlobalState` API.
/// As such, they intentionally do not implement traits for serialization.
#[derive(Debug)]
pub(crate) enum GetChannelForMatchingBuilderError {
    NoBuilderStateFound,
}

impl From<GetChannelForMatchingBuilderError> for BuildError {
    fn from(_error: GetChannelForMatchingBuilderError) -> Self {
        BuildError::Error("No builder state found".to_string())
    }
}

impl<Types: NodeType> GlobalState<Types> {
    /// Creates a new [`GlobalState`] with the given parameters.
    /// The resulting [`GlobalState`] will have the given
    /// `last_garbage_collected_view_num` as passed.  Additionally, the
    /// `highest_view_num_builder_id` will be set to a [`BuilderStateId`]
    /// comprised of the given `bootstrapped_builder_state_id` and
    /// `bootstrapped_view_num`.  The `spawned_builder_states` will be created
    /// with a single entry of the same [`BuilderStateId`] and the given
    /// `bootstrap_sender`.
    /// `protocol_max_block_size` is maximum block size allowed by the protocol,
    /// e.g. `chain_config.max_block_size` for espresso-sequencer.
    /// `max_block_size_increment_period` determines the interval between attempts
    /// to increase the builder's block size limit if it is less than the protocol maximum.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        bootstrap_sender: BroadcastSender<MessageType<Types>>,
        tx_sender: BroadcastSender<Arc<ReceivedTransaction<Types>>>,
        bootstrapped_builder_state_id: VidCommitment,
        bootstrapped_view_num: Types::View,
        last_garbage_collected_view_num: Types::View,
        max_block_size_increment_period: Duration,
        protocol_max_block_size: u64,
        num_nodes: usize,
        max_txn_num: usize,
    ) -> Self {
        let mut spawned_builder_states = HashMap::new();
        let bootstrap_id = BuilderStateId {
            parent_commitment: bootstrapped_builder_state_id,
            parent_view: bootstrapped_view_num,
        };
        spawned_builder_states.insert(bootstrap_id.clone(), (None, bootstrap_sender.clone()));
        GlobalState {
            blocks: LruCache::new(NonZeroUsize::new(256).unwrap()),
            spawned_builder_states,
            tx_sender,
            last_garbage_collected_view_num,
            builder_state_to_last_built_block: Default::default(),
            highest_view_num_builder_id: bootstrap_id,
            block_size_limits: BlockSizeLimits::new(
                protocol_max_block_size,
                max_block_size_increment_period,
            ),
            tx_status: RwLock::new(LruCache::new(
                NonZeroUsize::new(max_txn_num).expect("max_txn_num must be greater than zero "),
            )),
            num_nodes,
        }
    }

    /// Associates the given [`BuilderStateId`] with
    /// the given [`BroadcastSender`] in the [`GlobalState`].
    ///
    /// Additionally, if the view of the [`BuilderStateId`] is greater than the
    /// current highest view number, the [`BuilderStateId`] is set as the new
    /// highest view number.
    ///
    /// There is potential here for data loss.  Since we just blindly insert
    /// the [`BuilderStateId`] and [`BroadcastSender`] into the hashmap, we could
    /// potentially be overwriting an existing entry.  This would result in
    /// the loss of access to a [`BroadcastSender`], and could potentially
    /// result in unexpected behavior.
    pub fn register_builder_state(
        &mut self,
        parent_id: BuilderStateId<Types>,
        built_from_proposed_block: ParentBlockReferences<Types>,
        request_sender: BroadcastSender<MessageType<Types>>,
    ) {
        // register the builder state
        let previous_value = self.spawned_builder_states.insert(
            parent_id.clone(),
            (Some(built_from_proposed_block), request_sender),
        );

        if let Some(previous_value) = previous_value {
            tracing::warn!(
                "builder {parent_id} overwrote previous spawned_builder_state entry: {:?}",
                previous_value
            );
        }

        // keep track of the max view number
        if parent_id.parent_view > self.highest_view_num_builder_id.parent_view {
            tracing::info!("registering builder {parent_id} as highest",);
            self.highest_view_num_builder_id = parent_id;
        } else {
            tracing::warn!(
                "builder {parent_id} created; highest registered is {}",
                self.highest_view_num_builder_id,
            );
        }
    }

    /// Ensures that the given [`BuildBlockInfo`]'d id
    /// is within the [`GlobalState`]'s [`blocks`](GlobalState::blocks) LRU Cache.  The cache stores the
    /// [`BlockInfo`] associated with the given [`BuildBlockInfo`]'s id.  However
    /// if it already exists within the LRU cache, then the `BlockInfo` is not
    /// updated.
    ///
    /// Additionally, the [`BuilderStateId`] is associated with the given
    /// [`ResponseMessage`] in the [`Self::builder_state_to_last_built_block`] hashmap.
    ///
    /// No care or consideration is given to anything that may have been
    /// stored with the same key in the [`Self::builder_state_to_last_built_block`].
    pub fn update_global_state(
        &mut self,
        state_id: BuilderStateId<Types>,
        build_block_info: BuildBlockInfo<Types>,
        response_msg: ResponseMessage,
    ) {
        let BuildBlockInfo {
            id,
            block_payload,
            metadata,
            vid_trigger,
            offered_fee,
            truncated,
            ..
        } = build_block_info;

        let previous_cache_entry = self.blocks.put(
            id.clone(),
            BlockInfo {
                block_payload,
                metadata,
                vid_trigger: Arc::new(RwLock::new(Some(vid_trigger))),
                offered_fee,
                truncated,
            },
        );

        // update the builder state to last built block
        let previous_builder_state_entry = self
            .builder_state_to_last_built_block
            .insert(state_id, response_msg);

        if let Some(previous_builder_state_entry) = previous_builder_state_entry {
            tracing::warn!(
                "block {id} overwrote previous block: {:?}.  previous cache entry: {:?}",
                previous_builder_state_entry,
                previous_cache_entry
            );
        }
    }

    /// Cleans up the [`GlobalState`] by removing all
    /// `spawned_builder_states` that have been stored, up to a derived
    /// reference view.  This cutoff point can be up to the given
    /// `on_decide_view` so long as the provided value is less than or equal
    /// to the `highest_view_num_builder_id`'s view stored on the state.
    /// Beyond that, the state prefers to drop all `spawned_builder_states`
    /// preceding the derived cutoff view.
    ///
    /// In addition the `last_garbage_collected_view_num` is updated to the
    /// target cutoff view number for tracking purposes.  The value returned
    /// is the cutoff view number such that the returned value indicates the
    /// point before which everything was cleaned up.
    pub fn remove_handles(&mut self, on_decide_view: Types::View) -> Types::View {
        // remove everything from the spawned builder states when view_num <= on_decide_view;
        // if we don't have a highest view > decide, use highest view as cutoff.
        let cutoff = std::cmp::min(self.highest_view_num_builder_id.parent_view, on_decide_view);
        self.spawned_builder_states
            .retain(|id, _| id.parent_view >= cutoff);

        let cutoff_u64 = cutoff.u64();
        let gc_view = if cutoff_u64 > 0 { cutoff_u64 - 1 } else { 0 };

        self.last_garbage_collected_view_num = Types::View::new(gc_view);

        cutoff
    }

    // private mempool submit txn
    // Currently, we don't differentiate between the transactions from the hotshot and the private mempool
    pub async fn submit_client_txns(
        &self,
        txns: Vec<<Types as NodeType>::Transaction>,
    ) -> Vec<Result<Commitment<<Types as NodeType>::Transaction>, BuildError>> {
        handle_received_txns(
            &self.tx_sender,
            txns,
            TransactionSource::External,
            self.block_size_limits.max_block_size,
        )
        .await
    }

    // get transaction status
    // return one of "pending", "sequenced", "rejected" or "unknown"
    pub async fn txn_status(
        &self,
        txn_hash: Commitment<<Types as NodeType>::Transaction>,
    ) -> Result<TransactionStatus, BuildError> {
        if let Some(status) = self.tx_status.write().await.get(&txn_hash) {
            Ok(status.clone())
        } else {
            Ok(TransactionStatus::Unknown)
        }
    }

    pub async fn set_txn_status(
        &mut self,
        txn_hash: Commitment<<Types as NodeType>::Transaction>,
        txn_status: TransactionStatus,
    ) -> Result<(), BuildError> {
        let mut write_guard = self.tx_status.write().await;
        if write_guard.contains(&txn_hash) {
            let old_status = write_guard.get(&txn_hash);
            match old_status {
                Some(TransactionStatus::Rejected { reason }) => {
                    tracing::debug!("Changing the status of a rejected transaction to status {:?}! The reason it is previously rejected is {:?}", txn_status, reason);
                },
                Some(TransactionStatus::Sequenced { leaf }) => {
                    let e = format!("Changing the status of a sequenced transaction to status {:?} is not allowed! The transaction is sequenced in leaf {:?}", txn_status, leaf);
                    tracing::error!(e);
                    return Err(BuildError::Error(e));
                },
                _ => {
                    tracing::debug!(
                        "change status of transaction {txn_hash} from {:?} to {:?}",
                        old_status,
                        txn_status
                    );
                },
            }
        } else {
            tracing::debug!(
                "insert status of a first-seen transaction {txn_hash} : {:?}",
                txn_status
            );
        }
        write_guard.put(txn_hash, txn_status);
        Ok(())
    }

    /// Helper function that attempts to retrieve the broadcast sender for the given
    /// [`BuilderStateId`]. If the sender does not exist, it will return the
    /// broadcast sender for the for the highest view number [`BuilderStateId`]
    /// instead.
    pub(crate) fn get_channel_for_matching_builder_or_highest_view_builder(
        &self,
        key: &BuilderStateId<Types>,
    ) -> Result<&BroadcastSender<MessageType<Types>>, GetChannelForMatchingBuilderError> {
        if let Some(id_and_sender) = self.spawned_builder_states.get(key) {
            tracing::info!("Got matching builder for parent {}", key);
            Ok(&id_and_sender.1)
        } else {
            tracing::warn!(
                "failed to recover builder for parent {}, using highest view num builder with {}",
                key,
                self.highest_view_num_builder_id,
            );
            // get the sender for the highest view number builder
            self.spawned_builder_states
                .get(&self.highest_view_num_builder_id)
                .map(|(_, sender)| sender)
                .ok_or(GetChannelForMatchingBuilderError::NoBuilderStateFound)
        }
    }

    // check for the existence of the builder state for a view
    pub fn check_builder_state_existence_for_a_view(&self, key: &Types::View) -> bool {
        // iterate over the spawned builder states and check if the view number exists
        self.spawned_builder_states
            .iter()
            .any(|(id, _)| id.parent_view == *key)
    }

    pub fn should_view_handle_other_proposals(
        &self,
        builder_view: &Types::View,
        proposal_view: &Types::View,
    ) -> bool {
        *builder_view == self.highest_view_num_builder_id.parent_view
            && !self.check_builder_state_existence_for_a_view(proposal_view)
    }
}

#[derive(derive_more::Deref, derive_more::DerefMut)]
pub struct ProxyGlobalState<Types: NodeType> {
    #[deref(forward)]
    #[deref_mut(forward)]
    // global state
    global_state: Arc<RwLock<GlobalState<Types>>>,

    // identity keys for the builder
    // May be ideal place as GlobalState interacts with hotshot apis
    // and then can sign on responders as desired
    builder_keys: (
        Types::BuilderSignatureKey, // pub key
        <<Types as NodeType>::BuilderSignatureKey as BuilderSignatureKey>::BuilderPrivateKey, // private key
    ),

    // max waiting time to serve first api request
    max_api_waiting_time: Duration,
}

impl<Types: NodeType> ProxyGlobalState<Types> {
    pub fn new(
        global_state: Arc<RwLock<GlobalState<Types>>>,
        builder_keys: (
            Types::BuilderSignatureKey,
            <<Types as NodeType>::BuilderSignatureKey as BuilderSignatureKey>::BuilderPrivateKey,
        ),
        max_api_waiting_time: Duration,
    ) -> Self {
        ProxyGlobalState {
            global_state,
            builder_keys,
            max_api_waiting_time,
        }
    }
}

/// `AvailableBlocksError` is an error enum that represents the class of possible
/// errors  that can be returned when calling `available_blocks` on a
/// `ProxyGlobalState`.  These errors are used for internal representations
/// for consistency and testing, and do not leak beyond the `ProxyGlobalState`
/// API.  As such, they intentionally do not implement traits for serialization.
#[derive(Debug)]
enum AvailableBlocksError<Types: NodeType> {
    SignatureValidationFailed,
    RequestForAvailableViewThatHasAlreadyBeenDecided,
    SigningBlockFailed(
        <<Types as NodeType>::BuilderSignatureKey as BuilderSignatureKey>::SignError,
    ),
    GetChannelForMatchingBuilderError(GetChannelForMatchingBuilderError),
    NoBlocksAvailable,
    ChannelUnexpectedlyClosed,
}

impl<Types: NodeType> From<GetChannelForMatchingBuilderError> for AvailableBlocksError<Types> {
    fn from(error: GetChannelForMatchingBuilderError) -> Self {
        AvailableBlocksError::GetChannelForMatchingBuilderError(error)
    }
}

impl<Types: NodeType> From<AvailableBlocksError<Types>> for BuildError {
    fn from(error: AvailableBlocksError<Types>) -> Self {
        match error {
            AvailableBlocksError::SignatureValidationFailed => {
                BuildError::Error("Signature validation failed in get_available_blocks".to_string())
            },
            AvailableBlocksError::RequestForAvailableViewThatHasAlreadyBeenDecided => {
                BuildError::Error(
                    "Request for available blocks for a view that has already been decided."
                        .to_string(),
                )
            },
            AvailableBlocksError::SigningBlockFailed(e) => {
                BuildError::Error(format!("Signing over block info failed: {:?}", e))
            },
            AvailableBlocksError::GetChannelForMatchingBuilderError(e) => e.into(),
            AvailableBlocksError::NoBlocksAvailable => {
                BuildError::Error("No blocks available".to_string())
            },
            AvailableBlocksError::ChannelUnexpectedlyClosed => {
                BuildError::Error("Channel unexpectedly closed".to_string())
            },
        }
    }
}

/// `ClaimBlockError` is an error enum that represents the class of possible
/// errors that can be returned when calling `claim_block` on a
/// `ProxyGlobalState`.  These errors are used for internal representations
/// for consistency and testing, and do not leak beyond the `ProxyGlobalState`
/// API.  As such, they intentionally do not implement traits for serialization.
#[derive(Debug)]
enum ClaimBlockError<Types: NodeType> {
    SignatureValidationFailed,
    SigningCommitmentFailed(
        <<Types as NodeType>::BuilderSignatureKey as BuilderSignatureKey>::SignError,
    ),
    BlockDataNotFound,
}

impl<Types: NodeType> From<ClaimBlockError<Types>> for BuildError {
    fn from(error: ClaimBlockError<Types>) -> Self {
        match error {
            ClaimBlockError::SignatureValidationFailed => {
                BuildError::Error("Signature validation failed in claim block".to_string())
            },
            ClaimBlockError::SigningCommitmentFailed(e) => {
                BuildError::Error(format!("Signing over builder commitment failed: {:?}", e))
            },
            ClaimBlockError::BlockDataNotFound => {
                BuildError::Error("Block data not found".to_string())
            },
        }
    }
}

#[derive(Debug)]
enum ClaimBlockHeaderInputError<Types: NodeType> {
    SignatureValidationFailed,
    BlockHeaderNotFound,
    FailedToSignFeeInfo(
        <<Types as NodeType>::BuilderSignatureKey as BuilderSignatureKey>::SignError,
    ),
}

impl<Types: NodeType> From<ClaimBlockHeaderInputError<Types>> for BuildError {
    fn from(error: ClaimBlockHeaderInputError<Types>) -> Self {
        match error {
            ClaimBlockHeaderInputError::SignatureValidationFailed => BuildError::Error(
                "Signature validation failed in claim block header input".to_string(),
            ),
            ClaimBlockHeaderInputError::BlockHeaderNotFound => {
                BuildError::Error("Block header not found".to_string())
            },
            ClaimBlockHeaderInputError::FailedToSignFeeInfo(e) => {
                BuildError::Error(format!("Failed to sign fee info: {:?}", e))
            },
        }
    }
}

impl<Types: NodeType> ProxyGlobalState<Types> {
    async fn available_blocks_implementation(
        &self,
        for_parent: &VidCommitment,
        view_number: u64,
        sender: Types::SignatureKey,
        signature: &<Types::SignatureKey as SignatureKey>::PureAssembledSignatureType,
    ) -> Result<Vec<AvailableBlockInfo<Types>>, AvailableBlocksError<Types>> {
        let starting_time = Instant::now();

        let state_id = BuilderStateId {
            parent_commitment: *for_parent,
            parent_view: Types::View::new(view_number),
        };

        // verify the signature
        if !sender.validate(signature, state_id.parent_commitment.as_ref()) {
            tracing::error!("Signature validation failed in get_available_blocks");
            return Err(AvailableBlocksError::SignatureValidationFailed);
        }

        tracing::info!("Requesting available blocks for {state_id}",);

        let view_num = state_id.parent_view;
        // check in the local spawned builder states
        // if it doesn't exist; there are three cases
        // 1) it has already been garbage collected (view < decide) and we should return an error
        // 2) it has not yet been created, and we should try to wait
        // 3) we missed the triggering event, and should use the BuilderState with the highest available view

        {
            // 1st case: Decide event received, and not bootstrapping.
            // If this `BlockBuilder` hasn't been reaped, it should have been.
            let global_state = self.global_state.read_arc().await;
            if view_num < global_state.last_garbage_collected_view_num
                && global_state.highest_view_num_builder_id.parent_view
                    != global_state.last_garbage_collected_view_num
            {
                tracing::warn!(
                    "Requesting for view {:?}, last decide-triggered cleanup on view {:?}, highest view num is {:?}",
                    view_num,
                    global_state.last_garbage_collected_view_num,
                    global_state.highest_view_num_builder_id.parent_view
                );
                return Err(AvailableBlocksError::RequestForAvailableViewThatHasAlreadyBeenDecided);
            }
        }

        let (response_sender, mut response_receiver) = unbounded_channel();
        let req_msg = RequestMessage {
            state_id: state_id.clone(),
            response_channel: response_sender,
        };
        let timeout_after = starting_time + self.max_api_waiting_time;
        let check_duration = self.max_api_waiting_time / 10;

        let time_to_wait_for_matching_builder = starting_time + self.max_api_waiting_time / 2;

        let mut sent = false;
        while Instant::now() < time_to_wait_for_matching_builder {
            // try to broadcast the request to the correct builder state
            let found_builder_state = {
                let global_state_read_lock_guard = self.global_state.read_arc().await;

                global_state_read_lock_guard
                    .spawned_builder_states
                    .get(&state_id)
                    .cloned()
            };

            if let Some(id_and_sender) = found_builder_state {
                tracing::info!(
                    "Got matching BlockBuilder for {state_id}, sending get_available_blocks request",
                );

                if let Err(e) = id_and_sender
                    .1
                    .broadcast(MessageType::RequestMessage(req_msg.clone()))
                    .await
                {
                    tracing::warn!("Error {e} sending get_available_blocks request for {state_id}",);
                }
                sent = true;
                break;
            }

            tracing::info!("Failed to get matching BlockBuilder for {state_id}, will try again",);
            sleep(check_duration).await;
        }

        if !sent {
            // broadcast the request to the best fallback builder state
            if let Err(e) = self
                .global_state
                .read_arc()
                .await
                .get_channel_for_matching_builder_or_highest_view_builder(&state_id)?
                .broadcast(MessageType::RequestMessage(req_msg.clone()))
                .await
            {
                tracing::warn!(
                    "Error {e} sending get_available_blocks request for parent {state_id}"
                );
            }
        }

        tracing::debug!("Waiting for response for get_available_blocks with parent {state_id}");

        let response_received = loop {
            match timeout(check_duration, response_receiver.recv()).await {
                Err(toe) => {
                    if Instant::now() >= timeout_after {
                        tracing::debug!(%toe, "Couldn't get available blocks in time for parent {state_id}");
                        // lookup into the builder_state_to_last_built_block, if it contains the result, return that otherwise return error
                        if let Some(last_built_block) = self
                            .global_state
                            .read_arc()
                            .await
                            .builder_state_to_last_built_block
                            .get(&state_id)
                        {
                            tracing::info!("Returning last built block for parent {state_id}");
                            break Ok(last_built_block.clone());
                        }
                        break Err(AvailableBlocksError::NoBlocksAvailable);
                    }
                    continue;
                },
                Ok(recv_attempt) => {
                    if recv_attempt.is_none() {
                        tracing::error!(
                            "Channel closed while getting available blocks for parent {state_id}"
                        );
                    }
                    break recv_attempt
                        .ok_or_else(|| AvailableBlocksError::ChannelUnexpectedlyClosed);
                },
            }
        };

        match response_received {
            Ok(response) => {
                let (pub_key, sign_key) = self.builder_keys.clone();
                // sign over the block info
                let signature_over_block_info =
                    <Types as NodeType>::BuilderSignatureKey::sign_block_info(
                        &sign_key,
                        response.block_size,
                        response.offered_fee,
                        &response.builder_hash,
                    )
                    .map_err(AvailableBlocksError::SigningBlockFailed)?;

                // insert the block info into local hashmap
                let initial_block_info = AvailableBlockInfo::<Types> {
                    block_hash: response.builder_hash.clone(),
                    block_size: response.block_size,
                    offered_fee: response.offered_fee,
                    signature: signature_over_block_info,
                    sender: pub_key.clone(),
                    _phantom: Default::default(),
                };
                tracing::info!(
                    "Sending available Block info response for {state_id} with block hash: {:?}",
                    response.builder_hash
                );
                Ok(vec![initial_block_info])
            },

            // We failed to get available blocks
            Err(e) => {
                tracing::debug!("Failed to get available blocks for parent {state_id}",);
                Err(e)
            },
        }
    }

    async fn claim_block_implementation(
        &self,
        block_hash: &BuilderCommitment,
        view_number: u64,
        sender: Types::SignatureKey,
        signature: &<<Types as NodeType>::SignatureKey as SignatureKey>::PureAssembledSignatureType,
    ) -> Result<AvailableBlockData<Types>, ClaimBlockError<Types>> {
        let block_id = BlockId {
            hash: block_hash.clone(),
            view: Types::View::new(view_number),
        };

        tracing::info!("Received request for claiming block {block_id}",);
        // verify the signature
        if !sender.validate(signature, block_id.hash.as_ref()) {
            tracing::error!("Signature validation failed in claim block");
            return Err(ClaimBlockError::SignatureValidationFailed);
        }
        let (pub_key, sign_key) = self.builder_keys.clone();

        let extracted_block_info_option = {
            // We store this write lock guard separately to make it explicit
            // that this will end up holding a lock for the duration of this
            // closure.
            //
            // Additionally, we clone the properties from the block_info that
            // end up being cloned if found anyway.  Since we know this already
            // we can perform the clone here to avoid holding the lock for
            // longer than needed.
            let mut global_state_write_lock_guard = self.global_state.write_arc().await;
            let block_info_some = global_state_write_lock_guard.blocks.get(&block_id);

            block_info_some.map(|block_info| {
                (
                    block_info.vid_trigger.clone(),
                    block_info.block_payload.clone(),
                    block_info.metadata.clone(),
                )
            })
        };

        if let Some((vid_trigger, block_payload, metadata)) = extracted_block_info_option {
            tracing::info!("Trying sending vid trigger info for {block_id}",);

            if let Some(trigger_writer) = vid_trigger.write().await.take() {
                tracing::info!("Sending vid trigger for {block_id}");
                let _ = trigger_writer.send(TriggerStatus::Start);
                tracing::info!("Sent vid trigger for {block_id}");
            }
            tracing::info!("Done Trying sending vid trigger info for {block_id}",);

            // sign over the builder commitment, as the proposer can computer it based on provide block_payload
            // and the metadata
            let response_block_hash = block_payload.builder_commitment(&metadata);
            let signature_over_builder_commitment =
                <Types as NodeType>::BuilderSignatureKey::sign_builder_message(
                    &sign_key,
                    response_block_hash.as_ref(),
                )
                .map_err(ClaimBlockError::SigningCommitmentFailed)?;

            let block_data = AvailableBlockData::<Types> {
                block_payload: block_payload.clone(),
                metadata: metadata.clone(),
                signature: signature_over_builder_commitment,
                sender: pub_key.clone(),
            };
            tracing::info!("Sending Claim Block data for {block_id}",);
            Ok(block_data)
        } else {
            tracing::warn!("Claim Block not found");
            Err(ClaimBlockError::BlockDataNotFound)
        }
    }

    async fn claim_block_header_input_implementation(
        &self,
        block_hash: &BuilderCommitment,
        view_number: u64,
        sender: Types::SignatureKey,
        signature: &<<Types as NodeType>::SignatureKey as SignatureKey>::PureAssembledSignatureType,
    ) -> Result<AvailableBlockHeaderInputV1<Types>, ClaimBlockHeaderInputError<Types>> {
        let id = BlockId {
            hash: block_hash.clone(),
            view: Types::View::new(view_number),
        };

        tracing::info!("Received request for claiming block header input for block {id}");
        // verify the signature
        if !sender.validate(signature, id.hash.as_ref()) {
            tracing::error!("Signature validation failed in claim block header input");
            return Err(ClaimBlockHeaderInputError::SignatureValidationFailed);
        }
        let (pub_key, sign_key) = self.builder_keys.clone();

        let extracted_block_info_option = {
            // We store this write lock guard separately to make it explicit
            // that this will end up holding a lock for the duration of this
            // closure.
            //
            // Additionally, we clone the properties from the block_info that
            // end up being cloned if found anyway.  Since we know this already
            // we can perform the clone here to avoid holding the lock for
            // longer than needed.
            let mut global_state_write_lock_guard = self.global_state.write_arc().await;
            let block_info_some = global_state_write_lock_guard.blocks.get(&id);

            block_info_some.map(|block_info| {
                (
                    block_info.metadata.clone(),
                    block_info.offered_fee,
                    block_info.truncated,
                )
            })
        };

        // TODO Add precompute back.
        if let Some((metadata, offered_fee, _)) = extracted_block_info_option {
            let signature_over_fee_info =
                Types::BuilderSignatureKey::sign_fee(&sign_key, offered_fee, &metadata)
                    .map_err(ClaimBlockHeaderInputError::FailedToSignFeeInfo)?;

            let response = AvailableBlockHeaderInputV1::<Types> {
                fee_signature: signature_over_fee_info,
                sender: pub_key.clone(),
            };
            tracing::info!("Sending Claim Block Header Input response for {id}",);
            Ok(response)
        } else {
            tracing::warn!("Claim Block Header Input not found");
            Err(ClaimBlockHeaderInputError::BlockHeaderNotFound)
        }
    }
}

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
    async fn available_blocks(
        &self,
        for_parent: &VidCommitment,
        view_number: u64,
        sender: Types::SignatureKey,
        signature: &<Types::SignatureKey as SignatureKey>::PureAssembledSignatureType,
    ) -> Result<Vec<AvailableBlockInfo<Types>>, BuildError> {
        Ok(self
            .available_blocks_implementation(for_parent, view_number, sender, signature)
            .await?)
    }

    async fn claim_block(
        &self,
        block_hash: &BuilderCommitment,
        view_number: u64,
        sender: Types::SignatureKey,
        signature: &<<Types as NodeType>::SignatureKey as SignatureKey>::PureAssembledSignatureType,
    ) -> Result<AvailableBlockData<Types>, BuildError> {
        Ok(self
            .claim_block_implementation(block_hash, view_number, sender, signature)
            .await?)
    }

    async fn claim_block_with_num_nodes(
        &self,
        block_hash: &BuilderCommitment,
        view_number: u64,
        sender: <Types as NodeType>::SignatureKey,
        signature: &<<Types as NodeType>::SignatureKey as SignatureKey>::PureAssembledSignatureType,
        num_nodes: usize,
    ) -> Result<AvailableBlockData<Types>, BuildError> {
        // Update the stored `num_nodes` with the given value, which will be used for VID computation.
        self.global_state.write_arc().await.num_nodes = num_nodes;

        self.claim_block(block_hash, view_number, sender, signature)
            .await
    }

    async fn claim_block_header_input(
        &self,
        block_hash: &BuilderCommitment,
        view_number: u64,
        sender: Types::SignatureKey,
        signature: &<<Types as NodeType>::SignatureKey as SignatureKey>::PureAssembledSignatureType,
    ) -> Result<AvailableBlockHeaderInputV1<Types>, BuildError> {
        Ok(self
            .claim_block_header_input_implementation(block_hash, view_number, sender, signature)
            .await?)
    }

    /// Returns the public key of the builder
    async fn builder_address(
        &self,
    ) -> Result<<Types as NodeType>::BuilderSignatureKey, BuildError> {
        Ok(self.builder_keys.0.clone())
    }
}

#[async_trait]
impl<Types: NodeType> AcceptsTxnSubmits<Types> for ProxyGlobalState<Types> {
    async fn submit_txns(
        &self,
        txns: Vec<<Types as NodeType>::Transaction>,
    ) -> Result<Vec<Commitment<<Types as NodeType>::Transaction>>, BuildError> {
        tracing::debug!(
            "Submitting {:?} transactions to the builder states{:?}",
            txns.len(),
            txns.iter().map(|txn| txn.commit()).collect::<Vec<_>>()
        );
        let response = self
            .global_state
            .read_arc()
            .await
            .submit_client_txns(txns.clone())
            .await;

        let pairs: Vec<(Commitment<<Types as NodeType>::Transaction>, Result<_, _>)> = (0..txns
            .len())
            .map(|i| (txns[i].commit(), response[i].clone()))
            .collect();
        let mut write_guard = self.global_state.write_arc().await;
        for (txn_commit, res) in pairs {
            if let Err(some) = res {
                write_guard
                    .set_txn_status(
                        txn_commit,
                        TransactionStatus::Rejected {
                            reason: some.to_string(),
                        },
                    )
                    .await?;
            } else {
                write_guard
                    .set_txn_status(txn_commit, TransactionStatus::Pending)
                    .await?;
            }
        }

        tracing::debug!(
            "Transaction submitted to the builder states, sending response: {:?}",
            response
        );

        // NOTE: ideally we want to respond with original Vec<Result>
        // instead of Result<Vec> not to loose any information,
        //  but this requires changes to builder API
        response.into_iter().collect()
    }

    async fn txn_status(
        &self,
        txn_hash: Commitment<<Types as NodeType>::Transaction>,
    ) -> Result<TransactionStatus, BuildError> {
        self.global_state
            .read_arc()
            .await
            .txn_status(txn_hash)
            .await
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

/*
Running Non-Permissioned Builder Service
*/
pub async fn run_non_permissioned_standalone_builder_service<
    Types: NodeType,
    Ver: StaticVersionType,
    S: Stream<Item = Event<Types>> + Unpin,
>(
    // sending a DA proposal from the hotshot to the builder states
    da_sender: BroadcastSender<MessageType<Types>>,

    // sending a Quorum proposal from the hotshot to the builder states
    quorum_sender: BroadcastSender<MessageType<Types>>,

    // sending a Decide event from the hotshot to the builder states
    decide_sender: BroadcastSender<MessageType<Types>>,

    // HotShot event stream
    hotshot_event_stream: S,

    // Global state
    global_state: Arc<RwLock<GlobalState<Types>>>,
) -> Result<(), anyhow::Error> {
    let tx_sender = {
        // This closure is likely unnecessary, but we want to play it safe
        // with our RWLocks.
        let global_state_read_lock_guard = global_state.read_arc().await;
        global_state_read_lock_guard.tx_sender.clone()
    };
    let mut hotshot_event_stream = std::pin::pin!(hotshot_event_stream);

    loop {
        let Some(event) = hotshot_event_stream.next().await else {
            anyhow::bail!("Event stream ended");
        };

        match event.event {
            EventType::Error { error } => {
                tracing::error!("Error event in HotShot: {:?}", error);
            },
            // tx event
            EventType::Transactions { transactions } => {
                let max_block_size = {
                    // This closure is likely unnecessary, but we want
                    // to play it safe with our RWLocks.
                    let global_state_read_lock_guard = global_state.read_arc().await;
                    global_state_read_lock_guard
                        .block_size_limits
                        .max_block_size
                };

                let response = handle_received_txns(
                    &tx_sender,
                    transactions.clone(),
                    TransactionSource::HotShot,
                    max_block_size,
                )
                .await;
                let pairs: Vec<(Commitment<<Types as NodeType>::Transaction>, Result<_, _>)> = (0
                    ..transactions.len())
                    .map(|i| (transactions[i].commit(), response[i].clone()))
                    .collect();
                let mut write_guard = global_state.write_arc().await;
                for (txn_commit, res) in pairs {
                    if let Err(some) = res {
                        write_guard
                            .set_txn_status(
                                txn_commit,
                                TransactionStatus::Rejected {
                                    reason: some.to_string(),
                                },
                            )
                            .await?;
                    } else {
                        write_guard
                            .set_txn_status(txn_commit, TransactionStatus::Pending)
                            .await?;
                    }
                }
            },
            // decide event
            EventType::Decide {
                block_size: _,
                leaf_chain,
                qc: _,
            } => {
                let latest_decide_view_num = leaf_chain[0].leaf.view_number();
                handle_decide_event(&decide_sender, latest_decide_view_num).await;
            },
            // DA proposal event
            EventType::DaProposal { proposal, sender } => {
                handle_da_event(&da_sender, Arc::new(proposal), sender).await;
            },
            // QC proposal event
            EventType::QuorumProposal { proposal, sender } => {
                // get the leader for current view
                handle_quorum_event(&quorum_sender, Arc::new(proposal), sender).await;
            },
            _ => {
                tracing::debug!("Unhandled event from Builder");
            },
        }
    }
}

/// [`HandleDaEventError`] represents the internal class of errors that can
/// occur when attempting to process an incoming da proposal event.  More
/// specifically these are the class of error that can be returned from
/// [`handle_da_event_implementation`].
#[derive(Debug)]
enum HandleDaEventError<Types: NodeType> {
    SignatureValidationFailed,
    BroadcastFailed(async_broadcast::SendError<MessageType<Types>>),
}

/// [`handle_da_event`] is a utility function that will attempt to broadcast the
/// given `da_proposal` to the given `da_channel_sender` if the given details
/// pass validation checks, and the [`BroadcastSender`] `da_channel_sender` is
/// still open.
async fn handle_da_event<Types: NodeType>(
    da_channel_sender: &BroadcastSender<MessageType<Types>>,
    da_proposal: Arc<Proposal<Types, DaProposal2<Types>>>,
    sender: <Types as NodeType>::SignatureKey,
) {
    // We're explicitly not inspecting this error, as this function is not
    // expected to return an error or any indication of an error.
    let _ = handle_da_event_implementation(da_channel_sender, da_proposal, sender).await;
}

/// [`handle_da_event_implementation`] is a utility function that will attempt
/// to broadcast the given `da_proposal` to the given `da_channel_sender` if the
/// given details pass all relevant checks.
///
/// There are only three conditions under which this will fail to send the
/// message via the given `da_channel_sender`, and they are all represented
/// via [`HandleDaEventError`]. They are as follows:
/// - [`HandleDaEventError::SignatureValidationFailed`]: The signature validation failed
/// - [`HandleDaEventError::BroadcastFailed`]: The broadcast failed as no receiver
///   is in place to receive the message
///
/// This function is the implementation for [`handle_da_event`].
async fn handle_da_event_implementation<Types: NodeType>(
    da_channel_sender: &BroadcastSender<MessageType<Types>>,
    da_proposal: Arc<Proposal<Types, DaProposal2<Types>>>,
    sender: <Types as NodeType>::SignatureKey,
) -> Result<(), HandleDaEventError<Types>> {
    tracing::debug!(
        "DaProposal: Leader: {:?} for the view: {:?}",
        sender,
        da_proposal.data.view_number
    );

    // get the encoded transactions hash
    let encoded_txns_hash = Sha256::digest(&da_proposal.data.encoded_transactions);
    // check if the sender is the leader and the signature is valid; if yes, broadcast the DA proposal

    if !sender.validate(&da_proposal.signature, &encoded_txns_hash) {
        tracing::error!(
            "Validation Failure on DaProposal for view {:?}: Leader: {:?}",
            da_proposal.data.view_number,
            sender
        );
        return Err(HandleDaEventError::SignatureValidationFailed);
    }

    let da_msg = DaProposalMessage::<Types> {
        proposal: da_proposal,
        sender,
    };

    let view_number = da_msg.proposal.data.view_number;
    tracing::debug!(
        "Sending DA proposal to the builder states for view {:?}",
        view_number
    );

    if let Err(e) = da_channel_sender
        .broadcast(MessageType::DaProposalMessage(da_msg))
        .await
    {
        tracing::warn!(
            "Error {e}, failed to send DA proposal to builder states for view {:?}",
            view_number
        );

        return Err(HandleDaEventError::BroadcastFailed(e));
    }

    Ok(())
}

/// [`HandleQuorumEventError`] represents the internal class of errors that can
/// occur when attempting to process an incoming quorum proposal event.  More
/// specifically these are the class of error that can be returned from
/// [`handle_quorum_event_implementation`].
#[derive(Debug)]
enum HandleQuorumEventError<Types: NodeType> {
    SignatureValidationFailed,
    BroadcastFailed(async_broadcast::SendError<MessageType<Types>>),
}

/// [`handle_quorum_event`] is a utility function that will attempt to broadcast the
/// given `quorum_proposal` to the given `quorum_channel_sender` if the given details
/// pass validation checks, and the [`BroadcastSender`] `quorum_channel_sender` is
/// still open.
async fn handle_quorum_event<Types: NodeType>(
    quorum_channel_sender: &BroadcastSender<MessageType<Types>>,
    quorum_proposal: Arc<Proposal<Types, QuorumProposalWrapper<Types>>>,
    sender: <Types as NodeType>::SignatureKey,
) {
    // We're explicitly not inspecting this error, as this function is not
    // expected to return an error or any indication of an error.
    let _ =
        handle_quorum_event_implementation(quorum_channel_sender, quorum_proposal, sender).await;
}

/// Utility function that will attempt to broadcast the given `quorum_proposal`
/// to the given `quorum_channel_sender` if the given details pass all relevant checks.
///
/// There are only three conditions under which this will fail to send the
/// message via the given `quorum_channel_sender`, and they are all represented
/// via [`HandleQuorumEventError`]. They are as follows:
/// - [`HandleQuorumEventError::SignatureValidationFailed`]: The signature validation failed
/// - [`HandleQuorumEventError::BroadcastFailed`]: The broadcast failed as no receiver
///   is in place to receive the message
///
/// This function is the implementation for [`handle_quorum_event`].
async fn handle_quorum_event_implementation<Types: NodeType>(
    quorum_channel_sender: &BroadcastSender<MessageType<Types>>,
    quorum_proposal: Arc<Proposal<Types, QuorumProposalWrapper<Types>>>,
    sender: <Types as NodeType>::SignatureKey,
) -> Result<(), HandleQuorumEventError<Types>> {
    tracing::debug!(
        "QuorumProposal: Leader: {:?} for the view: {:?}",
        sender,
        quorum_proposal.data.view_number()
    );

    let leaf = Leaf2::from_quorum_proposal(&quorum_proposal.data);

    if !sender.validate(&quorum_proposal.signature, leaf.commit().as_ref()) {
        tracing::error!(
            "Validation Failure on QuorumProposal for view {:?}: Leader for the current view: {:?}",
            quorum_proposal.data.view_number(),
            sender
        );
        return Err(HandleQuorumEventError::SignatureValidationFailed);
    }

    let quorum_msg = QuorumProposalMessage::<Types> {
        proposal: quorum_proposal,
        sender,
    };
    let view_number = quorum_msg.proposal.data.view_number();
    tracing::debug!(
        "Sending Quorum proposal to the builder states for view {:?}",
        view_number
    );

    if let Err(e) = quorum_channel_sender
        .broadcast(MessageType::QuorumProposalMessage(quorum_msg))
        .await
    {
        tracing::warn!(
            "Error {e}, failed to send Quorum proposal to builder states for view {:?}",
            view_number
        );
        return Err(HandleQuorumEventError::BroadcastFailed(e));
    }

    Ok(())
}

async fn handle_decide_event<Types: NodeType>(
    decide_channel_sender: &BroadcastSender<MessageType<Types>>,
    latest_decide_view_number: Types::View,
) {
    let decide_msg: DecideMessage<Types> = DecideMessage::<Types> {
        latest_decide_view_number,
    };
    tracing::debug!(
        "Sending Decide event to builder states for view {:?}",
        latest_decide_view_number
    );
    if let Err(e) = decide_channel_sender
        .broadcast(MessageType::DecideMessage(decide_msg))
        .await
    {
        tracing::warn!(
            "Error {e}, failed to send Decide event to builder states for view {:?}",
            latest_decide_view_number
        );
    }
}

#[derive(Debug)]
enum HandleReceivedTxnsError<Types: NodeType> {
    TransactionTooBig {
        estimated_length: u64,
        max_txn_len: u64,
    },

    TooManyTransactions,

    Internal(TrySendError<Arc<ReceivedTransaction<Types>>>),
}

impl<Types: NodeType> From<HandleReceivedTxnsError<Types>> for BuildError {
    fn from(error: HandleReceivedTxnsError<Types>) -> Self {
        match error {
            HandleReceivedTxnsError::TransactionTooBig {
                estimated_length,
                max_txn_len,
            } => BuildError::Error(format!("Transaction too big (estimated length {estimated_length}, currently accepting <= {max_txn_len})")),
            HandleReceivedTxnsError::TooManyTransactions => BuildError::Error("Too many transactions".to_owned()),
            HandleReceivedTxnsError::Internal(err) => BuildError::Error(format!("Internal error when submitting transaction: {}", err)),
        }
    }
}

impl<Types: NodeType> From<TrySendError<Arc<ReceivedTransaction<Types>>>>
    for HandleReceivedTxnsError<Types>
{
    fn from(err: TrySendError<Arc<ReceivedTransaction<Types>>>) -> Self {
        match err {
            TrySendError::Full(_) => HandleReceivedTxnsError::TooManyTransactions,
            err => HandleReceivedTxnsError::Internal(err),
        }
    }
}

/// Utility function that will take the given list
/// of transactions, `txns`, wraps them in a [`ReceivedTransaction`] struct
/// and attempt to broadcast them to the given transaction [`BroadcastSender`]
/// `tx_sender`. The broadcast itself it a non-blocking operation, and any
/// failures of the broadcast are collected into the returned vector
/// of [Result]s.
///
/// There is also a `max_txn_len` parameter that is used to check to ensure
/// that transactions that exceed this threshold will also not be broadcasted.
pub(crate) async fn handle_received_txns<Types: NodeType>(
    tx_sender: &BroadcastSender<Arc<ReceivedTransaction<Types>>>,
    txns: Vec<Types::Transaction>,
    source: TransactionSource,
    max_txn_len: u64,
) -> Vec<Result<Commitment<<Types as NodeType>::Transaction>, BuildError>> {
    HandleReceivedTxns::new(tx_sender.clone(), txns, source, max_txn_len)
        .map(|res| res.map_err(Into::into))
        .collect()
}

/// `HandleReceivedTxns` is a struct that is used to handle the processing of
/// the function [`handle_received_txns`].  In order to avoid the need to
/// double allocate a [Vec] from processing these entries, this struct exists
/// to be processed as an [Iterator] instead.
struct HandleReceivedTxns<Types: NodeType> {
    tx_sender: BroadcastSender<Arc<ReceivedTransaction<Types>>>,
    txns: Vec<Types::Transaction>,
    source: TransactionSource,
    max_txn_len: u64,
    offset: usize,
    txns_length: usize,
    time_in: Instant,
}

impl<Types: NodeType> HandleReceivedTxns<Types> {
    fn new(
        tx_sender: BroadcastSender<Arc<ReceivedTransaction<Types>>>,
        txns: Vec<Types::Transaction>,
        source: TransactionSource,
        max_txn_len: u64,
    ) -> Self {
        let txns_length = txns.len();

        Self {
            tx_sender,
            txns,
            source,
            max_txn_len,
            offset: 0,
            txns_length,
            time_in: Instant::now(),
        }
    }
}

impl<Types: NodeType> Iterator for HandleReceivedTxns<Types>
where
    Types::Transaction: Transaction,
{
    type Item =
        Result<Commitment<<Types as NodeType>::Transaction>, HandleReceivedTxnsError<Types>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.txns.is_empty() {
            return None;
        }

        if self.offset >= self.txns_length {
            return None;
        }

        let offset = self.offset;
        // increment the offset so we can ensure we're making progress;
        self.offset += 1;

        let tx = self.txns[offset].clone();
        let commit = tx.commit();
        // This is a rough estimate, but we don't have any other way to get real
        // encoded transaction length. Luckily, this being roughly proportional
        // to encoded length is enough, because we only use this value to estimate
        // our limitations on computing the VID in time.
        let len = tx.minimum_block_size();
        let max_txn_len = self.max_txn_len;
        if len > max_txn_len {
            tracing::warn!(%commit, %len, %max_txn_len, "Transaction too big");
            return Some(Err(HandleReceivedTxnsError::TransactionTooBig {
                estimated_length: len,
                max_txn_len: self.max_txn_len,
            }));
        }

        let res = self
            .tx_sender
            .try_broadcast(Arc::new(ReceivedTransaction {
                tx,
                source: self.source.clone(),
                commit,
                time_in: self.time_in,
                len,
            }))
            .inspect(|val| {
                if let Some(evicted_txn) = val {
                    tracing::warn!(
                        "Overflow mode enabled, transaction {} evicted",
                        evicted_txn.commit
                    );
                }
            })
            .map(|_| commit)
            .inspect_err(|err| {
                tracing::warn!("Failed to broadcast txn with commit {:?}: {}", commit, err);
            })
            .map_err(HandleReceivedTxnsError::from);

        Some(res)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (
            self.txns_length - self.offset,
            Some(self.txns.capacity() - self.offset),
        )
    }
}

#[cfg(test)]
mod test {
    use std::{sync::Arc, time::Duration};

    use async_lock::RwLock;
    use committable::{Commitment, Committable};
    use futures::StreamExt;
    use hotshot::{
        traits::BlockPayload,
        types::{BLSPubKey, SignatureKey},
    };
    use hotshot_builder_api::{
        v0_1::data_source::AcceptsTxnSubmits,
        v0_2::{block_info::AvailableBlockInfo, builder::TransactionStatus},
    };
    use hotshot_example_types::{
        block_types::{TestBlockPayload, TestMetadata, TestTransaction},
        node_types::{TestTypes, TestVersions},
        state_types::{TestInstanceState, TestValidatedState},
    };
    use hotshot_types::{
        data::{
            vid_commitment, DaProposal2, EpochNumber, Leaf, Leaf2, QuorumProposal2,
            QuorumProposalWrapper, ViewNumber,
        },
        message::Proposal,
        simple_certificate::QuorumCertificate2,
        traits::{
            block_contents::Transaction,
            node_implementation::{ConsensusTime, Versions},
            signature_key::BuilderSignatureKey,
        },
        utils::{BuilderCommitment, EpochTransitionIndicator},
    };
    use marketplace_builder_shared::{
        block::{BlockId, BuilderStateId, ParentBlockReferences},
        testing::constants::{
            TEST_MAX_BLOCK_SIZE_INCREMENT_PERIOD, TEST_MAX_TX_NUM,
            TEST_NUM_NODES_IN_VID_COMPUTATION, TEST_PROTOCOL_MAX_BLOCK_SIZE,
        },
    };
    use sha2::{Digest, Sha256};
    use tokio::{
        spawn,
        sync::{mpsc::unbounded_channel, oneshot},
    };
    use vbs::version::StaticVersionType;

    use super::{
        handle_da_event_implementation, handle_quorum_event_implementation, AvailableBlocksError,
        BlockInfo, ClaimBlockError, ClaimBlockHeaderInputError, GlobalState, HandleDaEventError,
        HandleQuorumEventError, HandleReceivedTxns, ProxyGlobalState,
    };
    use crate::{
        builder_state::{
            BuildBlockInfo, MessageType, RequestMessage, ResponseMessage, TransactionSource,
            TriggerStatus,
        },
        service::{BlockSizeLimits, HandleReceivedTxnsError},
        testing::finalization_test::{
            process_available_blocks_round, progress_round_with_available_block_info,
            progress_round_without_available_block_info, setup_builder_for_test,
        },
    };

    /// A const number on `max_tx_len` to be used consistently spanning all the tests
    /// It is set to 1 as current estimation on `TestTransaction` is 1
    const TEST_MAX_TX_LEN: u64 = 1;

    // GlobalState Tests

    // GlobalState::new Tests

    /// This test checks a [GlobalState] created from [GlobalState::new] has
    /// the appropriate values stored within it.
    #[tokio::test]
    async fn test_global_state_new() {
        let (bootstrap_sender, _) = async_broadcast::broadcast(10);
        let (tx_sender, _) = async_broadcast::broadcast(10);
        let parent_commit = vid_commitment::<TestVersions>(
            &[],
            &[],
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            <TestVersions as Versions>::Base::VERSION,
        );
        let state = GlobalState::<TestTypes>::new(
            bootstrap_sender,
            tx_sender,
            parent_commit,
            ViewNumber::new(1),
            ViewNumber::new(2),
            TEST_MAX_BLOCK_SIZE_INCREMENT_PERIOD,
            TEST_PROTOCOL_MAX_BLOCK_SIZE,
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            TEST_MAX_TX_NUM,
        );

        assert_eq!(state.blocks.len(), 0, "The blocks LRU should be empty");

        let builder_state_id = BuilderStateId {
            parent_commitment: parent_commit,
            parent_view: ViewNumber::new(1),
        };

        // There should be a single entry within the spawned_builder_states,
        // and it should be the one that was just created.
        assert_eq!(
            state.spawned_builder_states.len(),
            1,
            "There should be a single entry in the spawned builder states hashmap"
        );

        assert!(state.spawned_builder_states.contains_key(&builder_state_id), "The spawned builder states should contain an entry with the bootstrapped parameters passed into new");

        assert!(!state.spawned_builder_states.contains_key(&BuilderStateId { parent_commitment: parent_commit, parent_view: ViewNumber::new(0) }), "The spawned builder states should not contain any other entry, as such it should not contain any entry with a higher view number, but the same parent commit");

        // We can't compare the Senders directly

        assert_eq!(
            state.last_garbage_collected_view_num,
            ViewNumber::new(2),
            "The last garbage collected view number should be the one passed into new"
        );

        assert_eq!(
            state.builder_state_to_last_built_block.len(),
            0,
            "The builder state to last built block should be empty"
        );

        assert_eq!(
            state.highest_view_num_builder_id, builder_state_id,
            "The highest view number builder id should be the bootstrapped build state id"
        );

        assert_eq!(
            state.block_size_limits.protocol_max_block_size, TEST_PROTOCOL_MAX_BLOCK_SIZE,
            "The protocol max block size should be the one passed into new"
        );

        assert_eq!(
            state.block_size_limits.max_block_size, state.block_size_limits.protocol_max_block_size,
            "The max block size should be initialized to protocol max block size"
        );
    }

    // GlobalState::register_builder_state Tests

    /// This test checks that the [GlobalState::register_builder_state] function
    /// will correctly register a new builder state, and that the highest view
    /// number builder id will be updated to the new builder state id.
    /// Additionally, it will check that the spawned builder states hashmap
    /// will contain the new builder state id.
    #[tokio::test]
    async fn test_global_state_register_builder_state_different_states() {
        let (bootstrap_sender, _) = async_broadcast::broadcast(10);
        let (tx_sender, _) = async_broadcast::broadcast(10);
        let parent_commit = vid_commitment::<TestVersions>(
            &[],
            &[],
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            <TestVersions as Versions>::Base::VERSION,
        );
        let mut state = GlobalState::<TestTypes>::new(
            bootstrap_sender,
            tx_sender,
            parent_commit,
            ViewNumber::new(0),
            ViewNumber::new(0),
            TEST_MAX_BLOCK_SIZE_INCREMENT_PERIOD,
            TEST_PROTOCOL_MAX_BLOCK_SIZE,
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            TEST_MAX_TX_NUM,
        );

        {
            let (req_sender, _) = async_broadcast::broadcast(10);
            let builder_state_id = BuilderStateId {
                parent_commitment: parent_commit,
                parent_view: ViewNumber::new(5),
            };

            state.register_builder_state(
                builder_state_id.clone(),
                ParentBlockReferences {
                    view_number: ViewNumber::new(5),
                    vid_commitment: parent_commit,
                    leaf_commit: Commitment::from_raw([0; 32]),
                    builder_commitment: BuilderCommitment::from_bytes([]),
                    // Unused in old legacy builder:
                    last_nonempty_view: None,
                    tx_count: 0,
                },
                req_sender.clone(),
            );

            assert_eq!(
                state.spawned_builder_states.len(),
                2,
                "The spawned_builder_states should now have 2 elements in it"
            );
            assert_eq!(
                state.highest_view_num_builder_id, builder_state_id,
                "The highest view number builder id should now be the one that was just registered"
            );
            assert!(
                state.spawned_builder_states.contains_key(&builder_state_id),
                "The spawned builder states should contain the new builder state id"
            );
        };

        {
            let (req_sender, _) = async_broadcast::broadcast(10);
            let builder_state_id = BuilderStateId {
                parent_commitment: parent_commit,
                parent_view: ViewNumber::new(6),
            };

            state.register_builder_state(
                builder_state_id.clone(),
                ParentBlockReferences {
                    view_number: ViewNumber::new(6),
                    vid_commitment: parent_commit,
                    leaf_commit: Commitment::from_raw([0; 32]),
                    builder_commitment: BuilderCommitment::from_bytes([]),
                    // Unused in old legacy builder:
                    last_nonempty_view: None,
                    tx_count: 0,
                },
                req_sender.clone(),
            );

            assert_eq!(
                state.spawned_builder_states.len(),
                3,
                "The spawned_builder_states should now have 3 elements in it"
            );
            assert_eq!(
                state.highest_view_num_builder_id, builder_state_id,
                "The highest view number builder id should now be the one that was just registered"
            );
            assert!(
                state.spawned_builder_states.contains_key(&builder_state_id),
                "The spawned builder states should contain the new builder state id"
            );
        };
    }

    /// This test checks that the register_builder_state method will overwrite
    /// the previous sender in the `spawned_builder_states` hashmap if the same
    /// `BuilderStateId` is used to register a new sender.
    ///
    /// It also demonstrates that doing this will drop the previous sender,
    /// effectively closing it if it is the only reference to it.
    #[tokio::test]
    async fn test_global_state_register_builder_state_same_builder_state_id() {
        let (bootstrap_sender, _) = async_broadcast::broadcast(10);
        let (tx_sender, _) = async_broadcast::broadcast(10);
        let parent_commit = vid_commitment::<TestVersions>(
            &[],
            &[],
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            <TestVersions as Versions>::Base::VERSION,
        );
        let mut state = GlobalState::<TestTypes>::new(
            bootstrap_sender,
            tx_sender,
            parent_commit,
            ViewNumber::new(0),
            ViewNumber::new(0),
            TEST_MAX_BLOCK_SIZE_INCREMENT_PERIOD,
            TEST_PROTOCOL_MAX_BLOCK_SIZE,
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            TEST_MAX_TX_NUM,
        );

        let mut req_receiver_1 = {
            let (req_sender, req_receiver) = async_broadcast::broadcast(10);
            let builder_state_id = BuilderStateId {
                parent_commitment: parent_commit,
                parent_view: ViewNumber::new(5),
            };

            state.register_builder_state(
                builder_state_id.clone(),
                ParentBlockReferences {
                    view_number: ViewNumber::new(5),
                    vid_commitment: parent_commit,
                    leaf_commit: Commitment::from_raw([0; 32]),
                    builder_commitment: BuilderCommitment::from_bytes([]),
                    // Unused in old legacy builder:
                    last_nonempty_view: None,
                    tx_count: 0,
                },
                req_sender.clone(),
            );

            assert_eq!(
                state.spawned_builder_states.len(),
                2,
                "The spawned_builder_states should now have 2 elements in it"
            );
            assert_eq!(
                state.highest_view_num_builder_id, builder_state_id,
                "The highest view number builder id should now be the one that was just registered"
            );

            req_receiver
        };

        let mut req_receiver_2 = {
            let (req_sender, req_receiver) = async_broadcast::broadcast(10);
            let builder_state_id = BuilderStateId {
                parent_commitment: parent_commit,
                parent_view: ViewNumber::new(5),
            };

            // This is the same BuilderStateId as the previous one, so it should
            // replace the previous one.  Which means that the previous one
            // may no longer be published to.
            state.register_builder_state(
                builder_state_id.clone(),
                ParentBlockReferences {
                    view_number: ViewNumber::new(5),
                    vid_commitment: parent_commit,
                    leaf_commit: Commitment::from_raw([0; 32]),
                    builder_commitment: BuilderCommitment::from_bytes([]),
                    // Unused in old legacy builder:
                    last_nonempty_view: None,
                    tx_count: 0,
                },
                req_sender.clone(),
            );

            assert_eq!(
                state.spawned_builder_states.len(),
                2,
                "The spawned_builder_states should still have 2 elements in it"
            );
            assert_eq!(state.highest_view_num_builder_id, builder_state_id, "The highest view number builder id should still be the one that was just registered");

            req_receiver
        };

        {
            let builder_state_id = BuilderStateId {
                parent_commitment: parent_commit,
                parent_view: ViewNumber::new(5),
            };

            let req_id_and_sender = state.spawned_builder_states.get(&builder_state_id).unwrap();
            let (response_sender, _) = unbounded_channel();

            assert!(
                req_id_and_sender
                    .1
                    .broadcast(MessageType::RequestMessage(RequestMessage {
                        state_id: builder_state_id,
                        response_channel: response_sender,
                    }))
                    .await
                    .is_ok(),
                "This should be able to send a Message through the sender"
            );
        }

        // The first receiver should have been replaced, so we won't get any
        // results from it.

        assert!(
            req_receiver_1.recv().await.is_err(),
            "This first receiver should be closed"
        );
        assert!(
            req_receiver_2.recv().await.is_ok(),
            "The second receiver should receive a message"
        );
    }

    /// This test checks that the register_builder_state method will only
    /// update the highest_view_num_builder_id if the new [BuilderStateId] has
    /// a higher view number than the current highest_view_num_builder_id.
    #[tokio::test]
    async fn test_global_state_register_builder_state_decrementing_builder_state_ids() {
        let (bootstrap_sender, _) = async_broadcast::broadcast(10);
        let (tx_sender, _) = async_broadcast::broadcast(10);
        let parent_commit = vid_commitment::<TestVersions>(
            &[],
            &[],
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            <TestVersions as Versions>::Base::VERSION,
        );
        let mut state = GlobalState::<TestTypes>::new(
            bootstrap_sender,
            tx_sender,
            parent_commit,
            ViewNumber::new(0),
            ViewNumber::new(0),
            TEST_MAX_BLOCK_SIZE_INCREMENT_PERIOD,
            TEST_PROTOCOL_MAX_BLOCK_SIZE,
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            TEST_MAX_TX_NUM,
        );

        {
            let (req_sender, _) = async_broadcast::broadcast(10);
            let builder_state_id = BuilderStateId {
                parent_commitment: parent_commit,
                parent_view: ViewNumber::new(6),
            };

            state.register_builder_state(
                builder_state_id.clone(),
                ParentBlockReferences {
                    view_number: ViewNumber::new(6),
                    vid_commitment: parent_commit,
                    leaf_commit: Commitment::from_raw([0; 32]),
                    builder_commitment: BuilderCommitment::from_bytes([]),
                    // Unused in old legacy builder:
                    last_nonempty_view: None,
                    tx_count: 0,
                },
                req_sender.clone(),
            );

            assert_eq!(
                state.spawned_builder_states.len(),
                2,
                "The spawned_builder_states should now have 2 elements in it"
            );
            assert_eq!(
                state.highest_view_num_builder_id, builder_state_id,
                "The highest view number builder id should now be the one that was just registered"
            );
            assert!(
                state.spawned_builder_states.contains_key(&builder_state_id),
                "The spawned builder states should contain the new builder state id"
            );
        };

        {
            let (req_sender, _) = async_broadcast::broadcast(10);
            let builder_state_id = BuilderStateId {
                parent_commitment: parent_commit,
                parent_view: ViewNumber::new(5),
            };

            state.register_builder_state(
                builder_state_id.clone(),
                ParentBlockReferences {
                    view_number: ViewNumber::new(5),
                    vid_commitment: parent_commit,
                    leaf_commit: Commitment::from_raw([0; 32]),
                    builder_commitment: BuilderCommitment::from_bytes([]),
                    // Unused in old legacy builder:
                    last_nonempty_view: None,
                    tx_count: 0,
                },
                req_sender.clone(),
            );

            assert_eq!(
                state.spawned_builder_states.len(),
                3,
                "The spawned_builder_states should now have 3 elements in it"
            );
            assert_eq!(
                state.highest_view_num_builder_id,
                BuilderStateId {
                    parent_commitment: parent_commit,
                    parent_view: ViewNumber::new(6)
                },
                "The highest view number builder id should now be the one that was just registered"
            );
            assert!(
                state.spawned_builder_states.contains_key(&builder_state_id),
                "The spawned builder states should contain the new builder state id"
            );
        };
    }

    // GlobalState::update_global_state Tests

    /// This test checks that the update_global_state method will correctly
    /// update the LRU blocks cache and the builder_state_to_last_built_block
    /// hashmap with values derived from the parameters passed into the method.
    ///
    /// The assumption behind this test is that the values being stored were
    /// not being stored previously.
    #[tokio::test]
    async fn test_global_state_update_global_state_success() {
        let (bootstrap_sender, _) = async_broadcast::broadcast(10);
        let (tx_sender, _) = async_broadcast::broadcast(10);
        let parent_commit = vid_commitment::<TestVersions>(
            &[],
            &[],
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            <TestVersions as Versions>::Base::VERSION,
        );
        let mut state = GlobalState::<TestTypes>::new(
            bootstrap_sender,
            tx_sender,
            parent_commit,
            ViewNumber::new(0),
            ViewNumber::new(0),
            TEST_MAX_BLOCK_SIZE_INCREMENT_PERIOD,
            TEST_PROTOCOL_MAX_BLOCK_SIZE,
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            TEST_MAX_TX_NUM,
        );

        let new_parent_commit =
            vid_commitment::<TestVersions>(&[], &[], 9, <TestVersions as Versions>::Base::VERSION);
        let new_view_num = ViewNumber::new(1);
        let builder_state_id = BuilderStateId {
            parent_commitment: new_parent_commit,
            parent_view: new_view_num,
        };

        let builder_hash_1 = BuilderCommitment::from_bytes([1, 2, 3, 4]);
        let block_id = BlockId {
            hash: builder_hash_1,
            view: new_view_num,
        };

        let (vid_trigger_sender, vid_trigger_receiver) = oneshot::channel();
        let (block_payload, metadata) =
            <TestBlockPayload as BlockPayload<TestTypes>>::from_transactions(
                vec![TestTransaction::new(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10])],
                &TestValidatedState::default(),
                &TestInstanceState::default(),
            )
            .await
            .unwrap();
        let offered_fee = 64u64;
        let block_size = 64u64;
        let truncated = false;

        let build_block_info = BuildBlockInfo {
            id: block_id.clone(),
            block_size,
            offered_fee,
            block_payload: block_payload.clone(),
            metadata,
            vid_trigger: vid_trigger_sender,
            truncated,
        };

        let builder_hash_2 = BuilderCommitment::from_bytes([2, 3, 4, 5]);
        let response_msg = ResponseMessage {
            builder_hash: builder_hash_2.clone(),
            block_size: 32,
            offered_fee: 128,
        };

        // Now that every object is prepared and setup for storage, we can
        // test the `update_global_state` method.

        // `update_global_state` has not return value from its method, so can
        // only inspect its "success" based on the mutation of the state object.
        state.update_global_state(builder_state_id.clone(), build_block_info, response_msg);

        // two things should be adjusted by `update_global_state`:
        // - state.blocks
        // - state.builder_state_to_last_built_block

        // start with blocks

        assert_eq!(
            state.blocks.len(),
            1,
            "The blocks LRU should have a single entry"
        );

        let retrieved_block_info = state.blocks.get(&block_id);
        assert!(
            retrieved_block_info.is_some(),
            "Retrieval of the block id should result is a valid block info data"
        );

        let retrieved_block_info = retrieved_block_info.unwrap();

        assert_eq!(
            retrieved_block_info.block_payload, block_payload,
            "The block payloads should match"
        );
        assert_eq!(
            retrieved_block_info.metadata, metadata,
            "The metadata should match"
        );
        assert_eq!(
            retrieved_block_info.offered_fee, offered_fee,
            "The offered fee should match"
        );
        assert_eq!(
            retrieved_block_info.truncated, truncated,
            "The truncated flag should match"
        );

        {
            // This ensures that the vid_trigger that is stored is still the
            // same, or links to the vid_trigger_receiver that we submitted.
            let mut vid_trigger_write_lock_guard =
                retrieved_block_info.vid_trigger.write_arc().await;
            if let Some(vid_trigger_sender) = vid_trigger_write_lock_guard.take() {
                vid_trigger_sender
                    .send(TriggerStatus::Start)
                    .expect("vid_trigger_sender failed");
            }

            match vid_trigger_receiver.await {
                Ok(TriggerStatus::Start) => {
                    // This is expected
                },
                _ => {
                    panic!("did not receive TriggerStatus::Start from vid_trigger_receiver as expected");
                },
            }
        }

        // TODO Add precompute back.

        // finish with builder_state_to_last_built_block

        assert_eq!(
            state.builder_state_to_last_built_block.len(),
            1,
            "The builder state to last built block should have a single entry"
        );

        let last_built_block = state
            .builder_state_to_last_built_block
            .get(&builder_state_id);

        assert!(
            last_built_block.is_some(),
            "The last built block should be retrievable"
        );

        let last_built_block = last_built_block.unwrap();

        assert_eq!(
            last_built_block.builder_hash, builder_hash_2,
            "The last built block id should match the block id"
        );

        assert_eq!(
            last_built_block.block_size, 32,
            "The last built block size should match the response message"
        );

        assert_eq!(
            last_built_block.offered_fee, 128,
            "The last built block offered fee should match the response message"
        );
    }

    /// This test demonstrates the replacement behavior of the the
    /// `update_global_state` method.
    ///
    /// When given a `BuilderStateId` that already exists in the `blocks` LRU,
    /// and the `builder_state_to_last_built_block` hashmap, the method will
    /// replace the values in the `builder_state_to_last_built_block` hashmap,
    /// and it will also replace the entry in the `block`s LRU.
    #[tokio::test]
    async fn test_global_state_update_global_state_replacement() {
        let (bootstrap_sender, _) = async_broadcast::broadcast(10);
        let (tx_sender, _) = async_broadcast::broadcast(10);
        let parent_commit = vid_commitment::<TestVersions>(
            &[],
            &[],
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            <TestVersions as Versions>::Base::VERSION,
        );
        let mut state = GlobalState::<TestTypes>::new(
            bootstrap_sender,
            tx_sender,
            parent_commit,
            ViewNumber::new(0),
            ViewNumber::new(0),
            TEST_MAX_BLOCK_SIZE_INCREMENT_PERIOD,
            TEST_PROTOCOL_MAX_BLOCK_SIZE,
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            TEST_MAX_TX_NUM,
        );

        let new_parent_commit =
            vid_commitment::<TestVersions>(&[], &[], 9, <TestVersions as Versions>::Base::VERSION);
        let new_view_num = ViewNumber::new(1);
        let builder_state_id = BuilderStateId {
            parent_commitment: new_parent_commit,
            parent_view: new_view_num,
        };

        let builder_hash = BuilderCommitment::from_bytes([1, 2, 3, 4]);
        let block_id_1 = BlockId {
            hash: builder_hash.clone(),
            view: new_view_num,
        };
        let (vid_trigger_sender_1, vid_trigger_receiver_1) = oneshot::channel();
        let (block_payload_1, metadata_1) =
            <TestBlockPayload as BlockPayload<TestTypes>>::from_transactions(
                vec![TestTransaction::new(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10])],
                &TestValidatedState::default(),
                &TestInstanceState::default(),
            )
            .await
            .unwrap();
        let offered_fee_1 = 64u64;
        let block_size_1 = 64u64;
        let truncated_1 = false;
        let build_block_info_1 = BuildBlockInfo {
            id: block_id_1.clone(),
            block_size: block_size_1,
            offered_fee: offered_fee_1,
            block_payload: block_payload_1.clone(),
            metadata: metadata_1,
            vid_trigger: vid_trigger_sender_1,
            truncated: truncated_1,
        };
        let response_msg_1 = ResponseMessage {
            builder_hash: builder_hash.clone(),
            block_size: block_size_1,
            offered_fee: offered_fee_1,
        };

        // Now that every object is prepared and setup for storage, we can
        // test the `update_global_state` method.

        // `update_global_state` has no return value from its method, so we can
        // only inspect its "success" based on the mutation of the state object.
        state.update_global_state(builder_state_id.clone(), build_block_info_1, response_msg_1);

        // We're going to enter another update_global_state_entry with the same
        // builder_state_id, but with different values for the block info and
        // response message.  This should highlight that the values get replaced
        // in this update.

        let block_id_2 = BlockId {
            hash: builder_hash.clone(),
            view: new_view_num,
        };
        let (vid_trigger_sender_2, vid_trigger_receiver_2) = oneshot::channel();
        let (block_payload_2, metadata_2) =
            <TestBlockPayload as BlockPayload<TestTypes>>::from_transactions(
                vec![TestTransaction::new(vec![2, 3, 4, 5, 6, 7, 8, 9, 10, 11])],
                &TestValidatedState::default(),
                &TestInstanceState::default(),
            )
            .await
            .unwrap();
        let offered_fee_2 = 16u64;
        let block_size_2 = 32u64;
        let truncated_2 = true;
        let build_block_info_2 = BuildBlockInfo {
            id: block_id_2.clone(),
            block_size: block_size_2,
            offered_fee: offered_fee_2,
            block_payload: block_payload_2.clone(),
            metadata: metadata_2,
            vid_trigger: vid_trigger_sender_2,
            truncated: truncated_2,
        };
        let response_msg_2: ResponseMessage = ResponseMessage {
            builder_hash: builder_hash.clone(),
            block_size: block_size_2,
            offered_fee: offered_fee_2,
        };

        // two things should be adjusted by `update_global_state`:
        // When given the same build_state_ids.
        state.update_global_state(builder_state_id.clone(), build_block_info_2, response_msg_2);

        // start with blocks

        assert_eq!(
            state.blocks.len(),
            1,
            "The blocks LRU should have a single entry"
        );

        let retrieved_block_info = state.blocks.get(&block_id_2);
        assert!(
            retrieved_block_info.is_some(),
            "Retrieval of the block id should result is a valid block info data"
        );

        let retrieved_block_info = retrieved_block_info.unwrap();

        assert_eq!(
            retrieved_block_info.block_payload, block_payload_2,
            "The block payloads should match"
        );
        assert_ne!(
            retrieved_block_info.block_payload, block_payload_1,
            "The block payloads should not match"
        );
        assert_eq!(
            retrieved_block_info.metadata, metadata_2,
            "The metadata should match"
        );
        assert_eq!(
            retrieved_block_info.metadata, metadata_1,
            "The metadata should match"
        );
        // TestMetadata will always match

        assert_eq!(
            retrieved_block_info.offered_fee, offered_fee_2,
            "The offered fee should match"
        );
        assert_ne!(
            retrieved_block_info.offered_fee, offered_fee_1,
            "The offered fee should not match"
        );
        assert_eq!(
            retrieved_block_info.truncated, truncated_2,
            "The truncated flag should match"
        );
        assert_ne!(
            retrieved_block_info.truncated, truncated_1,
            "The truncated flag should not match"
        );

        {
            // This ensures that the vid_trigger that is stored is still the
            // same, or links to the vid_trigger_receiver that we submitted.
            let mut vid_trigger_write_lock_guard =
                retrieved_block_info.vid_trigger.write_arc().await;
            if let Some(vid_trigger_sender) = vid_trigger_write_lock_guard.take() {
                vid_trigger_sender
                    .send(TriggerStatus::Start)
                    .expect("vid_trigger_sender failed");
            }

            match vid_trigger_receiver_2.await {
                Ok(TriggerStatus::Start) => {
                    // This is expected
                },
                _ => {
                    panic!("did not receive TriggerStatus::Start from vid_trigger_receiver as expected");
                },
            }

            assert!(
                vid_trigger_receiver_1.await.is_err(),
                "This should not receive anything from vid_trigger_receiver_1"
            );
        }

        // TODO Add precompute back.

        // finish with builder_state_to_last_built_block

        assert_eq!(
            state.builder_state_to_last_built_block.len(),
            1,
            "The builder state to last built block should have a single entry"
        );

        let last_built_block = state
            .builder_state_to_last_built_block
            .get(&builder_state_id);

        assert!(
            last_built_block.is_some(),
            "The last built block should be retrievable"
        );

        let last_built_block = last_built_block.unwrap();

        assert_eq!(
            last_built_block.builder_hash, builder_hash,
            "The last built block id should match the block id"
        );

        assert_eq!(
            last_built_block.block_size, block_size_2,
            "The last built block size should match the response message"
        );
        assert_ne!(
            last_built_block.block_size, block_size_1,
            "The last built block size should not match the previous block size"
        );

        assert_eq!(
            last_built_block.offered_fee, offered_fee_2,
            "The last built block offered fee should match the response message"
        );
        assert_ne!(
            last_built_block.offered_fee, offered_fee_1,
            "The last built block offered fee should not match the previous block offered fee"
        );
    }

    // GlobalState::remove_handles Tests

    /// This test checks to ensure that remove_handles will only consider
    /// views up to what is known to have been stored. As a result it will
    /// indicate that is has only targeted to the highest view number that it
    /// is aware of.
    #[tokio::test]
    async fn test_global_state_remove_handles_prune_up_to_latest() {
        let (bootstrap_sender, _) = async_broadcast::broadcast(10);
        let (tx_sender, _) = async_broadcast::broadcast(10);
        let parent_commit = vid_commitment::<TestVersions>(
            &[0],
            &[],
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            <TestVersions as Versions>::Base::VERSION,
        );
        let mut state = GlobalState::<TestTypes>::new(
            bootstrap_sender,
            tx_sender,
            parent_commit,
            ViewNumber::new(0),
            ViewNumber::new(0),
            TEST_MAX_BLOCK_SIZE_INCREMENT_PERIOD,
            TEST_PROTOCOL_MAX_BLOCK_SIZE,
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            TEST_MAX_TX_NUM,
        );

        // We register a few builder states.
        for i in 1..=10 {
            let vid_commit = vid_commitment::<TestVersions>(
                &[i],
                &[],
                TEST_NUM_NODES_IN_VID_COMPUTATION,
                <TestVersions as Versions>::Base::VERSION,
            );
            let view = ViewNumber::new(i as u64);

            state.register_builder_state(
                BuilderStateId {
                    parent_commitment: vid_commit,
                    parent_view: view,
                },
                ParentBlockReferences {
                    view_number: view,
                    vid_commitment: vid_commit,
                    leaf_commit: Commitment::from_raw([0; 32]),
                    builder_commitment: BuilderCommitment::from_bytes([]),
                    // Unused in old legacy builder:
                    last_nonempty_view: None,
                    tx_count: 0,
                },
                async_broadcast::broadcast(10).0,
            );
        }

        assert_eq!(
            state.spawned_builder_states.len(),
            11,
            "The spawned_builder_states should have the expected number of entries",
        );

        assert_eq!(
            state.remove_handles(ViewNumber::new(100)),
            ViewNumber::new(10),
            "It should only be able to prune up to what has been stored"
        );

        assert_eq!(
            state.spawned_builder_states.len(),
            1,
            "The spawned_builder_states should only have a single entry in it"
        );

        let builder_state_id = BuilderStateId {
            parent_commitment: vid_commitment::<TestVersions>(
                &[10],
                &[],
                TEST_NUM_NODES_IN_VID_COMPUTATION,
                <TestVersions as Versions>::Base::VERSION,
            ),
            parent_view: ViewNumber::new(10),
        };
        assert_eq!(
            state.highest_view_num_builder_id, builder_state_id,
            "The highest view number builder id should be the one that was just registered"
        );

        assert_eq!(
            state.last_garbage_collected_view_num,
            ViewNumber::new(9),
            "The last garbage collected view number should match expected value"
        );

        assert!(
            state.spawned_builder_states.contains_key(&BuilderStateId {
                parent_commitment: vid_commitment::<TestVersions>(
                    &[10],
                    &[],
                    TEST_NUM_NODES_IN_VID_COMPUTATION,
                    <TestVersions as Versions>::Base::VERSION,
                ),
                parent_view: ViewNumber::new(10),
            }),
            "The spawned builder states should contain the builder state id: {builder_state_id}"
        );
    }

    /// This test checks that the remove_handles doesn't ensure that the
    /// `last_garbage_collected_view_num` is strictly increasing. By first
    /// removing a higher view  number, followed by a smaller view number
    /// (with the highest_view_num_builder_id having a view greater than or
    /// equal to both targets) we can demonstrate this property.
    ///
    /// Furthermore this demonstrates that by supplying any view number to
    /// remove_handles that is less than `last_garbage_collected_view_num` will
    /// result in `last_garbage_collected_view_num` being updated to the given
    /// value minus 1, without regard for it actually removing / cleaning
    /// anything, or whether it is moving backwards in view numbers.
    ///
    /// If we were to account for the view numbers actually being cleaned up,
    /// we could still trigger this behavior be re-adding the builder states
    /// with a view number that precedes the last garbage collected view number,
    /// then removing them would trigger the same behavior.
    #[tokio::test]
    async fn test_global_state_remove_handles_can_reduce_last_garbage_collected_view_num_simple() {
        let (bootstrap_sender, _) = async_broadcast::broadcast(10);
        let (tx_sender, _) = async_broadcast::broadcast(10);
        let parent_commit = vid_commitment::<TestVersions>(
            &[0],
            &[],
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            <TestVersions as Versions>::Base::VERSION,
        );
        let mut state = GlobalState::<TestTypes>::new(
            bootstrap_sender,
            tx_sender,
            parent_commit,
            ViewNumber::new(0),
            ViewNumber::new(0),
            TEST_MAX_BLOCK_SIZE_INCREMENT_PERIOD,
            TEST_PROTOCOL_MAX_BLOCK_SIZE,
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            TEST_MAX_TX_NUM,
        );

        // We register a few builder states.
        for i in 1..=10 {
            let vid_commit = vid_commitment::<TestVersions>(
                &[i],
                &[],
                TEST_NUM_NODES_IN_VID_COMPUTATION,
                <TestVersions as Versions>::Base::VERSION,
            );
            let view = ViewNumber::new(i as u64);

            state.register_builder_state(
                BuilderStateId {
                    parent_commitment: vid_commit,
                    parent_view: view,
                },
                ParentBlockReferences {
                    view_number: view,
                    vid_commitment: vid_commit,
                    leaf_commit: Commitment::from_raw([0; 32]),
                    builder_commitment: BuilderCommitment::from_bytes([]),
                    // Unused in old legacy builder:
                    last_nonempty_view: None,
                    tx_count: 0,
                },
                async_broadcast::broadcast(10).0,
            );
        }

        assert_eq!(
            state.highest_view_num_builder_id,
            BuilderStateId {
                parent_commitment: vid_commitment::<TestVersions>(
                    &[10],
                    &[],
                    TEST_NUM_NODES_IN_VID_COMPUTATION,
                    <TestVersions as Versions>::Base::VERSION,
                ),
                parent_view: ViewNumber::new(10),
            },
            "The highest view number builder id should be the one that was just registered"
        );

        assert_eq!(
            state.remove_handles(ViewNumber::new(10)),
            ViewNumber::new(10),
            "It should remove what has been stored"
        );

        assert_eq!(
            state.last_garbage_collected_view_num,
            ViewNumber::new(9),
            "The last garbage collected view number should match expected value"
        );

        assert_eq!(
            state.remove_handles(ViewNumber::new(5)),
            ViewNumber::new(5),
            "If we only remove up to view 5, then only entries preceding view 5 should be removed"
        );

        // The last garbage collected view has gone down as a result of our
        // new remove_handles target, demonstrating that this number isn't
        // strictly increasing in value.
        assert_eq!(
            state.last_garbage_collected_view_num,
            ViewNumber::new(4),
            "The last garbage collected view number should match expected value",
        );
    }

    /// This test checks that the remove_handles doesn't ensure that the
    /// `last_garbage_collected_view_num` is strictly increasing. It is very
    /// similar to `test_global_state_remove_handles_can_reduce_last_garbage_collected_view_num_simple`
    /// but differs in that it re-adds the removed builder states, just in case
    /// the previous test's behavior is erroneous and fixed by ensuring that we
    /// only consider removed view numbers.
    #[tokio::test]
    async fn test_global_state_remove_handles_can_reduce_last_garbage_collected_view_num_strict() {
        let (bootstrap_sender, _) = async_broadcast::broadcast(10);
        let (tx_sender, _) = async_broadcast::broadcast(10);
        let parent_commit = vid_commitment::<TestVersions>(
            &[0],
            &[],
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            <TestVersions as Versions>::Base::VERSION,
        );
        let mut state = GlobalState::<TestTypes>::new(
            bootstrap_sender,
            tx_sender,
            parent_commit,
            ViewNumber::new(0),
            ViewNumber::new(0),
            TEST_MAX_BLOCK_SIZE_INCREMENT_PERIOD,
            TEST_PROTOCOL_MAX_BLOCK_SIZE,
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            TEST_MAX_TX_NUM,
        );

        // We register a few builder states.
        for i in 1..=10 {
            let vid_commit = vid_commitment::<TestVersions>(
                &[i],
                &[],
                TEST_NUM_NODES_IN_VID_COMPUTATION,
                <TestVersions as Versions>::Base::VERSION,
            );
            let view = ViewNumber::new(i as u64);

            state.register_builder_state(
                BuilderStateId {
                    parent_commitment: vid_commit,
                    parent_view: view,
                },
                ParentBlockReferences {
                    view_number: view,
                    vid_commitment: vid_commit,
                    leaf_commit: Commitment::from_raw([0; 32]),
                    builder_commitment: BuilderCommitment::from_bytes([]),
                    // Unused in old legacy builder:
                    last_nonempty_view: None,
                    tx_count: 0,
                },
                async_broadcast::broadcast(10).0,
            );
        }

        assert_eq!(
            state.highest_view_num_builder_id,
            BuilderStateId {
                parent_commitment: vid_commitment::<TestVersions>(
                    &[10],
                    &[],
                    TEST_NUM_NODES_IN_VID_COMPUTATION,
                    <TestVersions as Versions>::Base::VERSION,
                ),
                parent_view: ViewNumber::new(10),
            },
            "The highest view number builder id should be the one that was just registered"
        );

        assert_eq!(
            state.remove_handles(ViewNumber::new(10)),
            ViewNumber::new(10),
            "It should remove what has been stored"
        );

        assert_eq!(
            state.last_garbage_collected_view_num,
            ViewNumber::new(9),
            "The last garbage collected view number should match expected value"
        );

        // We re-add these removed builder_state_ids
        for i in 1..10 {
            let vid_commit = vid_commitment::<TestVersions>(
                &[i],
                &[],
                TEST_NUM_NODES_IN_VID_COMPUTATION,
                <TestVersions as Versions>::Base::VERSION,
            );
            let view = ViewNumber::new(i as u64);

            state.register_builder_state(
                BuilderStateId {
                    parent_commitment: vid_commit,
                    parent_view: view,
                },
                ParentBlockReferences {
                    view_number: view,
                    vid_commitment: vid_commit,
                    leaf_commit: Commitment::from_raw([0; 32]),
                    builder_commitment: BuilderCommitment::from_bytes([]),
                    // Unused in old legacy builder:
                    last_nonempty_view: None,
                    tx_count: 0,
                },
                async_broadcast::broadcast(10).0,
            );
        }

        assert_eq!(
            state.remove_handles(ViewNumber::new(5)),
            ViewNumber::new(5),
            "If we only remove up to view 5, then only entries preceding view 5 should be removed"
        );

        // The last garbage collected view has gone down as a result of our
        // new remove_handles target, demonstrating that this number isn't
        // strictly increasing in value.
        assert_eq!(
            state.last_garbage_collected_view_num,
            ViewNumber::new(4),
            "The last garbage collected view number should match expected value",
        );
    }

    /// This test checks that the remove_handles methods will correctly remove
    /// The expected number of builder states from the spawned_builder_states
    /// hashmap.  It does this by specifically controlling the number of builder
    /// states that are registered, and then removing a subset of them. It
    /// verifies the absence of the entries that should have been removed, and
    /// the presence of the entries that should have been kept.
    #[tokio::test]
    async fn test_global_state_remove_handles_expected() {
        let (bootstrap_sender, _) = async_broadcast::broadcast(10);
        let (tx_sender, _) = async_broadcast::broadcast(10);
        let parent_commit = vid_commitment::<TestVersions>(
            &[0],
            &[],
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            <TestVersions as Versions>::Base::VERSION,
        );
        let mut state = GlobalState::<TestTypes>::new(
            bootstrap_sender,
            tx_sender,
            parent_commit,
            ViewNumber::new(0),
            ViewNumber::new(0),
            TEST_MAX_BLOCK_SIZE_INCREMENT_PERIOD,
            TEST_PROTOCOL_MAX_BLOCK_SIZE,
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            TEST_MAX_TX_NUM,
        );

        // We register a few builder states.
        for i in 1..=10 {
            let vid_commit = vid_commitment::<TestVersions>(
                &[i],
                &[],
                TEST_NUM_NODES_IN_VID_COMPUTATION,
                <TestVersions as Versions>::Base::VERSION,
            );
            let view = ViewNumber::new(i as u64);

            state.register_builder_state(
                BuilderStateId {
                    parent_commitment: vid_commit,
                    parent_view: view,
                },
                ParentBlockReferences {
                    view_number: view,
                    vid_commitment: vid_commit,
                    leaf_commit: Commitment::from_raw([0; 32]),
                    builder_commitment: BuilderCommitment::from_bytes([]),
                    // Unused in old legacy builder:
                    last_nonempty_view: None,
                    tx_count: 0,
                },
                async_broadcast::broadcast(10).0,
            );
        }

        assert_eq!(
            state.spawned_builder_states.len(),
            11,
            "The spawned_builder_states should have 11 elements in it"
        );

        assert_eq!(
            state.highest_view_num_builder_id,
            BuilderStateId {
                parent_commitment: vid_commitment::<TestVersions>(
                    &[10],
                    &[],
                    TEST_NUM_NODES_IN_VID_COMPUTATION,
                    <TestVersions as Versions>::Base::VERSION,
                ),
                parent_view: ViewNumber::new(10),
            },
            "The highest view number builder id should be the one that was just registered"
        );

        assert_eq!(
            state.last_garbage_collected_view_num,
            ViewNumber::new(0),
            "The last garbage collected view number should be hat was passed in"
        );

        // Now we want to clean up some previous builder states to ensure that we
        // remove the appropriate targets.

        // This should remove the view builder states preceding the view number 5
        assert_eq!(
            state.remove_handles(ViewNumber::new(5)),
            ViewNumber::new(5),
            "The last garbage collected view number should match expected value"
        );

        // There should be 11 - 5 entries remaining
        assert_eq!(
            state.spawned_builder_states.len(),
            6,
            "The spawned_builder_states should have 6 elements in it"
        );

        for i in 0..5 {
            let builder_state_id = BuilderStateId {
                parent_commitment: vid_commitment::<TestVersions>(
                    &[i],
                    &[],
                    TEST_NUM_NODES_IN_VID_COMPUTATION,
                    <TestVersions as Versions>::Base::VERSION,
                ),
                parent_view: ViewNumber::new(i as u64),
            };
            assert!(
                !state.spawned_builder_states.contains_key(&builder_state_id),
                "the spawned builder states should contain the builder state id, {builder_state_id}"
            );
        }

        for i in 5..=10 {
            let builder_state_id = BuilderStateId {
                parent_commitment: vid_commitment::<TestVersions>(
                    &[i],
                    &[],
                    TEST_NUM_NODES_IN_VID_COMPUTATION,
                    <TestVersions as Versions>::Base::VERSION,
                ),
                parent_view: ViewNumber::new(i as u64),
            };
            assert!(
                state.spawned_builder_states.contains_key(&builder_state_id),
                "The spawned builder states should contain the builder state id: {builder_state_id}"
            );
        }
    }

    // Get Available Blocks Tests

    /// This test checks that the error `AvailableBlocksError::NoBlocksAvailable`
    /// is returned when no blocks are available.
    ///
    /// To trigger this condition, we simply submit a request to the
    /// implementation of get_available_blocks, and we do not provide any
    /// information for the block view number requested.  As a result, the
    /// implementation will ultimately timeout, and return an error that
    /// indicates that no blocks were available.
    #[tokio::test]
    async fn test_get_available_blocks_error_no_blocks_available() {
        let (bootstrap_sender, _) = async_broadcast::broadcast(10);
        let (tx_sender, _) = async_broadcast::broadcast(10);
        let (builder_public_key, builder_private_key) =
            <BLSPubKey as BuilderSignatureKey>::generated_from_seed_indexed([0; 32], 0);
        let (leader_public_key, leader_private_key) =
            <BLSPubKey as SignatureKey>::generated_from_seed_indexed([0; 32], 1);
        let parent_commit = vid_commitment::<TestVersions>(
            &[],
            &[],
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            <TestVersions as Versions>::Base::VERSION,
        );

        let state = ProxyGlobalState::<TestTypes>::new(
            Arc::new(RwLock::new(GlobalState::<TestTypes>::new(
                bootstrap_sender,
                tx_sender,
                parent_commit,
                ViewNumber::new(0),
                ViewNumber::new(0),
                TEST_MAX_BLOCK_SIZE_INCREMENT_PERIOD,
                TEST_PROTOCOL_MAX_BLOCK_SIZE,
                TEST_NUM_NODES_IN_VID_COMPUTATION,
                TEST_MAX_TX_NUM,
            ))),
            (builder_public_key, builder_private_key),
            Duration::from_millis(100),
        );

        // leader_private_key
        let signature = BLSPubKey::sign(&leader_private_key, parent_commit.as_ref()).unwrap();

        // This *should* just time out
        let result = state
            .available_blocks_implementation(
                &vid_commitment::<TestVersions>(
                    &[],
                    &[],
                    TEST_NUM_NODES_IN_VID_COMPUTATION,
                    <TestVersions as Versions>::Base::VERSION,
                ),
                1,
                leader_public_key,
                &signature,
            )
            .await;

        match result {
            Err(AvailableBlocksError::NoBlocksAvailable) => {
                // This is what we expect.
                // This message *should* indicate that no blocks were available.
            },
            Err(err) => {
                panic!("Unexpected error: {:?}", err);
            },
            Ok(_) => {
                panic!("Expected an error, but got a result");
            },
        }
    }

    /// This test checks that the error `AvailableBlocksError::SignatureValidationFailed`
    /// is returned when the signature is invalid.
    ///
    /// To trigger this condition, we simply submit a request to the
    /// implementation of get_available_blocks, but we sign the request with
    /// the builder's private key instead of the leader's private key.  Since
    /// these keys do not match, this will result in a signature verification
    /// error.
    #[tokio::test]
    async fn test_get_available_blocks_error_invalid_signature() {
        let (bootstrap_sender, _) = async_broadcast::broadcast(10);
        let (tx_sender, _) = async_broadcast::broadcast(10);
        let (builder_public_key, builder_private_key) =
            <BLSPubKey as BuilderSignatureKey>::generated_from_seed_indexed([0; 32], 0);
        let (leader_public_key, _leader_private_key) =
            <BLSPubKey as SignatureKey>::generated_from_seed_indexed([0; 32], 1);
        let parent_commit = vid_commitment::<TestVersions>(
            &[],
            &[],
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            <TestVersions as Versions>::Base::VERSION,
        );

        let state = ProxyGlobalState::<TestTypes>::new(
            Arc::new(RwLock::new(GlobalState::<TestTypes>::new(
                bootstrap_sender,
                tx_sender,
                parent_commit,
                ViewNumber::new(0),
                ViewNumber::new(0),
                TEST_MAX_BLOCK_SIZE_INCREMENT_PERIOD,
                TEST_PROTOCOL_MAX_BLOCK_SIZE,
                TEST_NUM_NODES_IN_VID_COMPUTATION,
                TEST_MAX_TX_NUM,
            ))),
            (builder_public_key, builder_private_key.clone()),
            Duration::from_millis(100),
        );

        // leader_private_key
        let signature = BLSPubKey::sign(&builder_private_key, parent_commit.as_ref()).unwrap();

        // This *should* just time out
        let result = state
            .available_blocks_implementation(
                &vid_commitment::<TestVersions>(
                    &[],
                    &[],
                    TEST_NUM_NODES_IN_VID_COMPUTATION,
                    <TestVersions as Versions>::Base::VERSION,
                ),
                1,
                leader_public_key,
                &signature,
            )
            .await;

        match result {
            Err(AvailableBlocksError::SignatureValidationFailed) => {
                // This is what we expect.
                // This message *should* indicate that the signature passed
                // did not match the given public key.
            },
            Err(err) => {
                panic!("Unexpected error: {:?}", err);
            },
            Ok(_) => {
                panic!("Expected an error, but got a result");
            },
        }
    }

    /// This test checks that the error `AvailableBlocksError::RequestForAvailableViewThatHasAlreadyBeenDecided`
    /// is returned when the requested view number has already been garbage
    /// collected.
    ///
    /// To trigger this condition, we initialize the GlobalState with a
    /// garbage collected view number that is higher than the view that will
    /// be requested.
    #[tokio::test]
    async fn test_get_available_blocks_error_requesting_previous_view_number() {
        let (bootstrap_sender, _) = async_broadcast::broadcast(10);
        let (tx_sender, _) = async_broadcast::broadcast(10);
        let (builder_public_key, builder_private_key) =
            <BLSPubKey as BuilderSignatureKey>::generated_from_seed_indexed([0; 32], 0);
        let (leader_public_key, leader_private_key) =
            <BLSPubKey as SignatureKey>::generated_from_seed_indexed([0; 32], 1);
        let parent_commit = vid_commitment::<TestVersions>(
            &[],
            &[],
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            <TestVersions as Versions>::Base::VERSION,
        );

        let state = ProxyGlobalState::<TestTypes>::new(
            Arc::new(RwLock::new(GlobalState::<TestTypes>::new(
                bootstrap_sender,
                tx_sender,
                parent_commit,
                ViewNumber::new(0),
                ViewNumber::new(2),
                TEST_MAX_BLOCK_SIZE_INCREMENT_PERIOD,
                TEST_PROTOCOL_MAX_BLOCK_SIZE,
                TEST_NUM_NODES_IN_VID_COMPUTATION,
                TEST_MAX_TX_NUM,
            ))),
            (builder_public_key, builder_private_key),
            Duration::from_millis(100),
        );

        // leader_private_key
        let signature = BLSPubKey::sign(&leader_private_key, parent_commit.as_ref()).unwrap();

        // This *should* just time out
        let result = state
            .available_blocks_implementation(
                &vid_commitment::<TestVersions>(
                    &[],
                    &[],
                    TEST_NUM_NODES_IN_VID_COMPUTATION,
                    <TestVersions as Versions>::Base::VERSION,
                ),
                1,
                leader_public_key,
                &signature,
            )
            .await;

        match result {
            Err(AvailableBlocksError::RequestForAvailableViewThatHasAlreadyBeenDecided) => {
                // This is what we expect.
                // This message *should* indicate that the signature passed
                // did not match the given public key.
            },
            Err(err) => {
                panic!("Unexpected error: {:?}", err);
            },
            Ok(_) => {
                panic!("Expected an error, but got a result");
            },
        }
    }

    /// This test checks that the error `AvailableBlocksError::GetChannelForMatchingBuilderError`
    /// is returned when attempting to retrieve a view that is not stored within the state, and
    /// the highest view is also no longer stored within the state.
    ///
    /// To trigger this condition, we initialize the GlobalState with an initial
    /// state, and then we mutate the state to record the wrong latest state id.
    /// When interacted with `GlobalState` via `register_builder_state`, and
    /// `remove_handles`, this error doesn't seem possible immediately possible.
    #[tokio::test]
    async fn test_get_available_blocks_error_get_channel_for_matching_builder() {
        let (bootstrap_sender, _) = async_broadcast::broadcast(10);
        let (tx_sender, _) = async_broadcast::broadcast(10);
        let (builder_public_key, builder_private_key) =
            <BLSPubKey as BuilderSignatureKey>::generated_from_seed_indexed([0; 32], 0);
        let (leader_public_key, leader_private_key) =
            <BLSPubKey as SignatureKey>::generated_from_seed_indexed([0; 32], 1);
        let parent_commit = vid_commitment::<TestVersions>(
            &[],
            &[],
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            <TestVersions as Versions>::Base::VERSION,
        );

        let state = Arc::new(ProxyGlobalState::<TestTypes>::new(
            Arc::new(RwLock::new(GlobalState::<TestTypes>::new(
                bootstrap_sender,
                tx_sender,
                parent_commit,
                ViewNumber::new(4),
                ViewNumber::new(4),
                TEST_MAX_BLOCK_SIZE_INCREMENT_PERIOD,
                TEST_PROTOCOL_MAX_BLOCK_SIZE,
                TEST_NUM_NODES_IN_VID_COMPUTATION,
                TEST_MAX_TX_NUM,
            ))),
            (builder_public_key, builder_private_key.clone()),
            Duration::from_secs(1),
        ));

        {
            let mut write_locked_global_state = state.global_state.write_arc().await;
            write_locked_global_state.highest_view_num_builder_id = BuilderStateId {
                parent_commitment: parent_commit,
                parent_view: ViewNumber::new(5),
            };
        }

        // As a result, we **should** be receiving a request for the available
        // blocks with our expected state id on the receiver, along with a channel
        // to send the response back to the caller.

        let signature = BLSPubKey::sign(&leader_private_key, parent_commit.as_ref()).unwrap();
        let result = state
            .available_blocks_implementation(&parent_commit, 6, leader_public_key, &signature)
            .await;
        match result {
            Err(AvailableBlocksError::GetChannelForMatchingBuilderError(_)) => {
                // This is what we expect.
                // This message *should* indicate that the response channel was closed.
            },
            Err(err) => {
                panic!("Unexpected error: {:?}", err);
            },
            Ok(_) => {
                panic!("Expected an error, but got a result");
            },
        }
    }

    // We have two error cases for `available_blocks_implementation` that we
    // cannot seem trigger directly due to the nature of how the implementation
    // performs.
    //
    // The first is ChannelUnexpectedlyClosed, which doesn't seem to be
    // producible as the unbounded channel doesn't seem to be able to be
    // closed.
    //
    // The second is SigningBlockFailed, which doesn't seem to be producible
    // with a valid private key, and it's not clear how to create an invalid
    // private key.

    /// This test checks that call to `available_blocks_implementation` returns
    /// a successful response when the function is called before blocks are
    /// made available.
    #[tokio::test]
    async fn test_get_available_blocks_requested_before_blocks_available() {
        let (bootstrap_sender, _) = async_broadcast::broadcast(10);
        let (tx_sender, _) = async_broadcast::broadcast(10);
        let (builder_public_key, builder_private_key) =
            <BLSPubKey as BuilderSignatureKey>::generated_from_seed_indexed([0; 32], 0);
        let (leader_public_key, leader_private_key) =
            <BLSPubKey as SignatureKey>::generated_from_seed_indexed([0; 32], 1);
        let parent_commit = vid_commitment::<TestVersions>(
            &[],
            &[],
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            <TestVersions as Versions>::Base::VERSION,
        );

        let state = Arc::new(ProxyGlobalState::<TestTypes>::new(
            Arc::new(RwLock::new(GlobalState::<TestTypes>::new(
                bootstrap_sender,
                tx_sender,
                parent_commit,
                ViewNumber::new(0),
                ViewNumber::new(0),
                TEST_MAX_BLOCK_SIZE_INCREMENT_PERIOD,
                TEST_PROTOCOL_MAX_BLOCK_SIZE,
                TEST_NUM_NODES_IN_VID_COMPUTATION,
                TEST_MAX_TX_NUM,
            ))),
            (builder_public_key, builder_private_key.clone()),
            Duration::from_secs(1),
        ));

        let cloned_parent_commit = parent_commit;
        let cloned_state = state.clone();
        let cloned_leader_private_key = leader_private_key.clone();

        // We want to trigger a request for the available blocks, before we make the available block available
        let get_available_blocks_handle = spawn(async move {
            // leader_private_key
            let signature =
                BLSPubKey::sign(&cloned_leader_private_key, cloned_parent_commit.as_ref()).unwrap();
            cloned_state
                .available_blocks_implementation(
                    &cloned_parent_commit,
                    1,
                    leader_public_key,
                    &signature,
                )
                .await
        });

        // Now we want to make the block data available to the state.
        let expected_builder_state_id = BuilderStateId {
            parent_commitment: parent_commit,
            parent_view: ViewNumber::new(1),
        };

        let mut response_receiver = {
            // We only want to keep this write lock for the time needed, and
            // no more.
            let mut write_locked_global_state = state.global_state.write_arc().await;

            // We insert a sender so that the next time this stateId is requested,
            // it will be available to send data back.
            let (response_sender, response_receiver) = async_broadcast::broadcast(10);
            write_locked_global_state.register_builder_state(
                expected_builder_state_id.clone(),
                ParentBlockReferences {
                    view_number: expected_builder_state_id.parent_view,
                    vid_commitment: expected_builder_state_id.parent_commitment,
                    leaf_commit: Commitment::from_raw([0; 32]),
                    builder_commitment: BuilderCommitment::from_bytes([]),
                    // Unused in old legacy builder:
                    last_nonempty_view: None,
                    tx_count: 0,
                },
                response_sender,
            );

            response_receiver
        };

        // As a result, we **should** be receiving a request for the available
        // blocks with our expected state id on the receiver, along with a channel
        // to send the response back to the caller.

        let response_channel = match response_receiver.next().await {
            None => {
                panic!("Expected a request for available blocks, but didn't get one");
            },
            Some(MessageType::RequestMessage(req_msg)) => {
                assert_eq!(req_msg.state_id, expected_builder_state_id);
                req_msg.response_channel
            },
            Some(message) => {
                panic!(
                    "Expected a request for available blocks, but got a different message: {:?}",
                    message
                );
            },
        };

        // We want to send a ResponseMessage to the channel
        let expected_response = ResponseMessage {
            block_size: 9,
            offered_fee: 7,
            builder_hash: BuilderCommitment::from_bytes([1, 2, 3, 4, 5]),
        };

        assert!(
            response_channel.send(expected_response.clone()).is_ok(),
            "failed to send ResponseMessage"
        );

        let result = get_available_blocks_handle
            .await
            .expect("get_available_blocks_handle failed");
        match result {
            Err(err) => {
                panic!("Unexpected error: {:?}", err);
            },
            Ok(result) => {
                assert_eq!(
                    result,
                    vec![AvailableBlockInfo {
                        block_hash: expected_response.builder_hash.clone(),
                        block_size: expected_response.block_size,
                        offered_fee: expected_response.offered_fee,
                        signature: <BLSPubKey as BuilderSignatureKey>::sign_block_info(
                            &builder_private_key,
                            expected_response.block_size,
                            expected_response.offered_fee,
                            &expected_response.builder_hash,
                        )
                        .unwrap(),
                        sender: builder_public_key,
                        _phantom: Default::default(),
                    }],
                    "get_available_blocks response matches expectation"
                );
            },
        }
    }

    /// This test checks that call to `available_blocks_implementation` returns
    /// a successful response when the function is called after blocks are
    /// made available.
    #[tokio::test]
    async fn test_get_available_blocks_requested_after_blocks_available() {
        let (bootstrap_sender, _) = async_broadcast::broadcast(10);
        let (tx_sender, _) = async_broadcast::broadcast(10);
        let (builder_public_key, builder_private_key) =
            <BLSPubKey as BuilderSignatureKey>::generated_from_seed_indexed([0; 32], 0);
        let (leader_public_key, leader_private_key) =
            <BLSPubKey as SignatureKey>::generated_from_seed_indexed([0; 32], 1);
        let parent_commit = vid_commitment::<TestVersions>(
            &[],
            &[],
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            <TestVersions as Versions>::Base::VERSION,
        );

        let state = Arc::new(ProxyGlobalState::<TestTypes>::new(
            Arc::new(RwLock::new(GlobalState::<TestTypes>::new(
                bootstrap_sender,
                tx_sender,
                parent_commit,
                ViewNumber::new(0),
                ViewNumber::new(0),
                TEST_MAX_BLOCK_SIZE_INCREMENT_PERIOD,
                TEST_PROTOCOL_MAX_BLOCK_SIZE,
                TEST_NUM_NODES_IN_VID_COMPUTATION,
                TEST_MAX_TX_NUM,
            ))),
            (builder_public_key, builder_private_key.clone()),
            Duration::from_secs(1),
        ));

        let cloned_parent_commit = parent_commit;
        let cloned_state = state.clone();
        let cloned_leader_private_key = leader_private_key.clone();

        // Now we want to make the block data available to the state.
        let expected_builder_state_id = BuilderStateId {
            parent_commitment: parent_commit,
            parent_view: ViewNumber::new(1),
        };

        let mut response_receiver = {
            // We only want to keep this write lock for the time needed, and
            // no more.
            let mut write_locked_global_state = state.global_state.write_arc().await;

            // We insert a sender so that the next time this stateId is requested,
            // it will be available to send data back.
            let (response_sender, response_receiver) = async_broadcast::broadcast(10);
            write_locked_global_state.register_builder_state(
                expected_builder_state_id.clone(),
                ParentBlockReferences {
                    view_number: expected_builder_state_id.parent_view,
                    vid_commitment: expected_builder_state_id.parent_commitment,
                    leaf_commit: Commitment::from_raw([0; 32]),
                    builder_commitment: BuilderCommitment::from_bytes([]),
                    // Unused in old legacy builder:
                    last_nonempty_view: None,
                    tx_count: 0,
                },
                response_sender,
            );

            response_receiver
        };

        // We want to trigger a request for the available blocks, before we make the available block available
        let get_available_blocks_handle = spawn(async move {
            // leader_private_key
            let signature =
                BLSPubKey::sign(&cloned_leader_private_key, cloned_parent_commit.as_ref()).unwrap();
            cloned_state
                .available_blocks_implementation(
                    &cloned_parent_commit,
                    1,
                    leader_public_key,
                    &signature,
                )
                .await
        });

        // As a result, we **should** be receiving a request for the available
        // blocks with our expected state id on the receiver, along with a channel
        // to send the response back to the caller.

        let response_channel = match response_receiver.next().await {
            None => {
                panic!("Expected a request for available blocks, but didn't get one");
            },
            Some(MessageType::RequestMessage(req_msg)) => {
                assert_eq!(req_msg.state_id, expected_builder_state_id);
                req_msg.response_channel
            },
            Some(message) => {
                panic!(
                    "Expected a request for available blocks, but got a different message: {:?}",
                    message
                );
            },
        };

        // We want to send a ResponseMessage to the channel
        let expected_response = ResponseMessage {
            block_size: 9,
            offered_fee: 7,
            builder_hash: BuilderCommitment::from_bytes([1, 2, 3, 4, 5]),
        };

        assert!(
            response_channel.send(expected_response.clone()).is_ok(),
            "failed to send ResponseMessage"
        );

        let result = get_available_blocks_handle
            .await
            .expect("get_available_blocks_handle failed");
        match result {
            Err(err) => {
                panic!("Unexpected error: {:?}", err);
            },
            Ok(result) => {
                assert_eq!(
                    result,
                    vec![AvailableBlockInfo {
                        block_hash: expected_response.builder_hash.clone(),
                        block_size: expected_response.block_size,
                        offered_fee: expected_response.offered_fee,
                        signature: <BLSPubKey as BuilderSignatureKey>::sign_block_info(
                            &builder_private_key,
                            expected_response.block_size,
                            expected_response.offered_fee,
                            &expected_response.builder_hash,
                        )
                        .unwrap(),
                        sender: builder_public_key,
                        _phantom: Default::default(),
                    }],
                    "get_available_blocks response matches expectation"
                );
            },
        }
    }

    // Claim Block Tests

    /// This test checks that the error `ClaimBlockError::SignatureValidationFailed`
    /// is returned when the signature is invalid.
    ///
    /// To trigger this condition, we simply submit a request to the
    /// implementation of claim_block, but we sign the request with
    /// the builder's private key instead of the leader's private key.  Since
    /// these keys do not match, this will result in a signature verification
    /// error.
    #[tokio::test]
    async fn test_claim_block_error_signature_validation_failed() {
        let (bootstrap_sender, _) = async_broadcast::broadcast(10);
        let (tx_sender, _) = async_broadcast::broadcast(10);
        let (builder_public_key, builder_private_key) =
            <BLSPubKey as BuilderSignatureKey>::generated_from_seed_indexed([0; 32], 0);
        let (leader_public_key, _leader_private_key) =
            <BLSPubKey as SignatureKey>::generated_from_seed_indexed([0; 32], 1);
        let parent_commit = vid_commitment::<TestVersions>(
            &[],
            &[],
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            <TestVersions as Versions>::Base::VERSION,
        );

        let state = Arc::new(ProxyGlobalState::<TestTypes>::new(
            Arc::new(RwLock::new(GlobalState::<TestTypes>::new(
                bootstrap_sender,
                tx_sender,
                parent_commit,
                ViewNumber::new(0),
                ViewNumber::new(0),
                TEST_MAX_BLOCK_SIZE_INCREMENT_PERIOD,
                TEST_PROTOCOL_MAX_BLOCK_SIZE,
                TEST_NUM_NODES_IN_VID_COMPUTATION,
                TEST_MAX_TX_NUM,
            ))),
            (builder_public_key, builder_private_key.clone()),
            Duration::from_secs(1),
        ));

        let commitment = BuilderCommitment::from_bytes([0; 256]);

        let signature = BLSPubKey::sign(&builder_private_key, commitment.as_ref()).unwrap();
        let result = state
            .claim_block_implementation(&commitment, 1, leader_public_key, &signature)
            .await;

        match result {
            Err(ClaimBlockError::SignatureValidationFailed) => {
                // This is what we expect.
                // This message *should* indicate that the signature passed
                // did not match the given public key.
            },
            Err(err) => {
                panic!("Unexpected error: {:?}", err);
            },
            Ok(_) => {
                panic!("Expected an error, but got a result");
            },
        }
    }

    /// This test checks that the error `ClaimBlockError::BlockDataNotFound`
    /// is returned when the block data is not found.
    ///
    /// To trigger this condition, we simply submit a request to the
    /// implementation of claim_block, but we do not provide any information
    /// for the block data requested.  As a result, the implementation will
    /// ultimately timeout, and return an error that indicates that the block
    /// data was not found.
    #[tokio::test]
    async fn test_claim_block_error_block_data_not_found() {
        let (bootstrap_sender, _) = async_broadcast::broadcast(10);
        let (tx_sender, _) = async_broadcast::broadcast(10);
        let (builder_public_key, builder_private_key) =
            <BLSPubKey as BuilderSignatureKey>::generated_from_seed_indexed([0; 32], 0);
        let (leader_public_key, leader_private_key) =
            <BLSPubKey as SignatureKey>::generated_from_seed_indexed([0; 32], 1);
        let parent_commit = vid_commitment::<TestVersions>(
            &[],
            &[],
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            <TestVersions as Versions>::Base::VERSION,
        );

        let state = Arc::new(ProxyGlobalState::<TestTypes>::new(
            Arc::new(RwLock::new(GlobalState::<TestTypes>::new(
                bootstrap_sender,
                tx_sender,
                parent_commit,
                ViewNumber::new(0),
                ViewNumber::new(0),
                TEST_MAX_BLOCK_SIZE_INCREMENT_PERIOD,
                TEST_PROTOCOL_MAX_BLOCK_SIZE,
                TEST_NUM_NODES_IN_VID_COMPUTATION,
                TEST_MAX_TX_NUM,
            ))),
            (builder_public_key, builder_private_key.clone()),
            Duration::from_secs(1),
        ));

        let commitment = BuilderCommitment::from_bytes([0; 256]);

        let signature = BLSPubKey::sign(&leader_private_key, commitment.as_ref()).unwrap();
        let result = state
            .claim_block_implementation(&commitment, 1, leader_public_key, &signature)
            .await;

        match result {
            Err(ClaimBlockError::BlockDataNotFound) => {
                // This is what we expect.
                // This message *should* indicate that the signature passed
                // did not match the given public key.
            },
            Err(err) => {
                panic!("Unexpected error: {:?}", err);
            },
            Ok(_) => {
                panic!("Expected an error, but got a result");
            },
        }
    }

    /// This test checks that the function completes successfully.
    #[tokio::test]
    async fn test_claim_block_success() {
        let (bootstrap_sender, _) = async_broadcast::broadcast(10);
        let (tx_sender, _) = async_broadcast::broadcast(10);
        let (builder_public_key, builder_private_key) =
            <BLSPubKey as BuilderSignatureKey>::generated_from_seed_indexed([0; 32], 0);
        let (leader_public_key, leader_private_key) =
            <BLSPubKey as SignatureKey>::generated_from_seed_indexed([0; 32], 1);
        let parent_commit = vid_commitment::<TestVersions>(
            &[],
            &[],
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            <TestVersions as Versions>::Base::VERSION,
        );

        let state = Arc::new(ProxyGlobalState::<TestTypes>::new(
            Arc::new(RwLock::new(GlobalState::<TestTypes>::new(
                bootstrap_sender,
                tx_sender,
                parent_commit,
                ViewNumber::new(0),
                ViewNumber::new(0),
                TEST_MAX_BLOCK_SIZE_INCREMENT_PERIOD,
                TEST_PROTOCOL_MAX_BLOCK_SIZE,
                TEST_NUM_NODES_IN_VID_COMPUTATION,
                TEST_MAX_TX_NUM,
            ))),
            (builder_public_key, builder_private_key.clone()),
            Duration::from_secs(1),
        ));

        let commitment = BuilderCommitment::from_bytes([0; 256]);
        let cloned_commitment = commitment.clone();
        let cloned_state = state.clone();

        let vid_trigger_receiver = {
            let mut global_state_write_lock = state.global_state.write_arc().await;
            let block_id = BlockId {
                hash: commitment,
                view: ViewNumber::new(1),
            };

            let payload = TestBlockPayload {
                transactions: vec![TestTransaction::new(vec![1, 2, 3, 4])],
            };

            let (vid_trigger_sender, vid_trigger_receiver) = oneshot::channel();

            global_state_write_lock.blocks.put(
                block_id,
                BlockInfo {
                    block_payload: payload,
                    metadata: TestMetadata {
                        num_transactions: 1,
                    },
                    vid_trigger: Arc::new(async_lock::RwLock::new(Some(vid_trigger_sender))),
                    offered_fee: 100,
                    truncated: false,
                },
            );

            vid_trigger_receiver
        };

        let claim_block_join_handle = spawn(async move {
            let signature =
                BLSPubKey::sign(&leader_private_key, cloned_commitment.as_ref()).unwrap();
            cloned_state
                .claim_block_implementation(&cloned_commitment, 1, leader_public_key, &signature)
                .await
        });

        // This should be the started event
        match vid_trigger_receiver.await {
            Ok(TriggerStatus::Start) => {
                // This is what we expect.
            },
            _ => {
                panic!("Expected a TriggerStatus::Start event");
            },
        }

        let result = claim_block_join_handle.await;

        match result {
            Err(err) => {
                panic!("Unexpected error: {:?}", err);
            },
            Ok(_) => {
                // This is expected
            },
        }
    }

    // Claim Block Header Input Tests

    /// This test checks that the error `ClaimBlockHeaderInputError::SignatureValidationFailed`
    /// is returned when the signature is invalid.
    ///
    /// To trigger this condition, we simply submit a request to the
    /// implementation of claim_block, but we sign the request with
    /// the builder's private key instead of the leader's private key.  Since
    /// these keys do not match, this will result in a signature verification
    /// error.
    #[tokio::test]
    async fn test_claim_block_header_input_error_signature_verification_failed() {
        let (bootstrap_sender, _) = async_broadcast::broadcast(10);
        let (tx_sender, _) = async_broadcast::broadcast(10);
        let (builder_public_key, builder_private_key) =
            <BLSPubKey as BuilderSignatureKey>::generated_from_seed_indexed([0; 32], 0);
        let (leader_public_key, _leader_private_key) =
            <BLSPubKey as SignatureKey>::generated_from_seed_indexed([0; 32], 1);
        let parent_commit = vid_commitment::<TestVersions>(
            &[],
            &[],
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            <TestVersions as Versions>::Base::VERSION,
        );

        let state = Arc::new(ProxyGlobalState::<TestTypes>::new(
            Arc::new(RwLock::new(GlobalState::<TestTypes>::new(
                bootstrap_sender,
                tx_sender,
                parent_commit,
                ViewNumber::new(0),
                ViewNumber::new(0),
                TEST_MAX_BLOCK_SIZE_INCREMENT_PERIOD,
                TEST_PROTOCOL_MAX_BLOCK_SIZE,
                TEST_NUM_NODES_IN_VID_COMPUTATION,
                TEST_MAX_TX_NUM,
            ))),
            (builder_public_key, builder_private_key.clone()),
            Duration::from_secs(1),
        ));

        let commitment = BuilderCommitment::from_bytes([0; 256]);

        let signature = BLSPubKey::sign(&builder_private_key, commitment.as_ref()).unwrap();

        let result = state
            .claim_block_header_input_implementation(&commitment, 1, leader_public_key, &signature)
            .await;

        match result {
            Err(ClaimBlockHeaderInputError::SignatureValidationFailed) => {
                // This is what we expect.
                // This message *should* indicate that the signature passed
                // did not match the given public key.
            },
            Err(err) => {
                panic!("Unexpected error: {:?}", err);
            },
            Ok(_) => {
                panic!("Expected an error, but got a result");
            },
        }
    }

    /// This test checks that the error `ClaimBlockHeaderInputError::BlockHeaderNotFound`
    /// is returned when the block header is not found.
    ///
    /// To trigger this condition, we simply submit a request to the
    /// implementation of claim_block, but we do not provide any information
    /// for the block header requested.  As a result, the implementation will
    /// ultimately timeout, and return an error that indicates that the block
    /// header was not found.
    #[tokio::test]
    async fn test_claim_block_header_input_error_block_header_not_found() {
        let (bootstrap_sender, _) = async_broadcast::broadcast(10);
        let (tx_sender, _) = async_broadcast::broadcast(10);
        let (builder_public_key, builder_private_key) =
            <BLSPubKey as BuilderSignatureKey>::generated_from_seed_indexed([0; 32], 0);
        let (leader_public_key, leader_private_key) =
            <BLSPubKey as SignatureKey>::generated_from_seed_indexed([0; 32], 1);
        let parent_commit = vid_commitment::<TestVersions>(
            &[],
            &[],
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            <TestVersions as Versions>::Base::VERSION,
        );

        let state = Arc::new(ProxyGlobalState::<TestTypes>::new(
            Arc::new(RwLock::new(GlobalState::<TestTypes>::new(
                bootstrap_sender,
                tx_sender,
                parent_commit,
                ViewNumber::new(0),
                ViewNumber::new(0),
                TEST_MAX_BLOCK_SIZE_INCREMENT_PERIOD,
                TEST_PROTOCOL_MAX_BLOCK_SIZE,
                TEST_NUM_NODES_IN_VID_COMPUTATION,
                TEST_MAX_TX_NUM,
            ))),
            (builder_public_key, builder_private_key.clone()),
            Duration::from_secs(1),
        ));

        let commitment = BuilderCommitment::from_bytes([0; 256]);
        let signature = BLSPubKey::sign(&leader_private_key, commitment.as_ref()).unwrap();

        let result = state
            .claim_block_header_input_implementation(&commitment, 1, leader_public_key, &signature)
            .await;

        match result {
            Err(ClaimBlockHeaderInputError::BlockHeaderNotFound) => {
                // This is what we expect.
                // This message *should* indicate that the signature passed
                // did not match the given public key.
            },
            Err(err) => {
                panic!("Unexpected error: {:?}", err);
            },
            Ok(_) => {
                panic!("Expected an error, but got a result");
            },
        }
    }

    /// This test checks that successful response is returned when the VID is
    /// received in time.
    #[tokio::test]
    async fn test_claim_block_header_input_success() {
        let (bootstrap_sender, _) = async_broadcast::broadcast(10);
        let (tx_sender, _) = async_broadcast::broadcast(10);
        let (builder_public_key, builder_private_key) =
            <BLSPubKey as BuilderSignatureKey>::generated_from_seed_indexed([0; 32], 0);
        let (leader_public_key, leader_private_key) =
            <BLSPubKey as SignatureKey>::generated_from_seed_indexed([0; 32], 1);
        let parent_commit = vid_commitment::<TestVersions>(
            &[],
            &[],
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            <TestVersions as Versions>::Base::VERSION,
        );

        let state = Arc::new(ProxyGlobalState::<TestTypes>::new(
            Arc::new(RwLock::new(GlobalState::<TestTypes>::new(
                bootstrap_sender,
                tx_sender,
                parent_commit,
                ViewNumber::new(0),
                ViewNumber::new(0),
                TEST_MAX_BLOCK_SIZE_INCREMENT_PERIOD,
                TEST_PROTOCOL_MAX_BLOCK_SIZE,
                TEST_NUM_NODES_IN_VID_COMPUTATION,
                TEST_MAX_TX_NUM,
            ))),
            (builder_public_key, builder_private_key.clone()),
            Duration::from_secs(1),
        ));

        let commitment = BuilderCommitment::from_bytes([0; 256]);
        let cloned_commitment = commitment.clone();
        let cloned_state = state.clone();

        let claim_block_header_input_join_handle = spawn(async move {
            let signature =
                BLSPubKey::sign(&leader_private_key, cloned_commitment.as_ref()).unwrap();
            cloned_state
                .claim_block_header_input_implementation(
                    &cloned_commitment,
                    1,
                    leader_public_key,
                    &signature,
                )
                .await
        });

        // TODO Add precompute back.

        let result = claim_block_header_input_join_handle.await;

        match result {
            Err(err) => {
                panic!("Unexpected error: {:?}", err);
            },
            Ok(_) => {
                // This is expected.
            },
        }
    }

    // handle_da_event Tests

    /// This test checks that the error [HandleDaEventError::SignatureValidationFailed]
    /// is returned under the right conditions of invoking
    /// [handle_da_event_implementation].
    ///
    /// To trigger this error, we simply need to ensure that signature provided
    /// to the [Proposal] does not match the public key of the sender.
    /// Additionally, the public keys passed for both the leader and the sender
    /// need to match each other.
    #[tokio::test]
    async fn test_handle_da_event_implementation_error_signature_validation_failed() {
        let (sender_public_key, _) =
            <BLSPubKey as BuilderSignatureKey>::generated_from_seed_indexed([0; 32], 0);
        let (_, leader_private_key) =
            <BLSPubKey as BuilderSignatureKey>::generated_from_seed_indexed([0; 32], 1);
        let (da_channel_sender, _) = async_broadcast::broadcast(10);
        let view_number = ViewNumber::new(10);
        let epoch = Some(EpochNumber::new(1));

        let da_proposal = DaProposal2::<TestTypes> {
            encoded_transactions: Arc::new([1, 2, 3, 4, 5, 6]),
            metadata: TestMetadata {
                num_transactions: 1,
            }, // arbitrary
            view_number,
            epoch,
            epoch_transition_indicator: EpochTransitionIndicator::NotInTransition,
        };

        let encoded_txns_hash = Sha256::digest(&da_proposal.encoded_transactions);
        let signature =
            <BLSPubKey as SignatureKey>::sign(&leader_private_key, &encoded_txns_hash).unwrap();

        let signed_da_proposal = Arc::new(Proposal {
            data: da_proposal,
            signature,
            _pd: Default::default(),
        });

        let result = handle_da_event_implementation(
            &da_channel_sender,
            signed_da_proposal.clone(),
            sender_public_key,
        )
        .await;

        match result {
            Err(HandleDaEventError::SignatureValidationFailed) => {
                // This is expected.
            },
            Ok(_) => {
                panic!("expected an error, but received a successful attempt instead")
            },
            Err(err) => {
                panic!("Unexpected error: {:?}", err);
            },
        }
    }

    /// This test checks that the error [HandleDaEventError::BroadcastFailed]
    /// is returned under the right conditions of invoking
    /// [handle_da_event_implementation].
    ///
    /// To trigger this error, we simply need to ensure that the broadcast
    /// channel receiver has been closed / dropped before the attempt to
    /// send on the broadcast sender is performed.
    #[tokio::test]
    async fn test_handle_da_event_implementation_error_broadcast_failed() {
        let (sender_public_key, sender_private_key) =
            <BLSPubKey as BuilderSignatureKey>::generated_from_seed_indexed([0; 32], 0);
        let da_channel_sender = {
            let (da_channel_sender, _) = async_broadcast::broadcast(10);
            da_channel_sender
        };

        let view_number = ViewNumber::new(10);
        let epoch = Some(EpochNumber::new(1));

        let da_proposal = DaProposal2::<TestTypes> {
            encoded_transactions: Arc::new([1, 2, 3, 4, 5, 6]),
            metadata: TestMetadata {
                num_transactions: 1,
            }, // arbitrary
            view_number,
            epoch,
            epoch_transition_indicator: EpochTransitionIndicator::NotInTransition,
        };

        let encoded_txns_hash = Sha256::digest(&da_proposal.encoded_transactions);
        let signature =
            <BLSPubKey as SignatureKey>::sign(&sender_private_key, &encoded_txns_hash).unwrap();

        let signed_da_proposal = Arc::new(Proposal {
            data: da_proposal,
            signature,
            _pd: Default::default(),
        });

        let result = handle_da_event_implementation(
            &da_channel_sender,
            signed_da_proposal.clone(),
            sender_public_key,
        )
        .await;

        match result {
            Err(HandleDaEventError::BroadcastFailed(_)) => {
                // This error is expected
            },
            Ok(_) => {
                panic!("Expected an error, but got a result");
            },
            Err(err) => {
                panic!("Unexpected error: {:?}", err);
            },
        }
    }

    /// This test checks the expected successful behavior of the
    /// [handle_da_event_implementation] function.
    #[tokio::test]
    async fn test_handle_da_event_implementation_success() {
        let (sender_public_key, sender_private_key) =
            <BLSPubKey as BuilderSignatureKey>::generated_from_seed_indexed([0; 32], 0);
        let (da_channel_sender, da_channel_receiver) = async_broadcast::broadcast(10);
        let view_number = ViewNumber::new(10);
        let epoch = Some(EpochNumber::new(1));

        let da_proposal = DaProposal2::<TestTypes> {
            encoded_transactions: Arc::new([1, 2, 3, 4, 5, 6]),
            metadata: TestMetadata {
                num_transactions: 1,
            }, // arbitrary
            view_number,
            epoch,
            epoch_transition_indicator: EpochTransitionIndicator::NotInTransition,
        };

        let encoded_txns_hash = Sha256::digest(&da_proposal.encoded_transactions);
        let signature =
            <BLSPubKey as SignatureKey>::sign(&sender_private_key, &encoded_txns_hash).unwrap();

        let signed_da_proposal = Arc::new(Proposal {
            data: da_proposal,
            signature,
            _pd: Default::default(),
        });

        let result = handle_da_event_implementation(
            &da_channel_sender,
            signed_da_proposal.clone(),
            sender_public_key,
        )
        .await;

        match result {
            Ok(_) => {
                // This is expected.
            },
            Err(err) => {
                panic!("Unexpected error: {:?}", err);
            },
        }

        let mut da_channel_receiver = da_channel_receiver;
        match da_channel_receiver.next().await {
            Some(MessageType::DaProposalMessage(da_proposal_message)) => {
                assert_eq!(da_proposal_message.proposal, signed_da_proposal);
            },
            _ => {
                panic!("Expected a DaProposalMessage, but got something else");
            },
        }
    }

    // handle_quorum_event Tests

    /// This test checks that the error [HandleQuorumEventError::SignatureValidationFailed]
    /// is returned under the right conditions of invoking
    /// [handle_quorum_event_implementation].
    ///
    /// To trigger this error, we simply need to ensure that the signature
    /// provided to the [Proposal] does not match the public key of the sender.
    ///
    /// Additionally, the public keys passed for both the leader and the sender
    /// need to match each other.
    #[tokio::test]
    async fn test_handle_quorum_event_error_signature_validation_failed() {
        let (sender_public_key, _) =
            <BLSPubKey as BuilderSignatureKey>::generated_from_seed_indexed([0; 32], 0);
        let (_, leader_private_key) =
            <BLSPubKey as BuilderSignatureKey>::generated_from_seed_indexed([0; 32], 1);
        let (quorum_channel_sender, _) = async_broadcast::broadcast(10);
        let view_number = ViewNumber::new(10);

        let quorum_proposal = {
            let leaf: Leaf2<_> = Leaf::<TestTypes>::genesis::<TestVersions>(
                &TestValidatedState::default(),
                &TestInstanceState::default(),
            )
            .await
            .into();

            QuorumProposalWrapper::<TestTypes> {
                proposal: QuorumProposal2::<TestTypes> {
                    block_header: leaf.block_header().clone(),
                    view_number,
                    justify_qc: QuorumCertificate2::genesis::<TestVersions>(
                        &TestValidatedState::default(),
                        &TestInstanceState::default(),
                    )
                    .await,
                    upgrade_certificate: None,
                    view_change_evidence: None,
                    next_epoch_justify_qc: None,
                    next_drb_result: None,
                    epoch: None,
                    state_cert: None,
                },
            }
        };

        let leaf = Leaf2::from_quorum_proposal(&quorum_proposal);

        let signature =
            <BLSPubKey as SignatureKey>::sign(&leader_private_key, leaf.commit().as_ref()).unwrap();

        let signed_quorum_proposal = Arc::new(Proposal {
            data: quorum_proposal,
            signature,
            _pd: Default::default(),
        });

        let result = handle_quorum_event_implementation(
            &quorum_channel_sender,
            signed_quorum_proposal.clone(),
            sender_public_key,
        )
        .await;

        match result {
            Err(HandleQuorumEventError::SignatureValidationFailed) => {
                // This is expected.
            },
            Ok(_) => {
                panic!("expected an error, but received a successful attempt instead");
            },
            Err(err) => {
                panic!("Unexpected error: {:?}", err);
            },
        }
    }

    /// This test checks that the error [HandleQuorumEventError::BroadcastFailed]
    /// is returned under the right conditions of invoking
    /// [handle_quorum_event_implementation].
    ///
    /// To trigger this error, we simply need to ensure that the broadcast
    /// channel receiver has been closed / dropped before the attempt to
    /// send on the broadcast sender is performed.
    #[tokio::test]
    async fn test_handle_quorum_event_error_broadcast_failed() {
        let (sender_public_key, sender_private_key) =
            <BLSPubKey as BuilderSignatureKey>::generated_from_seed_indexed([0; 32], 0);
        let quorum_channel_sender = {
            let (quorum_channel_sender, _) = async_broadcast::broadcast(10);
            quorum_channel_sender
        };

        let view_number = ViewNumber::new(10);

        let quorum_proposal = {
            let leaf: Leaf2<_> = Leaf::<TestTypes>::genesis::<TestVersions>(
                &TestValidatedState::default(),
                &TestInstanceState::default(),
            )
            .await
            .into();

            QuorumProposalWrapper::<TestTypes> {
                proposal: QuorumProposal2::<TestTypes> {
                    block_header: leaf.block_header().clone(),
                    view_number,
                    justify_qc: QuorumCertificate2::genesis::<TestVersions>(
                        &TestValidatedState::default(),
                        &TestInstanceState::default(),
                    )
                    .await,
                    upgrade_certificate: None,
                    view_change_evidence: None,
                    next_epoch_justify_qc: None,
                    next_drb_result: None,
                    epoch: None,
                    state_cert: None,
                },
            }
        };

        let leaf = Leaf2::from_quorum_proposal(&quorum_proposal);

        let signature =
            <BLSPubKey as SignatureKey>::sign(&sender_private_key, leaf.commit().as_ref()).unwrap();

        let signed_quorum_proposal = Arc::new(Proposal {
            data: quorum_proposal,
            signature,
            _pd: Default::default(),
        });

        let result = handle_quorum_event_implementation(
            &quorum_channel_sender,
            signed_quorum_proposal.clone(),
            sender_public_key,
        )
        .await;

        match result {
            Err(HandleQuorumEventError::BroadcastFailed(_)) => {
                // This is expected.
            },
            Ok(_) => {
                panic!("Expected an error, but got a result");
            },
            Err(err) => {
                panic!("Unexpected error: {:?}", err);
            },
        }
    }

    /// This test checks to ensure that [handle_quorum_event_implementation]
    /// completes successfully as expected when the correct conditions are met.
    #[tokio::test]
    async fn test_handle_quorum_event_success() {
        let (sender_public_key, sender_private_key) =
            <BLSPubKey as BuilderSignatureKey>::generated_from_seed_indexed([0; 32], 0);
        let (quorum_channel_sender, quorum_channel_receiver) = async_broadcast::broadcast(10);
        let view_number = ViewNumber::new(10);

        let quorum_proposal = {
            let leaf: Leaf2<_> = Leaf::<TestTypes>::genesis::<TestVersions>(
                &TestValidatedState::default(),
                &TestInstanceState::default(),
            )
            .await
            .into();

            QuorumProposalWrapper::<TestTypes> {
                proposal: QuorumProposal2::<TestTypes> {
                    block_header: leaf.block_header().clone(),
                    view_number,
                    justify_qc: QuorumCertificate2::genesis::<TestVersions>(
                        &TestValidatedState::default(),
                        &TestInstanceState::default(),
                    )
                    .await,
                    upgrade_certificate: None,
                    view_change_evidence: None,
                    next_epoch_justify_qc: None,
                    next_drb_result: None,
                    epoch: None,
                    state_cert: None,
                },
            }
        };

        let leaf = Leaf2::from_quorum_proposal(&quorum_proposal);

        let signature =
            <BLSPubKey as SignatureKey>::sign(&sender_private_key, leaf.commit().as_ref()).unwrap();

        let signed_quorum_proposal = Arc::new(Proposal {
            data: quorum_proposal,
            signature,
            _pd: Default::default(),
        });

        let result = handle_quorum_event_implementation(
            &quorum_channel_sender,
            signed_quorum_proposal.clone(),
            sender_public_key,
        )
        .await;

        match result {
            Ok(_) => {
                // This is expected.
            },
            Err(err) => {
                panic!("Unexpected error: {:?}", err);
            },
        }

        let mut quorum_channel_receiver = quorum_channel_receiver;
        match quorum_channel_receiver.next().await {
            Some(MessageType::QuorumProposalMessage(da_proposal_message)) => {
                assert_eq!(da_proposal_message.proposal, signed_quorum_proposal);
            },
            _ => {
                panic!("Expected a QuorumProposalMessage, but got something else");
            },
        }
    }

    // HandleReceivedTxns Tests

    /// This test checks that the error [HandleReceivedTxnsError::TooManyTransactions]
    /// is returned when the conditions are met.
    ///
    /// To trigger this error we simply provide a broadcast channel with a
    /// buffer smaller than the number of transactions we are attempting to
    /// send through it.
    #[tokio::test]
    async fn test_handle_received_txns_error_too_many_transactions() {
        let (tx_sender, tx_receiver) = async_broadcast::broadcast(2);
        let num_transactions = 5;
        let mut txns = Vec::with_capacity(num_transactions);
        for index in 0..num_transactions {
            txns.push(TestTransaction::new(vec![index as u8]));
        }
        let txns = txns;

        {
            let mut handle_received_txns_iter = HandleReceivedTxns::<TestTypes>::new(
                tx_sender,
                txns.clone(),
                TransactionSource::HotShot,
                TEST_MAX_TX_LEN,
            );

            assert!(handle_received_txns_iter.next().is_some());
            assert!(handle_received_txns_iter.next().is_some());
            match handle_received_txns_iter.next() {
                Some(Err(HandleReceivedTxnsError::TooManyTransactions)) => {
                    // This is expected,
                },
                Some(Err(err)) => {
                    panic!("Unexpected error: {:?}", err);
                },
                Some(Ok(_)) => {
                    panic!("Expected an error, but got a result");
                },
                None => {
                    panic!("Expected an error, but got a result");
                },
            }
        }

        let mut tx_receiver = tx_receiver;
        assert!(tx_receiver.next().await.is_some());
        assert!(tx_receiver.next().await.is_some());
        assert!(tx_receiver.next().await.is_none());
    }

    /// This test checks that the error [HandleReceivedTxnsError::TransactionTooBig]
    /// when the conditions are met.
    ///
    /// To trigger this error we simply provide a [TestTransaction] whose size
    /// exceeds the maximum transaction length. we pass to [HandleReceivedTxns].
    #[tokio::test]
    async fn test_handle_received_txns_error_transaction_too_big() {
        let (tx_sender, tx_receiver) = async_broadcast::broadcast(10);
        let num_transactions = 2;
        let mut txns = Vec::with_capacity(num_transactions + 1);
        for index in 0..num_transactions {
            txns.push(TestTransaction::new(vec![index as u8]));
        }
        txns.push(TestTransaction::new(vec![0; 256]));
        let txns = txns;

        {
            let mut handle_received_txns_iter = HandleReceivedTxns::<TestTypes>::new(
                tx_sender,
                txns.clone(),
                TransactionSource::HotShot,
                TEST_MAX_TX_LEN,
            );

            assert!(handle_received_txns_iter.next().is_some());
            assert!(handle_received_txns_iter.next().is_some());
            match handle_received_txns_iter.next() {
                Some(Err(HandleReceivedTxnsError::TransactionTooBig {
                    estimated_length,
                    max_txn_len,
                })) => {
                    // This is expected,
                    assert!(estimated_length >= 256);
                    assert_eq!(max_txn_len, TEST_MAX_TX_LEN);
                },
                Some(Err(err)) => {
                    panic!("Unexpected error: {:?}", err);
                },
                Some(Ok(_)) => {
                    panic!("Expected an error, but got a result");
                },
                None => {
                    panic!("Expected an error, but got a result");
                },
            }
        }

        let mut tx_receiver = tx_receiver;
        assert!(tx_receiver.next().await.is_some());
        assert!(tx_receiver.next().await.is_some());
        assert!(tx_receiver.next().await.is_none());
    }

    /// This test checks that the error [HandleReceivedTxnsError::Internal]
    /// is returned when the broadcast channel is closed.
    ///
    /// To trigger this error we simply close the broadcast channel receiver
    /// before attempting to send any transactions through the broadcast channel
    /// sender.
    #[tokio::test]
    async fn test_handle_received_txns_error_internal() {
        let tx_sender = {
            let (tx_sender, _) = async_broadcast::broadcast(10);
            tx_sender
        };

        let num_transactions = 10;
        let mut txns = Vec::with_capacity(num_transactions);
        for index in 0..num_transactions {
            txns.push(TestTransaction::new(vec![index as u8]));
        }
        txns.push(TestTransaction::new(vec![0; 256]));
        let txns = txns;

        {
            let mut handle_received_txns_iter = HandleReceivedTxns::<TestTypes>::new(
                tx_sender,
                txns.clone(),
                TransactionSource::HotShot,
                TEST_MAX_TX_LEN,
            );

            match handle_received_txns_iter.next() {
                Some(Err(HandleReceivedTxnsError::Internal(err))) => {
                    // This is expected,

                    match err {
                        async_broadcast::TrySendError::Closed(_) => {
                            // This is expected.
                        },
                        _ => {
                            panic!("Unexpected error: {:?}", err);
                        },
                    }
                },
                Some(Err(err)) => {
                    panic!("Unexpected error: {:?}", err);
                },
                Some(Ok(_)) => {
                    panic!("Expected an error, but got a result");
                },
                None => {
                    panic!("Expected an error, but got a result");
                },
            }
        }
    }

    /// This test checks that [HandleReceivedTxns] processes completely without
    /// issue when the conditions are correct for it to do so.
    #[tokio::test]
    async fn test_handle_received_txns_success() {
        let (tx_sender, tx_receiver) = async_broadcast::broadcast(10);
        let num_transactions = 10;
        let mut txns = Vec::with_capacity(num_transactions);
        for index in 0..num_transactions {
            txns.push(TestTransaction::new(vec![index as u8]));
        }
        let txns = txns;

        let handle_received_txns_iter = HandleReceivedTxns::<TestTypes>::new(
            tx_sender,
            txns.clone(),
            TransactionSource::HotShot,
            TEST_MAX_TX_LEN,
        );

        for iteration in handle_received_txns_iter {
            match iteration {
                Ok(_) => {
                    // This is expected.
                },
                Err(err) => {
                    panic!("Unexpected error: {:?}", err);
                },
            }
        }

        let mut tx_receiver = tx_receiver;
        for tx in txns {
            match tx_receiver.next().await {
                Some(received_txn) => {
                    assert_eq!(received_txn.tx, tx);
                },
                _ => {
                    panic!("Expected a TransactionMessage, but got something else");
                },
            }
        }
    }

    /// This test checks builder does save the status of transactions correctly
    #[tokio::test]
    async fn test_get_txn_status() {
        let (proxy_global_state, _, da_proposal_sender, quorum_proposal_sender, _) =
            setup_builder_for_test();
        tracing::debug!("start tests on correctly setting transaction status.");

        let mut round = 0;
        let mut current_builder_state_id = BuilderStateId::<TestTypes> {
            parent_commitment: vid_commitment::<TestVersions>(
                &[],
                &[],
                8,
                <TestVersions as Versions>::Base::VERSION,
            ),
            parent_view: ViewNumber::genesis(),
        };
        current_builder_state_id = progress_round_without_available_block_info(
            current_builder_state_id,
            round,
            &da_proposal_sender,
            &quorum_proposal_sender,
        )
        .await;

        // round 1: test status Pending
        let num_transactions = 10;
        let mut txns = Vec::with_capacity(num_transactions);
        for index in 0..num_transactions {
            txns.push(TestTransaction::new(vec![index as u8]));
        }
        let txns = txns;
        proxy_global_state
            .submit_txns(txns.clone())
            .await
            .expect("should submit transaction without issue");
        // advance the round
        {
            round = 1;
            let (_attempts, available_available_blocks_result) = process_available_blocks_round(
                &proxy_global_state,
                current_builder_state_id.clone(),
                round,
            )
            .await;
            current_builder_state_id = progress_round_with_available_block_info(
                &proxy_global_state,
                available_available_blocks_result.unwrap()[0].clone(),
                current_builder_state_id,
                round,
                &da_proposal_sender,
                &quorum_proposal_sender,
            )
            .await;
        }
        // tx submitted in round 1 should be pending
        for tx in txns.clone() {
            match proxy_global_state.txn_status(tx.commit()).await {
                Ok(txn_status) => {
                    assert_eq!(txn_status, TransactionStatus::Pending);
                },
                e => {
                    panic!("transaction status should be Pending instead of {:?}", e);
                },
            }
        }

        // round 2: test status Pending again
        let mut txns_2 = Vec::with_capacity(num_transactions);
        for index in 0..num_transactions {
            txns_2.push(TestTransaction::new(vec![(num_transactions + index) as u8]));
        }
        let txns_2 = txns_2;
        proxy_global_state
            .submit_txns(txns_2.clone())
            .await
            .expect("should submit transaction without issue");
        // advance the round
        {
            round = 2;
            let (_attempts, available_available_blocks_result) = process_available_blocks_round(
                &proxy_global_state,
                current_builder_state_id.clone(),
                round,
            )
            .await;
            progress_round_with_available_block_info(
                &proxy_global_state,
                available_available_blocks_result.unwrap()[0].clone(),
                current_builder_state_id,
                round,
                &da_proposal_sender,
                &quorum_proposal_sender,
            )
            .await;
        }
        // tx submitted in round 2 should be pending
        for tx in txns_2.clone() {
            match proxy_global_state.txn_status(tx.commit()).await {
                Ok(txn_status) => {
                    assert_eq!(txn_status, TransactionStatus::Pending);
                },
                e => {
                    panic!("transaction status should be Pending instead of {:?}", e);
                },
            }
        }

        // round 3: test status Rejected with correct error message
        let big_txns = vec![TestTransaction::new(vec![
            0;
            TEST_PROTOCOL_MAX_BLOCK_SIZE
                as usize
                + 1
        ])];
        let _ = proxy_global_state.submit_txns(big_txns.clone()).await;
        for tx in big_txns.clone() {
            match proxy_global_state.txn_status(tx.commit()).await {
                Ok(txn_status) => {
                    if tx.minimum_block_size() > TEST_PROTOCOL_MAX_BLOCK_SIZE {
                        tracing::debug!(
                            "In test_get_txn_status(), txn_status of large tx = {:?}",
                            txn_status
                        );
                        matches!(txn_status, TransactionStatus::Rejected { .. });
                        if let TransactionStatus::Rejected { reason } = txn_status {
                            assert!(reason.contains("Transaction too big"));
                        }
                    } else {
                        assert_eq!(txn_status, TransactionStatus::Pending);
                    }
                },
                e => {
                    panic!(
                        "transaction status should be a valid status instead of {:?}",
                        e
                    );
                },
            }
        }

        {
            // Test a rejected txn marked as other status again
            let mut write_guard = proxy_global_state.global_state.write_arc().await;
            for tx in big_txns {
                match write_guard
                    .set_txn_status(tx.commit(), TransactionStatus::Pending)
                    .await
                {
                    Err(err) => {
                        panic!("Expected a result, but got a error {:?}", err);
                    },
                    _ => {
                        // This is expected
                    },
                }

                match write_guard.txn_status(tx.commit()).await {
                    Ok(txn_status) => {
                        assert_eq!(txn_status, TransactionStatus::Pending);
                    },
                    e => {
                        panic!(
                            "transaction status should be a valid status instead of {:?}",
                            e
                        );
                    },
                }
            }
        }

        {
            // Test a sequenced txn cannot be marked as other status again
            let mut write_guard = proxy_global_state.global_state.write_arc().await;
            let tx_test_assigned_twice =
                TestTransaction::new(vec![(num_transactions * 3 + 1) as u8]);
            write_guard
                .set_txn_status(
                    tx_test_assigned_twice.commit(),
                    TransactionStatus::Sequenced { leaf: 0 },
                )
                .await
                .unwrap();
            match write_guard
                .set_txn_status(tx_test_assigned_twice.commit(), TransactionStatus::Pending)
                .await
            {
                Err(_err) => {
                    // This is expected
                },
                _ => {
                    panic!("Expected an error, but got a result");
                },
            }
        }

        {
            // Test status Unknown when the txn is unknown
            let unknown_tx = TestTransaction::new(vec![(num_transactions * 4 + 1) as u8]);
            match proxy_global_state.txn_status(unknown_tx.commit()).await {
                Ok(txn_status) => {
                    assert_eq!(txn_status, TransactionStatus::Unknown);
                },
                e => {
                    panic!("transaction status should be Unknown instead of {:?}", e);
                },
            }
        }
    }

    #[test]
    fn test_increment_block_size() {
        let mut block_size_limits =
            BlockSizeLimits::new(TEST_PROTOCOL_MAX_BLOCK_SIZE, Duration::from_millis(25));
        // Simulate decreased limits
        block_size_limits.max_block_size = TEST_PROTOCOL_MAX_BLOCK_SIZE / 2;

        // Shouldn't increment, increment period hasn't passed yet
        block_size_limits.try_increment_block_size(false);
        assert!(block_size_limits.max_block_size == TEST_PROTOCOL_MAX_BLOCK_SIZE / 2);

        // Should increment, increment period hasn't passed yet, but force flag is set
        block_size_limits.try_increment_block_size(true);
        assert!(block_size_limits.max_block_size > TEST_PROTOCOL_MAX_BLOCK_SIZE / 2);
        let new_size = block_size_limits.max_block_size;

        std::thread::sleep(Duration::from_millis(30));

        // Should increment, increment period has passed
        block_size_limits.try_increment_block_size(false);
        assert!(block_size_limits.max_block_size > new_size);
    }

    #[test]
    fn test_decrement_block_size() {
        let mut block_size_limits = BlockSizeLimits::new(
            TEST_PROTOCOL_MAX_BLOCK_SIZE,
            TEST_MAX_BLOCK_SIZE_INCREMENT_PERIOD,
        );
        block_size_limits.decrement_block_size();
        assert!(block_size_limits.max_block_size < TEST_PROTOCOL_MAX_BLOCK_SIZE);
    }

    #[test]
    fn test_max_block_size_floor() {
        let mut block_size_limits = BlockSizeLimits::new(
            BlockSizeLimits::MAX_BLOCK_SIZE_FLOOR + 1,
            TEST_MAX_BLOCK_SIZE_INCREMENT_PERIOD,
        );
        block_size_limits.decrement_block_size();
        assert_eq!(
            block_size_limits.max_block_size,
            BlockSizeLimits::MAX_BLOCK_SIZE_FLOOR
        );
    }
}
