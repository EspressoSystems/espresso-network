//! The LC contract's finalized state.

use std::ops::RangeInclusive;

use anyhow::ensure;
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    light_client::CircuitField,
    traits::block_contents::BlockHeader,
    utils::{epoch_from_block_number, root_block_in_epoch},
};

use crate::{BlockMerkleCommitment, Header};

/// The LC contract's `finalizedState` at one L1 finalized block: `F` and the block commitment
/// root of the header at `F`. Only headers below `F` are provable against that header.
///
/// An `epoch_height` of 5 or less is unsupported: the chain cannot run that way.
#[derive(Clone, Debug, PartialEq)]
pub struct LcContractFinalizedState {
    /// `finalizedState.blockHeight`: `F`.
    pub height: u64,
    /// `finalizedState.blockCommRoot`: hash of the block Merkle root of the header at `F`
    /// (`hotshot_types::light_client::CircuitField`).
    pub block_comm_root: CircuitField,
}

/// What a seed installs for `catchup(target)` when `start` is the top of the held pair.
#[derive(Clone, Debug, PartialEq)]
pub struct SeedWindow {
    /// Epochs whose stake tables are installed, ascending.
    pub epochs: RangeInclusive<EpochNumber>,
    /// Height of the epoch root below the lowest one, when its block reward reads it.
    pub preceding_root: Option<u64>,
}

impl SeedWindow {
    /// Heights of the epoch roots that define the window's tables, ascending.
    pub fn root_heights(&self, epoch_height: u64) -> impl Iterator<Item = u64> {
        (self.epochs.start().u64()..=self.epochs.end().u64())
            .map(move |epoch| root_block_in_epoch(epoch - 2, epoch_height))
    }
}

impl LcContractFinalizedState {
    /// The epoch of `F`.
    pub fn seed_epoch(&self, epoch_height: u64) -> EpochNumber {
        EpochNumber::new(epoch_from_block_number(self.height, epoch_height))
    }

    /// The window a seed installs, or `None` when the walk needs no seed.
    pub fn seed_window(
        &self,
        epoch_height: u64,
        first_epoch: EpochNumber,
        start: EpochNumber,
        target: EpochNumber,
    ) -> Option<SeedWindow> {
        let contract_epoch = self.seed_epoch(epoch_height);
        // The walk reaches the target, or the contract epoch, on its own from the held pair.
        if target <= start + 1 || contract_epoch <= start + 1 {
            return None;
        }
        // Only epoch roots below the finalized height can be proven.
        let last = target.min(contract_epoch + 1);
        // Five tables cover the DRB and block rewards of the target; held tables are not
        // installed again; epoch 3 is the first table an epoch root defines.
        let first = EpochNumber::new(last.saturating_sub(4).max(*start + 1).max(3));
        // The lowest block reward takes its block time from this root, except near genesis.
        let preceding_root =
            (first > first_epoch + 4).then(|| root_block_in_epoch(*first - 3, epoch_height));
        Some(SeedWindow {
            epochs: first..=last,
            preceding_root,
        })
    }

    /// Accepts `header` iff it is at `F` and `Header::get_light_client_state(..).block_comm_root`
    /// equals the LC contract's, and returns its block Merkle root, the only part to trust. Pure.
    pub fn verify_header(&self, header: &Header) -> anyhow::Result<BlockMerkleCommitment> {
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
        Ok(header.block_merkle_tree_root())
    }
}

#[cfg(test)]
mod tests {
    use committable::Commitment;
    use hotshot_query_service::testing::mocks::MOCK_UPGRADE;
    use hotshot_types::traits::BlockPayload;
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

        assert_eq!(
            lc.verify_header(&header).unwrap(),
            header.block_merkle_tree_root()
        );
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

    /// The first epoch of the chain in the tests below.
    const FIRST_EPOCH: u64 = 1;

    /// The epochs and root heights of a window, for comparison with a table row.
    fn window_parts(window: SeedWindow, epoch_height: u64) -> (u64, u64, Vec<u64>, Option<u64>) {
        (
            window.epochs.start().u64(),
            window.epochs.end().u64(),
            window.root_heights(epoch_height).collect(),
            window.preceding_root,
        )
    }

