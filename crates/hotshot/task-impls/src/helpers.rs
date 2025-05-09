// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Instant,
};

use alloy::primitives::U256;
use async_broadcast::{Receiver, SendError, Sender};
use async_lock::RwLock;
use committable::{Commitment, Committable};
use hotshot_task::dependency::{Dependency, EventDependency};
use hotshot_types::{
    consensus::OuterConsensus,
    data::{Leaf2, QuorumProposalWrapper, VidDisperseShare, ViewChangeEvidence2},
    drb::{DrbResult, DrbSeedInput},
    epoch_membership::EpochMembershipCoordinator,
    event::{Event, EventType, LeafInfo},
    message::{Proposal, UpgradeLock},
    request_response::ProposalRequestPayload,
    simple_certificate::{
        DaCertificate2, LightClientStateUpdateCertificate, NextEpochQuorumCertificate2,
        QuorumCertificate2, UpgradeCertificate,
    },
    simple_vote::HasEpoch,
    stake_table::StakeTableEntries,
    traits::{
        block_contents::BlockHeader,
        election::Membership,
        node_implementation::{ConsensusTime, NodeImplementation, NodeType, Versions},
        signature_key::{SignatureKey, StakeTableEntryType, StateSignatureKey},
        storage::Storage,
        BlockPayload, ValidatedState,
    },
    utils::{
        epoch_from_block_number, is_epoch_root, is_epoch_transition, is_transition_block,
        option_epoch_from_block_number, Terminator, View, ViewInner,
    },
    vote::{Certificate, HasViewNumber},
};
use hotshot_utils::anytrace::*;
use tokio::time::timeout;
use tracing::instrument;
use vbs::version::StaticVersionType;

use crate::{events::HotShotEvent, quorum_proposal_recv::ValidationInfo, request::REQUEST_TIMEOUT};

/// Trigger a request to the network for a proposal for a view and wait for the response or timeout.
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub(crate) async fn fetch_proposal<TYPES: NodeType, V: Versions>(
    qc: &QuorumCertificate2<TYPES>,
    event_sender: Sender<Arc<HotShotEvent<TYPES>>>,
    event_receiver: Receiver<Arc<HotShotEvent<TYPES>>>,
    membership_coordinator: EpochMembershipCoordinator<TYPES>,
    consensus: OuterConsensus<TYPES>,
    sender_public_key: TYPES::SignatureKey,
    sender_private_key: <TYPES::SignatureKey as SignatureKey>::PrivateKey,
    upgrade_lock: &UpgradeLock<TYPES, V>,
    epoch_height: u64,
) -> Result<(Leaf2<TYPES>, View<TYPES>)> {
    let view_number = qc.view_number();
    let leaf_commit = qc.data.leaf_commit;
    // We need to be able to sign this request before submitting it to the network. Compute the
    // payload first.
    let signed_proposal_request = ProposalRequestPayload {
        view_number,
        key: sender_public_key,
    };

    // Finally, compute the signature for the payload.
    let signature = TYPES::SignatureKey::sign(
        &sender_private_key,
        signed_proposal_request.commit().as_ref(),
    )
    .wrap()
    .context(error!("Failed to sign proposal. This should never happen."))?;

    tracing::info!("Sending proposal request for view {}", view_number);

    // First, broadcast that we need a proposal to the current leader
    broadcast_event(
        HotShotEvent::QuorumProposalRequestSend(signed_proposal_request, signature).into(),
        &event_sender,
    )
    .await;

    let mut rx = event_receiver.clone();
    // Make a background task to await the arrival of the event data.
    let Ok(Some(proposal)) =
        // We want to explicitly timeout here so we aren't waiting around for the data.
        timeout(REQUEST_TIMEOUT, async move {
            // We want to iterate until the proposal is not None, or until we reach the timeout.
            while let Ok(event) = rx.recv_direct().await {
                if let HotShotEvent::QuorumProposalResponseRecv(quorum_proposal) = event.as_ref() {
                    let leaf = Leaf2::from_quorum_proposal(&quorum_proposal.data);
                    if leaf.view_number() == view_number && leaf.commit() == leaf_commit {
                        return Some(quorum_proposal.clone());
                    }
                }
            }
            None
        })
        .await
    else {
        bail!("Request for proposal failed");
    };

    let view_number = proposal.data.view_number();
    let justify_qc = proposal.data.justify_qc().clone();

    let justify_qc_epoch = justify_qc.data.epoch();

    let epoch_membership = membership_coordinator
        .stake_table_for_epoch(justify_qc_epoch)
        .await?;
    let membership_stake_table = epoch_membership.stake_table().await;
    let membership_success_threshold = epoch_membership.success_threshold().await;

    justify_qc
        .is_valid_cert(
            &StakeTableEntries::<TYPES>::from(membership_stake_table).0,
            membership_success_threshold,
            upgrade_lock,
        )
        .await
        .context(|e| warn!("Invalid justify_qc in proposal for view {view_number}: {e}"))?;

    let mut consensus_writer = consensus.write().await;
    let leaf = Leaf2::from_quorum_proposal(&proposal.data);
    let state = Arc::new(
        <TYPES::ValidatedState as ValidatedState<TYPES>>::from_header(proposal.data.block_header()),
    );

    if let Err(e) = consensus_writer.update_leaf(leaf.clone(), Arc::clone(&state), None) {
        tracing::trace!("{e:?}");
    }
    let view = View {
        view_inner: ViewInner::Leaf {
            leaf: leaf.commit(),
            state,
            delta: None,
            epoch: leaf.epoch(epoch_height),
        },
    };
    Ok((leaf, view))
}
pub async fn handle_drb_result<TYPES: NodeType, I: NodeImplementation<TYPES>>(
    membership: &Arc<RwLock<TYPES::Membership>>,
    epoch: TYPES::Epoch,
    storage: &I::Storage,
    consensus: &OuterConsensus<TYPES>,
    drb_result: DrbResult,
) {
    let mut consensus_writer = consensus.write().await;
    consensus_writer.drb_results.store_result(epoch, drb_result);
    drop(consensus_writer);
    tracing::debug!("Calling add_drb_result for epoch {epoch}");
    if let Err(e) = storage.add_drb_result(epoch, drb_result).await {
        tracing::error!("Failed to store drb result for epoch {epoch}: {e}");
    }

    membership.write().await.add_drb_result(epoch, drb_result)
}
/// Start the DRB computation task for the next epoch.
fn start_drb_task<TYPES: NodeType, I: NodeImplementation<TYPES>>(
    seed: DrbSeedInput,
    epoch: TYPES::Epoch,
    membership: &Arc<RwLock<TYPES::Membership>>,
    storage: &I::Storage,
    consensus: &OuterConsensus<TYPES>,
) {
    let membership = membership.clone();
    let storage = storage.clone();
    let consensus = consensus.clone();
    tokio::spawn(async move {
        let drb_result = tokio::task::spawn_blocking(move || {
            hotshot_types::drb::compute_drb_result::<TYPES>(seed)
        })
        .await
        .unwrap();

        handle_drb_result::<TYPES, I>(&membership, epoch, &storage, &consensus, drb_result).await;
        drb_result
    });
}
/// Handles calling add_epoch_root and sync_l1 on Membership if necessary.
async fn decide_epoch_root<TYPES: NodeType, I: NodeImplementation<TYPES>>(
    decided_leaf: &Leaf2<TYPES>,
    epoch_height: u64,
    membership: &Arc<RwLock<TYPES::Membership>>,
    storage: &I::Storage,
    consensus: &OuterConsensus<TYPES>,
) {
    let decided_block_number = decided_leaf.block_header().block_number();

    // Skip if this is not the expected block.
    if epoch_height != 0 && is_epoch_root(decided_block_number, epoch_height) {
        let next_epoch_number =
            TYPES::Epoch::new(epoch_from_block_number(decided_block_number, epoch_height) + 2);

        let mut start = Instant::now();
        if let Err(e) = storage
            .add_epoch_root(next_epoch_number, decided_leaf.block_header().clone())
            .await
        {
            tracing::error!("Failed to store epoch root for epoch {next_epoch_number}: {e}");
        }
        tracing::info!("Time taken to store epoch root: {:?}", start.elapsed());

        start = Instant::now();
        let write_callback = {
            tracing::debug!("Calling add_epoch_root for epoch {next_epoch_number}");
            let membership_reader = membership.read().await;
            membership_reader
                .add_epoch_root(next_epoch_number, decided_leaf.block_header().clone())
                .await
        };
        if let Some(write_callback) = write_callback {
            let mut membership_writer = membership.write().await;
            write_callback(&mut *membership_writer);
        }
        tracing::info!("Time taken to add epoch root: {:?}", start.elapsed());

        let mut consensus_writer = consensus.write().await;
        consensus_writer
            .drb_results
            .garbage_collect(next_epoch_number);
        drop(consensus_writer);

        let Ok(drb_seed_input_vec) = bincode::serialize(&decided_leaf.justify_qc().signatures)
        else {
            tracing::error!("Failed to serialize the QC signature.");
            return;
        };

        let mut drb_seed_input = [0u8; 32];
        let len = drb_seed_input_vec.len().min(32);
        drb_seed_input[..len].copy_from_slice(&drb_seed_input_vec[..len]);

        start_drb_task::<TYPES, I>(
            drb_seed_input,
            next_epoch_number,
            membership,
            storage,
            consensus,
        );
    }
}

