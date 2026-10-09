//! The timeout certificate whose signers each sign the lock they hold.

use std::collections::BTreeMap;

use alloy_primitives::U256;
use bitvec::prelude::*;
use committable::{Commitment, Committable};
use hotshot_utils::anytrace::*;
use serde::{Deserialize, Serialize};

use super::QuorumCertificate2;
use crate::{
    data::{EpochNumber, ViewNumber, serialize_signature2},
    message::UpgradeLock,
    simple_vote::{HasEpoch, LockView, TimeoutData3, VersionedVoteData},
    traits::{
        node_implementation::NodeType,
        signature_key::{SignatureKey, StakeTableEntryType},
    },
    vote::HasViewNumber,
};

/// A timeout certificate over [`TimeoutData3`].
///
/// Every signer signs the lock it held when it timed out, so signers holding
/// different locks sign different data. Signers holding the same lock form one
/// group, whose signatures aggregate as in any other certificate. The
/// certificate's lock is the latest group's, and that group carries the
/// `Certificate1` that shows the lock exists: a signer cannot claim a lock that
/// no quorum certified. The lock is read from that certificate, so the two
/// cannot disagree.
#[derive(Serialize, Deserialize, Eq, Hash, PartialEq, Debug, Clone)]
#[serde(bound(deserialize = ""))]
pub struct TimeoutCertificate3<T: NodeType> {
    /// The view that timed out.
    pub view_number: ViewNumber,
    /// The epoch every signer named.
    pub epoch: EpochNumber,
    /// The signers holding the certificate's lock, the latest any signer holds.
    pub latest: LatestTimeoutSignatures<T>,
    /// The signers holding earlier locks, one aggregate per lock, earliest first.
    pub earlier: Vec<TimeoutSignatures<T>>,
}

/// The signers of a [`TimeoutCertificate3`] that hold its lock.
#[derive(Serialize, Deserialize, Eq, Hash, PartialEq, Debug, Clone)]
#[serde(bound(deserialize = ""))]
pub struct LatestTimeoutSignatures<T: NodeType> {
    /// The certificate of their lock; `None` if they are locked on nothing
    /// but genesis.
    pub lock_cert: Option<QuorumCertificate2<T>>,
    /// Their aggregate signature and signer bit vector.
    pub signatures: <T::SignatureKey as SignatureKey>::QcType,
}

/// The signers of a [`TimeoutCertificate3`] that hold the same lock, earlier
/// than the certificate's.
#[derive(Serialize, Deserialize, Eq, Hash, PartialEq, Debug, Clone)]
#[serde(bound(deserialize = ""))]
pub struct TimeoutSignatures<T: NodeType> {
    /// The lock every signer of this group signed.
    pub lock: Option<LockView>,
    /// Their aggregate signature and signer bit vector.
    pub signatures: <T::SignatureKey as SignatureKey>::QcType,
}

impl<T: NodeType> TimeoutCertificate3<T> {
    /// Aggregate signed timeout votes into a certificate.
    ///
    /// `votes` are each signer's lock and signature over
    /// `TimeoutData3 { view, epoch, lock }`; they are not checked here.
    /// `lock_cert` must certify the latest of their locks.
    pub fn assemble(
        view: ViewNumber,
        epoch: EpochNumber,
        stake_table: &[<T::SignatureKey as SignatureKey>::StakeTableEntry],
        votes: impl IntoIterator<
            Item = (
                Option<LockView>,
                T::SignatureKey,
                <T::SignatureKey as SignatureKey>::PureAssembledSignatureType,
            ),
        >,
        lock_cert: Option<QuorumCertificate2<T>>,
    ) -> Result<Self> {
        let mut by_lock: BTreeMap<Option<LockView>, BTreeMap<usize, _>> = BTreeMap::new();
        for (lock, key, signature) in votes {
            let Some(index) = stake_table
                .iter()
                .position(|entry| entry.public_key() == key)
            else {
                bail!("timeout vote signer {key} is not in the stake table");
            };
            by_lock.entry(lock).or_default().insert(index, signature);
        }
        let Some((latest_lock, latest)) = by_lock.pop_last() else {
            bail!("no timeout votes to assemble");
        };
        ensure!(
            lock_cert.as_ref().map(LockView::of) == latest_lock,
            "the lock certificate does not certify the latest lock of the votes"
        );
        let params = T::SignatureKey::public_parameter(stake_table, U256::ZERO);
        let aggregate = |signers: BTreeMap<usize, _>| {
            let mut bits = bitvec![0; stake_table.len()];
            for index in signers.keys() {
                bits.set(*index, true);
            }
            let signatures: Vec<_> = signers.into_values().collect();
            T::SignatureKey::assemble(&params, &bits, &signatures)
        };
        let earlier = by_lock
            .into_iter()
            .map(|(lock, signers)| TimeoutSignatures {
                lock,
                signatures: aggregate(signers),
            })
            .collect();
        Ok(Self {
            view_number: view,
            epoch,
            latest: LatestTimeoutSignatures {
                lock_cert,
                signatures: aggregate(latest),
            },
            earlier,
        })
    }

