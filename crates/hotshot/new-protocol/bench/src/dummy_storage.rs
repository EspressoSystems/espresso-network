//! Storage that retains nothing, for the benchmark.
//!
//! The bench deliberately takes persistence out of the measurement. Consensus
//! gates real work on storage confirmations — the proposal is held until
//! `append_proposal` and `record_action(Propose)` confirm, and vote2 waits on
//! `record_action(Vote)` + `append_vid` and on `append_high_qc2` — so whatever
//! backs those calls sets a floor on view latency. A deployed node pays SQL
//! round-trips there; `DummyStorage` pays nothing, which makes the measured
//! latency a lower bound with the storage term removed rather than an estimate
//! of it.
//!
//! This is safe because those confirmations are emitted by the `Storage`
//! wrapper itself, independently of the inner store, and because the
//! coordinator only ever *writes* during a run: the reads (`load_high_qc2`,
//! `load_drb_input`) are restart and epoch-transition paths that a bench with
//! `epoch_height = u64::MAX` never reaches.
//!
//! Retaining nothing also fixes an artifact of the previous `TestStorage`: it
//! kept every DA proposal — block payload included — for the whole run, since
//! the `Storage` wrapper's `gc` only aborts in-flight write tasks and never
//! prunes the inner store. Memory grew as payload_size × views, and the
//! resulting pressure is not something a deployed node experiences.

use anyhow::{Result, bail};
use async_trait::async_trait;
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

/// Storage that accepts every write, keeps none of it, and has nothing to read
/// back. See the module docs.
pub struct DummyStorage<T: NodeType>(std::marker::PhantomData<T>);

impl<T: NodeType> Clone for DummyStorage<T> {
    fn clone(&self) -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<T: NodeType> Default for DummyStorage<T> {
    fn default() -> Self {
        Self(std::marker::PhantomData)
    }
}

#[async_trait]
impl<T: NodeType> Storage<T> for DummyStorage<T> {
    async fn append_da(&self, _: &Proposal<T, DaProposal<T>>, _: VidCommitment) -> Result<()> {
        Ok(())
    }
    async fn append_da2(&self, _: &Proposal<T, DaProposal2<T>>, _: VidCommitment) -> Result<()> {
        Ok(())
    }
    async fn append_vid(&self, _: &Proposal<T, VidDisperseShare<T>>) -> Result<()> {
        Ok(())
    }
    async fn append_proposal(&self, _: &Proposal<T, QuorumProposal<T>>) -> Result<()> {
        Ok(())
    }
    async fn append_proposal2(&self, _: &Proposal<T, QuorumProposal2<T>>) -> Result<()> {
        Ok(())
    }
    async fn record_action(
        &self,
        _: ViewNumber,
        _: Option<EpochNumber>,
        _: HotShotAction,
    ) -> Result<()> {
        Ok(())
    }
    async fn update_high_qc(&self, _: QuorumCertificate<T>) -> Result<()> {
        Ok(())
    }
    async fn update_state_cert(&self, _: LightClientStateUpdateCertificateV2<T>) -> Result<()> {
        Ok(())
    }
    async fn update_next_epoch_high_qc2(&self, _: NextEpochQuorumCertificate2<T>) -> Result<()> {
        Ok(())
    }
    async fn update_eqc(
        &self,
        _: QuorumCertificate2<T>,
        _: NextEpochQuorumCertificate2<T>,
    ) -> Result<()> {
        Ok(())
    }
    async fn update_decided_upgrade_certificate(
        &self,
        _: Option<UpgradeCertificate<T>>,
    ) -> Result<()> {
        Ok(())
    }
    async fn store_drb_result(&self, _: EpochNumber, _: DrbResult) -> Result<()> {
        Ok(())
    }
    async fn store_epoch_root(&self, _: EpochNumber, _: T::BlockHeader) -> Result<()> {
        Ok(())
    }
    async fn store_drb_input(&self, _: DrbInput) -> Result<()> {
        Ok(())
    }

    /// Unreachable in a bench run (`epoch_height = u64::MAX`, so no epoch
    /// transition asks for a DRB input). Fail loudly rather than invent one:
    /// a silent default here would change consensus behaviour, not just timing.
    async fn load_drb_input(&self, epoch: u64) -> Result<DrbInput> {
        bail!("DummyStorage retains nothing; no DRB input for epoch {epoch}")
    }
}

#[async_trait]
impl<T: NodeType> NewProtocolStorage<T> for DummyStorage<T> {
    async fn append_cert2(&self, _: ViewNumber, _: Certificate2<T>) -> Result<()> {
        Ok(())
    }
    async fn append_high_qc2(&self, _: Certificate1<T>) -> Result<()> {
        Ok(())
    }

    /// Restart recovery only; the bench never restarts.
    async fn load_high_qc2(&self) -> Result<Option<Certificate1<T>>> {
        Ok(None)
    }
}