/// Helper type to give names and to the output values of the leaf chain traversal operation.
#[derive(Debug)]
pub struct LeafChainTraversalOutcome<TYPES: NodeType> {
    /// The new locked view obtained from a 2 chain starting from the proposal's parent.
    pub new_locked_view_number: Option<TYPES::View>,

    /// The new decided view obtained from a 3 chain starting from the proposal's parent.
    pub new_decided_view_number: Option<TYPES::View>,

    /// The qc for the decided chain.
    pub new_decide_qc: Option<QuorumCertificate2<TYPES>>,

    /// The decided leaves with corresponding validated state and VID info.
    pub leaf_views: Vec<LeafInfo<TYPES>>,

    /// The transactions in the block payload for each leaf.
    pub included_txns: Option<HashSet<Commitment<<TYPES as NodeType>::Transaction>>>,

    /// The most recent upgrade certificate from one of the leaves.
    pub decided_upgrade_cert: Option<UpgradeCertificate<TYPES>>,
}

/// We need Default to be implemented because the leaf ascension has very few failure branches,
/// and when they *do* happen, we still return intermediate states. Default makes the burden
/// of filling values easier.
impl<TYPES: NodeType + Default> Default for LeafChainTraversalOutcome<TYPES> {
    /// The default method for this type is to set all of the returned values to `None`.
    fn default() -> Self {
        Self {
            new_locked_view_number: None,
            new_decided_view_number: None,
            new_decide_qc: None,
            leaf_views: Vec::new(),
            included_txns: None,
            decided_upgrade_cert: None,
        }
    }
}

