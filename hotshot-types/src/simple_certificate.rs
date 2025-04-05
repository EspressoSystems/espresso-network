// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Implementations of the simple certificate type.  Used for Quorum, DA, and Timeout Certificates

use std::{
    fmt::{self, Debug, Display, Formatter},
    future::Future,
    hash::Hash,
    marker::PhantomData,
    sync::Arc,
};

use alloy::primitives::U256;
use async_lock::RwLock;
use committable::{Commitment, Committable};
use hotshot_utils::anytrace::*;
use serde::{Deserialize, Serialize};

use crate::{
    data::serialize_signature2,
    epoch_membership::EpochMembership,
    light_client::LightClientState,
    message::UpgradeLock,
    simple_vote::{
        DaData, DaData2, HasEpoch, NextEpochQuorumData2, QuorumData, QuorumData2, QuorumMarker,
        TimeoutData, TimeoutData2, UpgradeProposalData, VersionedVoteData, ViewSyncCommitData,
        ViewSyncCommitData2, ViewSyncFinalizeData, ViewSyncFinalizeData2, ViewSyncPreCommitData,
        ViewSyncPreCommitData2, Voteable,
    },
    traits::{
        node_implementation::{ConsensusTime, NodeType, Versions},
        signature_key::{SignatureKey, StateSignatureKey},
    },
    vote::{Certificate, HasViewNumber},
    PeerConfig, StakeTableEntries,
};

/// Trait which allows use to inject different threshold calculations into a Certificate type
pub trait Threshold<TYPES: NodeType> {
    /// Calculate a threshold based on the membership
    fn threshold(membership: &EpochMembership<TYPES>) -> impl Future<Output = U256> + Send;
}

/// Defines a threshold which is 2f + 1 (Amount needed for Quorum)
#[derive(Serialize, Deserialize, Eq, Hash, PartialEq, Debug, Clone)]
pub struct SuccessThreshold {}

impl<TYPES: NodeType> Threshold<TYPES> for SuccessThreshold {
    async fn threshold(membership: &EpochMembership<TYPES>) -> U256 {
        membership.success_threshold().await
    }
}

/// Defines a threshold which is f + 1 (i.e at least one of the stake is honest)
#[derive(Serialize, Deserialize, Eq, Hash, PartialEq, Debug, Clone)]
pub struct OneHonestThreshold {}

impl<TYPES: NodeType> Threshold<TYPES> for OneHonestThreshold {
    async fn threshold(membership: &EpochMembership<TYPES>) -> U256 {
        membership.failure_threshold().await
    }
}

/// Defines a threshold which is 0.9n + 1 (i.e. over 90% of the nodes with stake)
#[derive(Serialize, Deserialize, Eq, Hash, PartialEq, Debug, Clone)]
pub struct UpgradeThreshold {}

impl<TYPES: NodeType> Threshold<TYPES> for UpgradeThreshold {
    async fn threshold(membership: &EpochMembership<TYPES>) -> U256 {
        membership.upgrade_threshold().await
    }
}

/// A certificate which can be created by aggregating many simple votes on the commitment.
#[derive(Serialize, Deserialize, Eq, Hash, PartialEq, Debug, Clone)]
pub struct SimpleCertificate<
    TYPES: NodeType,
    VOTEABLE: Voteable<TYPES>,
    THRESHOLD: Threshold<TYPES>,
> {
    /// The data this certificate is for.  I.e the thing that was voted on to create this Certificate
    pub data: VOTEABLE,
    /// commitment of all the votes this cert should be signed over
    vote_commitment: Commitment<VOTEABLE>,
    /// Which view this QC relates to
    pub view_number: TYPES::View,
    /// assembled signature for certificate aggregation
    pub signatures: Option<<TYPES::SignatureKey as SignatureKey>::QcType>,
    /// phantom data for `THRESHOLD` and `TYPES`
    pub _pd: PhantomData<(TYPES, THRESHOLD)>,
}

