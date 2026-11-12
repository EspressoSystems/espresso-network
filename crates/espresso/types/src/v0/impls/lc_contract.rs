//! The LC contract's finalized state.

use anyhow::ensure;
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    light_client::CircuitField,
    traits::block_contents::BlockHeader,
    utils::{epoch_from_block_number, root_block_in_epoch},
};

use crate::Header;

/// The LC contract's `finalizedState` at one L1 finalized block: `F` and the block commitment
/// root of the header at `F`. Only headers below `F` are provable against that header.
#[derive(Clone, Debug, PartialEq)]
pub struct LcContractFinalizedState {
    /// `finalizedState.blockHeight`: `F`.
    pub height: u64,
    /// `finalizedState.blockCommRoot`: hash of the block Merkle root of the header at `F`
    /// (`hotshot_types::light_client::CircuitField`).
    pub block_comm_root: CircuitField,
}

impl LcContractFinalizedState {
    /// `E* = epoch(F)`: the epoch the seed installs up to `ST(E*+1)`. Pure.
    pub fn seed_epoch(&self, epoch_height: u64) -> EpochNumber {
        EpochNumber::new(epoch_from_block_number(self.height, epoch_height))
    }

    /// The first epoch whose table the seed installs above the held top `start`:
    /// `max(E*-3, start+1, 3)`. The seed installs `ST(first) ..= ST(E*+1)`. Pure.
    pub fn first_installed_epoch(&self, epoch_height: u64, start: EpochNumber) -> EpochNumber {
        let seed_epoch = *self.seed_epoch(epoch_height);
        // `ST(3)` is the first table defined by an epoch root.
        EpochNumber::new(seed_epoch.saturating_sub(3).max(*start + 1).max(3))
    }

    /// Whether a seed is needed: `seed_epoch > start + 1` and `target >= first_installed + 2`.
    /// The DRB of `target` needs `ST(target-1)` and `ST(target-2)`. Pure.
    pub fn seeds_target(&self, epoch_height: u64, target: EpochNumber, start: EpochNumber) -> bool {
        let first_installed = self.first_installed_epoch(epoch_height, start);
        self.seed_epoch(epoch_height) > start + 1 && target >= first_installed + 2
    }

    /// Heights `root(e-2)` defining `ST(e)` for `e` in `first_installed_epoch ..= E*+1`, ascending.
    /// Every height is positive and below `F`. Pure.
    pub fn proven_root_heights(&self, epoch_height: u64, start: EpochNumber) -> Vec<u64> {
        let first = *self.first_installed_epoch(epoch_height, start);
        (first..=*self.seed_epoch(epoch_height) + 1)
            .map(|epoch| root_block_in_epoch(epoch - 2, epoch_height))
            .collect()
    }

    /// Height of `root(first-3)`, the header the block reward of `ST(first)` takes its block time
    /// from; `None` when nothing is installed or `first < 4`. Below `F`. Pure.
    pub fn preceding_root_height(&self, epoch_height: u64, start: EpochNumber) -> Option<u64> {
        let first = *self.first_installed_epoch(epoch_height, start);
        (first >= 4 && first <= *self.seed_epoch(epoch_height) + 1)
            .then(|| root_block_in_epoch(first - 3, epoch_height))
    }

