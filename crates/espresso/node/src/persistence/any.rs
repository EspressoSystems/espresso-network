//! The single persistence type instantiated by the node.
//!
//! Every backend selected at startup is wrapped in [`AnyPersistence`] before it reaches consensus,
//! the API or the query service. Those layers are generic over the persistence type, so without
//! this wrapper each of them is monomorphized once per backend.

use std::{collections::BTreeMap, sync::Arc};

use alloy::primitives::Address;
use async_trait::async_trait;
use espresso_types::{
    AuthenticatedValidatorMap, BackoffParams, Header, Leaf2, NetworkConfig, NodeState, PubKey,
    SeqTypes, StakeTableHash,
    traits::{EventsPersistenceRead, MembershipPersistence, StakeTuple},
    v0::traits::{EventConsumer, SequencerPersistence, StateCatchup},
    v0_3::{EventKey, IndexedStake, RegisteredValidator, RewardAmount, StakeTableEvent},
};
use hotshot::{HotShotInitializer, InitializerEpochInfo};
use hotshot_libp2p_networking::network::behaviours::dht::store::persistent::{
    DhtPersistentStorage, SerializableRecord,
};
use hotshot_new_protocol::message::Certificate2;
use hotshot_types::{
    data::{
        DaProposal, DaProposal2, EpochNumber, QuorumProposalWrapper, VidCommitment,
        VidDisperseShare,
    },
    drb::{DrbInput, DrbResult},
    event::{HotShotAction, LeafInfo},
    message::Proposal,
    new_protocol::CoordinatorEvent,
    simple_certificate::{
        CertificatePair, LightClientStateUpdateCertificateV2, NextEpochQuorumCertificate2,
        QuorumCertificate2, UpgradeCertificate,
    },
    traits::metrics::Metrics,
};
use indexmap::IndexMap;
use versions::Upgrade;

use super::{fs, sql};
use crate::ViewNumber;

/// Persistence backend chosen at startup.
#[derive(Clone, Debug)]
pub enum AnyPersistence {
    Fs(fs::Persistence),
    Sql(Box<sql::Persistence>),
}

impl From<fs::Persistence> for AnyPersistence {
    fn from(p: fs::Persistence) -> Self {
        Self::Fs(p)
    }
}

impl From<sql::Persistence> for AnyPersistence {
    fn from(p: sql::Persistence) -> Self {
        Self::Sql(Box::new(p))
    }
}

#[async_trait]
impl SequencerPersistence for AnyPersistence {
    fn into_catchup_provider(
        self,
        backoff: BackoffParams,
    ) -> anyhow::Result<Arc<dyn StateCatchup>> {
        match self {
            Self::Fs(p) => p.into_catchup_provider(backoff),
            Self::Sql(p) => p.into_catchup_provider(backoff),
        }
    }

    async fn load_config(&self) -> anyhow::Result<Option<NetworkConfig>> {
        match self {
            Self::Fs(p) => p.load_config().await,
            Self::Sql(p) => p.load_config().await,
        }
    }

    async fn save_config(&self, cfg: &NetworkConfig) -> anyhow::Result<()> {
        match self {
            Self::Fs(p) => p.save_config(cfg).await,
            Self::Sql(p) => p.save_config(cfg).await,
        }
    }

    async fn load_latest_acted_view(&self) -> anyhow::Result<Option<ViewNumber>> {
        match self {
            Self::Fs(p) => p.load_latest_acted_view().await,
            Self::Sql(p) => p.load_latest_acted_view().await,
        }
    }

    async fn load_restart_view(&self) -> anyhow::Result<Option<ViewNumber>> {
        match self {
            Self::Fs(p) => p.load_restart_view().await,
            Self::Sql(p) => p.load_restart_view().await,
        }
    }

    async fn load_quorum_proposals(
        &self,
    ) -> anyhow::Result<BTreeMap<ViewNumber, Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>>>>
    {
        match self {
            Self::Fs(p) => p.load_quorum_proposals().await,
            Self::Sql(p) => p.load_quorum_proposals().await,
        }
    }

    async fn load_quorum_proposal(
        &self,
        view: ViewNumber,
    ) -> anyhow::Result<Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>>> {
        match self {
            Self::Fs(p) => p.load_quorum_proposal(view).await,
            Self::Sql(p) => p.load_quorum_proposal(view).await,
        }
    }