impl<TYPES: NodeType, VOTEABLE: Voteable<TYPES>, THRESHOLD: Threshold<TYPES>>
    SimpleCertificate<TYPES, VOTEABLE, THRESHOLD>
{
    /// Creates a new instance of `SimpleCertificate`
    pub fn new(
        data: VOTEABLE,
        vote_commitment: Commitment<VOTEABLE>,
        view_number: TYPES::View,
        signatures: Option<<TYPES::SignatureKey as SignatureKey>::QcType>,
        pd: PhantomData<(TYPES, THRESHOLD)>,
    ) -> Self {
        Self {
            data,
            vote_commitment,
            view_number,
            signatures,
            _pd: pd,
        }
    }
}

impl<TYPES: NodeType, VOTEABLE: Voteable<TYPES> + Committable, THRESHOLD: Threshold<TYPES>>
    Committable for SimpleCertificate<TYPES, VOTEABLE, THRESHOLD>
{
    fn commit(&self) -> Commitment<Self> {
        let signature_bytes = match self.signatures.as_ref() {
            Some(sigs) => serialize_signature2::<TYPES>(sigs),
            None => vec![],
        };
        committable::RawCommitmentBuilder::new("Certificate")
            .field("data", self.data.commit())
            .field("vote_commitment", self.vote_commitment)
            .field("view number", self.view_number.commit())
            .var_size_field("signatures", &signature_bytes)
            .finalize()
    }
}

impl<TYPES: NodeType, THRESHOLD: Threshold<TYPES>> Certificate<TYPES, DaData>
    for SimpleCertificate<TYPES, DaData, THRESHOLD>
{
    type Voteable = DaData;
    type Threshold = THRESHOLD;

    fn create_signed_certificate<V: Versions>(
        vote_commitment: Commitment<VersionedVoteData<TYPES, DaData, V>>,
        data: Self::Voteable,
        sig: <TYPES::SignatureKey as SignatureKey>::QcType,
        view: TYPES::View,
    ) -> Self {
        let vote_commitment_bytes: [u8; 32] = vote_commitment.into();

        SimpleCertificate {
            data,
            vote_commitment: Commitment::from_raw(vote_commitment_bytes),
            view_number: view,
            signatures: Some(sig),
            _pd: PhantomData,
        }
    }
    async fn is_valid_cert<V: Versions>(
        &self,
        stake_table: Vec<<TYPES::SignatureKey as SignatureKey>::StakeTableEntry>,
        threshold: U256,
        upgrade_lock: &UpgradeLock<TYPES, V>,
    ) -> Result<()> {
        if self.view_number == TYPES::View::genesis() {
            return Ok(());
        }
        let real_qc_pp =
            <TYPES::SignatureKey as SignatureKey>::public_parameter(stake_table, threshold);
        let commit = self.data_commitment(upgrade_lock).await?;

        <TYPES::SignatureKey as SignatureKey>::check(
            &real_qc_pp,
            commit.as_ref(),
            self.signatures.as_ref().unwrap(),
        )
        .wrap()
        .context(|e| warn!("Signature check failed: {}", e))
    }
    /// Proxy's to `Membership.stake`
    async fn stake_table_entry(
        membership: &EpochMembership<TYPES>,
        pub_key: &TYPES::SignatureKey,
    ) -> Option<PeerConfig<TYPES>> {
        membership.da_stake(pub_key).await
    }

    /// Proxy's to `Membership.da_stake_table`
    async fn stake_table(membership: &EpochMembership<TYPES>) -> Vec<PeerConfig<TYPES>> {
        membership.da_stake_table().await
    }
    /// Proxy's to `Membership.da_total_nodes`
    async fn total_nodes(membership: &EpochMembership<TYPES>) -> usize {
        membership.da_total_nodes().await
    }
    async fn threshold(membership: &EpochMembership<TYPES>) -> U256 {
        membership.da_success_threshold().await
    }
    fn data(&self) -> &Self::Voteable {
        &self.data
    }
    async fn data_commitment<V: Versions>(
        &self,
        upgrade_lock: &UpgradeLock<TYPES, V>,
    ) -> Result<Commitment<VersionedVoteData<TYPES, DaData, V>>> {
        Ok(
            VersionedVoteData::new(self.data.clone(), self.view_number, upgrade_lock)
                .await?
                .commit(),
        )
    }
}

