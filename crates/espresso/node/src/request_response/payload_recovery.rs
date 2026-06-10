//! Peer-based recovery of block payloads for the decide pipeline.
//!
//! Under the new protocol a node can decide a view without ever obtaining its payload (its vote
//! wasn't needed for quorum, or it restarted mid-view). When the decide processor reports such a
//! leaf, [`PayloadRecovery`] fetches the DA proposal from peers, verifies it against the header's
//! payload commitment, and delivers it (with the recomputed VID common) to consensus storage and
//! the query service.

use std::time::Duration;

use anyhow::{Context, bail, ensure};
use async_trait::async_trait;
use espresso_types::{
    Leaf2, PubKey, SeqTypes,
    v0::traits::{DecidePayloadRecovery, RecoveredPayload, SequencerPersistence},
};
use hotshot::traits::NodeImplementation;
use hotshot_types::{
    data::{VidCommitment, vid_disperse::vid_total_weight},
    epoch_membership::EpochMembershipCoordinator,
    traits::{EncodeBytes, network::ConnectedNetwork},
    vid::avidm_gf2::avidm_gf2_commit,
};
use request_response::RequestType;
use tokio::time::timeout;

use super::{
    RequestResponseProtocol,
    request::{Request, Response},
};

/// How long to wait for a single payload-recovery request before giving up. The caller
/// retries a bounded number of times (see `PAYLOAD_RECOVERY_ATTEMPTS` in the decide
/// processor) before leaving the gap to the query service's own fetching.
const RECOVERY_TIMEOUT: Duration = Duration::from_secs(15);

/// Fetches DA proposals from peers for views decided before this node obtained their payload,
/// verifying each response against the header's payload commitment.
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
    async fn recover_payload(&self, leaf: &Leaf2) -> anyhow::Result<Option<RecoveredPayload>> {
        let header = leaf.block_header();
        let expected = header.payload_commitment();
        // Recovery is only supported for new-protocol (V2) commitments; older versions
        // received payloads via DA proposal broadcast before voting, so they don't hit
        // the missing-payload path in practice.
        if !matches!(expected, VidCommitment::V2(_)) {
            return Ok(None);
        }
        let view = leaf.view_number();

        // Derive the VID parameters from the leaf epoch's stake table, as the disperser did, so
        // the recomputed commitment and VID common match. Wait for catchup if the epoch's
        // snapshot is missing: the synchronous lookup fails instantly in that case, which would
        // burn every recovery attempt before catchup has a chance to land.
        let epoch = leaf.epoch(self.epoch_height);
        let membership = match epoch {
            Some(e) => {
                match timeout(RECOVERY_TIMEOUT, self.membership.wait_for_stake_table(e)).await {
                    Ok(Ok(membership)) => membership,
                    Ok(Err(err)) => {
                        bail!("failed to get stake table for epoch {epoch:?}: {err:#}")
                    },
                    // Catchup didn't finish in time; the caller may retry later.
                    Err(_) => return Ok(None),
                }
            },
            None => self
                .membership
                .stake_table_for_epoch(None)
                .map_err(|err| anyhow::anyhow!("failed to get pre-epoch stake table: {err:#}"))?,
        };
        let total_weight = vid_total_weight::<SeqTypes, _>(membership.stake_table(), epoch);

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
                        // Recompute commitment and VID common; trust the response only if the
                        // commitment matches the header's.
                        let (commit, common) = avidm_gf2_commit(
                            total_weight,
                            &proposal.data.encoded_transactions,
                            &proposal.data.metadata.encode(),
                        )
                        .map_err(|err| {
                            anyhow::anyhow!("failed to compute VID commitment: {err}")
                        })?;
                        ensure!(
                            VidCommitment::V2(commit) == expected,
                            "payload commitment mismatch in DA proposal response"
                        );
                        Ok(RecoveredPayload {
                            proposal: *proposal,
                            vid_common: common,
                        })
                    }
                },
            ),
        )
        .await;

        match result {
            Ok(Ok(recovered)) => Ok(Some(recovered)),
            Ok(Err(err)) => Err(err).context("payload recovery request failed"),
            // Timed out waiting for a valid response; the caller may retry later.
            Err(_) => Ok(None),
        }
    }
}
