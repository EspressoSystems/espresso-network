use std::collections::{BTreeMap, HashMap, HashSet};

use hotshot::traits::BlockPayload;
use hotshot_types::{
    data::{VidCommitment, VidCommitment2, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    traits::{
        block_contents::BlockHeader, node_implementation::NodeType, signature_key::SignatureKey,
    },
};
use tokio::task::JoinSet;
use tracing::{error, warn};

use crate::{
    consensus::Consensus,
    coordinator::error::CoordinatorError,
    message::{Message, MessageType, PayloadFetchMessage, PayloadFetchResponse},
    network::Sender,
    vid::{VidReconstructOutput, expected_vid_param, matches_commitment},
};

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

    /// Verifications of received payloads, surfaced by [`Fetcher::next`].
    tasks: JoinSet<Option<VidReconstructOutput<T>>>,
}

impl<T: NodeType> Fetcher<T> {
    pub fn new(public_key: T::SignatureKey) -> Self {
        Self {
            public_key,
            requested: HashMap::new(),
            advertisers: BTreeMap::new(),
            tasks: JoinSet::new(),
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

    /// Verify a peer's response to a request we made, and yield the payload
    /// from [`Self::next`] once it checks out.
    ///
    /// Nothing the peer sent is trusted but the bytes, and those only if they
    /// re-commit: the commitment, metadata and epoch come from our own
    /// validated proposal and the erasure parameters from the committee. A
    /// peer we asked can at worst make us verify once, since answering
    /// consumes its slot in the request.
    pub fn accept_response(
        &mut self,
        response: PayloadFetchResponse,
        sender: &T::SignatureKey,
        consensus: &Consensus<T>,
        membership: &EpochMembershipCoordinator<T>,
    ) {
        let view = response.view;

        let Some(proposal) = consensus.proposal_at(view) else {
            return;
        };

        let VidCommitment::V2(payload_commitment) = proposal.block_header.payload_commitment()
        else {
            return;
        };

        if payload_commitment != response.payload_commitment {
            warn!(%view, "payload response does not match the proposal's commitment");
            return;
        }

        if !self
            .requested
            .get(&(view, payload_commitment))
            .is_some_and(|peers| peers.contains(sender))
        {
            warn!(%view, "payload response from a peer we did not ask");
            return;
        }

        let Some(param) = expected_vid_param(membership, Some(proposal.epoch)) else {
            warn!(%view, "no VID param for fetched payload; dropping");
            return;
        };

        if let Some(peers) = self.requested.get_mut(&(view, payload_commitment)) {
            peers.remove(sender);
            if peers.is_empty() {
                self.requested.remove(&(view, payload_commitment));
            }
        }

        let epoch = proposal.epoch;
        let metadata = proposal.block_header.metadata().clone();
        let bytes = response.payload;

        self.tasks.spawn_blocking(move || {
            if !matches_commitment::<T>(view, &param, &metadata, &bytes, &payload_commitment) {
                return None;
            }
            let payload = T::BlockPayload::from_bytes(&bytes, &metadata);
            let tx_commitments = payload.transaction_commitments(&metadata);
            Some(VidReconstructOutput {
                view,
                epoch,
                payload_commitment,
                payload,
                metadata,
                tx_commitments,
            })
        });
    }

    pub async fn next(&mut self) -> Option<VidReconstructOutput<T>> {
        loop {
            match self.tasks.join_next().await? {
                Ok(Some(out)) => return Some(out),
                Ok(None) => continue,
                Err(err) => {
                    if err.is_panic() {
                        error!(%err, "fetched payload verification panicked");
                    }
                },
            }
        }
    }

    pub fn gc(&mut self, view: ViewNumber) {
        self.requested.retain(|(v, _), _| *v > view);
        self.advertisers = self.advertisers.split_off(&(view + 1));
    }
}
