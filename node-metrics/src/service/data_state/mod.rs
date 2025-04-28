pub mod location_details;
pub mod node_identity;

use std::{collections::HashSet, iter::zip, sync::Arc};

use async_lock::RwLock;
use bitvec::vec::BitVec;
use circular_buffer::CircularBuffer;
use espresso_types::{Header, Payload, SeqTypes};
use futures::{channel::mpsc::SendError, Sink, SinkExt, Stream, StreamExt};
use hotshot_query_service::{
    availability::{BlockQueryData, Leaf1QueryData, QueryableHeader},
    explorer::{BlockDetail, ExplorerHeader, Timestamp},
    Resolvable,
};
use hotshot_stake_table::vec_based::StakeTable;
use hotshot_types::{
    light_client::{CircuitField, StateVerKey},
    signature_key::BLSPubKey,
    traits::{
        block_contents::BlockHeader,
        stake_table::{SnapshotVersion, StakeTableScheme},
        BlockPayload, EncodeBytes,
    },
    utils::epoch_from_block_number,
};
pub use location_details::LocationDetails;
pub use node_identity::NodeIdentity;
use time::OffsetDateTime;
use tokio::{spawn, task::JoinHandle};

use crate::api::node_validator::v0::{
    get_node_stake_table_from_sequencer, LeafAndBlock, PublicHotShotConfig, Version01,
};

/// MAX_HISTORY represents the last N records that are stored within the
/// DataState structure for the various different sample types.
pub const MAX_HISTORY: usize = 50;

/// MAX_VOTERS_HISTORY represents the last N records that are stored within
/// the DataState structure for the voters.
pub const MAX_VOTERS_HISTORY: usize = 100;

/// [DataState] represents the state of the data that is being stored within
/// the service.
#[cfg_attr(test, derive(Default))]
pub struct DataState {
    latest_blocks: CircularBuffer<MAX_HISTORY, BlockDetail<SeqTypes>>,
    latest_voters: CircularBuffer<MAX_VOTERS_HISTORY, BitVec<u16>>,
    stake_table: StakeTable<BLSPubKey, StateVerKey, CircuitField>,
    // Do we need any other data at the moment?
    node_identity: Vec<NodeIdentity>,
}

impl DataState {
    pub fn new(
        latest_blocks: CircularBuffer<MAX_HISTORY, BlockDetail<SeqTypes>>,
        latest_voters: CircularBuffer<MAX_VOTERS_HISTORY, BitVec<u16>>,
        stake_table: StakeTable<BLSPubKey, StateVerKey, CircuitField>,
    ) -> Self {
        let node_identity = {
            let stake_table_iter_result = stake_table.try_iter(SnapshotVersion::Head);
            match stake_table_iter_result {
                Ok(into_iter) => into_iter
                    .map(|(key, ..)| NodeIdentity::from_public_key(key))
                    .collect(),
                Err(_) => vec![],
            }
        };

        Self {
            latest_blocks,
            latest_voters,
            stake_table,
            node_identity,
        }
    }

    pub fn latest_blocks(&self) -> impl Iterator<Item = &BlockDetail<SeqTypes>> {
        self.latest_blocks.iter()
    }

    pub fn is_latest_blocks_empty(&self) -> bool {
        !self.latest_blocks.is_empty()
    }

    pub fn latest_voters(&self) -> impl Iterator<Item = &BitVec<u16>> {
        self.latest_voters.iter()
    }

    pub fn stake_table(&self) -> &StakeTable<BLSPubKey, StateVerKey, CircuitField> {
        &self.stake_table
    }

    pub fn node_identity(&self) -> impl Iterator<Item = &NodeIdentity> {
        self.node_identity.iter()
    }