impl<TYPES: NodeType, THRESHOLD: Threshold<TYPES>> Certificate<TYPES, DaData2<TYPES>>
    for SimpleCertificate<TYPES, DaData2<TYPES>, THRESHOLD>
{
    type Voteable = DaData2<TYPES>;
    type Threshold = THRESHOLD;

    fn create_signed_certificate<V: Versions>(
        vote_commitment: Commitment<VersionedVoteData<TYPES, DaData2<TYPES>, V>>,
        data: Self::Voteable,
        sig: <TYPES::SignatureKey as SignatureKey>::QcType,
        view: TYPES::View,
    ) -> Self {
        let vote_commitment_bytes: [u8; 32] = vote_commitment.into();

        SimpleCertificate {
            data,
            vote_commitment: Commitment::from_raw(vote_commitment_bytes),
            view_number: view,
            signatures: Some(sig),
            _pd: PhantomData,
        }
    }
    async fn is_valid_cert<V: Versions>(
        &self,
        stake_table: Vec<<TYPES::SignatureKey as SignatureKey>::StakeTableEntry>,
        threshold: U256,
        upgrade_lock: &UpgradeLock<TYPES, V>,
    ) -> Result<()> {
        if self.view_number == TYPES::View::genesis() {
            return Ok(());
        }
        let real_qc_pp =
            <TYPES::SignatureKey as SignatureKey>::public_parameter(stake_table, threshold);
        let commit = self.data_commitment(upgrade_lock).await?;

        <TYPES::SignatureKey as SignatureKey>::check(
            &real_qc_pp,
            commit.as_ref(),
            self.signatures.as_ref().unwrap(),
        )
        .wrap()
        .context(|e| warn!("Signature check failed: {}", e))
    }
    /// Proxy's to `Membership.stake`
    async fn stake_table_entry(
        membership: &EpochMembership<TYPES>,
        pub_key: &TYPES::SignatureKey,
    ) -> Option<PeerConfig<TYPES>> {
        membership.da_stake(pub_key).await
    }

    /// Proxy's to `Membership.da_stake_table`
    async fn stake_table(membership: &EpochMembership<TYPES>) -> Vec<PeerConfig<TYPES>> {
        membership.da_stake_table().await
    }
    /// Proxy's to `Membership.da_total_nodes`
    async fn total_nodes(membership: &EpochMembership<TYPES>) -> usize {
        membership.da_total_nodes().await
    }
    async fn threshold(membership: &EpochMembership<TYPES>) -> U256 {
        membership.da_success_threshold().await
    }
    fn data(&self) -> &Self::Voteable {
        &self.data
    }
    async fn data_commitment<V: Versions>(
        &self,
        upgrade_lock: &UpgradeLock<TYPES, V>,
    ) -> Result<Commitment<VersionedVoteData<TYPES, DaData2<TYPES>, V>>> {
        Ok(
            VersionedVoteData::new(self.data.clone(), self.view_number, upgrade_lock)
                .await?
                .commit(),
        )
    }
}

