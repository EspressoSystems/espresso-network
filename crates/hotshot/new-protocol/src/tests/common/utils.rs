use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
    time::Duration,
};

use async_lock::RwLock;
use committable::{Commitment, Committable};
use futures::StreamExt;
use hotshot::{
    traits::{BlockPayload, ValidatedState, implementations::MemoryNetwork},
    types::{BLSPrivKey, BLSPubKey, SchnorrPubKey},
};
use hotshot_example_types::{
    block_types::{TestBlockHeader, TestBlockPayload, TestMetadata},
    membership::{static_committee::StaticStakeTable, strict_membership::StrictMembership},
    node_types::{MemoryImpl, TEST_VERSIONS, TestTypes},
    state_types::{TestInstanceState, TestValidatedState},
    storage_types::TestStorage,
};
use hotshot_testing::{
    helpers::build_cert, node_stake::TestNodeStakes, test_builder::gen_node_lists,
    view_generator::TestViewGenerator,
};
use hotshot_types::{
    data::{
        EpochNumber, Leaf2, VidCommitment, VidDisperse, VidDisperse2, VidDisperseShare2,
        ViewNumber, vid_commitment,
    },
    epoch_membership::EpochMembershipCoordinator,
    message::Proposal as SignedProposal,
    simple_certificate::{TimeoutCertificate2, ViewSyncFinalizeCertificate2},
    simple_vote::{
        QuorumVote2, TimeoutData2, TimeoutVote2, ViewSyncFinalizeData2, ViewSyncFinalizeVote2,
        Vote2Data,
    },
    traits::{
        EncodeBytes,
        block_contents::{BlockHeader, BuilderFee},
        election::Membership,
        network::TestableNetworkingImplementation,
        node_implementation::NodeImplementation,
        signature_key::SignatureKey,
    },
    utils::{
        BuilderCommitment, epoch_from_block_number, is_epoch_root, is_epoch_transition,
        is_last_block,
    },
};

use crate::{
    consensus::{Consensus, ConsensusInput, ConsensusOutput},
    helpers::{proposal_commitment, upgrade_lock},
    message::{
        Certificate1, Certificate2, ConsensusMessage, Message, MessageType, Proposal,
        ProposalMessage, TimeoutVoteMessage, Validated, Vote1, Vote2,
    },
    outbox::Outbox,
    state::StateResponse,
};

/// DRB result used by `TestData` for epoch transition proposals.
pub const TEST_DRB_RESULT: hotshot_types::drb::DrbResult = [0u8; 32];

#[allow(dead_code)]
pub struct TestView {
    pub view_number: ViewNumber,
    pub epoch_number: EpochNumber,
    pub leader_public_key: BLSPubKey,
    pub proposal: SignedProposal<TestTypes, Proposal<TestTypes>>,
    pub leaf: Leaf2<TestTypes>,
    pub vid_disperse: VidDisperse2<TestTypes>,
    pub vid_shares: Vec<VidDisperseShare2<TestTypes>>,
    pub cert1: Certificate1<TestTypes>,
    pub cert2: Certificate2<TestTypes>,
    pub timeout_cert: TimeoutCertificate2<TestTypes>,
    pub view_sync_cert: ViewSyncFinalizeCertificate2<TestTypes>,
}

impl TestView {
    /// Build a ProposalMessage suitable for sending as a CoordinatorEvent::Proposal.
    /// `recipient_key` is the public key of the node that will receive the VID share.
    pub fn proposal_message(
        &self,
        recipient_key: &BLSPubKey,
    ) -> ProposalMessage<TestTypes, Validated> {
        let inner_proposal = SignedProposal {
            data: self.proposal.data.clone(),
            signature: self.proposal.signature.clone(),
            _pd: std::marker::PhantomData,
        };
        let vid_share = self
            .vid_shares
            .iter()
            .find(|s| s.recipient_key == *recipient_key)
            .expect("VID share not found for recipient key")
            .clone();
        ProposalMessage::validated(inner_proposal, vid_share)
    }