    // [stake_table_differences] is a helper function that will check the
    // public key entry differences between the [old_stake_table] and the new
    // [stake_table].
    //
    // This function will return a tuple of a list of of added public keys,
    // and a list of removed public keys
    fn stake_table_differences(
        &self,
        old_stake_table: &StakeTable<BLSPubKey, StateVerKey, CircuitField>,
        stake_table: &StakeTable<BLSPubKey, StateVerKey, CircuitField>,
    ) -> (Vec<BLSPubKey>, Vec<BLSPubKey>) {
        let old_stake_table_set = match old_stake_table.try_iter(SnapshotVersion::Head) {
            Ok(into_iter) => into_iter.map(|(key, ..)| key).collect::<HashSet<_>>(),
            Err(err) => {
                tracing::error!(
                    "replace_stake_table: error getting old stake table iterator: {}",
                    err
                );

                return (vec![], vec![]);
            },
        };

        let new_stake_table_set = match stake_table.try_iter(SnapshotVersion::Head) {
            Ok(into_iter) => into_iter.map(|(key, ..)| key).collect::<HashSet<_>>(),
            Err(err) => {
                tracing::error!(
                    "replace_stake_table: error getting new stake table iterator: {}",
                    err
                );

                return (vec![], vec![]);
            },
        };

        let added_public_keys = new_stake_table_set
            .difference(&old_stake_table_set)
            .cloned()
            .collect::<Vec<_>>();

        let removed_public_keys = old_stake_table_set
            .difference(&new_stake_table_set)
            .cloned()
            .collect::<Vec<_>>();

        tracing::info!(
            "new stake table added {:?} public keys",
            added_public_keys
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        );
        tracing::info!(
            "new stake table removed {:?} public keys",
            removed_public_keys
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        );

        (added_public_keys, removed_public_keys)
    }

    pub fn replace_stake_table(
        &mut self,
        stake_table: StakeTable<BLSPubKey, StateVerKey, CircuitField>,
    ) {
        let old_stake_table = std::mem::replace(&mut self.stake_table, stake_table.clone());

        let current_identity_set = self
            .node_identity
            .iter()
            .map(|node_identity| *node_identity.public_key())
            .collect::<HashSet<_>>();

        // We want to make sure that we're accounting for this node identity
        // information that we have.  In the case of any new public keys
        // being added, we want to ensure we have an entry for them in our
        // node identity list.

        // TODO @ayiga: We need to figure out how to prune old nodes from
        //      the list of node identities in such a way that does not
        //      cause issues or lend to confusion.  For now, we will just
        //      ignore these removals, and keep a potentially ever-growing
        //      list of node identities.
        let (new_public_keys, _) = self.stake_table_differences(&old_stake_table, &stake_table);

        let missing_node_identity_entries = new_public_keys
            .into_iter()
            .filter(|key| !current_identity_set.contains(key));

        self.node_identity
            .extend(missing_node_identity_entries.map(NodeIdentity::from_public_key));
    }

    pub fn add_latest_block(&mut self, block: BlockDetail<SeqTypes>) {
        self.latest_blocks.push_back(block);
    }

    pub fn add_latest_voters(&mut self, voters: BitVec<u16>) {
        self.latest_voters.push_back(voters);
    }

    pub fn add_node_identity(&mut self, identity: NodeIdentity) {
        // We need to check to see if this identity is already in the list,
        // if it is, we will want to replace it.

        let pub_key = identity.public_key();

        let mut matching_public_keys = self
            .node_identity
            .iter()
            // We want the index of the entry for easier editing
            .enumerate()
            .filter(|(_, node_identity)| node_identity.public_key() == pub_key);

        // We only expect this have a single entry.
        let existing_node_identity_option = matching_public_keys.next();

        debug_assert_eq!(matching_public_keys.next(), None);

        if let Some((index, _)) = existing_node_identity_option {
            self.node_identity[index] = identity;
            return;
        }

        // This entry doesn't appear in our table, so let's add it.
        self.node_identity.push(identity);
    }
}

/// [ProcessLeafError] represents the error that can occur when processing
/// a [Leaf].
#[derive(Debug)]
pub enum ProcessLeafError {
    BlockSendError(SendError),
    VotersSendError(SendError),
    FailedToGetNewStakeTable,
}

impl std::fmt::Display for ProcessLeafError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessLeafError::BlockSendError(err) => {
                write!(f, "error sending block detail to sender: {}", err)
            },
            ProcessLeafError::VotersSendError(err) => {
                write!(f, "error sending voters to sender: {}", err)
            },
            ProcessLeafError::FailedToGetNewStakeTable => {
                write!(f, "error getting new stake table from sequencer")
            },
        }
    }
}

impl std::error::Error for ProcessLeafError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ProcessLeafError::BlockSendError(err) => Some(err),
            ProcessLeafError::VotersSendError(err) => Some(err),
            ProcessLeafError::FailedToGetNewStakeTable => None,
        }
    }
}

