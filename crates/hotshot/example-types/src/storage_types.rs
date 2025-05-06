// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use anyhow::{bail, Result};
use async_lock::RwLock;
use async_trait::async_trait;
use hotshot_types::{
    data::{
        vid_disperse::{ADVZDisperseShare, VidDisperseShare2},
        DaProposal, DaProposal2, QuorumProposal, QuorumProposal2, QuorumProposalWrapper,
        VidCommitment,
    },
    drb::DrbResult,
    event::HotShotAction,
    message::{convert_proposal, Proposal},
    simple_certificate::{
        LightClientStateUpdateCertificate, NextEpochQuorumCertificate2, QuorumCertificate2,
        UpgradeCertificate,
    },
    traits::{
        node_implementation::{ConsensusTime, NodeType},
        storage::Storage,
    },
    vote::HasViewNumber,
};

use crate::testable_delay::{DelayConfig, SupportedTraitTypesForAsyncDelay, TestableDelay};

type VidShares<TYPES> = BTreeMap<
    <TYPES as NodeType>::View,
    HashMap<<TYPES as NodeType>::SignatureKey, Proposal<TYPES, ADVZDisperseShare<TYPES>>>,
>;
type VidShares2<TYPES> = BTreeMap<
    <TYPES as NodeType>::View,
    HashMap<<TYPES as NodeType>::SignatureKey, Proposal<TYPES, VidDisperseShare2<TYPES>>>,
>;

#[derive(Clone, Debug)]
pub struct TestStorageState<TYPES: NodeType> {
    vids: VidShares<TYPES>,
    vid2: VidShares2<TYPES>,
    das: HashMap<TYPES::View, Proposal<TYPES, DaProposal<TYPES>>>,
    da2s: HashMap<TYPES::View, Proposal<TYPES, DaProposal2<TYPES>>>,
    proposals: BTreeMap<TYPES::View, Proposal<TYPES, QuorumProposal<TYPES>>>,
    proposals2: BTreeMap<TYPES::View, Proposal<TYPES, QuorumProposal2<TYPES>>>,
    proposals_wrapper: BTreeMap<TYPES::View, Proposal<TYPES, QuorumProposalWrapper<TYPES>>>,
    high_qc: Option<hotshot_types::simple_certificate::QuorumCertificate<TYPES>>,
    high_qc2: Option<hotshot_types::simple_certificate::QuorumCertificate2<TYPES>>,
    next_epoch_high_qc2:
        Option<hotshot_types::simple_certificate::NextEpochQuorumCertificate2<TYPES>>,
    action: TYPES::View,
    epoch: Option<TYPES::Epoch>,
    state_certs: BTreeMap<TYPES::Epoch, LightClientStateUpdateCertificate<TYPES>>,
    drb_results: BTreeMap<TYPES::Epoch, DrbResult>,
    epoch_roots: BTreeMap<TYPES::Epoch, TYPES::BlockHeader>,
}

