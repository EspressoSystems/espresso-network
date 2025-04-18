//! Mock implementation of persistence, for testing.
#![cfg(any(test, feature = "testing"))]

use std::{collections::BTreeMap, sync::Arc};

use anyhow::bail;
use async_trait::async_trait;
use espresso_types::{
    traits::MembershipPersistence,
    v0::traits::{EventConsumer, PersistenceOptions, SequencerPersistence},
    v0_3::{EventKey, IndexedStake, StakeTableEvent, Validator},
    Leaf2, NetworkConfig,
};
use hotshot::{types::BLSPubKey, InitializerEpochInfo};
use hotshot_types::{
    data::{
        vid_disperse::{ADVZDisperseShare, VidDisperseShare2},
        DaProposal, DaProposal2, EpochNumber, QuorumProposalWrapper, VidCommitment,
        VidDisperseShare,
    },
    drb::DrbResult,
    event::{Event, EventType, HotShotAction, LeafInfo},
    message::Proposal,
    simple_certificate::{
        LightClientStateUpdateCertificate, NextEpochQuorumCertificate2, QuorumCertificate2,
        UpgradeCertificate,
    },
};
use indexmap::IndexMap;

use crate::{NodeType, SeqTypes, ViewNumber};

#[derive(Clone, Copy, Debug)]
pub struct Options;

#[async_trait]
impl PersistenceOptions for Options {
    type Persistence = NoStorage;

    fn set_view_retention(&mut self, _: u64) {}

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

    async fn append_decided_leaves(
        &self,
        view_number: ViewNumber,
        leaves: impl IntoIterator<Item = (&LeafInfo<SeqTypes>, QuorumCertificate2<SeqTypes>)> + Send,
        consumer: &impl EventConsumer,
    ) -> anyhow::Result<()> {
        let leaves = leaves
            .into_iter()
            .map(|(info_ref, qc)| (info_ref.clone(), qc))
            .collect::<Vec<_>>();
        for (leaf_info, qc) in leaves {
            consumer
                .handle_event(&Event {
                    view_number,
                    event: EventType::Decide {
                        leaf_chain: Arc::new(vec![leaf_info.clone()]),
                        qc: Arc::new(qc),
                        block_size: None,
                    },
                })
                .await?;
        }
        Ok(())
    }

    async fn load_latest_acted_view(&self) -> anyhow::Result<Option<ViewNumber>> {
        Ok(None)
    }

    async fn load_anchor_leaf(
        &self,
    ) -> anyhow::Result<Option<(Leaf2, QuorumCertificate2<SeqTypes>)>> {
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
        _proposal: &Proposal<SeqTypes, ADVZDisperseShare<SeqTypes>>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    async fn append_vid2(
        &self,
        _proposal: &Proposal<SeqTypes, VidDisperseShare2<SeqTypes>>,
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
    async fn record_action(
        &self,
        _view: ViewNumber,
        _epoch: Option<EpochNumber>,
        _action: HotShotAction,
    ) -> anyhow::Result<()> {
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

    async fn store_next_epoch_quorum_certificate(
        &self,
        _high_qc: NextEpochQuorumCertificate2<SeqTypes>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn load_next_epoch_quorum_certificate(
        &self,
    ) -> anyhow::Result<Option<NextEpochQuorumCertificate2<SeqTypes>>> {
        Ok(None)
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

    async fn migrate_anchor_leaf(&self) -> anyhow::Result<()> {
        Ok(())
    }
    async fn migrate_da_proposals(&self) -> anyhow::Result<()> {
        Ok(())
    }
    async fn migrate_vid_shares(&self) -> anyhow::Result<()> {
        Ok(())
    }
    async fn migrate_quorum_proposals(&self) -> anyhow::Result<()> {
        Ok(())
    }
    async fn migrate_quorum_certificates(&self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn add_drb_result(
        &self,
        _epoch: EpochNumber,
        _drb_result: DrbResult,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn add_epoch_root(
        &self,
        _epoch: EpochNumber,
        _block_header: <SeqTypes as NodeType>::BlockHeader,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn load_start_epoch_info(&self) -> anyhow::Result<Vec<InitializerEpochInfo<SeqTypes>>> {
        Ok(Vec::new())
    }

    async fn add_state_cert(
        &self,
        _state_cert: LightClientStateUpdateCertificate<SeqTypes>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn load_state_cert(
        &self,
    ) -> anyhow::Result<Option<LightClientStateUpdateCertificate<SeqTypes>>> {
        Ok(None)
    }
}

#[async_trait]
impl MembershipPersistence for NoStorage {
    async fn load_stake(
        &self,
        _epoch: EpochNumber,
    ) -> anyhow::Result<Option<IndexMap<alloy::primitives::Address, Validator<BLSPubKey>>>> {
        Ok(None)
    }

    async fn load_latest_stake(&self, _limit: u64) -> anyhow::Result<Option<Vec<IndexedStake>>> {
        Ok(None)
    }

    async fn store_stake(
        &self,
        _epoch: EpochNumber,
        _stake: IndexMap<alloy::primitives::Address, Validator<BLSPubKey>>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn store_events(
        &self,
        _l1_block: u64,
        _events: Vec<(EventKey, StakeTableEvent)>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    async fn load_events(&self) -> anyhow::Result<Option<(u64, Vec<(EventKey, StakeTableEvent)>)>> {
        Ok(None)
    }
}