/// create_block_detail_from_block is a helper function that will create a
/// [BlockDetail] from a [BlockQueryData].
pub fn create_block_detail_from_block(block: &BlockQueryData<SeqTypes>) -> BlockDetail<SeqTypes> {
    let block_header = block.header();
    let block_payload = block.payload();
    let num_transactions = block.num_transactions();
    let encoded_bytes = block_payload.encode();
    let total_payload_size = encoded_bytes.len() as u64;

    BlockDetail::<SeqTypes> {
        hash: block_header.commitment(),
        height: block_header.height(),
        time: Timestamp(
            OffsetDateTime::from_unix_timestamp(block_header.timestamp() as i64)
                .unwrap_or(OffsetDateTime::UNIX_EPOCH),
        ),
        proposer_id: block_header.proposer_id(),
        num_transactions,
        block_reward: vec![block_header.fee_info_balance().into()],
        fee_recipient: block_header.fee_info_account(),
        size: total_payload_size,
    }
}

// epoch_number_helper is a utility function that will determine the
// epoch number based on the given [block_height], [epoch_starting_block],
// and [num_blocks_per_epoch].
//
// This function will ensure that the epoch number is zero if the given
// [block_height] is less than the [epoch_starting_block]. Otherwise it
// will return the [epoch_from_block_number] result.
fn epoch_number_helper(
    block_height: u64,
    epoch_starting_block: u64,
    num_blocks_per_epoch: u64,
) -> u64 {
    if block_height < epoch_starting_block {
        return 0;
    }

    epoch_from_block_number(block_height, num_blocks_per_epoch)
}

async fn perform_stake_table_epoch_check_and_update(
    data_state: Arc<RwLock<DataState>>,
    client: surf_disco::Client<hotshot_query_service::Error, Version01>,
    hotshot_config: &PublicHotShotConfig,
    block_height: u64,
) -> Result<(), ProcessLeafError> {
    // Are we in a new epoch?
    // Do we need to replace our stake table?
    tracing::debug!("processing block height: {}", block_height);
    if let (Some(epoch_starting_block), Some(num_blocks_per_epoch)) = (
        hotshot_config.epoch_start_block,
        hotshot_config.epoch_height,
    ) {
        let previous_epoch = epoch_number_helper(
            block_height.saturating_sub(1),
            epoch_starting_block,
            num_blocks_per_epoch,
        );
        let upcoming_epoch =
            epoch_number_helper(block_height, epoch_starting_block, num_blocks_per_epoch);

        if upcoming_epoch != previous_epoch {
            tracing::debug!(
                "new epoch detected: {} -> {} for blocks {}, and {}",
                previous_epoch,
                upcoming_epoch,
                block_height.saturating_sub(1),
                block_height,
            );
            // We're in a new epoch, so we'll need to update our stake table
            let next_stake_table = match get_node_stake_table_from_sequencer(client, upcoming_epoch)
                .await
            {
                Ok(stake_table) => stake_table,
                Err(err) => {
                    tracing::error!("process_incoming_leaf_and_block: error getting stake table from sequencer: {}", err);
                    return Err(ProcessLeafError::FailedToGetNewStakeTable);
                },
            };

            {
                tracing::debug!(
                    "replacing stake table for epoch {}, new table contains {} entries",
                    upcoming_epoch,
                    next_stake_table.len(SnapshotVersion::Head).unwrap_or(0),
                );
                // Update the stake table
                let mut data_state_write_lock_guard = data_state.write().await;
                data_state_write_lock_guard.replace_stake_table(next_stake_table);
            }
        }
    }

    Ok(())
}

