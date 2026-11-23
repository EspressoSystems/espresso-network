//! The node's consensus store, on redb.

use std::path::PathBuf;

use anyhow::Context;
use consensus::store::{ConsensusStore, Import, Restored};
use consensus_store_redb::RedbStore;
use espresso_types::SeqTypes;

/// The consensus store of a node.
pub type NodeStore = ConsensusStore<SeqTypes, RedbStore>;

/// Opens the node's store in `dir`, creating it if needed, and returns it with what a restarted
/// node resumes from.
pub async fn open(dir: PathBuf) -> anyhow::Result<(NodeStore, Restored<SeqTypes>)> {
    let path = dir.display().to_string();
    let store = tokio::task::spawn_blocking(move || RedbStore::open(&dir))
        .await?
        .with_context(|| format!("opening the consensus store in {path}"))?;
    ConsensusStore::open(store, async || Ok(nothing_to_import())).await
}

/// What a node takes over into a new store: nothing, since an a2 node keeps all its state there
/// from the start.
fn nothing_to_import() -> Import<SeqTypes> {
    Import {
        proposals: Vec::new(),
        locked_qc: None,
        boundary_qc: None,
        acted_view: None,
        restart_view: None,
        upgrade_cert: None,
        state_cert: None,
        anchor: None,
        app: Vec::new(),
    }
}
