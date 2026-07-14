pub mod error;
pub(crate) mod metrics;
pub mod timer;

use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
    time::{Duration, Instant},
};

use bon::{Builder, bon};
use committable::Commitment;
use hotshot::{HotShotInitializer, traits::BlockPayload, types::SignatureKey};
use hotshot_types::{
    consensus::{ConsensusMetricsValue, ParticipationTracker},
    data::{
        EpochNumber, Leaf2, VidCommitment, VidCommitment2, VidDisperseShare2, ViewNumber,
        vid_disperse::vid_total_weight,
    },
    epoch_membership::EpochMembershipCoordinator,
    message::{Proposal as SignedProposal, UpgradeLock},
    simple_certificate::{QuorumCertificate2, TimeoutCertificate2},
    simple_vote::{HasEpoch, QuorumVote2, TimeoutVote2},
    traits::{
        block_contents::BlockHeader, metrics::Metrics, node_implementation::NodeType,
        signature_key::StateSignatureKey,
    },
    utils::{epoch_from_block_number, is_epoch_root},
    vid::avidm_gf2::{AvidmGf2Param, init_avidm_gf2_param},
    vote::{HasViewNumber, Vote},
};
use time::OffsetDateTime;
use tokio::{select, sync::oneshot};
use tracing::{debug, error, info, warn};

use crate::{
    block::{BlockAndHeaderRequest, BlockBuilder, BlockBuilderConfig},
    client::{ClientApi, ClientRequest, CoordinatorClient, QueryError},
    consensus::{Consensus, ConsensusInput, ConsensusOutput},
    coordinator::{
        error::{CoordinatorError, ErrorSource, Severity},
        timer::Timer,
    },
    epoch::{EpochManager, EpochRootResult},
    helpers::proposal_commitment,
    logging::KeyPrefix,
    message::{
        self, BlockMessage, Certificate1, Certificate2, ConsensusMessage, Message, MessageType,
        Proposal, ProposalFetchMessage, ProposalMessage, TimeoutOneHonest, TransactionMessage,
        Unchecked, Validated, Vote2,
    },
    network::Cliquenet,
    outbox::Outbox,
    proposal::{ProposalValidator, ValidatedProposal, VidShareValidator},
    state::{HeaderRequest, StateEntry, StateManager, StateManagerOutput},
    storage::{NewProtocolStorage, Storage},
    vid::{VidDisperseRequest, VidDisperser, VidFragmentAccumulator, VidReconstructor},
    vote::{EpochRootTally, SimpleTally, VoteCollector},
};

/// Views to retain in the VID reconstructor behind the decided view
///
/// A decide can land while an earlier view's payload is still being
/// reconstructed, and GC at the decided view would abort that task.
/// A decide proves a quorum reconstructed the payload
/// so it can be fetched later assuming the quorum includes at
/// least one query node serving catchup.
/// The margin gives in flight reconstruction tasks time to finish, which is
/// cheaper than fetching the payload through catchup.
///
/// Proposals are retained with the same margin: when a reconstruction
/// finishes, `BlockPayloadReconstructed` is only emitted if the proposal
/// (the block header) for that view is still available.
pub(crate) const VID_RECONSTRUCT_GC_MARGIN: u64 = 5;

/// Views to retain in storage GC behind the decided view, so in-flight
/// storage writes for recent views aren't aborted before they persist.
const STORAGE_GC_MARGIN: u64 = 5;

#[allow(clippy::large_enum_variant)]
pub enum CoordinatorOutput<T: NodeType> {
    Consensus(ConsensusOutput<T>),
    ExternalMessageReceived {
        sender: T::SignatureKey,
        data: Vec<u8>,
    },
}

#[derive(Builder)]
pub struct Coordinator<T: NodeType, S> {
    membership_coordinator: EpochMembershipCoordinator<T>,
    consensus: Consensus<T>,
    network: Cliquenet<T>,
    state_manager: StateManager<T>,
    #[builder(default)]
    client: CoordinatorClient<T>,
    vid_disperser: VidDisperser<T>,
    vid_reconstructor: VidReconstructor<T>,
    #[builder(default)]
    vid_fragment_accumulator: VidFragmentAccumulator<T>,
    vote1_collector: VoteCollector<T, SimpleTally<T, QuorumVote2<T>, QuorumCertificate2<T>>>,
    vote2_collector: VoteCollector<T, SimpleTally<T, Vote2<T>, Certificate2<T>>>,
    timeout_collector: VoteCollector<T, SimpleTally<T, TimeoutVote2<T>, TimeoutCertificate2<T>>>,
    timeout_one_honest_collector:
        VoteCollector<T, SimpleTally<T, TimeoutVote2<T>, TimeoutOneHonest<T>>>,
    epoch_root_collector: VoteCollector<T, EpochRootTally<T>>,
    epoch_manager: EpochManager<T>,
    block_builder: BlockBuilder<T>,
    proposal_validator: ProposalValidator<T>,
    share_validator: VidShareValidator<T>,
    storage: Storage<T, S>,
    #[builder(default)]
    outbox: Outbox<ConsensusOutput<T>>,
    #[builder(default)]
    coordinator_outbox: Outbox<CoordinatorOutput<T>>,
    public_key: T::SignatureKey,
    #[builder(default = KeyPrefix::from(&public_key))]
    node_id: KeyPrefix,
    timer: Timer,
    #[builder(skip)]
    pending_proposal_fetches: PendingProposalFetches<T>,
    #[builder(default)]
    cached_validated_proposals: BTreeMap<(ViewNumber, VidCommitment2), ValidatedProposal<T>>,
    #[builder(default)]
    cached_vid_shares: BTreeMap<(ViewNumber, VidCommitment2), VidDisperseShare2<T>>,
    #[builder(skip)]
    da_payloads: BTreeMap<(ViewNumber, VidCommitment2), PendingDa<T>>,
    metrics: Option<metrics::Metrics>,
    #[builder(default)]
    participation: ParticipationTracker<T>,
    #[builder(skip)]
    voted_view: Option<ViewNumber>,
    #[builder(skip)]
    view_started: Option<(ViewNumber, EpochNumber, Instant)>,
    #[builder(skip)]
    proposal_received_at: Option<(ViewNumber, Instant)>,
    #[builder(skip)]
    invalid_certs_at_decide: u64,
    #[builder(skip)]
    payload_txn_bytes: BTreeMap<ViewNumber, usize>,
}