/// calculate the new decided leaf chain based on the rules of HotStuff 2
///
/// # Panics
/// If the leaf chain contains no decided leaf while reaching a decided view, which should be
/// impossible.
pub async fn decide_from_proposal_2<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions>(
    proposal: &QuorumProposalWrapper<TYPES>,
    consensus: OuterConsensus<TYPES>,
    existing_upgrade_cert: Arc<RwLock<Option<UpgradeCertificate<TYPES>>>>,
    public_key: &TYPES::SignatureKey,
    with_epochs: bool,
    membership: &EpochMembershipCoordinator<TYPES>,
    storage: &I::Storage,
) -> LeafChainTraversalOutcome<TYPES> {
    let mut res = LeafChainTraversalOutcome::default();
    let consensus_reader = consensus.read().await;
    let proposed_leaf = Leaf2::from_quorum_proposal(proposal);
    res.new_locked_view_number = Some(proposed_leaf.justify_qc().view_number());

    // If we don't have the proposals parent return early
    let Some(parent_info) = consensus_reader
        .parent_leaf_info(&proposed_leaf, public_key)
        .await
    else {
        return res;
    };
    // Get the parents parent and check if it's consecutive in view to the parent, if so we can decided
    // the grandparents view.  If not we're done.
    let Some(grand_parent_info) = consensus_reader
        .parent_leaf_info(&parent_info.leaf, public_key)
        .await
    else {
        return res;
    };
    if grand_parent_info.leaf.view_number() + 1 != parent_info.leaf.view_number() {
        return res;
    }
    res.new_decide_qc = Some(parent_info.leaf.justify_qc().clone());
    let decided_view_number = grand_parent_info.leaf.view_number();
    res.new_decided_view_number = Some(decided_view_number);
    // We've reached decide, now get the leaf chain all the way back to the last decided view, not including it.
    let old_anchor_view = consensus_reader.last_decided_view();
    let mut current_leaf_info = Some(grand_parent_info);
    let existing_upgrade_cert_reader = existing_upgrade_cert.read().await;
    let mut txns = HashSet::new();
    while current_leaf_info
        .as_ref()
        .is_some_and(|info| info.leaf.view_number() > old_anchor_view)
    {
        // unwrap is safe, we just checked that he option is some
        let info = &mut current_leaf_info.unwrap();
        // Check if there's a new upgrade certificate available.
        if let Some(cert) = info.leaf.upgrade_certificate() {
            if info.leaf.upgrade_certificate() != *existing_upgrade_cert_reader {
                if cert.data.decide_by < decided_view_number {
                    tracing::warn!("Failed to decide an upgrade certificate in time. Ignoring.");
                } else {
                    tracing::info!("Reached decide on upgrade certificate: {cert:?}");
                    res.decided_upgrade_cert = Some(cert.clone());
                }
            }
        }

        // If the block payload is available for this leaf, include it in
        // the leaf chain that we send to the client.
        if let Some(payload) = consensus_reader
            .saved_payloads()
            .get(&info.leaf.view_number())
        {
            info.leaf
                .fill_block_payload_unchecked(payload.as_ref().payload.clone());
        }

        if let Some(ref payload) = info.leaf.block_payload() {
            for txn in payload.transaction_commitments(info.leaf.block_header().metadata()) {
                txns.insert(txn);
            }
        }

        current_leaf_info = consensus_reader
            .parent_leaf_info(&info.leaf, public_key)
            .await;
        res.leaf_views.push(info.clone());
    }

    if !txns.is_empty() {
        res.included_txns = Some(txns);
    }

    if with_epochs && res.new_decided_view_number.is_some() {
        let Some(first_leaf) = res.leaf_views.first() else {
            return res;
        };
        let epoch_height = consensus_reader.epoch_height;
        consensus_reader
            .metrics
            .last_synced_block_height
            .set(usize::try_from(first_leaf.leaf.height()).unwrap_or(0));
        drop(consensus_reader);

        for decided_leaf_info in &res.leaf_views {
            decide_epoch_root::<TYPES, I>(
                &decided_leaf_info.leaf,
                epoch_height,
                membership.membership(),
                storage,
                &consensus,
            )
            .await;
        }
    }

    res
}