impl<
        TYPES: NodeType,
        VOTEABLE: Voteable<TYPES> + 'static + QuorumMarker,
        THRESHOLD: Threshold<TYPES>,
    > Certificate<TYPES, VOTEABLE> for SimpleCertificate<TYPES, VOTEABLE, THRESHOLD>
{
    type Voteable = VOTEABLE;
    type Threshold = THRESHOLD;

    fn create_signed_certificate<V: Versions>(
        vote_commitment: Commitment<VersionedVoteData<TYPES, VOTEABLE, V>>,
        data: Self::Voteable,
        sig: <TYPES::SignatureKey as SignatureKey>::QcType,
        view: TYPES::View,
    ) -> Self {
        let vote_commitment_bytes: [u8; 32] = vote_commitment.into();

        SimpleCertificate {
            data,
            vote_commitment: Commitment::from_raw(vote_commitment_bytes),
            view_number: view,
            signatures: Some(sig),
            _pd: PhantomData,
        }
    }
    async fn is_valid_cert<V: Versions>(
        &self,
        stake_table: Vec<<TYPES::SignatureKey as SignatureKey>::StakeTableEntry>,
        threshold: U256,
        upgrade_lock: &UpgradeLock<TYPES, V>,
    ) -> Result<()> {
        if self.view_number == TYPES::View::genesis() {
            return Ok(());
        }
        let real_qc_pp =
            <TYPES::SignatureKey as SignatureKey>::public_parameter(stake_table, threshold);
        let commit = self.data_commitment(upgrade_lock).await?;

        <TYPES::SignatureKey as SignatureKey>::check(
            &real_qc_pp,
            commit.as_ref(),
            self.signatures.as_ref().unwrap(),
        )
        .wrap()
        .context(|e| warn!("Signature check failed: {}", e))
    }
    async fn threshold(membership: &EpochMembership<TYPES>) -> U256 {
        THRESHOLD::threshold(membership).await
    }

    async fn stake_table_entry(
        membership: &EpochMembership<TYPES>,
        pub_key: &TYPES::SignatureKey,
    ) -> Option<PeerConfig<TYPES>> {
        membership.stake(pub_key).await
    }

    async fn stake_table(membership: &EpochMembership<TYPES>) -> Vec<PeerConfig<TYPES>> {
        membership.stake_table().await
    }

    /// Proxy's to `Membership.total_nodes`
    async fn total_nodes(membership: &EpochMembership<TYPES>) -> usize {
        membership.total_nodes().await
    }

    fn data(&self) -> &Self::Voteable {
        &self.data
    }
    async fn data_commitment<V: Versions>(
        &self,
        upgrade_lock: &UpgradeLock<TYPES, V>,
    ) -> Result<Commitment<VersionedVoteData<TYPES, VOTEABLE, V>>> {
        Ok(
            VersionedVoteData::new(self.data.clone(), self.view_number, upgrade_lock)
                .await?
                .commit(),
        )
    }
}

impl<TYPES: NodeType, VOTEABLE: Voteable<TYPES> + 'static, THRESHOLD: Threshold<TYPES>>
    HasViewNumber<TYPES> for SimpleCertificate<TYPES, VOTEABLE, THRESHOLD>
{
    fn view_number(&self) -> TYPES::View {
        self.view_number
    }
}

impl<
        TYPES: NodeType,
        VOTEABLE: Voteable<TYPES> + HasEpoch<TYPES> + 'static,
        THRESHOLD: Threshold<TYPES>,
    > HasEpoch<TYPES> for SimpleCertificate<TYPES, VOTEABLE, THRESHOLD>
{
    fn epoch(&self) -> Option<TYPES::Epoch> {
        self.data.epoch()
    }
}

impl<TYPES: NodeType> Display for QuorumCertificate<TYPES> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "view: {:?}", self.view_number)
    }
}

