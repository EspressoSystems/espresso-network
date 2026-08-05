//! Leader lookup for arbitrary views.
//!
//! The leader of a view is a pure function of the view, the epoch's stake table and the epoch's DRB
//! result, so any past view can be answered without per-view records. The epoch is not derivable
//! from the view alone (epochs are counted in block heights), so it is resolved from the decided
//! chain: the epoch of the first leaf decided at or after the requested view. A view that produced
//! no block did not advance the block height, so that next leaf carries the height, and therefore
//! the epoch, the view would have had.
//!
//! Resolution bisects epochs, not leaves. An epoch's first block sits at a height fixed by
//! arithmetic, so `log2(block height / epoch_height)` reads suffice and they always land on the same
//! small set of rows, one per epoch, which the database keeps cached across requests. Bisecting
//! leaves instead costs `log2(block height)` reads at unpredictable heights, cold every time.

use std::{future::Future, time::Duration};

use anyhow::{Context, ensure};
use espresso_api::error::AvailabilityError;
use espresso_types::{PubKey, SeqTypes};
use hotshot_query_service::{
    availability::{AvailabilityDataSource, LeafId, LeafQueryData},
    node::NodeDataSource,
};
use hotshot_types::data::{EpochNumber, ViewNumber};
use serde::{Deserialize, Serialize};

use super::data_source::{PruningDataSource, StakeTableDataSource};

/// Maximum number of views one range request may cover.
pub const LEADER_RANGE_LIMIT: u64 = 10_000;

/// The leader of a single view.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewLeader {
    pub view: ViewNumber,
    /// `None` for views from before epochs were enabled.
    pub epoch: Option<EpochNumber>,
    pub leader: PubKey,
}

/// Data source for leader lookup: leaves to resolve the epoch of a view, the stake table to elect
/// from it, and the retained height window to keep reads off pruned heights.
pub(crate) trait LeaderDataSource:
    AvailabilityDataSource<SeqTypes>
    + NodeDataSource<SeqTypes>
    + StakeTableDataSource<SeqTypes>
    + PruningDataSource
    + Sync
{
}

impl<T> LeaderDataSource for T where
    T: AvailabilityDataSource<SeqTypes>
        + NodeDataSource<SeqTypes>
        + StakeTableDataSource<SeqTypes>
        + PruningDataSource
        + Sync
{
}

/// Leaders of the views in `from..=until`.
///
/// Ranges crossing an epoch boundary are truncated at the boundary: each entry carries its own view,
/// so the caller resumes from the last returned view plus one.
pub(crate) async fn leaders<DS: LeaderDataSource>(
    ds: &DS,
    from: ViewNumber,
    until: ViewNumber,
    timeout: Duration,
) -> anyhow::Result<Vec<ViewLeader>> {
    let (epoch, until) = resolve_epoch(ds, from, until, timeout).await?;

    let leaders = ds.leaders(from..=until, epoch).await?;
    Ok((from.u64()..=until.u64())
        .zip(leaders)
        .map(|(view, leader)| ViewLeader {
            view: ViewNumber::new(view),
            epoch,
            leader,
        })
        .collect())
}

