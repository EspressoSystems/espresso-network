//! Fixtures for tests that need signed leaf chains.

use alloy::primitives::U256;
use bitvec::vec::BitVec;
use committable::Committable;
use hotshot_types::{
    data::{EpochNumber, Leaf2, QuorumProposal2, QuorumProposalWrapper},
    message::UpgradeLock,
    simple_certificate::QuorumCertificate2,
    simple_vote::{QuorumData2, VersionedVoteData, Vote2Data, Voteable},
    stake_table::supermajority_threshold,
    traits::{node_implementation::NodeType, signature_key::SignatureKey},
    utils::epoch_from_block_number,
    vote::Certificate,
};

use crate::{message::Certificate2, utils::CertifiedLeafChain};

type Signer<T> = (
    <T as NodeType>::SignatureKey,
    <<T as NodeType>::SignatureKey as SignatureKey>::PrivateKey,
);

/// `leaves` re-signed by `signers`: every justify QC and a closing cert2, so
/// `CertifiedLeafChain::verify` accepts it.
///
/// `leaves` must be consecutive blocks of one epoch (`epoch_height` blocks per epoch). The signers
/// act as a quorum of unit stakes in the given order, and all of them sign. The oldest leaf keeps
/// its own justify QC, which the verifier never checks.
pub fn sign_leaf_chain<T: NodeType>(
    mut leaves: Vec<Leaf2<T>>,
    epoch_height: u64,
    signers: &[Signer<T>],
    upgrade_lock: &UpgradeLock<T>,
) -> CertifiedLeafChain<T> {
    leaves.sort_by_key(|leaf| leaf.height());

    let mut chain: Vec<Leaf2<T>> = Vec::with_capacity(leaves.len());
    for leaf in leaves {
        let leaf = match chain.last() {
            None => leaf,
            Some(parent) => {
                let data = QuorumData2 {
                    leaf_commit: parent.commit(),
                    epoch: parent.epoch(epoch_height),
                    block_number: Some(parent.height()),
                };
                let qc = sign_certificate::<T, _, QuorumCertificate2<T>>(
                    data,
                    parent.view_number(),
                    signers,
                    upgrade_lock,
                );
                with_justify_qc(&leaf, qc)
            },
        };
        chain.push(leaf);
    }

    let newest = chain.last().expect("empty leaf chain");
    let data = Vote2Data {
        leaf_commit: newest.commit(),
        epoch: EpochNumber::new(epoch_from_block_number(newest.height(), epoch_height)),
        block_number: newest.height(),
    };
    let cert2 = sign_certificate::<T, _, Certificate2<T>>(
        data,
        newest.view_number(),
        signers,
        upgrade_lock,
    );
    CertifiedLeafChain {
        leaves: chain,
        cert2,
    }
}

/// `leaf` with `justify_qc` replaced; the parent commitment follows from the QC.
fn with_justify_qc<T: NodeType>(leaf: &Leaf2<T>, justify_qc: QuorumCertificate2<T>) -> Leaf2<T> {
    Leaf2::from_quorum_proposal(&QuorumProposalWrapper {
        proposal: QuorumProposal2 {
            block_header: leaf.block_header().clone(),
            view_number: leaf.view_number(),
            // Any epoch: only whether the leaf has one is kept.
            epoch: leaf.epoch(1),
            justify_qc,
            next_epoch_justify_qc: leaf.next_epoch_justify_qc().cloned(),
            upgrade_certificate: leaf.upgrade_certificate().cloned(),
            view_change_evidence: leaf.view_change_evidence.clone(),
            next_drb_result: leaf.next_drb_result,
            state_cert: None,
        },
    })
}

/// A certificate over `data` at `view` signed by every signer, in table order.
pub fn sign_certificate<T, D, C>(
    data: D,
    view: hotshot_types::data::ViewNumber,
    signers: &[Signer<T>],
    upgrade_lock: &UpgradeLock<T>,
) -> C
where
    T: NodeType,
    D: Voteable<T> + 'static,
    C: Certificate<T, D, Voteable = D>,
{
    let entries: Vec<_> = signers
        .iter()
        .map(|(key, _)| key.stake_table_entry(U256::ONE))
        .collect();
    let params = T::SignatureKey::public_parameter(
        &entries,
        supermajority_threshold(U256::from(entries.len())),
    );
    let commit = VersionedVoteData::new_infallible(data.clone(), view, upgrade_lock).commit();
    let signatures: Vec<_> = signers
        .iter()
        .map(|(_, private)| T::SignatureKey::sign(private, commit.as_ref()).expect("sign"))
        .collect();
    let bits: BitVec = std::iter::repeat_n(true, signers.len()).collect();
    let assembled = T::SignatureKey::assemble(&params, &bits, &signatures);
    C::create_signed_certificate(commit, data, assembled, view)
}

