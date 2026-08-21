use std::collections::{HashMap, HashSet};

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
    message::{Message, MessageType, PayloadFetchMessage, PayloadFetchResponse, Unavailable},
    network::Sender,
    vid::{ObtainedPayload, expected_vid_param, matches_commitment},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Retry {
    NewRound,
    SameRound,
}

/// The requesting half of payload recovery.
pub struct Fetcher<T: NodeType> {
    public_key: T::SignatureKey,

    /// Outstanding fetches.
    ///
    /// The peers the request went to. Only responses from these peers are
    /// accepted, so an unsolicited garbage response cannot consume the
    /// round a genuine answer is still in flight for.
    requested: HashMap<(ViewNumber, VidCommitment2), HashSet<T::SignatureKey>>,

    /// Verifications of received payloads, surfaced by [`Fetcher::next`].
    tasks: JoinSet<Option<ObtainedPayload<T>>>,
}

impl<T: NodeType> Fetcher<T> {
    pub fn new(public_key: T::SignatureKey) -> Self {
        Self {
            public_key,
            requested: HashMap::new(),
            tasks: JoinSet::new(),
        }
    }

    /// Ask peers for the certified `view`'s payload.
    pub fn request(
        &mut self,
        view: ViewNumber,
        payload_commitment: VidCommitment2,
        retry: Retry,
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

        let mut asked = self.requested.get_mut(&(view, payload_commitment));

        let target = (|| {
            let epoch = consensus.proposal_at(view)?.epoch;
            let membership = membership.stake_table_for_epoch(Some(epoch)).ok()?;
            let stake_table = membership.stake_table();
            if let Some(asked) = &mut asked
                && retry == Retry::NewRound
                && asked.len() + 1 == stake_table.len()
            {
                asked.clear()
            }
            stake_table
                .map(|peer| T::SignatureKey::public_key(&peer.stake_table_entry))
                .filter(|peer| {
                    if *peer == self.public_key {
                        return false;
                    }
                    let Some(asked) = &asked else { return true };
                    !asked.contains(peer)
                })
                .choose(&mut rng)
        })();

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
            Some(ObtainedPayload {
                view,
                epoch,
                payload_commitment,
                payload,
                metadata,
                tx_commitments,
            })
        });
    }

    pub async fn next(&mut self) -> Option<ObtainedPayload<T>> {
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

    pub fn accept_refusal(
        &self,
        view: ViewNumber,
        reason: Unavailable,
        sender: &T::SignatureKey,
    ) -> bool {
        let asked = self
            .requested
            .iter()
            .any(|((asked_view, _), peers)| *asked_view == view && peers.contains(sender));
        if !asked {
            warn!(%view, %sender, "payload refusal from a peer we did not ask");
            return false;
        }
        match reason {
            Unavailable::NotHeld => true,
            Unavailable::TooLarge => {
                warn!(%view, "peer says the payload exceeds the message size limit");
                false
            },
        }
    }

    pub fn gc(&mut self, view: ViewNumber) {
        self.requested.retain(|(v, _), _| *v > view);
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

    use super::{Fetcher, Unavailable};
    use crate::{
        consensus::Consensus,
        message::PayloadFetchResponse,
        tests::common::utils::{ConsensusHarness, TestData},
        vid::expected_vid_param,
    };

    fn key(index: u64) -> BLSPubKey {
        BLSPubKey::generated_from_seed_indexed([0; 32], index).0
    }

    /// A payload, and a proposal at view 1 that commits to it.
    ///
    /// The commitment is computed the way a disperser computes it, from the
    /// committee's erasure parameters, so `accept_response` derives the same
    /// one from the proposal and the stake table.
    async fn payload_and_consensus() -> (Vec<u8>, VidCommitment2, ConsensusHarness) {
        let harness = ConsensusHarness::new(0).await;
        let test_data = TestData::new(1).await;

        let metadata = TestMetadata {
            num_transactions: 1,
        };
        let payload = TestBlockPayload {
            transactions: vec![TestTransaction::new(vec![1, 2, 3])],
        };
        let bytes = payload.encode().to_vec();

        let param = expected_vid_param(
            &harness.membership_coordinator,
            Some(EpochNumber::genesis()),
        )
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

        let mut harness = harness;
        harness
            .consensus
            .force_set_proposal(ViewNumber::new(1), proposal);
        (bytes, commitment, harness)
    }

    fn fetcher_expecting(peer: BLSPubKey, commitment: VidCommitment2) -> Fetcher<TestTypes> {
        let mut fetcher = Fetcher::new(key(0));
        fetcher
            .requested
            .insert((ViewNumber::new(1), commitment), HashSet::from([peer]));
        fetcher
    }

    fn response(commitment: VidCommitment2, payload: Vec<u8>) -> PayloadFetchResponse {
        PayloadFetchResponse {
            view: ViewNumber::new(1),
            payload_commitment: commitment,
            payload,
        }
    }

    async fn accept(
        fetcher: &mut Fetcher<TestTypes>,
        response: PayloadFetchResponse,
        sender: BLSPubKey,
        harness: &ConsensusHarness,
    ) {
        fetcher.accept_response(
            response,
            &sender,
            &harness.consensus as &Consensus<TestTypes>,
            &harness.membership_coordinator,
        );
    }

    /// The payload a peer we asked sends, which re-commits to what our own
    /// proposal names, comes back out of the fetcher.
    #[tokio::test]
    async fn a_payload_that_recommits_is_yielded() {
        let (bytes, commitment, harness) = payload_and_consensus().await;
        let peer = key(1);
        let mut fetcher = fetcher_expecting(peer, commitment);

        accept(&mut fetcher, response(commitment, bytes), peer, &harness).await;

        let obtained = fetcher.next().await.expect("payload is verified");
        assert_eq!(obtained.view, ViewNumber::new(1));
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

        accept(&mut fetcher, response(commitment, bytes), peer, &harness).await;

        assert!(fetcher.next().await.is_none());
    }

    /// A peer we did not ask does not get to spend our verification, even
    /// with a payload that would have verified.
    #[tokio::test]
    async fn an_unsolicited_payload_is_not_verified() {
        let (bytes, commitment, harness) = payload_and_consensus().await;
        let mut fetcher = fetcher_expecting(key(1), commitment);

        accept(&mut fetcher, response(commitment, bytes), key(2), &harness).await;

        assert!(fetcher.next().await.is_none());
        assert!(
            fetcher
                .requested
                .get(&(ViewNumber::new(1), commitment))
                .is_some_and(|peers| peers.contains(&key(1))),
            "the peer we asked keeps its slot"
        );
    }

    /// A response naming a commitment our proposal does not carry is not the
    /// block we asked for.
    #[tokio::test]
    async fn a_response_for_another_commitment_is_dropped() {
        let (bytes, commitment, harness) = payload_and_consensus().await;
        let peer = key(1);
        let mut fetcher = fetcher_expecting(peer, commitment);

        accept(
            &mut fetcher,
            response(VidCommitment2::default(), bytes),
            peer,
            &harness,
        )
        .await;

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
            response(commitment, bytes.clone()),
            peer,
            &harness,
        )
        .await;
        accept(&mut fetcher, response(commitment, bytes), peer, &harness).await;

        assert!(fetcher.next().await.is_some());
        assert!(
            fetcher.next().await.is_none(),
            "the second response was not verified"
        );
    }

    /// A refusal is acted on only if we asked that peer, and only "not me"
    /// is worth turning to another peer for. A claim about the block itself
    /// is not the sender's to make.
    #[test]
    fn only_a_solicited_not_held_refusal_redraws() {
        let commitment = VidCommitment2::default();
        let peer = key(1);
        let fetcher = fetcher_expecting(peer, commitment);
        let view = ViewNumber::new(1);

        assert!(fetcher.accept_refusal(view, Unavailable::NotHeld, &peer));
        assert!(!fetcher.accept_refusal(view, Unavailable::TooLarge, &peer));
        assert!(!fetcher.accept_refusal(view, Unavailable::NotHeld, &key(2)));
        assert!(!fetcher.accept_refusal(ViewNumber::new(2), Unavailable::NotHeld, &peer));
    }
}