impl<TYPES: NodeType> Default for TestStorageState<TYPES> {
    fn default() -> Self {
        Self {
            vids: BTreeMap::new(),
            vid2: BTreeMap::new(),
            das: HashMap::new(),
            da2s: HashMap::new(),
            proposals: BTreeMap::new(),
            proposals2: BTreeMap::new(),
            proposals_wrapper: BTreeMap::new(),
            high_qc: None,
            next_epoch_high_qc2: None,
            high_qc2: None,
            action: TYPES::View::genesis(),
            epoch: None,
            state_certs: BTreeMap::new(),
            drb_results: BTreeMap::new(),
            epoch_roots: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TestStorage<TYPES: NodeType> {
    inner: Arc<RwLock<TestStorageState<TYPES>>>,
    /// `should_return_err` is a testing utility to validate negative cases.
    pub should_return_err: bool,
    pub delay_config: DelayConfig,
    pub decided_upgrade_certificate: Arc<RwLock<Option<UpgradeCertificate<TYPES>>>>,
}

impl<TYPES: NodeType> Default for TestStorage<TYPES> {
    fn default() -> Self {
        Self {
            inner: Arc::new(RwLock::new(TestStorageState::default())),
            should_return_err: false,
            delay_config: DelayConfig::default(),
            decided_upgrade_certificate: Arc::new(RwLock::new(None)),
        }
    }
}

#[async_trait]
impl<TYPES: NodeType> TestableDelay for TestStorage<TYPES> {
    async fn run_delay_settings_from_config(delay_config: &DelayConfig) {
        if let Some(settings) = delay_config.get_setting(&SupportedTraitTypesForAsyncDelay::Storage)
        {
            Self::handle_async_delay(settings).await;
        }
    }
}

impl<TYPES: NodeType> TestStorage<TYPES> {
    pub async fn proposals_cloned(
        &self,
    ) -> BTreeMap<TYPES::View, Proposal<TYPES, QuorumProposalWrapper<TYPES>>> {
        self.inner.read().await.proposals_wrapper.clone()
    }

    pub async fn high_qc_cloned(&self) -> Option<QuorumCertificate2<TYPES>> {
        self.inner.read().await.high_qc2.clone()
    }

    pub async fn next_epoch_high_qc_cloned(&self) -> Option<NextEpochQuorumCertificate2<TYPES>> {
        self.inner.read().await.next_epoch_high_qc2.clone()
    }

    pub async fn decided_upgrade_certificate(&self) -> Option<UpgradeCertificate<TYPES>> {
        self.decided_upgrade_certificate.read().await.clone()
    }

    pub async fn last_actioned_view(&self) -> TYPES::View {
        self.inner.read().await.action
    }

    pub async fn last_actioned_epoch(&self) -> Option<TYPES::Epoch> {
        self.inner.read().await.epoch
    }
    pub async fn vids_cloned(&self) -> VidShares2<TYPES> {
        self.inner.read().await.vid2.clone()
    }

    pub async fn state_cert_cloned(&self) -> Option<LightClientStateUpdateCertificate<TYPES>> {
        self.inner
            .read()
            .await
            .state_certs
            .iter()
            .next_back()
            .map(|(_, cert)| cert.clone())
    }
}

#[async_trait]
impl<TYPES: NodeType> Storage<TYPES> for TestStorage<TYPES> {
    async fn append_vid(&self, proposal: &Proposal<TYPES, ADVZDisperseShare<TYPES>>) -> Result<()> {
        if self.should_return_err {
            bail!("Failed to append VID proposal to storage");
        }
        Self::run_delay_settings_from_config(&self.delay_config).await;
        let mut inner = self.inner.write().await;
        inner
            .vids
            .entry(proposal.data.view_number)
            .or_default()
            .insert(proposal.data.recipient_key.clone(), proposal.clone());
        Ok(())
    }

    async fn append_vid2(
        &self,
        proposal: &Proposal<TYPES, VidDisperseShare2<TYPES>>,
    ) -> Result<()> {
        if self.should_return_err {
            bail!("Failed to append VID proposal to storage");
        }
        Self::run_delay_settings_from_config(&self.delay_config).await;
        let mut inner = self.inner.write().await;
        inner
            .vid2
            .entry(proposal.data.view_number)
            .or_default()
            .insert(proposal.data.recipient_key.clone(), proposal.clone());
        Ok(())
    }

    async fn append_da(
        &self,
        proposal: &Proposal<TYPES, DaProposal<TYPES>>,
        _vid_commit: VidCommitment,
    ) -> Result<()> {
        if self.should_return_err {
            bail!("Failed to append DA proposal to storage");
        }
        Self::run_delay_settings_from_config(&self.delay_config).await;
        let mut inner = self.inner.write().await;
        inner
            .das
            .insert(proposal.data.view_number, proposal.clone());
        Ok(())
    }

    async fn append_da2(
        &self,
        proposal: &Proposal<TYPES, DaProposal2<TYPES>>,
        _vid_commit: VidCommitment,
    ) -> Result<()> {
        if self.should_return_err {
            bail!("Failed to append DA proposal (2) to storage");
        }
        Self::run_delay_settings_from_config(&self.delay_config).await;
        let mut inner = self.inner.write().await;
        inner
            .da2s
            .insert(proposal.data.view_number, proposal.clone());
        Ok(())
    }

    async fn append_proposal(
        &self,
        proposal: &Proposal<TYPES, QuorumProposal<TYPES>>,
    ) -> Result<()> {
        if self.should_return_err {
            bail!("Failed to append Quorum proposal (1) to storage");
        }
        Self::run_delay_settings_from_config(&self.delay_config).await;
        let mut inner = self.inner.write().await;
        inner
            .proposals
            .insert(proposal.data.view_number, proposal.clone());
        Ok(())
    }

    async fn append_proposal2(
        &self,
        proposal: &Proposal<TYPES, QuorumProposal2<TYPES>>,
    ) -> Result<()> {
        if self.should_return_err {
            bail!("Failed to append Quorum proposal (2) to storage");
        }
        Self::run_delay_settings_from_config(&self.delay_config).await;
        let mut inner = self.inner.write().await;
        inner
            .proposals2
            .insert(proposal.data.view_number, proposal.clone());
        Ok(())
    }

    async fn append_proposal_wrapper(
        &self,
        proposal: &Proposal<TYPES, QuorumProposalWrapper<TYPES>>,
    ) -> Result<()> {
        if self.should_return_err {
            bail!("Failed to append Quorum proposal (wrapped) to storage");
        }
        Self::run_delay_settings_from_config(&self.delay_config).await;
        let mut inner = self.inner.write().await;
        inner
            .proposals_wrapper
            .insert(proposal.data.view_number(), proposal.clone());
        Ok(())
    }

    async fn record_action(
        &self,
        view: <TYPES as NodeType>::View,
        epoch: Option<TYPES::Epoch>,
        action: hotshot_types::event::HotShotAction,
    ) -> Result<()> {
        if self.should_return_err {
            bail!("Failed to append Action to storage");
        }
        let mut inner = self.inner.write().await;
        if matches!(action, HotShotAction::Vote | HotShotAction::Propose) {
            if view > inner.action {
                inner.action = view;
            }
            if epoch > inner.epoch {
                inner.epoch = epoch;
            }
        }
        Self::run_delay_settings_from_config(&self.delay_config).await;
        Ok(())
    }

    async fn update_high_qc(
        &self,
        new_high_qc: hotshot_types::simple_certificate::QuorumCertificate<TYPES>,
    ) -> Result<()> {
        if self.should_return_err {
            bail!("Failed to update high qc to storage");
        }
        Self::run_delay_settings_from_config(&self.delay_config).await;
        let mut inner = self.inner.write().await;
        if let Some(ref current_high_qc) = inner.high_qc {
            if new_high_qc.view_number() > current_high_qc.view_number() {
                inner.high_qc = Some(new_high_qc);
            }
        } else {
            inner.high_qc = Some(new_high_qc);
        }
        Ok(())
    }

    async fn update_high_qc2(
        &self,
        new_high_qc: hotshot_types::simple_certificate::QuorumCertificate2<TYPES>,
    ) -> Result<()> {
        if self.should_return_err {
            bail!("Failed to update high qc to storage");
        }
        Self::run_delay_settings_from_config(&self.delay_config).await;
        let mut inner = self.inner.write().await;
        if let Some(ref current_high_qc) = inner.high_qc2 {
            if new_high_qc.view_number() > current_high_qc.view_number() {
                inner.high_qc2 = Some(new_high_qc);
            }
        } else {
            inner.high_qc2 = Some(new_high_qc);
        }
        Ok(())
    }

    async fn update_state_cert(
        &self,
        state_cert: LightClientStateUpdateCertificate<TYPES>,
    ) -> Result<()> {
        if self.should_return_err {
            bail!("Failed to update state_cert to storage");
        }
        Self::run_delay_settings_from_config(&self.delay_config).await;
        self.inner
            .write()
            .await
            .state_certs
            .insert(state_cert.epoch, state_cert);
        Ok(())
    }

    async fn update_next_epoch_high_qc2(
        &self,
        new_next_epoch_high_qc: hotshot_types::simple_certificate::NextEpochQuorumCertificate2<
            TYPES,
        >,
    ) -> Result<()> {
        if self.should_return_err {
            bail!("Failed to update next epoch high qc to storage");
        }
        Self::run_delay_settings_from_config(&self.delay_config).await;
        let mut inner = self.inner.write().await;
        if let Some(ref current_next_epoch_high_qc) = inner.next_epoch_high_qc2 {
            if new_next_epoch_high_qc.view_number() > current_next_epoch_high_qc.view_number() {
                inner.next_epoch_high_qc2 = Some(new_next_epoch_high_qc);
            }
        } else {
            inner.next_epoch_high_qc2 = Some(new_next_epoch_high_qc);
        }
        Ok(())
    }

    async fn update_decided_upgrade_certificate(
        &self,
        decided_upgrade_certificate: Option<UpgradeCertificate<TYPES>>,
    ) -> Result<()> {
        *self.decided_upgrade_certificate.write().await = decided_upgrade_certificate;

        Ok(())
    }

    async fn migrate_consensus(&self) -> Result<()> {
        let mut storage_writer = self.inner.write().await;

        for (view, proposal) in storage_writer.proposals.clone().iter() {
            storage_writer
                .proposals2
                .insert(*view, convert_proposal(proposal.clone()));
        }

        Ok(())
    }

    async fn add_drb_result(&self, epoch: TYPES::Epoch, drb_result: DrbResult) -> Result<()> {
        let mut inner = self.inner.write().await;

        inner.drb_results.insert(epoch, drb_result);

        Ok(())
    }

    async fn add_epoch_root(
        &self,
        epoch: TYPES::Epoch,
        block_header: TYPES::BlockHeader,
    ) -> Result<()> {
        let mut inner = self.inner.write().await;

        inner.epoch_roots.insert(epoch, block_header);

        Ok(())
    }
}