/// Ascends the leaf chain by traversing through the parent commitments of the proposal. We begin
/// by obtaining the parent view, and if we are in a chain (i.e. the next view from the parent is
/// one view newer), then we begin attempting to form the chain. This is a direct impl from
/// [HotStuff](https://arxiv.org/pdf/1803.05069) section 5:
///
/// > When a node b* carries a QC that refers to a direct parent, i.e., b*.justify.node = b*.parent,
/// > we say that it forms a One-Chain. Denote by b'' = b*.justify.node. Node b* forms a Two-Chain,
/// > if in addition to forming a One-Chain, b''.justify.node = b''.parent.
/// > It forms a Three-Chain, if b'' forms a Two-Chain.
///
/// We follow this exact logic to determine if we are able to reach a commit and a decide. A commit
/// is reached when we have a two chain, and a decide is reached when we have a three chain.
///
/// # Example
/// Suppose we have a decide for view 1, and we then move on to get undecided views 2, 3, and 4. Further,
/// suppose that our *next* proposal is for view 5, but this leader did not see info for view 4, so the
/// justify qc of the proposal points to view 3. This is fine, and the undecided chain now becomes
/// 2-3-5.
///
/// Assuming we continue with honest leaders, we then eventually could get a chain like: 2-3-5-6-7-8. This
/// will prompt a decide event to occur (this code), where the `proposal` is for view 8. Now, since the
/// lowest value in the 3-chain here would be 5 (excluding 8 since we only walk the parents), we begin at
/// the first link in the chain, and walk back through all undecided views, making our new anchor view 5,
/// and out new locked view will be 6.
///
/// Upon receipt then of a proposal for view 9, assuming it is valid, this entire process will repeat, and
/// the anchor view will be set to view 6, with the locked view as view 7.
///
/// # Panics
/// If the leaf chain contains no decided leaf while reaching a decided view, which should be
/// impossible.
#[allow(clippy::too_many_arguments)]
pub async fn decide_from_proposal<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions>(
    proposal: &QuorumProposalWrapper<TYPES>,
    consensus: OuterConsensus<TYPES>,
    existing_upgrade_cert: Arc<RwLock<Option<UpgradeCertificate<TYPES>>>>,
    public_key: &TYPES::SignatureKey,
    with_epochs: bool,
    membership: &Arc<RwLock<TYPES::Membership>>,
    storage: &I::Storage,
    epoch_height: u64,
) -> LeafChainTraversalOutcome<TYPES> {
    let consensus_reader = consensus.read().await;
    let existing_upgrade_cert_reader = existing_upgrade_cert.read().await;
    let view_number = proposal.view_number();
    let parent_view_number = proposal.justify_qc().view_number();
    let old_anchor_view = consensus_reader.last_decided_view();

    let mut last_view_number_visited = view_number;
    let mut current_chain_length = 0usize;
    let mut res = LeafChainTraversalOutcome::default();

    if let Err(e) = consensus_reader.visit_leaf_ancestors(
        parent_view_number,
        Terminator::Exclusive(old_anchor_view),
        true,
        |leaf, state, delta| {
            // This is the core paper logic. We're implementing the chain in chained hotstuff.
            if res.new_decided_view_number.is_none() {
                // If the last view number is the child of the leaf we've moved to...
                if last_view_number_visited == leaf.view_number() + 1 {
                    last_view_number_visited = leaf.view_number();

                    // The chain grows by one
                    current_chain_length += 1;

                    // We emit a locked view when the chain length is 2
                    if current_chain_length == 2 {
                        res.new_locked_view_number = Some(leaf.view_number());
                        // The next leaf in the chain, if there is one, is decided, so this
                        // leaf's justify_qc would become the QC for the decided chain.
                        res.new_decide_qc = Some(leaf.justify_qc().clone());
                    } else if current_chain_length == 3 {
                        // And we decide when the chain length is 3.
                        res.new_decided_view_number = Some(leaf.view_number());
                    }
                } else {
                    // There isn't a new chain extension available, so we signal to the callback
                    // owner that we can exit for now.
                    return false;
                }
            }

            // Now, if we *have* reached a decide, we need to do some state updates.
            if let Some(new_decided_view) = res.new_decided_view_number {
                // First, get a mutable reference to the provided leaf.
                let mut leaf = leaf.clone();

                // Update the metrics
                if leaf.view_number() == new_decided_view {
                    consensus_reader
                        .metrics
                        .last_synced_block_height
                        .set(usize::try_from(leaf.height()).unwrap_or(0));
                }

                // Check if there's a new upgrade certificate available.
                if let Some(cert) = leaf.upgrade_certificate() {
                    if leaf.upgrade_certificate() != *existing_upgrade_cert_reader {
                        if cert.data.decide_by < view_number {
                            tracing::warn!(
                                "Failed to decide an upgrade certificate in time. Ignoring."
                            );
                        } else {
                            tracing::info!("Reached decide on upgrade certificate: {cert:?}");
                            res.decided_upgrade_cert = Some(cert.clone());
                        }
                    }
                }
                // If the block payload is available for this leaf, include it in
                // the leaf chain that we send to the client.
                if let Some(payload) = consensus_reader.saved_payloads().get(&leaf.view_number()) {
                    leaf.fill_block_payload_unchecked(payload.as_ref().payload.clone());
                }

                // Get the VID share at the leaf's view number, corresponding to our key
                // (if one exists)
                let vid_share = consensus_reader
                    .vid_shares()
                    .get(&leaf.view_number())
                    .and_then(|key_map| key_map.get(public_key))
                    .and_then(|epoch_map| epoch_map.get(&leaf.epoch(epoch_height)))
                    .map(|prop| prop.data.clone());

                let state_cert = if leaf.with_epoch
                    && is_epoch_root(
                        leaf.block_header().block_number(),
                        consensus_reader.epoch_height,
                    ) {
                    match consensus_reader.state_cert() {
                        // Sanity check that the state cert is for the same view as the decided leaf
                        Some(state_cert)
                            if state_cert.light_client_state.view_number
                                == leaf.view_number().u64() =>
                        {
                            Some(state_cert.clone())
                        },
                        _ => None,
                    }
                } else {
                    None
                };

                // Add our data into a new `LeafInfo`
                res.leaf_views.push(LeafInfo::new(
                    leaf.clone(),
                    Arc::clone(&state),
                    delta.clone(),
                    vid_share,
                    state_cert,
                ));
                if let Some(ref payload) = leaf.block_payload() {
                    res.included_txns = Some(
                        payload
                            .transaction_commitments(leaf.block_header().metadata())
                            .into_iter()
                            .collect::<HashSet<_>>(),
                    );
                }
            }
            true
        },
    ) {
        tracing::debug!("Leaf ascension failed; error={e}");
    }

    let epoch_height = consensus_reader.epoch_height;
    drop(consensus_reader);

    if with_epochs && res.new_decided_view_number.is_some() {
        for decided_leaf_info in &res.leaf_views {
            decide_epoch_root::<TYPES, I>(
                &decided_leaf_info.leaf,
                epoch_height,
                membership,
                storage,
                &consensus,
            )
            .await;
        }
    }

    res
}

/// Gets the parent leaf and state from the parent of a proposal, returning an [`utils::anytrace::Error`] if not.
#[instrument(skip_all)]
#[allow(clippy::too_many_arguments)]
pub(crate) async fn parent_leaf_and_state<TYPES: NodeType, V: Versions>(
    event_sender: &Sender<Arc<HotShotEvent<TYPES>>>,
    event_receiver: &Receiver<Arc<HotShotEvent<TYPES>>>,
    membership: EpochMembershipCoordinator<TYPES>,
    public_key: TYPES::SignatureKey,
    private_key: <TYPES::SignatureKey as SignatureKey>::PrivateKey,
    consensus: OuterConsensus<TYPES>,
    upgrade_lock: &UpgradeLock<TYPES, V>,
    parent_qc: &QuorumCertificate2<TYPES>,
    epoch_height: u64,
) -> Result<(Leaf2<TYPES>, Arc<<TYPES as NodeType>::ValidatedState>)> {
    let consensus_reader = consensus.read().await;
    let vsm_contains_parent_view = consensus_reader
        .validated_state_map()
        .contains_key(&parent_qc.view_number());
    drop(consensus_reader);

    if !vsm_contains_parent_view {
        let _ = fetch_proposal(
            parent_qc,
            event_sender.clone(),
            event_receiver.clone(),
            membership,
            consensus.clone(),
            public_key.clone(),
            private_key.clone(),
            upgrade_lock,
            epoch_height,
        )
        .await
        .context(info!("Failed to fetch proposal"))?;
    }

    let consensus_reader = consensus.read().await;
    let parent_view = consensus_reader.validated_state_map().get(&parent_qc.view_number()).context(
        debug!("Couldn't find parent view in state map, waiting for replica to see proposal; parent_view_number: {}", *parent_qc.view_number())
    )?;

    let (leaf_commitment, state) = parent_view.leaf_and_state().context(
        info!("Parent of high QC points to a view without a proposal; parent_view_number: {}, parent_view {:?}", *parent_qc.view_number(), parent_view)
    )?;

    if leaf_commitment != consensus_reader.high_qc().data().leaf_commit {
        // NOTE: This happens on the genesis block
        tracing::debug!(
            "They don't equal: {:?}   {:?}",
            leaf_commitment,
            consensus_reader.high_qc().data().leaf_commit
        );
    }

    let leaf = consensus_reader
        .saved_leaves()
        .get(&leaf_commitment)
        .context(info!("Failed to find high QC of parent"))?;

    Ok((leaf.clone(), Arc::clone(state)))
}