    /// Get the VidCommitment2 for this view (for BlockReconstructed events).
    pub fn vid_commitment(&self) -> hotshot_types::data::VidCommitment2 {
        self.vid_disperse.payload_commitment
    }

    /// Build an Event for a proposal.
    pub fn proposal_input(&self, recipient_key: &BLSPubKey) -> Message<TestTypes, Validated> {
        Message {
            sender: self.leader_public_key,
            message_type: MessageType::Consensus(ConsensusMessage::Proposal(
                self.proposal_message(recipient_key),
            )),
        }
    }
    pub fn proposal_input_consensus(&self, recipient_key: &BLSPubKey) -> ConsensusInput<TestTypes> {
        ConsensusInput::Proposal(self.leader_public_key, self.proposal_message(recipient_key))
    }

    /// Build an Event for block reconstructed.
    pub fn block_reconstructed_input(&self) -> ConsensusInput<TestTypes> {
        ConsensusInput::BlockReconstructed(self.view_number, self.vid_commitment())
    }

    /// Build an Event for Certificate1.
    pub fn cert1_input(&self) -> ConsensusInput<TestTypes> {
        ConsensusInput::Certificate1(self.cert1.clone())
    }

    /// Build an Event for Certificate2.
    pub fn cert2_input(&self) -> ConsensusInput<TestTypes> {
        ConsensusInput::Certificate2(self.cert2.clone())
    }

    /// Build a Vote1 Event from a specific validator, carrying that validator's
    /// QuorumVote2 and VID share.
    pub fn vote1_input(&self, node_index: u64) -> Message<TestTypes, Validated> {
        let (pub_key, priv_key) = BLSPubKey::generated_from_seed_indexed([0u8; 32], node_index);
        let data = hotshot_types::simple_vote::QuorumData2 {
            leaf_commit: proposal_commitment(&self.proposal.data.clone()),
            epoch: Some(self.epoch_number),
            block_number: Some(BlockHeader::<TestTypes>::block_number(
                &self.proposal.data.block_header,
            )),
        };
        let vote = hotshot_types::simple_vote::SimpleVote::create_signed_vote(
            data,
            self.view_number,
            &pub_key,
            &priv_key,
            &upgrade_lock(),
        )
        .expect("Failed to sign QuorumVote2");
        let vid_share = self
            .vid_shares
            .iter()
            .find(|s| s.recipient_key == pub_key)
            .expect("VID share not found for node")
            .clone();
        Message {
            sender: self.leader_public_key,
            message_type: MessageType::Consensus(ConsensusMessage::Vote1(Vote1 {
                vote,
                vid_share,
            })),
        }
    }

    /// Build a Vote2 Event from a specific validator.
    pub fn vote2_input(&self, node_index: u64) -> Message<TestTypes, Validated> {
        let (pub_key, priv_key) = BLSPubKey::generated_from_seed_indexed([0u8; 32], node_index);
        let data = Vote2Data {
            leaf_commit: proposal_commitment(&self.proposal.data.clone()),
            epoch: self.epoch_number,
            block_number: BlockHeader::<TestTypes>::block_number(&self.proposal.data.block_header),
        };
        let vote = hotshot_types::simple_vote::SimpleVote::create_signed_vote(
            data,
            self.view_number,
            &pub_key,
            &priv_key,
            &upgrade_lock(),
        )
        .expect("Failed to sign Vote2");
        Message {
            sender: self.leader_public_key,
            message_type: MessageType::Consensus(ConsensusMessage::Vote2(vote)),
        }
    }

