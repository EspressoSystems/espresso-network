//! Recovery of a certified view's proposal and payload.
//!
//! A `Cert1` proves a quorum held the view's proposal and enough VID shares
//! to reconstruct its payload, but nothing retransmits what a node missed,
//! and a node that cannot reconstruct the view its peers' locks are pinned
//! at cannot vote on anything extending it
//! (`Consensus::request_missing_certified_data`). [`Fetcher`] is the pull
//! that closes the gap: proposals by broadcast, payloads that fit a message
//! whole from one random peer, larger payloads as share retransmissions from
//! a random stake-weighted subset. Never a broadcast of payload data, and
//! attempts are paced to the view timeout with targets re-randomized (not
//! excluded) each round. Nothing is taken on trust: whole payloads must
//! re-commit to the requester's own proposal's commitment, refetched shares
//! pass the ordinary share intake, and a whole-payload response counts only
//! from the transport-authenticated peer it was requested from.

use std::{
    cmp::max,
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};

use alloy::primitives::U256;
use committable::Commitment;
use hotshot::traits::BlockPayload;
use hotshot_types::{
    data::{Leaf2, VidCommitment, VidCommitment2, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    traits::{
        block_contents::{BlockHeader, EncodeBytes},
        node_implementation::NodeType,
        signature_key::{SignatureKey, StakeTableEntryType},
    },
};
use tracing::warn;

use crate::{
    consensus::Consensus,
    coordinator::error::CoordinatorError,
    message::{
        ConsensusMessage, Message, MessageType, PayloadFetchMessage, PayloadFetchResponse,
        ProposalFetchMessage,
    },
    network::Sender,
    vid::{VidReconstructor, expected_vid_param},
};

/// A proposal fetch is for one view's proposal with one specific leaf
/// commitment: the view alone does not identify it, since an equivocating
/// leader can put two proposals into one view.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ProposalFetchKey<T: NodeType> {
    pub view: ViewNumber,
    pub leaf_commitment: Commitment<Leaf2<T>>,
}

impl<T: NodeType> ProposalFetchKey<T> {
    pub fn new(view: ViewNumber, leaf_commitment: Commitment<Leaf2<T>>) -> Self {
        Self {
            view,
            leaf_commitment,
        }
    }
}

type FetchRequests<K> = HashMap<(ViewNumber, VidCommitment2), (Instant, Option<K>)>;

/// How many candidate payloads to keep around.
const CANDIDATE_WINDOW: usize = 3;

/// Requester- and server-side state of payload recovery.
pub struct Fetcher<T: NodeType> {
    public_key: T::SignatureKey,

    /// Payloads at or below this size are fetched whole from one peer.
    ///
    /// Larger payloads are retrieved as VID share retransmissions from many peers.
    max_whole_payload_fetch: usize,

    /// Outstanding fetches.
    ///
    /// When the attempt was made and for whole-payload requests, the peer it went to.
    /// Only that peer's response is accepted, so an unsolicited garbage response
    /// cannot consume the round a genuine answer is still in flight for. `None`
    /// marks a share-path attempt, whose responses arrive as ordinary
    /// `VidShareBroadcast`s instead.
    requested: FetchRequests<T::SignatureKey>,

    /// Outstanding proposal fetches, for the same pacing `requested` gives
    /// payload fetches. Responses are not tracked per target: a proposal
    /// response is self-certifying (validated like any proposal) and small.
    requested_proposals: HashMap<ProposalFetchKey<T>, Instant>,

    /// Peers that advertised a `Qc` for a view as catchup evidence: they lock
    /// what they advertise, and locking requires the reconstructed payload,
    /// so they are the first choice when that payload must be fetched whole.
    advertisers: BTreeMap<ViewNumber, HashSet<T::SignatureKey>>,

    /// The payload of the block this node is locked on, kept to serve
    /// whole-payload fetches. One block suffices because recovery only ever
    /// needs the view the network's locks are pinned at. Promoted from
    /// `candidate` when the lock reaches its view (`Self::note_locked`);
    /// tracking reconstruction order instead would let the stalled rounds'
    /// blocks, which reconstruct but never certify, evict the pinned one.
    retained: Option<(ViewNumber, VidCommitment2, Arc<[u8]>)>,

    /// The highest-view payloads this node reconstructed, awaiting the lock.
    /// A small window rather than one slot: reconstruction order and lock
    /// order need not agree — a later view can reconstruct before an earlier
    /// view's certificate arrives — and a single latch would then miss the
    /// promotion for good.
    candidates: BTreeMap<ViewNumber, (VidCommitment2, Arc<[u8]>)>,

