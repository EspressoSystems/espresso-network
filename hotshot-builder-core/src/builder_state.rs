use core::panic;
use std::{
    cmp::PartialEq,
    collections::{hash_map::Entry, HashMap, HashSet, VecDeque},
    fmt::Debug,
    marker::PhantomData,
    sync::Arc,
    time::{Duration, Instant},
};

use async_broadcast::{broadcast, Receiver as BroadcastReceiver, Sender as BroadcastSender};
use async_lock::RwLock;
use committable::{Commitment, Committable};
use futures::StreamExt;
use hotshot_types::{
    data::{DaProposal2, Leaf2, QuorumProposalWrapper},
    message::Proposal,
    traits::{
        block_contents::{BlockHeader, BlockPayload},
        node_implementation::{ConsensusTime, NodeType, Versions},
        EncodeBytes,
    },
    utils::BuilderCommitment,
};
use marketplace_builder_shared::block::{BlockId, BuilderStateId, ParentBlockReferences};
use tokio::{
    spawn,
    sync::{mpsc::UnboundedSender, oneshot},
    time::sleep,
};

use crate::service::{GlobalState, ReceivedTransaction};

pub type TxTimeStamp = u128;

/// Enum to hold the different sources of the transaction
#[derive(Clone, Debug, PartialEq)]
pub enum TransactionSource {
    External, // txn from the external source i.e private mempool
    HotShot,  // txn from the HotShot network i.e public mempool
}

/// Decide Message to be put on the decide channel
#[derive(Clone, Debug)]
pub struct DecideMessage<Types: NodeType> {
    pub latest_decide_view_number: Types::View,
}
/// DA Proposal Message to be put on the da proposal channel
#[derive(Clone, Debug, PartialEq)]
pub struct DaProposalMessage<Types: NodeType> {
    pub proposal: Arc<Proposal<Types, DaProposal2<Types>>>,
    pub sender: Types::SignatureKey,
}
/// Quorum proposal message to be put on the quorum proposal channel
#[derive(Clone, Debug, PartialEq)]
pub struct QuorumProposalMessage<Types: NodeType> {
    pub proposal: Arc<Proposal<Types, QuorumProposalWrapper<Types>>>,
    pub sender: Types::SignatureKey,
}
/// Request Message to be put on the request channel
#[derive(Clone, Debug)]
pub struct RequestMessage<Types: NodeType> {
    pub state_id: BuilderStateId<Types>,
    pub response_channel: UnboundedSender<ResponseMessage>,
}

#[derive(Debug)]
pub enum TriggerStatus {
    Start,
    Exit,
}

/// Response Message to be put on the response channel
#[derive(Debug)]
pub struct BuildBlockInfo<Types: NodeType> {
    pub id: BlockId<Types>,
    pub block_size: u64,
    pub offered_fee: u64,
    pub block_payload: Types::BlockPayload,
    pub metadata: <<Types as NodeType>::BlockPayload as BlockPayload<Types>>::Metadata,
    pub vid_trigger: oneshot::Sender<TriggerStatus>,
    // TODO Add precompute back.
    // Could we have included more transactions, but chose not to?
    pub truncated: bool,
}

/// Response Message to be put on the response channel
#[derive(Debug, Clone)]
pub struct ResponseMessage {
    pub builder_hash: BuilderCommitment,
    pub block_size: u64,
    pub offered_fee: u64,
}
#[derive(Debug, Clone)]
/// Enum to hold the status out of the decide event
pub enum Status {
    ShouldExit,
    ShouldContinue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DAProposalInfo<Types: NodeType> {
    pub view_number: Types::View,
    pub proposal: Arc<Proposal<Types, DaProposal2<Types>>>,
}

/// [`ALLOW_EMPTY_BLOCK_PERIOD`] is a constant that is used to determine the
/// number of future views that we will allow building empty blocks for.
///
/// This value governs the ability for the Builder to prioritize finalizing
/// transactions by producing empty blocks rather than avoiding the creation
/// of them, following the proposal that contains transactions.
pub(crate) const ALLOW_EMPTY_BLOCK_PERIOD: u64 = 3;

#[derive(Debug)]
pub struct BuilderState<Types: NodeType, V: Versions> {
    /// Recent included txs set while building blocks
    pub included_txns: HashSet<Commitment<Types::Transaction>>,

    /// Old txs to be garbage collected
    pub included_txns_old: HashSet<Commitment<Types::Transaction>>,

    /// Expiring txs to be garbage collected
    pub included_txns_expiring: HashSet<Commitment<Types::Transaction>>,

    /// txns currently in the `tx_queue`
    pub txns_in_queue: HashSet<Commitment<Types::Transaction>>,

    /// filtered queue of available transactions, taken from `tx_receiver`
    pub tx_queue: VecDeque<Arc<ReceivedTransaction<Types>>>,

    /// `da_proposal_payload_commit` to (`da_proposal`, `node_count`)
    #[allow(clippy::type_complexity)]
    pub da_proposal_payload_commit_to_da_proposal:
        HashMap<(BuilderCommitment, Types::View), DAProposalInfo<Types>>,

    /// `quorum_proposal_payload_commit` to `quorum_proposal`
    #[allow(clippy::type_complexity)]
    pub quorum_proposal_payload_commit_to_quorum_proposal: HashMap<
        (BuilderCommitment, Types::View),
        Arc<Proposal<Types, QuorumProposalWrapper<Types>>>,
    >,

    /// Spawned-from references to the parent block.
    pub parent_block_references: ParentBlockReferences<Types>,

    // Channel Receivers for the HotShot events, Tx_receiver could also receive the external transactions
    /// decide receiver
    pub decide_receiver: BroadcastReceiver<MessageType<Types>>,

    /// da proposal receiver
    pub da_proposal_receiver: BroadcastReceiver<MessageType<Types>>,

    /// quorum proposal receiver
    pub quorum_proposal_receiver: BroadcastReceiver<MessageType<Types>>,

    /// channel receiver for the block requests
    pub req_receiver: BroadcastReceiver<MessageType<Types>>,

    /// incoming stream of transactions
    pub tx_receiver: BroadcastReceiver<Arc<ReceivedTransaction<Types>>>,

    /// global state handle, defined in the service.rs
    pub global_state: Arc<RwLock<GlobalState<Types>>>,

    /// locally spawned builder Commitements
    pub builder_commitments: HashSet<(BuilderStateId<Types>, BuilderCommitment)>,

    /// timeout for maximising the txns in the block
    pub maximize_txn_capture_timeout: Duration,

    /// constant fee that the builder will offer per byte of data sequenced
    pub base_fee: u64,

    /// validated state that is required for a proposal to be considered valid. Needed for the
    /// purposes of building a valid block payload within the sequencer.
    pub validated_state: Arc<Types::ValidatedState>,

