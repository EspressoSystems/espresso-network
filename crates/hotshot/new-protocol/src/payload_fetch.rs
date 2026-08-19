//! Recovery of a certified view's payload.
//!
//! A `Cert1` proves a quorum held the view's proposal and enough VID shares
//! to reconstruct its payload, but nothing retransmits the share broadcasts a
//! node missed: proposals and shares are sent once, and the transport's
//! retries are garbage-collected two views behind the frontier. A node that
//! missed them while the network moved on can only pull — and must, because
//! it cannot vote on any proposal extending the certified view without the
//! payload, which stalls certification for good once the faulty nodes leave
//! no vote to spare (`Consensus::request_missing_certified_data`).
//!
//! [`PayloadFetcher`] is that pull. Payloads that fit a message are fetched
//! whole from one randomly chosen peer; larger ones as VID share
//! retransmissions from a random stake-weighted subset — never a broadcast,
//! which would amplify a recovery into a flood. Consensus re-raises the
//! request on every timeout round the payload stays missing, and each attempt
//! after the retry interval picks fresh targets, so a silent or Byzantine
//! target costs one round, not the recovery. Nothing served or received is
//! taken on trust: whole payloads are verified by recomputing the VID
//! commitment the requester's own validated proposal pins, refetched shares
//! go through the ordinary share intake, and a whole-payload response counts
//! only from the transport-authenticated peer it was requested from.

use std::{
    cmp::max,
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};

use alloy::primitives::U256;
use hotshot::traits::BlockPayload;
use hotshot_types::{
    data::{
        EpochNumber, VidCommitment, VidCommitment2, ViewNumber, vid_disperse::vid_total_weight,
    },
    epoch_membership::EpochMembershipCoordinator,
    traits::{
        block_contents::{BlockHeader, EncodeBytes},
        node_implementation::NodeType,
        signature_key::{SignatureKey, StakeTableEntryType},
    },
    vid::avidm_gf2::{AvidmGf2Param, init_avidm_gf2_param},
};
use tracing::warn;

use crate::{
    consensus::Consensus,
    coordinator::error::CoordinatorError,
    message::{ConsensusMessage, Message, MessageType, PayloadFetchMessage, PayloadFetchResponse},
    network::Sender,
    vid::VidReconstructor,
};

type FetchRequests<K> = HashMap<(ViewNumber, VidCommitment2), (Instant, Option<K>)>;

/// Requester- and server-side state of payload recovery.
pub struct PayloadFetcher<T: NodeType> {
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

    /// Peers that advertised a `Qc` for a view as catchup evidence: they lock
    /// what they advertise, and locking requires the reconstructed payload,
    /// so they are the first choice when that payload must be fetched whole.
    advertisers: BTreeMap<ViewNumber, HashSet<T::SignatureKey>>,

    /// The payload retained to serve whole-payload fetches: the highest
    /// view this node reconstructed — the block its lock rests on, or is
    /// about to. One block suffices because recovery only ever needs the
    /// view the network's locks are pinned at.
    reconstructed: Option<(ViewNumber, VidCommitment2, Arc<[u8]>)>,
}

impl<T: NodeType> PayloadFetcher<T> {
    pub fn new(public_key: T::SignatureKey, max_whole_payload_fetch: usize) -> Self {
        Self {
            public_key,
            max_whole_payload_fetch,
            requested: HashMap::new(),
            advertisers: BTreeMap::new(),
            reconstructed: None,
        }
    }

    /// Remember that `peer` advertised the view's certificate.
    pub fn note_advertiser(&mut self, view: ViewNumber, peer: T::SignatureKey) {
        self.advertisers.entry(view).or_default().insert(peer);
    }

    /// Retain a servable copy of a reconstructed payload.
    pub fn retain_payload(
        &mut self,
        view: ViewNumber,
        payload_commitment: VidCommitment2,
        payload: &T::BlockPayload,
    ) {
        if payload.txn_bytes() <= self.max_whole_payload_fetch
            && self.reconstructed.as_ref().is_none_or(|(v, ..)| *v <= view)
        {
            self.reconstructed = Some((view, payload_commitment, payload.encode()));
        }
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

        let retry_interval = max(view_timeout / 2, Duration::from_secs(1));
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

        let mut asked_stake = U256::ZERO;
        let targets: Vec<_> = peers
            .iter()
            .take_while(|(_, stake)| {
                let more = asked_stake * U256::from(2) < total_stake;
                asked_stake += *stake;
                more
            })
            .map(|(peer, _)| peer)
            .collect();

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
        &self,
        view: ViewNumber,
        requester: &T::SignatureKey,
        consensus: &Consensus<T>,
        network: &Sender<T>,
    ) {
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
            self.reconstructed
                .as_ref()
                .filter(|(v, c, _)| *v == view && *c == commitment)
                .map(|(.., payload)| payload.clone())
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

    /// Answer a fetch by re-unicasting our own `VidShareBroadcast`.
    pub fn serve_share(
        &self,
        view: ViewNumber,
        requester: &T::SignatureKey,
        consensus: &Consensus<T>,
        network: &Sender<T>,
    ) {
        let Some(share) = consensus.vid_share_at(view) else {
            return;
        };

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
    /// of our own validated proposal.
    pub fn handle_response(
        &mut self,
        response: PayloadFetchResponse,
        sender: &T::SignatureKey,
        consensus: &Consensus<T>,
        membership: &EpochMembershipCoordinator<T>,
        reconstructor: &mut VidReconstructor<T>,
    ) {
        let view = response.view;
        let key = (view, response.payload_commitment);

        if let Some((_, Some(target))) = self.requested.get(&key)
            && target == sender
        {
            self.requested.remove(&key);
        } else {
            return;
        }

        let Some(proposal) = consensus.proposal_at(view) else {
            return;
        };

        let VidCommitment::V2(payload_commitment) = proposal.block_header.payload_commitment()
        else {
            return;
        };

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
        self.advertisers = self.advertisers.split_off(&(view + 1));
        if self
            .reconstructed
            .as_ref()
            .is_some_and(|(v, ..)| *v <= view)
        {
            self.reconstructed = None;
        }
    }
}

/// The VID erasure parameters the committee fixes for `target_epoch`,
/// matching what an honest disperser derives. Used to reject shares whose
/// `common.param` is forged (the commitment binds `ns_commits`, not `param`)
/// and to verify payloads fetched whole. `None` if the committee cannot be
/// resolved.
pub fn expected_vid_param<T: NodeType>(
    membership: &EpochMembershipCoordinator<T>,
    target_epoch: Option<EpochNumber>,
) -> Option<AvidmGf2Param> {
    let membership = membership.stake_table_for_epoch(target_epoch).ok()?;
    let total_weight = vid_total_weight::<T, _>(membership.stake_table(), target_epoch);
    init_avidm_gf2_param(total_weight).ok()
}
