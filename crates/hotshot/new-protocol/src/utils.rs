use anyhow::{anyhow, ensure};
use committable::Committable;
use hotshot_types::{
    data::{EpochNumber, Leaf2},
    message::UpgradeLock,
    simple_certificate::QuorumCertificate2,
    stake_table::{EpochStakeTables, StakeTableEntries},
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
pub async fn verify_leaf_chain_with_cert2<T: NodeType>(
    mut leaf_chain: Vec<Leaf2<T>>,
    stake_tables: &EpochStakeTables<T>,
    expected_height: u64,
    upgrade_lock: &UpgradeLock<T>,
    cert2: Certificate2<T>,
) -> anyhow::Result<Leaf2<T>> {
    leaf_chain.sort_by_key(|l| l.view_number());
    leaf_chain.reverse();

    ensure!(!leaf_chain.is_empty(), "empty leaf chain");

    // The chain may cross an epoch boundary, in which case its certificates
    // are signed by different epochs' quorums. Derive each certificate's epoch
    // from the height of the leaf it certifies; trusting the claims in the
    // certificate itself would let a quorum of a different epoch pick the
    // stake table that verifies its own signatures.
    let check_qc = |qc: &QuorumCertificate2<T>, certified_height: u64| -> anyhow::Result<()> {
        let epoch = EpochNumber::new(epoch_from_block_number(
            certified_height,
            stake_tables.epoch_height,
        ));
        ensure!(
            qc.data.epoch == Some(epoch),
            "QC claims epoch {:?} but certifies the leaf at height {certified_height} in epoch \
             {epoch}",
            qc.data.epoch,
        );
        if let Some(block_number) = qc.data.block_number {
            ensure!(
                block_number == certified_height,
                "QC claims block number {block_number} but certifies the leaf at height \
                 {certified_height}"
            );
        }
        let table = stake_tables.for_epoch(Some(epoch))?;
        qc.is_valid_cert(
            &StakeTableEntries::<T>::from(table.stake_table.clone()).0,
            table.success_threshold,
            upgrade_lock,
        )?;
        Ok(())
    };

    // Bind cert2 to the newest leaf first, then verify it against the epoch
    // derived from that leaf's height.
    ensure!(
        cert2.data.leaf_commit == leaf_chain[0].commit(),
        "cert2 does not match the newest leaf in the chain"
    );
    ensure!(
        cert2.data.block_number == leaf_chain[0].height(),
        "cert2 block number does not match the newest leaf"
    );
    ensure!(
        cert2.view_number() == leaf_chain[0].view_number(),
        "cert2 view does not match the newest leaf"
    );
    let cert2_epoch = EpochNumber::new(epoch_from_block_number(
        leaf_chain[0].height(),
        stake_tables.epoch_height,
    ));
    ensure!(
        cert2.data.epoch == cert2_epoch,
        "cert2 claims epoch {} but certifies the leaf at height {} in epoch {cert2_epoch}",
        cert2.data.epoch,
        leaf_chain[0].height()
    );
    let cert2_table = stake_tables.for_epoch(Some(cert2_epoch))?;
    cert2.is_valid_cert(
        &StakeTableEntries::<T>::from(cert2_table.stake_table.clone()).0,
        cert2_table.success_threshold,
        upgrade_lock,
    )?;

    if leaf_chain[0].height() == expected_height {
        return Ok(leaf_chain[0].clone());
    }

    let mut current = &leaf_chain[0];
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
        check_qc(&justify_qc, leaf.height())?;
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
    use hotshot_example_types::{
        block_types::{TestBlockHeader, TestMetadata},
        node_types::TestTypes,
    };
    use hotshot_types::{
        PeerConfig,
        data::{EpochNumber, QuorumProposal2, QuorumProposalWrapper, VidCommitment, ViewNumber},
        signature_key::BLSPubKey,
        simple_vote::{QuorumData2, VersionedVoteData, Vote2Data},
        stake_table::{EpochStakeTable, StakeTableEntry, supermajority_threshold},
        traits::signature_key::SignatureKey,
        utils::{BuilderCommitment, verify_leaf_chain},
    };
    use versions::{EPOCH_VERSION, NEW_PROTOCOL_VERSION, Upgrade};

    use super::*;

    type PrivKey = <BLSPubKey as SignatureKey>::PrivateKey;
    type Quorum = (Vec<PrivKey>, Vec<PeerConfig<TestTypes>>);

    /// Epoch height used by every fixture: blocks 1..=10 are epoch 1, 11..=20 epoch 2.
    const EPOCH_HEIGHT: u64 = 10;

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

    fn stake_table_for(epoch: u64, (_, peers): &Quorum) -> EpochStakeTable<TestTypes> {
        let total: U256 = peers.iter().map(|p| p.stake_table_entry.stake_amount).sum();
        EpochStakeTable {
            epoch: Some(EpochNumber::new(epoch)),
            stake_table: peers.clone().into(),
            success_threshold: supermajority_threshold(total),
        }
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

    /// A legacy 3-chain deciding the last block of epoch 1 (epoch height 10), where epochs 1 and
    /// 2 have disjoint quorums. The QC on the first block of epoch 2 is signed by epoch 2's
    /// quorum; every other QC is signed by epoch 1's.
    fn boundary_3_chain(
        quorum1: &Quorum,
        deciding_quorum: &Quorum,
        upgrade_lock: &UpgradeLock<TestTypes>,
    ) -> Vec<Leaf2<TestTypes>> {
        let qc9 = signed_qc(
            QuorumData2 {
                leaf_commit: Commitment::from_raw([9; 32]),
                epoch: Some(EpochNumber::new(1)),
                block_number: Some(9),
            },
            9,
            quorum1,
            upgrade_lock,
        );
        let leaf10 = make_leaf(10, 10, 1, qc9, EPOCH_VERSION);
        let qc10 = signed_qc(
            QuorumData2 {
                leaf_commit: Committable::commit(&leaf10),
                epoch: Some(EpochNumber::new(1)),
                block_number: Some(10),
            },
            10,
            quorum1,
            upgrade_lock,
        );
        let leaf11 = make_leaf(11, 11, 2, qc10, EPOCH_VERSION);
        let qc11 = signed_qc(
            QuorumData2 {
                leaf_commit: Committable::commit(&leaf11),
                epoch: Some(EpochNumber::new(2)),
                block_number: Some(11),
            },
            11,
            deciding_quorum,
            upgrade_lock,
        );
        let leaf12 = make_leaf(12, 12, 2, qc11, EPOCH_VERSION);
        vec![leaf10, leaf11, leaf12]
    }

    /// A 3-chain deciding the last leaf of an epoch contains a QC signed by the next epoch's
    /// quorum; it must be verified against that quorum's stake table.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_verify_leaf_chain_across_epoch_boundary() {
        let quorum1 = quorum(0..5);
        let quorum2 = quorum(5..10);
        let upgrade_lock = UpgradeLock::<TestTypes>::new(Upgrade::trivial(EPOCH_VERSION));

        let chain = boundary_3_chain(&quorum1, &quorum2, &upgrade_lock);
        let expected = Committable::commit(&chain[0]);

        let stake_tables = EpochStakeTables {
            tables: vec![stake_table_for(1, &quorum1), stake_table_for(2, &quorum2)],
            epoch_height: EPOCH_HEIGHT,
        };
        let leaf = verify_leaf_chain(chain.clone(), &stake_tables, 10, &upgrade_lock)
            .await
            .unwrap();
        assert_eq!(Committable::commit(&leaf), expected);

        // With only the leaf's own epoch provided (the pre-fix behavior), verification must fail
        // instead of checking the next epoch's QC against the wrong stake table.
        let only_epoch1 = EpochStakeTables {
            tables: vec![stake_table_for(1, &quorum1)],
            epoch_height: EPOCH_HEIGHT,
        };
        verify_leaf_chain(chain, &only_epoch1, 10, &upgrade_lock)
            .await
            .unwrap_err();
    }

    /// A QC claiming epoch 2 in its signed payload but signed by epoch 1's quorum must fail
    /// verification against epoch 2's stake table.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_verify_leaf_chain_boundary_qc_wrong_quorum() {
        let quorum1 = quorum(0..5);
        let quorum2 = quorum(5..10);
        let upgrade_lock = UpgradeLock::<TestTypes>::new(Upgrade::trivial(EPOCH_VERSION));

        let chain = boundary_3_chain(&quorum1, &quorum1, &upgrade_lock);

        let stake_tables = EpochStakeTables {
            tables: vec![stake_table_for(1, &quorum1), stake_table_for(2, &quorum2)],
            epoch_height: EPOCH_HEIGHT,
        };
        verify_leaf_chain(chain, &stake_tables, 10, &upgrade_lock)
            .await
            .unwrap_err();
    }

    /// A QC claiming epoch 2 while certifying a leaf at an epoch 1 height must be rejected even
    /// when epoch 2's quorum validly signed it: the epoch is derived from the certified leaf's
    /// height, never taken from the QC's own claims.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_verify_leaf_chain_qc_epoch_block_mismatch() {
        let quorum1 = quorum(0..5);
        let quorum2 = quorum(5..10);
        let upgrade_lock = UpgradeLock::<TestTypes>::new(Upgrade::trivial(EPOCH_VERSION));

        // Same shape as `boundary_3_chain`, but the QC certifying block 10 (an epoch 1 block)
        // claims epoch 2 and carries epoch 2's signatures, dispatching verification to a stake
        // table the block's real quorum never belonged to.
        let qc9 = signed_qc(
            QuorumData2 {
                leaf_commit: Commitment::from_raw([9; 32]),
                epoch: Some(EpochNumber::new(1)),
                block_number: Some(9),
            },
            9,
            &quorum1,
            &upgrade_lock,
        );
        let leaf10 = make_leaf(10, 10, 1, qc9, EPOCH_VERSION);
        let forged_qc10 = signed_qc(
            QuorumData2 {
                leaf_commit: Committable::commit(&leaf10),
                epoch: Some(EpochNumber::new(2)),
                block_number: Some(10),
            },
            10,
            &quorum2,
            &upgrade_lock,
        );
        let leaf11 = make_leaf(11, 11, 2, forged_qc10, EPOCH_VERSION);
        let qc11 = signed_qc(
            QuorumData2 {
                leaf_commit: Committable::commit(&leaf11),
                epoch: Some(EpochNumber::new(2)),
                block_number: Some(11),
            },
            11,
            &quorum2,
            &upgrade_lock,
        );
        let leaf12 = make_leaf(12, 12, 2, qc11, EPOCH_VERSION);
        let chain = vec![leaf10, leaf11, leaf12];

        let stake_tables = EpochStakeTables {
            tables: vec![stake_table_for(1, &quorum1), stake_table_for(2, &quorum2)],
            epoch_height: EPOCH_HEIGHT,
        };
        let err = verify_leaf_chain(chain, &stake_tables, 10, &upgrade_lock)
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("claims epoch"),
            "expected claimed-epoch mismatch error, got: {err}"
        );
    }

    /// A QC whose claims are internally consistent (block 9 is in epoch 1) and validly signed by
    /// the correct epoch's quorum must still be rejected when its claimed block number does not
    /// match the height of the leaf it certifies.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_verify_leaf_chain_qc_certified_height_mismatch() {
        let quorum1 = quorum(0..5);
        let quorum2 = quorum(5..10);
        let upgrade_lock = UpgradeLock::<TestTypes>::new(Upgrade::trivial(EPOCH_VERSION));

        let qc9 = signed_qc(
            QuorumData2 {
                leaf_commit: Commitment::from_raw([9; 32]),
                epoch: Some(EpochNumber::new(1)),
                block_number: Some(9),
            },
            9,
            &quorum1,
            &upgrade_lock,
        );
        let leaf10 = make_leaf(10, 10, 1, qc9, EPOCH_VERSION);
        // Certifies the leaf at height 10 but claims block number 9.
        let forged_qc10 = signed_qc(
            QuorumData2 {
                leaf_commit: Committable::commit(&leaf10),
                epoch: Some(EpochNumber::new(1)),
                block_number: Some(9),
            },
            10,
            &quorum1,
            &upgrade_lock,
        );
        let leaf11 = make_leaf(11, 11, 2, forged_qc10, EPOCH_VERSION);
        let qc11 = signed_qc(
            QuorumData2 {
                leaf_commit: Committable::commit(&leaf11),
                epoch: Some(EpochNumber::new(2)),
                block_number: Some(11),
            },
            11,
            &quorum2,
            &upgrade_lock,
        );
        let leaf12 = make_leaf(12, 12, 2, qc11, EPOCH_VERSION);
        let chain = vec![leaf10, leaf11, leaf12];

        let stake_tables = EpochStakeTables {
            tables: vec![stake_table_for(1, &quorum1), stake_table_for(2, &quorum2)],
            epoch_height: EPOCH_HEIGHT,
        };
        let err = verify_leaf_chain(chain, &stake_tables, 10, &upgrade_lock)
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("claims block number"),
            "expected certified-height mismatch error, got: {err}"
        );
    }

    /// A new-protocol leaf range deciding the last block of epoch 1, finalized by a cert2 on the
    /// first block of epoch 2, signed by `cert2_quorum`.
    fn boundary_cert2_chain(
        quorum1: &Quorum,
        cert2_quorum: &Quorum,
        upgrade_lock: &UpgradeLock<TestTypes>,
    ) -> (Vec<Leaf2<TestTypes>>, Certificate2<TestTypes>) {
        let qc9 = signed_qc(
            QuorumData2 {
                leaf_commit: Commitment::from_raw([9; 32]),
                epoch: Some(EpochNumber::new(1)),
                block_number: Some(9),
            },
            9,
            quorum1,
            upgrade_lock,
        );
        let leaf10 = make_leaf(10, 10, 1, qc9, NEW_PROTOCOL_VERSION);
        let qc10 = signed_qc(
            QuorumData2 {
                leaf_commit: Committable::commit(&leaf10),
                epoch: Some(EpochNumber::new(1)),
                block_number: Some(10),
            },
            10,
            quorum1,
            upgrade_lock,
        );
        let leaf11 = make_leaf(11, 11, 2, qc10, NEW_PROTOCOL_VERSION);
        let cert2 = signed_cert2(
            Vote2Data {
                leaf_commit: Committable::commit(&leaf11),
                epoch: EpochNumber::new(2),
                block_number: 11,
            },
            11,
            cert2_quorum,
            upgrade_lock,
        );
        (vec![leaf10, leaf11], cert2)
    }

    /// A cert2 finalizing the last leaf of an epoch is produced in the next epoch and must be
    /// verified against the next epoch's stake table, while the boundary QC inside the chain is
    /// verified against the previous epoch's.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_verify_leaf_chain_with_cert2_across_epoch_boundary() {
        let quorum1 = quorum(0..5);
        let quorum2 = quorum(5..10);
        let upgrade_lock = UpgradeLock::<TestTypes>::new(Upgrade::trivial(NEW_PROTOCOL_VERSION));

        let (chain, cert2) = boundary_cert2_chain(&quorum1, &quorum2, &upgrade_lock);
        let expected = Committable::commit(&chain[0]);

        let stake_tables = EpochStakeTables {
            tables: vec![stake_table_for(1, &quorum1), stake_table_for(2, &quorum2)],
            epoch_height: EPOCH_HEIGHT,
        };
        let leaf = verify_leaf_chain_with_cert2(
            chain.clone(),
            &stake_tables,
            10,
            &upgrade_lock,
            cert2.clone(),
        )
        .await
        .unwrap();
        assert_eq!(Committable::commit(&leaf), expected);

        // With only the leaf's own epoch provided (the pre-fix behavior), verification must fail
        // instead of checking the cert2 against the wrong stake table.
        let only_epoch1 = EpochStakeTables {
            tables: vec![stake_table_for(1, &quorum1)],
            epoch_height: EPOCH_HEIGHT,
        };
        verify_leaf_chain_with_cert2(chain, &only_epoch1, 10, &upgrade_lock, cert2)
            .await
            .unwrap_err();
    }

    /// A cert2 claiming epoch 2 in its signed payload but signed by epoch 1's quorum must fail
    /// verification against epoch 2's stake table.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_verify_leaf_chain_with_cert2_wrong_quorum() {
        let quorum1 = quorum(0..5);
        let quorum2 = quorum(5..10);
        let upgrade_lock = UpgradeLock::<TestTypes>::new(Upgrade::trivial(NEW_PROTOCOL_VERSION));

        let (chain, cert2) = boundary_cert2_chain(&quorum1, &quorum1, &upgrade_lock);

        let stake_tables = EpochStakeTables {
            tables: vec![stake_table_for(1, &quorum1), stake_table_for(2, &quorum2)],
            epoch_height: EPOCH_HEIGHT,
        };
        verify_leaf_chain_with_cert2(chain, &stake_tables, 10, &upgrade_lock, cert2)
            .await
            .unwrap_err();
    }

    /// A cert2 claiming epoch 2 while certifying a leaf at an epoch 1 height must be rejected
    /// even when epoch 2's quorum validly signed it.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_verify_leaf_chain_with_cert2_epoch_block_mismatch() {
        let quorum1 = quorum(0..5);
        let quorum2 = quorum(5..10);
        let upgrade_lock = UpgradeLock::<TestTypes>::new(Upgrade::trivial(NEW_PROTOCOL_VERSION));

        let qc9 = signed_qc(
            QuorumData2 {
                leaf_commit: Commitment::from_raw([9; 32]),
                epoch: Some(EpochNumber::new(1)),
                block_number: Some(9),
            },
            9,
            &quorum1,
            &upgrade_lock,
        );
        let leaf10 = make_leaf(10, 10, 1, qc9, NEW_PROTOCOL_VERSION);
        // Block 10 is the last block of epoch 1, but the cert2 claims epoch 2, dispatching
        // verification to a stake table the block's real quorum never belonged to.
        let forged_cert2 = signed_cert2(
            Vote2Data {
                leaf_commit: Committable::commit(&leaf10),
                epoch: EpochNumber::new(2),
                block_number: 10,
            },
            10,
            &quorum2,
            &upgrade_lock,
        );

        let stake_tables = EpochStakeTables {
            tables: vec![stake_table_for(1, &quorum1), stake_table_for(2, &quorum2)],
            epoch_height: EPOCH_HEIGHT,
        };
        let err = verify_leaf_chain_with_cert2(
            vec![leaf10],
            &stake_tables,
            10,
            &upgrade_lock,
            forged_cert2,
        )
        .await
        .unwrap_err();
        assert!(
            err.to_string().contains("claims epoch"),
            "expected claimed-epoch mismatch error, got: {err}"
        );
    }
}
