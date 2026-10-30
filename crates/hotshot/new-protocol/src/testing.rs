//! Fixtures for tests that need signed leaf chains.

use std::marker::PhantomData;

use alloy::primitives::U256;
use bitvec::{slice::BitSlice, vec::BitVec};
use committable::Committable;
use hotshot_types::{
    data::{EpochNumber, Leaf2, QuorumProposal2, QuorumProposalWrapper, ViewNumber},
    drb::DrbResult,
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

/// `leaves` linked by parent commitments and closed by a cert2 of `signers`, so
/// `CertifiedLeafChain::verify_ancestors` accepts it.
///
/// `leaves` must be consecutive blocks of one epoch (`epoch_height` blocks per epoch). The signers
/// act as a quorum of unit stakes in the given order, and all of them sign the cert2. Justify QCs
/// are unsigned and the oldest leaf keeps its own, since the verifier checks no QC.
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
                // Unsigned: `verify_ancestors` checks no justify QC, only the parent link.
                let qc = QuorumCertificate2::new(
                    data,
                    data.commit(),
                    parent.view_number(),
                    None,
                    PhantomData,
                );
                rebuild(&leaf, qc, leaf.next_drb_result)
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

/// `leaf` with `justify_qc` and `next_drb_result` replaced; the parent commitment follows from the
/// QC.
fn rebuild<T: NodeType>(
    leaf: &Leaf2<T>,
    justify_qc: QuorumCertificate2<T>,
    next_drb_result: Option<DrbResult>,
) -> Leaf2<T> {
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
            next_drb_result,
            state_cert: None,
        },
    })
}

/// `leaf` carrying `drb`, as the transition leaf of an epoch does.
pub fn with_next_drb_result<T: NodeType>(leaf: &Leaf2<T>, drb: DrbResult) -> Leaf2<T> {
    rebuild(leaf, leaf.justify_qc().clone(), Some(drb))
}

/// A certificate over `data` at `view` signed by every signer, in table order.
pub fn sign_certificate<T, D, C>(
    data: D,
    view: ViewNumber,
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
    let commit = VersionedVoteData::new_infallible(data.clone(), view, upgrade_lock).commit();
    let signatures: Vec<_> = signers
        .iter()
        .map(|(_, private)| T::SignatureKey::sign(private, commit.as_ref()).expect("sign"))
        .collect();
    let bits: BitVec = std::iter::repeat_n(true, signers.len()).collect();
    assemble_certificate(
        data,
        view,
        &entries,
        supermajority_threshold(U256::from(entries.len())),
        &bits,
        &signatures,
        upgrade_lock,
    )
}

/// A certificate over `data` at `view` aggregating `signatures` of the table positions set in
/// `signer_bits`.
pub fn assemble_certificate<T, D, C>(
    data: D,
    view: ViewNumber,
    entries: &[<T::SignatureKey as SignatureKey>::StakeTableEntry],
    threshold: U256,
    signer_bits: &BitSlice,
    signatures: &[<T::SignatureKey as SignatureKey>::PureAssembledSignatureType],
    upgrade_lock: &UpgradeLock<T>,
) -> C
where
    T: NodeType,
    D: Voteable<T> + 'static,
    C: Certificate<T, D, Voteable = D>,
{
    let params = T::SignatureKey::public_parameter(entries, threshold);
    let assembled = T::SignatureKey::assemble(&params, signer_bits, signatures);
    let commit = VersionedVoteData::new_infallible(data.clone(), view, upgrade_lock).commit();
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
        leaves_at_views(heights, |height| height)
    }

    /// Like `leaves`, with the view of each leaf given by `view`.
    fn leaves_at_views(
        heights: std::ops::Range<u64>,
        view: impl Fn(u64) -> u64,
    ) -> Vec<Leaf2<TestTypes>> {
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
                    view_number: hotshot_types::data::ViewNumber::new(view(height)),
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
    fn verify_ancestors_ok() {
        let signers = signers(0..4);
        let table = table(&signers);
        let leaves = chain(&signers)
            .verify_ancestors(22, EPOCH_HEIGHT, &lock(), &table)
            .unwrap();
        let heights: Vec<_> = leaves.iter().map(|leaf| leaf.height()).collect();
        assert_eq!(heights, vec![22, 23, 24]);
    }

    #[test]
    fn verify_ancestors_broken_link_fails() {
        let signers = signers(0..4);
        let mut chain = chain(&signers);
        chain.leaves.retain(|leaf| leaf.height() != 23);
        chain
            .verify_ancestors(21, EPOCH_HEIGHT, &lock(), &table(&signers))
            .unwrap_err();
    }

    #[test]
    fn verify_ancestors_wrong_quorum_fails() {
        let other = table(&signers(10..14));
        chain(&signers(0..4))
            .verify_ancestors(21, EPOCH_HEIGHT, &lock(), &other)
            .unwrap_err();
    }

    #[test]
    fn verify_ancestors_genesis_cert2_fails() {
        let signers = signers(0..4);
        let leaves = leaves_at_views(1..2, |_| 0);
        let chain = sign_leaf_chain(leaves, EPOCH_HEIGHT, &signers, &lock());
        let err = chain
            .verify_ancestors(1, EPOCH_HEIGHT, &lock(), &table(&signers))
            .unwrap_err();
        assert!(err.to_string().contains("genesis view"), "{err:#}");
    }

    #[test]
    fn verify_ancestors_ignores_junk_high_view_leaf() {
        let signers = signers(0..4);
        let mut chain = chain(&signers);
        chain.leaves.extend(leaves(99..100));
        let leaves = chain
            .verify_ancestors(22, EPOCH_HEIGHT, &lock(), &table(&signers))
            .unwrap();
        let heights: Vec<_> = leaves.iter().map(|leaf| leaf.height()).collect();
        assert_eq!(heights, vec![22, 23, 24]);
    }
}
