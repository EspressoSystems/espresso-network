//! SQL-based persistent storage.

use crate::{
    Result,
    input::l1::{L1BlockSnapshot, L1Persistence, Snapshot},
    types::{common::Address, global::FullNodeSetDiff, wallet::WalletDiff},
};
use std::path::Path;

#[derive(Debug)]
pub struct Persistence;

impl Persistence {
    /// Create a new SQLite database at the given file location.
    pub async fn new(_file: &Path) -> Result<Self> {
        Ok(Self)
    }
}

impl L1Persistence for Persistence {
    async fn finalized_snapshot(&self) -> Result<Option<Snapshot>> {
        Ok(None)
    }

    async fn save_genesis(&self, _snapshot: Snapshot) -> Result<()> {
        Ok(())
    }

    async fn apply_events(
        &self,
        _block: L1BlockSnapshot,
        _node_set_diff: impl IntoIterator<Item = FullNodeSetDiff> + Send,
        _wallets_diff: impl IntoIterator<Item = (Address, WalletDiff)> + Send,
    ) -> Result<()> {
        Ok(())
    }
}
