// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, marker::PhantomData, sync::Arc};

    use alloy::primitives::U256;
    use bitvec::prelude::*;
    use committable::{Commitment, Committable};
    use hotshot_example_types::node_types::TestTypes;
    use hotshot_testing::helpers::key_pair_for_id;
    use hotshot_types::{
        consensus::{Consensus, ConsensusMetricsValue},
        data::{EpochNumber, ViewNumber},
        simple_certificate::{QuorumCertificate2, SimpleCertificate, SuccessThreshold},
        simple_vote::QuorumData2,
        stake_table::{supermajority_threshold, HSStakeTable},
        traits::{
            node_implementation::NodeType,
            signature_key::{SignatureKey, StateSignatureKey},
        },
        PeerConfig,
    };

    /// Build an n-node stake table (each node has 1 unit of stake) plus the
    /// corresponding supermajority threshold. Returns `(HSStakeTable, threshold)`.
    fn make_stake_table(n_nodes: usize) -> (HSStakeTable<TestTypes>, U256) {
        let peers: Vec<PeerConfig<TestTypes>> = (0..n_nodes)
            .map(|i| {
                let (_, pub_key) = key_pair_for_id::<TestTypes>(i as u64);
                let (state_key, _) =
                    <TestTypes as NodeType>::StateSignatureKey::generated_from_seed_indexed(
                        [0u8; 32],
                        i as u64,
                    );
                PeerConfig::<TestTypes> {
                    stake_table_entry: pub_key.stake_table_entry(U256::from(1u64)),
                    state_ver_key: state_key,
                }
            })
            .collect();
        let total_stake = U256::from(n_nodes as u64);
        let threshold = supermajority_threshold(total_stake);
        (HSStakeTable(peers), threshold)
    }

    /// Build a minimal `Consensus<TestTypes>` with the given stake table, threshold, and epoch.
    fn make_test_consensus(
        stake_table: HSStakeTable<TestTypes>,
        threshold: U256,
        epoch: Option<EpochNumber>,
    ) -> Consensus<TestTypes> {
        let genesis_data = QuorumData2::<TestTypes> {
            leaf_commit: Commitment::from_raw([0u8; 32]),
            epoch: None,
            block_number: None,
        };
        let genesis_qc: QuorumCertificate2<TestTypes> =
            SimpleCertificate::<TestTypes, QuorumData2<TestTypes>, SuccessThreshold>::new(
                genesis_data.clone(),
                genesis_data.commit(),
                ViewNumber::genesis(),
                None,
                PhantomData,
            );

        Consensus::new(
            BTreeMap::new(),
            None,
            ViewNumber::genesis(),
            epoch,
            ViewNumber::genesis(),
            ViewNumber::genesis(),
            ViewNumber::genesis(),
            BTreeMap::new(),
            std::collections::HashMap::new(),
            BTreeMap::new(),
            genesis_qc,
            None,
            Arc::new(ConsensusMetricsValue::default()),
            0, // epoch_height = 0 (no epochs)
            None,
            0, // drb_difficulty
            0, // drb_upgrade_difficulty
            stake_table,
            threshold,
        )
    }

    /// Build a `QuorumCertificate2<TestTypes>` for the given view, with the bitvec encoding
    /// which nodes (by index into `stake_table`) signed.
    ///
    /// The aggregated BLS signature is a dummy (any valid signature value) because
    /// `signers()` only reads the bitvec, not the aggregate signature.
    fn make_qc(
        stake_table: &HSStakeTable<TestTypes>,
        view: u64,
        signer_indices: &[usize],
        epoch: Option<EpochNumber>,
    ) -> QuorumCertificate2<TestTypes> {
        let n = stake_table.len();
        // Build a bitvec marking which nodes signed.
        let mut bv: BitVec = bitvec![0; n];
        for &idx in signer_indices {
            bv.set(idx, true);
        }
        // Get a valid BLS signature from the first signer (or signer 0 if none).
        // signers() only reads the bitvec; the aggregated sig is irrelevant.
        let (priv_key, _) = key_pair_for_id::<TestTypes>(
            signer_indices.first().copied().unwrap_or(0) as u64,
        );
        let dummy_sig = <TestTypes as NodeType>::SignatureKey::sign(&priv_key, &[0u8; 32])
            .expect("signing must succeed");
        let qc_type = (dummy_sig, bv);

        let data = QuorumData2::<TestTypes> {
            leaf_commit: Commitment::from_raw([0u8; 32]),
            epoch,
            block_number: Some(view),
        };
        SimpleCertificate::<TestTypes, QuorumData2<TestTypes>, SuccessThreshold>::new(
            data.clone(),
            data.commit(),
            ViewNumber::new(view),
            Some(qc_type),
            PhantomData,
        )
    }

    // ----- tests -----

    /// Fresh `Consensus` with 5 nodes and no views applied should report 0.0 participation
    /// for every node (zero-views branch in `calculate_ratio`).
    #[test]
    fn test_initial_state_all_zero_participation() {
        let (stake_table, threshold) = make_stake_table(5);
        let consensus = make_test_consensus(stake_table, threshold, None);

        let participation = consensus.current_vote_participation();
        assert_eq!(participation.len(), 5);
        for ratio in participation.values() {
            assert_eq!(*ratio, 0.0);
        }
    }

    /// After one QC where nodes [0,1,2] signed, those three nodes should have ratio 1.0
    /// and nodes [3,4] should have ratio 0.0.
    #[test]
    fn test_update_increments_voter_counts() {
        let (stake_table, threshold) = make_stake_table(5);
        let mut consensus = make_test_consensus(stake_table.clone(), threshold, None);

        let qc = make_qc(&stake_table, 5, &[0, 1, 2], None);
        consensus.update_vote_participation(qc).expect("update must succeed");

        let participation = consensus.current_vote_participation();
        assert_eq!(participation.len(), 5);

        // Nodes 0, 1, 2 signed in 1 of 1 views.
        let node_keys: Vec<_> = (0..5)
            .map(|i| key_pair_for_id::<TestTypes>(i).1)
            .collect();
        for i in 0..3usize {
            let ratio = participation[&node_keys[i]];
            assert!(
                (ratio - 1.0).abs() < f64::EPSILON,
                "node {i} expected 1.0, got {ratio}"
            );
        }
        // Nodes 3, 4 did not sign.
        for i in 3..5usize {
            let ratio = participation[&node_keys[i]];
            assert_eq!(ratio, 0.0, "node {i} expected 0.0, got {ratio}");
        }
    }

    /// Three views: nodes [0,1,2] sign views 1 and 3, all four nodes sign view 2.
    /// After 3 views: nodes 0,1,2 have ratio 3/3 = 1.0; node 3 has ratio 1/3.
    #[test]
    fn test_multiple_views_ratio_calculation() {
        let (stake_table, threshold) = make_stake_table(4);
        let mut consensus = make_test_consensus(stake_table.clone(), threshold, None);

        let qc1 = make_qc(&stake_table, 1, &[0, 1, 2], None);
        let qc2 = make_qc(&stake_table, 2, &[0, 1, 2, 3], None);
        let qc3 = make_qc(&stake_table, 3, &[0, 1, 2], None);

        consensus.update_vote_participation(qc1).unwrap();
        consensus.update_vote_participation(qc2).unwrap();
        consensus.update_vote_participation(qc3).unwrap();

        let participation = consensus.current_vote_participation();

        let node_keys: Vec<_> = (0..4).map(|i| key_pair_for_id::<TestTypes>(i).1).collect();

        for i in 0..3usize {
            let ratio = participation[&node_keys[i]];
            assert!(
                (ratio - 1.0).abs() < f64::EPSILON,
                "node {i}: expected 1.0, got {ratio}"
            );
        }
        // Node 3: 1 out of 3 views.
        let ratio3 = participation[&node_keys[3]];
        let expected3 = 1.0 / 3.0;
        assert!(
            (ratio3 - expected3).abs() < f64::EPSILON,
            "node 3: expected {expected3}, got {ratio3}"
        );
    }

    /// Applying a QC for the same view twice should return an error on the second call
    /// and leave participation counts unchanged.
    #[test]
    fn test_duplicate_view_ignored() {
        let (stake_table, threshold) = make_stake_table(4);
        let mut consensus = make_test_consensus(stake_table.clone(), threshold, None);

        // First application for view 7 should succeed.
        let qc = make_qc(&stake_table, 7, &[0, 1, 2, 3], None);
        consensus.update_vote_participation(qc.clone()).unwrap();

        // Second application for the same view 7 should be rejected.
        let result = consensus.update_vote_participation(qc);
        assert!(result.is_err(), "duplicate view should return Err");

        // Participation should still reflect only 1 view.
        let participation = consensus.current_vote_participation();
        assert_eq!(participation.len(), 4);
        for ratio in participation.values() {
            assert!(
                (ratio - 1.0).abs() < f64::EPSILON,
                "all four nodes should have ratio 1.0 after exactly 1 view"
            );
        }
    }

    /// A QC whose epoch does not match the current epoch should be rejected.
    #[test]
    fn test_wrong_epoch_returns_error() {
        let (stake_table, threshold) = make_stake_table(3);
        // Current epoch = None.
        let mut consensus = make_test_consensus(stake_table.clone(), threshold, None);

        // QC with epoch = Some(1) does not match current epoch (None).
        let qc = make_qc(&stake_table, 2, &[0, 1, 2], Some(EpochNumber::new(1)));
        let result = consensus.update_vote_participation(qc);
        assert!(result.is_err(), "mismatched epoch should return Err");
    }

    /// Applying a genesis-view QC (view == 0) returns an empty signers list.
    /// The view count is still incremented, but no individual vote counters are.
    #[test]
    fn test_genesis_qc_increments_view_but_not_votes() {
        let (stake_table, threshold) = make_stake_table(3);
        let mut consensus = make_test_consensus(stake_table.clone(), threshold, None);

        // Build a genesis-view QC (view == 0). SimpleCertificate::signers() returns Ok([])
        // for genesis view without looking at the signature.
        let genesis_data = QuorumData2::<TestTypes> {
            leaf_commit: Commitment::from_raw([0u8; 32]),
            epoch: None,
            block_number: None,
        };
        let genesis_qc: QuorumCertificate2<TestTypes> =
            SimpleCertificate::<TestTypes, QuorumData2<TestTypes>, SuccessThreshold>::new(
                genesis_data.clone(),
                genesis_data.commit(),
                ViewNumber::genesis(),
                // Provide a Some signature so the QC is not rejected for missing sigs
                // (but signers() returns [] early for genesis view anyway).
                None,
                PhantomData,
            );

        // For a genesis QC with None signatures, SimpleCertificate::signers() returns Ok([])
        // without even looking at the signature. So update_vote_participation succeeds.
        let result = consensus.update_vote_participation(genesis_qc);
        assert!(result.is_ok(), "genesis QC update should succeed");

        // Individual vote counters should all be 0 (no signer incremented).
        let node_keys: Vec<_> = (0..3).map(|i| key_pair_for_id::<TestTypes>(i).1).collect();
        let participation = consensus.current_vote_participation();
        for key in &node_keys {
            let ratio = participation[key];
            // 1 view total, 0 votes for each node → ratio = 0 / 1 = 0.0
            assert_eq!(ratio, 0.0, "no node should have votes from genesis QC");
        }
    }

    /// After an epoch transition, `previous_vote_participation` contains the old epoch's
    /// data and `current_vote_participation` is freshly zeroed.
    #[test]
    fn test_epoch_transition_preserves_history() {
        let (stake_table, threshold) = make_stake_table(5);
        let mut consensus = make_test_consensus(stake_table.clone(), threshold, None);

        // Apply 3 QCs in epoch None; nodes [0,1,2] sign each.
        for view in 1u64..=3 {
            let qc = make_qc(&stake_table, view, &[0, 1, 2], None);
            consensus.update_vote_participation(qc).unwrap();
        }

        // Transition to epoch 1.
        let result = consensus.update_vote_participation_epoch(
            stake_table.clone(),
            threshold,
            Some(EpochNumber::new(1)),
        );
        assert!(result.is_ok(), "epoch transition should succeed");

        let node_keys: Vec<_> = (0..5).map(|i| key_pair_for_id::<TestTypes>(i).1).collect();

        // Previous epoch: nodes 0,1,2 each signed 3 of 3 views → 1.0;
        // nodes 3,4 signed 0 of 3 views → 0.0.
        let prev = consensus.vote_participation(None);
        for i in 0..3usize {
            let ratio = prev[&node_keys[i]];
            assert!(
                (ratio - 1.0).abs() < f64::EPSILON,
                "node {i} prev ratio: expected 1.0, got {ratio}"
            );
        }
        for i in 3..5usize {
            let ratio = prev[&node_keys[i]];
            assert_eq!(ratio, 0.0, "node {i} prev ratio: expected 0.0, got {ratio}");
        }

        // Current epoch: all 0.0 (no views yet).
        let curr = consensus.current_vote_participation();
        for ratio in curr.values() {
            assert_eq!(*ratio, 0.0, "fresh epoch should have 0.0 participation");
        }
    }

    /// Transitioning to the same epoch (or a lower epoch) should return an error.
    #[test]
    fn test_epoch_transition_rejects_same_or_lower_epoch() {
        let (stake_table, threshold) = make_stake_table(3);
        let mut consensus =
            make_test_consensus(stake_table.clone(), threshold, Some(EpochNumber::new(3)));

        // Transition to the same epoch → Err.
        assert!(
            consensus
                .update_vote_participation_epoch(
                    stake_table.clone(),
                    threshold,
                    Some(EpochNumber::new(3)),
                )
                .is_err(),
            "same epoch should return Err"
        );

        // Transition to a lower epoch → Err.
        assert!(
            consensus
                .update_vote_participation_epoch(
                    stake_table.clone(),
                    threshold,
                    Some(EpochNumber::new(2)),
                )
                .is_err(),
            "lower epoch should return Err"
        );
    }

    /// `previous_vote_participation` is empty before any epoch transition.
    #[test]
    fn test_previous_participation_empty_before_epoch_change() {
        let (stake_table, threshold) = make_stake_table(3);
        let consensus = make_test_consensus(stake_table, threshold, None);

        let prev = consensus.vote_participation(None);
        assert!(
            prev.is_empty(),
            "previous participation should be empty before any epoch change"
        );
    }
}