    /// Build a TimeoutVote Event from a specific validator.
    pub fn timeout_vote_input(
        &self,
        node_index: u64,
        lock: Option<Certificate1<TestTypes>>,
    ) -> Message<TestTypes, Validated> {
        let (pub_key, priv_key) = BLSPubKey::generated_from_seed_indexed([0u8; 32], node_index);
        let data = TimeoutData2 {
            view: self.view_number,
            epoch: Some(self.epoch_number),
        };
        let vote = hotshot_types::simple_vote::SimpleVote::create_signed_vote(
            data,
            self.view_number,
            &pub_key,
            &priv_key,
            &upgrade_lock(),
        )
        .expect("Failed to sign TimeoutVote2");
        Message {
            sender: self.leader_public_key,
            message_type: MessageType::Consensus(ConsensusMessage::TimeoutVote(
                TimeoutVoteMessage { vote, lock },
            )),
        }
    }

    /// Build an Event for a timeout certificate.
    #[allow(dead_code)]
    pub fn timeout_cert_input(&self) -> ConsensusInput<TestTypes> {
        ConsensusInput::TimeoutCertificate(self.timeout_cert.clone())
    }
}

pub struct TestData {
    pub views: Vec<TestView>,
}

impl TestData {
    pub async fn new(num_views: usize) -> Self {
        Self::new_with_epoch_height(num_views, 0).await
    }

    /// Create test data with epoch-aware proposals. When `epoch_height > 0`,
    /// epoch transition views will have `next_drb_result` set to
    /// [`TEST_DRB_RESULT`] and all downstream commitments (leaf, cert1, cert2,
    /// justify_qc chain) are kept consistent.
    pub async fn new_with_epoch_height(num_views: usize, epoch_height: u64) -> Self {
        Self::new_with_epoch_height_and_num_nodes(num_views, epoch_height, 10).await
    }