    /// When each requester was last served per view, so that serving follows
    /// the same cadence the requester is paced to: signatures prove who asks,
    /// not how often, and a whole-payload response is worth megabytes. Both
    /// request kinds share one budget per view; an honest node never sends
    /// both for one view within an interval, since the path choice is a
    /// deterministic function of the payload size.
    served: HashMap<(ViewNumber, T::SignatureKey), Instant>,
}

impl<T: NodeType> Fetcher<T> {
    pub fn new(public_key: T::SignatureKey, max_whole_payload_fetch: usize) -> Self {
        Self {
            public_key,
            max_whole_payload_fetch,
            requested: HashMap::new(),
            requested_proposals: HashMap::new(),
            advertisers: BTreeMap::new(),
            retained: None,
            candidates: BTreeMap::new(),
            served: HashMap::new(),
        }
    }

    /// Remember that `peer` advertised the view's certificate.
    pub fn note_advertiser(&mut self, view: ViewNumber, peer: T::SignatureKey) {
        self.advertisers.entry(view).or_default().insert(peer);
    }

    /// Retain a servable copy of a reconstructed payload as a candidate for
    /// `Self::note_locked` to promote.
    pub fn retain_payload(
        &mut self,
        view: ViewNumber,
        payload_commitment: VidCommitment2,
        payload: &T::BlockPayload,
    ) {
        if self.candidates.len() >= CANDIDATE_WINDOW
            && self.candidates.keys().next().is_some_and(|v| view < *v)
            || self.candidates.contains_key(&view)
            || payload.txn_bytes() > self.max_whole_payload_fetch
        {
            return;
        }

        let payload = payload.encode();

        if payload.len() > self.max_whole_payload_fetch {
            return;
        }

        self.candidates.insert(view, (payload_commitment, payload));

        while self.candidates.len() > CANDIDATE_WINDOW {
            self.candidates.pop_first();
        }
    }

    /// Promote a candidate to the retained payload once the lock reaches its
    /// view. Locking requires the reconstructed block, so on the lock path
    /// the candidate is already in place when the lock moves.
    pub fn note_locked(&mut self, locked: Option<ViewNumber>) {
        if let Some(view) = locked
            && let Some((commitment, payload)) = self.candidates.get(&view)
            && self.retained.as_ref().is_none_or(|(v, ..)| *v < view)
        {
            self.retained = Some((view, *commitment, payload.clone()));
        }
    }

    /// How long to wait before repeating a fetch or serving the same
    /// requester again: half the view timeout, floored at one second, so
    /// every timeout round gets at least one attempt and at most two.
    fn retry_interval(view_timeout: Duration) -> Duration {
        max(view_timeout / 2, Duration::from_secs(1))
    }

    /// Ask peers for the certified `view`'s proposal, by broadcast: the
    /// response is a signed proposal, small and self-certifying, so there is
    /// nothing to pace on the serving side. Re-broadcast once per retry
    /// interval — during a stall nothing decides, so the keys are never
    /// trimmed, and a one-shot broadcast that is lost would leave the
    /// proposal missing for good.
    pub fn request_missing_proposal(
        &mut self,
        view: ViewNumber,
        leaf_commit: Commitment<Leaf2<T>>,
        view_timeout: Duration,
        consensus: &Consensus<T>,
        network: &Sender<T>,
    ) -> Result<(), CoordinatorError> {
        let now = Instant::now();
        match self
            .requested_proposals
            .entry(ProposalFetchKey::new(view, leaf_commit))
        {
            std::collections::hash_map::Entry::Occupied(mut requested) => {
                if now.duration_since(*requested.get()) < Self::retry_interval(view_timeout) {
                    return Ok(());
                }
                requested.insert(now);
            },
            std::collections::hash_map::Entry::Vacant(slot) => {
                slot.insert(now);
            },
        }
        self.broadcast_proposal_fetch(view, consensus, network)
    }

    /// Broadcast a signed proposal fetch request for `view` to all peers.
    pub fn broadcast_proposal_fetch(
        &self,
        view: ViewNumber,
        consensus: &Consensus<T>,
        network: &Sender<T>,
    ) -> Result<(), CoordinatorError> {
        let request = consensus.signed_fetch_request(view).map_err(|err| {
            let err = format!("failed to sign proposal request: {err}");
            CoordinatorError::regular(err).context("sign proposal request")
        })?;
        let message = Message {
            sender: self.public_key.clone(),
            message_type: MessageType::ProposalFetch(ProposalFetchMessage::Request(request)),
        };
        network
            .broadcast(consensus.current_view(), &message)
            .map_err(|err| CoordinatorError::from(err).context("broadcast proposal request"))
    }