impl<TYPES: NodeType> UpgradeCertificate<TYPES> {
    /// Determines whether or not a certificate is relevant (i.e. we still have time to reach a
    /// decide)
    ///
    /// # Errors
    /// Returns an error when the certificate is no longer relevant
    pub async fn is_relevant(
        &self,
        view_number: TYPES::View,
        decided_upgrade_certificate: Arc<RwLock<Option<Self>>>,
    ) -> Result<()> {
        let decided_upgrade_certificate_read = decided_upgrade_certificate.read().await;
        ensure!(
            self.data.decide_by >= view_number
                || decided_upgrade_certificate_read
                    .clone()
                    .is_some_and(|cert| cert == *self),
            "Upgrade certificate is no longer relevant."
        );

        Ok(())
    }

    /// Validate an upgrade certificate.
    /// # Errors
    /// Returns an error when the upgrade certificate is invalid.
    pub async fn validate<V: Versions>(
        upgrade_certificate: &Option<Self>,
        membership: &EpochMembership<TYPES>,
        epoch: Option<TYPES::Epoch>,
        upgrade_lock: &UpgradeLock<TYPES, V>,
    ) -> Result<()> {
        ensure!(epoch == membership.epoch(), "Epochs don't match!");
        if let Some(ref cert) = upgrade_certificate {
            let membership_stake_table = membership.stake_table().await;
            let membership_upgrade_threshold = membership.upgrade_threshold().await;

            cert.is_valid_cert(
                StakeTableEntries::<TYPES>::from(membership_stake_table).0,
                membership_upgrade_threshold,
                upgrade_lock,
            )
            .await
            .context(|e| warn!("Invalid upgrade certificate: {}", e))?;
        }

        Ok(())
    }

    /// Given an upgrade certificate and a view, tests whether the view is in the period
    /// where we are upgrading, which requires that we propose with null blocks.
    pub fn upgrading_in(&self, view: TYPES::View) -> bool {
        view > self.data.old_version_last_view && view < self.data.new_version_first_view
    }
}

impl<TYPES: NodeType> QuorumCertificate<TYPES> {
    /// Convert a `QuorumCertificate` into a `QuorumCertificate2`
    pub fn to_qc2(self) -> QuorumCertificate2<TYPES> {
        let bytes: [u8; 32] = self.data.leaf_commit.into();
        let data = QuorumData2 {
            leaf_commit: Commitment::from_raw(bytes),
            epoch: None,
            block_number: None,
        };

        let bytes: [u8; 32] = self.vote_commitment.into();
        let vote_commitment = Commitment::from_raw(bytes);

        SimpleCertificate {
            data,
            vote_commitment,
            view_number: self.view_number,
            signatures: self.signatures.clone(),
            _pd: PhantomData,
        }
    }
}

impl<TYPES: NodeType> QuorumCertificate2<TYPES> {
    /// Convert a `QuorumCertificate2` into a `QuorumCertificate`
    pub fn to_qc(self) -> QuorumCertificate<TYPES> {
        let bytes: [u8; 32] = self.data.leaf_commit.into();
        let data = QuorumData {
            leaf_commit: Commitment::from_raw(bytes),
        };

        let bytes: [u8; 32] = self.vote_commitment.into();
        let vote_commitment = Commitment::from_raw(bytes);

        SimpleCertificate {
            data,
            vote_commitment,
            view_number: self.view_number,
            signatures: self.signatures.clone(),
            _pd: PhantomData,
        }
    }
}

impl<TYPES: NodeType> DaCertificate<TYPES> {
    /// Convert a `DaCertificate` into a `DaCertificate2`
    pub fn to_dac2(self) -> DaCertificate2<TYPES> {
        let data = DaData2 {
            payload_commit: self.data.payload_commit,
            next_epoch_payload_commit: None,
            epoch: None,
        };

        let bytes: [u8; 32] = self.vote_commitment.into();
        let vote_commitment = Commitment::from_raw(bytes);

        SimpleCertificate {
            data,
            vote_commitment,
            view_number: self.view_number,
            signatures: self.signatures.clone(),
            _pd: PhantomData,
        }
    }
}

