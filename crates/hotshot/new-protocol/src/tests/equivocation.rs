//! A node that got an equivocating leader's other proposal still follows the chain.
//!
//! The leader of view 1 sends proposal A to the network and a different proposal B,
//! with this node's VID share, to this node alone. The network certifies and
//! decides A, and view 2's proposal extends it. Whichever proposal this node got
//! first, it has to end up holding A beside the rival: locking on A, casting its
//! vote2 for A, deciding view 1 with A, and voting for view 2's proposal all look
//! the proposal up by the certificate that names A.
//!
//! The node is a single `Consensus` fed by hand, in the orders a network can
//! deliver. It asks for A by fetch when it needs it, and the test answers the
//! fetch as a peer would. B's share is made up: consensus takes shares as
//! already verified, and only its pairing key matters here.

use committable::Committable;
use hotshot::types::BLSPubKey;
use hotshot_example_types::node_types::TestTypes;
use hotshot_types::{
    data::{VidCommitment, VidCommitment2, ViewNumber},
    message::Proposal as SignedProposal,
    traits::signature_key::SignatureKey,
    vote::HasViewNumber,
};

use crate::{
    consensus::{ConsensusInput, ConsensusOutput},
    helpers::proposal_commitment,
    message::ProposalMessage,
    tests::common::{
        assertions::{count_matching, is_vote1_for_view, node_index_for_key},
        utils::{ConsensusHarness, TestData, TestView},
    },
};

/// The rival arrives first: the node votes for it before it learns of A.
///
/// The node then receives view 2's proposal, which extends A, the certificates
/// over A, and A's payload reconstructed from the other voters' shares.
#[tokio::test]
async fn node_that_voted_for_the_rival_follows_the_decided_proposal() {
    let test_data = TestData::new(2).await;
    let (a, c) = (&test_data.views[0], &test_data.views[1]);
    let mut node = Node::new(&test_data).await;

    let rival_inputs = rival(a, &node.key);
    node.apply_pair(rival_inputs).await;
    assert_eq!(
        count_matching(node.harness.outputs(), |o| is_vote1_for_view(o, 1)),
        1,
        "setup: the node votes for the rival it was sent"
    );

    node.apply_pair(c.proposal_input_consensus(&node.key)).await;
    node.apply(a.cert1_input()).await;
    node.apply(a.block_reconstructed_input()).await;
    node.apply(a.cert2_input()).await;

    node.assert_follows(a);
}

/// The rival's proposal arrives later: the node already holds A, fetched without a share.
///
/// The leader sends the node its share of the rival while the node is still in
/// view 1, and holds the rival's proposal back. The share waits for a proposal
/// to pair with. The node then receives view 2's proposal and fetches its
/// parent, A. The rival's proposal arrives after that and pairs with the waiting
/// share, before A's certificates and its reconstructed payload.
#[tokio::test]
async fn node_holding_the_decided_proposal_keeps_it_when_the_rival_arrives() {
    let test_data = TestData::new(2).await;
    let (a, c) = (&test_data.views[0], &test_data.views[1]);
    let mut node = Node::new(&test_data).await;

    let (rival_proposal, rival_share) = rival(a, &node.key);
    node.apply(rival_share).await;

    node.apply_pair(c.proposal_input_consensus(&node.key)).await;
    assert_eq!(
        node.held_at_view_1(),
        Some(proposal_commitment(&a.proposal.data)),
        "setup: the node fetched A as view 2's parent"
    );

    node.apply(rival_proposal).await;
    node.apply(a.cert1_input()).await;
    node.apply(a.block_reconstructed_input()).await;
    node.apply(a.cert2_input()).await;

    node.assert_follows(a);
}

/// A node that leads neither view, with a peer answering its fetches for A.
struct Node<'a> {
    harness: ConsensusHarness,
    key: BLSPubKey,
    a: &'a TestView,
    fetches_answered: usize,
}

impl<'a> Node<'a> {
    async fn new(test_data: &'a TestData) -> Self {
        let leaders: Vec<u64> = test_data
            .views
            .iter()
            .map(|v| node_index_for_key(&v.leader_public_key))
            .collect();
        let index = (0..10)
            .find(|i| !leaders.contains(i))
            .expect("ten nodes, two leaders");
        Self {
            harness: ConsensusHarness::new(index).await,
            key: BLSPubKey::generated_from_seed_indexed([0; 32], index).0,
            a: &test_data.views[0],
            fetches_answered: 0,
        }
    }

