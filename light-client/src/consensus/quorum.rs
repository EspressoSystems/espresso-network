use std::future::Future;

use anyhow::{bail, ensure, Context, Result};
use committable::Committable;
use derivative::Derivative;
use espresso_types::{EpochVersion, Leaf2, MaxSupportedVersion, SeqTypes, SequencerVersions};
use hotshot_types::{
    epoch_membership::EpochMembership,
    message::UpgradeLock,
    simple_certificate::CertificatePair,
    stake_table::StakeTableEntries,
    vote::{Certificate as _, HasViewNumber},
};
use static_assertions::assert_type_eq_all;
use tracing::Instrument;
use vbs::version::{StaticVersion, StaticVersionType, Version};

pub type Certificate = CertificatePair<SeqTypes>;

pub trait Quorum {
    /// Check a threshold signature on a quorum certificate.
    fn verify(&self, cert: &Certificate, version: Version) -> impl Future<Output = Result<()>> {
        async move {
            match (version.major, version.minor) {
                (0, 1) => self.verify_static::<StaticVersion<0, 1>>(cert).await,
                (0, 2) => self.verify_static::<StaticVersion<0, 2>>(cert).await,
                (0, 3) => self.verify_static::<StaticVersion<0, 3>>(cert).await,
                (0, 4) => self.verify_static::<StaticVersion<0, 4>>(cert).await,
                _ => {
                    // Compile-time check that we aren't missing a case for a supported version.
                    assert_type_eq_all!(MaxSupportedVersion, StaticVersion<0, 4>);
                    bail!("unsupported version {version}");
                },
            }
        }
    }

    /// Same as [`verify`](Self::verify), but with the version as a type-level parameter.
    fn verify_static<V: StaticVersionType + 'static>(
        &self,
        qc: &Certificate,
    ) -> impl Future<Output = Result<()>>;

    /// Verify that QCs are signed, form a chain starting from `leaf`, with a particular protocol
    /// version.
    ///
    /// This check forms the bulk of the commit rule for both HotStuff and HotStuff2.
    fn verify_qc_chain_and_get_version<'a>(
        &self,
        leaf: &Leaf2,
        certs: impl IntoIterator<Item = &'a Certificate>,
    ) -> impl Future<Output = Result<Version>> {
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
            ensure!(version <= MaxSupportedVersion::version());
            if let Some(cert) = &upgrade {
                ensure!(cert.data.new_version <= MaxSupportedVersion::version());
            }
            tracing::debug!(
                %version,
                ?leaf,
                "verify QC chain for leaf"
            );

            // Check the QC chain: valid signatures and sequential views.
            let mut first = None;
            let mut curr: Option<&Certificate> = None;
            for cert in certs {
                tracing::trace!(?cert, "verify cert");

                // What version number do we expect the quorum to have signed over?
                let version = match &upgrade {
                    Some(upgrade) if cert.view_number() >= upgrade.data.new_version_first_view => {
                        tracing::debug!(?upgrade, view = ?cert.view_number(), "using upgraded version");
                        upgrade.data.new_version
                    },
                    _ => version,
                };

                // Check the signature.
                self.verify(cert, version).await?;

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

            Ok(version)
        }
        .instrument(span)
    }
}

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct StakeTableQuorum {
    #[derivative(Debug = "ignore")]
    membership: EpochMembership<SeqTypes>,
    epoch_height: u64,
}

impl Quorum for StakeTableQuorum {
    async fn verify_static<V: StaticVersionType + 'static>(
        &self,
        cert: &Certificate,
    ) -> Result<()> {
        let stake_table = self.membership.stake_table().await;
        let threshold = self.membership.success_threshold().await;
        cert.qc()
            .is_valid_cert::<SequencerVersions<V, V>>(
                &StakeTableEntries::<SeqTypes>::from(stake_table).0,
                threshold,
                &UpgradeLock::new(),
            )
            .await
            .context("invalid QC threshold signature")?;

        if V::version() >= EpochVersion::version() {
            // If this certificate is part of an epoch change, also check that the next epoch's
            // quorum has signed.
            if let Some(next_epoch_qc) = cert.verify_next_epoch_qc(self.epoch_height)? {
                let membership = self.membership.next_epoch_stake_table().await?;
                let stake_table = membership.stake_table().await;
                let threshold = membership.success_threshold().await;
                next_epoch_qc
                    .is_valid_cert::<SequencerVersions<V, V>>(
                        &StakeTableEntries::<SeqTypes>::from(stake_table).0,
                        threshold,
                        &UpgradeLock::new(),
                    )
                    .await
                    .context("invalid next epoch QC threshold signature")?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use espresso_types::EpochVersion;

    use super::*;
    use crate::testing::{
        custom_epoch_change_leaf_chain, custom_leaf_chain_with_upgrade, epoch_change_leaf_chain,
        leaf_chain, leaf_chain_with_upgrade, qc_chain_from_leaf_chain, AlwaysFalseQuorum,
        AlwaysTrueQuorum, EnableEpochs, EpochChangeQuorum, LegacyVersion, VersionCheckQuorum,
    };

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_valid_chain() {
        let leaves = leaf_chain::<EpochVersion>(1..=3).await;
        let version = AlwaysTrueQuorum
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
        let leaves = leaf_chain::<EpochVersion>(1..=3).await;
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
        let leaves = leaf_chain::<EpochVersion>(1..=2).await;
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
        let leaves = leaf_chain::<EpochVersion>(1..=4).await;
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
        let leaves = leaf_chain_with_upgrade::<EnableEpochs>(1..=3, 2).await;
        let version = VersionCheckQuorum::new(leaves.iter().map(|leaf| leaf.leaf().clone()))
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
        let leaves = custom_leaf_chain_with_upgrade::<EnableEpochs>(1..=3, 2, |proposal| {
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
        let leaves = epoch_change_leaf_chain::<EpochVersion>(1..=5, 5).await;
        let version = EpochChangeQuorum::new(5)
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
        let leaves = custom_epoch_change_leaf_chain::<EpochVersion>(1..=5, 5, |proposal| {
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
        let leaves = custom_epoch_change_leaf_chain::<EpochVersion>(1..=5, 5, |proposal| {
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
        let leaves = custom_epoch_change_leaf_chain::<EpochVersion>(1..=5, 5, |proposal| {
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
        let leaves = custom_epoch_change_leaf_chain::<LegacyVersion>(1..=5, 5, |proposal| {
            // Delete the next epoch justify QC; this is allowed since epochs are not enabled yet.
            proposal.next_epoch_justify_qc = None;
        })
        .await;
        let version = EpochChangeQuorum::new(5)
            .verify_qc_chain_and_get_version(
                leaves[0].leaf(),
                &qc_chain_from_leaf_chain(&leaves[1..]),
            )
            .await
            .unwrap();
        assert_eq!(version, leaves[0].header().version());
    }
}
