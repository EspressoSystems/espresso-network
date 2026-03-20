use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
    time::Duration,
};

use async_lock::RwLock;
use committable::{Commitment, Committable};
use futures::StreamExt;
use hotshot::{
    traits::{ValidatedState, implementations::MemoryNetwork},
    types::{BLSPrivKey, BLSPubKey, SchnorrPubKey},
};
use hotshot_example_types::{
    membership::{static_committee::StaticStakeTable, strict_membership::StrictMembership},
    node_types::{MemoryImpl, TEST_VERSIONS, TestTypes},
    storage_types::TestStorage,
};
use hotshot_testing::{
    helpers::build_cert, node_stake::TestNodeStakes, test_builder::gen_node_lists,
    view_generator::TestViewGenerator,
};
use hotshot_types::{
    data::{
        BlockNumber, EpochNumber, Leaf2, QuorumProposalWrapper, VidDisperse2, VidDisperseShare2,
        ViewNumber,
    },
    epoch_membership::EpochMembershipCoordinator,
    message::Proposal,
    simple_certificate::{TimeoutCertificate2, ViewSyncFinalizeCertificate2},
    simple_vote::{
        QuorumVote2, TimeoutData2, TimeoutVote2, ViewSyncFinalizeData2, ViewSyncFinalizeVote2,
    },
    traits::{
        block_contents::BlockHeader,
        election::Membership,
        network::TestableNetworkingImplementation,
        signature_key::{SignatureKey, StakeTableEntryType},
    },
};

use crate::{
    events::{ConsensusInput, StateResponse},
    helpers::{proposal_commitment, upgrade_lock},
    message::{Certificate1, Certificate2, ProposalMessage, Vote2, Vote2Data},
};

#[allow(dead_code)]
pub struct TestView {
    pub view_number: ViewNumber,
    pub epoch_number: EpochNumber,
    pub leader_public_key: BLSPubKey,
    pub proposal: Proposal<TestTypes, QuorumProposalWrapper<TestTypes>>,
    pub leaf: Leaf2<TestTypes>,
    pub vid_disperse: VidDisperse2<TestTypes>,
    pub vid_shares: Vec<VidDisperseShare2<TestTypes>>,
    pub cert1: Certificate1<TestTypes>,
    pub cert2: Certificate2<TestTypes>,
    pub timeout_cert: TimeoutCertificate2<TestTypes>,
    pub view_sync_cert: ViewSyncFinalizeCertificate2<TestTypes>,
}

impl TestView {
    /// Build a ProposalMessage suitable for sending as a ConsensusEvent::Proposal.
    /// `recipient_key` is the public key of the node that will receive the VID share.
    pub fn proposal_message(&self, recipient_key: &BLSPubKey) -> ProposalMessage<TestTypes> {
        let inner_proposal = Proposal {
            data: self.proposal.data.proposal.clone(),
            signature: self.proposal.signature.clone(),
            _pd: std::marker::PhantomData,
        };
        let vid_share = self
            .vid_shares
            .iter()
            .find(|s| s.recipient_key == *recipient_key)
            .expect("VID share not found for recipient key")
            .clone();
        ProposalMessage {
            proposal: inner_proposal,
            vid_share,
        }
    }

    /// Get the VidCommitment2 for this view (for BlockReconstructed events).
    pub fn vid_commitment(&self) -> hotshot_types::data::VidCommitment2 {
        self.vid_disperse.payload_commitment
    }

    /// Build a ConsensusEvent::Proposal for a given recipient node.
    pub fn proposal_event(&self, recipient_key: &BLSPubKey) -> ConsensusInput<TestTypes> {
        ConsensusInput::Proposal(self.proposal_message(recipient_key))
    }

    /// Build a ConsensusEvent::BlockReconstructed for this view.
    pub fn block_reconstructed_event(&self) -> ConsensusInput<TestTypes> {
        ConsensusInput::BlockReconstructed(self.view_number, self.vid_commitment())
    }

    /// Build a ConsensusInput::StateVerified for this view.
    pub fn state_verified_event(&self) -> ConsensusInput<TestTypes> {
        let commitment = proposal_commitment(&self.proposal.data.proposal);
        let state = <hotshot_example_types::state_types::TestValidatedState as ValidatedState<
            TestTypes,
        >>::from_header(&self.proposal.data.proposal.block_header);
        ConsensusInput::StateVerified(StateResponse {
            view: self.view_number,
            commitment,
            state: Arc::new(state),
        })
    }