    async fn apply(&mut self, input: ConsensusInput<TestTypes>) {
        self.harness.apply(input).await;
        self.answer_fetches().await;
    }

    async fn apply_pair(
        &mut self,
        (proposal, share): (ConsensusInput<TestTypes>, ConsensusInput<TestTypes>),
    ) {
        self.apply(proposal).await;
        self.apply(share).await;
    }

    /// Answer every fetch for A the node has asked for since the last answer.
    async fn answer_fetches(&mut self) {
        let a_commit = proposal_commitment(&self.a.proposal.data);
        loop {
            let requested = count_matching(self.harness.outputs(), |o| {
                matches!(
                    o,
                    ConsensusOutput::RequestMissingProposal { view, leaf_commit }
                        if *view == ViewNumber::new(1) && *leaf_commit == a_commit
                )
            });
            if requested <= self.fetches_answered {
                return;
            }
            self.fetches_answered = requested;
            self.harness
                .apply(ConsensusInput::FetchedProposal(self.a.proposal_message()))
                .await;
        }
    }

    fn held_at_view_1(
        &self,
    ) -> Option<committable::Commitment<hotshot_types::data::Leaf2<TestTypes>>> {
        self.harness
            .consensus
            .proposal_at(ViewNumber::new(1))
            .map(proposal_commitment)
    }

    /// The node holds A beside the rival, locked on A, cast its vote2 for A,
    /// decided view 1 with A, and voted for view 2's proposal.
    ///
    /// Checked together, so a failure shows which of them went wrong.
    fn assert_follows(&self, a: &TestView) {
        let a_commit = proposal_commitment(&a.proposal.data);
        let outputs = self.harness.outputs();
        let decided_at_view_1: Vec<_> = outputs
            .iter()
            .filter_map(|o| match o {
                ConsensusOutput::LeafDecided { leaves, .. } => Some(leaves),
                _ => None,
            })
            .flatten()
            .filter(|leaf| *leaf.view_number() == 1)
            .map(|leaf| leaf.commit())
            .collect();
        let vote2s_at_view_1: Vec<_> = outputs
            .iter()
            .filter_map(|o| match o {
                ConsensusOutput::SendVote2(v) if *v.view_number() == 1 => Some(v.data.leaf_commit),
                _ => None,
            })
            .collect();
        let proposals = self.harness.consensus.proposals();
        assert_eq!(
            (
                proposals.get(ViewNumber::new(1), a_commit).is_some(),
                proposals.at(ViewNumber::new(1)).count(),
                self.harness.consensus.locked_view(),
                vote2s_at_view_1,
                self.held_at_view_1(),
                decided_at_view_1,
                count_matching(outputs, |o| is_vote1_for_view(o, 2)),
            ),
            (
                true,
                2,
                Some(ViewNumber::new(1)),
                vec![a_commit],
                Some(a_commit),
                vec![a_commit],
                1,
            ),
            "(A held, proposals held at view 1, lock, vote2s at view 1, proposal picked at view \
             1, leaves decided at view 1, vote1s at view 2)"
        );
    }
}

/// The inputs that deliver another proposal for `a`'s view, from the same leader
/// and epoch, together with `node`'s share of it.
fn rival(a: &TestView, node: &BLSPubKey) -> (ConsensusInput<TestTypes>, ConsensusInput<TestTypes>) {
    let mut rival = a.proposal.data.clone();
    let payload = VidCommitment2::default();
    rival.block_header.payload_commitment = VidCommitment::V2(payload);
    let signature = <BLSPubKey as SignatureKey>::sign(
        &a.leader_private_key,
        proposal_commitment(&rival).as_ref(),
    )
    .expect("sign the rival");
    let mut share = a.vid_share_for(node);
    share.payload_commitment = payload;
    (
        ConsensusInput::Proposal(
            a.leader_public_key,
            ProposalMessage::validated(SignedProposal::new(rival, signature)),
        ),
        ConsensusInput::VidShare(share),
    )
}