impl<TYPES: NodeType> DaCertificate2<TYPES> {
    /// Convert a `DaCertificate` into a `DaCertificate2`
    pub fn to_dac(self) -> DaCertificate<TYPES> {
        let data = DaData {
            payload_commit: self.data.payload_commit,
        };

        let bytes: [u8; 32] = self.vote_commitment.into();
        let vote_commitment = Commitment::from_raw(bytes);

        SimpleCertificate {
            data,
            vote_commitment,
            view_number: self.view_number,
            signatures: self.signatures.clone(),
            _pd: PhantomData,
        }
    }
}

impl<TYPES: NodeType> ViewSyncPreCommitCertificate<TYPES> {
    /// Convert a `DaCertificate` into a `DaCertificate2`
    pub fn to_vsc2(self) -> ViewSyncPreCommitCertificate2<TYPES> {
        let data = ViewSyncPreCommitData2 {
            relay: self.data.relay,
            round: self.data.round,
            epoch: None,
        };

        let bytes: [u8; 32] = self.vote_commitment.into();
        let vote_commitment = Commitment::from_raw(bytes);

        SimpleCertificate {
            data,
            vote_commitment,
            view_number: self.view_number,
            signatures: self.signatures.clone(),
            _pd: PhantomData,
        }
    }
}

impl<TYPES: NodeType> ViewSyncPreCommitCertificate2<TYPES> {
    /// Convert a `DaCertificate` into a `DaCertificate2`
    pub fn to_vsc(self) -> ViewSyncPreCommitCertificate<TYPES> {
        let data = ViewSyncPreCommitData {
            relay: self.data.relay,
            round: self.data.round,
        };

        let bytes: [u8; 32] = self.vote_commitment.into();
        let vote_commitment = Commitment::from_raw(bytes);

        SimpleCertificate {
            data,
            vote_commitment,
            view_number: self.view_number,
            signatures: self.signatures.clone(),
            _pd: PhantomData,
        }
    }
}

impl<TYPES: NodeType> ViewSyncCommitCertificate<TYPES> {
    /// Convert a `DaCertificate` into a `DaCertificate2`
    pub fn to_vsc2(self) -> ViewSyncCommitCertificate2<TYPES> {
        let data = ViewSyncCommitData2 {
            relay: self.data.relay,
            round: self.data.round,
            epoch: None,
        };

        let bytes: [u8; 32] = self.vote_commitment.into();
        let vote_commitment = Commitment::from_raw(bytes);

        SimpleCertificate {
            data,
            vote_commitment,
            view_number: self.view_number,
            signatures: self.signatures.clone(),
            _pd: PhantomData,
        }
    }
}

impl<TYPES: NodeType> ViewSyncCommitCertificate2<TYPES> {
    /// Convert a `DaCertificate` into a `DaCertificate2`
    pub fn to_vsc(self) -> ViewSyncCommitCertificate<TYPES> {
        let data = ViewSyncCommitData {
            relay: self.data.relay,
            round: self.data.round,
        };

        let bytes: [u8; 32] = self.vote_commitment.into();
        let vote_commitment = Commitment::from_raw(bytes);

        SimpleCertificate {
            data,
            vote_commitment,
            view_number: self.view_number,
            signatures: self.signatures.clone(),
            _pd: PhantomData,
        }
    }
}

impl<TYPES: NodeType> ViewSyncFinalizeCertificate<TYPES> {
    /// Convert a `DaCertificate` into a `DaCertificate2`
    pub fn to_vsc2(self) -> ViewSyncFinalizeCertificate2<TYPES> {
        let data = ViewSyncFinalizeData2 {
            relay: self.data.relay,
            round: self.data.round,
            epoch: None,
        };

        let bytes: [u8; 32] = self.vote_commitment.into();
        let vote_commitment = Commitment::from_raw(bytes);

        SimpleCertificate {
            data,
            vote_commitment,
            view_number: self.view_number,
            signatures: self.signatures.clone(),
            _pd: PhantomData,
        }
    }
}