    /// Whether a fetched proposal would answer one of this node's own
    /// requests. Non-consuming: the request is only consumed once the
    /// response validated ([`Self::take_requested_proposal`]), or a bad
    /// response racing the honest ones would burn the round.
    pub fn is_requested_proposal(
        &self,
        view: ViewNumber,
        leaf_commitment: Commitment<Leaf2<T>>,
    ) -> bool {
        self.requested_proposals
            .contains_key(&ProposalFetchKey::new(view, leaf_commitment))
    }

    /// Whether a fetched proposal answers one of this node's own requests,
    /// consuming the request if so.
    pub fn take_requested_proposal(
        &mut self,
        view: ViewNumber,
        leaf_commitment: Commitment<Leaf2<T>>,
    ) -> bool {
        self.requested_proposals
            .remove(&ProposalFetchKey::new(view, leaf_commitment))
            .is_some()
    }

    /// Ask peers for the certified `view`'s payload.
    ///
    /// Paced at half the view timeout, floored at one second.
    pub fn request(
        &mut self,
        view: ViewNumber,
        payload_commitment: VidCommitment2,
        view_timeout: Duration,
        consensus: &Consensus<T>,
        membership: &EpochMembershipCoordinator<T>,
        network: &Sender<T>,
    ) -> Result<(), CoordinatorError> {
        use rand::seq::{IteratorRandom, SliceRandom};

        let retry_interval = Self::retry_interval(view_timeout);
        let now = Instant::now();

        if self
            .requested
            .get(&(view, payload_commitment))
            .is_some_and(|(last, _)| now.duration_since(*last) < retry_interval)
        {
            return Ok(());
        }

        let request = consensus.signed_fetch_request(view).map_err(|err| {
            let err = format!("failed to sign payload request: {err}");
            CoordinatorError::regular(err).context("sign payload request")
        })?;

        let payload_size = consensus
            .vid_share_at(view)
            .map(|share| share.common.payload_byte_len());

        let mut rng = rand::thread_rng();

        if payload_size.is_some_and(|size| size <= self.max_whole_payload_fetch) {
            // Small payload: fetch it whole from one peer — preferably one
            // that advertised the view's certificate, otherwise any committee
            // member; never this node itself.
            let advertiser = self.advertisers.get(&view).and_then(|advertisers| {
                advertisers
                    .iter()
                    .filter(|peer| **peer != self.public_key)
                    .choose(&mut rng)
                    .cloned()
            });

            let target = advertiser.or_else(|| {
                let epoch = consensus.proposal_at(view)?.epoch;
                let membership = membership.stake_table_for_epoch(Some(epoch)).ok()?;
                membership
                    .stake_table()
                    .map(|peer| T::SignatureKey::public_key(&peer.stake_table_entry))
                    .filter(|peer| *peer != self.public_key)
                    .choose(&mut rng)
            });

            let Some(target) = target else {
                warn!(%view, "no peer to fetch the payload from");
                return Ok(());
            };

            self.requested
                .insert((view, payload_commitment), (now, Some(target.clone())));

            let message = Message {
                sender: self.public_key.clone(),
                message_type: MessageType::PayloadFetch(PayloadFetchMessage::Request(request)),
            };

            return network
                .unicast(consensus.current_view(), &target, &message)
                .map_err(|err| CoordinatorError::from(err).context("unicast payload request"));
        }

        // Large or unknown-size payload: ask a random subset of peers holding
        // half the peer stake for their shares. Half is comfortably above the
        // roughly-a-third of total weight reconstruction needs, with margin
        // for silent peers — and shares accumulate across rounds, so any
        // shortfall is made up by the next round's fresh subset.
        let Some(proposal) = consensus.proposal_at(view) else {
            return Ok(());
        };

        let Ok(membership) = membership.stake_table_for_epoch(Some(proposal.epoch)) else {
            return Ok(());
        };

        let mut peers: Vec<(T::SignatureKey, U256)> = membership
            .stake_table()
            .map(|peer| {
                (
                    T::SignatureKey::public_key(&peer.stake_table_entry),
                    peer.stake_table_entry.stake(),
                )
            })
            .filter(|(peer, _)| *peer != self.public_key)
            .collect();

        let total_stake: U256 = peers.iter().map(|(_, stake)| *stake).sum();
        peers.shuffle(&mut rng);

        let half_stake = total_stake / U256::from(2);
        let mut asked_stake = U256::ZERO;
        let mut targets = Vec::new();
        for (peer, stake) in &peers {
            if asked_stake > half_stake {
                break;
            }
            asked_stake += *stake;
            targets.push(peer);
        }

        if targets.is_empty() {
            warn!(%view, "no peer to fetch shares from");
            return Ok(());
        }

        self.requested
            .insert((view, payload_commitment), (now, None));

        let message = Message {
            sender: self.public_key.clone(),
            message_type: MessageType::ShareFetch(request),
        };

        network
            .multicast(consensus.current_view(), targets, &message)
            .map_err(|err| CoordinatorError::from(err).context("multicast share request"))
    }

