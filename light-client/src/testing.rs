#![cfg(any(test, feature = "testing"))]

use std::collections::HashMap;

use anyhow::{bail, ensure, Context, Result};
use committable::{Commitment, Committable};
use espresso_types::{EpochVersion, FeeVersion, Leaf2, NodeState, SeqTypes, SequencerVersions};
use hotshot_query_service::availability::LeafQueryData;
use hotshot_types::{
    data::{QuorumProposal2, QuorumProposalWrapper, ViewNumber},
    simple_certificate::{QuorumCertificate2, UpgradeCertificate},
    simple_vote::UpgradeProposalData,
    traits::node_implementation::{ConsensusTime, Versions},
};
use vbs::version::StaticVersionType;

use crate::core::quorum::Quorum;

/// Upgrade to epochs during testing.
pub type EnableEpochs = SequencerVersions<LegacyVersion, EpochVersion>;

/// Test without epochs and with legacy HotStuff.
pub type LegacyVersion = FeeVersion;

/// Construct a valid leaf chain for the given height range.
pub async fn leaf_chain<V: StaticVersionType + 'static>(
    range: impl IntoIterator<Item = u64>,
) -> Vec<LeafQueryData<SeqTypes>> {
    custom_leaf_range::<SequencerVersions<V, V>>(range, |_| {}).await
}

/// Construct a valid leaf chain for the given height range.
///
/// The chain will upgrade from `V::Base` to `V::Upgrade` at height `upgrade_height`.
pub async fn leaf_chain_with_upgrade<V: Versions>(
    range: impl IntoIterator<Item = u64>,
    upgrade_height: u64,
) -> Vec<LeafQueryData<SeqTypes>> {
    custom_leaf_chain_with_upgrade::<V>(range, upgrade_height, |_| {}).await
}

/// Construct a customized leaf chain for the given height range.
///
/// The chain will upgrade from `V::Base` to `V::Upgrade` at height `upgrade_height`.
pub async fn custom_leaf_chain_with_upgrade<V: Versions>(
    range: impl IntoIterator<Item = u64>,
    upgrade_height: u64,
    map: impl Fn(&mut QuorumProposal2<SeqTypes>),
) -> Vec<LeafQueryData<SeqTypes>> {
    let upgrade_leaf: Leaf2 = Leaf2::genesis::<V>(
        &Default::default(),
        &NodeState::mock()
            .with_genesis_version(V::Upgrade::version())
            .with_current_version(V::Upgrade::version()),
    )
    .await;
    let upgrade_data = UpgradeProposalData {
        old_version: V::Base::version(),
        new_version: V::Upgrade::version(),
        new_version_hash: Default::default(),
        old_version_last_view: ViewNumber::new(upgrade_height - 1),
        new_version_first_view: ViewNumber::new(upgrade_height),
        decide_by: ViewNumber::new(upgrade_height),
    };
    let upgrade_commit = upgrade_data.commit();
    let upgrade_cert = UpgradeCertificate::new(
        upgrade_data,
        upgrade_commit,
        ViewNumber::new(upgrade_height),
        Default::default(),
        Default::default(),
    );

    custom_leaf_range::<V>(range, |proposal| {
        let height = proposal.block_header.height();
        if height < upgrade_height {
            // All views leading up to the upgrade get a certificate indicating the coming upgrade.
            proposal.upgrade_certificate = Some(upgrade_cert.clone());
        } else {
            // After the upgrade takes effect we stop attaching the upgrade certificate, and we use
            // the upgraded header version.
            proposal.upgrade_certificate = None;
            proposal.block_header = upgrade_leaf.block_header().clone();
            *proposal.block_header.height_mut() = height;
        }
        map(proposal);
    })
    .await
}

/// Construct a customized leaf chain for the given height range.
pub async fn custom_leaf_range<V: Versions>(
    range: impl IntoIterator<Item = u64>,
    map: impl Fn(&mut QuorumProposal2<SeqTypes>),
) -> Vec<LeafQueryData<SeqTypes>> {
    let node_state = NodeState::mock()
        .with_genesis_version(V::Base::version())
        .with_current_version(V::Base::version());
    let genesis_leaf: Leaf2 = Leaf2::genesis::<V>(&Default::default(), &node_state).await;
    tracing::info!(?genesis_leaf, "leaf chain");

    let mut qc = QuorumCertificate2::genesis::<V>(&Default::default(), &node_state).await;
    let mut quorum_proposal = QuorumProposalWrapper::<SeqTypes> {
        proposal: QuorumProposal2::<SeqTypes> {
            epoch: None,
            block_header: genesis_leaf.block_header().clone(),
            view_number: genesis_leaf.view_number(),
            justify_qc: qc.clone(),
            upgrade_certificate: None,
            view_change_evidence: None,
            next_drb_result: None,
            next_epoch_justify_qc: None,
            state_cert: None,
        },
    };

    let mut leaves = vec![];
    for height in range {
        *quorum_proposal.proposal.block_header.height_mut() = height;
        quorum_proposal.proposal.view_number = ViewNumber::new(height);
        map(&mut quorum_proposal.proposal);
        let leaf = Leaf2::from_quorum_proposal(&quorum_proposal);

        qc.view_number = ViewNumber::new(height);
        qc.data.leaf_commit = Committable::commit(&leaf);

        leaves.push(LeafQueryData::new(leaf, qc.clone()).unwrap());
        quorum_proposal.proposal.justify_qc = qc.clone();
    }

    leaves
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AlwaysTrueQuorum;

impl Quorum for AlwaysTrueQuorum {
    async fn verify_static<V: StaticVersionType + 'static>(
        &self,
        _: &QuorumCertificate2<SeqTypes>,
    ) -> Result<()> {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AlwaysFalseQuorum;

impl Quorum for AlwaysFalseQuorum {
    async fn verify_static<V: StaticVersionType + 'static>(
        &self,
        _: &QuorumCertificate2<SeqTypes>,
    ) -> Result<()> {
        bail!("always false quorum");
    }
}

/// A quorum which verifies that calls to `verify` use the correct version, but does not check
/// signatures.
#[derive(Clone, Debug, Default)]
pub struct VersionCheckQuorum {
    leaves: HashMap<Commitment<Leaf2>, Leaf2>,
}

impl VersionCheckQuorum {
    pub fn new(leaves: impl IntoIterator<Item = Leaf2>) -> Self {
        Self {
            leaves: leaves
                .into_iter()
                .map(|leaf| (leaf.commit(), leaf))
                .collect(),
        }
    }
}

impl Quorum for VersionCheckQuorum {
    async fn verify_static<V: StaticVersionType + 'static>(
        &self,
        qc: &QuorumCertificate2<SeqTypes>,
    ) -> anyhow::Result<()> {
        let leaf = self
            .leaves
            .get(&qc.data.leaf_commit)
            .context(format!("unknown leaf {}", qc.data.leaf_commit))?;
        ensure!(
            leaf.block_header().version() == V::version(),
            "version mismatch: leaf has version {}, but verifier is using version {}",
            leaf.block_header().version(),
            V::version()
        );
        Ok(())
    }
}