    async fn load_vid_share(
        &self,
        view: ViewNumber,
    ) -> anyhow::Result<Option<Proposal<SeqTypes, VidDisperseShare<SeqTypes>>>> {
        match self {
            Self::Fs(p) => p.load_vid_share(view).await,
            Self::Sql(p) => p.load_vid_share(view).await,
        }
    }

    async fn load_da_proposal(
        &self,
        view: ViewNumber,
    ) -> anyhow::Result<Option<Proposal<SeqTypes, DaProposal2<SeqTypes>>>> {
        match self {
            Self::Fs(p) => p.load_da_proposal(view).await,
            Self::Sql(p) => p.load_da_proposal(view).await,
        }
    }

    async fn load_upgrade_certificate(
        &self,
    ) -> anyhow::Result<Option<UpgradeCertificate<SeqTypes>>> {
        match self {
            Self::Fs(p) => p.load_upgrade_certificate().await,
            Self::Sql(p) => p.load_upgrade_certificate().await,
        }
    }

    async fn load_start_epoch_info(&self) -> anyhow::Result<Vec<InitializerEpochInfo<SeqTypes>>> {
        match self {
            Self::Fs(p) => p.load_start_epoch_info().await,
            Self::Sql(p) => p.load_start_epoch_info().await,
        }
    }

    async fn load_state_cert(
        &self,
    ) -> anyhow::Result<Option<LightClientStateUpdateCertificateV2<SeqTypes>>> {
        match self {
            Self::Fs(p) => p.load_state_cert().await,
            Self::Sql(p) => p.load_state_cert().await,
        }
    }

    async fn get_state_cert_by_epoch(
        &self,
        epoch: u64,
    ) -> anyhow::Result<Option<LightClientStateUpdateCertificateV2<SeqTypes>>> {
        match self {
            Self::Fs(p) => p.get_state_cert_by_epoch(epoch).await,
            Self::Sql(p) => p.get_state_cert_by_epoch(epoch).await,
        }
    }

    async fn insert_state_cert(
        &self,
        epoch: u64,
        cert: LightClientStateUpdateCertificateV2<SeqTypes>,
    ) -> anyhow::Result<()> {
        match self {
            Self::Fs(p) => p.insert_state_cert(epoch, cert).await,
            Self::Sql(p) => p.insert_state_cert(epoch, cert).await,
        }
    }

    async fn load_consensus_state(
        &self,
        state: NodeState,
        upgrade: Upgrade,
    ) -> anyhow::Result<(HotShotInitializer<SeqTypes>, Option<ViewNumber>)> {
        match self {
            Self::Fs(p) => p.load_consensus_state(state, upgrade).await,
            Self::Sql(p) => p.load_consensus_state(state, upgrade).await,
        }
    }