    pub async fn new_with_epoch_height_and_num_nodes(
        num_views: usize,
        epoch_height: u64,
        num_nodes: usize,
    ) -> Self {
        crate::logging::init_test_logging();
        let (membership, _storage) = mock_membership_with_num_nodes(num_nodes, epoch_height).await;
        let keys = key_map_with_num_nodes(num_nodes as u64);
        let node_key_map = Arc::new(keys.clone());
        let upgrade = TEST_VERSIONS.vid2;

        let mut generator =
            TestViewGenerator::generate(membership.clone(), node_key_map.clone(), upgrade);

        let gen_views: Vec<_> = (&mut generator).take(num_views).collect::<Vec<_>>().await;

        let mut views = Vec::new();
        // When we patch a view's proposal (DRB or justify_qc update) the leaf
        // commitment changes. The *next* view's justify_qc must reference the
        // new cert1, so we propagate it forward through this variable.
        let mut prev_new_cert1: Option<Certificate1<TestTypes>> = None;
        let mut prev_new_cert2: Option<Certificate2<TestTypes>> = None;
        // DRB results computed from epoch root leaves, keyed by target epoch.
        // With difficulty 0 the DRB equals SHA256(bincode(root_leaf.justify_qc.signatures)).
        let mut computed_drbs: HashMap<u64, hotshot_types::drb::DrbResult> = HashMap::new();

        for gen_view in &gen_views {
            let view_number = gen_view.view_number;

            let mut proposal: Proposal<TestTypes> = gen_view.quorum_proposal.data.clone().into();
            let leader_public_key = gen_view.leader_public_key;
            let leader_private_key = keys
                .get(&leader_public_key)
                .expect("Leader key not found in key map");

            let (vid_disperse, vid_shares) = extract_vid_disperse(gen_view);
            let block_number = BlockHeader::<TestTypes>::block_number(&proposal.block_header);

            // Compute epoch from block number (generator doesn't know about
            // epoch boundaries, so all views get genesis epoch).
            let epoch = if epoch_height > 0 && block_number > 0 {
                EpochNumber::new(epoch_from_block_number(block_number, epoch_height))
            } else {
                gen_view.epoch_number.unwrap_or(EpochNumber::genesis())
            };
            // Use genesis membership for signing — same committee in tests.
            let epoch_membership = membership
                .membership_for_epoch(Some(EpochNumber::genesis()))
                .await
                .unwrap();

            // ---- epoch-aware patching ----
            let needs_justify_update = prev_new_cert1.is_some();
            // Match the guard in `consensus.rs` maybe_propose / handle_proposal:
            // transitions in epoch >= 2 (`> genesis`) must carry
            // `next_drb_result`.
            let needs_drb = epoch > EpochNumber::genesis()
                && epoch_height > 0
                && is_epoch_transition(block_number, epoch_height);

            if let Some(new_cert1) = prev_new_cert1.take() {
                proposal.justify_qc = new_cert1;
            }
            if needs_drb {
                let next_epoch = *epoch + 1;
                proposal.next_drb_result = computed_drbs.get(&next_epoch).copied();
            }
            // Always set epoch (may differ from generator output).
            let gen_epoch = gen_view.epoch_number.unwrap_or(EpochNumber::genesis());
            let epoch_patched = epoch != gen_epoch;
            proposal.epoch = epoch;

            let needs_new_epoch = is_last_block(block_number.saturating_sub(1), epoch_height);
            if needs_new_epoch {
                proposal.next_epoch_justify_qc = prev_new_cert2.clone();
            }

            // Recompute leaf and commitment (may differ from generator output
            // when we touched justify_qc or next_drb_result).
            let leaf = Leaf2::from(proposal.clone());
            let leaf_commit = leaf.commit();

            // Compute DRB for epoch root blocks so transition-window
            // proposals carry the correct next_drb_result.  We call
            // add_epoch_root + compute_drb_result on the *generator's own*
            // membership (not the harness's), mirroring what the
            // EpochManager does in production.
            if epoch_height > 0 && is_epoch_root(block_number, epoch_height) {
                let target_epoch =
                    EpochNumber::new(epoch_from_block_number(block_number, epoch_height) + 2);
                let _ = <TestTypes as hotshot_types::traits::node_implementation::NodeType>
                    ::Membership::add_epoch_root(
                        membership.membership().clone(),
                        proposal.block_header.clone(),
                    )
                    .await;
                if let Ok(drb) = membership
                    .compute_drb_result(target_epoch, leaf.clone())
                    .await
                {
                    computed_drbs.insert(*target_epoch, drb);
                }
            }

            // Re-sign the proposal when the leaf commitment changed due to patching.

            let signature =
                <BLSPubKey as SignatureKey>::sign(leader_private_key, leaf_commit.as_ref())
                    .expect("Failed to sign patched leaf commitment");
            let signed_proposal = SignedProposal {
                data: proposal.clone(),
                signature,
                _pd: std::marker::PhantomData,
            };

            let cert1 = build_cert1(
                leaf_commit,
                epoch,
                block_number,
                &epoch_membership,
                view_number,
                &leader_public_key,
                leader_private_key,
            )
            .await;
            let cert2 = build_cert2(
                leaf_commit,
                epoch,
                block_number,
                &epoch_membership,
                view_number,
                &leader_public_key,
                leader_private_key,
            )
            .await;

            // Propagate the rebuilt cert1 so the next view's justify_qc is
            // consistent with our updated commitment.
            if needs_drb || needs_justify_update || needs_new_epoch || epoch_patched {
                prev_new_cert1 = Some(cert1.clone());
            }
            // Set prev_new_cert2 on the last block of each epoch so the
            // first block of the next epoch can use it as
            // next_epoch_justify_qc.
            if is_last_block(block_number, epoch_height) {
                prev_new_cert2 = Some(cert2.clone());
            }

            let timeout_cert = build_timeout_cert(
                view_number,
                epoch,
                &epoch_membership,
                &leader_public_key,
                leader_private_key,
            )
            .await;
            let view_sync_cert = build_view_sync_cert(
                view_number,
                epoch,
                &epoch_membership,
                &leader_public_key,
                leader_private_key,
            )
            .await;

            views.push(TestView {
                view_number,
                epoch_number: epoch,
                leader_public_key,
                proposal: signed_proposal,
                leaf,
                vid_disperse,
                vid_shares,
                cert1,
                cert2,
                timeout_cert,
                view_sync_cert,
            });
        }
        Self { views }
    }
}

