//! Mock implementation of persistence, for testing.
use std::collections::BTreeMap;

use anyhow::bail;
use async_trait::async_trait;
use espresso_types::{
    AuthenticatedValidatorMap, Header, Leaf2, NetworkConfig, PubKey, RegisteredValidatorMap,
    StakeTableHash,
    traits::{EventsPersistenceRead, MembershipPersistence, StakeTuple},
    v0::traits::{EventConsumer, PersistenceOptions, SequencerPersistence, StateCertPersistence},
    v0_3::{EventKey, IndexedStake, RegisteredValidator, RewardAmount, StakeTableEvent},
};
use hotshot_types::{
    data::{
        DaProposal, DaProposal2, EpochNumber, QuorumProposalWrapper, VidCommitment,
        VidDisperseShare,
    },
    drb::{DrbInput, DrbResult},
    event::{HotShotAction, LeafInfo},
    initializer::InitializerEpochInfo,
    message::Proposal,
    new_protocol::CoordinatorEvent,
    simple_certificate::{
        Certificate2, CertificatePair, LightClientStateUpdateCertificateV2,
        NextEpochQuorumCertificate2, QuorumCertificate2, UpgradeCertificate,
    },
    traits::metrics::Metrics,
};

use crate::{SeqTypes, ViewNumber};

#[derive(Clone, Copy, Debug)]
pub struct Options;

#[async_trait]
impl PersistenceOptions for Options {
    type Persistence = NoStorage;

    fn set_view_retention(&mut self, _: u64) {}

    fn set_consensus_only(&mut self) {}

    async fn create(&mut self) -> anyhow::Result<Self::Persistence> {
        Ok(NoStorage)
    }