#[cfg(test)]
mod tests {
    use hotshot_example_types::node_types::TestTypes;
    use hotshot_types::{PeerConfig, signature_key::BLSPubKey, stake_table::HSStakeTable};
    use versions::{EPOCH_VERSION, Upgrade};

    use super::*;
    use crate::utils::CertifiedLeafChain;

    const EPOCH_HEIGHT: u64 = 10;

    fn signers(range: std::ops::Range<u64>) -> Vec<Signer<TestTypes>> {
        range
            .map(|i| BLSPubKey::generated_from_seed_indexed([0; 32], i))
            .collect()
    }

    fn table(signers: &[Signer<TestTypes>]) -> HSStakeTable<TestTypes> {
        signers
            .iter()
            .map(|(key, _)| PeerConfig::<TestTypes> {
                stake_table_entry: key.stake_table_entry(U256::ONE),
                state_ver_key: Default::default(),
                connect_info: None,
            })
            .collect::<Vec<_>>()
            .into()
    }

    /// Consecutive unsigned leaves at `heights`.
    fn leaves(heights: std::ops::Range<u64>) -> Vec<Leaf2<TestTypes>> {
        use hotshot_example_types::block_types::{TestBlockHeader, TestMetadata};
        use hotshot_types::{
            data::VidCommitment, simple_certificate::QuorumCertificate2, utils::BuilderCommitment,
        };

        let mut qc = None::<QuorumCertificate2<TestTypes>>;
        let mut out = Vec::new();
        for height in heights {
            let justify_qc = qc.take().unwrap_or_else(|| {
                let data = QuorumData2 {
                    leaf_commit: committable::Commitment::from_raw([9; 32]),
                    epoch: Some(EpochNumber::new(2)),
                    block_number: Some(height - 1),
                };
                let view = hotshot_types::data::ViewNumber::new(height - 1);
                QuorumCertificate2::new(data, data.commit(), view, None, std::marker::PhantomData)
            });
            let leaf = Leaf2::from_quorum_proposal(&QuorumProposalWrapper {
                proposal: QuorumProposal2 {
                    block_header: TestBlockHeader {
                        block_number: height,
                        payload_commitment: VidCommitment::default(),
                        builder_commitment: BuilderCommitment::from_bytes([]),
                        metadata: TestMetadata {
                            num_transactions: 0,
                        },
                        timestamp: 0,
                        timestamp_millis: 0,
                        random: 0,
                        version: EPOCH_VERSION,
                    },
                    view_number: hotshot_types::data::ViewNumber::new(height),
                    epoch: Some(EpochNumber::new(2)),
                    justify_qc,
                    next_epoch_justify_qc: None,
                    upgrade_certificate: None,
                    view_change_evidence: None,
                    next_drb_result: None,
                    state_cert: None,
                },
            });
            qc = Some(QuorumCertificate2::new(
                QuorumData2 {
                    leaf_commit: leaf.commit(),
                    epoch: Some(EpochNumber::new(2)),
                    block_number: Some(height),
                },
                QuorumData2 {
                    leaf_commit: leaf.commit(),
                    epoch: Some(EpochNumber::new(2)),
                    block_number: Some(height),
                }
                .commit(),
                hotshot_types::data::ViewNumber::new(height),
                None,
                std::marker::PhantomData,
            ));
            out.push(leaf);
        }
        out
    }

    fn lock() -> UpgradeLock<TestTypes> {
        UpgradeLock::new(Upgrade::trivial(EPOCH_VERSION))
    }

    fn chain(signers: &[Signer<TestTypes>]) -> CertifiedLeafChain<TestTypes> {
        sign_leaf_chain(leaves(21..25), EPOCH_HEIGHT, signers, &lock())
    }

    #[test]
    fn certified_chain_verifies_with_given_quorum() {
        let signers = signers(0..4);
        let table = table(&signers);
        let leaf = chain(&signers)
            .verify(21, EPOCH_HEIGHT, &lock(), |epoch| {
                (epoch == EpochNumber::new(3)).then(|| table.clone())
            })
            .unwrap();
        assert_eq!(leaf.height(), 21);
    }

    #[test]
    fn certified_chain_rejects_missing_quorum() {
        let signers = signers(0..4);
        let err = chain(&signers)
            .verify(21, EPOCH_HEIGHT, &lock(), |_| None)
            .unwrap_err();
        assert!(err.to_string().contains("no stake table"), "{err:#}");
    }

    #[test]
    fn certified_chain_rejects_other_quorum() {
        let other = table(&signers(10..14));
        chain(&signers(0..4))
            .verify(21, EPOCH_HEIGHT, &lock(), |_| Some(other.clone()))
            .unwrap_err();
    }
}