pub async fn mock_membership() -> EpochMembershipCoordinator<TestTypes> {
    mock_membership_with_num_nodes(10, 10).await.0
}

pub async fn mock_membership_with_num_nodes(
    num_nodes: usize,
    epoch_height: u64,
) -> (
    EpochMembershipCoordinator<TestTypes>,
    TestStorage<TestTypes>,
) {
    // Unit-test callers don't have a real Coordinator network to share,
    // so spin up an isolated MemoryNetwork just for the `Leaf2Fetcher`.
    // These tests don't actually exercise peer catchup.
    let network =
        <MemoryNetwork<BLSPubKey> as TestableNetworkingImplementation<TestTypes>>::generator(
            num_nodes,
            0,
            1,
            num_nodes,
            None,
            Duration::from_secs(1),
            &mut HashMap::new(),
        )(0)
        .await;
    let (pk, _) = BLSPubKey::generated_from_seed_indexed([0u8; 32], 0);
    let (mut coordinator, storage) =
        mock_membership_with_network::<MemoryImpl>(num_nodes, epoch_height, network, pk).await;
    // Install a dummy external channel so Leaf2Fetcher::fetch_leaf
    // doesn't panic on `self.network_receiver.expect(...)` when these
    // unit tests incidentally drive catchup.
    let (_tx, rx) = async_broadcast::broadcast(1);
    coordinator.set_external_channel(rx).await;
    (coordinator, storage)
}

/// Create a mock membership coordinator for `num_nodes` validators.
///
/// The `network` is shared with the node's [`Coordinator`] — the
/// membership's `Leaf2Fetcher` uses it to SEND catchup direct-messages,
/// while the Coordinator is the sole owner of the receive loop.
/// Returns the coordinator along with the [`TestStorage`] its internal
/// `StrictMembership` uses.  The same `TestStorage` should be supplied to
/// the node's [`Coordinator`] so that `Leaf2Fetcher` can read leaves the
/// Coordinator writes during catchup.
pub async fn mock_membership_with_network<I>(
    num_nodes: usize,
    epoch_height: u64,
    network: Arc<<I as NodeImplementation<TestTypes>>::Network>,
    public_key: BLSPubKey,
) -> (
    EpochMembershipCoordinator<TestTypes>,
    TestStorage<TestTypes>,
)
where
    I: NodeImplementation<TestTypes>,
{
    let members = gen_node_lists(
        num_nodes as u64,
        num_nodes as u64,
        &TestNodeStakes::default(),
    )
    .0;
    let storage = TestStorage::<TestTypes>::default();
    let membership = Arc::new(RwLock::new(StrictMembership::<
        TestTypes,
        StaticStakeTable<BLSPubKey, SchnorrPubKey>,
    >::new::<I>(
        members.clone(),
        members.clone(),
        storage.clone(),
        network,
        public_key,
        epoch_height,
    )));
    // Initialize epoch data so membership works with epoch-aware versions (VID2 etc.)
    membership
        .write()
        .await
        .set_first_epoch(EpochNumber::genesis(), [0u8; 32]);

    let coordinator =
        EpochMembershipCoordinator::new(membership, num_nodes as u64, &TestStorage::default());
    // Set the DRB difficulty selector so compute_drb_result can run.
    // Difficulty 0 makes the computation instant for tests.
    coordinator
        .set_drb_difficulty_selector(std::sync::Arc::new(|_version| Box::pin(async { 0u64 })))
        .await;
    // Callers that need the `Leaf2Fetcher` external channel wired up
    // install it themselves — `build_test_coordinator` wires a real
    // channel to its Coordinator; `mock_membership` installs a dummy
    // for unit tests that don't exercise real catchup.
    (coordinator, storage)
}