    /// Accepts `header` iff it is at `F` and `Header::get_light_client_state(..).block_comm_root`
    /// equals the LC contract's. Callers trust only its block Merkle root afterwards. Pure.
    pub fn verify_header(&self, header: &Header) -> anyhow::Result<()> {
        ensure!(
            header.height() == self.height,
            "header is at height {}, expected {}",
            header.height(),
            self.height
        );
        let block_comm_root = header
            .get_light_client_state(ViewNumber::new(0))?
            .block_comm_root;
        ensure!(
            block_comm_root == self.block_comm_root,
            "header at height {} does not match the LC contract block commitment root",
            self.height
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use committable::Commitment;
    use hotshot_query_service::testing::mocks::MOCK_UPGRADE;
    use hotshot_types::{traits::BlockPayload, utils::is_epoch_root};
    use jf_merkle_tree_compat::{AppendableMerkleTreeScheme, MerkleTreeScheme};
    use proptest::prelude::*;
    use rstest::rstest;

    use super::*;
    use crate::{BLOCK_MERKLE_TREE_HEIGHT, BlockMerkleTree, NodeState, Payload};

    const EPOCH_HEIGHT: u64 = 100;

    /// A V4 genesis header moved to `height`.
    async fn header_at(height: u64) -> Header {
        let mut instance = NodeState::mock_v2();
        instance.genesis_version = versions::version(0, 4);
        let (payload, metadata) =
            Payload::from_transactions(&[], &instance.genesis_state, &instance)
                .await
                .expect("payload");
        match Header::genesis(&instance, payload, &metadata, MOCK_UPGRADE.base) {
            Header::V4(mut fields) => {
                fields.height = height;
                Header::V4(fields)
            },
            other => panic!("expected a V4 header, got {other:?}"),
        }
    }

    fn state_of(header: &Header) -> LcContractFinalizedState {
        LcContractFinalizedState {
            height: header.height(),
            block_comm_root: header
                .get_light_client_state(ViewNumber::new(0))
                .unwrap()
                .block_comm_root,
        }
    }

    /// Only the header at `F` with the contract's block commitment root is accepted.
    #[tokio::test]
    async fn verify_header_cases_ok() {
        let header = header_at(62).await;
        let lc = state_of(&header);

        lc.verify_header(&header).unwrap();
        lc.verify_header(&header_at(61).await).unwrap_err();

        let mut tree = BlockMerkleTree::new(BLOCK_MERKLE_TREE_HEIGHT);
        tree.push(Commitment::from_raw([1; 32])).unwrap();
        let mut forged = header;
        *forged.block_merkle_tree_root_mut() = tree.commitment();
        lc.verify_header(&forged).unwrap_err();
    }

    fn finalized_state(height: u64) -> LcContractFinalizedState {
        LcContractFinalizedState {
            height,
            block_comm_root: CircuitField::default(),
        }
    }

    #[rstest]
    // Far from genesis: `root(E*-5) ..= root(E*-1)`, defining `ST(E*-3) ..= ST(E*+1)`, and
    // `root(E*-6)` before them.
    #[case(1000, 1, vec![495, 595, 695, 795, 895], Some(395))]
    #[case(1000, 5, vec![495, 595, 695, 795, 895], Some(395))]
    // Epochs up to `start` are held.
    #[case(1000, 7, vec![595, 695, 795, 895], Some(495))]
    #[case(1000, 9, vec![795, 895], Some(695))]
    #[case(1000, 11, vec![], None)]
    // Near genesis: nothing below `ST(3)`, and no preceding root below epoch 4.
    #[case(400, 2, vec![95, 195, 295], None)]
    #[case(400, 4, vec![295], Some(195))]
    #[case(150, 2, vec![95], None)]
    fn proven_root_heights_cases(
        #[case] height: u64,
        #[case] start: u64,
        #[case] expected: Vec<u64>,
        #[case] expected_preceding: Option<u64>,
    ) {
        let lc = finalized_state(height);
        let start = EpochNumber::new(start);

        assert_eq!(lc.proven_root_heights(EPOCH_HEIGHT, start), expected);
        assert_eq!(
            lc.preceding_root_height(EPOCH_HEIGHT, start),
            expected_preceding
        );
    }

    #[rstest]
    // The seed epoch is at most `start + 1`.
    #[case(4, 9, 3, false)]
    #[case(3, 3, 2, false)]
    // The window is `ST(7) ..= ST(11)`: its two lowest tables lack `ST(target-2)`.
    #[case(10, 6, 2, false)]
    #[case(10, 7, 2, false)]
    #[case(10, 8, 2, false)]
    #[case(10, 9, 2, true)]
    #[case(10, 10, 2, true)]
    #[case(10, 15, 2, true)]
    // The held pair is the seeded genesis pair: the window is `ST(3) ..= ST(7)`.
    #[case(6, 4, 2, false)]
    #[case(6, 5, 2, true)]
    #[case(4, 4, 2, false)]
    #[case(4, 5, 2, true)]
    fn seeds_target_cases(
        #[case] seed_epoch: u64,
        #[case] target: u64,
        #[case] start: u64,
        #[case] expected: bool,
    ) {
        // The last block of `seed_epoch`.
        let lc = finalized_state(seed_epoch * 10);

        assert_eq!(
            lc.seeds_target(10, EpochNumber::new(target), EpochNumber::new(start)),
            expected
        );
    }

    /// A batch that failed after exactly one root leaves a lone table, so the held pair top stays
    /// and the next seed covers the same window. After two roots the pair `ST(7), ST(8)` is held
    /// and the next seed starts above it.
    #[test]
    fn seed_after_failed_batch_resumes_ok() {
        let lc = finalized_state(100);
        let (target, genesis_pair_top, pair_top) = (
            EpochNumber::new(11),
            EpochNumber::new(2),
            EpochNumber::new(8),
        );
        let window = lc.proven_root_heights(10, genesis_pair_top);
        assert_eq!(window, vec![45, 55, 65, 75, 85]);

        // Exactly one root installed: `ST(7)` is held, the top is still the genesis pair's.
        assert!(lc.seeds_target(10, target, genesis_pair_top));
        assert_eq!(lc.proven_root_heights(10, genesis_pair_top), window);

        assert!(lc.seeds_target(10, target, pair_top));
        assert_eq!(lc.proven_root_heights(10, pair_top), window[2..].to_vec());
    }

    proptest! {
        #[test]
        fn proven_roots_below_f_ok(
            height in 1u64..10_000,
            epoch_height in prop::sample::select(vec![10u64, 100]),
            start in 2u64..120,
        ) {
            let lc = finalized_state(height);
            let seed_epoch = *lc.seed_epoch(epoch_height);
            let heights = lc.proven_root_heights(epoch_height, EpochNumber::new(start));

            for root in &heights {
                prop_assert!(*root > 0 && *root < height);
                prop_assert!(is_epoch_root(*root, epoch_height));
            }
            let preceding = lc.preceding_root_height(epoch_height, EpochNumber::new(start));
            let epochs: Vec<u64> = heights
                .iter()
                .map(|root| (root + 5) / epoch_height + 2)
                .collect();
            for (epoch, root) in epochs.iter().zip(&heights) {
                prop_assert_eq!(*root, root_block_in_epoch(epoch - 2, epoch_height));
            }
            if let (Some(first), Some(last)) = (epochs.first(), epochs.last()) {
                prop_assert_eq!(*first, seed_epoch.saturating_sub(3).max(start + 1).max(3));
                prop_assert_eq!(
                    preceding,
                    (*first >= 4).then(|| root_block_in_epoch(first - 3, epoch_height))
                );
                prop_assert!(preceding.is_none_or(|root| root < height));
                prop_assert_eq!(*last, seed_epoch + 1);
                prop_assert!(epochs.windows(2).all(|pair| pair[0] + 1 == pair[1]));
            }
        }
    }
}
