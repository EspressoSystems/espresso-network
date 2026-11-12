//! The LC contract's finalized state.

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

impl LcContractFinalizedState {
    /// `E* = epoch(F)`: the epoch of the seed's anchor. Pure.
    pub fn seed_epoch(&self, epoch_height: u64) -> EpochNumber {
        EpochNumber::new(epoch_from_block_number(self.height, epoch_height))
    }

    /// The top table the seed installs: `min(E*+1, target)`. Roots at or above `F` are not
    /// provable, so the top is never above `E*+1`. Pure.
    pub fn top_installed_epoch(&self, epoch_height: u64, target: EpochNumber) -> EpochNumber {
        target.min(self.seed_epoch(epoch_height) + 1)
    }

    /// The first epoch whose table the seed installs above the held top `start`:
    /// `max(top-4, start+1, 3)` with `top = min(E*+1, target)`. Pure.
    pub fn first_installed_epoch(
        &self,
        epoch_height: u64,
        start: EpochNumber,
        target: EpochNumber,
    ) -> EpochNumber {
        let top = *self.top_installed_epoch(epoch_height, target);
        // `ST(3)` is the first table defined by an epoch root.
        EpochNumber::new(top.saturating_sub(4).max(*start + 1).max(3))
    }

    /// Whether a seed is needed: `target > start + 1` and `E* > start + 1`.
    ///
    /// The DRB of a target at or below `E*+1` needs `ST(target-1)` and `ST(target-2)`, installed
    /// or held. Both hold: `top-2 >= start` is held, and anything above `start` up to `top` is
    /// installed. A target above `E*+1` continues as a walk from `ST(E*+1)`. Pure.
    pub fn seeds_target(&self, epoch_height: u64, target: EpochNumber, start: EpochNumber) -> bool {
        target > start + 1 && self.seed_epoch(epoch_height) > start + 1
    }

    /// Heights `root(e-2)` defining `ST(e)` for `e` in `first_installed_epoch ..= top`, ascending.
    /// Every height is positive and below `F`. Pure.
    pub fn proven_root_heights(
        &self,
        epoch_height: u64,
        start: EpochNumber,
        target: EpochNumber,
    ) -> Vec<u64> {
        let first = *self.first_installed_epoch(epoch_height, start, target);
        (first..=*self.top_installed_epoch(epoch_height, target))
            .map(|epoch| root_block_in_epoch(epoch - 2, epoch_height))
            .collect()
    }

