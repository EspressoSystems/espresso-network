use std::collections::{HashMap, HashSet};

use hotshot::traits::BlockPayload;
use hotshot_types::{
    data::{VidCommitment, VidCommitment2, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    traits::{
        block_contents::BlockHeader, node_implementation::NodeType, signature_key::SignatureKey,
    },
    vote::HasViewNumber,
};
use tokio::task::JoinSet;
use tracing::{debug, error, warn};

use crate::{
    consensus::Consensus,
    message::{
        Message, MessageType, Validated,
        fetch::Request,
        payload::{
            PayloadFetchMessage, PayloadFetchResponse, PayloadRequestBody, PayloadResponseBody,
        },
    },
    vid::{ObtainedPayload, expected_vid_param, matches_commitment},
};

/// The requesting half of payload recovery.
pub struct Fetcher<T: NodeType> {
    public_key: T::SignatureKey,

    /// Outstanding fetches.
    requested: HashMap<(ViewNumber, VidCommitment2), Fetch<T::SignatureKey>>,

    /// Verifications of received payloads, surfaced by [`Fetcher::next`].
    tasks: JoinSet<Option<ObtainedPayload<T>>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Retry {
    NewRound,
    SameRound,
}

/// The peers one fetch has involved.
///
/// `asked` decides who not to draw again, and a new round forgets it.
/// `pending` is who still owes us an answer, and outlives the round.
struct Fetch<K> {
    asked: HashSet<K>,
    pending: HashSet<K>,
}

impl<K> Default for Fetch<K> {
    fn default() -> Self {
        Self {
            asked: HashSet::new(),
            pending: HashSet::new(),
        }
    }
}

impl<T: NodeType> Fetcher<T> {
    pub fn new(public_key: T::SignatureKey) -> Self {
        Self {
            public_key,
            requested: HashMap::new(),
            tasks: JoinSet::new(),
        }
    }

    pub async fn next(&mut self) -> Option<ObtainedPayload<T>> {
        loop {
            match self.tasks.join_next().await? {
                Ok(Some(out)) => return Some(out),
                Ok(None) => {},
                Err(err) => {
                    if err.is_panic() {
                        error!(%err, "fetched payload verification panicked");
                    }
                },
            }
        }
    }

    pub fn request(
        &mut self,
        view: ViewNumber,
        commitment: VidCommitment2,
        retry: Retry,
        consensus: &Consensus<T>,
        membership: &EpochMembershipCoordinator<T>,
    ) -> Option<(T::SignatureKey, Message<T, Validated>)> {
        let Some(target) = self.select_peer(view, commitment, retry, consensus, membership) else {
            warn!(%view, "no peer to fetch the payload from");
            return None;
        };

        let fetch = self.requested.entry((view, commitment)).or_default();

        fetch.asked.insert(target.clone());
        fetch.pending.insert(target.clone());

        let message = Message {
            sender: self.public_key.clone(),
            message_type: {
                let request = Request::new(view, PayloadRequestBody);
                MessageType::PayloadFetch(PayloadFetchMessage::Req(request))
            },
        };

        Some((target, message))
    }

    pub fn response(
        &mut self,
        response: PayloadFetchResponse,
        sender: &T::SignatureKey,
        consensus: &Consensus<T>,
        membership: &EpochMembershipCoordinator<T>,
    ) -> bool {
        let view = response.view_number();
        match response.into_body() {
            PayloadResponseBody::NotAvailable | PayloadResponseBody::TooLarge => {
                let spent = self
                    .requested
                    .iter_mut()
                    .filter(|((asked_view, _), _)| *asked_view == view)
                    .any(|(_, fetch)| fetch.pending.remove(sender));
                if !spent {
                    warn!(%view, %sender, "payload refusal from unexpected peer");
                }
                spent
            },
            PayloadResponseBody::Payload { commitment, data } => {
                let Some(proposal) = consensus.proposal_at(view) else {
                    return false;
                };

                let VidCommitment::V2(payload_commitment) =
                    proposal.block_header.payload_commitment()
                else {
                    return false;
                };

                if payload_commitment != commitment {
                    warn!(%view, "payload response does not match the proposal's commitment");
                    return false;
                }

                if !self
                    .requested
                    .get(&(view, payload_commitment))
                    .is_some_and(|fetch| fetch.pending.contains(sender))
                {
                    warn!(%view, "payload response from unexpected peer");
                    return false;
                }

                if consensus.is_reconstructed(view, payload_commitment) {
                    debug!(%view, "payload already obtained; dropping the response");
                    if let Some(fetch) = self.requested.get_mut(&(view, payload_commitment)) {
                        fetch.pending.remove(sender);
                    }
                    return false;
                }

                if let Some(fetch) = self.requested.get_mut(&(view, payload_commitment)) {
                    fetch.pending.remove(sender);
                }

                let Some(param) = expected_vid_param(membership, proposal.epoch) else {
                    warn!(%view, "no VID param for fetched payload; dropping");
                    return false;
                };

                let epoch = proposal.epoch;
                let metadata = proposal.block_header.metadata().clone();

                self.tasks.spawn_blocking(move || {
                    if !matches_commitment::<T>(view, &param, &metadata, &data, &payload_commitment)
                    {
                        return None;
                    }
                    let payload = T::BlockPayload::from_bytes(&data, &metadata);
                    let tx_commitments = payload.transaction_commitments(&metadata);
                    Some(ObtainedPayload {
                        view,
                        epoch,
                        payload_commitment,
                        payload,
                        metadata,
                        tx_commitments,
                    })
                });

                false
            },
        }
    }

    pub fn gc(&mut self, view: ViewNumber) {
        self.requested.retain(|(v, _), _| *v > view);
    }

    fn select_peer(
        &mut self,
        view: ViewNumber,
        payload_commitment: VidCommitment2,
        retry: Retry,
        consensus: &Consensus<T>,
        membership: &EpochMembershipCoordinator<T>,
    ) -> Option<T::SignatureKey> {
        use rand::seq::IteratorRandom;

        let epoch = consensus.proposal_at(view)?.epoch;
        let membership = membership.stake_table_for_epoch(Some(epoch)).ok()?;

        let mut rng = rand::thread_rng();
        let mut fetch = self.requested.get_mut(&(view, payload_commitment));
        let me = &self.public_key;

        let draw = |fetch: Option<&Fetch<T::SignatureKey>>, rng: &mut _| {
            membership
                .stake_table()
                .map(|peer| T::SignatureKey::public_key(&peer.stake_table_entry))
                .filter(|peer| peer != me && fetch.is_none_or(|fetch| !fetch.asked.contains(peer)))
                .choose(rng)
        };

        if let Some(peer) = draw(fetch.as_deref(), &mut rng) {
            return Some(peer);
        }

        if retry == Retry::SameRound {
            return None;
        }

        let fetch = fetch.as_mut()?;
        fetch.asked.clear();

        draw(None, &mut rng)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use hotshot::types::BLSPubKey;
    use hotshot_example_types::{
        block_types::{TestBlockHeader, TestBlockPayload, TestMetadata, TestTransaction},
        node_types::{TEST_VERSIONS, TestTypes},
    };
    use hotshot_types::{
        data::{EpochNumber, Leaf2, VidCommitment, VidCommitment2, ViewNumber, ns_table},
        traits::{block_contents::EncodeBytes, signature_key::SignatureKey},
        vid::avidm_gf2::AvidmGf2Scheme,
    };

    use super::{Fetch, Fetcher, Retry};
    use crate::{
        message::{
            fetch::Response,
            payload::{PayloadFetchResponse, PayloadResponseBody},
        },
        tests::common::utils::{ConsensusHarness, TestData},
        vid::expected_vid_param,
    };

    fn key(index: u64) -> BLSPubKey {
        BLSPubKey::generated_from_seed_indexed([0; 32], index).0
    }

    fn view() -> ViewNumber {
        ViewNumber::new(1)
    }

    /// A payload, and a proposal at view 1 that commits to it.
    ///
    /// The commitment is computed the way a disperser computes it, from the
    /// committee's erasure parameters, so `response` derives the same one from
    /// the proposal and the stake table.
    async fn payload_and_consensus() -> (Vec<u8>, VidCommitment2, ConsensusHarness) {
        let mut harness = ConsensusHarness::new(0).await;
        let test_data = TestData::new(1).await;

        let metadata = TestMetadata {
            num_transactions: 1,
        };
        let payload = TestBlockPayload {
            transactions: vec![TestTransaction::new(vec![1, 2, 3])],
        };
        let bytes = payload.encode().to_vec();

        let param = expected_vid_param(&harness.membership_coordinator, EpochNumber::genesis())
            .expect("committee resolves");
        let ns_table = ns_table::parse_ns_table(bytes.len(), &metadata.encode());
        let (commitment, _) = AvidmGf2Scheme::commit(&param, &bytes, ns_table).expect("commit");

        let template = &test_data.views[0];
        let parent_leaf: Leaf2<TestTypes> = template.proposal.data.clone().into();
        let mut proposal = template.proposal.data.clone();
        proposal.block_header = TestBlockHeader::new(
            &parent_leaf,
            VidCommitment::V2(commitment),
            template
                .proposal
                .data
                .block_header
                .builder_commitment
                .clone(),
            metadata,
            TEST_VERSIONS.test.base,
        );

        harness.consensus.force_set_proposal(view(), proposal);
        (bytes, commitment, harness)
    }

    fn fetcher_expecting(peer: BLSPubKey, commitment: VidCommitment2) -> Fetcher<TestTypes> {
        let mut fetcher = Fetcher::new(key(0));
        fetcher.requested.insert(
            (view(), commitment),
            Fetch {
                asked: HashSet::from([peer]),
                pending: HashSet::from([peer]),
            },
        );
        fetcher
    }

    fn payload_response(commitment: VidCommitment2, data: Vec<u8>) -> PayloadFetchResponse {
        Response::new(view(), PayloadResponseBody::Payload { commitment, data })
    }

    fn accept(
        fetcher: &mut Fetcher<TestTypes>,
        response: PayloadFetchResponse,
        sender: BLSPubKey,
        harness: &ConsensusHarness,
    ) -> bool {
        fetcher.response(
            response,
            &sender,
            &harness.consensus,
            &harness.membership_coordinator,
        )
    }

    /// The payload a peer we asked sends, which re-commits to what our own
    /// proposal names, comes back out of the fetcher.
    #[tokio::test]
    async fn a_payload_that_recommits_is_yielded() {
        let (bytes, commitment, harness) = payload_and_consensus().await;
        let peer = key(1);
        let mut fetcher = fetcher_expecting(peer, commitment);

        assert!(!accept(
            &mut fetcher,
            payload_response(commitment, bytes),
            peer,
            &harness
        ));

        let obtained = fetcher.next().await.expect("payload is verified");
        assert_eq!(obtained.view, view());
        assert_eq!(obtained.payload_commitment, commitment);
    }

    /// Bytes that do not re-commit are dropped, however well-formed the rest
    /// of the response is. Nothing but the commitment vouches for them.
    #[tokio::test]
    async fn a_payload_that_does_not_recommit_is_dropped() {
        let (mut bytes, commitment, harness) = payload_and_consensus().await;
        bytes.push(0);
        let peer = key(1);
        let mut fetcher = fetcher_expecting(peer, commitment);

        accept(
            &mut fetcher,
            payload_response(commitment, bytes),
            peer,
            &harness,
        );

        assert!(fetcher.next().await.is_none());
    }

    /// A peer we did not ask does not get to spend our verification, and does
    /// not get to send us to another peer either.
    #[tokio::test]
    async fn an_unsolicited_payload_is_not_verified() {
        let (bytes, commitment, harness) = payload_and_consensus().await;
        let mut fetcher = fetcher_expecting(key(1), commitment);

        assert!(!accept(
            &mut fetcher,
            payload_response(commitment, bytes),
            key(2),
            &harness
        ));

        assert!(fetcher.next().await.is_none());
        assert!(
            fetcher
                .requested
                .get(&(view(), commitment))
                .is_some_and(|fetch| fetch.pending.contains(&key(1))),
            "the peer we asked still owes us an answer"
        );
    }

    /// A response naming a commitment our proposal does not carry is not the
    /// block we asked for, and is not a reason to draw another peer.
    #[tokio::test]
    async fn a_response_for_another_commitment_is_dropped() {
        let (bytes, commitment, harness) = payload_and_consensus().await;
        let peer = key(1);
        let mut fetcher = fetcher_expecting(peer, commitment);

        assert!(!accept(
            &mut fetcher,
            payload_response(VidCommitment2::default(), bytes),
            peer,
            &harness
        ));

        assert!(fetcher.next().await.is_none());
    }

    /// Answering consumes the peer's slot, so one request buys one
    /// verification and a peer cannot keep sending payloads to verify.
    #[tokio::test]
    async fn answering_consumes_the_peers_slot() {
        let (bytes, commitment, harness) = payload_and_consensus().await;
        let peer = key(1);
        let mut fetcher = fetcher_expecting(peer, commitment);

        accept(
            &mut fetcher,
            payload_response(commitment, bytes.clone()),
            peer,
            &harness,
        );
        accept(
            &mut fetcher,
            payload_response(commitment, bytes),
            peer,
            &harness,
        );

        assert!(fetcher.next().await.is_some());
        assert!(
            fetcher.next().await.is_none(),
            "the second response was not verified"
        );
    }

    /// The draw excludes this node and everyone this fetch already asked, so
    /// a peer that stays silent is not drawn again while the round lasts.
    #[tokio::test]
    async fn the_draw_excludes_this_node_and_the_peers_already_asked() {
        let (_, commitment, harness) = payload_and_consensus().await;
        let mut fetcher = Fetcher::new(key(0));
        let mut drawn = HashSet::new();

        // Ten committee members, one of them us.
        for _ in 0..9 {
            let peer = fetcher
                .select_peer(
                    view(),
                    commitment,
                    Retry::SameRound,
                    &harness.consensus,
                    &harness.membership_coordinator,
                )
                .expect("a peer this round has not asked");
            assert!(peer != key(0), "never ourselves");
            assert!(drawn.insert(peer), "never the same peer twice");
            fetcher
                .requested
                .entry((view(), commitment))
                .or_default()
                .asked
                .insert(peer);
        }
        assert_eq!(drawn.len(), 9);
    }

    /// Once every peer has been asked the round is over, and only the
    /// periodic retry may start another one. A refusal must not, or peers
    /// refusing at message speed would keep the draw going round the
    /// committee.
    #[tokio::test]
    async fn only_a_new_round_starts_the_draw_over() {
        let (_, commitment, harness) = payload_and_consensus().await;
        let mut fetcher = Fetcher::new(key(0));
        let everyone: HashSet<BLSPubKey> = (1..10).map(key).collect();
        fetcher.requested.insert(
            (view(), commitment),
            Fetch {
                asked: everyone.clone(),
                pending: HashSet::new(),
            },
        );

        assert!(
            fetcher
                .select_peer(
                    view(),
                    commitment,
                    Retry::SameRound,
                    &harness.consensus,
                    &harness.membership_coordinator,
                )
                .is_none(),
            "the round is exhausted"
        );

        let peer = fetcher
            .select_peer(
                view(),
                commitment,
                Retry::NewRound,
                &harness.consensus,
                &harness.membership_coordinator,
            )
            .expect("a new round draws again");
        assert!(everyone.contains(&peer));
        assert!(
            fetcher.requested[&(view(), commitment)].asked.is_empty(),
            "the new round forgot who was asked"
        );
    }

    /// Only a peer that owes us an answer sends us to another one, and only
    /// once: refusing spends the slot, so a peer cannot repeat it to keep the
    /// draw going round the committee.
    #[tokio::test]
    async fn only_a_peer_we_asked_redraws_and_only_once() {
        let (_, commitment, harness) = payload_and_consensus().await;
        let peer = key(1);
        let mut fetcher = fetcher_expecting(peer, commitment);
        let refusal = || Response::new(view(), PayloadResponseBody::NotAvailable);

        assert!(!accept(&mut fetcher, refusal(), key(2), &harness));
        assert!(accept(&mut fetcher, refusal(), peer, &harness));
        assert!(
            !accept(&mut fetcher, refusal(), peer, &harness),
            "the slot was spent by the first refusal"
        );
        assert!(
            fetcher
                .requested
                .get(&(view(), commitment))
                .is_some_and(|fetch| fetch.asked.contains(&peer)),
            "a spent peer is still not drawn again"
        );
    }
}