/// The epoch of `from`, and the last view of `from..=until` that belongs to it.
async fn resolve_epoch<DS: LeaderDataSource>(
    ds: &DS,
    from: ViewNumber,
    until: ViewNumber,
    timeout: Duration,
) -> anyhow::Result<(Option<EpochNumber>, ViewNumber)> {
    let block_height = NodeDataSource::block_height(ds)
        .await
        .context("failed to get block height")? as u64;
    ensure!(block_height > 0, "no blocks have been decided");
    let tip = block_height - 1;
    let epoch_height = ds.epoch_height().await;

    // Pruning deletes a contiguous prefix of heights, so the retained leaves are `floor..=tip` and
    // no read may go below `floor`.
    let oldest = match ds.get_oldest_leaf().await? {
        Some(oldest) => oldest,
        None => leaf_at(ds, 0, timeout).await?,
    };
    let floor = oldest.leaf().height();
    if from < oldest.leaf().view_number() {
        return Err(AvailabilityError::NotFound(format!(
            "view {from} predates the oldest retained leaf (height {floor}, view {})",
            oldest.leaf().view_number()
        ))
        .into());
    }

    let newest = leaf_at(ds, tip, timeout).await?;
    if from > newest.leaf().view_number() {
        // Nothing in the requested range is decided yet, so the views are elected from the epoch the
        // node is currently in.
        return Ok((ds.current_epoch().await, until));
    }

    let window = Window {
        floor,
        tip,
        epoch_height,
        timeout,
    };
    match (
        oldest.leaf().epoch(epoch_height),
        newest.leaf().epoch(epoch_height),
    ) {
        (None, None) => Ok((None, until)),
        (Some(floor_epoch), Some(tip_epoch)) => {
            let epoch = window
                .epoch_of_view(ds, from, floor_epoch, tip_epoch)
                .await?;
            let until = window.truncate(ds, until, epoch, tip_epoch).await?;
            Ok((Some(epoch), until))
        },
        // The retained window straddles the epoch upgrade, so which side `from` falls on is not
        // arithmetic and has to be read off the leaf itself.
        (None, Some(tip_epoch)) => {
            let height = window
                .first_leaf_at_or_after(ds, from)
                .await?
                .context("leaf at or after the requested view is retained")?;
            let epoch = leaf_at(ds, height, timeout)
                .await?
                .leaf()
                .epoch(epoch_height);
            let Some(epoch) = epoch else {
                let until_height = window.first_leaf_at_or_after(ds, until).await?;
                let until_epoch = match until_height {
                    Some(height) => leaf_at(ds, height, timeout)
                        .await?
                        .leaf()
                        .epoch(epoch_height),
                    None => ds.current_epoch().await,
                };
                ensure!(
                    until_epoch.is_none(),
                    "view range {from}..={until} crosses the epoch upgrade; request the \
                     pre-upgrade and post-upgrade views separately"
                );
                return Ok((None, until));
            };
            let until = window.truncate(ds, until, epoch, tip_epoch).await?;
            Ok((Some(epoch), until))
        },
        // `with_epoch` never goes back to false once an upgrade enables epochs.
        (Some(floor_epoch), None) => Err(anyhow::anyhow!(
            "leaf {floor} is in epoch {floor_epoch} but leaf {tip} has no epoch"
        )),
    }
}

/// The retained height window that reads are confined to.
struct Window {
    floor: u64,
    tip: u64,
    epoch_height: u64,
    timeout: Duration,
}

impl Window {
    /// The view of the first retained leaf of `epoch`.
    ///
    /// Non-decreasing in `epoch`, which is what makes it bisectable.
    async fn epoch_first_view<DS: LeaderDataSource>(
        &self,
        ds: &DS,
        epoch: u64,
    ) -> anyhow::Result<ViewNumber> {
        // Epoch `e` covers heights `(e - 1) * epoch_height + 1 ..= e * epoch_height`.
        let height = ((epoch - 1) * self.epoch_height + 1).max(self.floor);
        leaf_view(ds, height, self.timeout).await
    }

    /// The epoch of the first leaf decided at `view` or later.
    async fn epoch_of_view<DS: LeaderDataSource>(
        &self,
        ds: &DS,
        view: ViewNumber,
        floor_epoch: EpochNumber,
        tip_epoch: EpochNumber,
    ) -> anyhow::Result<EpochNumber> {
        let first_at_or_after =
            first_key_at_or_after(view, floor_epoch.u64(), tip_epoch.u64(), |e| {
                self.epoch_first_view(ds, e)
            })
            .await?;

        let Some(first_at_or_after) = first_at_or_after else {
            return Ok(tip_epoch);
        };
        if first_at_or_after == floor_epoch.u64() {
            return Ok(floor_epoch);
        }
        // The preceding epoch may still be the answer: its own first view is below `view`, but its
        // last leaf need not be.
        let previous = first_at_or_after - 1;
        let last_height = (previous * self.epoch_height).min(self.tip);
        let epoch = if leaf_view(ds, last_height, self.timeout).await? >= view {
            previous
        } else {
            first_at_or_after
        };
        Ok(EpochNumber::new(epoch))
    }