    /// instance state to enforce `max_block_size`
    pub instance_state: Arc<Types::InstanceState>,

    /// txn garbage collection every duration time
    pub txn_garbage_collect_duration: Duration,

    /// time of next garbage collection for txns
    pub next_txn_garbage_collect_time: Instant,

    /// `allow_empty_block_until` is a variable that dictates the time until which
    /// a builder should stop producing empty blocks. This is done specifically
    /// to allow for faster finalization of previous blocks that have had
    /// transactions included in them.
    pub allow_empty_block_until: Option<Types::View>,

    phantom: PhantomData<V>,
}

/// [`best_builder_states_to_extend`] is a utility function that is used to
/// in order to determine which [`BuilderState`]s are the best fit to extend
/// from.
///
/// This function is designed to inspect the current state of the global state
/// in order to determine which [`BuilderState`]s are the best fit to extend
/// from. We only want to use information from [`GlobalState`] as otherwise
/// we would have some insider knowledge unique to our specific [`BuilderState`]
/// rather than knowledge that is available to all [`BuilderState`]s. In fact,
/// in order to ensure this, this function lives outside of the [`BuilderState`]
/// itself.
///
/// In an ideal circumstance the best [`BuilderState`] to extend from is going to
/// be the one that is immediately preceding the [`QuorumProposalWrapper`] that we are
/// attempting to extend from. However, if all we know is the view number of
/// the [`QuorumProposalWrapper`] that we are attempting to extend from, then we may end
/// up in a scenario where we have multiple [`BuilderState`]s that are all equally
/// valid to extend from.  When this happens, we have the potential for a data
/// race.
///
/// The primary cause of this has to due with the interface of the
/// [`ProxyGlobalState`](crate::service::ProxyGlobalState)'s API.  In general,
/// we want to be able to retrieve a [`BuilderState`] via the [`BuilderStateId`].
/// The [`BuilderStateId`] only references a [`ViewNumber`](hotshot_types::data::ViewNumber)
/// and a [`VidCommitment`] While this information is available in the [`QuorumProposalWrapper`],
/// it only helps us to rule out [`BuilderState`]s that already exist.
/// It does **NOT** help us to pick a [`BuilderState`] that is the best fit to extend from.
///
/// This is where the `justify_qc` comes in to consideration.  The `justify_qc`
/// contains the previous [`ViewNumber`](hotshot_types::data::ViewNumber) that is
/// being extended from, and in addition it also contains the previous [`Commitment<Leaf<Types>>`]
/// that is being built on top of.  Since our [`BuilderState`]s store identifying
/// information that contains this same `leaf_commit` we can compare these
/// directly to ensure that we are extending from the correct [`BuilderState`].
///
/// This function determines the best [`BuilderState`] in the following steps:
///
/// 1. If we have a [`BuilderState`] that is already spawned for the current
///    [`QuorumProposalWrapper`], then we should should return no states, as one already
///    exists.  This will prevent us from attempting to spawn duplicate
///    [`BuilderState`]s.
/// 2. Attempt to find all [`BuilderState`]s that are recorded within
///    [`GlobalState`] that have matching view number and leaf commitments. There
///    *should* only be one of these.  But all would be valid extension points.
/// 3. If we can't find any [`BuilderState`]s that match the view number
///    and leaf commitment, then we should return for the maximum stored view
///    number that is smaller than the current [`QuorumProposalWrapper`].
/// 4. If there is is only one [`BuilderState`] stored in the [`GlobalState`], then
///    we should return that [`BuilderState`] as the best fit.
/// 5. If none of the other criteria match, we return an empty result as it is
///    unclear what to do in this case.
///
/// > Note: Any time this function returns more than a single entry in its
/// > [HashSet] result, there is a potential for a race condition.  This is
/// > because there are multiple [BuilderState]s that are equally valid to
/// > extend from.  This race could be avoided by just picking one of the
/// > entries in the resulting [HashSet], but this is not done here in order
/// > to allow us to highlight the possibility of the race.
async fn best_builder_states_to_extend<Types: NodeType>(
    quorum_proposal: Arc<Proposal<Types, QuorumProposalWrapper<Types>>>,
    global_state: Arc<RwLock<GlobalState<Types>>>,
) -> HashSet<BuilderStateId<Types>> {
    let current_view_number = quorum_proposal.data.view_number();
    let current_commitment = quorum_proposal.data.block_header().payload_commitment();
    let current_builder_state_id = BuilderStateId::<Types> {
        parent_commitment: current_commitment,
        parent_view: current_view_number,
    };

    let global_state_read_lock = global_state.read_arc().await;

    // The first step is to check if we already have a spawned [BuilderState].
    // If we do, then we should indicate that there is no best fit, as we
    // don't want to spawn another [BuilderState].
    if global_state_read_lock
        .spawned_builder_states
        .contains_key(&current_builder_state_id)
    {
        // We already have a spawned [BuilderState] for this proposal.
        // So we should just ignore it.
        return HashSet::new();
    }

    // Next we want to see if there is an immediate match for a [BuilderState]
    // that we can extend from.  This is the most ideal situation, as it
    // implies that we are extending from the correct [BuilderState].
    // We do this by checking the `justify_qc` stored within the
    // [QuorumProposal], and checking it against the current spawned
    // [BuilderState]s
    let justify_qc = quorum_proposal.data.justify_qc();
    let existing_states: HashSet<_> = global_state_read_lock
        .spawned_builder_states
        .iter()
        .filter(
            |(_, (parent_block_references, _))| match parent_block_references {
                None => false,
                Some(parent_block_references) => {
                    parent_block_references.leaf_commit == justify_qc.data.leaf_commit
                        && parent_block_references.view_number == justify_qc.view_number
                },
            },
        )
        .map(|(builder_state_id, _)| builder_state_id.clone())
        .collect();

    // If we found any matching [BuilderState]s, then we should return them
    // as the best fit.
    if !existing_states.is_empty() {
        return existing_states;
    }

    // At this point, we don't have any "ideal" matches or scenarios.  So we
    // need to look for a suitable fall-back. The best fallback condition to
    // start with is any [BuilderState] that has the maximum spawned view
    // number whose value is smaller than the current [QuorumProposal].
    let maximum_stored_view_number_smaller_than_quorum_proposal = global_state_read_lock
        .spawned_builder_states
        .keys()
        .map(|builder_state_id| *builder_state_id.parent_view)
        .filter(|view_number| *view_number < *current_view_number)
        .max();

    // If we have a maximum view number that meets our criteria, then we should
    // return all [BuilderStateId]s that match this view number.
    // This can lead to multiple [BuilderStateId]s being returned.
    if let Some(maximum_stored_view_number_smaller_than_quorum_proposal) =
        maximum_stored_view_number_smaller_than_quorum_proposal
    {
        // If we are the maximum stored view number smaller than the quorum
        // proposal's view number, then we are the best fit.
        let mut result = HashSet::new();
        for builder_state_id in
            global_state_read_lock
                .spawned_builder_states
                .keys()
                .filter(|builder_state_id| {
                    builder_state_id.parent_view.u64()
                        == maximum_stored_view_number_smaller_than_quorum_proposal
                })
        {
            result.insert(builder_state_id.clone());
        }
        return result;
    }

    // This is our last ditch effort to continue making progress.  If there is
    // only one [BuilderState] active, then we should return that as the best
    // fit, as it will be the only way we can continue making progress with
    // the builder.
    if global_state_read_lock.spawned_builder_states.len() == 1 {
        let mut result = HashSet::new();
        for builder_state_id in global_state_read_lock.spawned_builder_states.keys() {
            result.insert(builder_state_id.clone());
        }
        return result;
    }

    // This implies that there are only larger [BuilderState]s active than
    // the one we are.  This is weird, it implies that some sort of time
    // travel has occurred view-wise.  It is unclear what to do in this
    // situation.

    HashSet::new()
}

impl<Types: NodeType, V: Versions> BuilderState<Types, V> {
    /// Utility method that attempts to determine whether
    /// we are among the best [`BuilderState`]s to extend from.
    async fn am_i_the_best_builder_state_to_extend(
        &self,
        quorum_proposal: Arc<Proposal<Types, QuorumProposalWrapper<Types>>>,
    ) -> bool {
        let best_builder_states_to_extend =
            best_builder_states_to_extend(quorum_proposal.clone(), self.global_state.clone()).await;

        tracing::debug!(
            "{}@{} thinks these are the best builder states to extend from: {:?} for proposal {}@{}",
            self.parent_block_references.vid_commitment,
            self.parent_block_references.view_number.u64(),
            best_builder_states_to_extend
                .iter()
                .map(|builder_state_id| format!(
                    "{}@{}",
                    builder_state_id.parent_commitment,
                    builder_state_id.parent_view.u64()
                ))
                .collect::<Vec<String>>(),
            quorum_proposal.data.block_header().payload_commitment(),
            quorum_proposal.data.view_number().u64(),
        );

        // We are a best fit if we are contained within the returned set of
        // best [BuilderState]s to extend from.
        best_builder_states_to_extend.contains(&BuilderStateId {
            parent_commitment: self.parent_block_references.vid_commitment,
            parent_view: self.parent_block_references.view_number,
        })
    }

