use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use async_trait::async_trait;
use consensus_store::Bytes;
use hotshot_types::{
    data::{VidCommitment2, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    traits::{
        BlockPayload,
        block_contents::{BlockHeader, EncodeBytes},
        node_implementation::NodeType,
        signature_key::SignatureKey,
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

/// Where a node finds the payloads of blocks it voted on but no longer holds, as after a restart:
/// its deployment's store, which acknowledged each before the node voted vote2, and other
/// deployments' stores.
#[async_trait]
pub trait PayloadSource: Send + Sync {
    /// The payload with VID commitment `commitment`, as consensus encodes payloads, if a store has
    /// it. It is checked against the commitment, so a source need not be trusted.
    async fn payload(&self, commitment: VidCommitment2) -> anyhow::Result<Option<Bytes>>;
}

/// The requesting half of payload recovery: a payload missing from memory is asked of the
/// [`PayloadSource`] first, then of peers.
pub struct Fetcher<T: NodeType> {
    public_key: T::SignatureKey,

    /// Where payloads are asked for before peers, if anywhere.
    source: Option<Arc<dyn PayloadSource>>,

    /// Payloads being asked of the source.
    from_source: HashSet<(ViewNumber, VidCommitment2)>,

    /// Outstanding fetches from peers.
    requested: HashMap<(ViewNumber, VidCommitment2), Fetch<T::SignatureKey>>,

    /// Requests of the source, and verifications of received payloads, surfaced by
    /// [`Fetcher::next`].
    tasks: JoinSet<Fetched<T>>,
}

/// What [`Fetcher::next`] surfaces.
pub enum Fetched<T: NodeType> {
    /// A payload, checked against its commitment.
    Payload(ObtainedPayload<T>),

    /// The source does not have the payload, which then has to come from peers.
    NotInSource(ViewNumber, VidCommitment2),

    /// A payload from a peer that did not match its commitment.
    Rejected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Retry {
    /// A new round, which asks the source first, then peers again.
    NewRound,

    /// A new round of peers, once the source does not have the payload.
    AfterSource,

    /// The next peer of the current round.
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
            source: None,
            from_source: HashSet::new(),
            requested: HashMap::new(),
            tasks: JoinSet::new(),
        }
    }

    /// Asks `source` for each missing payload before asking peers.
    pub fn with_source(mut self, source: Arc<dyn PayloadSource>) -> Self {
        self.source = Some(source);
        self
    }

    pub async fn next(&mut self) -> Option<Fetched<T>> {
        loop {
            match self.tasks.join_next().await? {
                Ok(Fetched::Rejected) => {},
                Ok(fetched) => return Some(fetched),
                Err(err) => {
                    if err.is_panic() {
                        error!(%err, "fetched payload verification panicked");
                    }
                },
            }
        }
    }

    /// Asks for the payload with commitment `commitment` of the block proposed in `view`. A new
    /// round asks the source first, unless it is being asked already, and returns no request for
    /// a peer: [`Fetched::NotInSource`] says when to ask peers. Otherwise returns the request for
    /// the next peer, if any is left.
    pub fn request(
        &mut self,
        view: ViewNumber,
        commitment: VidCommitment2,
        retry: Retry,
        consensus: &Consensus<T>,
        membership: &EpochMembershipCoordinator<T>,
    ) -> Option<(T::SignatureKey, Message<T, Validated>)> {
        if retry == Retry::NewRound
            && let Some(source) = self.source.clone()
            && let Some(check) = Check::of(view, commitment, consensus, membership)
        {
            if self.from_source.insert((view, commitment)) {
                self.tasks.spawn(async move {
                    match source.payload(commitment).await {
                        Ok(Some(data)) => match check.run(data.to_vec()).await {
                            Some(payload) => Fetched::Payload(payload),
                            None => {
                                warn!(%view, "a payload from the source does not match");
                                Fetched::NotInSource(view, commitment)
                            },
                        },
                        Ok(None) => Fetched::NotInSource(view, commitment),
                        Err(err) => {
                            debug!(%view, "asking the source for a payload failed: {err:#}");
                            Fetched::NotInSource(view, commitment)
                        },
                    }
                });
            }
            return None;
        }
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
    ) -> Vec<VidCommitment2> {
        let view = response.view_number();
        match response.into_body() {
            PayloadResponseBody::NotAvailable | PayloadResponseBody::TooLarge => {
                let refused = self.refused(view, sender);
                if refused.is_empty() {
                    warn!(%view, %sender, "payload refusal from unexpected peer");
                }
                refused
            },
            PayloadResponseBody::Payload {
                commitment: payload_commitment,
                data,
            } => {
                // A request names only the view, and a peer that kept another
                // proposal's payload for it answers with that one. That is no
                // answer to what was asked, so it counts as a refusal.
                if !self
                    .requested
                    .get(&(view, payload_commitment))
                    .is_some_and(|fetch| fetch.pending.contains(sender))
                {
                    let refused = self.refused(view, sender);
                    if refused.is_empty() {
                        warn!(%view, %sender, "payload response from unexpected peer");
                    } else {
                        debug!(%view, %sender, "payload response for another payload than asked");
                    }
                    return refused;
                }

                if consensus
                    .proposal_with_payload(view, payload_commitment)
                    .is_none()
                {
                    warn!(%view, "payload response does not match a proposal's commitment");
                    return Vec::new();
                }

                if consensus.is_reconstructed(view, payload_commitment) {
                    debug!(%view, "payload already obtained; dropping the response");
                    if let Some(fetch) = self.requested.get_mut(&(view, payload_commitment)) {
                        fetch.pending.remove(sender);
                    }
                    return Vec::new();
                }

                if let Some(fetch) = self.requested.get_mut(&(view, payload_commitment)) {
                    fetch.pending.remove(sender);
                }

                let Some(check) = Check::of(view, payload_commitment, consensus, membership) else {
                    warn!(%view, "no VID param for fetched payload; dropping");
                    return Vec::new();
                };
                self.tasks.spawn(async move {
                    check
                        .run(data)
                        .await
                        .map_or(Fetched::Rejected, Fetched::Payload)
                });

                Vec::new()
            },
        }
    }

    /// Drop the fetches for views before `view`, that of a block just decided.
    ///
    /// The decided block's own fetch stays: the payload of the newest decided
    /// block is still needed to vote on its children, and a decide can follow
    /// the request at once.
    pub fn gc(&mut self, view: ViewNumber) {
        self.requested.retain(|(v, _), _| *v >= view);
        self.from_source.retain(|(v, _)| *v >= view);
    }

    /// Notes that the source answered for the payload, so that a later round asks it again.
    pub fn source_answered(&mut self, view: ViewNumber, commitment: VidCommitment2) {
        self.from_source.remove(&(view, commitment));
    }

    /// `sender` answered a request at `view` without the payload asked for.
    ///
    /// Every payload pending from it at `view` counts as refused, not only the
    /// one it answered: a peer serves one block per view (`Server::blocks`), so
    /// one that refuses or sends another payload has none of the others either.
    /// Returns those payloads, which are the ones worth asking another peer for.
    fn refused(&mut self, view: ViewNumber, sender: &T::SignatureKey) -> Vec<VidCommitment2> {
        self.requested
            .iter_mut()
            .filter(|((asked_view, _), _)| *asked_view == view)
            .filter_map(|((_, commitment), fetch)| {
                fetch.pending.remove(sender).then_some(*commitment)
            })
            .collect()
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

        let epoch = consensus
            .proposal_with_payload(view, payload_commitment)?
            .epoch;
        let membership = membership.stake_table_for_epoch(epoch).ok()?;

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

/// What a payload of the block proposed in `view` must match, from the block's proposal.
struct Check<T: NodeType> {
    view: ViewNumber,
    epoch: hotshot_types::data::EpochNumber,
    commitment: VidCommitment2,
    param: hotshot_types::vid::avidm_gf2::AvidmGf2Param,
    metadata: <T::BlockPayload as BlockPayload<T>>::Metadata,
}

impl<T: NodeType> Check<T> {
    /// The check of a payload with commitment `commitment`, if this node holds a proposal of it
    /// and the stake table of its epoch.
    fn of(
        view: ViewNumber,
        commitment: VidCommitment2,
        consensus: &Consensus<T>,
        membership: &EpochMembershipCoordinator<T>,
    ) -> Option<Self> {
        let proposal = consensus.proposal_with_payload(view, commitment)?;
        Some(Self {
            view,
            epoch: proposal.epoch,
            commitment,
            param: expected_vid_param(membership, proposal.epoch)?,
            metadata: proposal.block_header.metadata().clone(),
        })
    }

    /// The payload `data` encodes, if it matches the commitment.
    async fn run(self, data: Vec<u8>) -> Option<ObtainedPayload<T>> {
        tokio::task::spawn_blocking(move || {
            if !matches_commitment(
                self.view,
                &self.param,
                &self.metadata.encode(),
                &data,
                &self.commitment,
            ) {
                return None;
            }
            let payload = T::BlockPayload::from_bytes(&data, &self.metadata);
            let tx_commitments = payload.transaction_commitments(&self.metadata);
            Some(ObtainedPayload {
                view: self.view,
                epoch: self.epoch,
                payload_commitment: self.commitment,
                payload,
                metadata: self.metadata,
                tx_commitments,
            })
        })
        .await
        .ok()
        .flatten()
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, sync::Arc};

    use async_trait::async_trait;
    use consensus_store::Bytes;
    use hotshot_example_types::{
        block_types::{TestBlockHeader, TestBlockPayload, TestMetadata, TestTransaction},
        node_types::{TEST_VERSIONS, TestTypes},
    };
    use hotshot_types::{
        data::{EpochNumber, Leaf2, VidCommitment, VidCommitment2, ViewNumber, ns_table},
        signature_key::BLSPubKey,
        traits::{block_contents::EncodeBytes, signature_key::SignatureKey},
        vid::avidm_gf2::AvidmGf2Scheme,
    };

    use super::{Fetch, Fetched, Fetcher, PayloadSource, Retry};
    use crate::{
        message::{
            Message, Validated,
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

        harness.force_set_proposal(view(), proposal);
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
        !fetcher
            .response(
                response,
                &sender,
                &harness.consensus,
                &harness.membership_coordinator,
            )
            .is_empty()
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

        let Some(Fetched::Payload(obtained)) = fetcher.next().await else {
            panic!("payload is verified");
        };
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

    /// A response naming another commitment than the one asked for is not the
    /// payload we wanted, so it counts as a refusal.
    ///
    /// A request names only the view, and a peer that kept another proposal's
    /// payload for it answers with that one. Waiting on it would stall the fetch,
    /// so another peer is drawn, once: the answer uses up the peer's slot.
    #[tokio::test]
    async fn a_response_for_another_commitment_is_a_refusal() {
        let (bytes, commitment, harness) = payload_and_consensus().await;
        let peer = key(1);
        let mut fetcher = fetcher_expecting(peer, commitment);

        assert!(accept(
            &mut fetcher,
            payload_response(VidCommitment2::default(), bytes.clone()),
            peer,
            &harness
        ));
        assert!(
            !fetcher
                .requested
                .get(&(view(), commitment))
                .is_some_and(|fetch| fetch.pending.contains(&peer)),
            "the peer no longer owes us an answer"
        );
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

    /// A decide keeps the fetch of the decided block's own payload, which its
    /// children still need, and drops those of earlier views.
    #[tokio::test]
    async fn gc_keeps_the_fetch_of_the_decided_view() {
        let (_, commitment, _harness) = payload_and_consensus().await;
        let mut fetcher = fetcher_expecting(key(1), commitment);
        let earlier = ViewNumber::new(view().u64() - 1);
        fetcher
            .requested
            .insert((earlier, commitment), Fetch::default());

        fetcher.gc(view());

        assert!(fetcher.requested.contains_key(&(view(), commitment)));
        assert!(!fetcher.requested.contains_key(&(earlier, commitment)));
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

    /// A source that has one payload.
    struct OnePayload(Option<(VidCommitment2, Vec<u8>)>);

    #[async_trait]
    impl PayloadSource for OnePayload {
        async fn payload(&self, commitment: VidCommitment2) -> anyhow::Result<Option<Bytes>> {
            Ok(self
                .0
                .as_ref()
                .filter(|(held, _)| *held == commitment)
                .map(|(_, data)| Bytes::from(data.clone())))
        }
    }

    fn request(
        fetcher: &mut Fetcher<TestTypes>,
        commitment: VidCommitment2,
        retry: Retry,
        harness: &ConsensusHarness,
    ) -> Option<(BLSPubKey, Message<TestTypes, Validated>)> {
        fetcher.request(
            view(),
            commitment,
            retry,
            &harness.consensus,
            &harness.membership_coordinator,
        )
    }

    /// A new round asks the source before any peer, and a payload from it that re-commits is
    /// yielded.
    #[tokio::test]
    async fn a_new_round_asks_the_source_first() {
        let (bytes, commitment, harness) = payload_and_consensus().await;
        let source = OnePayload(Some((commitment, bytes)));
        let mut fetcher = Fetcher::new(key(0)).with_source(Arc::new(source));

        assert!(
            request(&mut fetcher, commitment, Retry::NewRound, &harness).is_none(),
            "no peer is asked while the source is"
        );
        let Some(Fetched::Payload(obtained)) = fetcher.next().await else {
            panic!("the source's payload is verified");
        };
        assert_eq!(obtained.payload_commitment, commitment);
    }

    /// A payload the source lacks, or has wrong, is asked of peers after it.
    #[tokio::test]
    async fn peers_are_asked_after_the_source() {
        let (bytes, commitment, harness) = payload_and_consensus().await;
        let mut wrong = bytes.clone();
        wrong.push(0);
        for source in [OnePayload(None), OnePayload(Some((commitment, wrong)))] {
            let mut fetcher = Fetcher::new(key(0)).with_source(Arc::new(source));
            assert!(request(&mut fetcher, commitment, Retry::NewRound, &harness).is_none());
            let Some(Fetched::NotInSource(asked_view, asked)) = fetcher.next().await else {
                panic!("the source does not have the payload");
            };
            assert_eq!((asked_view, asked), (view(), commitment));
            fetcher.source_answered(asked_view, asked);
            assert!(
                request(&mut fetcher, commitment, Retry::AfterSource, &harness).is_some(),
                "a peer is asked"
            );
        }
    }
}
