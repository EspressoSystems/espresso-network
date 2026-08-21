use std::collections::{BTreeMap, HashMap, HashSet};

use hotshot::traits::BlockPayload;
use hotshot_types::{
    data::{EpochNumber, VidCommitment, VidCommitment2, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    traits::{
        block_contents::BlockHeader, node_implementation::NodeType, signature_key::SignatureKey,
    },
    vid::avidm_gf2::AvidmGf2Param,
};
use tracing::warn;

use crate::{
    consensus::Consensus,
    coordinator::error::CoordinatorError,
    message::{Message, MessageType, PayloadFetchMessage, PayloadFetchResponse},
    network::Sender,
    vid::expected_vid_param,
};

type Metadata<T> = <<T as NodeType>::BlockPayload as BlockPayload<T>>::Metadata;

/// A payload a peer sent us whole, on its way into
/// [`VidReconstructor::handle_fetched_payload`].
///
/// Only `payload` comes from the peer. Commitment, metadata and epoch are
/// taken from the requester's own validated proposal and `param` from the
/// committee, so the response is checked against what we already believe.
///
/// [`VidReconstructor::handle_fetched_payload`]: crate::vid::VidReconstructor
#[non_exhaustive]
pub struct FetchedPayload<T: NodeType> {
    pub view: ViewNumber,
    pub epoch: EpochNumber,
    pub payload_commitment: VidCommitment2,
    pub metadata: Metadata<T>,
    pub param: AvidmGf2Param,
    pub payload: Vec<u8>,
}

/// Requester- and server-side state of payload recovery.
pub struct Fetcher<T: NodeType> {
    public_key: T::SignatureKey,

    /// Outstanding fetches.
    ///
    /// The peers the request went to. Only responses from these peers are
    /// accepted, so an unsolicited garbage response cannot consume the
    /// round a genuine answer is still in flight for.
    requested: HashMap<(ViewNumber, VidCommitment2), HashSet<T::SignatureKey>>,

    /// Peers that advertised a `Qc` for a view as catchup evidence: they lock
    /// what they advertise, and locking requires the reconstructed payload,
    /// so they are the first choice when that payload must be fetched whole.
    advertisers: BTreeMap<ViewNumber, HashSet<T::SignatureKey>>,
}

impl<T: NodeType> Fetcher<T> {
    pub fn new(public_key: T::SignatureKey) -> Self {
        Self {
            public_key,
            requested: HashMap::new(),
            advertisers: BTreeMap::new(),
        }
    }

    /// Remember that `peer` advertised the view's certificate.
    pub fn note_advertiser(&mut self, view: ViewNumber, peer: T::SignatureKey) {
        self.advertisers.entry(view).or_default().insert(peer);
    }

    /// Ask peers for the certified `view`'s payload.
    pub fn request(
        &mut self,
        view: ViewNumber,
        payload_commitment: VidCommitment2,
        consensus: &Consensus<T>,
        membership: &EpochMembershipCoordinator<T>,
        network: &Sender<T>,
    ) -> Result<(), CoordinatorError> {
        use rand::seq::IteratorRandom;

        let request = consensus.signed_fetch_request(view).map_err(|err| {
            let err = format!("failed to sign payload request: {err}");
            CoordinatorError::regular(err).context("sign payload request")
        })?;

        let mut rng = rand::thread_rng();

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
            .entry((view, payload_commitment))
            .or_default()
            .insert(target.clone());

        let message = Message {
            sender: self.public_key.clone(),
            message_type: MessageType::PayloadFetch(PayloadFetchMessage::Request(request)),
        };

        network
            .unicast(consensus.current_view(), &target, &message)
            .map_err(|err| CoordinatorError::from(err).context("unicast payload request"))
    }

    /// Take a peer's response to a request we made, with what
    /// [`VidReconstructor::handle_fetched_payload`] needs to verify it.
    ///
    /// The commitment, metadata and epoch come from our own validated
    /// proposal and the param from the committee, so nothing the peer sent
    /// but the bytes themselves is carried through.
    ///
    /// [`VidReconstructor::handle_fetched_payload`]: crate::vid::VidReconstructor
    pub fn accept_response(
        &mut self,
        response: PayloadFetchResponse,
        sender: &T::SignatureKey,
        consensus: &Consensus<T>,
        membership: &EpochMembershipCoordinator<T>,
    ) -> Option<FetchedPayload<T>> {
        let view = response.view;

        let proposal = consensus.proposal_at(view)?;

        let VidCommitment::V2(payload_commitment) = proposal.block_header.payload_commitment()
        else {
            return None;
        };

        if payload_commitment != response.payload_commitment {
            warn!(%view, "payload response does not match the proposal's commitment");
            return None;
        }

        if !self
            .requested
            .get(&(view, payload_commitment))
            .is_some_and(|nodes| nodes.contains(sender))
        {
            warn!(%view, "payload response from a peer we did not ask");
            return None;
        }

        let param = expected_vid_param(membership, Some(proposal.epoch)).or_else(|| {
            warn!(%view, "no VID param for fetched payload; dropping");
            None
        })?;

        if let Some(peers) = self.requested.get_mut(&(view, payload_commitment)) {
            peers.remove(sender);
            if peers.is_empty() {
                self.requested.remove(&(view, payload_commitment));
            }
        }

        Some(FetchedPayload {
            view,
            epoch: proposal.epoch,
            payload_commitment,
            metadata: proposal.block_header.metadata().clone(),
            param,
            payload: response.payload,
        })
    }

    pub fn gc(&mut self, view: ViewNumber) {
        self.requested.retain(|(v, _), _| *v > view);
        self.advertisers = self.advertisers.split_off(&(view + 1));
    }
}