    /// Height of `root(first-3)`, the header the block reward of `ST(first)` takes its block time
    /// from; `None` when that reward uses the assumed block time (`first - 3 <= first_epoch + 1`)
    /// or nothing is installed. Below `F`. Pure.
    pub fn preceding_root_height(
        &self,
        epoch_height: u64,
        first_epoch: EpochNumber,
        start: EpochNumber,
        target: EpochNumber,
    ) -> Option<u64> {
        let first = self.first_installed_epoch(epoch_height, start, target);
        (first <= self.top_installed_epoch(epoch_height, target) && first > first_epoch + 4)
            .then(|| root_block_in_epoch(*first - 3, epoch_height))
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

    #[rstest]
    // `E* = 10`. A target at or above `E*+1` gets `ST(E*-3) ..= ST(E*+1)` from `root(E*-5) ..=
    // root(E*-1)`, and `root(E*-6)` before them.
    #[case(1000, 1, 99, vec![495, 595, 695, 795, 895], Some(395))]
    #[case(1000, 5, 99, vec![495, 595, 695, 795, 895], Some(395))]
    #[case(1000, 1, 11, vec![495, 595, 695, 795, 895], Some(395))]
    #[case(1000, 1, 12, vec![495, 595, 695, 795, 895], Some(395))]
    // Epochs up to `start` are held.
    #[case(1000, 7, 99, vec![595, 695, 795, 895], Some(495))]
    #[case(1000, 9, 99, vec![795, 895], Some(695))]
    #[case(1000, 11, 99, vec![], None)]
    // A target below `E*+1` gets its own window, topped by its table.
    #[case(1000, 1, 8, vec![195, 295, 395, 495, 595], None)]
    #[case(1000, 1, 3, vec![95], None)]
    #[case(1000, 5, 9, vec![395, 495, 595, 695], Some(295))]
    #[case(1000, 5, 7, vec![395, 495], Some(295))]
    // The reward of `ST(first)` assumes the block time when `first - 3 <= FIRST_EPOCH + 1`.
    #[case(1000, 2, 7, vec![95, 195, 295, 395, 495], None)]
    #[case(1000, 2, 8, vec![195, 295, 395, 495, 595], None)]
    #[case(1000, 2, 9, vec![295, 395, 495, 595, 695], None)]
    #[case(1000, 2, 10, vec![395, 495, 595, 695, 795], Some(295))]
    // Near genesis: nothing below `ST(3)`.
    #[case(400, 2, 99, vec![95, 195, 295], None)]
    #[case(400, 4, 99, vec![295], None)]
    #[case(150, 2, 99, vec![95], None)]
    fn proven_root_heights_cases(
        #[case] height: u64,
        #[case] start: u64,
        #[case] target: u64,
        #[case] expected: Vec<u64>,
        #[case] expected_preceding: Option<u64>,
    ) {
        let lc = finalized_state(height);
        let (start, target) = (EpochNumber::new(start), EpochNumber::new(target));

        assert_eq!(
            lc.proven_root_heights(EPOCH_HEIGHT, start, target),
            expected
        );
        assert_eq!(
            lc.preceding_root_height(EPOCH_HEIGHT, EpochNumber::new(FIRST_EPOCH), start, target),
            expected_preceding
        );
    }

    #[rstest]
    // The seed epoch is at most `start + 1`.
    #[case(4, 9, 3, false)]
    #[case(3, 3, 2, false)]
    // The target is at most `start + 1`.
    #[case(10, 3, 2, false)]
    #[case(10, 2, 2, false)]
    // A target far below `E*`, at `E*+1` and above it.
    #[case(10, 4, 2, true)]
    #[case(10, 6, 2, true)]
    #[case(10, 9, 2, true)]
    #[case(10, 11, 2, true)]
    #[case(10, 15, 2, true)]
    // The held pair is the seeded genesis pair.
    #[case(6, 4, 2, true)]
    #[case(6, 5, 2, true)]
    #[case(4, 4, 2, true)]
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

    /// A seed from the genesis pair top covers the whole window; from top 8 it covers the window
    /// without its first two roots.
    #[test]
    fn seed_after_failed_batch_resumes_ok() {
        let lc = finalized_state(100);
        let (target, genesis_pair_top, pair_top) = (
            EpochNumber::new(11),
            EpochNumber::new(2),
            EpochNumber::new(8),
        );
        let window = lc.proven_root_heights(10, genesis_pair_top, target);
        assert_eq!(window, vec![45, 55, 65, 75, 85]);

        assert!(lc.seeds_target(10, target, genesis_pair_top));
        assert!(lc.seeds_target(10, target, pair_top));
        assert_eq!(
            lc.proven_root_heights(10, pair_top, target),
            window[2..].to_vec()
        );
    }

    proptest! {
        #[test]
        fn proven_roots_below_f_ok(
            height in 1u64..10_000,
            epoch_height in prop::sample::select(vec![10u64, 100]),
            start in 2u64..120,
            target in 0u64..200,
            first_epoch in 1u64..4,
        ) {
            let lc = finalized_state(height);
            let seed_epoch = *lc.seed_epoch(epoch_height);
            let (start, target) = (EpochNumber::new(start), EpochNumber::new(target));
            let top = *lc.top_installed_epoch(epoch_height, target);
            let heights = lc.proven_root_heights(epoch_height, start, target);

            prop_assert!(top <= seed_epoch + 1 && top <= *target);
            for root in &heights {
                prop_assert!(*root > 0 && *root < height);
                prop_assert!(is_epoch_root(*root, epoch_height));
            }
            let preceding =
                lc.preceding_root_height(epoch_height, EpochNumber::new(first_epoch), start, target);
            let epochs: Vec<u64> = heights
                .iter()
                .map(|root| (root + 5) / epoch_height + 2)
                .collect();
            for (epoch, root) in epochs.iter().zip(&heights) {
                prop_assert_eq!(*root, root_block_in_epoch(epoch - 2, epoch_height));
            }
            if let (Some(first), Some(last)) = (epochs.first(), epochs.last()) {
                prop_assert_eq!(*first, top.saturating_sub(4).max(*start + 1).max(3));
                prop_assert_eq!(
                    preceding,
                    (*first > first_epoch + 4).then(|| root_block_in_epoch(first - 3, epoch_height))
                );
                prop_assert!(preceding.is_none_or(|root| root < height));
                prop_assert_eq!(*last, top);
                prop_assert!(epochs.windows(2).all(|pair| pair[0] + 1 == pair[1]));
            }
            if lc.seeds_target(epoch_height, target, start) {
                prop_assert!(!heights.is_empty());
                let installed = epochs[0]..=top;
                for below in [top - 2, top - 1] {
                    prop_assert!(installed.contains(&below) || below == *start || below + 1 == *start);
                }
            }
        }
    }
}
