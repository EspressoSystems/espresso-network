use serde::{Deserialize, Serialize};

use crate::{
    data::{
        EpochNumber, Leaf2, QuorumProposal2, QuorumProposalWrapper, ViewChangeEvidence2, ViewNumber,
    },
    drb::DrbResult,
    simple_certificate::{
        LightClientStateUpdateCertificateV2, QuorumCertificate2, SimpleCertificate,
        SuccessThreshold, UpgradeCertificate,
    },
    simple_vote::{HasEpoch, TimeoutData2, Vote2Data},
    traits::node_implementation::NodeType,
    vote::HasViewNumber,
};

/// Proposal to append a block.
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
#[serde(bound(deserialize = ""))]
pub struct Proposal<T: NodeType> {
    /// The block header to append
    pub block_header: T::BlockHeader,

    /// view number for the proposal
    pub view_number: ViewNumber,

    /// The epoch number corresponding to the block number.
    pub epoch: EpochNumber,

    /// certificate that the proposal is chaining from
    pub justify_qc: QuorumCertificate2<T>,

    /// certificate proving the last block of the epoch is decided
    pub next_epoch_justify_qc: Option<SimpleCertificate<T, Vote2Data<T>, SuccessThreshold>>,

    /// Possible upgrade certificate, which the leader may optionally attach.
    pub upgrade_certificate: Option<UpgradeCertificate<T>>,

    /// Possible timeout certificate.
    ///
    /// If the `justify_qc` is not for a proposal in the immediately preceding
    /// view, then a timeout certificate must be attached.
    pub view_change_evidence: Option<SimpleCertificate<T, TimeoutData2, SuccessThreshold>>,

    /// The DRB result for the next epoch.
    ///
    /// This is required only for the last block of the epoch. Nodes will verify
    /// that it's consistent with the result from their computations.
    #[serde(with = "serde_bytes")]
    pub next_drb_result: Option<DrbResult>,

    /// The light client state update certificate for the next epoch.
    /// This is required for the epoch root.
    pub state_cert: Option<LightClientStateUpdateCertificateV2<T>>,
}

impl<T: NodeType> HasViewNumber for Proposal<T> {
    fn view_number(&self) -> ViewNumber {
        self.view_number
    }
}

impl<T: NodeType> HasEpoch for Proposal<T> {
    fn epoch(&self) -> Option<EpochNumber> {
        Some(self.epoch)
    }
}

impl<T: NodeType> From<QuorumProposalWrapper<T>> for Proposal<T> {
    fn from(wrapper: QuorumProposalWrapper<T>) -> Self {
        let qp = wrapper.proposal;
        Self {
            block_header: qp.block_header,
            view_number: qp.view_number,
            epoch: qp.epoch.unwrap_or(EpochNumber::new(0)),
            justify_qc: qp.justify_qc,
            next_epoch_justify_qc: None,
            upgrade_certificate: qp.upgrade_certificate,
            view_change_evidence: qp.view_change_evidence.and_then(|e| match e {
                ViewChangeEvidence2::Timeout(tc) => Some(tc),
                ViewChangeEvidence2::ViewSync(_) => None,
            }),
            next_drb_result: qp.next_drb_result,
            state_cert: qp.state_cert,
        }
    }
}

impl<T: NodeType> From<Proposal<T>> for QuorumProposalWrapper<T> {
    fn from(p: Proposal<T>) -> Self {
        QuorumProposalWrapper::from(QuorumProposal2 {
            block_header: p.block_header,
            view_number: p.view_number,
            epoch: Some(p.epoch),
            justify_qc: p.justify_qc,
            next_epoch_justify_qc: None,
            upgrade_certificate: p.upgrade_certificate,
            view_change_evidence: p.view_change_evidence.map(ViewChangeEvidence2::Timeout),
            next_drb_result: p.next_drb_result,
            state_cert: p.state_cert,
        })
    }
}

impl<T: NodeType> From<Proposal<T>> for Leaf2<T> {
    fn from(p: Proposal<T>) -> Self {
        Self::from_quorum_proposal(&QuorumProposalWrapper::from(p))
    }
}
