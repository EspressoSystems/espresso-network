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

    /// Heights `root(e-2)` defining `ST(e)` for `e` in `E*-4 ..= E*+1`, above the held top `start`,
    /// ascending. The reward of `ST(e)` needs `ST(e-3)`, so the nodes at `E*` find the rewards of
    /// `E*-1 ..= E*+1`. Every height is positive and below `F`. Pure.
    pub fn proven_root_heights(&self, epoch_height: u64, start: EpochNumber) -> Vec<u64> {
        let seed_epoch = *self.seed_epoch(epoch_height);
        // `ST(3)` is the first table defined by an epoch root.
        let first = seed_epoch.saturating_sub(4).max(*start + 1).max(3);
        (first..=seed_epoch + 1)
            .map(|epoch| root_block_in_epoch(epoch - 2, epoch_height))
            .collect()
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
    use hotshot_types::utils::is_epoch_root;
    use proptest::prelude::*;
    use rstest::rstest;

    use super::*;

    const EPOCH_HEIGHT: u64 = 100;

    fn finalized_state(height: u64) -> LcContractFinalizedState {
        LcContractFinalizedState {
            height,
            block_comm_root: CircuitField::default(),
        }
    }

    #[rstest]
    #[case(0, 1)]
    #[case(50, 1)]
    // The root of epoch 3 (block 295) and the blocks after it up to the epoch end.
    #[case(295, 3)]
    #[case(296, 3)]
    #[case(300, 3)]
    #[case(301, 4)]
    #[case(395, 4)]
    fn seed_epoch_cases(#[case] height: u64, #[case] expected: u64) {
        assert_eq!(
            finalized_state(height).seed_epoch(EPOCH_HEIGHT),
            EpochNumber::new(expected)
        );
    }

    #[rstest]
    // Far from genesis: `root(E*-6) ..= root(E*-1)`, defining `ST(E*-4) ..= ST(E*+1)`.
    #[case(1000, 1, vec![395, 495, 595, 695, 795, 895])]
    #[case(1000, 5, vec![395, 495, 595, 695, 795, 895])]
    // Epochs up to `start` are held.
    #[case(1000, 7, vec![595, 695, 795, 895])]
    #[case(1000, 9, vec![795, 895])]
    #[case(1000, 11, vec![])]
    // Near genesis: nothing below `ST(3)`.
    #[case(400, 2, vec![95, 195, 295])]
    #[case(400, 4, vec![295])]
    #[case(150, 2, vec![95])]
    fn proven_root_heights_cases(
        #[case] height: u64,
        #[case] start: u64,
        #[case] expected: Vec<u64>,
    ) {
        assert_eq!(
            finalized_state(height).proven_root_heights(EPOCH_HEIGHT, EpochNumber::new(start)),
            expected
        );
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
            let epochs: Vec<u64> = heights
                .iter()
                .map(|root| (root + 5) / epoch_height + 2)
                .collect();
            for (epoch, root) in epochs.iter().zip(&heights) {
                prop_assert_eq!(*root, root_block_in_epoch(epoch - 2, epoch_height));
            }
            if let (Some(first), Some(last)) = (epochs.first(), epochs.last()) {
                prop_assert_eq!(*first, seed_epoch.saturating_sub(4).max(start + 1).max(3));
                prop_assert_eq!(*last, seed_epoch + 1);
                prop_assert!(epochs.windows(2).all(|pair| pair[0] + 1 == pair[1]));
            }
        }
    }
}