    /// Rows are `(F, first_epoch, start, target)` and the expected `(first epoch, last epoch,
    /// root heights, preceding root)`. The contract epoch is `ceil(F / 100)`.
    #[rstest]
    // Contract epoch 10. A target at or above 11 gets epochs 7 to 11 and the root below them.
    #[case(1000, FIRST_EPOCH, 1, 99, Some((7, 11, vec![495, 595, 695, 795, 895], Some(395))))]
    #[case(1000, FIRST_EPOCH, 5, 99, Some((7, 11, vec![495, 595, 695, 795, 895], Some(395))))]
    #[case(1000, FIRST_EPOCH, 1, 11, Some((7, 11, vec![495, 595, 695, 795, 895], Some(395))))]
    #[case(1000, FIRST_EPOCH, 1, 12, Some((7, 11, vec![495, 595, 695, 795, 895], Some(395))))]
    // `F` at the root of the contract epoch and at its first block.
    #[case(995, FIRST_EPOCH, 2, 99, Some((7, 11, vec![495, 595, 695, 795, 895], Some(395))))]
    #[case(901, FIRST_EPOCH, 2, 99, Some((7, 11, vec![495, 595, 695, 795, 895], Some(395))))]
    // Epochs up to `start` are held.
    #[case(1000, FIRST_EPOCH, 7, 99, Some((8, 11, vec![595, 695, 795, 895], Some(495))))]
    // The contract epoch is within one epoch of `start`: no seed.
    #[case(1000, FIRST_EPOCH, 9, 99, None)]
    #[case(1000, FIRST_EPOCH, 11, 99, None)]
    // A target below 11 gets its own window, topped by its table.
    #[case(1000, FIRST_EPOCH, 1, 8, Some((4, 8, vec![195, 295, 395, 495, 595], None)))]
    #[case(1000, FIRST_EPOCH, 1, 3, Some((3, 3, vec![95], None)))]
    #[case(1000, FIRST_EPOCH, 5, 9, Some((6, 9, vec![395, 495, 595, 695], Some(295))))]
    #[case(1000, FIRST_EPOCH, 5, 7, Some((6, 7, vec![395, 495], Some(295))))]
    // The reward of the lowest table assumes the block time when its epoch is at most
    // `first_epoch + 4`.
    #[case(1000, FIRST_EPOCH, 2, 7, Some((3, 7, vec![95, 195, 295, 395, 495], None)))]
    #[case(1000, FIRST_EPOCH, 2, 8, Some((4, 8, vec![195, 295, 395, 495, 595], None)))]
    #[case(1000, FIRST_EPOCH, 2, 9, Some((5, 9, vec![295, 395, 495, 595, 695], None)))]
    #[case(1000, FIRST_EPOCH, 2, 10, Some((6, 10, vec![395, 495, 595, 695, 795], Some(295))))]
    #[case(1000, FIRST_EPOCH, 2, 11, Some((7, 11, vec![495, 595, 695, 795, 895], Some(395))))]
    #[case(1000, FIRST_EPOCH, 2, 15, Some((7, 11, vec![495, 595, 695, 795, 895], Some(395))))]
    // Near genesis: nothing below epoch 3, and no seed when the contract epoch is at most
    // `start + 1`.
    #[case(400, FIRST_EPOCH, 2, 99, Some((3, 5, vec![95, 195, 295], None)))]
    #[case(400, FIRST_EPOCH, 4, 99, None)]
    #[case(150, FIRST_EPOCH, 2, 99, None)]
    #[case(1000, FIRST_EPOCH, 2, 4, Some((3, 4, vec![95, 195], None)))]
    #[case(1000, FIRST_EPOCH, 2, 6, Some((3, 6, vec![95, 195, 295, 395], None)))]
    #[case(600, FIRST_EPOCH, 2, 4, Some((3, 4, vec![95, 195], None)))]
    #[case(600, FIRST_EPOCH, 2, 5, Some((3, 5, vec![95, 195, 295], None)))]
    #[case(400, FIRST_EPOCH, 2, 4, Some((3, 4, vec![95, 195], None)))]
    #[case(400, FIRST_EPOCH, 2, 5, Some((3, 5, vec![95, 195, 295], None)))]
    // The target is within one epoch of `start`, or the contract epoch is within one of it.
    #[case(400, FIRST_EPOCH, 3, 9, None)]
    #[case(300, FIRST_EPOCH, 2, 3, None)]
    #[case(1000, FIRST_EPOCH, 2, 3, None)]
    #[case(1000, FIRST_EPOCH, 2, 2, None)]
    // A chain far from genesis: the reward reads the root below the lowest table once its epoch
    // is above `first_epoch + 4`.
    #[case(28_300, 275, 276, 999, Some((280, 284, vec![27_795, 27_895, 27_995, 28_095, 28_195], Some(27_695))))]
    #[case(28_200, 275, 276, 999, Some((279, 283, vec![27_695, 27_795, 27_895, 27_995, 28_095], None)))]
    fn seed_window_cases(
        #[case] height: u64,
        #[case] first_epoch: u64,
        #[case] start: u64,
        #[case] target: u64,
        #[case] expected: Option<(u64, u64, Vec<u64>, Option<u64>)>,
    ) {
        let window = finalized_state(height).seed_window(
            EPOCH_HEIGHT,
            EpochNumber::new(first_epoch),
            EpochNumber::new(start),
            EpochNumber::new(target),
        );

        assert_eq!(window.map(|w| window_parts(w, EPOCH_HEIGHT)), expected);
    }