pub(crate) async fn update_high_qc<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions>(
    proposal: &Proposal<TYPES, QuorumProposalWrapper<TYPES>>,
    validation_info: &ValidationInfo<TYPES, I, V>,
) -> Result<()> {
    let in_transition_epoch = proposal
        .data
        .justify_qc()
        .data
        .block_number
        .is_some_and(|bn| {
            !is_transition_block(bn, validation_info.epoch_height)
                && is_epoch_transition(bn, validation_info.epoch_height)
                && bn % validation_info.epoch_height != 0
        });
    let justify_qc = proposal.data.justify_qc();
    let maybe_next_epoch_justify_qc = proposal.data.next_epoch_justify_qc();
    if !in_transition_epoch {
        tracing::debug!(
            "Storing high QC for view {:?} and height {:?}",
            justify_qc.view_number(),
            justify_qc.data.block_number
        );
        if let Err(e) = validation_info
            .storage
            .update_high_qc2(justify_qc.clone())
            .await
        {
            bail!("Failed to store High QC, not voting; error = {e:?}");
        }
        if justify_qc
            .data
            .block_number
            .is_some_and(|bn| is_epoch_root(bn, validation_info.epoch_height))
        {
            let Some(state_cert) = proposal.data.state_cert() else {
                bail!("Epoch root QC has no state cert, not voting!");
            };
            if let Err(e) = validation_info
                .storage
                .update_state_cert(state_cert.clone())
                .await
            {
                bail!("Failed to store the light client state update certificate, not voting; error = {:?}", e);
            }
            validation_info
                .consensus
                .write()
                .await
                .update_state_cert(state_cert.clone())?;
        }
        if let Some(ref next_epoch_justify_qc) = maybe_next_epoch_justify_qc {
            if let Err(e) = validation_info
                .storage
                .update_next_epoch_high_qc2(next_epoch_justify_qc.clone())
                .await
            {
                bail!("Failed to store next epoch High QC, not voting; error = {e:?}");
            }
        }
    }
    let mut consensus_writer = validation_info.consensus.write().await;
    if let Some(ref next_epoch_justify_qc) = maybe_next_epoch_justify_qc {
        if justify_qc
            .data
            .block_number
            .is_some_and(|bn| is_transition_block(bn, validation_info.epoch_height))
        {
            consensus_writer.reset_high_qc(justify_qc.clone(), next_epoch_justify_qc.clone())?;
            consensus_writer
                .update_transition_qc(justify_qc.clone(), next_epoch_justify_qc.clone());
            return Ok(());
        }
        consensus_writer.update_next_epoch_high_qc(next_epoch_justify_qc.clone())?;
    }
    consensus_writer.update_high_qc(justify_qc.clone())?;

    Ok(())
}

async fn transition_qc<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions>(
    validation_info: &ValidationInfo<TYPES, I, V>,
) -> Option<(
    QuorumCertificate2<TYPES>,
    NextEpochQuorumCertificate2<TYPES>,
)> {
    validation_info
        .consensus
        .read()
        .await
        .transition_qc()
        .cloned()
}

pub(crate) async fn validate_epoch_transition_qc<
    TYPES: NodeType,
    I: NodeImplementation<TYPES>,
    V: Versions,
>(
    proposal: &Proposal<TYPES, QuorumProposalWrapper<TYPES>>,
    validation_info: &ValidationInfo<TYPES, I, V>,
) -> Result<()> {
    let proposed_qc = proposal.data.justify_qc();
    let Some(qc_block_number) = proposed_qc.data().block_number else {
        bail!("Justify QC has no block number");
    };
    if !is_epoch_transition(qc_block_number, validation_info.epoch_height)
        || qc_block_number % validation_info.epoch_height == 0
    {
        return Ok(());
    }
    let Some(next_epoch_qc) = proposal.data.next_epoch_justify_qc() else {
        bail!("Next epoch justify QC is not present");
    };
    ensure!(
        next_epoch_qc.data.leaf_commit == proposed_qc.data().leaf_commit,
        "Next epoch QC has different leaf commit to justify QC"
    );

    if is_transition_block(qc_block_number, validation_info.epoch_height) {
        // Height is epoch height - 2
        ensure!(
            transition_qc(validation_info).await.is_none_or(
                |(qc, _)| qc.view_number() <= proposed_qc.view_number()
            ),
            "Proposed transition qc must have view number greater than or equal to previous transition QC"
        );

        validation_info
            .consensus
            .write()
            .await
            .update_transition_qc(proposed_qc.clone(), next_epoch_qc.clone());
        // reset the high qc to the transition qc
        update_high_qc(proposal, validation_info).await?;
    } else {
        // Height is either epoch height - 1 or epoch height
        ensure!(
            transition_qc(validation_info)
                .await
                .is_none_or(|(qc, _)| qc.view_number() < proposed_qc.view_number()),
            "Transition block must have view number greater than previous transition QC"
        );
        ensure!(
            proposal.data.view_change_evidence().is_none(),
            "Second to last block and last block of epoch must directly extend previous block, Qc Block number: {qc_block_number}, Proposal Block number: {}",
            proposal.data.block_header().block_number()
        );
        ensure!(
            proposed_qc.view_number() + 1 == proposal.data.view_number()
            || transition_qc(validation_info).await.is_some_and(|(qc, _)| &qc == proposed_qc),
            "Transition proposals must extend the previous view directly, or extend the previous transition block"
        );
    }
    Ok(())
}