    /// The certificate's lock: the latest lock any of its signers signed.
    pub fn lock(&self) -> Option<LockView> {
        self.lock_cert().map(LockView::of)
    }

    /// The certificate of the certificate's lock.
    pub fn lock_cert(&self) -> Option<&QuorumCertificate2<T>> {
        self.latest.lock_cert.as_ref()
    }

    /// The data the signers of the group holding `lock` signed.
    pub fn data(&self, lock: Option<LockView>) -> TimeoutData3 {
        TimeoutData3 {
            view: self.view_number,
            epoch: self.epoch,
            lock,
        }
    }

    /// Every group: its lock and aggregate signature, earliest lock first.
    fn groups(
        &self,
    ) -> impl Iterator<Item = (Option<LockView>, &<T::SignatureKey as SignatureKey>::QcType)> {
        self.earlier
            .iter()
            .map(|group| (group.lock, &group.signatures))
            .chain([(self.lock(), &self.latest.signatures)])
    }

    /// Check everything but the lock certificate's own signatures.
    ///
    /// The earlier groups must be in lock order with no lock twice, all
    /// earlier than the certificate's lock, which must be no later than the
    /// timed out view, and no signer may be in two groups. Each group's
    /// signature must be valid over its own data, and together the groups
    /// must reach `threshold`. That the lock certificate is signed by a quorum
    /// of its own epoch is [`Self::lock_epoch`]'s committee's to check, and
    /// is not checked here: a certificate is valid only with both.
    pub fn check_signatures(
        &self,
        stake_table: &[<T::SignatureKey as SignatureKey>::StakeTableEntry],
        threshold: U256,
        upgrade_lock: &UpgradeLock<T>,
    ) -> Result<()> {
        let locks: Vec<_> = self.groups().map(|(lock, _)| lock).collect();
        ensure!(
            locks.windows(2).all(|w| w[0] < w[1]),
            "timeout certificate groups are not in strict lock order"
        );
        if let Some(lock) = self.lock() {
            ensure!(
                lock.view <= self.view_number,
                "timeout certificate lock {lock} is later than its view {}",
                self.view_number
            );
        }

        let params = T::SignatureKey::public_parameter(stake_table, U256::ZERO);
        let mut seen = bitvec![0; stake_table.len()];
        let mut weight = U256::ZERO;
        for (lock, signatures) in self.groups() {
            let commit =
                VersionedVoteData::new(self.data(lock), self.view_number, upgrade_lock)?.commit();
            T::SignatureKey::check(&params, commit.as_ref(), signatures)
                .wrap()
                .context(|e| warn!("timeout certificate signature check failed: {e}"))?;
            let (_, signers) = T::SignatureKey::sig_proof(signatures);
            ensure!(
                signers.len() == stake_table.len(),
                "signer bit vector does not match the stake table"
            );
            for (index, entry) in stake_table.iter().enumerate() {
                if signers[index] {
                    ensure!(
                        !seen[index],
                        "a timeout certificate signer is in two groups"
                    );
                    seen.set(index, true);
                    weight += entry.stake();
                }
            }
        }
        ensure!(
            weight >= threshold,
            "timeout certificate weight {weight} is below threshold {threshold}"
        );
        Ok(())
    }

    /// The epoch whose committee signed the lock certificate.
    pub fn lock_epoch(&self) -> Option<EpochNumber> {
        self.lock().map(|lock| lock.epoch)
    }
}

impl<T: NodeType> HasViewNumber for TimeoutCertificate3<T> {
    fn view_number(&self) -> ViewNumber {
        self.view_number
    }
}

impl<T: NodeType> HasEpoch for TimeoutCertificate3<T> {
    fn epoch(&self) -> Option<EpochNumber> {
        Some(self.epoch)
    }
}

impl<T: NodeType> Committable for TimeoutCertificate3<T> {
    fn commit(&self) -> Commitment<Self> {
        let mut builder = committable::RawCommitmentBuilder::new("Timeout certificate v3")
            .u64_field("view number", *self.view_number)
            .u64_field("epoch number", *self.epoch)
            .u64_field("earlier groups", self.earlier.len() as u64);
        for (lock, signatures) in self.groups() {
            builder = builder
                .field("data", self.data(lock).commit())
                .var_size_field("signatures", &serialize_signature2::<T>(signatures));
        }
        builder
            .optional("lock certificate", &self.latest.lock_cert)
            .finalize()
    }
}