    /// Answer a whole-payload fetch.
    ///
    /// A reconstructed copy, or the block this node built, if it fits a message;
    /// otherwise fall back to our own share, as if the requester had asked for
    /// shares — it may have wrongly sized the payload.
    pub fn serve_payload(
        &mut self,
        view: ViewNumber,
        requester: &T::SignatureKey,
        view_timeout: Duration,
        consensus: &Consensus<T>,
        network: &Sender<T>,
    ) {
        if !self.may_serve(view, requester, view_timeout) {
            return;
        }
        // Only the payload of the proposal we hold is worth serving: the
        // requester verifies against its own proposal's commitment and drops
        // anything else.
        let payload_commitment = consensus.proposal_at(view).and_then(|proposal| {
            match proposal.block_header.payload_commitment() {
                VidCommitment::V2(commitment) => Some(commitment),
                _ => None,
            }
        });
        let payload = payload_commitment.and_then(|commitment| {
            self.retained
                .as_ref()
                .filter(|(v, c, _)| *v == view && *c == commitment)
                .map(|(.., payload)| payload.clone())
                .or_else(|| {
                    self.candidates
                        .get(&view)
                        .filter(|(c, _)| *c == commitment)
                        .map(|(_, payload)| payload.clone())
                })
                .or_else(|| {
                    consensus
                        .built_payload_at(view, commitment)
                        .filter(|payload| payload.txn_bytes() <= self.max_whole_payload_fetch)
                        .map(|payload| payload.encode())
                })
                .filter(|payload| payload.len() <= self.max_whole_payload_fetch)
                .map(|payload| (commitment, payload))
        });

        let Some((payload_commitment, payload)) = payload else {
            self.serve_share(view, requester, consensus, network);
            return;
        };

        self.note_served(view, requester);
        let response = Message {
            sender: self.public_key.clone(),
            message_type: MessageType::PayloadFetch(PayloadFetchMessage::Response(
                PayloadFetchResponse {
                    view,
                    payload_commitment,
                    payload: payload.to_vec(),
                },
            )),
        };

        if let Err(err) = network.unicast(consensus.current_view(), requester, &response) {
            warn!(%view, %err, "network error while sending payload response");
        }
    }

    /// Answer a share fetch by re-unicasting our own `VidShareBroadcast`.
    pub fn serve_share_request(
        &mut self,
        view: ViewNumber,
        requester: &T::SignatureKey,
        view_timeout: Duration,
        consensus: &Consensus<T>,
        network: &Sender<T>,
    ) {
        if !self.may_serve(view, requester, view_timeout) {
            return;
        }
        self.serve_share(view, requester, consensus, network);
    }

    /// Honest requesters pace themselves to half the view timeout per view,
    /// so serving refuses anything faster: a signature proves who is asking,
    /// not how often, and a whole-payload response is worth megabytes.
    fn may_serve(
        &self,
        view: ViewNumber,
        requester: &T::SignatureKey,
        view_timeout: Duration,
    ) -> bool {
        // Slightly under the requester's interval: this side measures
        // receipts while the requester paces sends, so an honest retry can
        // arrive early by the difference in network latency.
        let interval = Self::retry_interval(view_timeout) * 9 / 10;
        self.served
            .get(&(view, requester.clone()))
            .is_none_or(|last| Instant::now().duration_since(*last) >= interval)
    }

    /// Record a send to `requester`, starting its pacing interval. Recorded
    /// only when something is actually sent: the view in a request is
    /// attacker-chosen, and recording requests for views this node holds
    /// nothing of would grow `served` without bound.
    fn note_served(&mut self, view: ViewNumber, requester: &T::SignatureKey) {
        self.served
            .insert((view, requester.clone()), Instant::now());
    }

