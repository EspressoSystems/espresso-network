pub mod error;
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
    data::{EpochNumber, Leaf2, VidCommitment, VidDisperseShare2, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    message::{Proposal as SignedProposal, UpgradeLock},
    simple_certificate::{QuorumCertificate2, TimeoutCertificate2},
    simple_vote::{HasEpoch, QuorumVote2, TimeoutVote2},
    traits::{
        block_contents::BlockHeader, node_implementation::NodeType,
        signature_key::StateSignatureKey,
    },
    utils::is_epoch_root,
    vote::HasViewNumber,
};
use tokio::{select, sync::oneshot};
use tracing::{error, info, warn};

use crate::{
    block::{BlockAndHeaderRequest, BlockBuilder, BlockBuilderConfig},
    client::{ClientApi, ClientRequest, CoordinatorClient, QueryError},
    consensus::{Consensus, ConsensusInput, ConsensusOutput},
    coordinator::{
        error::{CoordinatorError, ErrorSource, Severity},
        timer::Timer,
    },
    epoch::{EpochManager, EpochRootResult},
    epoch_root_vote_collector::EpochRootVoteCollector,
    helpers::proposal_commitment,
    logging::KeyPrefix,
    message::{
        self, BlockMessage, Certificate2, CheckpointCertificate, CheckpointVote, ConsensusMessage,
        Message, MessageType, Proposal, ProposalFetchMessage, ProposalMessage, TimeoutOneHonest,
        TransactionMessage, Unchecked, Vote2,
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
    checkpoint_collector: VoteCollector<T, CheckpointVote<T>, CheckpointCertificate<T>>,
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
    cached_validated_proposals: BTreeMap<ViewNumber, ValidatedProposal<T>>,
    #[builder(default)]
    cached_vid_shares: BTreeMap<ViewNumber, VidDisperseShare2<T>>,
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
        garbage_collection_interval: u64,
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
            garbage_collection_interval,
        );

        let genesis_cert1 = initializer.high_qc.clone();
        let genesis_proposal = message::Proposal {
            block_header: initializer.anchor_leaf.block_header().clone(),
            view_number: ViewNumber::genesis(),
            epoch: EpochNumber::genesis(),
            justify_qc: genesis_cert1.clone(),
            next_epoch_justify_qc: None,
            upgrade_certificate: None,
            view_change_evidence: None,
            next_drb_result: None,
            state_cert: None,
        };
        let mut state_manager = StateManager::new(
            Arc::new(initializer.instance_state.clone()),
            upgrade_lock.clone(),
        );
        state_manager.seed_state(
            initializer.anchor_leaf.view_number(),
            initializer.anchor_state.clone(),
            initializer.anchor_leaf.clone(),
        );
        // The synthetic genesis proposal has a non-null justify_qc (the genesis
        // cert1) so the leaf derived from it has a different commitment than
        // the anchor leaf produced by `Leaf2::genesis`. `request_header` for
        // view 1 looks up the parent state by the *proposal's* leaf
        // commitment, so seed the same state under that commitment too.
        state_manager.seed_state(
            ViewNumber::genesis(),
            initializer.anchor_state.clone(),
            Leaf2::from(genesis_proposal.clone()),
        );
        consensus.seed_genesis(genesis_cert1, genesis_proposal);

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
            .checkpoint_collector(VoteCollector::new(
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
            .build()
    }

    /// Bootstrap the coordinator so the view-1 leader can propose.
    pub fn start(&mut self) {
        let view = ViewNumber::new(1);
        let epoch = EpochNumber::genesis();

        if self.consensus.last_decided_leaf().view_number() == ViewNumber::genesis() {
            // Append the genesis DA proposal to storage.
            //
            // The genesis payload is always empty, but it never flows through the
            // regular block-builder/VID path that would otherwise persist a DA
            // proposal for view 0. Storage consumers downstream still expect one,
            // so we synthesize and append it here.
            let genesis_leaf = self.consensus.last_decided_leaf().clone();
            let (payload, metadata) = T::BlockPayload::empty();
            self.storage.append_da(
                ViewNumber::genesis(),
                EpochNumber::genesis(),
                payload,
                metadata,
                genesis_leaf.payload_commitment(),
            );

            // Genesis is never decided through the normal consensus path, so
            // downstream consumers (persistence, query service) would never see
            // the genesis header. We emit a `LeafDecided` for it here so that
            // application layer sees this event
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
            .push_back(ConsensusOutput::ViewChanged(view, epoch));

        if let Some(leader) = self.leader(view, epoch)
            && leader == self.public_key
        {
            let genesis_proposal = self
                .consensus
                .proposal_at(ViewNumber::genesis())
                .expect("genesis proposal must be seeded before start()")
                .clone();
            self.outbox
                .push_back(ConsensusOutput::RequestBlockAndHeader(
                    BlockAndHeaderRequest {
                        view,
                        epoch,
                        parent_proposal: genesis_proposal,
                    },
                ));
        }
    }

    pub async fn stop(mut self) {
        self.network.shutdown().await
    }

    pub async fn next_consensus_input(&mut self) -> Result<ConsensusInput<T>, CoordinatorError> {
        loop {
            select! {
                message = self.network.receive() => match message {
                    Ok(m) => {
                        if let Some(input) = self.on_network_message(m).await {
                            return Ok(input)
                        }
                    }
                    Err(e) => {
                        return Err(CoordinatorError::from(e).context("network receive"))
                    }
                },
                () = &mut self.timer => {
                    let input = ConsensusInput::Timeout(self.timer.view(), self.timer.epoch());
                    // Timer is only reset so we can resend the timeout vote
                    // This isn't strictly necessary for the protocol, but it's a good idea to
                    // resend the timeout vote to avoid a situation where the network is stuck
                    // view because we fail to form a timeout certificate.
                    self.timer.reset();
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
                        let Some(validated) = self.cached_validated_proposals.remove(&view) else {
                            // Wait for the proposal
                            self.cached_vid_shares.insert(view, vid_share);
                            continue;
                        };
                        if !check_payload_commitment(&validated.message.proposal, &vid_share) {
                            continue;
                        }
                        return self.on_proposal_and_vid_share(validated, vid_share)
                    },
                    Err(e) => {
                        return Err(CoordinatorError::regular(e).context("vid share validation"))
                    }
                },
                Some(item) = self.proposal_validator.next() => match item {
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
                        let Some(vid_share) = self.cached_vid_shares.remove(&view) else {
                            // Wait for the vid share
                            self.cached_validated_proposals.insert(view, validated);
                            continue;
                        };
                        // Check for commitment correspondence
                        if !check_payload_commitment(&validated.message.proposal, &vid_share) {
                            continue;
                        }
                        return self.on_proposal_and_vid_share(validated, vid_share)
                    }
                    Err(e) => {
                        return Err(CoordinatorError::regular(e).context("proposal validation"))
                    }
                },
                Some(cert) = self.checkpoint_collector.next() => {
                    let Some(epoch) = cert.epoch() else {
                        let msg = format!("missing epoch in view {}", cert.view_number());
                        return Err(CoordinatorError::critical(msg).context("gc certificate"))
                    };
                    self.gc(cert.view_number(), epoch);
                }
                Some(item) = self.block_builder.next() => match item {
                    Ok(block) => {
                        self.state_manager.request_header(HeaderRequest::from(&block));
                        let next_view = block.view + 1;
                        let epoch = block.epoch;
                        let manifest = block.manifest.clone();
                        self.storage.append_da(
                            block.view,
                            block.epoch,
                            block.payload.payload.clone(),
                            block.payload.metadata.clone(),
                            block.payload_commitment,
                        );
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
                        return Ok(ConsensusInput::VidDisperseCreated(out.view, out.disperse))
                    }
                    Err(()) => {
                        return Err(CoordinatorError::unspecified().context("vid disperse"))
                    }
                },
                Some(item) = self.vid_reconstructor.next() => match item {
                    Ok(out) => {
                        self.block_builder.on_block_reconstructed(out.tx_commitments);
                        self.storage.append_da(
                            out.view,
                            out.epoch,
                            out.payload,
                            out.metadata,
                            VidCommitment::V2(out.payload_commitment),
                        );
                        return Ok(ConsensusInput::BlockReconstructed(out.view, out.payload_commitment))
                    }
                    Err(()) => {
                        return Err(CoordinatorError::unspecified().context("vid reconstruction"))
                    }
                },
                Some(result) = self.epoch_manager.next() => match result {
                    Ok(EpochRootResult::DrbResult(epoch, drb_result)) => {
                        // New epoch data available — retry votes that were
                        // buffered because their membership wasn't ready.
                        self.vote1_collector.retry_pending_votes().await;
                        self.vote2_collector.retry_pending_votes().await;
                        self.timeout_collector.retry_pending_votes().await;
                        self.timeout_one_honest_collector.retry_pending_votes().await;
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
        match output {
            ConsensusOutput::RequestState(state_request) => {
                self.state_manager.request_state(state_request);
            },
            ConsensusOutput::RequestVidDisperse {
                view,
                epoch,
                payload,
                metadata,
            } => {
                self.vid_disperser.request_vid_disperse(VidDisperseRequest {
                    view,
                    epoch,
                    block: payload,
                    metadata,
                });
            },
            ConsensusOutput::RequestDrbResult(epoch) => {
                self.epoch_manager.request_drb_result(epoch);
            },
            ConsensusOutput::SendCheckpointVote(checkpoint_vote) => {
                let message = Message {
                    sender: self.public_key.clone(),
                    message_type: MessageType::Consensus(ConsensusMessage::Checkpoint(
                        checkpoint_vote,
                    )),
                };
                self.network
                    .broadcast(message.view_number(), &message)
                    .map_err(|e| CoordinatorError::from(e).context("broadcast checkpoint vote"))?
            },
            ConsensusOutput::LeafDecided { leaves, cert2, .. } => {
                if let Some(cert2) = cert2 {
                    self.storage.append_cert2(cert2.view_number, cert2.clone());
                }
                for leaf in leaves {
                    self.epoch_manager.handle_leaf_decided(leaf);
                }
            },
            ConsensusOutput::LockUpdated(_) => {}, // TODO
            ConsensusOutput::RequestBlockAndHeader(request) => {
                self.block_builder.request_block(request);
            },
            ConsensusOutput::SendProposal(proposal) => {
                self.storage.append_proposal(proposal.data.clone());
                // TODO: This may be done async in network so we do not spend
                // too much time here in this loop.

                let message = Message {
                    sender: self.public_key.clone(),
                    message_type: MessageType::Consensus(ConsensusMessage::Proposal(
                        ProposalMessage::validated(proposal.clone()),
                    )),
                };
                if let Err(err) = self.network.broadcast(message.view_number(), &message) {
                    let err = CoordinatorError::from(err).context("proposal broadcast");
                    if err.severity == Severity::Critical {
                        return Err(err);
                    } else {
                        warn!(%err, "network error while broadcasting proposal")
                    }
                }
            },
            ConsensusOutput::SendVidShares(vid_shares) => {
                for share in vid_shares {
                    let recipient = share.data.recipient_key.clone();
                    let message = Message {
                        sender: self.public_key.clone(),
                        message_type: MessageType::Consensus(ConsensusMessage::VidShare(share)),
                    };
                    if let Err(err) =
                        self.network
                            .unicast(message.view_number(), &recipient, &message)
                    {
                        let err = CoordinatorError::from(err).context("vid share unicast");
                        if err.severity == Severity::Critical {
                            return Err(err);
                        } else {
                            warn!(%err, "network error while sending vid share")
                        }
                    }
                }
            },
            ConsensusOutput::SendTimeoutVote(vote, lock) => {
                let message = Message {
                    sender: self.public_key.clone(),
                    message_type: MessageType::Consensus(ConsensusMessage::TimeoutVote(
                        message::TimeoutVoteMessage { vote, lock },
                    )),
                };
                self.network
                    .broadcast(message.view_number(), &message)
                    .map_err(|e| CoordinatorError::from(e).context("broadcast timeout vote"))?
            },
            ConsensusOutput::SendTimeoutCertificate(tc, view, epoch) => {
                if let Some(leader) = self.leader(view, epoch) {
                    let message = Message {
                        sender: self.public_key.clone(),
                        message_type: MessageType::Consensus(ConsensusMessage::TimeoutCertificate(
                            tc,
                        )),
                    };
                    self.network
                        .unicast(message.view_number(), &leader, &message)
                        .map_err(|e| CoordinatorError::from(e).context("timeout certificate"))?;
                }
            },
            ConsensusOutput::SendVote1(vote1) => {
                let message = Message {
                    sender: self.public_key.clone(),
                    message_type: MessageType::Consensus(ConsensusMessage::Vote1(vote1)),
                };
                self.network
                    .broadcast(message.view_number(), &message)
                    .map_err(|e| CoordinatorError::from(e).context("broadcast vote1"))?
            },
            ConsensusOutput::SendVote2(vote2) => {
                let message = Message {
                    sender: self.public_key.clone(),
                    message_type: MessageType::Consensus(ConsensusMessage::Vote2(vote2)),
                };
                self.network
                    .broadcast(message.view_number(), &message)
                    .map_err(|e| CoordinatorError::from(e).context("broadcast vote2"))?
            },
            ConsensusOutput::SendEpochChange(epoch_change) => {
                let message = Message {
                    sender: self.public_key.clone(),
                    message_type: MessageType::Consensus(ConsensusMessage::EpochChange(
                        epoch_change,
                    )),
                };
                self.network
                    .broadcast(message.view_number(), &message)
                    .map_err(|e| CoordinatorError::from(e).context("broadcast epoch change"))?
            },
            ConsensusOutput::SendCertificate1(cert1) => {
                let message = Message {
                    sender: self.public_key.clone(),
                    message_type: MessageType::Consensus(ConsensusMessage::Certificate1(
                        cert1,
                        self.public_key.clone(),
                    )),
                };
                self.network
                    .broadcast(message.view_number(), &message)
                    .map_err(|e| CoordinatorError::from(e).context("broadcast certificate1"))?
            },
            ConsensusOutput::ProposalValidated { .. } => {},
            ConsensusOutput::ViewChanged(view, epoch) => {
                self.consensus.set_view(view, epoch);
                self.timer.reset_with_epoch(view, epoch);
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

                // Proactively fetch DRBs for the next epoch so
                // late-starting nodes have them before they need to
                // propose or verify certs in a new epoch. The dedup
                // in request_drb_result makes repeated calls free.
                let next_epoch = epoch + 1;
                if next_epoch > EpochNumber::genesis() + 1 {
                    self.epoch_manager.request_drb_result(next_epoch);
                    self.epoch_manager.request_drb_result(next_epoch + 1);
                }
            },
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
        match message.message_type {
            MessageType::Consensus(msg) => match msg {
                ConsensusMessage::Proposal(p) => {
                    if self.consensus.wants_proposal_for_view(&p.view_number()) {
                        self.proposal_validator.validate(p);
                    }
                    None
                },
                ConsensusMessage::VidShare(share) => {
                    if self
                        .consensus
                        .wants_proposal_for_view(&share.data.view_number())
                    {
                        self.share_validator.validate(share);
                    }
                    None
                },
                ConsensusMessage::Vote1(vote1) => {
                    let bn = vote1.vote.data.block_number.unwrap_or(0);
                    let epoch_height = *self.consensus.epoch_height;
                    if is_epoch_root(bn, epoch_height) {
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
                    self.vote2_collector.accumulate_vote(vote2).await;
                    None
                },
                ConsensusMessage::Certificate1(certificate1, _key) => {
                    Some(ConsensusInput::Certificate1(certificate1))
                },
                ConsensusMessage::Certificate2(certificate2, _key) => {
                    Some(ConsensusInput::Certificate2(certificate2))
                },
                ConsensusMessage::TimeoutVote(timeout_msg) => {
                    self.timeout_collector
                        .accumulate_vote(timeout_msg.vote.clone())
                        .await;
                    self.timeout_one_honest_collector
                        .accumulate_vote(timeout_msg.vote)
                        .await;
                    None
                },
                ConsensusMessage::TimeoutCertificate(tc) => {
                    Some(ConsensusInput::TimeoutCertificate(tc))
                },
                ConsensusMessage::EpochChange(epoch_change) => {
                    Some(ConsensusInput::EpochChange(epoch_change))
                },
                ConsensusMessage::Checkpoint(checkpoint) => {
                    self.checkpoint_collector.accumulate_vote(checkpoint).await;
                    None
                },
            },
            MessageType::Block(msg) => {
                match msg {
                    BlockMessage::Transactions(msg) => self.block_builder.on_transactions(msg),
                    BlockMessage::DedupManifest(manifest) => {
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
                if !request.validate_sender(&message.sender) {
                    warn!(
                        sender = %message.sender,
                        view = %request.view_number(),
                        "ignoring invalid proposal fetch request signature"
                    );
                    return None;
                }
                if let Some(proposal) = self
                    .consensus
                    .signed_proposal(&request.view_number())
                    .cloned()
                {
                    let response = Message {
                        sender: self.public_key.clone(),
                        message_type: MessageType::ProposalFetch(ProposalFetchMessage::Response(
                            Box::new(proposal),
                        )),
                    };

                    if let Err(err) =
                        self.network
                            .unicast(request.view_number(), &message.sender, &response)
                    {
                        let err = CoordinatorError::from(err).context("proposal response");
                        warn!(%err, "network error while sending proposal response");
                    }
                }
                None
            },
            MessageType::ProposalFetch(ProposalFetchMessage::Response(proposal)) => {
                self.pending_proposal_fetches.resolve(&proposal);
                None
            },
            MessageType::External(data) => {
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
        self.cached_vid_shares = self.cached_vid_shares.split_off(&(view + 1));
        self.cached_validated_proposals = self.cached_validated_proposals.split_off(&(view + 1));

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
            .unicast(message.view_number(), &leader, &message)
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
                        .broadcast(message.view_number(), &message)
                        .map_err(|err| {
                            CoordinatorError::from(err).context("broadcast proposal request")
                        })?;
                }
                self.pending_proposal_fetches
                    .push(view, leaf_commitment, respond);
            },
            ClientRequest::SendExternalMessage {
                view,
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
                    .unicast(view, &recipient, &message)
                    .map_err(|err| {
                        CoordinatorError::from(err)
                            .context("send external message")
                            .into()
                    });
                let _ = respond.send(result);
            },
        }

        Ok(())
    }

    fn gc(&mut self, view: ViewNumber, epoch: EpochNumber) {
        info!(node = %self.node_id, %view, "garbage collecting");
        self.consensus.gc(view, epoch);
        self.checkpoint_collector.gc(view, epoch);
        let _ = self.network.gc(view); // TODO
        self.state_manager.gc(view);
        self.vid_disperser.gc(view);
        self.vid_reconstructor.gc(view);
        self.vote1_collector.gc(view, epoch);
        self.vote2_collector.gc(view, epoch);
        self.timeout_collector.gc(view, epoch);
        self.timeout_one_honest_collector.gc(view, epoch);
        self.epoch_root_collector.gc(view, epoch);
        self.epoch_manager.gc(epoch);
        self.block_builder.gc(view);
        self.pending_proposal_fetches.gc(view);
        self.storage.gc(view);
        self.cached_validated_proposals = self.cached_validated_proposals.split_off(&view);
        self.cached_vid_shares = self.cached_vid_shares.split_off(&view);
    }
}

fn check_payload_commitment<T: NodeType>(
    proposal: &SignedProposal<T, Proposal<T>>,
    vid_share: &VidDisperseShare2<T>,
) -> bool {
    let VidCommitment::V2(commit) = proposal.data.block_header.payload_commitment() else {
        warn!(
            "unexpected payload commitment type in view {}, proposal discarded",
            proposal.data.view_number
        );
        return false;
    };
    if commit != vid_share.payload_commitment {
        warn!(
            "payload commitment mismatch in view {}, discard the proposal",
            proposal.data.view_number
        );
        return false;
    }
    true
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
