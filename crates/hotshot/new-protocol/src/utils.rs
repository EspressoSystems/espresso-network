use anyhow::{anyhow, ensure};
use committable::Committable;
use hotshot_types::{
    data::{EpochNumber, Leaf2, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    stake_table::StakeTableEntries,
    traits::node_implementation::NodeType,
    utils::epoch_from_block_number,
    vote::{Certificate, HasViewNumber},
};

use crate::message::Certificate2;

/// Verify that a leaf is finalized by a new-protocol Certificate2.
///
/// `cert2` directly commits the newest leaf in `leaf_chain`. By the indirect
/// commit rule, every ancestor of that leaf is finalized as well. This verifier
/// validates `cert2`, then walks backward through the certified leaf's parent
/// links until it finds `expected_height`.
///
/// View numbers are allowed to skip after timeouts, so the input may contain
/// leaves that are not on the certified ancestry path. Those leaves are ignored;
/// every accepted step must match the current leaf's justify QC, parent
/// commitment, and block height.
pub async fn verify_new_protocol_leaf_chain<T: NodeType>(
    mut leaf_chain: Vec<Leaf2<T>>,
    coordinator: &EpochMembershipCoordinator<T>,
    expected_height: u64,
    upgrade_lock: &UpgradeLock<T>,
    cert2: Certificate2<T>,
) -> anyhow::Result<Leaf2<T>> {
    leaf_chain.sort_by_key(|l| l.view_number());
    leaf_chain.reverse();

    ensure!(!leaf_chain.is_empty(), "empty leaf chain");
    let newest = &leaf_chain[0];

    ensure!(
        cert2.view_number() > ViewNumber::genesis(),
        "cert2 must not be the genesis view"
    );
    let epoch = EpochNumber::new(epoch_from_block_number(
        cert2.data.block_number,
        *coordinator.epoch_height(),
    ));
    ensure!(
        cert2.data.epoch == epoch,
        "cert2 epoch {} does not match epoch {epoch} derived from its block number {}",
        cert2.data.epoch,
        cert2.data.block_number
    );

    let membership = coordinator
        .stake_table_for_epoch(Some(epoch))
        .map_err(|err| anyhow!("no stake table available for epoch {epoch}: {err:?}"))?;
    let entries = StakeTableEntries::<T>::from_iter(membership.stake_table()).0;
    cert2.is_valid_cert(&entries, membership.success_threshold(), upgrade_lock)?;

    ensure!(
        cert2.data.leaf_commit == newest.commit(),
        "cert2 does not match the newest leaf in the chain"
    );
    ensure!(
        cert2.data.block_number == newest.height(),
        "cert2 block number does not match the newest leaf"
    );
    ensure!(
        cert2.view_number() == newest.view_number(),
        "cert2 view does not match the newest leaf"
    );

    if newest.height() == expected_height {
        return Ok(newest.clone());
    }

    let mut current = newest;
    for leaf in leaf_chain[1..].iter() {
        let justify_qc = current.justify_qc();
        if justify_qc.view_number() != leaf.view_number()
            || justify_qc.data().leaf_commit != leaf.commit()
        {
            tracing::warn!(
                view = ?leaf.view_number(),
                expected_view = ?justify_qc.view_number(),
                "leaf is off the leafchain path; expected only after a view timeout"
            );
            continue;
        }
        ensure!(
            current.parent_commitment() == leaf.commit(),
            "current leaf parent commitment does not match parent leaf"
        );
        ensure!(
            leaf.height().checked_add(1) == Some(current.height()),
            "leaf heights do not chain"
        );
        let qc_epoch = EpochNumber::new(epoch_from_block_number(
            leaf.height(),
            *coordinator.epoch_height(),
        ));
        ensure!(
            justify_qc.data().epoch == Some(qc_epoch),
            "justify QC claims epoch {:?} but certifies the leaf at height {} in epoch {qc_epoch}",
            justify_qc.data().epoch,
            leaf.height()
        );
        if let Some(block_number) = justify_qc.data().block_number {
            ensure!(
                block_number == leaf.height(),
                "justify QC claims block number {block_number} but certifies the leaf at height {}",
                leaf.height()
            );
        }
        let membership = coordinator
            .stake_table_for_epoch(Some(qc_epoch))
            .map_err(|err| anyhow!("no stake table available for epoch {qc_epoch}: {err:?}"))?;
        let entries = StakeTableEntries::<T>::from_iter(membership.stake_table()).0;
        justify_qc.is_valid_cert(&entries, membership.success_threshold(), upgrade_lock)?;
        if leaf.height() == expected_height {
            return Ok(leaf.clone());
        }
        current = leaf;
    }

    Err(anyhow!(
        "expected height was not found in the cert2-finalized chain"
    ))
}

#[cfg(test)]
mod test {
    use alloy::primitives::U256;
    use bitvec::vec::BitVec;
    use committable::Commitment;
    use hotshot::types::{BLSPubKey, SchnorrPubKey};
    use hotshot_example_types::{
        block_types::{TestBlockHeader, TestMetadata},
        membership::{
            TestableMembership, static_committee::StaticStakeTable,
            strict_membership::StrictMembership,
        },
        node_types::TestTypes,
        storage_types::TestStorage,
    };
    use hotshot_types::{
        PeerConfig,
        data::{QuorumProposal2, QuorumProposalWrapper, VidCommitment},
        simple_certificate::QuorumCertificate2,
        simple_vote::{QuorumData2, VersionedVoteData, Vote2Data},
        stake_table::{StakeTableEntry, supermajority_threshold},
        traits::{election::Membership, signature_key::SignatureKey},
        utils::{BuilderCommitment, verify_leaf_chain},
    };
    use versions::{EPOCH_VERSION, NEW_PROTOCOL_VERSION, Upgrade};

    use super::*;

    const EPOCH_HEIGHT: u64 = 10;

    type PrivKey = <BLSPubKey as SignatureKey>::PrivateKey;
    type Quorum = (Vec<PrivKey>, Vec<PeerConfig<TestTypes>>);

    fn quorum(indexes: std::ops::Range<u64>) -> Quorum {
        indexes
            .map(|i| {
                let (stake_key, priv_key) =
                    BLSPubKey::generated_from_seed_indexed(Default::default(), i);
                (
                    priv_key,
                    PeerConfig::<TestTypes> {
                        stake_table_entry: StakeTableEntry {
                            stake_key,
                            stake_amount: U256::from(1),
                        },
                        state_ver_key: Default::default(),
                        connect_info: None,
                    },
                )
            })
            .unzip()
    }

    /// A coordinator over a membership whose quorum is `quorum1` in epoch 1
    /// and `quorum2` in epoch 2; no later epoch has a stake table.
    fn coordinator(
        (_, peers1): &Quorum,
        (_, peers2): &Quorum,
    ) -> EpochMembershipCoordinator<TestTypes> {
        let membership =
            StrictMembership::<TestTypes, StaticStakeTable<BLSPubKey, SchnorrPubKey>>::new(
                peers1.clone(),
                peers1.clone(),
                peers1[0].stake_table_entry.stake_key,
                EPOCH_HEIGHT,
            );
        membership.add_quorum_committee(EpochNumber::new(2), peers2.clone());
        membership.set_first_epoch(EpochNumber::new(1), [0u8; 32]);
        EpochMembershipCoordinator::new(
            membership,
            EPOCH_HEIGHT,
            &TestStorage::<TestTypes>::default(),
        )
    }

    fn sign((keys, peers): &Quorum, msg: &[u8]) -> <BLSPubKey as SignatureKey>::QcType {
        let entries: Vec<_> = peers.iter().map(|p| p.stake_table_entry.clone()).collect();
        let total: U256 = entries.iter().map(|e| e.stake_amount).sum();
        let pp = BLSPubKey::public_parameter(&entries, supermajority_threshold(total));
        let sigs: Vec<_> = keys
            .iter()
            .map(|key| BLSPubKey::sign(key, msg).unwrap())
            .collect();
        BLSPubKey::assemble(
            &pp,
            &std::iter::repeat_n(true, keys.len()).collect::<BitVec>(),
            &sigs,
        )
    }

    fn signed_qc(
        data: QuorumData2<TestTypes>,
        view: u64,
        quorum: &Quorum,
        upgrade_lock: &UpgradeLock<TestTypes>,
    ) -> QuorumCertificate2<TestTypes> {
        let view = ViewNumber::new(view);
        let commit = VersionedVoteData::new_infallible(data, view, upgrade_lock).commit();
        QuorumCertificate2::create_signed_certificate(
            commit,
            data,
            sign(quorum, commit.as_ref()),
            view,
        )
    }

    fn signed_cert2(
        data: Vote2Data<TestTypes>,
        view: u64,
        quorum: &Quorum,
        upgrade_lock: &UpgradeLock<TestTypes>,
    ) -> Certificate2<TestTypes> {
        let view = ViewNumber::new(view);
        let commit = VersionedVoteData::new_infallible(data.clone(), view, upgrade_lock).commit();
        Certificate2::create_signed_certificate(commit, data, sign(quorum, commit.as_ref()), view)
    }

    fn make_leaf(
        height: u64,
        view: u64,
        epoch: u64,
        justify_qc: QuorumCertificate2<TestTypes>,
        version: vbs::version::Version,
    ) -> Leaf2<TestTypes> {
        Leaf2::from_quorum_proposal(&QuorumProposalWrapper {
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
                    version,
                },
                view_number: ViewNumber::new(view),
                epoch: Some(EpochNumber::new(epoch)),
                justify_qc,
                next_epoch_justify_qc: None,
                upgrade_certificate: None,
                view_change_evidence: None,
                next_drb_result: None,
                state_cert: None,
            },
        })
    }

    /// A legacy 3-chain deciding the last block of `epoch` (epoch height 10), where consecutive
    /// epochs have disjoint quorums. The QC on the first block of `epoch + 1` is signed by
    /// `deciding_quorum`; every other QC is signed by `epoch_quorum`.
    fn boundary_3_chain(
        epoch: u64,
        epoch_quorum: &Quorum,
        deciding_quorum: &Quorum,
        upgrade_lock: &UpgradeLock<TestTypes>,
    ) -> Vec<Leaf2<TestTypes>> {
        let last = epoch * EPOCH_HEIGHT;
        let parent_qc = signed_qc(
            QuorumData2 {
                leaf_commit: Commitment::from_raw([9; 32]),
                epoch: Some(EpochNumber::new(epoch)),
                block_number: Some(last - 1),
            },
            last - 1,
            epoch_quorum,
            upgrade_lock,
        );
        let last_leaf = make_leaf(last, last, epoch, parent_qc, EPOCH_VERSION);
        let last_qc = signed_qc(
            QuorumData2 {
                leaf_commit: Committable::commit(&last_leaf),
                epoch: Some(EpochNumber::new(epoch)),
                block_number: Some(last),
            },
            last,
            epoch_quorum,
            upgrade_lock,
        );
        let boundary_leaf = make_leaf(last + 1, last + 1, epoch + 1, last_qc, EPOCH_VERSION);
        let boundary_qc = signed_qc(
            QuorumData2 {
                leaf_commit: Committable::commit(&boundary_leaf),
                epoch: Some(EpochNumber::new(epoch + 1)),
                block_number: Some(last + 1),
            },
            last + 1,
            deciding_quorum,
            upgrade_lock,
        );
        let deciding_leaf = make_leaf(last + 2, last + 2, epoch + 1, boundary_qc, EPOCH_VERSION);
        vec![last_leaf, boundary_leaf, deciding_leaf]
    }

    /// A self-consistent 3-chain starting at `first_height`, entirely within `epoch`, with
    /// every QC signed by `epoch_quorum`.
    fn same_epoch_3_chain(
        first_height: u64,
        epoch: u64,
        epoch_quorum: &Quorum,
        upgrade_lock: &UpgradeLock<TestTypes>,
    ) -> Vec<Leaf2<TestTypes>> {
        let parent_qc = signed_qc(
            QuorumData2 {
                leaf_commit: Commitment::from_raw([9; 32]),
                epoch: Some(EpochNumber::new(epoch)),
                block_number: Some(first_height - 1),
            },
            first_height - 1,
            epoch_quorum,
            upgrade_lock,
        );
        let mut chain = vec![make_leaf(
            first_height,
            first_height,
            epoch,
            parent_qc,
            EPOCH_VERSION,
        )];
        for height in first_height + 1..first_height + 3 {
            let qc = signed_qc(
                QuorumData2 {
                    leaf_commit: Committable::commit(chain.last().unwrap()),
                    epoch: Some(EpochNumber::new(epoch)),
                    block_number: Some(height - 1),
                },
                height - 1,
                epoch_quorum,
                upgrade_lock,
            );
            chain.push(make_leaf(height, height, epoch, qc, EPOCH_VERSION));
        }
        chain
    }

    /// A 3-chain deciding the last leaf of an epoch contains a QC signed by the next epoch's
    /// quorum; it must be verified against that quorum's stake table.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_verify_leaf_chain_across_epoch_boundary() {
        let quorum1 = quorum(0..5);
        let quorum2 = quorum(5..10);
        let upgrade_lock = UpgradeLock::<TestTypes>::new(Upgrade::trivial(EPOCH_VERSION));

        let chain = boundary_3_chain(1, &quorum1, &quorum2, &upgrade_lock);
        let expected = Committable::commit(&chain[0]);

        let coordinator = coordinator(&quorum1, &quorum2);
        let leaf = verify_leaf_chain(chain, &coordinator, 10, &upgrade_lock)
            .await
            .unwrap();
        assert_eq!(Committable::commit(&leaf), expected);
    }

    /// A QC claiming epoch 2 in its signed payload but signed by epoch 1's quorum must fail
    /// verification against epoch 2's stake table.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_verify_leaf_chain_boundary_qc_wrong_quorum() {
        let quorum1 = quorum(0..5);
        let quorum2 = quorum(5..10);
        let upgrade_lock = UpgradeLock::<TestTypes>::new(Upgrade::trivial(EPOCH_VERSION));

        let chain = boundary_3_chain(1, &quorum1, &quorum1, &upgrade_lock);

        let coordinator = coordinator(&quorum1, &quorum2);
        verify_leaf_chain(chain, &coordinator, 10, &upgrade_lock)
            .await
            .unwrap_err();
    }

    /// A fully self-consistent chain deciding a height in epoch 2 whose QCs claim — and are
    /// genuinely signed by — epoch 1's quorum must be rejected: the signed epoch only binds a
    /// QC to the claimed epoch's quorum, so accepting any claimed epoch would let a stale
    /// quorum forge chains for later heights.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_verify_leaf_chain_rejects_stale_epoch_quorum() {
        let quorum1 = quorum(0..5);
        let quorum2 = quorum(5..10);
        let upgrade_lock = UpgradeLock::<TestTypes>::new(Upgrade::trivial(EPOCH_VERSION));

        let chain = same_epoch_3_chain(20, 1, &quorum1, &upgrade_lock);

        let coordinator = coordinator(&quorum1, &quorum2);
        let err = verify_leaf_chain(chain, &coordinator, 20, &upgrade_lock)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("claims epoch"), "{err:#}");
    }

    /// A boundary chain whose deciding QC belongs to an epoch with no available stake table
    /// must fail verification instead of falling back to another epoch's table.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_verify_leaf_chain_missing_next_epoch_stake_table() {
        let quorum1 = quorum(0..5);
        let quorum2 = quorum(5..10);
        let upgrade_lock = UpgradeLock::<TestTypes>::new(Upgrade::trivial(EPOCH_VERSION));

        let chain = boundary_3_chain(2, &quorum2, &quorum2, &upgrade_lock);

        let coordinator = coordinator(&quorum1, &quorum2);
        let err = verify_leaf_chain(chain, &coordinator, 20, &upgrade_lock)
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("no stake table available"),
            "{err:#}"
        );
    }

    /// A new-protocol leaf range deciding the last block of `epoch`, finalized by a cert2 on
    /// the first block of `epoch + 1`, signed by `cert2_quorum`.
    fn boundary_cert2_chain(
        epoch: u64,
        epoch_quorum: &Quorum,
        cert2_quorum: &Quorum,
        upgrade_lock: &UpgradeLock<TestTypes>,
    ) -> (Vec<Leaf2<TestTypes>>, Certificate2<TestTypes>) {
        let last = epoch * EPOCH_HEIGHT;
        let parent_qc = signed_qc(
            QuorumData2 {
                leaf_commit: Commitment::from_raw([9; 32]),
                epoch: Some(EpochNumber::new(epoch)),
                block_number: Some(last - 1),
            },
            last - 1,
            epoch_quorum,
            upgrade_lock,
        );
        let last_leaf = make_leaf(last, last, epoch, parent_qc, NEW_PROTOCOL_VERSION);
        let last_qc = signed_qc(
            QuorumData2 {
                leaf_commit: Committable::commit(&last_leaf),
                epoch: Some(EpochNumber::new(epoch)),
                block_number: Some(last),
            },
            last,
            epoch_quorum,
            upgrade_lock,
        );
        let boundary_leaf = make_leaf(last + 1, last + 1, epoch + 1, last_qc, NEW_PROTOCOL_VERSION);
        let cert2 = signed_cert2(
            Vote2Data {
                leaf_commit: Committable::commit(&boundary_leaf),
                epoch: EpochNumber::new(epoch + 1),
                block_number: last + 1,
            },
            last + 1,
            cert2_quorum,
            upgrade_lock,
        );
        (vec![last_leaf, boundary_leaf], cert2)
    }

    /// A cert2 finalizing the last leaf of an epoch is produced in the next epoch and must be
    /// verified against the next epoch's stake table, while the boundary QC inside the chain is
    /// verified against the previous epoch's.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_verify_new_protocol_leaf_chain_across_epoch_boundary() {
        let quorum1 = quorum(0..5);
        let quorum2 = quorum(5..10);
        let upgrade_lock = UpgradeLock::<TestTypes>::new(Upgrade::trivial(NEW_PROTOCOL_VERSION));

        let (chain, cert2) = boundary_cert2_chain(1, &quorum1, &quorum2, &upgrade_lock);
        let expected = Committable::commit(&chain[0]);

        let coordinator = coordinator(&quorum1, &quorum2);
        let leaf = verify_new_protocol_leaf_chain(chain, &coordinator, 10, &upgrade_lock, cert2)
            .await
            .unwrap();
        assert_eq!(Committable::commit(&leaf), expected);
    }

    /// A cert2 claiming epoch 2 in its signed payload but signed by epoch 1's quorum must fail
    /// verification against epoch 2's stake table.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_verify_new_protocol_leaf_chain_wrong_quorum() {
        let quorum1 = quorum(0..5);
        let quorum2 = quorum(5..10);
        let upgrade_lock = UpgradeLock::<TestTypes>::new(Upgrade::trivial(NEW_PROTOCOL_VERSION));

        let (chain, cert2) = boundary_cert2_chain(1, &quorum1, &quorum1, &upgrade_lock);

        let coordinator = coordinator(&quorum1, &quorum2);
        verify_new_protocol_leaf_chain(chain, &coordinator, 10, &upgrade_lock, cert2)
            .await
            .unwrap_err();
    }

    /// A cert2 belonging to an epoch with no available stake table must fail verification
    /// instead of falling back to another epoch's table.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_verify_new_protocol_leaf_chain_missing_next_epoch_stake_table() {
        let quorum1 = quorum(0..5);
        let quorum2 = quorum(5..10);
        let upgrade_lock = UpgradeLock::<TestTypes>::new(Upgrade::trivial(NEW_PROTOCOL_VERSION));

        let (chain, cert2) = boundary_cert2_chain(2, &quorum2, &quorum2, &upgrade_lock);

        let coordinator = coordinator(&quorum1, &quorum2);
        let err = verify_new_protocol_leaf_chain(chain, &coordinator, 20, &upgrade_lock, cert2)
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("no stake table available"),
            "{err:#}"
        );
    }
}