/// Validate the state and safety and liveness of a proposal then emit
/// a `QuorumProposalValidated` event.
///
///
/// # Errors
/// If any validation or state update fails.
#[allow(clippy::too_many_lines)]
#[instrument(skip_all, fields(id = validation_info.id, view = *proposal.data.view_number()))]
pub async fn validate_proposal_safety_and_liveness<
    TYPES: NodeType,
    I: NodeImplementation<TYPES>,
    V: Versions,
>(
    proposal: Proposal<TYPES, QuorumProposalWrapper<TYPES>>,
    parent_leaf: Leaf2<TYPES>,
    validation_info: &ValidationInfo<TYPES, I, V>,
    event_stream: Sender<Arc<HotShotEvent<TYPES>>>,
    sender: TYPES::SignatureKey,
) -> Result<()> {
    let view_number = proposal.data.view_number();

    let mut valid_epoch_transition = false;
    if validation_info
        .upgrade_lock
        .version(proposal.data.justify_qc().view_number())
        .await
        .is_ok_and(|v| v >= V::Epochs::VERSION)
    {
        let Some(block_number) = proposal.data.justify_qc().data.block_number else {
            bail!("Quorum Proposal has no block number but it's after the epoch upgrade");
        };
        if is_epoch_transition(block_number, validation_info.epoch_height) {
            validate_epoch_transition_qc(&proposal, validation_info).await?;
            valid_epoch_transition = true;
        }
    }

    let proposed_leaf = Leaf2::from_quorum_proposal(&proposal.data);
    ensure!(
        proposed_leaf.parent_commitment() == parent_leaf.commit(),
        "Proposed leaf does not extend the parent leaf."
    );
    let proposal_epoch = option_epoch_from_block_number::<TYPES>(
        validation_info
            .upgrade_lock
            .epochs_enabled(view_number)
            .await,
        proposed_leaf.height(),
        validation_info.epoch_height,
    );

    let state = Arc::new(
        <TYPES::ValidatedState as ValidatedState<TYPES>>::from_header(proposal.data.block_header()),
    );

    {
        let mut consensus_writer = validation_info.consensus.write().await;
        if let Err(e) = consensus_writer.update_leaf(proposed_leaf.clone(), state, None) {
            tracing::trace!("{e:?}");
        }

        // Update our internal storage of the proposal. The proposal is valid, so
        // we swallow this error and just log if it occurs.
        if let Err(e) = consensus_writer.update_proposed_view(proposal.clone()) {
            tracing::debug!("Internal proposal update failed; error = {e:#}");
        };
    }

    UpgradeCertificate::validate(
        proposal.data.upgrade_certificate(),
        &validation_info.membership,
        proposal_epoch,
        &validation_info.upgrade_lock,
    )
    .await?;

    // Validate that the upgrade certificate is re-attached, if we saw one on the parent
    proposed_leaf
        .extends_upgrade(
            &parent_leaf,
            &validation_info.upgrade_lock.decided_upgrade_certificate,
        )
        .await?;

    let justify_qc = proposal.data.justify_qc().clone();
    // Create a positive vote if either liveness or safety check
    // passes.

    {
        let consensus_reader = validation_info.consensus.read().await;
        // Epoch safety check:
        // The proposal is safe if
        // 1. the proposed block and the justify QC block belong to the same epoch or
        // 2. the justify QC is the eQC for the previous block
        let justify_qc_epoch = option_epoch_from_block_number::<TYPES>(
            validation_info
                .upgrade_lock
                .epochs_enabled(view_number)
                .await,
            parent_leaf.height(),
            validation_info.epoch_height,
        );
        ensure!(
            proposal_epoch == justify_qc_epoch
                || consensus_reader.check_eqc(&proposed_leaf, &parent_leaf),
            {
                error!("Failed epoch safety check \n Proposed leaf is {proposed_leaf:?} \n justify QC leaf is {parent_leaf:?}")
            }
        );

        // Make sure that the epoch transition proposal includes the next epoch QC
        if is_epoch_transition(parent_leaf.height(), validation_info.epoch_height)
            && validation_info
                .upgrade_lock
                .epochs_enabled(view_number)
                .await
        {
            ensure!(proposal.data.next_epoch_justify_qc().is_some(),
            "Epoch transition proposal does not include the next epoch justify QC. Do not vote!");
        }

        // Liveness check.
        let liveness_check =
            justify_qc.view_number() > consensus_reader.locked_view() || valid_epoch_transition;

        // Safety check.
        // Check if proposal extends from the locked leaf.
        let outcome = consensus_reader.visit_leaf_ancestors(
            justify_qc.view_number(),
            Terminator::Inclusive(consensus_reader.locked_view()),
            false,
            |leaf, _, _| {
                // if leaf view no == locked view no then we're done, report success by
                // returning true
                leaf.view_number() != consensus_reader.locked_view()
            },
        );
        let safety_check = outcome.is_ok();

        ensure!(safety_check || liveness_check, {
            if let Err(e) = outcome {
                broadcast_event(
                    Event {
                        view_number,
                        event: EventType::Error { error: Arc::new(e) },
                    },
                    &validation_info.output_event_stream,
                )
                .await;
            }

            error!("Failed safety and liveness check \n High QC is {:?}  Proposal QC is {:?}  Locked view is {:?}", consensus_reader.high_qc(), proposal.data.clone(), consensus_reader.locked_view())
        });
    }

    // We accept the proposal, notify the application layer
    broadcast_event(
        Event {
            view_number,
            event: EventType::QuorumProposal {
                proposal: proposal.clone(),
                sender,
            },
        },
        &validation_info.output_event_stream,
    )
    .await;

    // Notify other tasks
    broadcast_event(
        Arc::new(HotShotEvent::QuorumProposalValidated(
            proposal.clone(),
            parent_leaf,
        )),
        &event_stream,
    )
    .await;

    Ok(())
}