    /// Build a ConsensusEvent::Certificate1 for this view.
    pub fn cert1_event(&self) -> ConsensusInput<TestTypes> {
        ConsensusInput::Certificate1(self.cert1.clone())
    }

    /// Build a ConsensusEvent::Certificate2 for this view.
    pub fn cert2_event(&self) -> ConsensusInput<TestTypes> {
        ConsensusInput::Certificate2(self.cert2.clone())
    }

    /// Build an Event for a timeout certificate.
    pub fn timeout_cert_event(&self) -> ConsensusInput<TestTypes> {
        ConsensusInput::TimeoutCertificate(self.timeout_cert.clone())
    }

    /// Build an Event for a view sync certificate.
    pub fn view_sync_event(&self) -> ConsensusInput<TestTypes> {
        ConsensusInput::ViewSyncCertificate(self.view_sync_cert.clone())
    }
}

#[allow(dead_code)]
pub struct TestData {
    pub views: Vec<TestView>,
}

impl TestData {
    pub async fn new(num_views: usize) -> Self {
        let membership = mock_membership().await;
        let keys = key_map();
        let node_key_map = Arc::new(keys.clone());
        let upgrade = TEST_VERSIONS.vid2;

        let mut generator =
            TestViewGenerator::generate(membership.clone(), node_key_map.clone(), upgrade);

        let gen_views: Vec<_> = (&mut generator).take(num_views).collect::<Vec<_>>().await;

        let mut views = Vec::new();
        for gen_view in &gen_views {
            let view_number = gen_view.view_number;
            let epoch = gen_view.epoch_number.unwrap_or(EpochNumber::genesis());
            let epoch_membership = membership.membership_for_epoch(Some(epoch)).await.unwrap();

            let proposal = gen_view.quorum_proposal.clone();
            let leaf = gen_view.leaf.clone();
            let leader_public_key = gen_view.leader_public_key;
            let leader_private_key = keys
                .get(&leader_public_key)
                .expect("Leader key not found in key map");

            let (vid_disperse, vid_shares) = extract_vid_disperse(gen_view);
            let leaf_commit = leaf.commit();
            let block_number =
                BlockHeader::<TestTypes>::block_number(&proposal.data.proposal.block_header);

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
                block_number.into(),
                &epoch_membership,
                view_number,
                &leader_public_key,
                leader_private_key,
            )
            .await;
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
                proposal,
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
    let network =
        <MemoryNetwork<BLSPubKey> as TestableNetworkingImplementation<TestTypes>>::generator(
            10,
            0,
            1,
            10,
            None,
            Duration::from_secs(1),
            &mut HashMap::new(),
        )(0)
        .await;
    let members = gen_node_lists(10, 10, &TestNodeStakes::default()).0;
    let membership = Arc::new(RwLock::new(StrictMembership::<
        TestTypes,
        StaticStakeTable<BLSPubKey, SchnorrPubKey>,
    >::new::<MemoryImpl>(
        members.clone(),
        members.clone(),
        TestStorage::default(),
        network,
        members[0].stake_table_entry.public_key(),
        10,
    )));
    // Initialize epoch data so membership works with epoch-aware versions (VID2 etc.)
    membership
        .write()
        .await
        .set_first_epoch(EpochNumber::genesis(), [0u8; 32]);
    EpochMembershipCoordinator::new(membership, 10, &TestStorage::default())
}

pub fn key_map() -> BTreeMap<BLSPubKey, BLSPrivKey> {
    let mut map = BTreeMap::new();
    for i in 0..10 {
        let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([0u8; 32], i);
        map.insert(public_key, private_key);
    }
    map
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

async fn build_cert1(
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

async fn build_cert2(
    leaf_commit: Commitment<Leaf2<TestTypes>>,
    epoch: EpochNumber,
    block: BlockNumber,
    epoch_membership: &hotshot_types::epoch_membership::EpochMembership<TestTypes>,
    view_number: ViewNumber,
    public_key: &BLSPubKey,
    private_key: &BLSPrivKey,
) -> Certificate2<TestTypes> {
    let data = Vote2Data {
        leaf_commit,
        epoch,
        block,
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