    /// `until`, clamped to the last view of `epoch`.
    async fn truncate<DS: LeaderDataSource>(
        &self,
        ds: &DS,
        until: ViewNumber,
        epoch: EpochNumber,
        tip_epoch: EpochNumber,
    ) -> anyhow::Result<ViewNumber> {
        if epoch >= tip_epoch {
            return Ok(until);
        }
        let next = self.epoch_first_view(ds, epoch.u64() + 1).await?;
        Ok(until.min(next - 1))
    }

    /// The height of the first leaf decided at `view` or later, by bisecting leaves.
    ///
    /// Only for the pre-epoch era, where epoch arithmetic does not apply.
    async fn first_leaf_at_or_after<DS: LeaderDataSource>(
        &self,
        ds: &DS,
        view: ViewNumber,
    ) -> anyhow::Result<Option<u64>> {
        first_key_at_or_after(view, self.floor, self.tip, |h| {
            leaf_view(ds, h, self.timeout)
        })
        .await
    }
}

async fn leaf_at<DS: AvailabilityDataSource<SeqTypes> + Sync>(
    ds: &DS,
    height: u64,
    timeout: Duration,
) -> anyhow::Result<LeafQueryData<SeqTypes>> {
    ds.get_leaf(LeafId::Number(height as usize))
        .await
        .with_timeout(timeout)
        .await
        .with_context(|| format!("leaf {height} unavailable"))
}

async fn leaf_view<DS: AvailabilityDataSource<SeqTypes> + Sync>(
    ds: &DS,
    height: u64,
    timeout: Duration,
) -> anyhow::Result<ViewNumber> {
    Ok(leaf_at(ds, height, timeout).await?.leaf().view_number())
}

/// Smallest key in `lo..=hi` whose view is `view` or later, or `None` if every key up to `hi` is
/// older than `view`.
///
/// `view_at` must be non-decreasing in the key. Used over epochs, and over heights before epochs
/// were enabled.
async fn first_key_at_or_after<F, Fut>(
    view: ViewNumber,
    lo: u64,
    hi: u64,
    view_at: F,
) -> anyhow::Result<Option<u64>>
where
    F: Fn(u64) -> Fut,
    Fut: Future<Output = anyhow::Result<ViewNumber>>,
{
    if view_at(hi).await? < view {
        return Ok(None);
    }
    let (mut lo, mut hi) = (lo, hi);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if view_at(mid).await? < view {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    Ok(Some(lo))
}

#[cfg(test)]
mod test {
    use super::*;

    /// Exhaustive check of the bisect against a linear scan, over every requested view and every
    /// window of a key space with gaps (views that produced no block) throughout.
    #[tokio::test]
    async fn first_key_at_or_after_matches_linear_scan() {
        let views = [2u64, 3, 6, 7, 11];
        let hi = views.len() as u64 - 1;

        for lo in 0..=hi {
            // A read below `lo` is a read of a pruned height, which the bisect must never do.
            let view_at = |key: u64| async move {
                assert!(key >= lo, "read key {key} below the window start {lo}");
                Ok(ViewNumber::new(views[key as usize]))
            };

            for view in 0..=13 {
                let expected = (views[hi as usize] >= view).then(|| {
                    (lo..=hi)
                        .find(|key| views[*key as usize] >= view)
                        .expect("non-decreasing views, so hi qualifies")
                });
                let found = first_key_at_or_after(ViewNumber::new(view), lo, hi, view_at)
                    .await
                    .unwrap();
                assert_eq!(found, expected, "view {view}, window start {lo}");
            }
        }
    }

    #[tokio::test]
    async fn first_key_at_or_after_single_key() {
        let view_at = |_| async { Ok(ViewNumber::new(5)) };
        for (view, expected) in [(0, Some(0)), (5, Some(0)), (6, None)] {
            let found = first_key_at_or_after(ViewNumber::new(view), 0, 0, view_at)
                .await
                .unwrap();
            assert_eq!(found, expected, "view {view}");
        }
    }
}