    /// processing the DA proposal
    #[tracing::instrument(skip_all, name = "process da proposal",
                                    fields(builder_parent_block_references = %self.parent_block_references))]
    async fn process_da_proposal(&mut self, da_msg: DaProposalMessage<Types>) {
        tracing::debug!(
            "Builder Received DA message for view {:?}",
            da_msg.proposal.data.view_number
        );

        // we do not have the option to ignore DA proposals if we want to be able to handle failed view reorgs.

        // If the respective builder state exists to handle the request
        let proposal = da_msg.proposal.clone();

        // get the view number and encoded txns from the da_proposal_data
        let view_number = proposal.data.view_number;
        let encoded_txns = &proposal.data.encoded_transactions;

        let metadata = &proposal.data.metadata;

        // form a block payload from the encoded transactions
        let block_payload =
            <Types::BlockPayload as BlockPayload<Types>>::from_bytes(encoded_txns, metadata);
        // get the builder commitment from the block payload
        let payload_builder_commitment = block_payload.builder_commitment(metadata);

        tracing::debug!(
            "Extracted builder commitment from the da proposal: {:?}",
            payload_builder_commitment
        );

        // form the DA proposal info
        let da_proposal_info = DAProposalInfo {
            view_number,
            proposal,
        };

        let std::collections::hash_map::Entry::Vacant(e) = self
            .da_proposal_payload_commit_to_da_proposal
            .entry((payload_builder_commitment.clone(), view_number))
        else {
            tracing::debug!("Payload commitment already exists in the da_proposal_payload_commit_to_da_proposal hashmap, so ignoring it");
            return;
        };

        // if we have matching da and quorum proposals, we can skip storing the one, and remove
        // the other from storage, and call build_block with both, to save a little space.

        let Entry::Occupied(quorum_proposal) = self
            .quorum_proposal_payload_commit_to_quorum_proposal
            .entry((payload_builder_commitment.clone(), view_number))
        else {
            e.insert(da_proposal_info);
            return;
        };

        let quorum_proposal = quorum_proposal.remove();

        // if we have a matching quorum proposal
        //  if (this is the correct parent or
        //      (the correct parent is missing and this is the highest view))
        //    spawn a clone
        if quorum_proposal.data.view_number() != view_number {
            tracing::debug!("Not spawning a clone despite matching DA and quorum payload commitments, as they corresponds to different view numbers");
            return;
        }

        tracing::info!(
            "Spawning a clone from process DA proposal for view number: {:?}",
            view_number
        );
        // remove this entry from quorum_proposal_payload_commit_to_quorum_proposal
        self.quorum_proposal_payload_commit_to_quorum_proposal
            .remove(&(payload_builder_commitment.clone(), view_number));
        self.spawn_clone_that_extends_self(da_proposal_info, quorum_proposal.clone())
            .await;
    }

    /// processing the quorum proposal
    //#[tracing::instrument(skip_all, name = "Process Quorum Proposal")]
    #[tracing::instrument(skip_all, name = "process quorum proposal",
                                    fields(builder_parent_block_references = %self.parent_block_references))]
    async fn process_quorum_proposal(&mut self, quorum_msg: QuorumProposalMessage<Types>) {
        tracing::debug!(
            "Builder Received Quorum proposal message for view {:?}",
            quorum_msg.proposal.data.view_number()
        );

        // Two cases to handle:
        // Case 1: Bootstrapping phase
        // Case 2: No intended builder state exist
        // To handle both cases, we can have the highest view number builder state running
        // and only doing the insertion if and only if intended builder state for a particulat view is not present
        // check the presence of quorum_proposal.data.view_number-1 in the spawned_builder_states list
        let quorum_proposal = &quorum_msg.proposal;
        let view_number = quorum_proposal.data.view_number();
        let payload_builder_commitment = quorum_proposal.data.block_header().builder_commitment();

        tracing::debug!(
            "Extracted payload builder commitment from the quorum proposal: {:?}",
            payload_builder_commitment
        );

        let std::collections::hash_map::Entry::Vacant(e) = self
            .quorum_proposal_payload_commit_to_quorum_proposal
            .entry((payload_builder_commitment.clone(), view_number))
        else {
            tracing::debug!("Payload commitment already exists in the quorum_proposal_payload_commit_to_quorum_proposal hashmap, so ignoring it");
            return;
        };

        // first check whether vid_commitment exists in the quorum_proposal_payload_commit_to_quorum_proposal hashmap, if yer, ignore it, otherwise validate it and later insert in
        // if we have matching da and quorum proposals, we can skip storing the one, and remove the other from storage, and call build_block with both, to save a little space.
        let Entry::Occupied(da_proposal) = self
            .da_proposal_payload_commit_to_da_proposal
            .entry((payload_builder_commitment.clone(), view_number))
        else {
            e.insert(quorum_proposal.clone());
            return;
        };

        let da_proposal_info = da_proposal.remove();
        // remove the entry from the da_proposal_payload_commit_to_da_proposal hashmap
        self.da_proposal_payload_commit_to_da_proposal
            .remove(&(payload_builder_commitment.clone(), view_number));

        // also make sure we clone for the same view number( check incase payload commitments are same)
        if da_proposal_info.view_number != view_number {
            tracing::debug!("Not spawning a clone despite matching DA and quorum payload commitments, as they corresponds to different view numbers");
            return;
        }

        tracing::info!(
            "Spawning a clone from process quorum proposal for view number: {:?}",
            view_number
        );

        self.spawn_clone_that_extends_self(da_proposal_info, quorum_proposal.clone())
            .await;
    }

    /// A helper function that is used by both [`BuilderState::process_da_proposal`]
    /// and [`BuilderState::process_quorum_proposal`] to spawn a new [`BuilderState`]
    /// that extends from the current [`BuilderState`].
    ///
    /// This helper function also adds additional checks in order to ensure
    /// that the [`BuilderState`] that is being spawned is the best fit for the
    /// [`QuorumProposalWrapper`] that is being extended from.
    async fn spawn_clone_that_extends_self(
        &mut self,
        da_proposal_info: DAProposalInfo<Types>,
        quorum_proposal: Arc<Proposal<Types, QuorumProposalWrapper<Types>>>,
    ) {
        if !self
            .am_i_the_best_builder_state_to_extend(quorum_proposal.clone())
            .await
        {
            tracing::debug!(
                "{} is not the best fit for forking, {}@{}, so ignoring the quorum proposal, and leaving it to another BuilderState",
                self.parent_block_references,
                quorum_proposal.data.block_header().payload_commitment(),
                quorum_proposal.data.view_number().u64(),
            );
            return;
        }

        let (req_sender, req_receiver) = broadcast(self.req_receiver.capacity());

        tracing::debug!(
            "extending BuilderState with a clone from {} with new proposal {}@{}",
            self.parent_block_references,
            quorum_proposal.data.block_header().payload_commitment(),
            quorum_proposal.data.view_number().u64()
        );
        // We literally fork ourselves
        self.clone_with_receiver(req_receiver)
            .spawn_clone(da_proposal_info, quorum_proposal.clone(), req_sender)
            .await;
    }

    /// processing the decide event
    #[tracing::instrument(skip_all, name = "process decide event",
                                   fields(builder_parent_block_references = %self.parent_block_references))]
    async fn process_decide_event(&mut self, decide_msg: DecideMessage<Types>) -> Option<Status> {
        // Exit out all the builder states if their parent_block_references.view_number is less than the latest_decide_view_number
        // The only exception is that we want to keep the highest view number builder state active to ensure that
        // we have a builder state to handle the incoming DA and quorum proposals
        let decide_view_number = decide_msg.latest_decide_view_number;

        let retained_view_cutoff = self
            .global_state
            .write_arc()
            .await
            .remove_handles(decide_view_number);
        if self.parent_block_references.view_number < retained_view_cutoff {
            tracing::info!(
                "Decide@{:?}; Task@{:?} exiting; views < {:?} being reclaimed",
                decide_view_number.u64(),
                self.parent_block_references.view_number.u64(),
                retained_view_cutoff.u64(),
            );
            return Some(Status::ShouldExit);
        }
        tracing::info!(
            "Decide@{:?}; Task@{:?} not exiting; views >= {:?} being retained",
            decide_view_number.u64(),
            self.parent_block_references.view_number.u64(),
            retained_view_cutoff.u64(),
        );

        Some(Status::ShouldContinue)
    }

    // spawn a clone of the builder state
    #[tracing::instrument(skip_all, name = "spawn_clone",
                                    fields(builder_parent_block_references = %self.parent_block_references))]
    async fn spawn_clone(
        mut self,
        da_proposal_info: DAProposalInfo<Types>,
        quorum_proposal: Arc<Proposal<Types, QuorumProposalWrapper<Types>>>,
        req_sender: BroadcastSender<MessageType<Types>>,
    ) {
        let leaf = Leaf2::from_quorum_proposal(&quorum_proposal.data);

        // We replace our parent_block_references with information from the
        // quorum proposal.  This is identifying the block that this specific
        // instance of [BuilderState] is attempting to build for.
        self.parent_block_references = ParentBlockReferences {
            view_number: quorum_proposal.data.view_number(),
            vid_commitment: quorum_proposal.data.block_header().payload_commitment(),
            leaf_commit: leaf.commit(),
            builder_commitment: quorum_proposal.data.block_header().builder_commitment(),
            // Unused in old legacy builder:
            last_nonempty_view: None,
            tx_count: 0,
        };

        let builder_state_id = BuilderStateId {
            parent_commitment: self.parent_block_references.vid_commitment,
            parent_view: self.parent_block_references.view_number,
        };

        {
            // Let's ensure that we don't already have one of these BuilderStates
            // running already.

            let global_state_read_lock = self.global_state.read_arc().await;
            if global_state_read_lock
                .spawned_builder_states
                .contains_key(&builder_state_id)
            {
                tracing::warn!(
                    "Aborting spawn_clone, builder state already exists in spawned_builder_states: {:?}",
                    builder_state_id
                );
                return;
            }
        }

        let encoded_txns = &da_proposal_info.proposal.data.encoded_transactions;
        let metadata = &da_proposal_info.proposal.data.metadata;

        let block_payload =
            <Types::BlockPayload as BlockPayload<Types>>::from_bytes(encoded_txns, metadata);
        let txn_commitments = block_payload.transaction_commitments(metadata);

        for tx in txn_commitments.iter() {
            self.txns_in_queue.remove(tx);
        }

        self.included_txns.extend(txn_commitments.iter());
        self.tx_queue
            .retain(|tx| self.txns_in_queue.contains(&tx.commit));

        if !txn_commitments.is_empty() {
            self.allow_empty_block_until = Some(Types::View::new(
                da_proposal_info.view_number.u64() + ALLOW_EMPTY_BLOCK_PERIOD,
            ));
        }

        // register the spawned builder state to spawned_builder_states in the global state
        self.global_state.write_arc().await.register_builder_state(
            BuilderStateId {
                parent_commitment: self.parent_block_references.vid_commitment,
                parent_view: self.parent_block_references.view_number,
            },
            self.parent_block_references.clone(),
            req_sender,
        );

        self.event_loop();
    }

    // build a block
    #[tracing::instrument(skip_all, name = "build block",
                                    fields(builder_parent_block_references = %self.parent_block_references))]
    async fn build_block(
        &mut self,
        state_id: BuilderStateId<Types>,
    ) -> Option<BuildBlockInfo<Types>> {
        let timeout_after = Instant::now() + self.maximize_txn_capture_timeout;
        let sleep_interval = self.maximize_txn_capture_timeout / 10;
        while Instant::now() <= timeout_after {
            self.collect_txns(timeout_after).await;

            if !self.tx_queue.is_empty() // we have transactions
            || Instant::now() + sleep_interval > timeout_after
            // we don't have time for another iteration
            {
                break;
            }

            sleep(sleep_interval).await
        }

        // should_prioritize_finalization is a flag that is used to determine
        // whether we should return empty blocks or not.

        let should_prioritize_finalization = self
            .allow_empty_block_until
            .map(|until| state_id.parent_view < until)
            .unwrap_or(false);

        if self.tx_queue.is_empty() && !should_prioritize_finalization {
            // Don't build an empty block
            return None;
        }

        let max_block_size = self
            .global_state
            .read_arc()
            .await
            .block_size_limits
            .max_block_size;
        let transactions_to_include = self.tx_queue.iter().scan(0, |total_size, tx| {
            let prev_size = *total_size;
            *total_size += tx.len;
            // We will include one transaction over our target block length
            // if it's the first transaction in queue, otherwise we'd have a possible failure
            // state where a single transaction larger than target block state is stuck in
            // queue and we just build empty blocks forever
            if *total_size >= max_block_size && prev_size != 0 {
                None
            } else {
                Some(tx.tx.clone())
            }
        });

        let Ok((payload, metadata)) =
            <Types::BlockPayload as BlockPayload<Types>>::from_transactions(
                transactions_to_include,
                &self.validated_state,
                &self.instance_state,
            )
            .await
        else {
            tracing::warn!("build block, returning None");
            return None;
        };

        let builder_hash = payload.builder_commitment(&metadata);
        // count the number of txns
        let actual_txn_count = payload.num_transactions(&metadata);

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
        // We need to drop it and mark as "included" so that if we receive
        // it again we don't even bother with it.
        if actual_txn_count == 0 && !should_prioritize_finalization {
            if let Some(txn) = self.tx_queue.pop_front() {
                self.txns_in_queue.remove(&txn.commit);
                self.included_txns.insert(txn.commit);
            };
            return None;
        }

        // insert the recently built block into the builder commitments
        self.builder_commitments
            .insert((state_id, builder_hash.clone()));

        let encoded_txns: Vec<u8> = payload.encode().to_vec();
        let block_size: u64 = encoded_txns.len() as u64;
        let offered_fee: u64 = self.base_fee * block_size;

        // TODO Add precompute back.
        let (trigger_send, _) = oneshot::channel();

        tracing::info!(
            "Builder view num {:?}, building block with {:?} txns, with builder hash {:?}",
            self.parent_block_references.view_number,
            actual_txn_count,
            builder_hash
        );

        Some(BuildBlockInfo {
            id: BlockId {
                view: self.parent_block_references.view_number,
                hash: builder_hash,
            },
            block_size,
            offered_fee,
            block_payload: payload,
            metadata,
            vid_trigger: trigger_send,
            truncated: actual_txn_count < self.tx_queue.len(),
        })
    }

    async fn process_block_request(&mut self, req: RequestMessage<Types>) {
        // If a spawned clone is active then it will handle the request, otherwise the highest view num builder will handle
        if req.state_id.parent_commitment != self.parent_block_references.vid_commitment
            || req.state_id.parent_view != self.parent_block_references.view_number
        {
            tracing::debug!(
                "Builder {:?} Requested Builder commitment does not match the built_from_view, so ignoring it",
                self.parent_block_references.view_number
            );
            return;
        }

        let highest_view_num_builder_id = self
            .global_state
            .read_arc()
            .await
            .highest_view_num_builder_id
            .clone();

        if highest_view_num_builder_id.parent_view != self.parent_block_references.view_number {
            tracing::debug!(
                "Builder {:?} Requested Builder commitment does not match the highest_view_num_builder_id, so ignoring it",
                self.parent_block_references.view_number
            );
            return;
        }

        tracing::info!(
            "Request for parent {} handled by builder with view {:?}",
            req.state_id,
            self.parent_block_references.view_number,
        );
        let response = self.build_block(req.state_id.clone()).await;

        let Some(response) = response else {
            tracing::debug!("No response to send");
            return;
        };

        // form the response message
        let response_msg = ResponseMessage {
            builder_hash: response.id.hash.clone(),
            block_size: response.block_size,
            offered_fee: response.offered_fee,
        };

        let builder_hash = response.id.hash.clone();
        self.global_state.write_arc().await.update_global_state(
            req.state_id.clone(),
            response,
            response_msg.clone(),
        );

        // ... and finally, send the response
        if let Err(e) = req.response_channel.send(response_msg) {
            tracing::warn!(
                "Builder {:?} failed to send response to {:?} with builder hash {:?}, Err: {:?}",
                self.parent_block_references.view_number,
                req,
                builder_hash,
                e
            );
            return;
        }

        tracing::info!(
            "Builder {:?} Sent response to the request{:?} with builder hash {:?}",
            self.parent_block_references.view_number,
            req,
            builder_hash
        );
    }

    #[tracing::instrument(skip_all, name = "event loop",
                                    fields(builder_parent_block_references = %self.parent_block_references))]
    pub fn event_loop(mut self) {
        let _builder_handle = spawn(async move {
            loop {
                tracing::debug!(
                    "Builder {:?} event loop",
                    self.parent_block_references.view_number
                );
                futures::select! {
                    req = self.req_receiver.next() => {
                        tracing::debug!("Received request msg in builder {:?}: {:?}", self.parent_block_references.view_number, req);
                        match req {
                            Some(req) => {
                                if let MessageType::RequestMessage(req) = req {
                                    tracing::debug!(
                                        "Received request msg in builder {:?}: {:?}",
                                        self.parent_block_references.view_number,
                                        req
                                    );
                                    self.process_block_request(req).await;
                                } else {
                                    tracing::warn!("Unexpected message on requests channel: {:?}", req);
                                }
                            }
                            None => {
                                tracing::warn!("No more request messages to consume");
                            }
                        }
                    },
                    da = self.da_proposal_receiver.next() => {
                        match da {
                            Some(da) => {
                                if let MessageType::DaProposalMessage(rda_msg) = da {
                                    tracing::debug!("Received da proposal msg in builder {:?}:\n {:?}", self.parent_block_references, rda_msg.proposal.data.view_number);
                                    self.process_da_proposal(rda_msg).await;
                                } else {
                                    tracing::warn!("Unexpected message on da proposals channel: {:?}", da);
                                }
                            }
                            None => {
                                tracing::warn!("No more da proposal messages to consume");
                            }
                        }
                    },
                    quorum = self.quorum_proposal_receiver.next() => {
                        match quorum {
                            Some(quorum) => {
                                if let MessageType::QuorumProposalMessage(rquorum_msg) = quorum {
                                    tracing::debug!("Received quorum proposal msg in builder {:?}:\n {:?} for view ", self.parent_block_references, rquorum_msg.proposal.data.view_number());
                                    self.process_quorum_proposal(rquorum_msg).await;
                                } else {
                                    tracing::warn!("Unexpected message on quorum proposals channel: {:?}", quorum);
                                }
                            }
                            None => {
                                tracing::warn!("No more quorum proposal messages to consume");
                            }
                        }
                    },
                    decide = self.decide_receiver.next() => {
                        match decide {
                            Some(decide) => {
                                if let MessageType::DecideMessage(rdecide_msg) = decide {
                                    let latest_decide_view_num = rdecide_msg.latest_decide_view_number;
                                    tracing::debug!("Received decide msg view {:?} in builder {:?}",
                                        &latest_decide_view_num,
                                        self.parent_block_references);
                                    let decide_status = self.process_decide_event(rdecide_msg).await;
                                    match decide_status{
                                        Some(Status::ShouldExit) => {
                                            tracing::info!("Exiting builder {:?} with decide view {:?}",
                                                self.parent_block_references,
                                                &latest_decide_view_num);
                                            return;
                                        }
                                        Some(Status::ShouldContinue) => {
                                            tracing::debug!("Continuing builder {:?}",
                                                self.parent_block_references);
                                            continue;
                                        }
                                        None => {
                                            tracing::warn!("decide_status was None; Continuing builder {:?}",
                                                self.parent_block_references);
                                            continue;
                                        }
                                    }
                                } else {
                                    tracing::warn!("Unexpected message on decide channel: {:?}", decide);
                                }
                            }
                            None => {
                                tracing::warn!("No more decide messages to consume");
                            }
                        }
                    },
                };
            }
        });
    }
}
/// Unifies the possible messages that can be received by the builder
#[derive(Debug, Clone)]
pub enum MessageType<Types: NodeType> {
    DecideMessage(DecideMessage<Types>),
    DaProposalMessage(DaProposalMessage<Types>),
    QuorumProposalMessage(QuorumProposalMessage<Types>),
    RequestMessage(RequestMessage<Types>),
}

