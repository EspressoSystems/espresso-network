//! The LC contract's voting stake table.

use hotshot_types::{
    data::EpochNumber,
    stake_table::HSStakeTable,
    utils::{epoch_from_block_number, is_epoch_root, root_block_in_epoch},
};

use crate::SeqTypes;

/// The circuit commitment of a stake table. Aliased because `StakeTableState` in this crate is the
/// L1 validator state.
pub type StakeTableCommitment = hotshot_types::light_client::StakeTableState;

/// The LC contract's voting stake table, read at one L1 block.
#[derive(Clone, Debug)]
pub struct LcContractVotingStakeTable {
    /// L1 block the getters were called at.
    pub l1_block: u64,
    /// `finalizedState.blockHeight`.
    pub finalized_block_height: u64,
    /// `E'`: epoch whose table `commitment` is.
    pub voting_epoch: EpochNumber,
    /// `votingStakeTableState`: `comm(ST(voting_epoch))`.
    pub commitment: StakeTableCommitment,
}

impl LcContractVotingStakeTable {
    /// `E'`: `epoch(height)`, plus 1 when `height` is an epoch root; `first_epoch` before the first
    /// root. Pure.
    pub fn voting_epoch(
        finalized_block_height: u64,
        epoch_height: u64,
        first_epoch: EpochNumber,
    ) -> EpochNumber {
        let epoch = epoch_from_block_number(finalized_block_height, epoch_height)
            + u64::from(is_epoch_root(finalized_block_height, epoch_height));
        first_epoch.max(EpochNumber::new(epoch))
    }

    /// Whether `table` is the table this commitment is of. Errors when `table` is over `capacity`.
    pub fn commits_to(
        &self,
        table: &HSStakeTable<SeqTypes>,
        capacity: usize,
    ) -> anyhow::Result<bool> {
        Ok(table.commitment(capacity)? == self.commitment)
    }

    /// `(epoch, root height)` for `root(E'-5) ..= root(E'-2)`, defining `ST(E'-3) ..= ST(E')`;
    /// epochs below `first + 2` omitted (seeded). Ascending. Pure.
    pub fn proven_root_heights(
        &self,
        epoch_height: u64,
        first_epoch: EpochNumber,
    ) -> Vec<(EpochNumber, u64)> {
        ((*self.voting_epoch).saturating_sub(3)..=*self.voting_epoch)
            .filter(|epoch| *epoch >= *first_epoch + 2)
            .map(|epoch| {
                (
                    EpochNumber::new(epoch),
                    root_block_in_epoch(epoch - 2, epoch_height),
                )
            })
            .collect()
    }

    /// `H*(E'-1)+1`: the first leaf of `E'`, the anchor of the header proofs. Pure.
    pub fn anchor_height(&self, epoch_height: u64) -> u64 {
        epoch_height * (*self.voting_epoch - 1) + 1
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    const EPOCH_HEIGHT: u64 = 100;
    const FIRST: u64 = 3;

    #[rstest]
    // Before the first root: the genesis table.
    #[case(0, 3)]
    #[case(50, 3)]
    #[case(299, 3)]
    // The root of the first epoch (block 295): `first + 1`, the seeded genesis pair.
    #[case(295, 4)]
    // Another root: `epoch + 1`.
    #[case(395, 5)]
    #[case(495, 6)]
    // Not a root: `epoch`.
    #[case(396, 4)]
    #[case(400, 4)]
    #[case(401, 5)]
    fn voting_epoch(#[case] height: u64, #[case] expected: u64) {
        assert_eq!(
            LcContractVotingStakeTable::voting_epoch(height, EPOCH_HEIGHT, EpochNumber::new(FIRST)),
            EpochNumber::new(expected)
        );
    }

    #[test]
    fn voting_epoch_at_root_ok() {
        let first = EpochNumber::new(1);
        let root = root_block_in_epoch(7, EPOCH_HEIGHT);
        assert_eq!(
            LcContractVotingStakeTable::voting_epoch(root, EPOCH_HEIGHT, first),
            EpochNumber::new(8)
        );
        assert_eq!(
            LcContractVotingStakeTable::voting_epoch(root + 1, EPOCH_HEIGHT, first),
            EpochNumber::new(7)
        );
    }

    #[rstest]
    // Far from genesis: `root(E'-5) ..= root(E'-2)`.
    #[case(10, 3, vec![(7, 495), (8, 595), (9, 695), (10, 795)])]
    // Near genesis: the seeded epochs `first`, `first + 1` are omitted.
    #[case(6, 3, vec![(5, 295), (6, 395)])]
    #[case(4, 3, vec![])]
    fn proven_root_heights_near_genesis_ok(
        #[case] voting_epoch: u64,
        #[case] first: u64,
        #[case] expected: Vec<(u64, u64)>,
    ) {
        let lc = LcContractVotingStakeTable {
            l1_block: 0,
            finalized_block_height: 0,
            voting_epoch: EpochNumber::new(voting_epoch),
            commitment: StakeTableCommitment::default(),
        };
        let expected: Vec<_> = expected
            .into_iter()
            .map(|(epoch, height)| (EpochNumber::new(epoch), height))
            .collect();
        assert_eq!(
            lc.proven_root_heights(EPOCH_HEIGHT, EpochNumber::new(first)),
            expected
        );
    }

    #[test]
    fn anchor_height_ok() {
        let lc = LcContractVotingStakeTable {
            l1_block: 0,
            finalized_block_height: 0,
            voting_epoch: EpochNumber::new(7),
            commitment: StakeTableCommitment::default(),
        };
        assert_eq!(lc.anchor_height(EPOCH_HEIGHT), 601);
    }
}
