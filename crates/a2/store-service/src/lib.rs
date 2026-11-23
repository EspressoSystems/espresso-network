//! A storage instance: serves the chain store over gRPC, and takes its share of the decide log
//! from the deployment's consensus node into the store.

pub mod ingest;

use std::{sync::Arc, time::Duration};

use chain_store::Backend;
use tokio::time::sleep;

/// How often the store prunes.
const PRUNE_INTERVAL: Duration = Duration::from_secs(60);

/// The fewest heights a store may keep. The API services over it resume from snapshots of the
/// chain's state some heights back, and consensus nodes catch up from its recent heights; both
/// need the heights to still be there.
pub const MIN_KEEP_HEIGHTS: u64 = 1024;

/// Keeps the newest `keep` heights, pruning the rest every [`PRUNE_INTERVAL`].
pub async fn prune(backend: Arc<dyn Backend>, keep: u64) {
    loop {
        match backend.newest().await {
            Ok(Some(newest)) => {
                let below = (newest + 1).saturating_sub(keep);
                if let Err(err) = backend.prune_below(below).await {
                    tracing::warn!(below, "pruning failed: {err:#}");
                }
            },
            Ok(None) => {},
            Err(err) => tracing::warn!("reading the newest height failed: {err:#}"),
        }
        sleep(PRUNE_INTERVAL).await;
    }
}