#[allow(clippy::too_many_arguments)]
impl<Types: NodeType, V: Versions> BuilderState<Types, V> {
    pub fn new(
        parent_block_references: ParentBlockReferences<Types>,
        decide_receiver: BroadcastReceiver<MessageType<Types>>,
        da_proposal_receiver: BroadcastReceiver<MessageType<Types>>,
        quorum_proposal_receiver: BroadcastReceiver<MessageType<Types>>,
        req_receiver: BroadcastReceiver<MessageType<Types>>,
        tx_receiver: BroadcastReceiver<Arc<ReceivedTransaction<Types>>>,
        tx_queue: VecDeque<Arc<ReceivedTransaction<Types>>>,
        global_state: Arc<RwLock<GlobalState<Types>>>,
        maximize_txn_capture_timeout: Duration,
        base_fee: u64,
        instance_state: Arc<Types::InstanceState>,
        txn_garbage_collect_duration: Duration,
        validated_state: Arc<Types::ValidatedState>,
    ) -> Self {
        let txns_in_queue: HashSet<_> = tx_queue.iter().map(|tx| tx.commit).collect();
        BuilderState {
            included_txns: HashSet::new(),
            included_txns_old: HashSet::new(),
            included_txns_expiring: HashSet::new(),
            txns_in_queue,
            parent_block_references,
            decide_receiver,
            da_proposal_receiver,
            quorum_proposal_receiver,
            req_receiver,
            da_proposal_payload_commit_to_da_proposal: HashMap::new(),
            quorum_proposal_payload_commit_to_quorum_proposal: HashMap::new(),
            tx_receiver,
            tx_queue,
            global_state,
            builder_commitments: HashSet::new(),
            maximize_txn_capture_timeout,
            base_fee,
            instance_state,
            txn_garbage_collect_duration,
            next_txn_garbage_collect_time: Instant::now() + txn_garbage_collect_duration,
            validated_state,
            allow_empty_block_until: None,
            phantom: PhantomData,
        }
    }
    pub fn clone_with_receiver(&self, req_receiver: BroadcastReceiver<MessageType<Types>>) -> Self {
        // Handle the garbage collection of txns
        let (
            included_txns,
            included_txns_old,
            included_txns_expiring,
            next_txn_garbage_collect_time,
        ) = if Instant::now() >= self.next_txn_garbage_collect_time {
            (
                HashSet::new(),
                self.included_txns.clone(),
                self.included_txns_old.clone(),
                Instant::now() + self.txn_garbage_collect_duration,
            )
        } else {
            (
                self.included_txns.clone(),
                self.included_txns_old.clone(),
                self.included_txns_expiring.clone(),
                self.next_txn_garbage_collect_time,
            )
        };

        BuilderState {
            included_txns,
            included_txns_old,
            included_txns_expiring,
            txns_in_queue: self.txns_in_queue.clone(),
            parent_block_references: self.parent_block_references.clone(),
            decide_receiver: self.decide_receiver.clone(),
            da_proposal_receiver: self.da_proposal_receiver.clone(),
            quorum_proposal_receiver: self.quorum_proposal_receiver.clone(),
            req_receiver,
            da_proposal_payload_commit_to_da_proposal: HashMap::new(),
            quorum_proposal_payload_commit_to_quorum_proposal: HashMap::new(),
            tx_receiver: self.tx_receiver.clone(),
            tx_queue: self.tx_queue.clone(),
            global_state: self.global_state.clone(),
            builder_commitments: self.builder_commitments.clone(),
            maximize_txn_capture_timeout: self.maximize_txn_capture_timeout,
            base_fee: self.base_fee,
            instance_state: self.instance_state.clone(),
            txn_garbage_collect_duration: self.txn_garbage_collect_duration,
            next_txn_garbage_collect_time,
            validated_state: self.validated_state.clone(),
            allow_empty_block_until: self.allow_empty_block_until,
            phantom: PhantomData,
        }
    }