/// [process_incoming_leaf_and_block] is a helper function that will process
/// an incoming [Leaf] and update the [DataState] with the new information.
/// Additionally, the block that is contained within the [Leaf] will be
/// computed into a [BlockDetail] and sent to the [Sink] so that it can be
/// processed for real-time considerations.
async fn process_incoming_leaf_and_block<BDSink, BVSink>(
    leaf: Leaf1QueryData<SeqTypes>,
    block: BlockQueryData<SeqTypes>,
    data_state: Arc<RwLock<DataState>>,
    client: surf_disco::Client<hotshot_query_service::Error, Version01>,
    hotshot_config: &PublicHotShotConfig,
    mut block_sender: BDSink,
    mut voters_sender: BVSink,
) -> Result<(), ProcessLeafError>
where
    Header: BlockHeader<SeqTypes> + QueryableHeader<SeqTypes> + ExplorerHeader<SeqTypes>,
    Payload: BlockPayload<SeqTypes>,
    BDSink: Sink<BlockDetail<SeqTypes>, Error = SendError> + Unpin,
    BVSink: Sink<BitVec<u16>, Error = SendError> + Unpin,
{
    let block_detail = create_block_detail_from_block(&block);
    let block_detail_copy = create_block_detail_from_block(&block);

    let certificate = leaf.leaf().justify_qc();
    let signatures = &certificate.signatures;

    // Let's take a look at the quorum certificate signatures.
    // It looks like all of these blocks are being decided by the
    // same Quorum Certificate.

    // Where's the stake table?
    let signatures = signatures.as_ref();

    // Let's determine the participants of the voter participants
    // in the Quorum Certificate.

    // We shouldn't ever have a BitVec that is empty, with the possible
    // exception of the genesis block.
    let stake_table_voters_bit_vec = signatures.map_or(Default::default(), |sig| sig.1.clone());

    // This BitVec should be in the same order as the Stake Table.
    // The StakeTable will be able to change its order between epochs,
    // which means that its order can change between blocks.
    // However, the BitVec is a really nice size in order for storing
    // information.  We should be able to remap the BitVec order from
    // the StakeTable order to our installed order representation.  This
    // should allow us to still store as a BitVec while containing our
    // out order of the voters.
    // We will need to recompute these BitVecs if the node information that
    // is stored shrinks instead of growing.

    let block_height = block.header().height();

    // We want to check to see if we need a new stake table before we process
    // the block and leaf.  Otherwise we might have an outdated stake-table
    // and may potentially result in miss-mapped staking entries.
    perform_stake_table_epoch_check_and_update(
        data_state.clone(),
        client.clone(),
        hotshot_config,
        block_height,
    )
    .await?;

    let mut data_state_write_lock_guard = data_state.write().await;

    let stake_table = &data_state_write_lock_guard.stake_table;
    let stable_table_entries_vec = stake_table
        .try_iter(SnapshotVersion::Head)
        .map_or(vec![], |into_iter| into_iter.collect::<Vec<_>>());

    // We have a BitVec of voters who signed the QC.
    // We can use this to determine the weight of the QC
    let stake_table_entry_voter_participation_and_entries_pairs =
        zip(stake_table_voters_bit_vec, stable_table_entries_vec);
    let stake_table_keys_that_voted = stake_table_entry_voter_participation_and_entries_pairs
        .filter(|(bit_ref, _)| *bit_ref)
        .map(|(_, entry)| {
            // Alright this is our entry that we care about.
            // In this case, we just want to determine who voted for this
            // Leaf.

            let (key, ..): (BLSPubKey, _, _) = entry;
            key
        });

    let voters_set: HashSet<BLSPubKey> = stake_table_keys_that_voted.collect();

    let voters_bitvec = data_state_write_lock_guard.node_identity.iter().fold(
        BitVec::with_capacity(data_state_write_lock_guard.node_identity.len()),
        |mut acc, node_identity| {
            acc.push(voters_set.contains(node_identity.public_key()));
            acc
        },
    );

    data_state_write_lock_guard
        .latest_blocks
        .push_back(block_detail);
    data_state_write_lock_guard
        .latest_voters
        .push_back(voters_bitvec.clone());

    drop(data_state_write_lock_guard);

    if let Err(err) = block_sender.send(block_detail_copy).await {
        // We have an error that prevents us from continuing
        return Err(ProcessLeafError::BlockSendError(err));
    }

    if let Err(err) = voters_sender.send(voters_bitvec).await {
        // We have an error that prevents us from continuing
        return Err(ProcessLeafError::VotersSendError(err));
    }

    Ok(())
}

/// [ProcessLeafAndBlockPairStreamTask] represents the task that is responsible
/// for processing a stream of incoming pairs of [Leaf]s and [BlockQueryData].
pub struct ProcessLeafAndBlockPairStreamTask {
    pub task_handle: Option<JoinHandle<()>>,
}