impl<TYPES: NodeType> ViewSyncFinalizeCertificate2<TYPES> {
    /// Convert a `DaCertificate` into a `DaCertificate2`
    pub fn to_vsc(self) -> ViewSyncFinalizeCertificate<TYPES> {
        let data = ViewSyncFinalizeData {
            relay: self.data.relay,
            round: self.data.round,
        };

        let bytes: [u8; 32] = self.vote_commitment.into();
        let vote_commitment = Commitment::from_raw(bytes);

        SimpleCertificate {
            data,
            vote_commitment,
            view_number: self.view_number,
            signatures: self.signatures.clone(),
            _pd: PhantomData,
        }
    }
}

impl<TYPES: NodeType> TimeoutCertificate<TYPES> {
    /// Convert a `DaCertificate` into a `DaCertificate2`
    pub fn to_tc2(self) -> TimeoutCertificate2<TYPES> {
        let data = TimeoutData2 {
            view: self.data.view,
            epoch: None,
        };

        let bytes: [u8; 32] = self.vote_commitment.into();
        let vote_commitment = Commitment::from_raw(bytes);

        SimpleCertificate {
            data,
            vote_commitment,
            view_number: self.view_number,
            signatures: self.signatures.clone(),
            _pd: PhantomData,
        }
    }
}

impl<TYPES: NodeType> TimeoutCertificate2<TYPES> {
    /// Convert a `DaCertificate` into a `DaCertificate2`
    pub fn to_tc(self) -> TimeoutCertificate<TYPES> {
        let data = TimeoutData {
            view: self.data.view,
        };

        let bytes: [u8; 32] = self.vote_commitment.into();
        let vote_commitment = Commitment::from_raw(bytes);

        SimpleCertificate {
            data,
            vote_commitment,
            view_number: self.view_number,
            signatures: self.signatures.clone(),
            _pd: PhantomData,
        }
    }
}

/// Type alias for a `QuorumCertificate`, which is a `SimpleCertificate` over `QuorumData`
pub type QuorumCertificate<TYPES> = SimpleCertificate<TYPES, QuorumData<TYPES>, SuccessThreshold>;
/// Type alias for a `QuorumCertificate2`, which is a `SimpleCertificate` over `QuorumData2`
pub type QuorumCertificate2<TYPES> = SimpleCertificate<TYPES, QuorumData2<TYPES>, SuccessThreshold>;
/// Type alias for a `QuorumCertificate2`, which is a `SimpleCertificate` over `QuorumData2`
pub type NextEpochQuorumCertificate2<TYPES> =
    SimpleCertificate<TYPES, NextEpochQuorumData2<TYPES>, SuccessThreshold>;
/// Type alias for a `DaCertificate`, which is a `SimpleCertificate` over `DaData`
pub type DaCertificate<TYPES> = SimpleCertificate<TYPES, DaData, SuccessThreshold>;
/// Type alias for a `DaCertificate2`, which is a `SimpleCertificate` over `DaData2`
pub type DaCertificate2<TYPES> = SimpleCertificate<TYPES, DaData2<TYPES>, SuccessThreshold>;
/// Type alias for a Timeout certificate over a view number
pub type TimeoutCertificate<TYPES> = SimpleCertificate<TYPES, TimeoutData<TYPES>, SuccessThreshold>;
/// Type alias for a `TimeoutCertificate2`, which is a `SimpleCertificate` over `TimeoutData2`
pub type TimeoutCertificate2<TYPES> =
    SimpleCertificate<TYPES, TimeoutData2<TYPES>, SuccessThreshold>;
/// Type alias for a `ViewSyncPreCommit` certificate over a view number
pub type ViewSyncPreCommitCertificate<TYPES> =
    SimpleCertificate<TYPES, ViewSyncPreCommitData<TYPES>, OneHonestThreshold>;