    /// Re-unicast our own `VidShareBroadcast` for `view`.
    fn serve_share(
        &mut self,
        view: ViewNumber,
        requester: &T::SignatureKey,
        consensus: &Consensus<T>,
        network: &Sender<T>,
    ) {
        let Some(share) = consensus.vid_share_at(view) else {
            return;
        };
        self.note_served(view, requester);

        let message = Message {
            sender: self.public_key.clone(),
            message_type: MessageType::Consensus(ConsensusMessage::VidShareBroadcast(
                share.clone(),
            )),
        };

        if let Err(err) = network.unicast(consensus.current_view(), requester, &message) {
            warn!(%view, %err, "network error while sending own share");
        }
    }

    /// Take in a payload response.
    ///
    /// Hand it to the reconstructor for verification against the commitment
    /// of our own validated proposal. A provably bad answer from the peer the
    /// request went to — a mismatched commitment here, or bytes the
    /// reconstructor rejects — consumes the pending entry, so the next
    /// request round fires immediately against a fresh target instead of
    /// waiting out the retry interval.
    pub fn handle_response(
        &mut self,
        response: PayloadFetchResponse,
        sender: &T::SignatureKey,
        consensus: &Consensus<T>,
        membership: &EpochMembershipCoordinator<T>,
        reconstructor: &mut VidReconstructor<T>,
    ) {
        let view = response.view;

        // The commitment worth anything is the one our own validated proposal
        // pins; the request was keyed by it, and only the peer the request
        // went to may answer.
        let Some(proposal) = consensus.proposal_at(view) else {
            return;
        };

        let VidCommitment::V2(payload_commitment) = proposal.block_header.payload_commitment()
        else {
            return;
        };

        if let Some((_, Some(target))) = self.requested.get(&(view, payload_commitment))
            && target == sender
        {
            self.requested.remove(&(view, payload_commitment));
        } else {
            return;
        }

        if payload_commitment != response.payload_commitment {
            return;
        }

        let epoch = proposal.epoch;
        let metadata = proposal.block_header.metadata().clone();
        let Some(param) = expected_vid_param(membership, Some(epoch)) else {
            warn!(%view, "no VID param for fetched payload; dropping");
            return;
        };

        reconstructor.handle_fetched_payload(
            view,
            epoch,
            payload_commitment,
            metadata,
            param,
            response.payload,
        );
    }

    pub fn gc(&mut self, view: ViewNumber) {
        self.requested.retain(|(v, _), _| *v > view);
        self.requested_proposals.retain(|key, _| key.view > view);
        self.served.retain(|(v, _), _| *v > view);
        self.advertisers = self.advertisers.split_off(&(view + 1));
        if self.retained.as_ref().is_some_and(|(v, ..)| *v <= view) {
            self.retained = None;
        }
        self.candidates = self.candidates.split_off(&(view + 1));
    }
}

#[cfg(test)]
mod tests {
    use hotshot::types::BLSPubKey;
    use hotshot_example_types::{
        block_types::{TestBlockPayload, TestTransaction},
        node_types::TestTypes,
    };
    use hotshot_types::{
        data::ViewNumber,
        traits::{metrics::NoMetrics, signature_key::SignatureKey},
    };

    use super::*;
    use crate::{
        helpers::{proposal_commitment, test_upgrade_lock},
        network::Cliquenet,
        tests::common::utils::{ConsensusHarness, TestData, mock_membership},
        vid::VidReconstructErrorKind,
    };

    fn key(index: u64) -> BLSPubKey {
        BLSPubKey::generated_from_seed_indexed([0u8; 32], index).0
    }

    fn fetcher() -> Fetcher<TestTypes> {
        Fetcher::new(key(0), 5 * 1024 * 1024)
    }

    /// A peerless localhost network: sends to unknown targets are dropped by
    /// the wrapper, so the tests observe the fetcher's decisions through its
    /// own state rather than the wire.
    async fn network() -> Cliquenet<TestTypes> {
        let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([0u8; 32], 0);
        let keypair = hotshot_types::x25519::Keypair::derive_from::<BLSPubKey>(&private_key)
            .expect("keypair derivation should succeed");
        let port =
            test_utils::reserve_tcp_port().expect("OS should have ephemeral ports available");
        let addr = hotshot_types::addr::NetAddr::Inet(std::net::Ipv4Addr::LOCALHOST.into(), port);
        Cliquenet::create(
            "fetch-tests",
            public_key,
            keypair,
            addr,
            vec![],
            test_upgrade_lock(),
            Box::new(NoMetrics),
        )
        .await
        .expect("cliquenet creation should succeed")
    }