    async fn reset(self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct NoStorage;

#[async_trait]
impl SequencerPersistence for NoStorage {
    async fn load_config(&self) -> anyhow::Result<Option<NetworkConfig>> {
        Ok(None)
    }

    async fn save_config(&self, _: &NetworkConfig) -> anyhow::Result<()> {
        Ok(())
    }

    async fn persist_decided_leaves(
        &self,
        leaves: impl IntoIterator<Item = (&LeafInfo<SeqTypes>, CertificatePair<SeqTypes>)> + Send,
        _cert2: Option<&Certificate2<SeqTypes>>,
        consumer: &(impl EventConsumer + 'static),
    ) -> anyhow::Result<()> {
        // The iterator is not `Send`, so build the event before awaiting.
        let event = {
            let mut leaves = leaves.into_iter();
            let Some((newest, cert)) = leaves.next() else {
                return Ok(());
            };
            CoordinatorEvent::NewDecide {
                leaf_infos: std::iter::once(newest.clone())
                    .chain(leaves.map(|(info, _)| info.clone()))
                    .collect(),
                cert1: cert.qc().clone(),
                cert2: None,
            }
        };
        consumer.handle_event(&event).await
    }

    async fn load_latest_acted_view(&self) -> anyhow::Result<Option<ViewNumber>> {
        Ok(None)
    }

    async fn load_restart_view(&self) -> anyhow::Result<Option<ViewNumber>> {
        Ok(None)
    }

    async fn load_anchor_leaf(&self) -> anyhow::Result<Option<(Leaf2, CertificatePair<SeqTypes>)>> {
        Ok(None)
    }

    async fn load_da_proposal(
        &self,
        _view: ViewNumber,
    ) -> anyhow::Result<Option<Proposal<SeqTypes, DaProposal2<SeqTypes>>>> {
        Ok(None)
    }

    async fn load_vid_share(
        &self,
        _view: ViewNumber,
    ) -> anyhow::Result<Option<Proposal<SeqTypes, VidDisperseShare<SeqTypes>>>> {
        Ok(None)
    }

    async fn load_quorum_proposals(
        &self,
    ) -> anyhow::Result<BTreeMap<ViewNumber, Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>>>>
    {
        Ok(Default::default())
    }
    async fn load_quorum_proposal(
        &self,
        view: ViewNumber,
    ) -> anyhow::Result<Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>>> {
        bail!("proposal {view:?} not available");
    }
    async fn load_upgrade_certificate(
        &self,
    ) -> anyhow::Result<Option<UpgradeCertificate<SeqTypes>>> {
        Ok(None)
    }

    async fn append_vid(
        &self,
        _proposal: &Proposal<SeqTypes, VidDisperseShare<SeqTypes>>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    async fn append_da(
        &self,
        _proposal: &Proposal<SeqTypes, DaProposal<SeqTypes>>,
        _vid_commit: VidCommitment,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    async fn record_action(&self, _view: ViewNumber, _action: HotShotAction) -> anyhow::Result<()> {
        Ok(())
    }
    async fn append_quorum_proposal2(
        &self,
        _proposal: &Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    async fn store_upgrade_certificate(
        &self,
        _decided_upgrade_certificate: Option<UpgradeCertificate<SeqTypes>>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn load_next_epoch_quorum_certificate(
        &self,
    ) -> anyhow::Result<Option<NextEpochQuorumCertificate2<SeqTypes>>> {
        Ok(None)
    }

    async fn store_eqc(
        &self,
        _high_qc: QuorumCertificate2<SeqTypes>,
        _next_epoch_high_qc: NextEpochQuorumCertificate2<SeqTypes>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn load_eqc(
        &self,
    ) -> Option<(
        QuorumCertificate2<SeqTypes>,
        NextEpochQuorumCertificate2<SeqTypes>,
    )> {
        None
    }

    async fn append_da2(
        &self,
        _proposal: &Proposal<SeqTypes, DaProposal2<SeqTypes>>,
        _vid_commit: VidCommitment,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn append_proposal2(
        &self,
        _proposal: &Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn store_drb_result(
        &self,
        _epoch: EpochNumber,
        _drb_result: DrbResult,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn store_drb_input(&self, _drb_input: DrbInput) -> anyhow::Result<()> {
        Ok(())
    }
    async fn load_drb_input(&self, _epoch: u64) -> anyhow::Result<DrbInput> {
        bail!("Cannot load from NoStorage")
    }

    async fn load_start_epoch_info(&self) -> anyhow::Result<Vec<InitializerEpochInfo<SeqTypes>>> {
        Ok(Vec::new())
    }

    async fn add_state_cert(
        &self,
        _state_cert: LightClientStateUpdateCertificateV2<SeqTypes>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn load_state_cert(
        &self,
    ) -> anyhow::Result<Option<LightClientStateUpdateCertificateV2<SeqTypes>>> {
        Ok(None)
    }

    fn enable_metrics(&mut self, _metrics: &dyn Metrics) {}
}

#[async_trait]
impl StateCertPersistence for NoStorage {
    async fn get_state_cert_by_epoch(
        &self,
        _epoch: u64,
    ) -> anyhow::Result<Option<LightClientStateUpdateCertificateV2<SeqTypes>>> {
        Ok(None)
    }

    async fn insert_state_cert(
        &self,
        _epoch: u64,
        _cert: LightClientStateUpdateCertificateV2<SeqTypes>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

#[async_trait]
impl MembershipPersistence for NoStorage {
    async fn load_stake(&self, _epoch: EpochNumber) -> anyhow::Result<Option<StakeTuple>> {
        Ok(None)
    }

    async fn load_latest_stake(&self, _limit: u64) -> anyhow::Result<Option<Vec<IndexedStake>>> {
        Ok(None)
    }

    async fn load_drb_result(&self, _epoch: EpochNumber) -> anyhow::Result<Option<DrbResult>> {
        Ok(None)
    }

    async fn load_epoch_root(&self, _epoch: EpochNumber) -> anyhow::Result<Option<Header>> {
        Ok(None)
    }

    async fn store_epoch_root(
        &self,
        _epoch: EpochNumber,
        _block_header: Header,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn store_stake(
        &self,
        _epoch: EpochNumber,
        _stake: AuthenticatedValidatorMap,
        _block_reward: Option<RewardAmount>,
        _stake_table_hash: Option<StakeTableHash>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn store_events(
        &self,
        _l1: u64,
        _events: Vec<(EventKey, StakeTableEvent)>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    async fn load_events(
        &self,
        _from_l1_block: u64,
        _l1_block: u64,
    ) -> anyhow::Result<(
        Option<EventsPersistenceRead>,
        Vec<(EventKey, StakeTableEvent)>,
    )> {
        Ok((None, Vec::new()))
    }

    async fn delete_stake_tables(&self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn store_all_validators(
        &self,
        _epoch: EpochNumber,
        _all_validators: RegisteredValidatorMap,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn load_all_validators(
        &self,
        _epoch: EpochNumber,
        _offset: u64,
        _limit: u64,
    ) -> anyhow::Result<Vec<RegisteredValidator<PubKey>>> {
        Ok(Default::default())
    }
}