/// Type alias for a `ViewSyncPreCommitCertificate2`, which is a `SimpleCertificate` over `ViewSyncPreCommitData2`
pub type ViewSyncPreCommitCertificate2<TYPES> =
    SimpleCertificate<TYPES, ViewSyncPreCommitData2<TYPES>, OneHonestThreshold>;
/// Type alias for a `ViewSyncCommit` certificate over a view number
pub type ViewSyncCommitCertificate<TYPES> =
    SimpleCertificate<TYPES, ViewSyncCommitData<TYPES>, SuccessThreshold>;
/// Type alias for a `ViewSyncCommitCertificate2`, which is a `SimpleCertificate` over `ViewSyncCommitData2`
pub type ViewSyncCommitCertificate2<TYPES> =
    SimpleCertificate<TYPES, ViewSyncCommitData2<TYPES>, SuccessThreshold>;
/// Type alias for a `ViewSyncFinalize` certificate over a view number
pub type ViewSyncFinalizeCertificate<TYPES> =
    SimpleCertificate<TYPES, ViewSyncFinalizeData<TYPES>, SuccessThreshold>;
/// Type alias for a `ViewSyncFinalizeCertificate2`, which is a `SimpleCertificate` over `ViewSyncFinalizeData2`
pub type ViewSyncFinalizeCertificate2<TYPES> =
    SimpleCertificate<TYPES, ViewSyncFinalizeData2<TYPES>, SuccessThreshold>;
/// Type alias for a `UpgradeCertificate`, which is a `SimpleCertificate` of `UpgradeProposalData`
pub type UpgradeCertificate<TYPES> =
    SimpleCertificate<TYPES, UpgradeProposalData<TYPES>, UpgradeThreshold>;

/// Type for light client state update certificate
#[derive(Serialize, Deserialize, Eq, Hash, PartialEq, Debug, Clone)]
pub struct LightClientStateUpdateCertificate<TYPES: NodeType> {
    /// The epoch of the light client state
    pub epoch: TYPES::Epoch,
    /// Light client state for epoch transition
    pub light_client_state: LightClientState,
    /// Signatures to the light client state
    pub signatures: Vec<(
        TYPES::StateSignatureKey,
        <TYPES::StateSignatureKey as StateSignatureKey>::StateSignature,
    )>,
}

impl<TYPES: NodeType> HasViewNumber<TYPES> for LightClientStateUpdateCertificate<TYPES> {
    fn view_number(&self) -> TYPES::View {
        TYPES::View::new(self.light_client_state.view_number)
    }
}

impl<TYPES: NodeType> HasEpoch<TYPES> for LightClientStateUpdateCertificate<TYPES> {
    fn epoch(&self) -> Option<TYPES::Epoch> {
        Some(self.epoch)
    }
}

impl<TYPES: NodeType> LightClientStateUpdateCertificate<TYPES> {
    pub fn genesis() -> Self {
        Self {
            epoch: TYPES::Epoch::genesis(),
            light_client_state: Default::default(),
            signatures: vec![],
        }
    }
}

#[derive(Serialize, Deserialize, Eq, Hash, PartialEq, Debug, Clone)]
#[serde(bound(deserialize = "QuorumCertificate2<TYPES>:for<'a> Deserialize<'a>"))]
pub struct EpochRootQuorumCertificate<TYPES: NodeType> {
    pub qc: QuorumCertificate2<TYPES>,
    pub state_cert: LightClientStateUpdateCertificate<TYPES>,
}

impl<TYPES: NodeType> HasViewNumber<TYPES> for EpochRootQuorumCertificate<TYPES> {
    fn view_number(&self) -> TYPES::View {
        self.qc.view_number()
    }
}

impl<TYPES: NodeType> HasEpoch<TYPES> for EpochRootQuorumCertificate<TYPES> {
    fn epoch(&self) -> Option<TYPES::Epoch> {
        self.qc.epoch()
    }
}
