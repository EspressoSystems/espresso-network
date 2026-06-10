pub mod error;
mod metrics;
pub mod timer;

use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
    time::Duration,
};

use bon::{Builder, bon};
use committable::Commitment;
use hotshot::{HotShotInitializer, traits::BlockPayload, types::SignatureKey};
use hotshot_types::{
    data::{EpochNumber, Leaf2, VidCommitment, VidCommitment2, VidDisperseShare2, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    message::{Proposal as SignedProposal, UpgradeLock},
    simple_certificate::{QuorumCertificate2, TimeoutCertificate2},
    simple_vote::{HasEpoch, QuorumVote2, TimeoutVote2},
    traits::{
        block_contents::BlockHeader, metrics::Metrics, node_implementation::NodeType,
        signature_key::StateSignatureKey,
    },
    utils::{epoch_from_block_number, is_epoch_root},
    vote::HasViewNumber,
};
use metrics::Measurement;
use tokio::{select, sync::oneshot};
use tracing::{debug, error, info, warn};

use crate::{
    block::{BlockAndHeaderRequest, BlockBuilder, BlockBuilderConfig},
    client::{ClientApi, ClientRequest, CoordinatorClient, QueryError},
    consensus::{Consensus, ConsensusInput, ConsensusOutput},
    coordinator::{
        error::{CoordinatorError, ErrorSource, Severity},
        metrics::finish_measurement,
        timer::Timer,
    },
    epoch::{EpochManager, EpochRootResult},
    epoch_root_vote_collector::EpochRootVoteCollector,
    helpers::proposal_commitment,
    logging::KeyPrefix,
    message::{
        self, BlockMessage, Certificate2, ConsensusMessage, Message, MessageType, Proposal,
        ProposalFetchMessage, ProposalMessage, TimeoutOneHonest, TransactionMessage, Unchecked,
        Vote2,
    },
    network::Network,
    outbox::Outbox,
    proposal::{ProposalValidator, ValidatedProposal, VidShareValidator},
    state::{HeaderRequest, StateEntry, StateManager, StateManagerOutput},
    storage::{NewProtocolStorage, Storage},
    vid::{VidDisperseRequest, VidDisperser, VidReconstructor},
    vote::VoteCollector,
};

#[allow(clippy::large_enum_variant)]
pub enum CoordinatorOutput<T: NodeType> {
    Consensus(ConsensusOutput<T>),
    ExternalMessageReceived {
        sender: T::SignatureKey,
        data: Vec<u8>,
    },
}

#[derive(Builder)]
pub struct Coordinator<T: NodeType, N, S> {
    membership_coordinator: EpochMembershipCoordinator<T>,
    consensus: Consensus<T>,
    network: N,
    state_manager: StateManager<T>,
    #[builder(default)]
    client: CoordinatorClient<T>,
    vid_disperser: VidDisperser<T>,
    vid_reconstructor: VidReconstructor<T>,
    vote1_collector: VoteCollector<T, QuorumVote2<T>, QuorumCertificate2<T>>,
    vote2_collector: VoteCollector<T, Vote2<T>, Certificate2<T>>,
    timeout_collector: VoteCollector<T, TimeoutVote2<T>, TimeoutCertificate2<T>>,
    timeout_one_honest_collector: VoteCollector<T, TimeoutVote2<T>, TimeoutOneHonest<T>>,
    epoch_root_collector: EpochRootVoteCollector<T>,
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
}

#[bon]
impl<T, N, S> Coordinator<T, N, S>
where
    T: NodeType,
    N: Network<T>,
    S: NewProtocolStorage<T>,
{
    #[builder(builder_type = CoordinatorMaker, finish_fn = make)]
    #[allow(clippy::too_many_arguments)]
    pub fn maker(
        membership_coordinator: EpochMembershipCoordinator<T>,
        network: N,
        initializer: &HotShotInitializer<T>,
        upgrade_lock: UpgradeLock<T>,
        public_key: T::SignatureKey,
        private_key: <T::SignatureKey as SignatureKey>::PrivateKey,
        state_private_key: <T::StateSignatureKey as StateSignatureKey>::StatePrivateKey,
        stake_table_capacity: usize,
        timeout_duration: Duration,
        storage: S,
        metrics: &dyn Metrics,
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

        let mut state_manager = StateManager::new(
            Arc::new(initializer.instance_state.clone()),
            upgrade_lock.clone(),
        );
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
        consensus.seed_parent(cert1, parent_proposal, reconstructed_blocks);
        consensus.set_view(anchor_view, anchor_epoch);

        let lock = upgrade_lock.clone();
        Self::builder()
            .consensus(consensus)
            .network(network)
            .state_manager(state_manager)
            .vid_disperser(VidDisperser::new(membership_coordinator.clone()))
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
            .epoch_root_collector(EpochRootVoteCollector::new(
                membership_coordinator.clone(),
                lock,
            ))
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
            .storage(Storage::new(storage, private_key))
            .membership_coordinator(membership_coordinator)
            .timer(Timer::new(
                timeout_duration,
                ViewNumber::genesis(),
                EpochNumber::genesis(),
            ))
            .public_key(public_key)
            .maybe_metrics(
                metrics
                    .is_recording()
                    .then(|| metrics::Metrics::new(metrics)),
            )
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
            let parent_proposal = self
                .consensus
                .proposal_at(cur_view)
                .expect("parent proposal must be seeded before start()")
                .clone();
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

    pub async fn stop(mut self) {
        self.network.shutdown().await
    }

    pub async fn next_consensus_input(&mut self) -> Result<ConsensusInput<T>, CoordinatorError> {
        loop {
            let next_input = self
                .metrics
                .as_ref()
                .map(|m| Measurement::start(m.next_consensus_input.clone()));
            select! {
                message = self.network.receive() => match message {
                    Ok(m) => {
                        finish_measurement(next_input);
                        if let Some(input) = self.on_network_message(m).await {
                            return Ok(input)
                        }
                    }
                    Err(e) => {
                        finish_measurement(next_input);
                        return Err(CoordinatorError::from(e).context("network receive"))
                    }
                },
                () = &mut self.timer => {
                    let view = self.timer.view();
                    let epoch = self.timer.epoch();
                    if let Some(stats) = self.vote1_collector.stats(view, epoch).await {
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
                    finish_measurement(next_input);
                    if let Some(m) = &mut self.metrics {
                        m.timeouts.add(1)
                    }
                    return Ok(input)
                }
                Some(output) = self.state_manager.next() => {
                    finish_measurement(next_input);
                    if let Some(input) = self.on_state_manager_output(output) {
                        return Ok(input)
                    }
                }
                Some(request) = self.client.next_request() => {
                    finish_measurement(next_input);
                    if let Err(err) = self.on_client_request(request).await {
                        error!(%err, "error while handling client request");
                    }
                }
                Some(tcert) = self.timeout_collector.next() => {
                    finish_measurement(next_input);
                    return Ok(ConsensusInput::TimeoutCertificate(tcert))
                }
                Some(out) = self.timeout_one_honest_collector.next() => {
                    finish_measurement(next_input);
                    let Some(epoch) = out.data.epoch else {
                        let msg = format!("missing epoch in view {}", out.view_number());
                        return Err(CoordinatorError::regular(msg).context("gc timeout one honest"))
                    };
                    return Ok(ConsensusInput::TimeoutOneHonest(out.view_number(), epoch))
                }
                Some(cert1) = self.vote1_collector.next() => {
                    finish_measurement(next_input);
                    let view = cert1.view_number();
                    crate::trace_leader_event!(
                        self.consensus.tracer,
                        view,
                        crate::leader_trace::LeaderEvent::Cert1VMinus1InputDispatched
                    );
                    return Ok(ConsensusInput::Certificate1(cert1))
                }
                Some(cert2) = self.vote2_collector.next() => {
                    finish_measurement(next_input);
                    let view = cert2.view_number();
                    crate::trace_leader_event!(
                        self.consensus.tracer,
                        view,
                        crate::leader_trace::LeaderEvent::Cert2VMinus1InputDispatched
                    );
                    return Ok(ConsensusInput::Certificate2(cert2))
                }
                Some((cert1, state_cert)) = self.epoch_root_collector.next() => {
                    finish_measurement(next_input);
                    self.storage.append_state_cert(
                        ViewNumber::new(state_cert.light_client_state.view_number),
                        state_cert.clone(),
                    );
                    return Ok(ConsensusInput::EpochRootCertificates { cert1, state_cert })
                }
                Some(item) = self.share_validator.next() => match item {
                    Ok(vid_share) => {
                        finish_measurement(next_input);
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
                        finish_measurement(next_input);
                        return Err(CoordinatorError::regular(e).context("vid share validation"))
                    }
                },
                Some(item) = self.proposal_validator.next() => match item {
                    Ok(validated) => {
                        finish_measurement(next_input);
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
                        finish_measurement(next_input);
                        return Err(CoordinatorError::regular(e).context("proposal validation"))
                    }
                },
                Some(item) = self.block_builder.next() => match item {
                    Ok(block) => {
                        finish_measurement(next_input);
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
                        self.vid_reconstructor.mark_reconstructed(block.view);
                        self.unicast_to_leader(
                            next_view,
                            epoch,
                            BlockMessage::DedupManifest(manifest),
                        )?;
                        return Ok(block.into())
                    }
                    Err(err) => {
                        finish_measurement(next_input);
                        return Err(CoordinatorError::regular(err).context("block building"))
                    }
                },
                Some(item) = self.vid_disperser.next() => match item {
                    Ok(out) => {
                        finish_measurement(next_input);
                        return Ok(ConsensusInput::VidDisperseCreated(out.view, out.disperse))
                    }
                    Err(()) => {
                        finish_measurement(next_input);
                        return Err(CoordinatorError::unspecified().context("vid disperse"))
                    }
                },
                Some(item) = self.vid_reconstructor.next() => match item {
                    Ok(out) => {
                        finish_measurement(next_input);
                        self.block_builder.on_block_reconstructed(out.tx_commitments);
                        self.storage.append_da(
                            out.view,
                            out.epoch,
                            out.payload.clone(),
                            out.metadata.clone(),
                            VidCommitment::V2(out.payload_commitment),
                        );
                        if let Some(proposal) = self.consensus.proposal_at(out.view) {
                            self.outbox.push_back(ConsensusOutput::BlockPayloadReconstructed {
                                view: out.view,
                                header: proposal.block_header.clone(),
                                payload: out.payload,
                            });
                        }
                        return Ok(ConsensusInput::BlockReconstructed(out.view, out.payload_commitment))
                    }
                    Err(()) => {
                        finish_measurement(next_input);
                        return Err(CoordinatorError::unspecified().context("vid reconstruction"))
                    }
                },
                Some(result) = self.epoch_manager.next() => match result {
                    Ok(EpochRootResult::DrbResult(epoch, drb_result)) => {
                        finish_measurement(next_input);
                        // New epoch data available — retry votes that were
                        // buffered because their membership wasn't ready.
                        self.vote1_collector.retry_pending_votes().await;
                        self.vote2_collector.retry_pending_votes().await;
                        self.timeout_collector.retry_pending_votes().await;
                        self.timeout_one_honest_collector.retry_pending_votes().await;
                        return Ok(ConsensusInput::DrbResult(epoch, drb_result))
                    }
                    Ok(EpochRootResult::EpochRootAdded(epoch)) => {
                        finish_measurement(next_input);
                        debug!(%epoch, "epoch root added to membership");
                        continue;
                    }
                    Err(failure) => {
                        finish_measurement(next_input);
                        // Catchup/compute failed. The epoch manager clears
                        // the pending guard; consensus's `maybe_propose`
                        // will re-request the DRB when it next tries to
                        // build a transition proposal and finds it missing.
                        warn!(%failure.error, epoch = %failure.epoch, "DRB request failed");
                        continue;
                    }
                },
                else => {
                    finish_measurement(next_input);
                    return Err(CoordinatorError::critical(ErrorSource::NoInput))
                }
            }
        }
    }

    pub fn apply_consensus(&mut self, input: ConsensusInput<T>) {
        let _m = self
            .metrics
            .as_ref()
            .map(|m| Measurement::start(m.apply_consensus.clone()));
        self.consensus.apply(input, &mut self.outbox)
    }

    pub fn process_consensus_output(
        &mut self,
        output: ConsensusOutput<T>,
    ) -> Result<(), CoordinatorError> {
        let node = self.node_id;
        let _m = self
            .metrics
            .as_ref()
            .map(|m| Measurement::start(m.process_consensus_output.clone()));
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
                crate::trace_leader_event!(
                    self.consensus.tracer,
                    view,
                    crate::leader_trace::LeaderEvent::NsDisperseStart
                );
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
                for leaf in leaves {
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
            ConsensusOutput::SendProposal(proposal) => {
                let view = proposal.data.view_number;
                let epoch = proposal.data.epoch;
                let block = proposal.data.block_header.block_number();
                info!(%node, %view, %epoch, %block, "send proposal");
                self.storage.append_proposal(proposal.data.clone());
                // Two blocks can be built for one view. Here we know which one
                // wins and we persist just that one:
                if let VidCommitment::V2(commit) = proposal.data.block_header.payload_commitment() {
                    if let Some(da) = self.da_payloads.remove(&(view, commit)) {
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
                // TODO: This may be done async in network so we do not spend
                // too much time here in this loop.

                let view = proposal.data.view_number();
                let message = Message {
                    sender: self.public_key.clone(),
                    message_type: MessageType::Consensus(ConsensusMessage::Proposal(
                        ProposalMessage::validated(proposal.clone()),
                    )),
                };
                crate::trace_leader_event!(
                    self.consensus.tracer,
                    view,
                    crate::leader_trace::LeaderEvent::ProposalBroadcastStart
                );
                if let Err(err) = self
                    .network
                    .broadcast(self.consensus.current_view(), &message)
                {
                    let err = CoordinatorError::from(err).context("proposal broadcast");
                    if err.severity == Severity::Critical {
                        return Err(err);
                    } else {
                        warn!(%node, %err, "network error while broadcasting proposal")
                    }
                }
                crate::trace_leader_event!(
                    self.consensus.tracer,
                    view,
                    crate::leader_trace::LeaderEvent::ProposalBroadcastEnd
                );
            },
            ConsensusOutput::SendVidShares(vid_shares) => {
                debug!(%node, count = vid_shares.len(), "send vid shares");
                // Move the per-recipient serialize + unicast loop onto a
                // blocking thread so the main coordinator loop can keep
                // processing votes while ~99 shares are being fanned out.
                // We snapshot a Clone-able `CliquenetSender` from the network
                // and ship it into the spawn — no `&mut self` borrow held.
                let sender = self.network.sender();
                let public_key = self.public_key.clone();
                let tracer = self.consensus.tracer.clone();
                let trace_view: u64 = vid_shares
                    .first()
                    .map(|s| *s.data.view_number())
                    .unwrap_or(0);
                tokio::task::spawn_blocking(move || {
                    crate::trace_leader_event!(
                        tracer,
                        trace_view,
                        crate::leader_trace::LeaderEvent::VidSharesUnicastStart
                    );
                    for share in vid_shares {
                        let recipient = share.data.recipient_key.clone();
                        let view = share.data.view_number();
                        let message = Message {
                            sender: public_key.clone(),
                            message_type: MessageType::Consensus(ConsensusMessage::VidShare(share)),
                        };
                        if let Err(err) = crate::network::NetworkSender::unicast(
                            &sender, view, &recipient, &message,
                        ) {
                            // Inside spawn_blocking we can't propagate Err to
                            // the coordinator loop; log and continue.  Severity
                            // is not actionable here.
                            warn!(%err, "network error while sending vid share");
                        }
                    }
                    crate::trace_leader_event!(
                        tracer,
                        trace_view,
                        crate::leader_trace::LeaderEvent::VidSharesUnicastEnd
                    );
                });
            },
            ConsensusOutput::SendTimeoutVote(vote, lock) => {
                let view = vote.view_number();
                debug!(%node, %view, has_lock = lock.is_some(), "send timeout vote");
                let message = Message {
                    sender: self.public_key.clone(),
                    message_type: MessageType::Consensus(ConsensusMessage::TimeoutVote(
                        message::TimeoutVoteMessage { vote, lock },
                    )),
                };
                self.network
                    .broadcast(self.consensus.current_view(), &message)
                    .map_err(|e| CoordinatorError::from(e).context("broadcast timeout vote"))?
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
                let message = Message {
                    sender: self.public_key.clone(),
                    message_type: MessageType::Consensus(ConsensusMessage::Vote1(vote1)),
                };
                crate::trace_leader_event!(
                    self.consensus.tracer,
                    view,
                    crate::leader_trace::LeaderEvent::Vote1BroadcastStart
                );
                let r = self.network
                    .broadcast(self.consensus.current_view(), &message)
                    .map_err(|e| CoordinatorError::from(e).context("broadcast vote1"));
                crate::trace_leader_event!(
                    self.consensus.tracer,
                    view,
                    crate::leader_trace::LeaderEvent::Vote1BroadcastEnd
                );
                r?
            },
            ConsensusOutput::SendVote2(vote2) => {
                let view = vote2.view_number();
                debug!(%node, %view, "send vote2");
                let message = Message {
                    sender: self.public_key.clone(),
                    message_type: MessageType::Consensus(ConsensusMessage::Vote2(vote2)),
                };
                crate::trace_leader_event!(
                    self.consensus.tracer,
                    view,
                    crate::leader_trace::LeaderEvent::Vote2VMinus1BroadcastStart
                );
                self.network
                    .broadcast(self.consensus.current_view(), &message)
                    .map_err(|e| CoordinatorError::from(e).context("broadcast vote2"))?;
                crate::trace_leader_event!(
                    self.consensus.tracer,
                    view,
                    crate::leader_trace::LeaderEvent::Vote2VMinus1BroadcastEnd
                );
            },
            ConsensusOutput::SendEpochChange(epoch_change) => {
                info!(
                    %node,
                    view = %epoch_change.cert1.view_number(),
                    epoch = ?epoch_change.cert1.epoch().map(|e| *e),
                    "send epoch change"
                );
                let message = Message {
                    sender: self.public_key.clone(),
                    message_type: MessageType::Consensus(ConsensusMessage::EpochChange(
                        epoch_change,
                    )),
                };
                self.network
                    .broadcast(self.consensus.current_view(), &message)
                    .map_err(|e| CoordinatorError::from(e).context("broadcast epoch change"))?
            },
            ConsensusOutput::SendCertificate1(cert1) => {
                let view = cert1.view_number();
                debug!(
                    %node,
                    %view,
                    epoch = ?cert1.epoch().map(|e| *e),
                    "send certificate1"
                );
                let message = Message {
                    sender: self.public_key.clone(),
                    message_type: MessageType::Consensus(ConsensusMessage::Certificate1(
                        cert1,
                        self.public_key.clone(),
                    )),
                };
                crate::trace_leader_event!(
                    self.consensus.tracer,
                    view,
                    crate::leader_trace::LeaderEvent::Cert1VMinus1BroadcastStart
                );
                self.network
                    .broadcast(self.consensus.current_view(), &message)
                    .map_err(|e| CoordinatorError::from(e).context("broadcast certificate1"))?;
                crate::trace_leader_event!(
                    self.consensus.tracer,
                    view,
                    crate::leader_trace::LeaderEvent::Cert1VMinus1BroadcastEnd
                );
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
                self.consensus.set_view(view, epoch);
                self.timer.reset_with_epoch(view, epoch);
                self.gc(epoch, GcScope::Local(view))?;
                let txns = self.block_builder.on_view_changed(view, epoch);
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

    pub(crate) async fn on_network_message(
        &mut self,
        message: Message<T, Unchecked>,
    ) -> Option<ConsensusInput<T>> {
        let sender = KeyPrefix::from(&message.sender);
        let node = self.node_id;
        let _m = self
            .metrics
            .as_ref()
            .map(|m| Measurement::start(m.on_network_message.clone()));
        match message.message_type {
            MessageType::Consensus(msg) => match msg {
                ConsensusMessage::Proposal(p) => {
                    let view = p.view_number();
                    let epoch = p.proposal.data.epoch;
                    let block = p.proposal.data.block_header.block_number();
                    debug!(%node, %sender, %view, %epoch, %block, "recv proposal");
                    if self.consensus.wants_proposal_for_view(&view) {
                        self.proposal_validator.validate(p);
                    }
                    None
                },
                ConsensusMessage::VidShare(share) => {
                    let view = share.data.view_number();
                    debug!(%node, %sender, %view, "recv vid share");
                    if self.consensus.wants_proposal_for_view(&view) {
                        self.share_validator.validate(share);
                    }
                    None
                },
                ConsensusMessage::Vote1(vote1) => {
                    let view = vote1.vote.view_number();
                    crate::trace_leader_event!(
                        self.consensus.tracer,
                        view,
                        crate::leader_trace::LeaderEvent::Vote1VMinus1Arrived
                    );
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
                        self.epoch_root_collector.accumulate(vote1.clone()).await;
                    } else {
                        self.vote1_collector
                            .accumulate_vote(vote1.vote.clone())
                            .await;
                    }
                    self.vid_reconstructor
                        .handle_vid_share(vote1.vid_share, None);
                    None
                },
                ConsensusMessage::Vote2(vote2) => {
                    let view = vote2.view_number();
                    debug!(%node, %sender, %view, "recv vote2");
                    self.vote2_collector.accumulate_vote(vote2).await;
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
                        .accumulate_vote(timeout_msg.vote.clone())
                        .await;
                    self.timeout_one_honest_collector
                        .accumulate_vote(timeout_msg.vote)
                        .await;
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
                    if let Err(err) = self.network.unicast(
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
        let _m = self
            .metrics
            .as_ref()
            .map(|m| Measurement::start(m.on_state_manager_output.clone()));
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
                tracing::warn!(view = %response.view, "header creation failed");
                None
            },
        }
    }

    fn on_proposal_and_vid_share(
        &mut self,
        validated: ValidatedProposal<T>,
        vid_share: VidDisperseShare2<T>,
    ) -> Result<ConsensusInput<T>, CoordinatorError> {
        let _m = self
            .metrics
            .as_ref()
            .map(|m| Measurement::start(m.on_proposal_and_vid_share.clone()));
        self.storage.append_vid(vid_share.clone());
        self.storage
            .append_proposal(validated.message.proposal.data.clone());

        let m = validated
            .message
            .proposal
            .data
            .block_header
            .metadata()
            .clone();
        self.vid_reconstructor
            .handle_vid_share(vid_share.clone(), m);

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

    async fn on_client_request(
        &mut self,
        request: ClientRequest<T>,
    ) -> Result<(), CoordinatorError> {
        let _m = self
            .metrics
            .as_ref()
            .map(|m| Measurement::start(m.on_client_request.clone()));
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
                    tracing::info!(
                        %current_view,
                        cutover_view = *seed.cutover_view,
                        "coordinator: ignoring pre-cutover seed; already past the cutover",
                    );
                    let _ = respond.send(());
                    return Ok(());
                }
                tracing::info!(
                    undecided = seed.undecided.len(),
                    anchor_view = *seed.decided_anchor.view_number(),
                    high_qc_view = seed.high_qc.as_ref().map(|qc| *qc.view_number()),
                    cutover_view = *seed.cutover_view,
                    states = seed.validated_states.len(),
                    "coordinator: applying legacy → new-protocol seed",
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
                    tracing::error!(
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
                        tracing::warn!(
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
                self.timeout_collector.accumulate_vote(vote.clone()).await;
                self.timeout_one_honest_collector
                    .accumulate_vote(vote.clone())
                    .await;
                // Rebroadcast so peer coordinators can aggregate too.
                let message = Message {
                    sender: self.public_key.clone(),
                    message_type: MessageType::Consensus(ConsensusMessage::TimeoutVote(
                        message::TimeoutVoteMessage { vote, lock: None },
                    )),
                };
                if let Err(err) = self
                    .network
                    .broadcast(self.consensus.current_view(), &message)
                {
                    tracing::warn!(%err, "failed to rebroadcast bridged timeout vote");
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
                    tracing::info!(
                        %cutover_view,
                        "bridged late legacy high QC; proposing cutover view on it (no timeout)"
                    );
                    self.start();
                    while let Some(output) = self.outbox.pop_front() {
                        if let Err(err) = self.process_consensus_output(output) {
                            tracing::warn!(
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
                    tracing::warn!(%epoch, %err, "network on_epoch_change failed");
                }
                let _ = respond.send(());
            },
        }

        Ok(())
    }

    /// Kick the leader after the seed lands when a forwarded TC2 had
    /// already advanced `current_view`. No-op unless leader and all
    /// prerequisites are present.
    fn resume_after_cutover_tc(&mut self) {
        let cur_view = self.consensus.current_view();
        if self.consensus.timeout_cert_at(cur_view).is_none() {
            return;
        }
        let epoch = self
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
                self.block_builder.gc(view);
                self.cached_validated_proposals = self
                    .cached_validated_proposals
                    .split_off(&(view, VidCommitment2::default()));
                self.cached_vid_shares = self
                    .cached_vid_shares
                    .split_off(&(view, VidCommitment2::default()));
                // When we enter a new view, we do not want to GC enqueued messages
                // for the previous view yet:
                self.network.gc(view.saturating_sub(1).into())?;
                self.timeout_collector.gc(view, epoch);
                self.timeout_one_honest_collector.gc(view, epoch);
                self.vid_disperser.gc(view);
                self.vote1_collector.gc(view, epoch);
                self.vote2_collector.gc(view, epoch);
            },
            GcScope::Decided(view) => {
                self.epoch_manager.gc(epoch);
                self.epoch_root_collector.gc(view, epoch);
                self.pending_proposal_fetches.gc(view);
                self.state_manager.gc(view);
                self.storage.gc(view);
                self.vid_reconstructor.gc(view);
                self.da_payloads = self
                    .da_payloads
                    .split_off(&(view, VidCommitment2::default()));
            },
        }
        Ok(())
    }
}

/// Garbage collection scope.
#[derive(Debug, Clone, Copy)]
pub enum GcScope {
    /// GC is invoked on local view changes.
    Local(ViewNumber),
    /// GC is invoked on local decided views.
    Decided(ViewNumber),
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

    #[allow(dead_code)]
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
