// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Abstract storage type for storing DA proposals and VID shares
//!
//! This modules provides the [`Storage`] trait.
//!

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use futures::future::BoxFuture;

use super::node_implementation::NodeType;
use crate::{
    data::{
        vid_disperse::{ADVZDisperseShare, VidDisperseShare2},
        DaProposal, DaProposal2, QuorumProposal, QuorumProposal2, QuorumProposalWrapper,
        VidCommitment, VidDisperseShare,
    },
    drb::DrbResult,
    event::HotShotAction,
    message::{convert_proposal, Proposal},
    simple_certificate::{
        LightClientStateUpdateCertificate, NextEpochQuorumCertificate2, QuorumCertificate,
        QuorumCertificate2, UpgradeCertificate,
    },
};

/// Abstraction for storing a variety of consensus payload datum.
#[async_trait]
pub trait Storage<TYPES: NodeType>: Send + Sync + Clone {
    /// Add a proposal to the stored VID proposals.
    async fn append_vid(&self, proposal: &Proposal<TYPES, ADVZDisperseShare<TYPES>>) -> Result<()>;
    /// Add a proposal to the stored VID proposals.
    /// TODO(Chengyu): fix this
    async fn append_vid2(&self, proposal: &Proposal<TYPES, VidDisperseShare2<TYPES>>)
        -> Result<()>;

    async fn append_vid_general(
        &self,
        proposal: &Proposal<TYPES, VidDisperseShare<TYPES>>,
    ) -> Result<()> {
        let signature = proposal.signature.clone();
        match &proposal.data {
            VidDisperseShare::V0(share) => {
                self.append_vid(&Proposal {
                    data: share.clone(),
                    signature,
                    _pd: std::marker::PhantomData,
                })
                .await
            },
            VidDisperseShare::V1(share) => {
                self.append_vid2(&Proposal {
                    data: share.clone(),
                    signature,
                    _pd: std::marker::PhantomData,
                })
                .await
            },
        }
    }
    /// Add a proposal to the stored DA proposals.
    async fn append_da(
        &self,
        proposal: &Proposal<TYPES, DaProposal<TYPES>>,
        vid_commit: VidCommitment,
    ) -> Result<()>;
    /// Add a proposal to the stored DA proposals.
    async fn append_da2(
        &self,
        proposal: &Proposal<TYPES, DaProposal2<TYPES>>,
        vid_commit: VidCommitment,
    ) -> Result<()> {
        self.append_da(&convert_proposal(proposal.clone()), vid_commit)
            .await
    }
    /// Add a proposal we sent to the store
    async fn append_proposal(
        &self,
        proposal: &Proposal<TYPES, QuorumProposal<TYPES>>,
    ) -> Result<()>;
    /// Add a proposal we sent to the store
    async fn append_proposal2(
        &self,
        proposal: &Proposal<TYPES, QuorumProposal2<TYPES>>,
    ) -> Result<()>;
    /// Add a proposal we sent to the store
    async fn append_proposal_wrapper(
        &self,
        proposal: &Proposal<TYPES, QuorumProposalWrapper<TYPES>>,
    ) -> Result<()> {
        self.append_proposal2(&convert_proposal(proposal.clone()))
            .await
    }
    /// Record a HotShotAction taken.
    async fn record_action(
        &self,
        view: TYPES::View,
        epoch: Option<TYPES::Epoch>,
        action: HotShotAction,
    ) -> Result<()>;
    /// Update the current high QC in storage.
    async fn update_high_qc(&self, high_qc: QuorumCertificate<TYPES>) -> Result<()>;
    /// Update the current high QC in storage.
    async fn update_high_qc2(&self, high_qc: QuorumCertificate2<TYPES>) -> Result<()> {
        self.update_high_qc(high_qc.to_qc()).await
    }
    /// Update the light client state update certificate in storage.
    async fn update_state_cert(
        &self,
        state_cert: LightClientStateUpdateCertificate<TYPES>,
    ) -> Result<()>;

    async fn update_high_qc2_and_state_cert(
        &self,
        high_qc: QuorumCertificate2<TYPES>,
        state_cert: LightClientStateUpdateCertificate<TYPES>,
    ) -> Result<()> {
        self.update_high_qc2(high_qc).await?;
        self.update_state_cert(state_cert).await
    }
    /// Update the current high QC in storage.
    async fn update_next_epoch_high_qc2(
        &self,
        _next_epoch_high_qc: NextEpochQuorumCertificate2<TYPES>,
    ) -> Result<()> {
        Ok(())
    }

    /// Upgrade the current decided upgrade certificate in storage.
    async fn update_decided_upgrade_certificate(
        &self,
        decided_upgrade_certificate: Option<UpgradeCertificate<TYPES>>,
    ) -> Result<()>;
    /// Migrate leaves from `Leaf` to `Leaf2`, and proposals from `QuorumProposal` to `QuorumProposal2`
    async fn migrate_consensus(&self) -> Result<()> {
        Ok(())
    }
    /// Add a drb result
    async fn add_drb_result(&self, epoch: TYPES::Epoch, drb_result: DrbResult) -> Result<()>;
    /// Add an epoch block header
    async fn add_epoch_root(
        &self,
        epoch: TYPES::Epoch,
        block_header: TYPES::BlockHeader,
    ) -> Result<()>;
}

pub type StorageAddDrbResultFn<TYPES> = Arc<
    Box<
        dyn Fn(<TYPES as NodeType>::Epoch, DrbResult) -> BoxFuture<'static, Result<()>>
            + Send
            + Sync
            + 'static,
    >,
>;

async fn storage_add_drb_result_impl<TYPES: NodeType>(
    storage: impl Storage<TYPES>,
    epoch: TYPES::Epoch,
    drb_result: DrbResult,
) -> Result<()> {
    storage.add_drb_result(epoch, drb_result).await
}

/// Helper function to create a callback to add a drb result to storage
pub fn storage_add_drb_result<TYPES: NodeType>(
    storage: impl Storage<TYPES> + 'static,
) -> StorageAddDrbResultFn<TYPES> {
    Arc::new(Box::new(move |epoch, drb_result| {
        let st = storage.clone();
        Box::pin(storage_add_drb_result_impl(st, epoch, drb_result))
    }))
}
