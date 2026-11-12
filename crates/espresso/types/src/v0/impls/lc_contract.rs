//! The LC contract's finalized state.

#[cfg(feature = "node")]
use alloy::{eips::BlockId, primitives::Address};
#[cfg(feature = "node")]
use anyhow::Context;
use anyhow::ensure;
#[cfg(feature = "node")]
use hotshot_contract_adapter::sol_types::{LightClientStateSol, LightClientV3, StakeTableV3};
#[cfg(feature = "node")]
use hotshot_types::light_client::LightClientState;
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    light_client::CircuitField,
    traits::block_contents::BlockHeader,
    utils::{epoch_from_block_number, root_block_in_epoch},
};
#[cfg(feature = "node")]
use moka::future::Cache;

#[cfg(feature = "node")]
use super::instance_state;
use crate::Header;
#[cfg(feature = "node")]
use crate::v0::L1Client;

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

/// Reads the LC contract over L1. Holds no coordinator, so a long-lived holder does not own one.
#[cfg(feature = "node")]
#[derive(Clone)]
pub struct LcContractReader {
    l1_client: L1Client,
    stake_table_contract: Address,
    light_client_contract_address: Cache<(), Address>,
}

#[cfg(feature = "node")]
impl LcContractReader {
    /// A reader over `l1_client`, with the stake table contract that points to the LC contract and a
    /// cache of the LC contract address.
    pub(super) fn new(
        l1_client: L1Client,
        stake_table_contract: Address,
        light_client_contract_address: Cache<(), Address>,
    ) -> Self {
        Self {
            l1_client,
            stake_table_contract,
            light_client_contract_address,
        }
    }

    /// The LC contract's address: cached, else read from the stake table contract.
    pub async fn address(&self) -> anyhow::Result<Address> {
        if let Some(address) = self.light_client_contract_address.get(&()).await {
            return Ok(address);
        }
        let stake_table =
            StakeTableV3::new(self.stake_table_contract, self.l1_client.provider.clone());
        let address = stake_table.lightClient().call().await?;
        self.light_client_contract_address.insert((), address).await;
        Ok(address)
    }

    /// The LC contract's `finalizedState` at the L1 finalized block. Waiting for the first block
    /// and the contract reads are each bounded by `L1_FINALIZED_WAIT`.
    pub async fn finalized_state(&self) -> anyhow::Result<LcContractFinalizedState> {
        let block = tokio::time::timeout(
            instance_state::L1_FINALIZED_WAIT,
            self.l1_client.wait_for_latest_finalized_at_least(0),
        )
        .await
        .context("no L1 finalized block yet")?
        .number;
        let state = tokio::time::timeout(instance_state::L1_FINALIZED_WAIT, async {
            LightClientV3::new(self.address().await?, self.l1_client.provider.clone())
                .finalizedState()
                .block(BlockId::number(block))
                .call()
                .await
                .context("failed to read finalizedState")
        })
        .await
        .context("reading the LC contract exceeded the L1 wait")??;
        let state = LightClientState::from(LightClientStateSol::from(state));
        Ok(LcContractFinalizedState {
            height: state.block_height,
            block_comm_root: state.block_comm_root,
        })
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
