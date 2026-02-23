//! A null implementation of `EspressoClient` for L1-only mode.

use std::future::pending;

use bitvec::vec::BitVec;
use espresso_types::{AuthenticatedValidatorMap, Leaf2};
use futures::{Stream, stream};

use crate::{
    Error, Result,
    input::espresso::EspressoClient,
    types::common::{Address, ESPTokenAmount, Ratio},
};

/// A null Espresso client that blocks indefinitely and returns "not initialized" errors.
///
/// This is used when running in L1-only mode, where the Espresso data source is disabled.
#[derive(Clone, Debug, Default)]
pub struct NullEspressoClient;

impl EspressoClient for NullEspressoClient {
    async fn wait_for_epochs(&self) -> u64 {
        // Block forever, waiting for epochs that will never come in L1-only mode.
        pending().await
    }

    async fn epoch_height(&self) -> Result<u64> {
        // Return a placeholder value. This is needed by `espresso::State::new` but won't be
        // meaningfully used in L1-only mode.
        Ok(0)
    }

    async fn stake_table_for_epoch(&self, _epoch: u64) -> Result<AuthenticatedValidatorMap> {
        Err(Error::not_initialized()
            .context("Espresso data source is disabled (running in L1-only mode)"))
    }

    async fn apr_for_epoch(&self, _epoch: u64, _total_staked: ESPTokenAmount) -> Result<Ratio> {
        Err(Error::not_initialized()
            .context("Espresso data source is disabled (running in L1-only mode)"))
    }

    async fn leaf(&self, _height: u64) -> Result<Leaf2> {
        Err(Error::not_initialized()
            .context("Espresso data source is disabled (running in L1-only mode)"))
    }

    async fn block_reward(&self, _epoch: u64) -> Result<ESPTokenAmount> {
        Err(Error::not_initialized()
            .context("Espresso data source is disabled (running in L1-only mode)"))
    }

    async fn fetch_all_reward_accounts(
        &self,
        _block: u64,
    ) -> Result<Vec<(Address, ESPTokenAmount)>> {
        Err(Error::not_initialized()
            .context("Espresso data source is disabled (running in L1-only mode)"))
    }

    fn leaves(&self, _from: u64) -> impl Send + Unpin + Stream<Item = (Leaf2, BitVec)> {
        // Return an empty stream that never yields any items.
        stream::empty()
    }
}
