use std::{future::Future, sync::Arc};

use alloy::primitives::U256;
use anyhow::{Context, Result, bail, ensure};
use committable::Committable;
use espresso_types::{Certificate2, Leaf2, PubKey, SeqTypes};
use hotshot_types::{
    epoch_membership::EpochMembership,
    message::UpgradeLock,
    simple_certificate::CertificatePair,
    stake_table::{HSStakeTable, StakeTableEntries, StakeTableEntry, supermajority_threshold},
    vote::{self, HasViewNumber},
};
use tracing::Instrument;
use vbs::version::{StaticVersion, StaticVersionType, Version};
use versions::{EPOCH_VERSION, MAX_SUPPORTED_VERSION, Upgrade, version};

pub type Certificate = CertificatePair<SeqTypes>;

pub trait Quorum: Sync {
    /// Check a threshold signature on a quorum certificate.
    fn verify(
        &self,
        cert: &Certificate,
        version: Version,
    ) -> impl Send + Future<Output = Result<()>> {
        async move {
            match (version.major, version.minor) {
                (0, 1) => self.verify_static::<StaticVersion<0, 1>>(cert).await,
                (0, 2) => self.verify_static::<StaticVersion<0, 2>>(cert).await,
                (0, 3) => self.verify_static::<StaticVersion<0, 3>>(cert).await,
                (0, 4) => self.verify_static::<StaticVersion<0, 4>>(cert).await,
                (0, 5) => self.verify_static::<StaticVersion<0, 5>>(cert).await,
                (0, 6) => self.verify_static::<StaticVersion<0, 6>>(cert).await,
                _ => {
                    const {
                        assert!(MAX_SUPPORTED_VERSION.major == 0);
                        assert!(MAX_SUPPORTED_VERSION.minor == 6);
                    }
                    bail!("unsupported version {version}");
                },
            }
        }
    }