    async fn persist_event(
        &self,
        event: &CoordinatorEvent<SeqTypes>,
        consumer: &(impl EventConsumer + 'static),
    ) -> Option<(ViewNumber, Option<Arc<CertificatePair<SeqTypes>>>)> {
        match self {
            Self::Fs(p) => p.persist_event(event, consumer).await,
            Self::Sql(p) => p.persist_event(event, consumer).await,
        }
    }

    async fn append_decided_leaves(
        &self,
        decided_view: ViewNumber,
        leaf_chain: impl IntoIterator<Item = (&LeafInfo<SeqTypes>, CertificatePair<SeqTypes>)> + Send,
        deciding_qc: Option<Arc<CertificatePair<SeqTypes>>>,
        consumer: &(impl EventConsumer + 'static),
    ) -> anyhow::Result<()> {
        match self {
            Self::Fs(p) => {
                p.append_decided_leaves(decided_view, leaf_chain, deciding_qc, consumer)
                    .await
            },
            Self::Sql(p) => {
                p.append_decided_leaves(decided_view, leaf_chain, deciding_qc, consumer)
                    .await
            },
        }
    }

    async fn persist_decided_leaves(
        &self,
        decided_view: ViewNumber,
        leaf_chain: impl IntoIterator<Item = (&LeafInfo<SeqTypes>, CertificatePair<SeqTypes>)> + Send,
        deciding_qc: Option<Arc<CertificatePair<SeqTypes>>>,
        consumer: &(impl EventConsumer + 'static),
    ) -> anyhow::Result<()> {
        match self {
            Self::Fs(p) => {
                p.persist_decided_leaves(decided_view, leaf_chain, deciding_qc, consumer)
                    .await
            },
            Self::Sql(p) => {
                p.persist_decided_leaves(decided_view, leaf_chain, deciding_qc, consumer)
                    .await
            },
        }
    }

    async fn process_decided_events(
        &self,
        decided_view: ViewNumber,
        deciding_qc: Option<Arc<CertificatePair<SeqTypes>>>,
        consumer: &(impl EventConsumer + 'static),
    ) -> anyhow::Result<Option<ViewNumber>> {
        match self {
            Self::Fs(p) => {
                p.process_decided_events(decided_view, deciding_qc, consumer)
                    .await
            },
            Self::Sql(p) => {
                p.process_decided_events(decided_view, deciding_qc, consumer)
                    .await
            },
        }
    }

    async fn load_anchor_leaf(&self) -> anyhow::Result<Option<(Leaf2, CertificatePair<SeqTypes>)>> {
        match self {
            Self::Fs(p) => p.load_anchor_leaf().await,
            Self::Sql(p) => p.load_anchor_leaf().await,
        }
    }

    async fn append_vid(
        &self,
        proposal: &Proposal<SeqTypes, VidDisperseShare<SeqTypes>>,
    ) -> anyhow::Result<()> {
        match self {
            Self::Fs(p) => p.append_vid(proposal).await,
            Self::Sql(p) => p.append_vid(proposal).await,
        }
    }

    async fn append_da(
        &self,
        proposal: &Proposal<SeqTypes, DaProposal<SeqTypes>>,
        vid_commit: VidCommitment,
    ) -> anyhow::Result<()> {
        match self {
            Self::Fs(p) => p.append_da(proposal, vid_commit).await,
            Self::Sql(p) => p.append_da(proposal, vid_commit).await,
        }
    }

    async fn record_action(
        &self,
        view: ViewNumber,
        epoch: Option<EpochNumber>,
        action: HotShotAction,
    ) -> anyhow::Result<()> {
        match self {
            Self::Fs(p) => p.record_action(view, epoch, action).await,
            Self::Sql(p) => p.record_action(view, epoch, action).await,
        }
    }

    async fn append_quorum_proposal2(
        &self,
        proposal: &Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>>,
    ) -> anyhow::Result<()> {
        match self {
            Self::Fs(p) => p.append_quorum_proposal2(proposal).await,
            Self::Sql(p) => p.append_quorum_proposal2(proposal).await,
        }
    }

    async fn append_cert2(
        &self,
        view: ViewNumber,
        cert2: Certificate2<SeqTypes>,
    ) -> anyhow::Result<()> {
        match self {
            Self::Fs(p) => p.append_cert2(view, cert2).await,
            Self::Sql(p) => p.append_cert2(view, cert2).await,
        }
    }

    async fn load_cert2(&self, view: ViewNumber) -> anyhow::Result<Option<Certificate2<SeqTypes>>> {
        match self {
            Self::Fs(p) => p.load_cert2(view).await,
            Self::Sql(p) => p.load_cert2(view).await,
        }
    }

    async fn append_high_qc2(&self, high_qc: QuorumCertificate2<SeqTypes>) -> anyhow::Result<()> {
        match self {
            Self::Fs(p) => p.append_high_qc2(high_qc).await,
            Self::Sql(p) => p.append_high_qc2(high_qc).await,
        }
    }

    async fn load_high_qc2(&self) -> anyhow::Result<Option<QuorumCertificate2<SeqTypes>>> {
        match self {
            Self::Fs(p) => p.load_high_qc2().await,
            Self::Sql(p) => p.load_high_qc2().await,
        }
    }

    async fn store_eqc(
        &self,
        high_qc: QuorumCertificate2<SeqTypes>,
        next_epoch_high_qc: NextEpochQuorumCertificate2<SeqTypes>,
    ) -> anyhow::Result<()> {
        match self {
            Self::Fs(p) => p.store_eqc(high_qc, next_epoch_high_qc).await,
            Self::Sql(p) => p.store_eqc(high_qc, next_epoch_high_qc).await,
        }
    }

    async fn load_eqc(
        &self,
    ) -> Option<(
        QuorumCertificate2<SeqTypes>,
        NextEpochQuorumCertificate2<SeqTypes>,
    )> {
        match self {
            Self::Fs(p) => p.load_eqc().await,
            Self::Sql(p) => p.load_eqc().await,
        }
    }

    async fn store_upgrade_certificate(
        &self,
        decided_upgrade_certificate: Option<UpgradeCertificate<SeqTypes>>,
    ) -> anyhow::Result<()> {
        match self {
            Self::Fs(p) => {
                p.store_upgrade_certificate(decided_upgrade_certificate)
                    .await
            },
            Self::Sql(p) => {
                p.store_upgrade_certificate(decided_upgrade_certificate)
                    .await
            },
        }
    }

    async fn load_anchor_view(&self) -> anyhow::Result<ViewNumber> {
        match self {
            Self::Fs(p) => p.load_anchor_view().await,
            Self::Sql(p) => p.load_anchor_view().await,
        }
    }

    async fn load_next_epoch_quorum_certificate(
        &self,
    ) -> anyhow::Result<Option<NextEpochQuorumCertificate2<SeqTypes>>> {
        match self {
            Self::Fs(p) => p.load_next_epoch_quorum_certificate().await,
            Self::Sql(p) => p.load_next_epoch_quorum_certificate().await,
        }
    }

    async fn append_next_epoch_high_qc2(
        &self,
        next_epoch_high_qc: NextEpochQuorumCertificate2<SeqTypes>,
    ) -> anyhow::Result<()> {
        match self {
            Self::Fs(p) => p.append_next_epoch_high_qc2(next_epoch_high_qc).await,
            Self::Sql(p) => p.append_next_epoch_high_qc2(next_epoch_high_qc).await,
        }
    }

    async fn append_da2(
        &self,
        proposal: &Proposal<SeqTypes, DaProposal2<SeqTypes>>,
        vid_commit: VidCommitment,
    ) -> anyhow::Result<()> {
        match self {
            Self::Fs(p) => p.append_da2(proposal, vid_commit).await,
            Self::Sql(p) => p.append_da2(proposal, vid_commit).await,
        }
    }

    async fn append_proposal2(
        &self,
        proposal: &Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>>,
    ) -> anyhow::Result<()> {
        match self {
            Self::Fs(p) => p.append_proposal2(proposal).await,
            Self::Sql(p) => p.append_proposal2(proposal).await,
        }
    }

    async fn store_drb_result(
        &self,
        epoch: EpochNumber,
        drb_result: DrbResult,
    ) -> anyhow::Result<()> {
        match self {
            Self::Fs(p) => p.store_drb_result(epoch, drb_result).await,
            Self::Sql(p) => p.store_drb_result(epoch, drb_result).await,
        }
    }

    async fn store_drb_input(&self, drb_input: DrbInput) -> anyhow::Result<()> {
        match self {
            Self::Fs(p) => p.store_drb_input(drb_input).await,
            Self::Sql(p) => p.store_drb_input(drb_input).await,
        }
    }

    async fn load_drb_input(&self, epoch: u64) -> anyhow::Result<DrbInput> {
        match self {
            Self::Fs(p) => p.load_drb_input(epoch).await,
            Self::Sql(p) => p.load_drb_input(epoch).await,
        }
    }

    async fn add_state_cert(
        &self,
        state_cert: LightClientStateUpdateCertificateV2<SeqTypes>,
    ) -> anyhow::Result<()> {
        match self {
            Self::Fs(p) => p.add_state_cert(state_cert).await,
            Self::Sql(p) => p.add_state_cert(state_cert).await,
        }
    }

    fn enable_metrics(&mut self, metrics: &dyn Metrics) {
        match self {
            Self::Fs(p) => p.enable_metrics(metrics),
            Self::Sql(p) => p.enable_metrics(metrics),
        }
    }
}

#[async_trait]
impl MembershipPersistence for AnyPersistence {
    async fn load_stake(&self, epoch: EpochNumber) -> anyhow::Result<Option<StakeTuple>> {
        match self {
            Self::Fs(p) => p.load_stake(epoch).await,
            Self::Sql(p) => p.load_stake(epoch).await,
        }
    }

    async fn load_latest_stake(&self, limit: u64) -> anyhow::Result<Option<Vec<IndexedStake>>> {
        match self {
            Self::Fs(p) => p.load_latest_stake(limit).await,
            Self::Sql(p) => p.load_latest_stake(limit).await,
        }
    }

    async fn load_drb_result(&self, epoch: EpochNumber) -> anyhow::Result<Option<DrbResult>> {
        match self {
            Self::Fs(p) => p.load_drb_result(epoch).await,
            Self::Sql(p) => p.load_drb_result(epoch).await,
        }
    }

    async fn load_epoch_root(&self, epoch: EpochNumber) -> anyhow::Result<Option<Header>> {
        match self {
            Self::Fs(p) => p.load_epoch_root(epoch).await,
            Self::Sql(p) => p.load_epoch_root(epoch).await,
        }
    }

    async fn store_epoch_root(
        &self,
        epoch: EpochNumber,
        block_header: Header,
    ) -> anyhow::Result<()> {
        match self {
            Self::Fs(p) => p.store_epoch_root(epoch, block_header).await,
            Self::Sql(p) => p.store_epoch_root(epoch, block_header).await,
        }
    }

    async fn store_stake(
        &self,
        epoch: EpochNumber,
        stake: AuthenticatedValidatorMap,
        block_reward: Option<RewardAmount>,
        stake_table_hash: Option<StakeTableHash>,
    ) -> anyhow::Result<()> {
        match self {
            Self::Fs(p) => {
                p.store_stake(epoch, stake, block_reward, stake_table_hash)
                    .await
            },
            Self::Sql(p) => {
                p.store_stake(epoch, stake, block_reward, stake_table_hash)
                    .await
            },
        }
    }

    async fn store_events(
        &self,
        l1_finalized: u64,
        events: Vec<(EventKey, StakeTableEvent)>,
    ) -> anyhow::Result<()> {
        match self {
            Self::Fs(p) => p.store_events(l1_finalized, events).await,
            Self::Sql(p) => p.store_events(l1_finalized, events).await,
        }
    }

    async fn load_events(
        &self,
        from_l1_block: u64,
        l1_finalized: u64,
    ) -> anyhow::Result<(
        Option<EventsPersistenceRead>,
        Vec<(EventKey, StakeTableEvent)>,
    )> {
        match self {
            Self::Fs(p) => p.load_events(from_l1_block, l1_finalized).await,
            Self::Sql(p) => p.load_events(from_l1_block, l1_finalized).await,
        }
    }

    async fn delete_stake_tables(&self) -> anyhow::Result<()> {
        match self {
            Self::Fs(p) => p.delete_stake_tables().await,
            Self::Sql(p) => p.delete_stake_tables().await,
        }
    }

    async fn store_all_validators(
        &self,
        epoch: EpochNumber,
        all_validators: IndexMap<Address, RegisteredValidator<PubKey>>,
    ) -> anyhow::Result<()> {
        match self {
            Self::Fs(p) => p.store_all_validators(epoch, all_validators).await,
            Self::Sql(p) => p.store_all_validators(epoch, all_validators).await,
        }
    }

    async fn load_all_validators(
        &self,
        epoch: EpochNumber,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<Vec<RegisteredValidator<PubKey>>> {
        match self {
            Self::Fs(p) => p.load_all_validators(epoch, offset, limit).await,
            Self::Sql(p) => p.load_all_validators(epoch, offset, limit).await,
        }
    }
}

#[async_trait]
impl DhtPersistentStorage for AnyPersistence {
    async fn save(&self, records: Vec<SerializableRecord>) -> anyhow::Result<()> {
        match self {
            Self::Fs(p) => p.save(records).await,
            Self::Sql(p) => p.save(records).await,
        }
    }

    async fn load(&self) -> anyhow::Result<Vec<SerializableRecord>> {
        match self {
            Self::Fs(p) => p.load().await,
            Self::Sql(p) => p.load().await,
        }
    }
}