    /// The retained payload tracks the lock, not reconstruction order.
    ///
    /// During a stall the later rounds' blocks keep reconstructing while the
    /// locks stay pinned, so retention keyed to reconstruction would evict
    /// the one payload recovery asks for.
    #[tokio::test]
    async fn retained_payload_tracks_the_lock() {
        let test_data = TestData::new(3).await;
        let payload = |i: usize| {
            let proposal = &test_data.views[i].proposal.data;
            (
                proposal.view_number,
                test_data.views[i].vid_commitment(),
                TestBlockPayload::genesis(),
            )
        };
        let mut fetcher = fetcher();

        let (view_1, commitment_1, payload_1) = payload(0);
        fetcher.retain_payload(view_1, commitment_1, &payload_1);
        fetcher.note_locked(Some(view_1));
        assert_eq!(fetcher.retained.as_ref().map(|(v, ..)| *v), Some(view_1));

        // The stall: views 2 and 3 reconstruct but never certify, so the lock
        // stays at view 1 and the retained payload must too.
        for i in 1..3 {
            let (view, commitment, payload) = payload(i);
            fetcher.retain_payload(view, commitment, &payload);
            fetcher.note_locked(Some(view_1));
        }
        assert_eq!(
            fetcher.retained.as_ref().map(|(v, ..)| *v),
            Some(view_1),
            "stalled reconstructions must not evict the locked block"
        );
        assert_eq!(
            fetcher.candidates.keys().max().copied(),
            Some(test_data.views[2].view_number)
        );

        // The window keeps earlier candidates around, so a lock arriving
        // after later reconstructions can still be promoted.
        assert!(fetcher.candidates.contains_key(&view_1));

        // Once the lock catches up, so does the retained payload; a decide
        // clears both.
        let view_3 = test_data.views[2].view_number;
        fetcher.note_locked(Some(view_3));
        assert_eq!(fetcher.retained.as_ref().map(|(v, ..)| *v), Some(view_3));
        fetcher.gc(view_3);
        assert!(fetcher.retained.is_none());
        assert!(fetcher.candidates.is_empty());
    }

    /// Only the peer a request went to can consume it, and a provably bad
    /// answer from that peer re-arms the retry immediately.
    #[tokio::test]
    async fn response_is_bound_to_the_requested_peer() {
        let test_data = TestData::new(1).await;
        let view = test_data.views[0].view_number;
        let commitment = test_data.views[0].vid_commitment();
        let target = key(1);

        let mut harness = ConsensusHarness::new(0).await;
        harness
            .apply_pair(test_data.views[0].proposal_input_consensus(&key(0)))
            .await;

        let membership = mock_membership();
        let mut reconstructor = VidReconstructor::new();
        let mut fetcher = fetcher();
        fetcher
            .requested
            .insert((view, commitment), (Instant::now(), Some(target)));

        // An unsolicited response, valid-looking or not, is ignored and the
        // pending entry survives.
        fetcher.handle_response(
            PayloadFetchResponse {
                view,
                payload_commitment: commitment,
                payload: vec![0xb7; 8],
            },
            &key(2),
            &harness.consensus,
            &membership,
            &mut reconstructor,
        );
        assert!(fetcher.requested.contains_key(&(view, commitment)));

        // Garbage bytes from the requested peer consume the entry (the next
        // round re-targets immediately) and are rejected by verification.
        fetcher.handle_response(
            PayloadFetchResponse {
                view,
                payload_commitment: commitment,
                payload: vec![0xb7; 8],
            },
            &target,
            &harness.consensus,
            &membership,
            &mut reconstructor,
        );
        assert!(!fetcher.requested.contains_key(&(view, commitment)));
        let result = tokio::time::timeout(Duration::from_secs(5), reconstructor.next())
            .await
            .expect("verification should complete in time")
            .expect("should produce a result");
        match result {
            Ok(_) => panic!("garbage bytes must not verify"),
            Err(err) => {
                assert_eq!(err.kind, VidReconstructErrorKind::FetchedPayloadMismatch);
            },
        }
    }

    /// Serving refuses repeats within the pacing interval, per requester,
    /// and records nothing until something is actually sent — the view in a
    /// request is attacker-chosen, and unserved views must leave no state.
    #[test]
    fn serving_is_paced_per_requester() {
        let mut fetcher = fetcher();
        let view = ViewNumber::new(1);
        let timeout = Duration::from_secs(60);

        assert!(fetcher.may_serve(view, &key(1), timeout));
        assert!(
            fetcher.served.is_empty(),
            "a request that served nothing leaves no state"
        );

        fetcher.note_served(view, &key(1));
        assert!(
            !fetcher.may_serve(view, &key(1), timeout),
            "a repeat within the interval is refused"
        );
        assert!(
            fetcher.may_serve(view, &key(2), timeout),
            "another requester has its own budget"
        );
        assert!(
            fetcher.may_serve(view + 1, &key(1), timeout),
            "another view has its own budget"
        );

        fetcher.gc(view + 1);
        assert!(fetcher.served.is_empty());
    }