pub fn key_map_with_num_nodes(num_nodes: u64) -> BTreeMap<BLSPubKey, BLSPrivKey> {
    let mut map = BTreeMap::new();
    for i in 0..num_nodes {
        let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([0u8; 32], i);
        map.insert(public_key, private_key);
    }
    map
}

/// A mock block with its derived commitments and metadata.
pub struct MockBlock {
    pub block: TestBlockPayload,
    pub metadata: TestMetadata,
    pub payload_commitment: VidCommitment,
    pub builder_commitment: BuilderCommitment,
}

impl MockBlock {
    pub fn new() -> Self {
        let block = TestBlockPayload::genesis();
        let metadata = TestMetadata {
            num_transactions: 0,
        };
        let payload_commitment = vid_commitment(
            &block.encode(),
            &metadata.encode(),
            10,
            TEST_VERSIONS.test.base,
        );
        let builder_commitment =
            <TestBlockPayload as BlockPayload<TestTypes>>::builder_commitment(&block, &metadata);
        Self {
            block,
            metadata,
            payload_commitment,
            builder_commitment,
        }
    }
}

#[allow(dead_code)]
pub fn mock_builder_fee() -> BuilderFee<TestTypes> {
    use hotshot_types::traits::signature_key::BuilderSignatureKey;
    let (builder_key, builder_private_key) =
        <hotshot_types::signature_key::BuilderKey as BuilderSignatureKey>::generated_from_seed_indexed([0; 32], 0);
    let builder_signature =
        <hotshot_types::signature_key::BuilderKey as BuilderSignatureKey>::sign_builder_message(
            &builder_private_key,
            &[0u8],
        )
        .unwrap();
    BuilderFee {
        fee_amount: 0,
        fee_account: builder_key,
        fee_signature: builder_signature,
    }
}

pub fn state_verified_input(
    proposal: &Proposal<TestTypes>,
    view: ViewNumber,
) -> ConsensusInput<TestTypes> {
    let commitment = proposal_commitment(proposal);
    let state =
        <TestValidatedState as ValidatedState<TestTypes>>::from_header(&proposal.block_header);
    ConsensusInput::StateValidated(StateResponse {
        view,
        commitment,
        state: Arc::new(state),
        delta: None,
    })
}

fn extract_vid_shares(disperse: &VidDisperse2<TestTypes>) -> Vec<VidDisperseShare2<TestTypes>> {
    disperse
        .shares
        .iter()
        .map(|(key, share)| VidDisperseShare2 {
            view_number: disperse.view_number,
            epoch: disperse.epoch,
            target_epoch: disperse.target_epoch,
            payload_commitment: disperse.payload_commitment,
            share: share.clone(),
            recipient_key: *key,
            common: disperse.common.clone(),
        })
        .collect()
}

fn extract_vid_disperse(
    gen_view: &hotshot_testing::view_generator::TestView,
) -> (VidDisperse2<TestTypes>, Vec<VidDisperseShare2<TestTypes>>) {
    let hotshot_types::data::VidDisperse::V2(vid_disperse) = gen_view.vid_disperse.data.clone()
    else {
        panic!("Expected V2 VID disperse");
    };
    let vid_shares = extract_vid_shares(&vid_disperse);
    (vid_disperse, vid_shares)
}

/// Lightweight consensus-only test harness. Wraps a single [`Consensus`]
/// instance and auto-responds to outputs that consensus expects feedback for
/// (`RequestState`, `RequestBlockAndHeader`, `RequestVidDisperse`,
/// `RequestDrbResult`).
pub(crate) struct ConsensusHarness {
    pub consensus: Consensus<TestTypes>,
    pub membership_coordinator: EpochMembershipCoordinator<TestTypes>,
    pub collected: Outbox<ConsensusOutput<TestTypes>>,
}

impl ConsensusHarness {
    pub async fn new(node_index: u64) -> Self {
        Self::new_with_epoch_height(node_index, 10).await
    }

