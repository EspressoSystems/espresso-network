//! Peer-based recovery of block payloads for the decide pipeline.
//!
//! Under the new protocol a node can decide a view without ever obtaining its payload:
//! payloads are reconstructed from VID shares carried by Vote1 broadcasts, and a node
//! whose vote is not needed for quorum (or that was restarted mid-view) may miss them
//! entirely. The decide processor uses [`PayloadRecovery`] to fetch the DA proposal from
//! peers — who retain DA proposals for their consensus storage retention window — and
//! verifies the payload against the block header's payload commitment before trusting it.

use std::time::Duration;

use anyhow::{Context, bail, ensure};
use async_trait::async_trait;
use espresso_types::{
    Leaf2, PubKey, SeqTypes,
    v0::traits::{DecidePayloadRecovery, SequencerPersistence},
};
use hotshot::traits::NodeImplementation;
use hotshot_types::{
    data::{DaProposal2, VidCommitment, vid_commitment, vid_disperse::vid_total_weight},
    epoch_membership::EpochMembershipCoordinator,
    message::Proposal,
    traits::{EncodeBytes, network::ConnectedNetwork},
};
use request_response::RequestType;
use tokio::time::timeout;

use super::{
    RequestResponseProtocol,
    request::{Request, Response},
};

/// How long to wait for a single payload-recovery request before giving up. A failed
/// recovery is retried on later decide processing passes, up to a bounded number of
/// attempts (see `MAX_PAYLOAD_RECOVERY_ATTEMPTS`).
const RECOVERY_TIMEOUT: Duration = Duration::from_secs(15);

/// Fetches DA proposals (block payloads) from peers over the request-response protocol
/// for views that were decided before this node obtained their payload. Responses are
/// verified against the block header's payload commitment, recomputing the VID commitment
/// with the same parameters the disperser used.
pub struct PayloadRecovery<I, N, P>
where
    I: NodeImplementation<SeqTypes>,
    N: ConnectedNetwork<PubKey>,
    P: SequencerPersistence,
{
    protocol: RequestResponseProtocol<I, N, P>,
    membership: EpochMembershipCoordinator<SeqTypes>,
    epoch_height: u64,
}

impl<I, N, P> PayloadRecovery<I, N, P>
where
    I: NodeImplementation<SeqTypes>,
    N: ConnectedNetwork<PubKey>,
    P: SequencerPersistence,
{
    pub fn new(
        protocol: RequestResponseProtocol<I, N, P>,
        membership: EpochMembershipCoordinator<SeqTypes>,
        epoch_height: u64,
    ) -> Self {
        Self {
            protocol,
            membership,
            epoch_height,
        }
    }
}

impl<I, N, P> std::fmt::Debug for PayloadRecovery<I, N, P>
where
    I: NodeImplementation<SeqTypes>,
    N: ConnectedNetwork<PubKey>,
    P: SequencerPersistence,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PayloadRecovery")
            .field("epoch_height", &self.epoch_height)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl<I, N, P> DecidePayloadRecovery for PayloadRecovery<I, N, P>
where
    I: NodeImplementation<SeqTypes>,
    N: ConnectedNetwork<PubKey>,
    P: SequencerPersistence,
{
    async fn recover_payload(
        &self,
        leaf: &Leaf2,
    ) -> anyhow::Result<Option<Proposal<SeqTypes, DaProposal2<SeqTypes>>>> {
        let header = leaf.block_header();
        let expected = header.payload_commitment();
        // Recovery is only supported for new-protocol (V2) commitments; older versions
        // received payloads via DA proposal broadcast before voting, so they don't hit
        // the missing-payload path in practice.
        if !matches!(expected, VidCommitment::V2(_)) {
            return Ok(None);
        }
        let view = leaf.view_number();

        // Derive the VID parameters exactly as the disperser did — from the leaf epoch's
        // stake table — so the recomputed commitment matches.
        let epoch = leaf.epoch(self.epoch_height);
        let total_weight = vid_total_weight::<SeqTypes, _>(
            self.membership
                .stake_table_for_epoch(epoch)
                .map_err(|err| {
                    anyhow::anyhow!("failed to get stake table for epoch {epoch:?}: {err:#}")
                })?
                .stake_table(),
            epoch,
        );

        let version = header.version();
        let ns_table = header.ns_table().clone();

        let result = timeout(
            RECOVERY_TIMEOUT,
            self.protocol.request_indefinitely(
                Request::DaProposal(view.u64()),
                RequestType::Batched,
                move |_req, response| {
                    let ns_table = ns_table.clone();
                    async move {
                        let Response::DaProposal(proposal) = response else {
                            bail!("unexpected response type");
                        };
                        ensure!(
                            proposal.data.view_number == view,
                            "DA proposal response for wrong view"
                        );
                        ensure!(
                            proposal.data.metadata == ns_table,
                            "namespace table mismatch in DA proposal response"
                        );
                        let computed = vid_commitment(
                            &proposal.data.encoded_transactions,
                            &proposal.data.metadata.encode(),
                            total_weight,
                            version,
                        );
                        ensure!(
                            computed == expected,
                            "payload commitment mismatch in DA proposal response"
                        );
                        Ok(*proposal)
                    }
                },
            ),
        )
        .await;

        match result {
            Ok(Ok(proposal)) => Ok(Some(proposal)),
            Ok(Err(err)) => Err(err).context("payload recovery request failed"),
            // Timed out waiting for a valid response; the caller may retry later.
            Err(_) => Ok(None),
        }
    }
}