    /// The request path picks whole-payload fetching when this node's own
    /// share sizes the payload within the threshold, prefers an advertiser as
    /// the target, and never targets itself.
    #[tokio::test]
    async fn request_fetches_small_payloads_whole_from_an_advertiser() {
        let test_data = TestData::new(1).await;
        let view = test_data.views[0].view_number;
        let commitment = test_data.views[0].vid_commitment();

        let mut harness = ConsensusHarness::new(0).await;
        harness
            .apply_pair(test_data.views[0].proposal_input_consensus(&key(0)))
            .await;

        let membership = mock_membership();
        let network = network().await;
        let timeout = Duration::from_secs(60);

        // Both advertisers are known; this node itself must be skipped.
        {
            let mut fetcher = fetcher();
            fetcher.note_advertiser(view, key(0));
            fetcher.note_advertiser(view, key(3));
            fetcher
                .request(
                    view,
                    commitment,
                    timeout,
                    &harness.consensus,
                    &membership,
                    network.sender(),
                )
                .expect("request should succeed");
            let target = match fetcher.requested.get(&(view, commitment)) {
                Some((_, Some(target))) => *target,
                other => panic!("expected a whole-payload attempt, got {other:?}"),
            };
            assert_eq!(target, key(3), "the advertiser is preferred, self skipped");
        }

        // Without advertisers the target is any committee member but self.
        let mut fetcher = fetcher();
        fetcher
            .request(
                view,
                commitment,
                timeout,
                &harness.consensus,
                &membership,
                network.sender(),
            )
            .expect("request should succeed");
        let target = match fetcher.requested.get(&(view, commitment)) {
            Some((_, Some(target))) => *target,
            other => panic!("expected a whole-payload attempt, got {other:?}"),
        };
        assert_ne!(target, key(0), "this node never fetches from itself");
    }

    /// The size threshold is inclusive, and a payload whose size is unknown
    /// because this node holds no share of its own goes through the share
    /// path. (The over-threshold direction of the comparison is not reachable
    /// with this fixture: its blocks are empty, and a zero-size payload fits
    /// every threshold.)
    #[tokio::test]
    async fn request_falls_back_to_shares() {
        let test_data = TestData::new(1).await;
        let view = test_data.views[0].view_number;
        let commitment = test_data.views[0].vid_commitment();

        let membership = mock_membership();
        let network = network().await;
        let timeout = Duration::from_secs(60);

        // The boundary: the fixture's empty payload has size zero, which fits
        // even a zero threshold, so the comparison being inclusive routes it
        // whole.
        {
            let mut harness = ConsensusHarness::new(0).await;
            harness
                .apply_pair(test_data.views[0].proposal_input_consensus(&key(0)))
                .await;
            let size = harness
                .consensus
                .vid_share_at(view)
                .expect("own share is held")
                .common
                .payload_byte_len();
            assert_eq!(size, 0, "the fixture's blocks are empty");
            let mut fetcher = Fetcher::<TestTypes>::new(key(0), 0);
            fetcher
                .request(
                    view,
                    commitment,
                    timeout,
                    &harness.consensus,
                    &membership,
                    network.sender(),
                )
                .expect("request should succeed");
            assert!(
                matches!(
                    fetcher.requested.get(&(view, commitment)),
                    Some((_, Some(_)))
                ),
                "a payload exactly at the threshold is fetched whole"
            );
        }

        // Unknown size: the proposal was fetched, so this node has no share
        // of its own to read the size from.
        let mut harness = ConsensusHarness::new(0).await;
        harness
            .apply(crate::consensus::ConsensusInput::FetchedProposal(
                test_data.views[0].proposal_message(),
            ))
            .await;
        let mut fetcher = fetcher();
        fetcher
            .request(
                view,
                commitment,
                timeout,
                &harness.consensus,
                &membership,
                network.sender(),
            )
            .expect("request should succeed");
        assert!(
            matches!(fetcher.requested.get(&(view, commitment)), Some((_, None))),
            "without a share of our own the size is unknown and shares work at any size"
        );
    }

