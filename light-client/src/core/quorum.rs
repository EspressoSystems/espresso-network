use std::future::Future;

use anyhow::{bail, ensure, Context, Result};
use committable::Committable;
use espresso_types::{Leaf2, MaxSupportedVersion, SeqTypes, SequencerVersions};
use hotshot_types::{
    epoch_membership::EpochMembership, message::UpgradeLock,
    simple_certificate::QuorumCertificate2, stake_table::StakeTableEntries, vote::Certificate,
};
use static_assertions::assert_type_eq_all;
use vbs::version::{StaticVersion, StaticVersionType, Version};

pub trait Quorum {
    /// Check a threshold signature on a quorum certificate.
    fn verify(
        &self,
        qc: &QuorumCertificate2<SeqTypes>,
        version: Version,
    ) -> impl Future<Output = Result<()>> {
        async move {
            match (version.major, version.minor) {
                (0, 1) => self.verify_static::<StaticVersion<0, 1>>(qc).await,
                (0, 2) => self.verify_static::<StaticVersion<0, 2>>(qc).await,
                (0, 3) => self.verify_static::<StaticVersion<0, 3>>(qc).await,
                (0, 4) => self.verify_static::<StaticVersion<0, 4>>(qc).await,
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
        qc: &QuorumCertificate2<SeqTypes>,
    ) -> impl Future<Output = Result<()>>;

    /// Verify that QCs are signed, form a chain starting from `leaf`, with a particular protocol
    /// version.
    ///
    /// This check forms the bulk of the commit rule for both HotStuff and HotStuff2.
    fn verify_qc_chain_and_get_version<'a>(
        &self,
        leaf: &Leaf2,
        qcs: impl IntoIterator<Item = &'a QuorumCertificate2<SeqTypes>>,
    ) -> impl Future<Output = Result<Version>> {
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

            // Check the QC chain: valid signatures and sequential views.
            let mut first = None;
            let mut curr: Option<&QuorumCertificate2<SeqTypes>> = None;
            for qc in qcs {
                // What version number do we expect the quorum to have signed over?
                let version = match &upgrade {
                    Some(cert) if qc.view_number >= cert.data.new_version_first_view => {
                        cert.data.new_version
                    },
                    _ => version,
                };

                // Check the signature.
                self.verify(qc, version).await?;

                // Check chaining.
                if let Some(prev_qc) = curr {
                    ensure!(qc.view_number == prev_qc.view_number + 1);
                }
                curr = Some(qc);

                // Save the first QC.
                if first.is_none() {
                    first = Some(qc);
                }
            }

            // Check that the first QC in the chain signs the required leaf.
            let first_qc = first.context("empty QC chain")?;
            ensure!(first_qc.data.leaf_commit == leaf.commit());

            Ok(version)
        }
    }
}

impl Quorum for EpochMembership<SeqTypes> {
    async fn verify_static<V: StaticVersionType + 'static>(
        &self,
        qc: &QuorumCertificate2<SeqTypes>,
    ) -> Result<()> {
        let stake_table = self.stake_table().await;
        let threshold = self.success_threshold().await;
        qc.is_valid_cert::<SequencerVersions<V, V>>(
            &StakeTableEntries::<SeqTypes>::from(stake_table).0,
            threshold,
            &UpgradeLock::new(),
        )
        .await
        .context("invalid QC threshold signature")
    }
}

#[cfg(test)]
mod test {
    use espresso_types::EpochVersion;

    use super::*;
    use crate::testing::{
        custom_leaf_chain_with_upgrade, leaf_chain, leaf_chain_with_upgrade, AlwaysFalseQuorum,
        AlwaysTrueQuorum, EnableEpochs, VersionCheckQuorum,
    };

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_valid_chain() {
        let leaves = leaf_chain::<EpochVersion>(1..=2).await;
        let version = AlwaysTrueQuorum
            .verify_qc_chain_and_get_version(leaves[0].leaf(), [leaves[0].qc(), leaves[1].qc()])
            .await
            .unwrap();
        assert_eq!(version, leaves[0].header().version());
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_invalid_qc() {
        let leaves = leaf_chain::<EpochVersion>(1..=1).await;
        AlwaysFalseQuorum
            .verify_qc_chain_and_get_version(leaves[0].leaf(), [leaves[0].qc()])
            .await
            .unwrap_err();
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_non_consecutive() {
        let leaves = leaf_chain::<EpochVersion>(1..=3).await;
        AlwaysTrueQuorum
            .verify_qc_chain_and_get_version(leaves[0].leaf(), [leaves[0].qc(), leaves[2].qc()])
            .await
            .unwrap_err();
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_upgrade() {
        let leaves = leaf_chain_with_upgrade::<EnableEpochs>(1..=2, 2).await;
        let version = VersionCheckQuorum::new(leaves.iter().map(|leaf| leaf.leaf().clone()))
            .verify_qc_chain_and_get_version(leaves[0].leaf(), [leaves[0].qc(), leaves[1].qc()])
            .await
            .unwrap();
        assert_eq!(version, leaves[0].header().version());
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_illegal_upgrade() {
        let leaves = custom_leaf_chain_with_upgrade::<EnableEpochs>(1..=2, 2, |proposal| {
            // Don't attach an upgrade certificate, so that the version change that happens within
            // the QC change is actually malicious.
            proposal.upgrade_certificate = None;
        })
        .await;
        VersionCheckQuorum::new(leaves.iter().map(|leaf| leaf.leaf().clone()))
            .verify_qc_chain_and_get_version(leaves[0].leaf(), [leaves[0].qc(), leaves[1].qc()])
            .await
            .unwrap_err();
    }
}