/// Validates, from a given `proposal` that the view that it is being submitted for is valid when
/// compared to `cur_view` which is the highest proposed view (so far) for the caller. If the proposal
/// is for a view that's later than expected, that the proposal includes a timeout or view sync certificate.
///
/// # Errors
/// If any validation or view number check fails.
pub(crate) async fn validate_proposal_view_and_certs<
    TYPES: NodeType,
    I: NodeImplementation<TYPES>,
    V: Versions,
>(
    proposal: &Proposal<TYPES, QuorumProposalWrapper<TYPES>>,
    validation_info: &ValidationInfo<TYPES, I, V>,
) -> Result<()> {
    let view_number = proposal.data.view_number();
    ensure!(
        view_number >= validation_info.consensus.read().await.cur_view(),
        "Proposal is from an older view {:?}",
        proposal.data
    );

    // Validate the proposal's signature. This should also catch if the leaf_commitment does not equal our calculated parent commitment
    let mut membership = validation_info.membership.clone();
    proposal.validate_signature(&membership).await?;

    // Verify a timeout certificate OR a view sync certificate exists and is valid.
    if proposal.data.justify_qc().view_number() != view_number - 1 {
        let received_proposal_cert =
            proposal.data.view_change_evidence().clone().context(debug!(
                "Quorum proposal for view {view_number} needed a timeout or view sync certificate, but did not have one",
        ))?;

        match received_proposal_cert {
            ViewChangeEvidence2::Timeout(timeout_cert) => {
                ensure!(
                    timeout_cert.data().view == view_number - 1,
                    "Timeout certificate for view {view_number} was not for the immediately preceding view"
                );
                let timeout_cert_epoch = timeout_cert.data().epoch();
                membership = membership.get_new_epoch(timeout_cert_epoch).await?;

                let membership_stake_table = membership.stake_table().await;
                let membership_success_threshold = membership.success_threshold().await;

                timeout_cert
                    .is_valid_cert(
                        &StakeTableEntries::<TYPES>::from(membership_stake_table).0,
                        membership_success_threshold,
                        &validation_info.upgrade_lock,
                    )
                    .await
                    .context(|e| {
                        warn!("Timeout certificate for view {view_number} was invalid: {e}")
                    })?;
            },
            ViewChangeEvidence2::ViewSync(view_sync_cert) => {
                ensure!(
                    view_sync_cert.view_number == view_number,
                    "View sync cert view number {:?} does not match proposal view number {:?}",
                    view_sync_cert.view_number,
                    view_number
                );

                let view_sync_cert_epoch = view_sync_cert.data().epoch();
                membership = membership.get_new_epoch(view_sync_cert_epoch).await?;

                let membership_stake_table = membership.stake_table().await;
                let membership_success_threshold = membership.success_threshold().await;

                // View sync certs must also be valid.
                view_sync_cert
                    .is_valid_cert(
                        &StakeTableEntries::<TYPES>::from(membership_stake_table).0,
                        membership_success_threshold,
                        &validation_info.upgrade_lock,
                    )
                    .await
                    .context(|e| warn!("Invalid view sync finalize cert provided: {e}"))?;
            },
        }
    }

    // Validate the upgrade certificate -- this is just a signature validation.
    // Note that we don't do anything with the certificate directly if this passes; it eventually gets stored as part of the leaf if nothing goes wrong.
    {
        let epoch = option_epoch_from_block_number::<TYPES>(
            proposal.data.epoch().is_some(),
            proposal.data.block_header().block_number(),
            validation_info.epoch_height,
        );
        UpgradeCertificate::validate(
            proposal.data.upgrade_certificate(),
            &validation_info.membership,
            epoch,
            &validation_info.upgrade_lock,
        )
        .await?;
    }

    Ok(())
}

/// Helper function to send events and log errors
pub async fn broadcast_event<E: Clone + std::fmt::Debug>(event: E, sender: &Sender<E>) {
    match sender.broadcast_direct(event).await {
        Ok(None) => (),
        Ok(Some(overflowed)) => {
            tracing::error!(
                "Event sender queue overflow, Oldest event removed form queue: {overflowed:?}"
            );
        },
        Err(SendError(e)) => {
            tracing::warn!("Event: {e:?}\n Sending failed, event stream probably shutdown");
        },
    }
}