    /// Same as [`verify`](Self::verify), but with the version as a type-level parameter.
    fn verify_static<V: StaticVersionType + 'static>(
        &self,
        qc: &Certificate,
    ) -> impl Send + Future<Output = Result<()>>;

    /// Verify a new protocol Certificate2 threshold signature.
    fn verify_cert2(
        &self,
        cert2: &Certificate2<SeqTypes>,
        version: Version,
    ) -> impl Send + Future<Output = Result<()>> {
        async move {
            match (version.major, version.minor) {
                (0, 1) => self.verify_cert2_static::<StaticVersion<0, 1>>(cert2).await,
                (0, 2) => self.verify_cert2_static::<StaticVersion<0, 2>>(cert2).await,
                (0, 3) => self.verify_cert2_static::<StaticVersion<0, 3>>(cert2).await,
                (0, 4) => self.verify_cert2_static::<StaticVersion<0, 4>>(cert2).await,
                (0, 5) => self.verify_cert2_static::<StaticVersion<0, 5>>(cert2).await,
                (0, 6) => self.verify_cert2_static::<StaticVersion<0, 6>>(cert2).await,
                _ => {
                    const {
                        assert!(MAX_SUPPORTED_VERSION.major == 0);
                        assert!(MAX_SUPPORTED_VERSION.minor == 6);
                    }
                    bail!("unsupported version {version}");
                },
            }
        }
    }

    /// Same as [`verify_cert2`](Self::verify_cert2), but with the version as a type-level parameter.
    fn verify_cert2_static<V: StaticVersionType + 'static>(
        &self,
        cert2: &Certificate2<SeqTypes>,
    ) -> impl Send + Future<Output = Result<()>>;

    /// Verify that QCs are signed, form a chain starting from `leaf`, with a particular protocol
    /// version.
    ///
    /// This check forms the bulk of the commit rule for both HotStuff and HotStuff2.
    fn verify_qc_chain_and_get_version<'a>(
        &self,
        leaf: &Leaf2,
        certs: impl Send + IntoIterator<Item = &'a Certificate, IntoIter: Send>,
    ) -> impl Send + Future<Output = Result<ChainVersions>> {
        let span = tracing::trace_span!(
            "verify_qc_chain_and_get_version",
            height = leaf.block_header().height()
        );
        async move {
            // Get the protocol version that the leaf claims it is using. At this point, the leaf is
            // not trusted, but we will verify that this quorum (the root of trust in the system)
            // has produced a threshold signature on this leaf, including the version number, before
            // we act on that version.
            //
            // The only reason we need to read the version before checking this signature is that
            // the version feeds into the commitment that the signature is over.
            let version = leaf.block_header().version();
            // Similarly, check if the protocol version is supposed to change at some point in the
            // middle of the QC chain. Any valid (signed by this quorum) leaf that is within a few
            // views of an upgrade taking effect will have an upgrade certificate attached telling
            // us so.
            let upgrade = leaf.upgrade_certificate();
            // Enforce that this version of the software supports these protocol versions. If we see
            // a version from the future, we must fail because we don't necessarily know how to
            // treat objects with this version.
            ensure!(version <= MAX_SUPPORTED_VERSION);
            if let Some(cert) = &upgrade {
                ensure!(cert.data.new_version <= MAX_SUPPORTED_VERSION);
            }
            tracing::debug!(
                %version,
                ?leaf,
                "verify QC chain for leaf"
            );

            // Check the QC chain: valid signatures and sequential views.
            let mut first = None;
            let mut curr: Option<&Certificate> = None;
            let mut max_cert_version = version;
            for cert in certs {
                tracing::trace!(?cert, "verify cert");

                // What version number do we expect the quorum to have signed over?
                let cert_version = match &upgrade {
                    Some(upgrade) if cert.view_number() >= upgrade.data.new_version_first_view => {
                        tracing::debug!(?upgrade, view = ?cert.view_number(), "using upgraded version");
                        upgrade.data.new_version
                    },
                    _ => version,
                };
                if cert_version > max_cert_version {
                    max_cert_version = cert_version;
                }

                // Check the signature.
<<<<<<< HEAD
                self.verify(cert, version).await?;
||||||| parent of 82b8967ccf8 (fix(light-client): reject HotStuff2 finality proofs across the cutover (#4722))
                if next_epoch {
                    self.verify_next_epoch(cert, version).await?;
                } else {
                    self.verify(cert, version).await?;
                }
=======
                if next_epoch {
                    self.verify_next_epoch(cert, cert_version).await?;
                } else {
                    self.verify(cert, cert_version).await?;
                }
>>>>>>> 82b8967ccf8 (fix(light-client): reject HotStuff2 finality proofs across the cutover (#4722))

                // Check chaining.
                if let Some(prev) = curr {
                    ensure!(cert.view_number() == prev.view_number() + 1);
                }
                curr = Some(cert);

                // Save the first QC.
                if first.is_none() {
                    first = Some(cert);
                }
            }

            // Check that the first QC in the chain signs the required leaf.
            let first_qc = first.context("empty QC chain")?;
            ensure!(first_qc.leaf_commit() == leaf.commit());

            Ok(ChainVersions {
                leaf: version,
                max_cert: max_cert_version,
            })
        }
        .instrument(span)
    }
}

/// The protocol versions extracted while verifying a QC chain.
#[derive(Clone, Copy, Debug)]
pub struct ChainVersions {
    /// The protocol version of the leaf the chain proves finalized.
    pub leaf: Version,

    /// The highest protocol version any certificate in the chain was verified under.
    ///
    /// May exceed [`leaf`](Self::leaf) when an upgrade takes effect within the chain.
    pub max_cert: Version,
}

/// A stake table representing a particular quorum.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StakeTable {
    entries: Vec<StakeTableEntry<PubKey>>,
    threshold: U256,
}

impl From<HSStakeTable<SeqTypes>> for StakeTable {
    fn from(table: HSStakeTable<SeqTypes>) -> Self {
        StakeTableEntries::from(table).into()
    }
}

impl From<Vec<StakeTableEntry<PubKey>>> for StakeTable {
    fn from(entries: Vec<StakeTableEntry<PubKey>>) -> Self {
        StakeTableEntries(entries).into()
    }
}

impl From<StakeTableEntries<SeqTypes>> for StakeTable {
    fn from(entries: StakeTableEntries<SeqTypes>) -> Self {
        Self::from_iter(entries.0)
    }
}

impl FromIterator<StakeTableEntry<PubKey>> for StakeTable {
    fn from_iter<T: IntoIterator<Item = StakeTableEntry<PubKey>>>(entries: T) -> Self {
        let mut total_stake = U256::ZERO;
        let entries = entries
            .into_iter()
            .inspect(|entry| {
                total_stake += entry.stake_amount;
            })
            .collect();
        Self {
            entries,
            threshold: supermajority_threshold(total_stake),
        }
    }
}

impl StakeTable {
    /// Get a stake table from a particular epoch's quorum membership.
    pub async fn from_membership(membership: &EpochMembership<SeqTypes>) -> Self {
        HSStakeTable::from_iter(membership.stake_table()).into()
    }

    /// Verify that a certificate is signed by a quorum of this stake table.
    pub async fn verify_cert<V, T>(&self, cert: &impl vote::Certificate<SeqTypes, T>) -> Result<()>
    where
        V: StaticVersionType + 'static,
    {
        let upgrade = Upgrade::trivial(version(V::MAJOR, V::MINOR));
        cert.is_valid_cert(&self.entries, self.threshold, &UpgradeLock::new(upgrade))
            .context("invalid threshold signature")
    }
}

/// Getters for the current epoch's stake table and the next.
///
/// The current [`stake_table`](StakeTablePair::stake_table) is always needed to verify a
/// [`Certificate`] from this epoch. Depending on the [`Certificate`], the next epoch's stake table
/// may also need to be fetched (in the case where the certificate is part of an epoch transition).
pub trait StakeTablePair {
    /// Get the stake table for the current epoch.
    fn stake_table(&self) -> impl Send + Future<Output = Result<Arc<StakeTable>>>;

    /// Get the stake table for the next epoch.
    fn next_epoch_stake_table(&self) -> impl Send + Future<Output = Result<Arc<StakeTable>>>;
}

impl StakeTablePair for EpochMembership<SeqTypes> {
    async fn stake_table(&self) -> Result<Arc<StakeTable>> {
        Ok(Arc::new(StakeTable::from_membership(self).await))
    }

    async fn next_epoch_stake_table(&self) -> Result<Arc<StakeTable>> {
        let membership = self.next_epoch_stake_table()?;
        Ok(Arc::new(StakeTable::from_membership(&membership).await))
    }
}

impl StakeTablePair for (Arc<StakeTable>, Arc<StakeTable>) {
    async fn stake_table(&self) -> Result<Arc<StakeTable>> {
        Ok(self.0.clone())
    }

    async fn next_epoch_stake_table(&self) -> Result<Arc<StakeTable>> {
        Ok(self.1.clone())
    }
}

/// A quorum based on a [`StakeTablePair`] for a particular epoch.
#[derive(Clone, Debug)]
pub struct StakeTableQuorum<T> {
    membership: T,
    epoch_height: u64,
}

impl<T> StakeTableQuorum<T> {
    /// Construct a quorum given a [`StakeTablePair`] and the epoch height.
    pub fn new(membership: T, epoch_height: u64) -> Self {
        Self {
            membership,
            epoch_height,
        }
    }
}

impl<T> Quorum for StakeTableQuorum<T>
where
    T: StakeTablePair + Sync,
{
    async fn verify_static<V: StaticVersionType + 'static>(
        &self,
        cert: &Certificate,
    ) -> Result<()> {
        let stake_table = self.membership.stake_table().await?;
        stake_table
            .verify_cert::<V, _>(cert.qc())
            .await
            .context("verifying QC")?;

        if version(V::MAJOR, V::MINOR) >= EPOCH_VERSION {
            // If this certificate is part of an epoch change, also check that the next epoch's
            // quorum has signed.
            if let Some(next_epoch_qc) = cert.verify_next_epoch_qc(self.epoch_height)? {
                let stake_table = self.membership.next_epoch_stake_table().await?;
                stake_table
                    .verify_cert::<V, _>(next_epoch_qc)
                    .await
                    .context("verifying next epoch QC")?;
            }
        }

        Ok(())
    }

    async fn verify_cert2_static<V: StaticVersionType + 'static>(
        &self,
        cert2: &Certificate2<SeqTypes>,
    ) -> Result<()> {
        let stake_table = self.membership.stake_table().await?;
        stake_table
            .verify_cert::<V, _>(cert2)
            .await
            .context("verifying cert2")
    }
}