    // collect outstanding transactions
    async fn collect_txns(&mut self, timeout_after: Instant) {
        while Instant::now() <= timeout_after {
            match self.tx_receiver.try_recv() {
                Ok(tx) => {
                    if self.included_txns.contains(&tx.commit)
                        || self.included_txns_old.contains(&tx.commit)
                        || self.included_txns_expiring.contains(&tx.commit)
                        || self.txns_in_queue.contains(&tx.commit)
                    {
                        continue;
                    }
                    self.txns_in_queue.insert(tx.commit);
                    self.tx_queue.push_back(tx);
                },
                Err(async_broadcast::TryRecvError::Empty)
                | Err(async_broadcast::TryRecvError::Closed) => {
                    break;
                },
                Err(async_broadcast::TryRecvError::Overflowed(lost)) => {
                    tracing::warn!("Missed {lost} transactions due to backlog");
                    continue;
                },
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use async_broadcast::broadcast;
    use committable::RawCommitmentBuilder;
    use hotshot_example_types::{
        block_types::TestTransaction,
        node_types::{TestTypes, TestVersions},
    };
    use hotshot_types::{
        data::{Leaf2, QuorumProposalWrapper, ViewNumber},
        traits::node_implementation::{ConsensusTime, NodeType},
        utils::BuilderCommitment,
    };
    use marketplace_builder_shared::testing::constants::TEST_NUM_NODES_IN_VID_COMPUTATION;
    use tracing_subscriber::EnvFilter;

    use super::{DAProposalInfo, MessageType, ParentBlockReferences};
    use crate::testing::{calc_builder_commitment, calc_proposal_msg, create_builder_state};

    /// This test the function `process_da_proposal`.
    /// It checks da_proposal_payload_commit_to_da_proposal change appropriately
    /// when receiving a da proposal message.
    /// This test also checks whether corresponding BuilderStateId is in global_state.
    #[tokio::test]
    async fn test_process_da_proposal() {
        // Setup logging
        let _ = tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .try_init();
        tracing::info!("Testing the function `process_da_proposal` in `builder_state.rs`");

        // Number of views to simulate
        const NUM_ROUNDS: usize = 5;
        // Capacity of broadcast channels
        const CHANNEL_CAPACITY: usize = NUM_ROUNDS * 5;
        // Number of nodes on DA committee
        const NUM_STORAGE_NODES: usize = TEST_NUM_NODES_IN_VID_COMPUTATION;

        // create builder_state without entering event loop
        let (_senders, global_state, mut builder_state) =
            create_builder_state::<TestVersions>(CHANNEL_CAPACITY, NUM_STORAGE_NODES).await;

        // randomly generate a transaction
        let transactions = vec![TestTransaction::new(vec![1, 2, 3]); 3];
        let (_quorum_proposal, _quorum_proposal_msg, da_proposal_msg, builder_state_id) =
            calc_proposal_msg::<TestVersions>(NUM_STORAGE_NODES, 0, None, transactions.clone())
                .await;

        // sub-test one
        // call process_da_proposal without matching quorum proposal message
        // da_proposal_payload_commit_to_da_proposal should insert the message
        let mut correct_da_proposal_payload_commit_to_da_proposal: HashMap<
            (BuilderCommitment, <TestTypes as NodeType>::View),
            DAProposalInfo<TestTypes>,
        > = HashMap::new();
        let (payload_builder_commitment, da_proposal_info) =
            calc_builder_commitment(da_proposal_msg.clone()).await;

        builder_state
            .process_da_proposal(da_proposal_msg.clone())
            .await;
        correct_da_proposal_payload_commit_to_da_proposal.insert(
            (
                payload_builder_commitment,
                da_proposal_msg.proposal.data.view_number,
            ),
            da_proposal_info,
        );

        assert_eq!(
            builder_state.da_proposal_payload_commit_to_da_proposal,
            correct_da_proposal_payload_commit_to_da_proposal
        );
        // check global_state didn't change
        if global_state
            .read_arc()
            .await
            .spawned_builder_states
            .contains_key(&builder_state_id)
        {
            panic!("global_state shouldn't have corresponding builder_state_id without matching quorum proposal.");
        }

        // sub-test two
        // call process_da_proposal with the same msg again
        // we should skip the process and everything should be the same
        let transactions_1 = transactions.clone();
        let (_quorum_proposal_1, _quorum_proposal_msg_1, da_proposal_msg_1, builder_state_id_1) =
            calc_proposal_msg::<TestVersions>(NUM_STORAGE_NODES, 0, None, transactions_1).await;
        builder_state
            .process_da_proposal(da_proposal_msg_1.clone())
            .await;
        assert_eq!(
            builder_state.da_proposal_payload_commit_to_da_proposal,
            correct_da_proposal_payload_commit_to_da_proposal
        );
        // check global_state didn't change
        if global_state
            .read_arc()
            .await
            .spawned_builder_states
            .contains_key(&builder_state_id_1)
        {
            panic!("global_state shouldn't have corresponding builder_state_id without matching quorum proposal.");
        }

        // sub-test three
        // add the matching quorum proposal message with different tx
        // and call process_da_proposal with this matching da proposal message and quorum proposal message
        // we should spawn_clone here
        // and check whether global_state has correct BuilderStateId
        let transactions_2 = vec![TestTransaction::new(vec![1, 2, 3, 4]); 2];
        let (_quorum_proposal_2, quorum_proposal_msg_2, da_proposal_msg_2, builder_state_id_2) =
            calc_proposal_msg::<TestVersions>(NUM_STORAGE_NODES, 0, None, transactions_2).await;

        // process quorum proposal first, so that later when process_da_proposal we can directly call `build_block` and skip storage
        builder_state
            .process_quorum_proposal(quorum_proposal_msg_2.clone())
            .await;

        // process da proposal message and do the check
        builder_state
            .process_da_proposal(da_proposal_msg_2.clone())
            .await;
        assert_eq!(
            builder_state.da_proposal_payload_commit_to_da_proposal,
            correct_da_proposal_payload_commit_to_da_proposal,
        );
        // check global_state has this new builder_state_id
        if global_state
            .read_arc()
            .await
            .spawned_builder_states
            .contains_key(&builder_state_id_2)
        {
            tracing::debug!("global_state updated successfully");
        } else {
            panic!("global_state should have corresponding builder_state_id as now we have matching quorum proposal.");
        }
    }

    /// This test the function `process_quorum_proposal`.
    /// It checks quorum_proposal_payload_commit_to_quorum_proposal change appropriately
    /// when receiving a quorum proposal message.
    /// This test also checks whether corresponding BuilderStateId is in global_state.
    #[tokio::test]
    async fn test_process_quorum_proposal() {
        // Setup logging
        let _ = tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .try_init();

        tracing::info!("Testing the function `process_quorum_proposal` in `builder_state.rs`");

        // Number of views to simulate
        const NUM_ROUNDS: usize = 5;
        // Capacity of broadcast channels
        const CHANNEL_CAPACITY: usize = NUM_ROUNDS * 5;
        // Number of nodes on DA committee
        const NUM_STORAGE_NODES: usize = TEST_NUM_NODES_IN_VID_COMPUTATION;

        // create builder_state without entering event loop
        let (_senders, global_state, mut builder_state) =
            create_builder_state::<TestVersions>(CHANNEL_CAPACITY, NUM_STORAGE_NODES).await;

        // randomly generate a transaction
        let transactions = vec![TestTransaction::new(vec![1, 2, 3]); 3];
        let (_quorum_proposal, quorum_proposal_msg, _da_proposal_msg, builder_state_id) =
            calc_proposal_msg::<TestVersions>(NUM_STORAGE_NODES, 0, None, transactions.clone())
                .await;

        // sub-test one
        // call process_quorum_proposal without matching da proposal message
        // quorum_proposal_payload_commit_to_quorum_proposal should insert the message
        let mut correct_quorum_proposal_payload_commit_to_quorum_proposal = HashMap::new();
        builder_state
            .process_quorum_proposal(quorum_proposal_msg.clone())
            .await;
        correct_quorum_proposal_payload_commit_to_quorum_proposal.insert(
            (
                quorum_proposal_msg
                    .proposal
                    .data
                    .block_header()
                    .builder_commitment
                    .clone(),
                quorum_proposal_msg.proposal.data.view_number(),
            ),
            quorum_proposal_msg.proposal,
        );
        assert_eq!(
            builder_state
                .quorum_proposal_payload_commit_to_quorum_proposal
                .clone(),
            correct_quorum_proposal_payload_commit_to_quorum_proposal.clone()
        );
        // check global_state didn't change
        if global_state
            .read_arc()
            .await
            .spawned_builder_states
            .contains_key(&builder_state_id)
        {
            panic!("global_state shouldn't have corresponding builder_state_id without matching quorum proposal.");
        }

        // sub-test two
        // add the matching da proposal message with different tx
        // and call process_da_proposal with this matching quorum proposal message and quorum da message
        // we should spawn_clone here
        // and check whether global_state has correct BuilderStateId
        let transactions_2 = vec![TestTransaction::new(vec![2, 3, 4]); 2];
        let (_quorum_proposal_2, quorum_proposal_msg_2, da_proposal_msg_2, builder_state_id_2) =
            calc_proposal_msg::<TestVersions>(NUM_STORAGE_NODES, 0, None, transactions_2).await;

        // process da proposal message first, so that later when process_da_proposal we can directly call `build_block` and skip storage
        builder_state
            .process_da_proposal(da_proposal_msg_2.clone())
            .await;

        // process quorum proposal, and do the check
        builder_state
            .process_quorum_proposal(quorum_proposal_msg_2.clone())
            .await;

        assert_eq!(
            builder_state
                .quorum_proposal_payload_commit_to_quorum_proposal
                .clone(),
            correct_quorum_proposal_payload_commit_to_quorum_proposal.clone()
        );

        // check global_state has this new builder_state_id
        if global_state
            .read_arc()
            .await
            .spawned_builder_states
            .contains_key(&builder_state_id_2)
        {
            tracing::debug!("global_state updated successfully");
        } else {
            panic!("global_state should have corresponding builder_state_id as now we have matching da proposal.");
        }
    }

    /// This test the function `process_decide_event`.
    /// It checks whether we exit out correct builder states when there's a decide event coming in.
    /// This test also checks whether corresponding BuilderStateId is removed in global_state.
    #[tokio::test]
    async fn test_process_decide_event() {
        // Setup logging
        let _ = tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .try_init();
        tracing::info!("Testing the builder core with multiple messages from the channels");

        // Number of views to simulate
        const NUM_ROUNDS: usize = 5;
        // Number of transactions to submit per round
        const NUM_TXNS_PER_ROUND: usize = 4;
        // Capacity of broadcast channels
        const CHANNEL_CAPACITY: usize = NUM_ROUNDS * 5;
        // Number of nodes on DA committee
        const NUM_STORAGE_NODES: usize = TEST_NUM_NODES_IN_VID_COMPUTATION;

        // create builder_state without entering event loop
        let (_senders, global_state, mut builder_state) =
            create_builder_state::<TestVersions>(CHANNEL_CAPACITY, NUM_STORAGE_NODES).await;

        // Transactions to send
        let all_transactions = (0..NUM_ROUNDS)
            .map(|round| {
                (0..NUM_TXNS_PER_ROUND)
                    .map(|tx_num| TestTransaction::new(vec![round as u8, tx_num as u8]))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let mut prev_quorum_proposal: Option<QuorumProposalWrapper<TestTypes>> = None;
        // register some builder states for later decide event
        #[allow(clippy::needless_range_loop)]
        for round in 0..NUM_ROUNDS {
            let transactions = all_transactions[round].clone();
            let (quorum_proposal, _quorum_proposal_msg, _da_proposal_msg, builder_state_id) =
                calc_proposal_msg::<TestVersions>(
                    NUM_STORAGE_NODES,
                    round,
                    prev_quorum_proposal,
                    transactions,
                )
                .await;
            prev_quorum_proposal = Some(quorum_proposal.clone());
            let (req_sender, _req_receiver) = broadcast(CHANNEL_CAPACITY);
            let leaf: Leaf2<TestTypes> = Leaf2::from_quorum_proposal(&quorum_proposal);
            let leaf_commit = RawCommitmentBuilder::new("leaf commitment")
                .u64_field("view number", leaf.view_number().u64())
                .u64_field("block number", leaf.height())
                .field("parent Leaf commitment", leaf.parent_commitment())
                .var_size_field(
                    "block payload commitment",
                    leaf.payload_commitment().as_ref(),
                )
                .finalize();
            global_state.write_arc().await.register_builder_state(
                builder_state_id,
                ParentBlockReferences {
                    view_number: quorum_proposal.view_number(),
                    vid_commitment: quorum_proposal.block_header().payload_commitment,
                    leaf_commit,
                    builder_commitment: quorum_proposal.block_header().builder_commitment.clone(),
                    // Unused in old legacy builder:
                    last_nonempty_view: None,
                    tx_count: 0,
                },
                req_sender,
            );
        }

        // send out a decide event in a middle round
        let latest_decide_view_number = ViewNumber::new(3);

        let decide_message = MessageType::DecideMessage(crate::builder_state::DecideMessage {
            latest_decide_view_number,
        });
        if let MessageType::DecideMessage(practice_decide_msg) = decide_message.clone() {
            builder_state
                .process_decide_event(practice_decide_msg.clone())
                .await;
        } else {
            panic!("Not a decide_message in correct format");
        }
        // check whether spawned_builder_states have correct builder_state_id and already exit-ed builder_states older than decides
        let current_spawned_builder_states =
            global_state.read_arc().await.spawned_builder_states.clone();
        current_spawned_builder_states
            .iter()
            .for_each(|(builder_state_id, _)| {
                assert!(builder_state_id.parent_view >= latest_decide_view_number)
            });
    }
}
