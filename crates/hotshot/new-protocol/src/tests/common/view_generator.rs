use std::{marker::PhantomData, sync::Arc};

use committable::Committable;
use hotshot_example_types::{
    block_types::{TestBlockHeader, TestBlockPayload, TestMetadata, TestTransaction},
    node_types::TestTypes,
    state_types::{TestInstanceState, TestValidatedState},
};
use hotshot_types::{
    data::{
        EpochNumber, Leaf2, QuorumProposal2, QuorumProposalWrapper, VidCommitment, VidDisperse,
        ViewNumber, vid_commitment,
    },
    epoch_membership::{EpochMembership, EpochMembershipCoordinator},
    message::{Proposal, UpgradeLock},
    signature_key::{BLSPrivKey, BLSPubKey},
    simple_certificate::QuorumCertificate2,
    simple_vote::{QuorumData2, QuorumVote2},
    traits::{BlockPayload, EncodeBytes, signature_key::SignatureKey},
};
use rand::{Rng, thread_rng};
use vbs::version::Version;
use versions::Upgrade;

use super::certs::{TestNodeKeyMap, build_cert};

/// One view of a generated chain.
#[derive(Clone)]
pub struct GeneratedView {
    pub quorum_proposal: Proposal<TestTypes, QuorumProposalWrapper<TestTypes>>,
    /// The proposal's leaf, with its (empty) payload filled in.
    pub leaf: Leaf2<TestTypes>,
    pub view_number: ViewNumber,
    pub leader_public_key: BLSPubKey,
    pub vid_disperse: VidDisperse<TestTypes>,
}

/// Generates consecutive views on `TestTypes` with empty payloads, each justified by a QC on the
/// previous leaf signed by the whole committee. Epochs never advance: every view carries the
/// genesis epoch, and callers patch epoch boundaries in afterwards.
pub struct TestViewGenerator {
    membership: EpochMembershipCoordinator<TestTypes>,
    node_key_map: Arc<TestNodeKeyMap>,
    upgrade: Upgrade,
    upgrade_lock: UpgradeLock<TestTypes>,
    last: Option<GeneratedView>,
}

impl TestViewGenerator {
    pub fn new(
        membership: EpochMembershipCoordinator<TestTypes>,
        node_key_map: Arc<TestNodeKeyMap>,
        upgrade: Upgrade,
    ) -> Self {
        Self {
            membership,
            node_key_map,
            upgrade,
            upgrade_lock: UpgradeLock::new(upgrade),
            last: None,
        }
    }

    /// The next view of the chain, starting with view 1 on the genesis leaf.
    pub async fn next_view(&mut self) -> GeneratedView {
        let view = match &self.last {
            None => self.genesis().await,
            Some(last) => self.view_after(last).await,
        };
        self.last = Some(view.clone());
        view
    }

    async fn genesis(&self) -> GeneratedView {
        let view_number = ViewNumber::new(1);
        let membership = self.epoch_membership();
        let version = self.upgrade_lock.version_infallible(view_number);
        let (payload, metadata) = empty_payload().await;
        let block_header = TestBlockHeader::new(
            &Leaf2::<TestTypes>::genesis(
                &TestValidatedState::default(),
                &TestInstanceState::default(),
                self.upgrade.base,
            )
            .await,
            payload_commitment(&membership, &metadata, version),
            <TestBlockPayload as BlockPayload<TestTypes>>::builder_commitment(&payload, &metadata),
            metadata,
            version,
        );
        let justify_qc = QuorumCertificate2::genesis(
            &TestValidatedState::default(),
            &TestInstanceState::default(),
            self.upgrade,
        )
        .await;
        self.propose(
            view_number,
            block_header,
            justify_qc,
            &membership,
            payload,
            metadata,
        )
        .await
    }