#[bon]
impl<T, S> Coordinator<T, S>
where
    T: NodeType,
    S: NewProtocolStorage<T>,
{
    #[builder(builder_type = CoordinatorMaker, finish_fn = make)]
    #[allow(clippy::too_many_arguments)]
    pub fn maker(
        membership_coordinator: EpochMembershipCoordinator<T>,
        network: Cliquenet<T>,
        initializer: &HotShotInitializer<T>,
        upgrade_lock: UpgradeLock<T>,
        public_key: T::SignatureKey,
        private_key: <T::SignatureKey as SignatureKey>::PrivateKey,
        state_private_key: <T::StateSignatureKey as StateSignatureKey>::StatePrivateKey,
        stake_table_capacity: usize,
        timeout_duration: Duration,
        storage: S,
        metrics: &dyn Metrics,
        consensus_metrics: ConsensusMetricsValue,
        /// Locked QC persisted on a prior run; restored so the lock survives restart.
        locked_qc: Option<Certificate1<T>>,
    ) -> Self {
        let mut consensus = Consensus::new(
            membership_coordinator.clone(),
            public_key.clone(),
            private_key.clone(),
            state_private_key,
            stake_table_capacity,
            upgrade_lock.clone(),
            initializer.anchor_leaf.clone(),
            initializer.epoch_height,
        );

        let anchor_leaf = &initializer.anchor_leaf;
        let anchor_view = anchor_leaf.view_number();
        let anchor_epoch = anchor_leaf
            .epoch(initializer.epoch_height)
            .unwrap_or(EpochNumber::genesis());
        let cert1 = initializer.high_qc.clone();
        let parent_proposal = message::Proposal {
            block_header: anchor_leaf.block_header().clone(),
            view_number: anchor_view,
            epoch: anchor_epoch,
            justify_qc: anchor_leaf.justify_qc(),
            next_epoch_justify_qc: None,
            upgrade_certificate: anchor_leaf.upgrade_certificate(),
            view_change_evidence: anchor_leaf
                .view_change_evidence
                .clone()
                .and_then(|e| match e {
                    hotshot_types::data::ViewChangeEvidence2::Timeout(tc) => Some(tc),
                    hotshot_types::data::ViewChangeEvidence2::ViewSync(_) => None,
                }),
            next_drb_result: anchor_leaf.next_drb_result,
            state_cert: None,
        };

        let coordinator_metrics = metrics
            .is_recording()
            .then(|| metrics::Metrics::new(consensus_metrics));

        let mut state_manager = StateManager::new(
            Arc::new(initializer.instance_state.clone()),
            upgrade_lock.clone(),
        )
        .with_metrics(
            coordinator_metrics.as_ref().map(|m| {
                m.consensus
                    .validate_and_apply_header_duration
                    .clone()
                    .into()
            }),
            coordinator_metrics
                .as_ref()
                .map(|m| m.consensus.update_leaf_duration.clone().into()),
        );
        // Seed `from_header` stubs for restored undecided proposals so a child
        // proposal can be validated; anchor seeded last so its state wins.
        for p in initializer.saved_proposals.values() {
            state_manager.seed_from_header(message::Proposal::from(p.data.clone()));
        }
        state_manager.seed_state(
            anchor_view,
            initializer.anchor_state.clone(),
            anchor_leaf.clone(),
        );
        // The anchor leaf and persisted proposals are blocks this node had
        // reconstructed before it went down, so treat them as reconstructed on
        // restart
        let reconstructed_blocks =
            std::iter::once((anchor_view, anchor_leaf.block_header().clone()))
                .chain(
                    initializer
                        .saved_proposals
                        .iter()
                        .map(|(view, p)| (*view, p.data.block_header().clone())),
                )
                .filter_map(|(view, header)| match header.payload_commitment() {
                    VidCommitment::V2(commitment) => Some((view, commitment)),
                    _ => None,
                });
        // Seed every persisted proposal before `seed_parent` so its authoritative anchor wins.
        let saved_proposals = initializer
            .saved_proposals
            .values()
            .map(|p| message::Proposal::from(p.data.clone()));
        consensus.seed_proposals(saved_proposals);
        // `seed_parent` sets the current epoch from the anchor proposal;
        // `resume_from_restart` positions the view so the node never
        // re-enters a view it may have voted or proposed in before it went
        // down.
        consensus.seed_parent(cert1, parent_proposal, reconstructed_blocks);
        // Restore the persisted lock; it can be newer than the anchor QC, so
        // this must run after `seed_parent`.
        if let Some(locked_qc) = locked_qc {
            consensus.seed_locked_cert(locked_qc);
        }
        consensus.resume_from_restart(
            anchor_view,
            initializer.start_view,
            initializer.last_actioned_view,
        );
        if let Some(state_cert) = initializer.state_cert.clone() {
            consensus.seed_state_cert(state_cert);
        }

        let participation = ParticipationTracker::new(&membership_coordinator, anchor_epoch);

        let vid_disperser = VidDisperser::new(
            membership_coordinator.clone(),
            network.sender().clone(),
            public_key.clone(),
            private_key.clone(),
        )
        .with_metrics(
            coordinator_metrics
                .as_ref()
                .map(|m| m.consensus.vid_disperse_duration.clone().into()),
        );

        let lock = upgrade_lock.clone();
        Self::builder()
            .consensus(consensus)
            .network(network)
            .state_manager(state_manager)
            .vid_disperser(vid_disperser)
            .vid_reconstructor(VidReconstructor::new())
            .vote1_collector(VoteCollector::new(
                membership_coordinator.clone(),
                lock.clone(),
            ))
            .vote2_collector(VoteCollector::new(
                membership_coordinator.clone(),
                lock.clone(),
            ))
            .timeout_collector(VoteCollector::new(
                membership_coordinator.clone(),
                lock.clone(),
            ))
            .timeout_one_honest_collector(VoteCollector::new(
                membership_coordinator.clone(),
                lock.clone(),
            ))
            .epoch_root_collector(VoteCollector::new(membership_coordinator.clone(), lock))
            .epoch_manager(EpochManager::new(
                initializer.epoch_height,
                membership_coordinator.clone(),
            ))
            .block_builder(BlockBuilder::new(
                Arc::new(initializer.instance_state.clone()),
                membership_coordinator.clone(),
                BlockBuilderConfig::default(),
                upgrade_lock.clone(),
            ))
            .proposal_validator(ProposalValidator::new(
                membership_coordinator.clone(),
                initializer.epoch_height,
                upgrade_lock.clone(),
            ))
            .share_validator(VidShareValidator::new(
                membership_coordinator.clone(),
                initializer.epoch_height,
                upgrade_lock,
            ))
            .storage(Storage::new(storage, private_key).with_metrics(metrics))
            .membership_coordinator(membership_coordinator)
            .timer(Timer::new(timeout_duration, anchor_view, anchor_epoch))
            .public_key(public_key)
            .maybe_metrics(coordinator_metrics)
            .participation(participation)
            .build()
    }

    /// Emit `ViewChanged(current_view + 1)` and, if leader, a
    /// `RequestBlockAndHeader`.
    pub fn start(&mut self) {
        let cur_view = self.consensus.current_view();
        let next_view = cur_view + 1;
        let epoch = self
            .consensus
            .current_epoch()
            .unwrap_or(EpochNumber::genesis());

        if self.consensus.last_decided_leaf().view_number() == ViewNumber::genesis() {
            // Genesis DA never flows through the normal block-builder path.
            let genesis_leaf = self.consensus.last_decided_leaf().clone();
            let (payload, metadata) = T::BlockPayload::empty();
            self.storage.append_da(
                ViewNumber::genesis(),
                EpochNumber::genesis(),
                payload,
                metadata,
                genesis_leaf.payload_commitment(),
            );

            // Emit `LeafDecided` for genesis so persistence sees the header.
            self.outbox.push_back(ConsensusOutput::LeafDecided {
                leaves: vec![genesis_leaf],
                cert1: self
                    .consensus
                    .cert1_at(ViewNumber::genesis())
                    .cloned()
                    .expect("genesis cert1 must be seeded"),
                cert2: None,
                vid_shares: vec![None],
            });
        }

        self.outbox
            .push_back(ConsensusOutput::ViewChanged(next_view, epoch));

        if let Some(leader) = self.leader(next_view, epoch)
            && leader == self.public_key
        {
            // No parent proposal when restarting past the anchor view: the
            // node cannot propose off the anchor for a later view; the
            // timeout path takes over instead.
            if let Some(parent_proposal) = self.consensus.proposal_at(cur_view).cloned() {
                self.outbox
                    .push_back(ConsensusOutput::RequestBlockAndHeader(
                        BlockAndHeaderRequest {
                            view: next_view,
                            epoch,
                            parent_proposal,
                        },
                    ));
            }
        }
    }

    pub async fn stop(mut self) {
        futures::join!(self.network.shutdown(), self.storage.flush());
    }

    pub async fn next_consensus_input(&mut self) -> Result<ConsensusInput<T>, CoordinatorError> {
        loop {
            select! {
                message = self.network.receive() => match message {
                    Ok(m) => {
                        if let Some(input) = self.on_network_message(m) {
                            return Ok(input)
                        }
                    }
                    Err(e) => {
                        return Err(CoordinatorError::from(e).context("network receive"))
                    }
                },
                () = &mut self.timer => {
                    let view = self.timer.view();
                    let epoch = self.timer.epoch();
                    if let Some(stats) = self.vote1_collector.stats(view, epoch) {
                        warn!(
                            %view, %epoch,
                            stake = %stats.stake,
                            threshold = %stats.threshold,
                            "timeout: vote1 stake observed (deduped by signer)"
                        );
                    } else {
                        warn!(%view, %epoch, "timeout: no vote1 received for this view");
                    }
                    let input = ConsensusInput::Timeout(view, epoch);
                    let leader = self.leader(view, epoch);
                    if let Some(leader) = leader.clone() {
                        self.participation.leader_missed(leader, epoch);
                    }
                    if let Some(m) = &self.metrics {
                        m.consensus.number_of_timeouts.add(1);
                        if leader.as_ref() == Some(&self.public_key) {
                            m.consensus.number_of_timeouts_as_leader.add(1);
                        }
                    }
                    return Ok(input)
                }
                Some(output) = self.state_manager.next() => {
                    if let Some(input) = self.on_state_manager_output(output) {
                        return Ok(input)
                    }
                }
                Some(request) = self.client.next_request() => {
                    if let Err(err) = self.on_client_request(request) {
                        error!(%err, "error while handling client request");
                    }
                }
                Some(tcert) = self.timeout_collector.next() => {
                    return Ok(ConsensusInput::TimeoutCertificate(tcert))
                }
                Some(out) = self.timeout_one_honest_collector.next() => {
                    let Some(epoch) = out.data.epoch else {
                        let msg = format!("missing epoch in view {}", out.view_number());
                        return Err(CoordinatorError::regular(msg).context("gc timeout one honest"))
                    };
                    return Ok(ConsensusInput::TimeoutOneHonest(out.view_number(), epoch))
                }
                Some(cert1) = self.vote1_collector.next() => {
                    return Ok(ConsensusInput::Certificate1(cert1))
                }
                Some(cert2) = self.vote2_collector.next() => {
                    return Ok(ConsensusInput::Certificate2(cert2))
                }
                Some((cert1, state_cert)) = self.epoch_root_collector.next() => {
                    self.storage.append_state_cert(
                        ViewNumber::new(state_cert.light_client_state.view_number),
                        state_cert.clone(),
                    );
                    return Ok(ConsensusInput::EpochRootCertificates { cert1, state_cert })
                }
                Some(item) = self.share_validator.next() => match item {
                    Ok(vid_share) => {
                        let view = vid_share.view_number();
                        let key = (view, vid_share.payload_commitment);
                        let Some(validated) = self.cached_validated_proposals.remove(&key) else {
                            // Wait for the proposal
                            self.cached_vid_shares.insert(key, vid_share);
                            continue;
                        };
                        return self.on_proposal_and_vid_share(validated, vid_share)
                    },
                    Err(e) => {
                        return Err(CoordinatorError::regular(e).context("vid share validation"))
                    }
                },
                Some(item) = self.proposal_validator.next() => match item {
<<<<<<< HEAD
||||||| parent of 3013c5ed3f7 ([Fast Finality] Metric Parity b/t Protocols (#4674))
                    Ok(validated) if validated.fetched => {
                        finish_measurement(next_input);
                        return Ok(ConsensusInput::FetchedProposal(validated.message))
                    }
=======
                    Ok(validated) if validated.fetched => {
                        return Ok(ConsensusInput::FetchedProposal(validated.message))
                    }
>>>>>>> 3013c5ed3f7 ([Fast Finality] Metric Parity b/t Protocols (#4674))
                    Ok(validated) => {
                        // Refresh the network's peer set when a proposal is validated.
                        let epoch = validated.message.proposal.data.epoch;
                        if let Err(err) = self
                            .network
                            .apply_epoch(epoch, &self.membership_coordinator)
                        {
                            error!(%epoch, %err, "network apply_epoch failed");
                        }

                        let view = validated.message.proposal.data.view_number();
                        let VidCommitment::V2(commit) =
                            validated.message.proposal.data.block_header.payload_commitment()
                        else {
                            warn!(%view, "proposal payload commitment is not V2, discarding");
                            continue;
                        };
                        let key = (view, commit);
                        let Some(vid_share) = self.cached_vid_shares.remove(&key) else {
                            // Wait for the vid share describing this payload.
                            self.cached_validated_proposals.insert(key, validated);
                            continue;
                        };
                        return self.on_proposal_and_vid_share(validated, vid_share)
                    }
                    Err(e) => {
                        return Err(CoordinatorError::regular(e).context("proposal validation"))
                    }
                },
                Some(item) = self.block_builder.next() => match item {
                    Ok(block) => {
                        self.state_manager.request_header(HeaderRequest::from(&block));
                        let next_view = block.view + 1;
                        let epoch = block.epoch;
                        let manifest = block.manifest.clone();
                        // Retain the payload and persist it when consensus proposes this
                        // exact block (cf. SendProposal):
                        if let VidCommitment::V2(commit) = block.payload_commitment {
                            self.da_payloads.insert(
                                (block.view, commit),
                                PendingDa {
                                    epoch: block.epoch,
                                    payload: block.payload.payload.clone(),
                                    metadata: block.payload.metadata.clone(),
                                },
                            );
                        } else {
                            warn!(view = %block.view, "block payload commitment is not V2");
                        }
                        // We built this block; skip reconstructing it from our own loopback share.
                        self.vid_reconstructor.retire_view(block.view);
                        self.unicast_to_leader(
                            next_view,
                            epoch,
                            BlockMessage::DedupManifest(manifest),
                        )?;
                        return Ok(block.into())
                    }
                    Err(err) => {
                        return Err(CoordinatorError::regular(err).context("block building"))
                    }
                },
                Some(item) = self.vid_disperser.next() => match item {
                    Ok(out) => {
                        return Ok(ConsensusInput::VidDisperseCreated(out.view, out.payload_commitment))
                    }
                    Err(err) => {
                        return Err(CoordinatorError::from(err).context("vid disperse"))
                    }
                },
                Some(item) = self.vid_reconstructor.next() => match item {
                    Ok(out) => {
                        self.payload_txn_bytes.insert(out.view, out.payload.txn_bytes());
                        self.block_builder.on_block_reconstructed(out.tx_commitments);
                        self.storage.append_da(
                            out.view,
                            out.epoch,
                            out.payload.clone(),
                            out.metadata.clone(),
                            VidCommitment::V2(out.payload_commitment),
                        );
                        if let Some(proposal) = self.consensus.proposal_at(out.view) {
                            // Only pair the payload with the header if the proposal commits to it
                            if proposal.block_header.payload_commitment()
                                == VidCommitment::V2(out.payload_commitment)
                            {
                                self.outbox.push_back(ConsensusOutput::BlockPayloadReconstructed {
                                    view: out.view,
                                    header: proposal.block_header.clone(),
                                    payload: out.payload,
                                });
                            } else {
                                warn!(
                                    view = %out.view,
                                    header = %proposal.block_header.payload_commitment(),
                                    reconstructed = %out.payload_commitment,
                                    "reconstructed payload commitment does not match proposal header"
                                );
                            }
                        }
                        return Ok(ConsensusInput::BlockReconstructed(out.view, out.payload_commitment))
                    }
                    Err(err) => {
                        return Err(CoordinatorError::regular(err).context("vid reconstruction"))
                    }
                },
                Some(stored) = self.storage.next() => {
                    return Ok(ConsensusInput::Stored(stored))
                },
                Some(result) = self.epoch_manager.next() => match result {
                    Ok(EpochRootResult::DrbResult(epoch, drb_result)) => {
                        // New epoch data available — retry votes that were
                        // buffered because their membership wasn't ready.
                        self.vote1_collector.retry_pending_votes();
                        self.vote2_collector.retry_pending_votes();
                        self.timeout_collector.retry_pending_votes();
                        self.timeout_one_honest_collector.retry_pending_votes();
                        self.epoch_root_collector.retry_pending_votes();
                        return Ok(ConsensusInput::DrbResult(epoch, drb_result))
                    }
                    Err(failure) => {
                        // Catchup/compute failed. The epoch manager clears
                        // the pending guard; consensus's `maybe_propose`
                        // will re-request the DRB when it next tries to
                        // build a transition proposal and finds it missing.
                        warn!(%failure.error, epoch = %failure.epoch, "DRB request failed");
                        continue;
                    }
                },
                else => {
                    return Err(CoordinatorError::critical(ErrorSource::NoInput))
                }
            }
        }
    }

    pub fn apply_consensus(&mut self, input: ConsensusInput<T>) {
        self.consensus.apply(input, &mut self.outbox)
    }

    pub fn process_consensus_output(
        &mut self,
        output: ConsensusOutput<T>,
    ) -> Result<(), CoordinatorError> {
        let node = self.node_id;
        match output {
            ConsensusOutput::RequestState(state_request) => {
                debug!(
                    %node,
                    view = %state_request.view,
                    epoch = %state_request.epoch,
                    block = %state_request.block,
                    "request state validation"
                );
                self.state_manager.request_state(state_request);
            },
            ConsensusOutput::RequestVidDisperse {
                view,
                epoch,
                payload,
                metadata,
                payload_commitment,
            } => {
                debug!(%node, %view, %epoch, "request vid disperse");
                self.vid_disperser.request_vid_disperse(VidDisperseRequest {
                    view,
                    epoch,
                    block: payload,
                    metadata,
                    payload_commitment,
                });
            },
            ConsensusOutput::RequestDrbResult(epoch) => {
                debug!(%node, %epoch, "request drb result");
                self.epoch_manager.request_drb_result(epoch);
            },
            ConsensusOutput::LeafDecided {
                leaves,
                cert1,
                cert2,
                ..
            } => {
                info!(
                    %node,
                    view = %cert1.view_number(),
                    epoch = ?cert1.epoch().map(|e| *e),
                    leaves = leaves.len(),
                    "leaves decided"
                );
                self.on_decide_metrics(&leaves);
                if let Some(cert2) = cert2 {
                    self.storage.append_cert2(cert2.view_number, cert2.clone());
                }
                // `leaves` is ordered newest first.
                //  Garbage collect the data for views < decided view
                if let Some(newest) = leaves.first() {
                    let gc_view = newest.view_number();
                    let gc_epoch = newest.justify_qc().epoch().unwrap_or_default();
                    self.gc(gc_epoch, GcScope::Decided(gc_view))?;
                }
                for leaf in leaves.into_iter().rev() {
                    self.participation
                        .on_leaf_decided(&leaf, &self.membership_coordinator);
                    self.epoch_manager.handle_leaf_decided(leaf);
                }
            },
            ConsensusOutput::LockUpdated(cert) => {
                debug!(
                    %node,
                    view = %cert.view_number(),
                    epoch = ?cert.epoch().map(|e| *e),
                    "lock updated"
                );
            },
            ConsensusOutput::RequestBlockAndHeader(request) => {
                debug!(
                    %node,
                    view = %request.view,
                    epoch = %request.epoch,
                    "request block and header"
                );
                self.block_builder.request_block(request);
            },
            ConsensusOutput::RecordAction(view, epoch, kind) => {
                debug!(%node, %view, ?kind, "record action");
                self.storage.record_action(view, epoch, kind);
            },
            ConsensusOutput::PersistProposal(proposal) => {
                let view = proposal.data.view_number;
                debug!(%node, %view, "persist proposal");
                self.storage.append_proposal(proposal.data.clone());
                // Two blocks can be built for one view. Here we know which one
                // wins and we persist just that one:
                if let VidCommitment::V2(commit) = proposal.data.block_header.payload_commitment() {
                    if let Some(da) = self.da_payloads.remove(&(view, commit)) {
                        self.payload_txn_bytes.insert(view, da.payload.txn_bytes());
                        if let Some(m) = &self.metrics
                            && da.payload.transactions(&da.metadata).next().is_none()
                        {
                            m.consensus.number_of_empty_blocks_proposed.add(1);
                        }
                        self.storage.append_da(
                            view,
                            da.epoch,
                            da.payload,
                            da.metadata,
                            VidCommitment::V2(commit),
                        );
                    } else {
                        warn!(%node, %view, "no payload for proposed block");
                    }
                }
            },
            ConsensusOutput::SendProposal(proposal) => {
                let view = proposal.data.view_number;
                let epoch = proposal.data.epoch;
                let block = proposal.data.block_header.block_number();
                info!(%node, %view, %epoch, %block, "send proposal");
                if let Some(m) = &self.metrics
                    && proposal.data.view_change_evidence.is_none()
                    && let Some((prev_view, received_at)) = self.proposal_received_at
                    && (*prev_view).checked_add(1) == Some(*view)
                {
                    m.consensus
                        .previous_proposal_to_proposal_time
                        .add_point(received_at.elapsed().as_millis() as f64);
                }
                let message = Message {
                    sender: self.public_key.clone(),
                    message_type: MessageType::Consensus(ConsensusMessage::Proposal(
                        ProposalMessage::validated(proposal.clone()),
                    )),
                };
                if let Err(err) = self
                    .network
                    .sender()
                    .broadcast(self.consensus.current_view(), &message)
                {
                    let err = CoordinatorError::from(err).context("proposal broadcast");
                    if err.severity == Severity::Critical {
                        return Err(err);
                    } else {
                        warn!(%node, %err, "network error while broadcasting proposal")
                    }
                }
            },
            ConsensusOutput::SendTimeoutVote(vote, lock) => {
                let view = vote.view_number();
                debug!(%node, %view, has_lock = lock.is_some(), "send timeout vote");
                self.broadcast(
                    ConsensusMessage::TimeoutVote(message::TimeoutVoteMessage { vote, lock }),
                    "broadcast timeout vote",
                )?
            },
            ConsensusOutput::SendTimeoutCertificate(tc, view, epoch) => {
                debug!(
                    %node, %view, %epoch,
                    cert_view = %tc.view_number(),
                    "send timeout certificate"
                );
                if let Some(leader) = self.leader(view, epoch) {
                    let message = Message {
                        sender: self.public_key.clone(),
                        message_type: MessageType::Consensus(ConsensusMessage::TimeoutCertificate(
                            tc,
                        )),
                    };
                    self.network
                        .sender()
                        .unicast(self.consensus.current_view(), &leader, &message)
                        .map_err(|e| CoordinatorError::from(e).context("timeout certificate"))?;
                }
            },
            ConsensusOutput::SendVote1(vote1) => {
                let view = vote1.vote.view_number();
                debug!(
                    %node, %view,
                    epoch_root = vote1.state_vote.is_some(),
                    "send vote1"
                );
                self.record_voted_view(view);
                if let Some(epoch) = vote1.vote.data.epoch
                    && let Some(leader) = self.leader(view, epoch)
                {
                    self.participation.leader_proposed(leader, epoch);
                }
                self.broadcast(ConsensusMessage::Vote1(vote1), "broadcast vote1")?
            },
            ConsensusOutput::BroadcastVidShare(share) => {
                debug!(%node, view = %share.view_number(), "send vid share");
                self.broadcast(
                    ConsensusMessage::VidShareBroadcast(share),
                    "broadcast vid share",
                )?
            },
            ConsensusOutput::SendVote2(vote2) => {
                let view = vote2.view_number();
                debug!(%node, %view, "send vote2");
                self.record_voted_view(view);
                self.broadcast(ConsensusMessage::Vote2(vote2), "broadcast vote2")?
            },
            ConsensusOutput::PersistHighQc(high_qc) => {
                debug!(%node, view = %high_qc.view_number(), "persist high qc");
                self.storage.append_high_qc2(high_qc);
            },
            ConsensusOutput::SendEpochChange(epoch_change) => {
                info!(
                    %node,
                    view = %epoch_change.cert1.view_number(),
                    epoch = ?epoch_change.cert1.epoch().map(|e| *e),
                    "send epoch change"
                );
                self.broadcast(
                    ConsensusMessage::EpochChange(epoch_change),
                    "broadcast epoch change",
                )?
            },
            ConsensusOutput::SendCertificate1(cert1) => {
                debug!(
                    %node,
                    view = %cert1.view_number(),
                    epoch = ?cert1.epoch().map(|e| *e),
                    "send certificate1"
                );
                self.broadcast(
                    ConsensusMessage::Certificate1(cert1, self.public_key.clone()),
                    "broadcast certificate1",
                )?
            },
            ConsensusOutput::SendCertificate2(cert2) => {
                debug!(
                    %node,
                    view = %cert2.view_number(),
                    epoch = ?cert2.epoch().map(|e| *e),
                    "send certificate2"
                );
                self.broadcast(
                    ConsensusMessage::Certificate2(cert2, self.public_key.clone()),
                    "broadcast certificate2",
                )?
            },
            ConsensusOutput::ProposalValidated { proposal, sender } => {
                debug!(
                    %node,
                    view = %proposal.data.view_number,
                    sender = %KeyPrefix::from(&sender),
                    "proposal validated"
                );
            },
            ConsensusOutput::ViewChanged(view, epoch) => {
                let current_view = self.consensus.current_view();
                if view < current_view {
                    warn!(
                        %node, %view, %epoch, %current_view,
                        "ignoring view change to stale view"
                    );
                    return Ok(());
                }
                info!(%node, %view, %epoch, "view changed");
                self.timer.reset_with_epoch(view, epoch);
                self.gc(epoch, GcScope::Local(view))?;
                let txns = self.block_builder.on_view_changed(view, epoch);
                self.participation.on_view_changed(epoch);
                self.on_view_changed_metrics(view, epoch);
                if !txns.is_empty() {
                    let next_view = view + 1;
                    self.unicast_to_leader(
                        next_view,
                        epoch,
                        BlockMessage::Transactions(TransactionMessage {
                            view: next_view,
                            transactions: txns,
                        }),
                    )
                    .map_err(|e| e.context("unicast transactions"))?;
                }

                // Proactively fetch the DRB for the next epoch so
                // late-starting nodes have it before they need it
                let next_epoch = epoch + 1;
                if next_epoch > EpochNumber::genesis() + 1 {
                    self.epoch_manager.request_drb_result(next_epoch);
                }
            },
            ConsensusOutput::ViewTimedOut(view) => {
                debug!(%node, %view, "view timed out");
                let epoch = self
                    .consensus
                    .current_epoch()
                    .unwrap_or_else(EpochNumber::genesis);
                self.gc(epoch, GcScope::Timeout(view))?;
            },
            ConsensusOutput::BlockPayloadReconstructed { .. } => {},
        }
        Ok(())
    }

    pub fn node_id(&self) -> &KeyPrefix {
        &self.node_id
    }

    pub fn outbox(&self) -> &Outbox<ConsensusOutput<T>> {
        &self.outbox
    }

    pub fn outbox_mut(&mut self) -> &mut Outbox<ConsensusOutput<T>> {
        &mut self.outbox
    }

    pub fn coordinator_outbox(&self) -> &Outbox<CoordinatorOutput<T>> {
        &self.coordinator_outbox
    }

    pub fn coordinator_outbox_mut(&mut self) -> &mut Outbox<CoordinatorOutput<T>> {
        &mut self.coordinator_outbox
    }

    pub fn current_view(&self) -> ViewNumber {
        self.consensus.current_view()
    }

    pub fn state(&self, v: ViewNumber) -> Option<&StateEntry<T>> {
        self.state_manager.get_state(v)
    }

    pub fn client_api(&self) -> &ClientApi<T> {
        self.client.handle()
    }

    pub(crate) fn on_network_message(
        &mut self,
        message: Message<T, Unchecked>,
    ) -> Option<ConsensusInput<T>> {
        let sender = KeyPrefix::from(&message.sender);
        let node = self.node_id;
        match message.message_type {
            MessageType::Consensus(msg) => match msg {
                ConsensusMessage::Proposal(p) => {
                    let view = p.view_number();
                    let epoch = p.proposal.data.epoch;
                    let block = p.proposal.data.block_header.block_number();
                    debug!(%node, %sender, %view, %epoch, %block, "recv proposal");
                    if !self.is_too_far_ahead(view)
                        && self.proposal_received_at.is_none_or(|(v, _)| v < view)
                    {
                        self.proposal_received_at = Some((view, Instant::now()));
                    }
                    if self.consensus.wants_proposal_for_view(&view) {
                        self.proposal_validator.validate(p);
                    }
                    None
                },
                ConsensusMessage::VidShareFragment(fragment) => {
                    let view = fragment.data.view_number();
                    debug!(%node, %sender, %view, "received vid share fragment");
                    if fragment.data.recipient_key != self.public_key {
                        warn!(
                            %node,
                            %sender,
                            %view,
                            "ignoring vid share fragment not addressed to this node"
                        );
                        return None;
                    }
                    let leader = fragment
                        .data
                        .epoch
                        .and_then(|epoch| self.leader(view, epoch));
                    if leader.as_ref() != Some(&message.sender) {
                        warn!(
                            %node,
                            %sender,
                            %view,
                            "ignoring vid share fragment not from the view leader"
                        );
                        return None;
                    }
                    if self.consensus.wants_proposal_for_view(&view) {
                        let signature = fragment.signature.clone();
                        match self.vid_fragment_accumulator.accept(fragment.data) {
                            Ok(Some(share)) => self
                                .share_validator
                                .validate(SignedProposal::new(share, signature)),
                            Ok(None) => {}, // Still missing some fragments.
                            Err(err) => {
                                warn!(
                                    %node,
                                    %sender,
                                    %view,
                                    %err, "rejecting malformed vid share fragment"
                                );
                            },
                        }
                    }
                    None
                },
                ConsensusMessage::Vote1(vote1) => {
                    let view = vote1.vote.view_number();
                    if self.is_too_far_ahead(view) {
                        warn!(%node, %sender, %view, "vote1 is too far ahead");
                        return None;
                    }
                    if vote1.vote.signing_key() != message.sender {
                        warn!(%node, %sender, %view, "vote1 signing key != sender");
                        return None;
                    }
                    let bn = vote1.vote.data.block_number.unwrap_or(0);
                    let epoch_height = *self.consensus.epoch_height;
                    let is_epoch_root_vote = is_epoch_root(bn, epoch_height);
                    debug!(
                        %node, %sender, %view,
                        epoch_root = is_epoch_root_vote,
                        has_state_vote = vote1.state_vote.is_some(),
                        "recv vote1"
                    );
                    if is_epoch_root_vote {
                        // An epoch-root Vote1 MUST carry a state_vote.
                        // Reject otherwise.
                        vote1.state_vote.as_ref()?;
                        self.epoch_root_collector.accumulate_vote(vote1);
                    } else {
                        self.vote1_collector.accumulate_vote(vote1.vote);
                    }
                    None
                },
                ConsensusMessage::VidShareBroadcast(share) => {
                    let view = share.view_number();
                    if self.is_too_far_ahead(view) {
                        warn!(%node, %sender, %view, "vid share broadcast is too far ahead");
                        return None;
                    }
                    debug!(%node, %sender, %view, "recv vid share broadcast");
                    // The share belongs to the sender (`recipient_key == sender`,
                    // enforced by `handle_vid_share`); it is verified lazily
                    // against the pinned commitment at reconstruction time.
                    self.vid_reconstructor
                        .handle_vid_share(message.sender.clone(), share);
                    None
                },
                ConsensusMessage::Vote2(vote2) => {
                    let view = vote2.view_number();
                    if self.is_too_far_ahead(view) {
                        warn!(%node, %sender, %view, "vote2 is too far ahead");
                        return None;
                    }
                    if vote2.signing_key() != message.sender {
                        warn!(%node, %sender, %view, "vote2 signing key != sender");
                        return None;
                    }
                    debug!(%node, %sender, %view, "recv vote2");
                    self.vote2_collector.accumulate_vote(vote2);
                    None
                },
                ConsensusMessage::Certificate1(certificate1, _key) => {
                    debug!(
                        %node, %sender,
                        view = %certificate1.view_number(),
                        epoch = ?certificate1.epoch().map(|e| *e),
                        "recv certificate1"
                    );
                    Some(ConsensusInput::Certificate1(certificate1))
                },
                ConsensusMessage::Certificate2(certificate2, _key) => {
                    debug!(
                        %node, %sender,
                        view = %certificate2.view_number(),
                        epoch = ?certificate2.epoch().map(|e| *e),
                        "recv certificate2"
                    );
                    Some(ConsensusInput::Certificate2(certificate2))
                },
                ConsensusMessage::TimeoutVote(timeout_msg) => {
                    let view = timeout_msg.vote.view_number();
                    if self.is_too_far_ahead(view) {
                        warn!(%node, %sender, %view, "timeout vote is too far ahead");
                        return None;
                    }
                    if timeout_msg.vote.signing_key() != message.sender {
                        warn!(%node, %sender, %view, "timeout vote signing key != sender");
                        return None;
                    }
                    let current_view = self.consensus.current_view();
                    if view < current_view {
                        debug!(
                            %node, %sender, %view, %current_view,
                            "ignoring timeout vote for stale view"
                        );
                        return None;
                    }
                    debug!(
                        %node, %sender, %view,
                        has_lock = timeout_msg.lock.is_some(),
                        "recv timeout vote"
                    );
                    self.timeout_collector
                        .accumulate_vote(timeout_msg.vote.clone());
                    self.timeout_one_honest_collector
                        .accumulate_vote(timeout_msg.vote);

                    // If a peer times out in a view at or ahead of us we adopt the
                    // view of an embedded cert1, so divergent nodes re-converge on
                    // the highest justified view on restart.
                    //
                    // TODO: Above we reject timeout votes too far ahead. A valid cert1
                    // has precedence over that check so we need to move this up, but
                    // first we need to verify the cert1 itself.
                    if let Some(cert) = timeout_msg.lock
                        && cert.view_number() >= current_view
                    {
                        return Some(ConsensusInput::AdvanceView(cert));
                    }

                    None
                },
                ConsensusMessage::TimeoutCertificate(tc) => {
                    debug!(
                        %node, %sender,
                        view = %tc.view_number(),
                        epoch = ?tc.epoch().map(|e| *e),
                        "recv timeout certificate"
                    );
                    Some(ConsensusInput::TimeoutCertificate(tc))
                },
                ConsensusMessage::EpochChange(epoch_change) => {
                    debug!(
                        %node, %sender,
                        view = %epoch_change.cert1.view_number(),
                        epoch = ?epoch_change.cert1.epoch().map(|e| *e),
                        "recv epoch change"
                    );
                    Some(ConsensusInput::EpochChange(epoch_change))
                },
            },
            MessageType::Block(msg) => {
                match msg {
                    BlockMessage::Transactions(msg) => {
                        debug!(
                            %node, %sender,
                            view = %msg.view,
                            count = msg.transactions.len(),
                            "recv transactions"
                        );
                        self.block_builder.on_transactions(msg)
                    },
                    BlockMessage::DedupManifest(manifest) => {
                        debug!(
                            %node, %sender,
                            view = %manifest.view,
                            epoch = %manifest.epoch,
                            hashes = manifest.hashes.len(),
                            "recv dedup manifest"
                        );
                        if let Some(view_leader) = self.leader(manifest.view, manifest.epoch)
                            && view_leader == message.sender
                        {
                            self.block_builder.on_dedup_manifest(manifest)
                        }
                    },
                }
                None
            },
            MessageType::ProposalFetch(ProposalFetchMessage::Request(request)) => {
                let view = request.view_number();
                debug!(%node, %sender, %view, "recv proposal fetch request");
                if !request.validate_sender(&message.sender) {
                    warn!(
                        %node,
                        sender = %message.sender,
                        %view,
                        "ignoring invalid proposal fetch request signature"
                    );
                    return None;
                }
                if let Some(proposal) = self.consensus.signed_proposal(&view).cloned() {
                    let response = Message {
                        sender: self.public_key.clone(),
                        message_type: MessageType::ProposalFetch(ProposalFetchMessage::Response(
                            Box::new(proposal),
                        )),
                    };
                    if let Err(err) = self.network.sender().unicast(
                        self.consensus.current_view(),
                        &message.sender,
                        &response,
                    ) {
                        let err = CoordinatorError::from(err).context("proposal response");
                        warn!(%node, %err, "network error while sending proposal response");
                    }
                }
                None
            },
            MessageType::ProposalFetch(ProposalFetchMessage::Response(proposal)) => {
                debug!(
                    %node, %sender,
                    view = %proposal.data.view_number,
                    "recv proposal fetch response"
                );
                self.pending_proposal_fetches.resolve(&proposal);
                None
            },
            MessageType::External(data) => {
                debug!(%node, %sender, bytes = data.len(), "recv external message");
                self.coordinator_outbox
                    .push_back(CoordinatorOutput::ExternalMessageReceived {
                        sender: message.sender,
                        data,
                    });
                None
            },
        }
    }

    fn on_state_manager_output(
        &mut self,
        output: StateManagerOutput<T>,
    ) -> Option<ConsensusInput<T>> {
        match output {
            StateManagerOutput::State {
                response,
                validated: true,
            } => Some(ConsensusInput::StateValidated(response)),
            StateManagerOutput::State {
                response,
                validated: false,
            } => Some(ConsensusInput::StateValidationFailed(response)),
            StateManagerOutput::Header {
                response,
                header: Some(hdr),
            } => Some(ConsensusInput::HeaderCreated(
                response.view,
                proposal_commitment(&response.parent_proposal),
                hdr,
            )),
            StateManagerOutput::Header {
                response,
                header: None,
            } => {
                warn!(view = %response.view, "header creation failed");
                None
            },
        }
    }

    /// The VID erasure parameters the committee fixes for `target_epoch`,
    /// matching what an honest disperser derives. Used to reject shares whose
    /// `common.param` is forged (the commitment binds `ns_commits`, not
    /// `param`). `None` if the committee cannot be resolved.
    fn expected_vid_param(&self, target_epoch: Option<EpochNumber>) -> Option<AvidmGf2Param> {
        let membership = self
            .membership_coordinator
            .stake_table_for_epoch(target_epoch)
            .ok()?;
        let total_weight = vid_total_weight::<T, _>(membership.stake_table(), target_epoch);
        init_avidm_gf2_param(total_weight).ok()
    }

    fn on_proposal_and_vid_share(
        &mut self,
        validated: ValidatedProposal<T>,
        vid_share: VidDisperseShare2<T>,
    ) -> Result<ConsensusInput<T>, CoordinatorError> {
        self.storage.append_vid(vid_share.clone());
        self.storage
            .append_proposal(validated.message.proposal.data.clone());

        if let Some(state_cert) = &validated.message.proposal.data.state_cert {
            self.storage.append_state_cert(
                ViewNumber::new(state_cert.light_client_state.view_number),
                state_cert.clone(),
            );
        }

        let expected_param = self.expected_vid_param(vid_share.target_epoch);
        let proposal = &validated.message.proposal.data;
        self.vid_reconstructor.handle_proposal(
            proposal.view_number(),
            vid_share.payload_commitment,
            proposal.block_header.metadata().clone(),
            proposal.epoch,
            expected_param,
        );
        // This is our own share, addressed to us by the leader and already
        // verified by the share validator.
        self.vid_reconstructor
            .handle_vid_share(self.public_key.clone(), vid_share.clone());

        // GC for the cache
        let view = validated.message.proposal.data.view_number();
        self.cached_vid_shares = self
            .cached_vid_shares
            .split_off(&(view + 1, VidCommitment2::default()));
        self.cached_validated_proposals = self
            .cached_validated_proposals
            .split_off(&(view + 1, VidCommitment2::default()));

        Ok(ConsensusInput::ProposalWithVidShare(
            validated.sender,
            validated.message,
            vid_share,
        ))
    }

    fn broadcast(
        &self,
        message_type: ConsensusMessage<T, Validated>,
        ctx: &'static str,
    ) -> Result<(), CoordinatorError> {
        let message = Message {
            sender: self.public_key.clone(),
            message_type: MessageType::Consensus(message_type),
        };
        self.network
            .sender()
            .broadcast(self.consensus.current_view(), &message)
            .map_err(|e| CoordinatorError::from(e).context(ctx))
    }

    fn unicast_to_leader(
        &mut self,
        view: ViewNumber,
        epoch: EpochNumber,
        msg: BlockMessage<T>,
    ) -> Result<(), CoordinatorError> {
        let Some(leader) = self.leader(view, epoch) else {
            warn!(%view, %epoch, "failed to resolve leader for unicast");
            return Ok(());
        };
        let message = Message {
            sender: self.public_key.clone(),
            message_type: MessageType::Block(msg),
        };
        self.network
            .sender()
            .unicast(self.consensus.current_view(), &leader, &message)
            .map_err(|e| CoordinatorError::from(e).context("leader unicast"))
    }

    fn leader(&mut self, view: ViewNumber, epoch: EpochNumber) -> Option<T::SignatureKey> {
        let membership = self
            .membership_coordinator
            .membership_for_epoch(Some(epoch))
            .ok()?;
        membership.leader(view).ok()
    }

    fn on_client_request(&mut self, request: ClientRequest<T>) -> Result<(), CoordinatorError> {
        match request {
            ClientRequest::CurrentView(tx) => {
                let _ = tx.send(self.consensus.current_view());
            },
            ClientRequest::CurrentEpoch(tx) => {
                let _ = tx.send(self.consensus.current_epoch());
            },
            ClientRequest::DecidedLeaf(tx) => {
                let _ = tx.send(self.consensus.last_decided_leaf().clone());
            },
            ClientRequest::DecidedState(tx) => {
                let view = self.consensus.last_decided_leaf().view_number();
                let _ = tx.send(self.state(view).map(|s| s.state.clone()));
            },
            ClientRequest::UndecidedLeaves(tx) => {
                let _ = tx.send(self.consensus.undecided_leaves().cloned().collect());
            },
            ClientRequest::GetState { view, respond } => {
                let _ = respond.send(self.state(view).map(|s| s.state.clone()));
            },
            ClientRequest::GetStateAndDelta { view, respond } => {
                let _ = respond.send(match self.state(view) {
                    Some(s) => (Some(s.state.clone()), s.delta.clone()),
                    None => (None, None),
                });
            },
            ClientRequest::ProposalParticipation { epoch, respond } => {
                let _ = respond.send(match epoch {
                    Some(epoch) => self.participation.proposal_participation(epoch),
                    None => self.participation.current_proposal_participation(),
                });
            },
            ClientRequest::VoteParticipation { epoch, respond } => {
                let _ = respond.send(match epoch {
                    Some(epoch) => self.participation.vote_participation(epoch),
                    None => self.participation.current_vote_participation(),
                });
            },
            ClientRequest::SubmitTransaction { tx, respond } => {
                self.block_builder.on_submit_transaction(tx);
                let _ = respond.send(());
            },
            ClientRequest::UpdateLeaf { update, respond } => {
                self.state_manager.update_state(update);
                let _ = respond.send(());
            },
            ClientRequest::RequestProposal {
                view,
                leaf_commitment,
                respond,
            } => {
                if let Some(proposal) = self.consensus.signed_proposal(&view)
                    && proposal_commitment(&proposal.data) == leaf_commitment
                {
                    let _ = respond.send(Ok(proposal.clone()));
                    return Ok(());
                }
                if !self
                    .pending_proposal_fetches
                    .contains_request(view, leaf_commitment)
                {
                    let request =
                        self.consensus
                            .signed_proposal_fetch_request(view)
                            .map_err(|err| {
                                let err = format!("failed to sign proposal request: {err}");
                                CoordinatorError::regular(err).context("sign proposal request")
                            })?;

                    let message = Message {
                        sender: self.public_key.clone(),
                        message_type: MessageType::ProposalFetch(ProposalFetchMessage::Request(
                            request,
                        )),
                    };

                    self.network
                        .sender()
                        .broadcast(self.consensus.current_view(), &message)
                        .map_err(|err| {
                            CoordinatorError::from(err).context("broadcast proposal request")
                        })?;
                }
                self.pending_proposal_fetches
                    .push(view, leaf_commitment, respond);
            },
            ClientRequest::SendExternalMessage {
                payload,
                recipient,
                respond,
            } => {
                let message = Message {
                    sender: self.public_key.clone(),
                    message_type: MessageType::External(payload),
                };
                let result = self
                    .network
                    .sender()
                    .unicast(self.consensus.current_view(), &recipient, &message)
                    .map_err(|err| {
                        CoordinatorError::from(err)
                            .context("send external message")
                            .into()
                    });
                let _ = respond.send(result);
            },
            ClientRequest::SeedPreCutover { seed, respond } => {
                let current_view = self.consensus.current_view();
                if seed.cutover_view > ViewNumber::genesis() && current_view >= seed.cutover_view {
                    info!(
                        node = %self.node_id,
                        %current_view,
                        cutover_view = *seed.cutover_view,
                        "ignoring pre-cutover seed; already past the cutover",
                    );
                    let _ = respond.send(());
                    return Ok(());
                }
                info!(
                    node = %self.node_id,
                    undecided = seed.undecided.len(),
                    anchor_view = *seed.decided_anchor.view_number(),
                    high_qc_view = seed.high_qc.as_ref().map(|qc| *qc.view_number()),
                    cutover_view = *seed.cutover_view,
                    states = seed.validated_states.len(),
                    "applying legacy -> new-protocol seed",
                );

                // State manager is owned by the coordinator, so the
                // validated-state map must be applied here before the
                // seed is consumed by consensus.
                let anchor_view = seed.decided_anchor.view_number();
                if let Some(state) = seed.validated_states.get(&anchor_view).cloned() {
                    self.state_manager
                        .seed_state(anchor_view, state, seed.decided_anchor.clone());
                }
                for leaf in &seed.undecided {
                    let view = leaf.view_number();
                    if let Some(state) = seed.validated_states.get(&view).cloned() {
                        self.state_manager.seed_state(view, state, leaf.clone());
                    }
                }

                let highest_seeded_leaf = seed.undecided.last().unwrap_or(&seed.decided_anchor);
                let cutover_epoch = EpochNumber::new(epoch_from_block_number(
                    highest_seeded_leaf.block_header().block_number(),
                    *self.consensus.epoch_height,
                ));
                let cutover_view = seed.cutover_view;

                self.consensus.apply_pre_cutover_seed(seed);

                // Refresh peers for the cutover epoch before kicking the
                // leader — the proposal-driven site can't fire yet.
                if let Err(err) = self
                    .network
                    .apply_epoch(cutover_epoch, &self.membership_coordinator)
                {
                    error!(
                        %cutover_epoch,
                        %err,
                        "network on_epoch_change failed during seed_pre_cutover",
                    );
                }

                let cur_view = self.consensus.current_view();
                if self.consensus.timeout_cert_at(cur_view).is_some() {
                    self.resume_after_cutover_tc();
                } else if cur_view + 1 == cutover_view
                    && self.consensus.cert1_at(cur_view).is_some()
                    && self.consensus.proposal_at(cur_view).is_some()
                {
                    self.start();
                } else {
                    let epoch = self
                        .consensus
                        .current_epoch()
                        .unwrap_or(EpochNumber::genesis());
                    self.outbox
                        .push_back(ConsensusOutput::ViewChanged(cur_view, epoch));
                }
                while let Some(output) = self.outbox.pop_front() {
                    if let Err(err) = self.process_consensus_output(output) {
                        warn!(
                            %err,
                            "error processing post-seed bootstrap output"
                        );
                    }
                }
                let _ = respond.send(());
            },
            ClientRequest::SubmitTimeoutVote { vote, respond } => {
                let view = vote.view_number();
                let current_view = self.consensus.current_view();
                if view < current_view {
                    debug!(
                        %view, %current_view,
                        "ignoring bridged timeout vote for stale view"
                    );
                    let _ = respond.send(());
                    return Ok(());
                }
                self.timeout_collector.accumulate_vote(vote.clone());
                self.timeout_one_honest_collector
                    .accumulate_vote(vote.clone());
                // Rebroadcast so peer coordinators can aggregate too.
                let message = Message {
                    sender: self.public_key.clone(),
                    message_type: MessageType::Consensus(ConsensusMessage::TimeoutVote(
                        message::TimeoutVoteMessage { vote, lock: None },
                    )),
                };
                if let Err(err) = self
                    .network
                    .sender()
                    .broadcast(self.consensus.current_view(), &message)
                {
                    warn!(%err, "failed to rebroadcast bridged timeout vote");
                }
                let _ = respond.send(());
            },
            ClientRequest::SubmitLegacyHighQc { qc, respond } => {
                // QC certifies the last legacy view; cutover view is the next.
                // Register idempotently so the smooth-start precondition holds
                // regardless of arrival order vs. the cutover seed.
                let qc_view = qc.view_number();
                let cutover_view = qc_view + 1;
                self.consensus.register_legacy_qc(&qc);

                // Still parked on the last legacy view (seed landed without this
                // QC, waiting out the timer) and not yet skipped via TC2: propose
                // the cutover view on the real QC now. Self-idempotent — once
                // started, `cur_view` advances past `qc_view` and `maybe_propose`
                // dedups by `proposed_views`.
                let cur_view = self.consensus.current_view();
                if cur_view == qc_view
                    && self.consensus.timeout_cert_at(cutover_view).is_none()
                    && self.consensus.cert1_at(qc_view).is_some()
                    && self.consensus.proposal_at(qc_view).is_some()
                {
                    info!(
                        %cutover_view,
                        "bridged late legacy high QC; proposing cutover view on it (no timeout)"
                    );
                    self.start();
                    while let Some(output) = self.outbox.pop_front() {
                        if let Err(err) = self.process_consensus_output(output) {
                            warn!(
                                %err,
                                "error processing bridged-high-qc bootstrap output"
                            );
                        }
                    }
                }
                let _ = respond.send(());
            },
            ClientRequest::BumpNetworkEpoch { epoch, respond } => {
                if let Err(err) = self
                    .network
                    .apply_epoch(epoch, &self.membership_coordinator)
                {
                    warn!(%epoch, %err, "network on_epoch_change failed");
                }
                let _ = respond.send(());
            },
        }

        Ok(())
    }

<<<<<<< HEAD
    /// Kick the leader after the seed lands when a forwarded TC2 had
    /// already advanced `current_view`. No-op unless leader and all
    /// prerequisites are present.
    fn resume_after_cutover_tc(&mut self) {
        let cur_view = self.consensus.current_view();
        if self.consensus.timeout_cert_at(cur_view).is_none() {
            return;
        }
        let epoch = self
||||||| parent of 3013c5ed3f7 ([Fast Finality] Metric Parity b/t Protocols (#4674))
    /// Broadcast a signed proposal fetch request for `view` to all peers.
    fn broadcast_proposal_fetch(&mut self, view: ViewNumber) -> Result<(), CoordinatorError> {
        let request = self
=======
    fn record_voted_view(&mut self, view: ViewNumber) {
        if self.voted_view.is_some_and(|v| v >= view) {
            return;
        }
        self.voted_view = Some(view);
        if let Some(m) = &self.metrics {
            m.consensus.last_voted_view.set(*view as usize);
        }
    }

    fn on_view_changed_metrics(&mut self, view: ViewNumber, epoch: EpochNumber) {
        if let Some((started_view, started_epoch, started_at)) = self.view_started
            && started_view == view
        {
            if epoch > started_epoch {
                self.view_started = Some((view, epoch, started_at));
            }
            return;
        }
        let prev = self.view_started.replace((view, epoch, Instant::now()));
        let duration_as_leader = prev.and_then(|(prev_view, prev_epoch, entered)| {
            (self.leader(prev_view, prev_epoch).as_ref() == Some(&self.public_key))
                .then(|| entered.elapsed())
        });
        let Some(m) = &self.metrics else { return };
        let consensus = &m.consensus;
        consensus.current_view.set(*view as usize);
        let invalid_certs = self
            .consensus
            .invalid_certs()
            .saturating_sub(self.invalid_certs_at_decide);
        consensus.invalid_qc.set(invalid_certs as usize);
        let last_decided_view = self.consensus.last_decided_view();
        if view > last_decided_view {
            consensus
                .number_of_views_since_last_decide
                .set((*view - *last_decided_view) as usize);
        }
        if let Some(duration) = duration_as_leader {
            consensus
                .view_duration_as_leader
                .add_point(duration.as_secs_f64());
        }
        let (outstanding_txns, outstanding_bytes) = self.block_builder.outstanding_transactions();
        consensus.outstanding_transactions.set(outstanding_txns);
        consensus
            .outstanding_transactions_memory_size
            .set(outstanding_bytes);
    }

    fn on_decide_metrics(&mut self, leaves: &[Leaf2<T>]) {
        let Some(newest) = leaves.first() else { return };
        // The consensus watermark already includes this batch, so the batch
        // advanced the decide frontier iff its newest view is the watermark;
        // a gap-fill decide of older views must not regress the gauges.
        let advanced = newest.view_number() == self.consensus.last_decided_view();
        if advanced {
            self.invalid_certs_at_decide = self.consensus.invalid_certs();
        }
        let Some(m) = &self.metrics else { return };
        let consensus = &m.consensus;
        let now = OffsetDateTime::now_utc().unix_timestamp();
        for leaf in leaves {
            let txn_bytes = self
                .payload_txn_bytes
                .get(&leaf.view_number())
                .copied()
                .or_else(|| leaf.block_payload_ref().map(|p| p.txn_bytes()));
            if let Some(txn_bytes) = txn_bytes {
                consensus.finalized_bytes.add_point(txn_bytes as f64);
            }
            if advanced {
                match (now as u64).checked_sub(leaf.block_header().timestamp()) {
                    Some(age) => consensus.proposal_to_decide_time.add_point(age as f64),
                    None => error!(
                        timestamp = leaf.block_header().timestamp(),
                        "failed to calculate proposal to decide time: timestamp in the future"
                    ),
                }
            }
        }
        if !advanced {
            return;
        }
        consensus.last_decided_time.set(now as usize);
        consensus.invalid_qc.set(0);
        consensus
            .last_decided_view
            .set(*newest.view_number() as usize);
        consensus
            .last_synced_block_height
            .set(newest.block_header().block_number() as usize);
        if let Some(views_in_flight) =
            (*self.consensus.current_view()).checked_sub(*newest.view_number())
        {
            consensus
                .number_of_views_per_decide_event
                .add_point(views_in_flight as f64);
        }
    }

    /// Broadcast a signed proposal fetch request for `view` to all peers.
    fn broadcast_proposal_fetch(&mut self, view: ViewNumber) -> Result<(), CoordinatorError> {
        let request = self
>>>>>>> 3013c5ed3f7 ([Fast Finality] Metric Parity b/t Protocols (#4674))
            .consensus
            .current_epoch()
            .unwrap_or(EpochNumber::genesis());
        let Some(leader) = self.leader(cur_view, epoch) else {
            return;
        };
        if leader != self.public_key {
            return;
        }
        let Some(locked_view) = self.consensus.locked_view() else {
            return;
        };
        let Some(parent_proposal) = self.consensus.proposal_at(locked_view).cloned() else {
            return;
        };
        self.outbox
            .push_back(ConsensusOutput::RequestBlockAndHeader(
                BlockAndHeaderRequest {
                    view: cur_view,
                    epoch,
                    parent_proposal,
                },
            ));
    }

    fn gc(&mut self, epoch: EpochNumber, scope: GcScope) -> Result<(), CoordinatorError> {
        self.consensus.gc(scope);
        match scope {
            GcScope::Local(view) => {
                let vc = VidCommitment2::default();
                self.block_builder.gc(view);
                self.cached_validated_proposals =
                    self.cached_validated_proposals.split_off(&(view, vc));
                self.cached_vid_shares = self.cached_vid_shares.split_off(&(view, vc));
                self.vid_disperser.gc(view);
                self.vid_fragment_accumulator.gc(view);
                // When we enter a new view, we do not want to GC certain data
                // for the previous view yet:
                let view = view.saturating_sub(1).into();
                self.network.gc(view)?;
                self.timeout_collector.gc(view);
                self.timeout_one_honest_collector.gc(view);
                self.vote1_collector.gc(view);
                self.vote2_collector.gc(view);
            },
            GcScope::Decided(view) => {
                self.epoch_manager.gc(epoch);
                self.epoch_root_collector.gc(view);
                self.pending_proposal_fetches.gc(view);
                self.state_manager.gc(view);
                self.storage
                    .gc(view.saturating_sub(STORAGE_GC_MARGIN).into());
                self.vid_reconstructor
                    .gc(view.saturating_sub(VID_RECONSTRUCT_GC_MARGIN).into());
                let vc = VidCommitment2::default();
                self.da_payloads = self.da_payloads.split_off(&(view, vc));
                self.payload_txn_bytes = self
                    .payload_txn_bytes
                    .split_off(&(self.consensus.decide_floor() + 1));
            },
            GcScope::Timeout(view) => {
                self.vid_reconstructor.retire_view(view);
                let vc = VidCommitment2::default();
                self.da_payloads
                    .extract_if((view, vc)..(view + 1, vc), |_, _| true)
                    .for_each(drop);
            },
        }
        Ok(())
    }

    /// We ignore votes more that 30 views ahead of our current view.
    fn is_too_far_ahead(&self, v: ViewNumber) -> bool {
        v > self.consensus.current_view() + 30
    }
}

/// Garbage collection scope.
#[derive(Debug, Clone, Copy)]
pub enum GcScope {
    /// GC is invoked on local view changes.
    Local(ViewNumber),
    /// GC is invoked on local decided views.
    Decided(ViewNumber),
    /// GC is invoked on a view that advanced via timeout certificate.
    Timeout(ViewNumber),
}

/// A payload built locally and awaiting DA persistence.
struct PendingDa<T: NodeType> {
    epoch: EpochNumber,
    payload: T::BlockPayload,
    metadata: <T::BlockPayload as BlockPayload<T>>::Metadata,
}

type ProposalFetchResponseSender<T> =
    oneshot::Sender<Result<SignedProposal<T, Proposal<T>>, QueryError>>;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct ProposalFetchKey<T: NodeType> {
    view: ViewNumber,
    leaf_commitment: Commitment<Leaf2<T>>,
}

impl<T: NodeType> ProposalFetchKey<T> {
    fn new(view: ViewNumber, leaf_commitment: Commitment<Leaf2<T>>) -> Self {
        Self {
            view,
            leaf_commitment,
        }
    }
}

#[derive(Default)]
struct PendingProposalFetches<T: NodeType> {
    pending: HashMap<ProposalFetchKey<T>, Vec<ProposalFetchResponseSender<T>>>,
}

impl<T: NodeType> PendingProposalFetches<T> {
    fn prune_closed(&mut self) {
        self.pending.retain(|_, responders| {
            responders.retain(|respond| !respond.is_closed());
            !responders.is_empty()
        });
    }

    fn contains_request(
        &mut self,
        view: ViewNumber,
        leaf_commitment: Commitment<Leaf2<T>>,
    ) -> bool {
        self.prune_closed();
        self.pending
            .contains_key(&ProposalFetchKey::new(view, leaf_commitment))
    }

    fn push(
        &mut self,
        view: ViewNumber,
        leaf_commitment: Commitment<Leaf2<T>>,
        respond: ProposalFetchResponseSender<T>,
    ) {
        self.pending
            .entry(ProposalFetchKey::new(view, leaf_commitment))
            .or_default()
            .push(respond);
    }

    fn gc(&mut self, view: ViewNumber) {
        self.pending.retain(|key, responders| {
            responders.retain(|respond| !respond.is_closed());
            key.view >= view && !responders.is_empty()
        });
    }

    fn resolve(&mut self, proposal: &SignedProposal<T, Proposal<T>>) {
        self.prune_closed();
        let view = proposal.data.view_number;
        let leaf_commitment = proposal_commitment(&proposal.data);
        let key = ProposalFetchKey::new(view, leaf_commitment);

        if let Some(responders) = self.pending.remove(&key) {
            for respond in responders {
                let _ = respond.send(Ok(proposal.clone()));
            }
        }
    }
}