/// Validates qc's signatures and, if provided, validates next_epoch_qc's signatures and whether it
/// corresponds to the provided high_qc.
pub async fn validate_qc_and_next_epoch_qc<TYPES: NodeType, V: Versions>(
    qc: &QuorumCertificate2<TYPES>,
    maybe_next_epoch_qc: Option<&NextEpochQuorumCertificate2<TYPES>>,
    consensus: &OuterConsensus<TYPES>,
    membership_coordinator: &EpochMembershipCoordinator<TYPES>,
    upgrade_lock: &UpgradeLock<TYPES, V>,
    epoch_height: u64,
) -> Result<()> {
    let mut epoch_membership = membership_coordinator
        .stake_table_for_epoch(qc.data.epoch)
        .await?;

    let membership_stake_table = epoch_membership.stake_table().await;
    let membership_success_threshold = epoch_membership.success_threshold().await;

    if let Err(e) = qc
        .is_valid_cert(
            &StakeTableEntries::<TYPES>::from(membership_stake_table).0,
            membership_success_threshold,
            upgrade_lock,
        )
        .await
    {
        consensus.read().await.metrics.invalid_qc.update(1);
        return Err(warn!("Invalid certificate: {e}"));
    }

    if upgrade_lock.epochs_enabled(qc.view_number()).await {
        ensure!(
            qc.data.block_number.is_some(),
            "QC for epoch {:?} has no block number",
            qc.data.epoch
        );
    }

    if qc
        .data
        .block_number
        .is_some_and(|b| is_epoch_transition(b, epoch_height))
    {
        ensure!(
            maybe_next_epoch_qc.is_some(),
            error!("Received High QC for the transition block but not the next epoch QC")
        );
    }

    if let Some(next_epoch_qc) = maybe_next_epoch_qc {
        // If the next epoch qc exists, make sure it's equal to the qc
        if qc.view_number() != next_epoch_qc.view_number() || qc.data != *next_epoch_qc.data {
            bail!("Next epoch qc exists but it's not equal with qc.");
        }
        epoch_membership = epoch_membership.next_epoch_stake_table().await?;
        let membership_next_stake_table = epoch_membership.stake_table().await;
        let membership_next_success_threshold = epoch_membership.success_threshold().await;

        // Validate the next epoch qc as well
        next_epoch_qc
            .is_valid_cert(
                &StakeTableEntries::<TYPES>::from(membership_next_stake_table).0,
                membership_next_success_threshold,
                upgrade_lock,
            )
            .await
            .context(|e| warn!("Invalid next epoch certificate: {e}"))?;
    }
    Ok(())
}

/// Validates the light client state update certificate
pub async fn validate_light_client_state_update_certificate<TYPES: NodeType>(
    state_cert: &LightClientStateUpdateCertificate<TYPES>,
    membership_coordinator: &EpochMembershipCoordinator<TYPES>,
) -> Result<()> {
    tracing::debug!("Validating light client state update certificate");

    let epoch_membership = membership_coordinator
        .membership_for_epoch(state_cert.epoch())
        .await?;

    let membership_stake_table = epoch_membership.stake_table().await;
    let membership_success_threshold = epoch_membership.success_threshold().await;

    let mut state_key_map = HashMap::new();
    membership_stake_table.into_iter().for_each(|config| {
        state_key_map.insert(
            config.state_ver_key.clone(),
            config.stake_table_entry.stake(),
        );
    });

    let mut accumulated_stake = U256::from(0);
    for (key, sig) in state_cert.signatures.iter() {
        if let Some(stake) = state_key_map.get(key) {
            accumulated_stake += *stake;
            if !key.verify_state_sig(
                sig,
                &state_cert.light_client_state,
                &state_cert.next_stake_table_state,
            ) {
                bail!("Invalid light client state update certificate signature");
            }
        } else {
            bail!("Invalid light client state update certificate signature");
        }
    }
    if accumulated_stake < membership_success_threshold {
        bail!("Light client state update certificate does not meet the success threshold");
    }

    Ok(())
}

pub(crate) fn check_qc_state_cert_correspondence<TYPES: NodeType>(
    qc: &QuorumCertificate2<TYPES>,
    state_cert: &LightClientStateUpdateCertificate<TYPES>,
    epoch_height: u64,
) -> bool {
    qc.data
        .block_number
        .is_some_and(|bn| is_epoch_root(bn, epoch_height))
        && Some(state_cert.epoch) == qc.data.epoch()
        && qc.view_number().u64() == state_cert.light_client_state.view_number
}

/// Gets the second VID share, the current or the next epoch accordingly, from the shared consensus state;
/// makes sure it corresponds to the given DA certificate;
/// if it's not yet available, waits for it with the given timeout.
pub async fn wait_for_second_vid_share<TYPES: NodeType>(
    target_epoch: Option<TYPES::Epoch>,
    vid_share: &Proposal<TYPES, VidDisperseShare<TYPES>>,
    da_cert: &DaCertificate2<TYPES>,
    consensus: &OuterConsensus<TYPES>,
    receiver: &Receiver<Arc<HotShotEvent<TYPES>>>,
) -> Result<Proposal<TYPES, VidDisperseShare<TYPES>>> {
    tracing::debug!("getting the second VID share for epoch {:?}", target_epoch);
    let maybe_second_vid_share = consensus
        .read()
        .await
        .vid_shares()
        .get(&vid_share.data.view_number())
        .and_then(|key_map| key_map.get(vid_share.data.recipient_key()))
        .and_then(|epoch_map| epoch_map.get(&target_epoch))
        .cloned();
    if let Some(second_vid_share) = maybe_second_vid_share {
        if (target_epoch == da_cert.epoch()
            && second_vid_share.data.payload_commitment() == da_cert.data().payload_commit)
            || (target_epoch != da_cert.epoch()
                && Some(second_vid_share.data.payload_commitment())
                    == da_cert.data().next_epoch_payload_commit)
        {
            return Ok(second_vid_share);
        }
    }

    let receiver = receiver.clone();
    let da_cert_clone = da_cert.clone();
    let Some(event) = EventDependency::new(
        receiver,
        Box::new(move |event| {
            let event = event.as_ref();
            if let HotShotEvent::VidShareValidated(second_vid_share) = event {
                if target_epoch == da_cert_clone.epoch() {
                    second_vid_share.data.payload_commitment()
                        == da_cert_clone.data().payload_commit
                } else {
                    Some(second_vid_share.data.payload_commitment())
                        == da_cert_clone.data().next_epoch_payload_commit
                }
            } else {
                false
            }
        }),
    )
    .completed()
    .await
    else {
        return Err(warn!("Error while waiting for the second VID share."));
    };
    let HotShotEvent::VidShareValidated(second_vid_share) = event.as_ref() else {
        // this shouldn't happen
        return Err(warn!("Received event is not VidShareValidated but we checked it earlier. Shouldn't be possible."));
    };
    Ok(second_vid_share.clone())
}