impl ProcessLeafAndBlockPairStreamTask {
    /// [new] creates a new [ProcessLeafStreamTask] that will process a stream
    /// of incoming [Leaf]s.
    ///
    /// Calling this function will create an asynchronous task that will start
    /// processing immediately. The handle for the task will be stored within
    /// the returned structure.
    pub fn new<S, K1, K2>(
        leaf_receiver: S,
        data_state: Arc<RwLock<DataState>>,
        client: surf_disco::Client<hotshot_query_service::Error, Version01>,
        hotshot_config: PublicHotShotConfig,
        block_detail_sender: K1,
        voters_sender: K2,
    ) -> Self
    where
        S: Stream<Item = LeafAndBlock<SeqTypes>> + Send + Sync + Unpin + 'static,
        K1: Sink<BlockDetail<SeqTypes>, Error = SendError> + Clone + Send + Sync + Unpin + 'static,
        K2: Sink<BitVec<u16>, Error = SendError> + Clone + Send + Sync + Unpin + 'static,
    {
        let task_handle = spawn(Self::process_leaf_stream(
            leaf_receiver,
            data_state.clone(),
            client,
            hotshot_config,
            block_detail_sender,
            voters_sender,
        ));

        Self {
            task_handle: Some(task_handle),
        }
    }

    /// [process_leaf_stream] allows for the consumption of a [Stream] when
    /// attempting to process new incoming [Leaf]s.
    async fn process_leaf_stream<S, BDSink, BVSink>(
        mut stream: S,
        data_state: Arc<RwLock<DataState>>,
        client: surf_disco::Client<hotshot_query_service::Error, Version01>,
        hotshot_config: PublicHotShotConfig,
        block_sender: BDSink,
        voters_senders: BVSink,
    ) where
        S: Stream<Item = LeafAndBlock<SeqTypes>> + Unpin,
        Header: BlockHeader<SeqTypes> + QueryableHeader<SeqTypes> + ExplorerHeader<SeqTypes>,
        Payload: BlockPayload<SeqTypes>,
        BDSink: Sink<BlockDetail<SeqTypes>, Error = SendError> + Clone + Unpin,
        BVSink: Sink<BitVec<u16>, Error = SendError> + Clone + Unpin,
    {
        loop {
            let leaf_result = stream.next().await;
            let (leaf, block) = if let Some(pair) = leaf_result {
                pair
            } else {
                // We have reached the end of the stream
                tracing::error!("process leaf stream: end of stream reached for leaf stream.");
                return;
            };

            if let Err(err) = process_incoming_leaf_and_block(
                leaf,
                block,
                data_state.clone(),
                client.clone(),
                &hotshot_config,
                block_sender.clone(),
                voters_senders.clone(),
            )
            .await
            {
                // We have an error that prevents us from continuing
                tracing::error!("process leaf stream: error processing leaf: {}", err);

                // At the moment, all underlying errors are due to `SendError`
                // which will ultimately mean that further processing attempts
                // will fail, and be fruitless.
                match err {
                    ProcessLeafError::BlockSendError(_) => {
                        panic!("ProcessLeafStreamTask: process_incoming_leaf failed, underlying sink is closed, blocks will stagnate: {}", err)
                    },
                    ProcessLeafError::VotersSendError(_) => {
                        panic!("ProcessLeafStreamTask: process_incoming_leaf failed, underlying sink is closed, voters will stagnate: {}", err)
                    },
                    ProcessLeafError::FailedToGetNewStakeTable => {
                        panic!("ProcessLeafStreamTask: process_incoming_leaf failed, underlying stake table is closed, blocks will stagnate: {}", err)
                    },
                }
            }
        }
    }
}

/// [Drop] implementation for [ProcessLeafStreamTask] that will cancel the
/// task if it is dropped.
impl Drop for ProcessLeafAndBlockPairStreamTask {
    fn drop(&mut self) {
        let task_handle = self.task_handle.take();
        if let Some(task_handle) = task_handle {
            task_handle.abort();
        }
    }
}

/// [ProcessNodeIdentityError] represents the error that can occur when processing
/// a [NodeIdentity].
#[derive(Debug)]
pub enum ProcessNodeIdentityError {
    SendError(SendError),
}