    pub async fn new_with_epoch_height(node_index: u64, epoch_height: u64) -> Self {
        let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], node_index);
        let membership = mock_membership().await;
        let instance = Arc::new(TestInstanceState::default());
        let genesis_leaf = Leaf2::<TestTypes>::genesis(
            &TestValidatedState::default(),
            &instance,
            TEST_VERSIONS.test.base,
        )
        .await;
        let consensus = Consensus::new(
            membership.clone(),
            public_key,
            private_key,
            genesis_leaf,
            epoch_height,
        );
        Self {
            consensus,
            membership_coordinator: membership,
            collected: Outbox::new(),
        }
    }

    /// Apply a [`ConsensusInput`] and drain outputs, auto-responding to
    /// actions that consensus expects feedback for.
    pub async fn apply(&mut self, input: ConsensusInput<TestTypes>) {
        let mut outbox = Outbox::new();
        self.consensus.apply(input, &mut outbox).await;
        self.drain_outbox(&mut outbox).await;
    }

    async fn drain_outbox(&mut self, outbox: &mut Outbox<ConsensusOutput<TestTypes>>) {
        while let Some(output) = outbox.pop_front() {
            self.handle_output(&output, outbox).await;
            self.collected.push_back(output);
        }
    }

    async fn handle_output(
        &mut self,
        output: &ConsensusOutput<TestTypes>,
        outbox: &mut Outbox<ConsensusOutput<TestTypes>>,
    ) {
        match output {
            ConsensusOutput::RequestState(req) => {
                let input = state_verified_input(&req.proposal, req.view);
                self.consensus.apply(input, outbox).await;
            },
            ConsensusOutput::RequestBlockAndHeader(req) => {
                let mock_block = MockBlock::new();
                let parent_leaf = req.parent_proposal.clone().into();
                let header = TestBlockHeader::new(
                    &parent_leaf,
                    mock_block.payload_commitment,
                    mock_block.builder_commitment,
                    mock_block.metadata,
                    TEST_VERSIONS.test.base,
                );
                self.consensus
                    .apply(ConsensusInput::HeaderCreated(req.view, header), outbox)
                    .await;
                self.consensus
                    .apply(
                        ConsensusInput::BlockBuilt {
                            view: req.view,
                            epoch: req.epoch,
                            payload: mock_block.block,
                            metadata: mock_block.metadata,
                        },
                        outbox,
                    )
                    .await;
            },
            ConsensusOutput::RequestVidDisperse {
                view,
                epoch,
                payload,
                metadata,
            } => {
                let vid_disperse = VidDisperse::calculate_vid_disperse(
                    payload,
                    &self.membership_coordinator,
                    *view,
                    Some(*epoch),
                    Some(*epoch),
                    metadata,
                    &upgrade_lock(),
                )
                .await
                .unwrap();
                let VidDisperse::V2(vid) = vid_disperse.disperse else {
                    panic!("VidDisperse is not a V2");
                };
                self.consensus
                    .apply(ConsensusInput::VidDisperseCreated(*view, vid), outbox)
                    .await;
            },
            ConsensusOutput::RequestDrbResult(epoch) => {
                self.consensus
                    .apply(ConsensusInput::DrbResult(*epoch, TEST_DRB_RESULT), outbox)
                    .await;
            },
            ConsensusOutput::LeafDecided { leaves, .. } => {
                // Mirror the EpochManager: when an epoch-root block is decided,
                // register the future epoch in the membership (add_epoch_root)
                // and store a DRB result for it (compute_drb_result).
                let epoch_height = self.consensus.epoch_height;
                for leaf in leaves {
                    let block_number = <TestBlockHeader as BlockHeader<TestTypes>>::block_number(
                        leaf.block_header(),
                    );
                    if !is_epoch_root(block_number, *epoch_height) {
                        continue;
                    }
                    let header = leaf.block_header().clone();
                    <TestTypes as hotshot_types::traits::node_implementation::NodeType>::Membership::add_epoch_root(
                        self.membership_coordinator.membership().clone(),
                        header,
                    )
                    .await
                    .expect("add_epoch_root should succeed in test harness");

                    let epoch =
                        hotshot_types::utils::epoch_from_block_number(block_number, *epoch_height);
                    let target_epoch = EpochNumber::new(epoch + 2);
                    self.membership_coordinator
                        .membership()
                        .write()
                        .await
                        .add_drb_result(target_epoch, TEST_DRB_RESULT);
                }
            },
            _ => {},
        }
    }

    pub fn outputs(&self) -> &Outbox<ConsensusOutput<TestTypes>> {
        &self.collected
    }
}

