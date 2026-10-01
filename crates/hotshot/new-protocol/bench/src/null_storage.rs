//! No-op DA storage for the benchmark.
//!
//! `TestStorage` retains every DA proposal — including the multi-megabyte block
//! payload — for the whole run: the `Storage` wrapper's `gc` only aborts
//! in-flight write tasks, it never prunes the inner store, so memory grows
//! without bound (~payload_size × views).
//!
//! The persistence *confirmations* consensus waits on (`Action`, `HighQc`,
//! `Proposal`) are emitted by the `Storage` wrapper itself, independent of the
//! inner store, so dropping the DA payloads is safe. `NullStorage` delegates
//! everything to a real `TestStorage` — so all small state and reads stay
//! correct (the bench never restarts, but reads still behave) — and drops only
//! `append_da`/`append_da2`.

use anyhow::Result;
use async_trait::async_trait;
use hotshot_example_types::storage_types::TestStorage;
use hotshot_new_protocol::{
    message::{Certificate1, Certificate2},
    storage::NewProtocolStorage,
};
use hotshot_types::{
    data::{
        DaProposal, DaProposal2, EpochNumber, QuorumProposal, QuorumProposal2, VidCommitment,
        VidDisperseShare, ViewNumber,
    },
    drb::{DrbInput, DrbResult},
    event::HotShotAction,
    message::Proposal,
    simple_certificate::{
        LightClientStateUpdateCertificateV2, NextEpochQuorumCertificate2, QuorumCertificate,
        QuorumCertificate2, UpgradeCertificate,
    },
    traits::{node_implementation::NodeType, storage::Storage},
};

/// Storage that drops DA payloads but delegates all other state to a real
/// `TestStorage`. See the module docs.
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
    // --- DA: dropped. This is the whole point. ---
    async fn append_da(&self, _: &Proposal<T, DaProposal<T>>, _: VidCommitment) -> Result<()> {
        Ok(())
    }
    async fn append_da2(&self, _: &Proposal<T, DaProposal2<T>>, _: VidCommitment) -> Result<()> {
        Ok(())
    }

    // --- everything else: delegate to the real store (small data). ---
    async fn append_vid(&self, proposal: &Proposal<T, VidDisperseShare<T>>) -> Result<()> {
        self.0.append_vid(proposal).await
    }
    async fn append_proposal(&self, proposal: &Proposal<T, QuorumProposal<T>>) -> Result<()> {
        self.0.append_proposal(proposal).await
    }
    async fn append_proposal2(&self, proposal: &Proposal<T, QuorumProposal2<T>>) -> Result<()> {
        self.0.append_proposal2(proposal).await
    }
    async fn record_action(
        &self,
        view: ViewNumber,
        epoch: Option<EpochNumber>,
        action: HotShotAction,
    ) -> Result<()> {
        self.0.record_action(view, epoch, action).await
    }
    async fn update_high_qc(&self, high_qc: QuorumCertificate<T>) -> Result<()> {
        self.0.update_high_qc(high_qc).await
    }
    async fn update_state_cert(
        &self,
        state_cert: LightClientStateUpdateCertificateV2<T>,
    ) -> Result<()> {
        self.0.update_state_cert(state_cert).await
    }
    async fn update_next_epoch_high_qc2(
        &self,
        next_epoch_high_qc: NextEpochQuorumCertificate2<T>,
    ) -> Result<()> {
        self.0.update_next_epoch_high_qc2(next_epoch_high_qc).await
    }
    async fn update_eqc(
        &self,
        high_qc: QuorumCertificate2<T>,
        next_epoch_high_qc: NextEpochQuorumCertificate2<T>,
    ) -> Result<()> {
        self.0.update_eqc(high_qc, next_epoch_high_qc).await
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
    async fn append_cert2(&self, view: ViewNumber, cert: Certificate2<T>) -> Result<()> {
        self.0.append_cert2(view, cert).await
    }
    async fn append_high_qc2(&self, high_qc: Certificate1<T>) -> Result<()> {
        self.0.append_high_qc2(high_qc).await
    }
    async fn load_high_qc2(&self) -> Result<Option<Certificate1<T>>> {
        self.0.load_high_qc2().await
    }
}