    async fn view_after(&self, last: &GeneratedView) -> GeneratedView {
        let view_number = last.view_number + 1;
        let membership = self.epoch_membership();
        let (last_private_key, last_public_key) =
            self.leader_key_pair(&membership, last.view_number);
        let justify_qc = build_cert::<
            TestTypes,
            QuorumData2<TestTypes>,
            QuorumVote2<TestTypes>,
            QuorumCertificate2<TestTypes>,
        >(
            QuorumData2 {
                leaf_commit: last.leaf.commit(),
                epoch: Some(EpochNumber::genesis()),
                block_number: Some(last.leaf.height()),
            },
            &membership,
            last.view_number,
            &last_public_key,
            &last_private_key,
            &self.upgrade_lock,
        );
        let version = self.upgrade_lock.version_infallible(view_number);
        let (payload, metadata) = empty_payload().await;
        let block_header = TestBlockHeader {
            block_number: *view_number,
            timestamp: *view_number,
            timestamp_millis: *view_number * 1_000,
            payload_commitment: payload_commitment(&membership, &metadata, version),
            builder_commitment: <TestBlockPayload as BlockPayload<TestTypes>>::builder_commitment(
                &payload, &metadata,
            ),
            metadata,
            random: thread_rng().gen_range(0..=u64::MAX),
            version,
        };
        self.propose(
            view_number,
            block_header,
            justify_qc,
            &membership,
            payload,
            metadata,
        )
        .await
    }

    /// The leader's signed proposal and leaf for `view_number`, and the VID dispersal of its
    /// payload.
    async fn propose(
        &self,
        view_number: ViewNumber,
        block_header: TestBlockHeader,
        justify_qc: QuorumCertificate2<TestTypes>,
        membership: &EpochMembership<TestTypes>,
        payload: TestBlockPayload,
        metadata: TestMetadata,
    ) -> GeneratedView {
        let (private_key, leader_public_key) = self.leader_key_pair(membership, view_number);
        let vid_disperse = VidDisperse::calculate_vid_disperse(
            &payload,
            &membership.coordinator,
            view_number,
            Some(EpochNumber::genesis()),
            Some(EpochNumber::genesis()),
            &metadata,
            &self.upgrade_lock,
        )
        .await
        .unwrap()
        .disperse;

        let proposal = QuorumProposalWrapper::<TestTypes> {
            proposal: QuorumProposal2::<TestTypes> {
                block_header,
                view_number,
                epoch: Some(EpochNumber::genesis()),
                justify_qc,
                next_epoch_justify_qc: None,
                upgrade_certificate: None,
                view_change_evidence: None,
                next_drb_result: None,
                state_cert: None,
            },
        };
        let mut leaf = Leaf2::from_quorum_proposal(&proposal);
        leaf.fill_block_payload_unchecked(payload);
        let signature = <BLSPubKey as SignatureKey>::sign(&private_key, leaf.commit().as_ref())
            .expect("Failed to sign leaf commitment");

        GeneratedView {
            quorum_proposal: Proposal {
                data: proposal,
                signature,
                _pd: PhantomData,
            },
            leaf,
            view_number,
            leader_public_key,
            vid_disperse,
        }
    }

    fn epoch_membership(&self) -> EpochMembership<TestTypes> {
        self.membership
            .membership_for_epoch(Some(EpochNumber::genesis()))
            .unwrap()
    }

    fn leader_key_pair(
        &self,
        membership: &EpochMembership<TestTypes>,
        view_number: ViewNumber,
    ) -> (BLSPrivKey, BLSPubKey) {
        let leader = membership
            .leader(view_number)
            .expect("expected Membership::leader to succeed");
        let private_key = self
            .node_key_map
            .get(&leader)
            .expect("expected the leader's private key to be in the key map")
            .clone();
        (private_key, leader)
    }
}

async fn empty_payload() -> (TestBlockPayload, TestMetadata) {
    <TestBlockPayload as BlockPayload<TestTypes>>::from_transactions(
        &[],
        &TestValidatedState::default(),
        &TestInstanceState::default(),
    )
    .await
    .unwrap()
}

fn payload_commitment(
    membership: &EpochMembership<TestTypes>,
    metadata: &TestMetadata,
    version: Version,
) -> VidCommitment {
    vid_commitment(
        &TestTransaction::encode(&[]),
        &metadata.encode(),
        membership.total_nodes(),
        version,
    )
}
