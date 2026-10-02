//! Storage that drops DA payloads and delegates everything else to `TestStorage`.
//!
//! `TestStorage` keeps every DA payload for the whole run, so memory grows with
//! payload size times views. Dropping them is safe: the persistence confirmations
//! consensus waits on come from the `Storage` wrapper, not the inner store, and the
//! bench never restarts, so nothing reads DA proposals back.

use anyhow::Result;
use async_trait::async_trait;
use hotshot_example_types::storage_types::TestStorage;
use hotshot_new_protocol::{message::Certificate1, storage::NewProtocolStorage};
use hotshot_types::{
    data::{
        DaProposal, DaProposal2, EpochNumber, QuorumProposalWrapper, VidCommitment,
        VidDisperseShare, ViewNumber,
    },
    drb::{DrbInput, DrbResult},
    event::HotShotAction,
    message::Proposal,
    simple_certificate::{LightClientStateUpdateCertificateV2, UpgradeCertificate},
    traits::{node_implementation::NodeType, storage::Storage},
};

pub struct NullStorage<T: NodeType>(TestStorage<T>);

impl<T: NodeType> Clone for NullStorage<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: NodeType> Default for NullStorage<T> {
    fn default() -> Self {
        Self(TestStorage::default())
    }
}

#[async_trait]
impl<T: NodeType> Storage<T> for NullStorage<T> {
    async fn append_da(&self, _: &Proposal<T, DaProposal<T>>, _: VidCommitment) -> Result<()> {
        Ok(())
    }
    async fn append_da2(&self, _: &Proposal<T, DaProposal2<T>>, _: VidCommitment) -> Result<()> {
        Ok(())
    }

    async fn append_vid(&self, proposal: &Proposal<T, VidDisperseShare<T>>) -> Result<()> {
        self.0.append_vid(proposal).await
    }
    async fn append_proposal_wrapper(
        &self,
        proposal: &Proposal<T, QuorumProposalWrapper<T>>,
    ) -> Result<()> {
        self.0.append_proposal_wrapper(proposal).await
    }
    async fn record_action(&self, view: ViewNumber, action: HotShotAction) -> Result<()> {
        self.0.record_action(view, action).await
    }
    async fn update_state_cert(
        &self,
        state_cert: LightClientStateUpdateCertificateV2<T>,
    ) -> Result<()> {
        self.0.update_state_cert(state_cert).await
    }
    async fn update_decided_upgrade_certificate(
        &self,
        decided_upgrade_certificate: Option<UpgradeCertificate<T>>,
    ) -> Result<()> {
        self.0
            .update_decided_upgrade_certificate(decided_upgrade_certificate)
            .await
    }
    async fn store_drb_result(&self, epoch: EpochNumber, drb_result: DrbResult) -> Result<()> {
        self.0.store_drb_result(epoch, drb_result).await
    }
    async fn store_epoch_root(
        &self,
        epoch: EpochNumber,
        block_header: T::BlockHeader,
    ) -> Result<()> {
        self.0.store_epoch_root(epoch, block_header).await
    }
    async fn store_drb_input(&self, drb_input: DrbInput) -> Result<()> {
        self.0.store_drb_input(drb_input).await
    }
    async fn load_drb_input(&self, epoch: u64) -> Result<DrbInput> {
        self.0.load_drb_input(epoch).await
    }
}

#[async_trait]
impl<T: NodeType> NewProtocolStorage<T> for NullStorage<T> {
    async fn append_high_qc2(&self, high_qc: Certificate1<T>) -> Result<()> {
        self.0.append_high_qc2(high_qc).await
    }
    async fn load_high_qc2(&self) -> Result<Option<Certificate1<T>>> {
        self.0.load_high_qc2().await
    }
}