pub(crate) async fn build_cert1(
    leaf_commit: Commitment<Leaf2<TestTypes>>,
    epoch: EpochNumber,
    block_number: u64,
    epoch_membership: &hotshot_types::epoch_membership::EpochMembership<TestTypes>,
    view_number: ViewNumber,
    public_key: &BLSPubKey,
    private_key: &BLSPrivKey,
) -> Certificate1<TestTypes> {
    let data = hotshot_types::simple_vote::QuorumData2 {
        leaf_commit,
        epoch: Some(epoch),
        block_number: Some(block_number),
    };
    build_cert::<
        TestTypes,
        hotshot_types::simple_vote::QuorumData2<TestTypes>,
        QuorumVote2<TestTypes>,
        Certificate1<TestTypes>,
    >(
        data,
        epoch_membership,
        view_number,
        public_key,
        private_key,
        &upgrade_lock::<TestTypes>(),
    )
    .await
}

pub(crate) async fn build_cert2(
    leaf_commit: Commitment<Leaf2<TestTypes>>,
    epoch: EpochNumber,
    block_number: u64,
    epoch_membership: &hotshot_types::epoch_membership::EpochMembership<TestTypes>,
    view_number: ViewNumber,
    public_key: &BLSPubKey,
    private_key: &BLSPrivKey,
) -> Certificate2<TestTypes> {
    let data = Vote2Data {
        leaf_commit,
        epoch,
        block_number,
    };
    build_cert::<TestTypes, Vote2Data<TestTypes>, Vote2<TestTypes>, Certificate2<TestTypes>>(
        data,
        epoch_membership,
        view_number,
        public_key,
        private_key,
        &upgrade_lock::<TestTypes>(),
    )
    .await
}

async fn build_timeout_cert(
    view_number: ViewNumber,
    epoch: EpochNumber,
    epoch_membership: &hotshot_types::epoch_membership::EpochMembership<TestTypes>,
    public_key: &BLSPubKey,
    private_key: &BLSPrivKey,
) -> TimeoutCertificate2<TestTypes> {
    let data = TimeoutData2 {
        view: view_number,
        epoch: Some(epoch),
    };
    build_cert::<TestTypes, TimeoutData2, TimeoutVote2<TestTypes>, TimeoutCertificate2<TestTypes>>(
        data,
        epoch_membership,
        view_number,
        public_key,
        private_key,
        &upgrade_lock::<TestTypes>(),
    )
    .await
}

async fn build_view_sync_cert(
    view_number: ViewNumber,
    epoch: EpochNumber,
    epoch_membership: &hotshot_types::epoch_membership::EpochMembership<TestTypes>,
    public_key: &BLSPubKey,
    private_key: &BLSPrivKey,
) -> ViewSyncFinalizeCertificate2<TestTypes> {
    let data = ViewSyncFinalizeData2 {
        relay: 0,
        round: view_number,
        epoch: Some(epoch),
    };
    build_cert::<
        TestTypes,
        ViewSyncFinalizeData2,
        ViewSyncFinalizeVote2<TestTypes>,
        ViewSyncFinalizeCertificate2<TestTypes>,
    >(
        data,
        epoch_membership,
        view_number,
        public_key,
        private_key,
        &upgrade_lock::<TestTypes>(),
    )
    .await
}