    /// A repeated request within the interval changes nothing; once the
    /// interval has passed the attempt is repeated.
    #[tokio::test]
    async fn request_is_paced_per_interval() {
        let test_data = TestData::new(1).await;
        let view = test_data.views[0].view_number;
        let commitment = test_data.views[0].vid_commitment();

        let mut harness = ConsensusHarness::new(0).await;
        harness
            .apply_pair(test_data.views[0].proposal_input_consensus(&key(0)))
            .await;

        let membership = mock_membership();
        let network = network().await;
        let timeout = Duration::from_secs(60);

        let mut fetcher = fetcher();
        fetcher
            .request(
                view,
                commitment,
                timeout,
                &harness.consensus,
                &membership,
                network.sender(),
            )
            .expect("request should succeed");
        let first = fetcher.requested.get(&(view, commitment)).unwrap().0;

        fetcher
            .request(
                view,
                commitment,
                timeout,
                &harness.consensus,
                &membership,
                network.sender(),
            )
            .expect("request should succeed");
        assert_eq!(
            fetcher.requested.get(&(view, commitment)).unwrap().0,
            first,
            "a repeat within the interval does not fire"
        );

        // Rewind the recorded attempt past the interval; the next request
        // fires again.
        fetcher.requested.get_mut(&(view, commitment)).unwrap().0 =
            first - Fetcher::<TestTypes>::retry_interval(timeout);
        fetcher
            .request(
                view,
                commitment,
                timeout,
                &harness.consensus,
                &membership,
                network.sender(),
            )
            .expect("request should succeed");
        assert!(
            fetcher.requested.get(&(view, commitment)).unwrap().0 > first,
            "past the interval the attempt is repeated"
        );
    }
    /// The size pre-filter keeps oversized payloads out of retention.
    #[test]
    fn oversized_payloads_are_not_retained() {
        let mut fetcher = Fetcher::<TestTypes>::new(key(0), 8);
        let oversized = TestBlockPayload {
            transactions: vec![TestTransaction::new(vec![0; 16])],
        };
        fetcher.retain_payload(ViewNumber::new(1), VidCommitment2::default(), &oversized);
        assert!(fetcher.candidates.is_empty());

        let small = TestBlockPayload {
            transactions: vec![TestTransaction::new(vec![0; 4])],
        };
        fetcher.retain_payload(ViewNumber::new(2), VidCommitment2::default(), &small);
        assert!(fetcher.candidates.contains_key(&ViewNumber::new(2)));
    }

    /// A proposal request outlives responses that do not validate: receipt
    /// only peeks, and the request is consumed once a response validated —
    /// otherwise one bad response racing the broadcast would burn the round
    /// for the honest copies behind it.
    #[tokio::test]
    async fn proposal_requests_survive_until_a_response_validates() {
        let test_data = TestData::new(1).await;
        let view = test_data.views[0].view_number;
        let commit = proposal_commitment(&test_data.views[0].proposal.data);

        let mut fetcher = fetcher();
        fetcher
            .requested_proposals
            .insert(ProposalFetchKey::new(view, commit), Instant::now());

        assert!(fetcher.is_requested_proposal(view, commit));
        assert!(
            fetcher.is_requested_proposal(view, commit),
            "peeking consumes nothing"
        );
        assert!(fetcher.take_requested_proposal(view, commit));
        assert!(!fetcher.is_requested_proposal(view, commit));
        assert!(!fetcher.take_requested_proposal(view, commit));
    }

    /// The serve chain answers whole from the retained slot, and a request
    /// for a view this node holds nothing of sends nothing and leaves no
    /// state. The proposal here was fetched, so the node holds no share of
    /// its own: a recorded send can only have been the whole-payload path.
    #[tokio::test]
    async fn serve_payload_answers_from_the_retained_slot() {
        let test_data = TestData::new(1).await;
        let view = test_data.views[0].view_number;
        let commitment = test_data.views[0].vid_commitment();

        let mut harness = ConsensusHarness::new(0).await;
        harness
            .apply(crate::consensus::ConsensusInput::FetchedProposal(
                test_data.views[0].proposal_message(),
            ))
            .await;

        let network = network().await;
        let timeout = Duration::from_secs(60);

        {
            let mut fetcher = fetcher();
            fetcher.retain_payload(view, commitment, &TestBlockPayload::genesis());
            fetcher.note_locked(Some(view));
            fetcher.serve_payload(view, &key(1), timeout, &harness.consensus, network.sender());
            assert!(
                fetcher.served.contains_key(&(view, key(1))),
                "the retained payload was served whole"
            );
        }

        let mut fetcher = fetcher();
        fetcher.serve_payload(view, &key(1), timeout, &harness.consensus, network.sender());
        assert!(
            fetcher.served.is_empty(),
            "with neither payload nor share held, nothing is sent and no state is kept"
        );
    }
}