impl std::fmt::Display for ProcessNodeIdentityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessNodeIdentityError::SendError(err) => {
                write!(f, "error sending node identity to sender: {}", err)
            },
        }
    }
}

impl std::error::Error for ProcessNodeIdentityError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ProcessNodeIdentityError::SendError(err) => Some(err),
        }
    }
}

impl From<SendError> for ProcessNodeIdentityError {
    fn from(err: SendError) -> Self {
        ProcessNodeIdentityError::SendError(err)
    }
}

/// [process_incoming_node_identity] is a helper function that will process an
/// incoming [NodeIdentity] and update the [DataState] with the new information.
/// Additionally, the [NodeIdentity] will be sent to the [Sink] so that it can
/// be processed for real-time considerations.
async fn process_incoming_node_identity<NISink>(
    node_identity: NodeIdentity,
    data_state: Arc<RwLock<DataState>>,
    mut node_identity_sender: NISink,
) -> Result<(), ProcessNodeIdentityError>
where
    NISink: Sink<NodeIdentity, Error = SendError> + Unpin,
{
    let mut data_state_write_lock_guard = data_state.write().await;
    data_state_write_lock_guard.add_node_identity(node_identity.clone());
    node_identity_sender.send(node_identity).await?;

    Ok(())
}

/// [ProcessNodeIdentityStreamTask] represents the task that is responsible for
/// processing a stream of incoming [NodeIdentity]s and updating the [DataState]
/// with the new information.
pub struct ProcessNodeIdentityStreamTask {
    pub task_handle: Option<JoinHandle<()>>,
}

impl ProcessNodeIdentityStreamTask {
    /// [new] creates a new [ProcessNodeIdentityStreamTask] that will process a
    /// stream of incoming [NodeIdentity]s.
    ///
    /// Calling this function will create an asynchronous task that will start
    /// processing immediately. The handle for the task will be stored within
    /// the returned structure.
    pub fn new<S, K>(
        node_identity_receiver: S,
        data_state: Arc<RwLock<DataState>>,
        node_identity_sender: K,
    ) -> Self
    where
        S: Stream<Item = NodeIdentity> + Send + Sync + Unpin + 'static,
        K: Sink<NodeIdentity, Error = SendError> + Clone + Send + Sync + Unpin + 'static,
    {
        let task_handle = spawn(Self::process_node_identity_stream(
            node_identity_receiver,
            data_state.clone(),
            node_identity_sender,
        ));

        Self {
            task_handle: Some(task_handle),
        }
    }

    /// [process_node_identity_stream] allows for the consumption of a [Stream] when
    /// attempting to process new incoming [NodeIdentity]s.
    /// This function will process the incoming [NodeIdentity] and update the
    /// [DataState] with the new information.
    /// Additionally, the [NodeIdentity] will be sent to the [Sink] so that it can
    /// be processed for real-time considerations.
    async fn process_node_identity_stream<S, NISink>(
        mut stream: S,
        data_state: Arc<RwLock<DataState>>,
        node_identity_sender: NISink,
    ) where
        S: Stream<Item = NodeIdentity> + Unpin,
        NISink: Sink<NodeIdentity, Error = SendError> + Clone + Unpin,
    {
        loop {
            let node_identity_result = stream.next().await;
            let node_identity = if let Some(node_identity) = node_identity_result {
                node_identity
            } else {
                // We have reached the end of the stream
                tracing::info!(
                    "process node identity stream: end of stream reached for node identity stream."
                );
                return;
            };

            if let Err(err) = process_incoming_node_identity(
                node_identity,
                data_state.clone(),
                node_identity_sender.clone(),
            )
            .await
            {
                // We have an error that prevents us from continuing
                tracing::error!(
                    "process node identity stream: error processing node identity: {}",
                    err
                );

                // The only underlying class of errors that can be returned from
                // `process_incoming_node_identity` are due to `SendError` which
                // will ultimately mean that further processing attempts will fail
                // and be fruitless.
                panic!("ProcessNodeIdentityStreamTask: process_incoming_node_identity failed, underlying sink is closed, node identities will stagnate: {}", err);
            }
        }
    }
}