#[cfg(test)]
mod test {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::testing::{
        AlwaysFalseQuorum, AlwaysTrueQuorum, ENABLE_EPOCHS, EpochChangeQuorum, LEGACY_VERSION,
        VersionCheckQuorum, custom_epoch_change_leaf_chain, custom_leaf_chain_with_upgrade,
        epoch_change_leaf_chain, leaf_chain, leaf_chain_with_upgrade, qc_chain_from_leaf_chain,
    };

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_valid_chain() {
        let leaves = leaf_chain(1..=3, EPOCH_VERSION).await;
        let ChainVersions { leaf: version, .. } = AlwaysTrueQuorum
            .verify_qc_chain_and_get_version(
                leaves[0].leaf(),
                &qc_chain_from_leaf_chain(&leaves[1..]),
            )
            .await
            .unwrap();
        assert_eq!(version, leaves[0].header().version());
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_wrong_leaf() {
        let leaves = leaf_chain(1..=3, EPOCH_VERSION).await;
        AlwaysTrueQuorum
            .verify_qc_chain_and_get_version(
                leaves[2].leaf(),
                &qc_chain_from_leaf_chain(&leaves[1..]),
            )
            .await
            .unwrap_err();
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_invalid_qc() {
        let leaves = leaf_chain(1..=2, EPOCH_VERSION).await;
        AlwaysFalseQuorum
            .verify_qc_chain_and_get_version(
                leaves[0].leaf(),
                &[Certificate::for_parent(leaves[1].leaf())],
            )
            .await
            .unwrap_err();
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_non_consecutive() {
        let leaves = leaf_chain(1..=4, EPOCH_VERSION).await;
        AlwaysTrueQuorum
            .verify_qc_chain_and_get_version(
                leaves[0].leaf(),
                &qc_chain_from_leaf_chain([&leaves[1], &leaves[3]]),
            )
            .await
            .unwrap_err();
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_upgrade() {
        let leaves = leaf_chain_with_upgrade(1..=3, 2, ENABLE_EPOCHS).await;
        let ChainVersions { leaf: version, .. } =
            VersionCheckQuorum::new(leaves.iter().map(|leaf| leaf.leaf().clone()))
                .verify_qc_chain_and_get_version(
                    leaves[0].leaf(),
                    &qc_chain_from_leaf_chain(&leaves[1..]),
                )
                .await
                .unwrap();
        assert_eq!(version, leaves[0].header().version());
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_illegal_upgrade() {
        let leaves = custom_leaf_chain_with_upgrade(1..=3, 2, ENABLE_EPOCHS, |proposal| {
            // Don't attach an upgrade certificate, so that the version change that happens within
            // the QC change is actually malicious.
            proposal.upgrade_certificate = None;
        })
        .await;
        VersionCheckQuorum::new(leaves.iter().map(|leaf| leaf.leaf().clone()))
            .verify_qc_chain_and_get_version(
                leaves[0].leaf(),
                &qc_chain_from_leaf_chain(&leaves[1..]),
            )
            .await
            .unwrap_err();
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_epoch_change() {
        let leaves = epoch_change_leaf_chain(1..=5, 5, EPOCH_VERSION).await;
        let ChainVersions { leaf: version, .. } = EpochChangeQuorum::new(5)
            .verify_qc_chain_and_get_version(
                leaves[0].leaf(),
                &qc_chain_from_leaf_chain(&leaves[1..]),
            )
            .await
            .unwrap();
        assert_eq!(version, leaves[0].header().version());
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_epoch_change_missing_eqc() {
        let leaves = custom_epoch_change_leaf_chain(1..=5, 5, EPOCH_VERSION, |proposal| {
            // Delete the next epoch justify QC, making this an invalid epoch change QC.
            proposal.next_epoch_justify_qc = None;
        })
        .await;
        EpochChangeQuorum::new(5)
            .verify_qc_chain_and_get_version(
                leaves[0].leaf(),
                &qc_chain_from_leaf_chain(&leaves[1..]),
            )
            .await
            .unwrap_err();
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_epoch_change_inconsistent_eqc_view_number() {
        let leaves = custom_epoch_change_leaf_chain(1..=5, 5, EPOCH_VERSION, |proposal| {
            // Tamper with the next epoch justify QC, making this an invalid epoch change QC.
            if let Some(next_epoch_justify_qc) = &mut proposal.next_epoch_justify_qc {
                next_epoch_justify_qc.view_number += 1;
            }
        })
        .await;
        EpochChangeQuorum::new(5)
            .verify_qc_chain_and_get_version(
                leaves[0].leaf(),
                &qc_chain_from_leaf_chain(&leaves[1..]),
            )
            .await
            .unwrap_err();
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_epoch_change_inconsistent_eqc_data() {
        let leaves = custom_epoch_change_leaf_chain(1..=5, 5, EPOCH_VERSION, |proposal| {
            // Tamper with the next epoch justify QC, making this an invalid epoch change QC.
            if let Some(next_epoch_justify_qc) = &mut proposal.next_epoch_justify_qc {
                *next_epoch_justify_qc.data.block_number.as_mut().unwrap() += 1;
            }
        })
        .await;
        EpochChangeQuorum::new(5)
            .verify_qc_chain_and_get_version(
                leaves[0].leaf(),
                &qc_chain_from_leaf_chain(&leaves[1..]),
            )
            .await
            .unwrap_err();
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_epoch_change_absent_eqc_before_upgrade() {
        let leaves = custom_epoch_change_leaf_chain(1..=5, 5, LEGACY_VERSION, |proposal| {
            // Delete the next epoch justify QC; this is allowed since epochs are not enabled yet.
            proposal.next_epoch_justify_qc = None;
        })
        .await;
        let ChainVersions { leaf: version, .. } = EpochChangeQuorum::new(5)
            .verify_qc_chain_and_get_version(
                leaves[0].leaf(),
                &qc_chain_from_leaf_chain(&leaves[1..]),
            )
            .await
            .unwrap();
        assert_eq!(version, leaves[0].header().version());
    }
<<<<<<< HEAD
||||||| parent of 82b8967ccf8 (fix(light-client): reject HotStuff2 finality proofs across the cutover (#4722))

    const BOUNDARY_EPOCH_HEIGHT: u64 = 10;

    fn boundary_quorum(seeds: impl IntoIterator<Item = u64>) -> QuorumKeys {
        seeds
            .into_iter()
            .map(|i| {
                let (stake_key, priv_key) =
                    PubKey::generated_from_seed_indexed(Default::default(), i);
                (
                    priv_key,
                    StakeTableEntry {
                        stake_key,
                        stake_amount: U256::from(1),
                    },
                )
            })
            .unzip()
    }

    type QuorumKeys = (Vec<PrivKey>, Vec<StakeTableEntry<PubKey>>);

    fn boundary_signature(
        msg: &[u8],
        (keys, entries): &QuorumKeys,
    ) -> <PubKey as SignatureKey>::QcType {
        let total = entries
            .iter()
            .fold(U256::ZERO, |acc, entry| acc + entry.stake_amount);
        let pp = PubKey::public_parameter(entries, supermajority_threshold(total));
        let sigs = keys
            .iter()
            .map(|key| PubKey::sign(key, msg).unwrap())
            .collect::<Vec<_>>();
        PubKey::assemble(
            &pp,
            &std::iter::repeat_n(true, keys.len()).collect::<BitVec>(),
            &sigs,
        )
    }

    fn boundary_signed_qc(
        data: QuorumData2<SeqTypes>,
        view: ViewNumber,
        quorum: &QuorumKeys,
    ) -> QuorumCertificate2<SeqTypes> {
        let commit = VersionedVoteData::new_infallible(
            data,
            view,
            &UpgradeLock::<SeqTypes>::new(Upgrade::trivial(EPOCH_VERSION)),
        )
        .commit();
        let sig = boundary_signature(commit.as_ref(), quorum);
        QuorumCertificate2::create_signed_certificate(commit, data, sig, view)
    }

    fn boundary_signed_next_epoch_qc(
        data: QuorumData2<SeqTypes>,
        view: ViewNumber,
        quorum: &QuorumKeys,
    ) -> NextEpochQuorumCertificate2<SeqTypes> {
        let data: NextEpochQuorumData2<SeqTypes> = data.into();
        let commit = VersionedVoteData::new_infallible(
            data.clone(),
            view,
            &UpgradeLock::<SeqTypes>::new(Upgrade::trivial(EPOCH_VERSION)),
        )
        .commit();
        let commit_bytes: [u8; 32] = commit.into();
        let sig = boundary_signature(commit.as_ref(), quorum);
        NextEpochQuorumCertificate2::new(
            data,
            Commitment::from_raw(commit_bytes),
            view,
            Some(sig),
            Default::default(),
        )
    }

    /// Build a 2-chain proving the last leaf of epoch 1 (block 10), where epochs 1 and 2 have
    /// disjoint quorums. The deciding QC is produced in epoch 2. If `deciding_signed_by_next` it
    /// is correctly signed by epoch 2's quorum; otherwise it is (invalidly) signed by epoch 1's
    /// quorum.
    async fn epoch_boundary_fixture(
        deciding_signed_by_next: bool,
    ) -> (
        Vec<LeafQueryData<SeqTypes>>,
        Certificate,
        Certificate,
        StakeTableQuorum<(Arc<StakeTable>, Arc<StakeTable>)>,
    ) {
        let current = boundary_quorum(0..5);
        let next = boundary_quorum(5..10);

        let leaves = leaf_chain(9..=11, EPOCH_VERSION).await;

        // Block 10 is an epoch transition block, so its QC is dual-signed by both quorums.
        let committing_data = QuorumData2 {
            leaf_commit: Committable::commit(leaves[1].leaf()),
            epoch: Some(EpochNumber::new(1)),
            block_number: Some(10),
        };
        let committing_qc = Certificate::new(
            boundary_signed_qc(committing_data, ViewNumber::new(10), &current),
            Some(boundary_signed_next_epoch_qc(
                committing_data,
                ViewNumber::new(10),
                &next,
            )),
        );

        let deciding_quorum = if deciding_signed_by_next {
            &next
        } else {
            &current
        };
        let deciding_qc = Certificate::non_epoch_change(boundary_signed_qc(
            QuorumData2 {
                leaf_commit: Committable::commit(leaves[2].leaf()),
                epoch: Some(EpochNumber::new(2)),
                block_number: Some(11),
            },
            ViewNumber::new(11),
            deciding_quorum,
        ));

        let quorum = StakeTableQuorum::new(
            (
                Arc::new(StakeTable::from(current.1)),
                Arc::new(StakeTable::from(next.1)),
            ),
            BOUNDARY_EPOCH_HEIGHT,
        );
        (leaves, committing_qc, deciding_qc, quorum)
    }

    /// A 2-chain proving the last leaf of an epoch includes a deciding QC signed by the next
    /// epoch's quorum; it must be verified against that quorum, not the quorum of the epoch of the
    /// leaf under proof.
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_epoch_boundary_quorum_change() {
        let (leaves, committing_qc, deciding_qc, quorum) = epoch_boundary_fixture(true).await;
        let version = quorum
            .verify_qc_chain_and_get_version(leaves[1].leaf(), [&committing_qc, &deciding_qc])
            .await
            .unwrap();
        assert_eq!(version, leaves[1].header().version());
    }

    /// A deciding QC claiming to be from the next epoch but signed by the current epoch's quorum
    /// must fail verification.
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_epoch_boundary_deciding_qc_wrong_quorum() {
        let (leaves, committing_qc, deciding_qc, quorum) = epoch_boundary_fixture(false).await;
        quorum
            .verify_qc_chain_and_get_version(leaves[1].leaf(), [&committing_qc, &deciding_qc])
            .await
            .unwrap_err();
    }

    /// Build a 2-chain over an interior (non-boundary) leaf of epoch 1 whose deciding QC is forged
    /// by epoch 2's quorum, mimicking a next epoch trying to finalize a leaf the current epoch
    /// never decided. If `disguise_as_boundary`, the committing QC also carries a next-epoch QC to
    /// imitate a genuine epoch-transition committing QC.
    async fn mid_epoch_forgery_fixture(
        disguise_as_boundary: bool,
    ) -> (
        Vec<LeafQueryData<SeqTypes>>,
        Certificate,
        Certificate,
        StakeTableQuorum<(Arc<StakeTable>, Arc<StakeTable>)>,
    ) {
        let current = boundary_quorum(0..5);
        let next = boundary_quorum(5..10);

        // Block 5 is in the interior of epoch 1 (epoch height 10), not an epoch boundary.
        let leaves = leaf_chain(4..=6, EPOCH_VERSION).await;

        let committing_data = QuorumData2 {
            leaf_commit: Committable::commit(leaves[1].leaf()),
            epoch: Some(EpochNumber::new(1)),
            block_number: Some(5),
        };
        let committing_qc = Certificate::new(
            boundary_signed_qc(committing_data, ViewNumber::new(5), &current),
            disguise_as_boundary
                .then(|| boundary_signed_next_epoch_qc(committing_data, ViewNumber::new(5), &next)),
        );

        let deciding_qc = Certificate::non_epoch_change(boundary_signed_qc(
            QuorumData2 {
                leaf_commit: Committable::commit(leaves[2].leaf()),
                epoch: Some(EpochNumber::new(2)),
                block_number: Some(6),
            },
            ViewNumber::new(6),
            &next,
        ));

        let quorum = StakeTableQuorum::new(
            (
                Arc::new(StakeTable::from(current.1)),
                Arc::new(StakeTable::from(next.1)),
            ),
            BOUNDARY_EPOCH_HEIGHT,
        );
        (leaves, committing_qc, deciding_qc, quorum)
    }

    /// The next epoch's quorum must not be able to finalize an interior leaf of the previous epoch:
    /// only the last leaf of an epoch is justified by the next epoch, so a deciding QC signed by
    /// the next epoch over a non-boundary leaf must be rejected.
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_mid_epoch_next_epoch_forgery_rejected() {
        let (leaves, committing_qc, deciding_qc, quorum) = mid_epoch_forgery_fixture(false).await;
        quorum
            .verify_qc_chain_and_get_version(leaves[1].leaf(), [&committing_qc, &deciding_qc])
            .await
            .unwrap_err();
    }

    /// The same forgery must be rejected even when the committing QC is dressed up with a
    /// next-epoch QC to imitate a genuine epoch-transition committing QC.
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_mid_epoch_next_epoch_forgery_with_fake_transition_rejected() {
        let (leaves, committing_qc, deciding_qc, quorum) = mid_epoch_forgery_fixture(true).await;
        quorum
            .verify_qc_chain_and_get_version(leaves[1].leaf(), [&committing_qc, &deciding_qc])
            .await
            .unwrap_err();
    }
=======

    const BOUNDARY_EPOCH_HEIGHT: u64 = 10;

    fn boundary_quorum(seeds: impl IntoIterator<Item = u64>) -> QuorumKeys {
        seeds
            .into_iter()
            .map(|i| {
                let (stake_key, priv_key) =
                    PubKey::generated_from_seed_indexed(Default::default(), i);
                (
                    priv_key,
                    StakeTableEntry {
                        stake_key,
                        stake_amount: U256::from(1),
                    },
                )
            })
            .unzip()
    }

    type QuorumKeys = (Vec<PrivKey>, Vec<StakeTableEntry<PubKey>>);

    fn boundary_signature(
        msg: &[u8],
        (keys, entries): &QuorumKeys,
    ) -> <PubKey as SignatureKey>::QcType {
        let total = entries
            .iter()
            .fold(U256::ZERO, |acc, entry| acc + entry.stake_amount);
        let pp = PubKey::public_parameter(entries, supermajority_threshold(total));
        let sigs = keys
            .iter()
            .map(|key| PubKey::sign(key, msg).unwrap())
            .collect::<Vec<_>>();
        PubKey::assemble(
            &pp,
            &std::iter::repeat_n(true, keys.len()).collect::<BitVec>(),
            &sigs,
        )
    }

    fn boundary_signed_qc(
        data: QuorumData2<SeqTypes>,
        view: ViewNumber,
        quorum: &QuorumKeys,
    ) -> QuorumCertificate2<SeqTypes> {
        let commit = VersionedVoteData::new_infallible(
            data,
            view,
            &UpgradeLock::<SeqTypes>::new(Upgrade::trivial(EPOCH_VERSION)),
        )
        .commit();
        let sig = boundary_signature(commit.as_ref(), quorum);
        QuorumCertificate2::create_signed_certificate(commit, data, sig, view)
    }

    fn boundary_signed_next_epoch_qc(
        data: QuorumData2<SeqTypes>,
        view: ViewNumber,
        quorum: &QuorumKeys,
    ) -> NextEpochQuorumCertificate2<SeqTypes> {
        let data: NextEpochQuorumData2<SeqTypes> = data.into();
        let commit = VersionedVoteData::new_infallible(
            data.clone(),
            view,
            &UpgradeLock::<SeqTypes>::new(Upgrade::trivial(EPOCH_VERSION)),
        )
        .commit();
        let commit_bytes: [u8; 32] = commit.into();
        let sig = boundary_signature(commit.as_ref(), quorum);
        NextEpochQuorumCertificate2::new(
            data,
            Commitment::from_raw(commit_bytes),
            view,
            Some(sig),
            Default::default(),
        )
    }

    /// Build a 2-chain proving the last leaf of epoch 1 (block 10), where epochs 1 and 2 have
    /// disjoint quorums. The deciding QC is produced in epoch 2. If `deciding_signed_by_next` it
    /// is correctly signed by epoch 2's quorum; otherwise it is (invalidly) signed by epoch 1's
    /// quorum.
    async fn epoch_boundary_fixture(
        deciding_signed_by_next: bool,
    ) -> (
        Vec<LeafQueryData<SeqTypes>>,
        Certificate,
        Certificate,
        StakeTableQuorum<(Arc<StakeTable>, Arc<StakeTable>)>,
    ) {
        let current = boundary_quorum(0..5);
        let next = boundary_quorum(5..10);

        let leaves = leaf_chain(9..=11, EPOCH_VERSION).await;

        // Block 10 is an epoch transition block, so its QC is dual-signed by both quorums.
        let committing_data = QuorumData2 {
            leaf_commit: Committable::commit(leaves[1].leaf()),
            epoch: Some(EpochNumber::new(1)),
            block_number: Some(10),
        };
        let committing_qc = Certificate::new(
            boundary_signed_qc(committing_data, ViewNumber::new(10), &current),
            Some(boundary_signed_next_epoch_qc(
                committing_data,
                ViewNumber::new(10),
                &next,
            )),
        );

        let deciding_quorum = if deciding_signed_by_next {
            &next
        } else {
            &current
        };
        let deciding_qc = Certificate::non_epoch_change(boundary_signed_qc(
            QuorumData2 {
                leaf_commit: Committable::commit(leaves[2].leaf()),
                epoch: Some(EpochNumber::new(2)),
                block_number: Some(11),
            },
            ViewNumber::new(11),
            deciding_quorum,
        ));

        let quorum = StakeTableQuorum::new(
            (
                Arc::new(StakeTable::from(current.1)),
                Arc::new(StakeTable::from(next.1)),
            ),
            BOUNDARY_EPOCH_HEIGHT,
        );
        (leaves, committing_qc, deciding_qc, quorum)
    }

    /// A 2-chain proving the last leaf of an epoch includes a deciding QC signed by the next
    /// epoch's quorum; it must be verified against that quorum, not the quorum of the epoch of the
    /// leaf under proof.
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_epoch_boundary_quorum_change() {
        let (leaves, committing_qc, deciding_qc, quorum) = epoch_boundary_fixture(true).await;
        let ChainVersions { leaf: version, .. } = quorum
            .verify_qc_chain_and_get_version(leaves[1].leaf(), [&committing_qc, &deciding_qc])
            .await
            .unwrap();
        assert_eq!(version, leaves[1].header().version());
    }

    /// A deciding QC claiming to be from the next epoch but signed by the current epoch's quorum
    /// must fail verification.
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_epoch_boundary_deciding_qc_wrong_quorum() {
        let (leaves, committing_qc, deciding_qc, quorum) = epoch_boundary_fixture(false).await;
        quorum
            .verify_qc_chain_and_get_version(leaves[1].leaf(), [&committing_qc, &deciding_qc])
            .await
            .unwrap_err();
    }

    /// Build a 2-chain over an interior (non-boundary) leaf of epoch 1 whose deciding QC is forged
    /// by epoch 2's quorum, mimicking a next epoch trying to finalize a leaf the current epoch
    /// never decided. If `disguise_as_boundary`, the committing QC also carries a next-epoch QC to
    /// imitate a genuine epoch-transition committing QC.
    async fn mid_epoch_forgery_fixture(
        disguise_as_boundary: bool,
    ) -> (
        Vec<LeafQueryData<SeqTypes>>,
        Certificate,
        Certificate,
        StakeTableQuorum<(Arc<StakeTable>, Arc<StakeTable>)>,
    ) {
        let current = boundary_quorum(0..5);
        let next = boundary_quorum(5..10);

        // Block 5 is in the interior of epoch 1 (epoch height 10), not an epoch boundary.
        let leaves = leaf_chain(4..=6, EPOCH_VERSION).await;

        let committing_data = QuorumData2 {
            leaf_commit: Committable::commit(leaves[1].leaf()),
            epoch: Some(EpochNumber::new(1)),
            block_number: Some(5),
        };
        let committing_qc = Certificate::new(
            boundary_signed_qc(committing_data, ViewNumber::new(5), &current),
            disguise_as_boundary
                .then(|| boundary_signed_next_epoch_qc(committing_data, ViewNumber::new(5), &next)),
        );

        let deciding_qc = Certificate::non_epoch_change(boundary_signed_qc(
            QuorumData2 {
                leaf_commit: Committable::commit(leaves[2].leaf()),
                epoch: Some(EpochNumber::new(2)),
                block_number: Some(6),
            },
            ViewNumber::new(6),
            &next,
        ));

        let quorum = StakeTableQuorum::new(
            (
                Arc::new(StakeTable::from(current.1)),
                Arc::new(StakeTable::from(next.1)),
            ),
            BOUNDARY_EPOCH_HEIGHT,
        );
        (leaves, committing_qc, deciding_qc, quorum)
    }

    /// The next epoch's quorum must not be able to finalize an interior leaf of the previous epoch:
    /// only the last leaf of an epoch is justified by the next epoch, so a deciding QC signed by
    /// the next epoch over a non-boundary leaf must be rejected.
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_mid_epoch_next_epoch_forgery_rejected() {
        let (leaves, committing_qc, deciding_qc, quorum) = mid_epoch_forgery_fixture(false).await;
        quorum
            .verify_qc_chain_and_get_version(leaves[1].leaf(), [&committing_qc, &deciding_qc])
            .await
            .unwrap_err();
    }

    /// The same forgery must be rejected even when the committing QC is dressed up with a
    /// next-epoch QC to imitate a genuine epoch-transition committing QC.
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_mid_epoch_next_epoch_forgery_with_fake_transition_rejected() {
        let (leaves, committing_qc, deciding_qc, quorum) = mid_epoch_forgery_fixture(true).await;
        quorum
            .verify_qc_chain_and_get_version(leaves[1].leaf(), [&committing_qc, &deciding_qc])
            .await
            .unwrap_err();
    }
>>>>>>> 82b8967ccf8 (fix(light-client): reject HotStuff2 finality proofs across the cutover (#4722))
}