    /// A seed from the genesis pair top covers the whole window; from top 8 it covers the window
    /// without its first two roots.
    #[test]
    fn seed_after_failed_batch_resumes_ok() {
        let lc = finalized_state(100);
        let first_epoch = EpochNumber::new(1);
        let target = EpochNumber::new(11);
        let roots = |start: u64| -> Vec<u64> {
            lc.seed_window(10, first_epoch, EpochNumber::new(start), target)
                .expect("seed needed")
                .root_heights(10)
                .collect()
        };
        let window = roots(2);
        assert_eq!(window, vec![45, 55, 65, 75, 85]);

        assert_eq!(roots(8), window[2..].to_vec());
    }

    proptest! {
        #[test]
        fn seed_window_ok(
            height in 1u64..100_000,
            epoch_height in prop::sample::select(vec![10u64, 100]),
            first_epoch in 1u64..300,
            start_above_first in 1u64..120,
            target in 0u64..500,
        ) {
            let lc = finalized_state(height);
            let contract_epoch = *lc.seed_epoch(epoch_height);
            let start = first_epoch + start_above_first;
            let window = lc.seed_window(
                epoch_height,
                EpochNumber::new(first_epoch),
                EpochNumber::new(start),
                EpochNumber::new(target),
            );

            prop_assert_eq!(
                window.is_some(),
                target > start + 1 && contract_epoch > start + 1
            );
            let Some(window) = window else { return Ok(()) };
            let (first, last) = (window.epochs.start().u64(), window.epochs.end().u64());
            let heights: Vec<u64> = window.root_heights(epoch_height).collect();

            prop_assert!(first >= first_epoch + 2 && first > start);
            prop_assert!(last <= target && last <= contract_epoch + 1);
            prop_assert_eq!(heights.len() as u64, last + 1 - first);
            for (epoch, root) in (first..=last).zip(&heights) {
                prop_assert_eq!(*root, root_block_in_epoch(epoch - 2, epoch_height));
                prop_assert!(*root > 0 && *root < height);
            }
            // The DRB of the target needs the two tables below it, installed or held.
            for below in [last - 2, last - 1] {
                prop_assert!((first..=last).contains(&below) || below == start || below + 1 == start);
            }
            // The reward of the lowest table reads the root one epoch below its own root unless
            // the epoch before that root's is within the first two.
            let previous_epoch = epoch_from_block_number(heights[0], epoch_height) - 1;
            prop_assert_eq!(
                window.preceding_root,
                (previous_epoch > first_epoch + 1).then(|| heights[0] - epoch_height)
            );
        }
    }
}