/// [Drop] implementation for [ProcessNodeIdentityStreamTask] that will cancel
/// the task if it is dropped.
impl Drop for ProcessNodeIdentityStreamTask {
    fn drop(&mut self) {
        let task_handle = self.task_handle.take();
        if let Some(task_handle) = task_handle {
            task_handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use async_lock::RwLock;
    use espresso_types::{
        v0_1::RewardMerkleTree, v0_99::ChainConfig, BlockMerkleTree, FeeMerkleTree, NodeState,
        ValidatedState,
    };
    use futures::{channel::mpsc, SinkExt, StreamExt};
    use hotshot_example_types::node_types::TestVersions;
    use hotshot_query_service::{
        availability::{BlockQueryData, Leaf1QueryData},
        testing::mocks::MockVersions,
    };
    use hotshot_types::{
        data::Leaf2, signature_key::BLSPubKey, traits::signature_key::SignatureKey,
    };
    use tokio::time::timeout;
    use url::Url;

    use super::{DataState, ProcessLeafAndBlockPairStreamTask};
    use crate::{
        api::node_validator::v0::PublicHotShotConfig,
        service::data_state::{LocationDetails, NodeIdentity, ProcessNodeIdentityStreamTask},
    };

    #[tokio::test(flavor = "multi_thread")]
    async fn test_process_leaf_error_debug() {
        let (mut sender, receiver) = mpsc::channel(1);
        // deliberately close the receiver.
        drop(receiver);

        // Attempt to receive, and we should get an error.
        let receive_result = sender.send(1).await;

        assert!(receive_result.is_err());
        let err = receive_result.unwrap_err();

        let process_leaf_err = super::ProcessLeafError::BlockSendError(err);

        assert_eq!(
            format!("{:?}", process_leaf_err),
            "BlockSendError(SendError { kind: Disconnected })"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_process_leaf_stream() {
        let data_state: DataState = Default::default();
        let data_state = Arc::new(RwLock::new(data_state));
        let (block_sender, block_receiver) = futures::channel::mpsc::channel(1);
        let (voters_sender, voters_receiver) = futures::channel::mpsc::channel(1);
        let (leaf_sender, leaf_receiver) = futures::channel::mpsc::channel(1);

        let mut process_leaf_stream_task_handle = ProcessLeafAndBlockPairStreamTask::new(
            leaf_receiver,
            data_state.clone(),
            surf_disco::Client::new("http://localhost/".parse().unwrap()),
            PublicHotShotConfig {
                epoch_start_block: None,
                epoch_height: None,
                known_nodes_with_stake: vec![],
            },
            block_sender,
            voters_sender,
        );

        {
            let data_state = data_state.read().await;
            // Latest blocks should be empty
            assert_eq!(data_state.latest_blocks().count(), 0);
            // Latest voters should be empty
            assert_eq!(data_state.latest_voters().count(), 0);
        }

        let validated_state = ValidatedState {
            block_merkle_tree: BlockMerkleTree::new(32),
            fee_merkle_tree: FeeMerkleTree::new(32),
            reward_merkle_tree: RewardMerkleTree::new(32),
            chain_config: ChainConfig::default().into(),
        };
        let instance_state = NodeState::mock();

        let sample_leaf = Leaf2::genesis::<TestVersions>(&validated_state, &instance_state).await;
        let sample_block_query_data =
            BlockQueryData::genesis::<MockVersions>(&validated_state, &instance_state).await;

        let mut leaf_sender = leaf_sender;
        // We should be able to send a leaf without issue
        assert_eq!(
            leaf_sender
                .send((
                    Leaf1QueryData::new(
                        sample_leaf.clone().to_leaf_unsafe(),
                        sample_leaf.justify_qc().to_qc()
                    ),
                    sample_block_query_data
                ))
                .await,
            Ok(()),
        );

        let mut block_receiver = block_receiver;
        // We should receive a Block Detail.

        let next_block = block_receiver.next().await;
        assert!(next_block.is_some());

        let mut voters_receiver = voters_receiver;
        // We should receive a BitVec of voters.
        let next_voters = voters_receiver.next().await;
        assert!(next_voters.is_some());

        {
            let data_state = data_state.read().await;
            // Latest blocks should now have a single entry
            assert_eq!(data_state.latest_blocks().count(), 1);
            // Latest voters should now have a single entry
            assert_eq!(data_state.latest_voters().count(), 1);
        }

        // We explicitly drop these, as it should make the task clean up.
        drop(block_receiver);
        drop(leaf_sender);

        assert!(timeout(
            Duration::from_millis(200),
            process_leaf_stream_task_handle.task_handle.take().unwrap()
        )
        .await
        .is_ok());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_process_node_identity_stream() {
        let data_state: DataState = Default::default();
        let data_state = Arc::new(RwLock::new(data_state));
        let (node_identity_sender_1, node_identity_receiver_1) = futures::channel::mpsc::channel(1);
        let (node_identity_sender_2, node_identity_receiver_2) = futures::channel::mpsc::channel(1);

        let mut process_node_identity_task_handle = ProcessNodeIdentityStreamTask::new(
            node_identity_receiver_1,
            data_state.clone(),
            node_identity_sender_2,
        );

        {
            let data_state = data_state.read().await;
            // Latest blocks should be empty
            assert_eq!(data_state.node_identity().count(), 0);
        }

        // Send a node update to the Stream
        let public_key_1 = BLSPubKey::generated_from_seed_indexed([0; 32], 0).0;
        let node_identity_1 = NodeIdentity::from_public_key(public_key_1);

        let mut node_identity_sender_1 = node_identity_sender_1;
        let mut node_identity_receiver_2 = node_identity_receiver_2;

        assert_eq!(
            node_identity_sender_1.send(node_identity_1.clone()).await,
            Ok(())
        );

        assert_eq!(
            node_identity_receiver_2.next().await,
            Some(node_identity_1.clone())
        );

        {
            let data_state = data_state.read().await;
            // Latest blocks should now have a single entry
            assert_eq!(data_state.node_identity().count(), 1);
            assert_eq!(data_state.node_identity().next(), Some(&node_identity_1));
        }

        // If we send the same node identity again, we should not have a new entry.
        assert_eq!(
            node_identity_sender_1.send(node_identity_1.clone()).await,
            Ok(())
        );

        assert_eq!(
            node_identity_receiver_2.next().await,
            Some(node_identity_1.clone())
        );

        {
            let data_state = data_state.read().await;
            // Latest blocks should now have a single entry
            assert_eq!(data_state.node_identity().count(), 1);
            assert_eq!(data_state.node_identity().next(), Some(&node_identity_1));
        }

        // If we send an update for that node instead, it should update the
        // entry.
        let node_identity_1 = NodeIdentity::new(
            public_key_1,
            Some("name".to_string()),
            Some(Url::parse("https://example.com/").unwrap()),
            Some("company".to_string()),
            Some(Url::parse("https://example.com/").unwrap()),
            Some(LocationDetails::new(
                Some((40.7128, -74.0060)),
                Some("US".to_string()),
            )),
            Some("operating_system".to_string()),
            Some("node_type".to_string()),
            Some("network_type".to_string()),
        );
        assert_eq!(
            node_identity_sender_1.send(node_identity_1.clone()).await,
            Ok(())
        );

        assert_eq!(
            node_identity_receiver_2.next().await,
            Some(node_identity_1.clone())
        );

        {
            let data_state = data_state.read().await;
            // Latest blocks should now have a single entry
            assert_eq!(data_state.node_identity().count(), 1);
            assert_eq!(data_state.node_identity().next(), Some(&node_identity_1));
        }

        // If we send a new node identity, it should result in a new node
        // identity

        let public_key_2 = BLSPubKey::generated_from_seed_indexed([0; 32], 1).0;
        let node_identity_2 = NodeIdentity::from_public_key(public_key_2);

        assert_eq!(
            node_identity_sender_1.send(node_identity_2.clone()).await,
            Ok(())
        );

        assert_eq!(
            node_identity_receiver_2.next().await,
            Some(node_identity_2.clone())
        );

        {
            let data_state = data_state.read().await;
            // Latest blocks should now have a single entry
            assert_eq!(data_state.node_identity().count(), 2);
            assert_eq!(data_state.node_identity().next(), Some(&node_identity_1));
            assert_eq!(data_state.node_identity().last(), Some(&node_identity_2));
        }

        if let Some(process_node_identity_task_handle) =
            process_node_identity_task_handle.task_handle.take()
        {
            process_node_identity_task_handle.abort();
        }
    }
}
