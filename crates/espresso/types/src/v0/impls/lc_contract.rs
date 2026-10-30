//! The LC contract's voting stake table.

use hotshot_types::{
    data::EpochNumber,
    light_client::StakeTableState as StakeTableCommitment,
    utils::{epoch_from_block_number, is_epoch_root, root_block_in_epoch},
};

/// The LC contract's voting stake table, read at one L1 block.
#[derive(Clone, Debug)]
pub struct LcContractVotingStakeTable {
    /// L1 block the getters were called at.
    pub l1_block: u64,
    /// `E'`: epoch whose table `commitment` is.
    pub voting_epoch: EpochNumber,
    /// `votingStakeTableState`: `comm(ST(voting_epoch))`.
    pub commitment: StakeTableCommitment,
}

impl LcContractVotingStakeTable {
    /// `E'`: `epoch(height)`, plus 1 when `height` is an epoch root.
    pub fn voting_epoch(finalized_block_height: u64, epoch_height: u64) -> EpochNumber {
        EpochNumber::new(
            epoch_from_block_number(finalized_block_height, epoch_height)
                + u64::from(is_epoch_root(finalized_block_height, epoch_height)),
        )
    }

    /// `(epoch, root height)` for `root(E'-5) ..= root(E'-2)`, defining `ST(E'-3) ..= ST(E')`,
    /// ascending; epochs up to `held_floor + 1` are held and omitted.
    pub fn proven_root_heights(
        &self,
        epoch_height: u64,
        held_floor: EpochNumber,
    ) -> Vec<(EpochNumber, u64)> {
        ((*self.voting_epoch).saturating_sub(3)..=*self.voting_epoch)
            .filter(|epoch| *epoch >= *held_floor + 2)
            .map(|epoch| {
                (
                    EpochNumber::new(epoch),
                    root_block_in_epoch(epoch - 2, epoch_height),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    const EPOCH_HEIGHT: u64 = 100;

    #[rstest]
    #[case(0, 1)]
    #[case(50, 1)]
    #[case(299, 3)]
    // The root of epoch 3 (block 295): `epoch + 1`.
    #[case(295, 4)]
    #[case(395, 5)]
    #[case(495, 6)]
    // Not a root: `epoch`.
    #[case(396, 4)]
    #[case(400, 4)]
    #[case(401, 5)]
    fn voting_epoch(#[case] height: u64, #[case] expected: u64) {
        assert_eq!(
            LcContractVotingStakeTable::voting_epoch(height, EPOCH_HEIGHT),
            EpochNumber::new(expected)
        );
    }

    #[rstest]
    // Far from genesis: `root(E'-5) ..= root(E'-2)`.
    #[case(10, 1, vec![(7, 495), (8, 595), (9, 695), (10, 795)])]
    // Epochs up to `held_floor + 1` are held.
    #[case(6, 3, vec![(5, 295), (6, 395)])]
    #[case(10, 7, vec![(9, 695), (10, 795)])]
    #[case(4, 3, vec![])]
    fn proven_root_heights_skip_held_epochs(
        #[case] voting_epoch: u64,
        #[case] held_floor: u64,
        #[case] expected: Vec<(u64, u64)>,
    ) {
        let lc = LcContractVotingStakeTable {
            l1_block: 0,
            voting_epoch: EpochNumber::new(voting_epoch),
            commitment: StakeTableCommitment::default(),
        };
        let expected: Vec<_> = expected
            .into_iter()
            .map(|(epoch, height)| (EpochNumber::new(epoch), height))
            .collect();
        assert_eq!(
            lc.proven_root_heights(EPOCH_HEIGHT, EpochNumber::new(held_floor)),
            expected
        );
    }
}
