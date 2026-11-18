//! The certificates the chain does not commit to: the `Certificate2` that decided a block directly,
//! and the state certificate of an epoch root.
//!
//! Each is checked against the stake table of its block's epoch, so they are filled an epoch at a
//! time, once membership has that stake table and the store has every block of the epoch. Heights
//! the ingester wrote have their certificates from consensus already; the others are asked of the
//! peers. A height a peer has but sends no `Certificate2` for was decided by a later block, and
//! keeps none. The store then records the epoch as checked, and it is not asked again.

use std::{collections::BTreeSet, ops::Range};

use chain_store::{Item, Part, Parts};
use hotshot_types::data::EpochNumber;
use url::Url;

use crate::{GapFiller, Membership, gaps::runs};

/// How many epoch roots one pass derives stake tables from.
const ROOTS_PER_PASS: usize = 8;

/// How many epochs one pass checks the certificates of.
const EPOCHS_PER_PASS: usize = 4;

impl GapFiller {
    /// Derives the stake tables that the stored epoch roots give and membership lacks, newest
    /// first.
    pub(crate) async fn follow_roots(&self, membership: &Membership) -> anyhow::Result<()> {
        let Some(newest) = self.store.newest().await? else {
            return Ok(());
        };
        let first = membership.first_epoch();
        let mut installed = 0;
        let mut epoch = membership.epoch_of(newest);
        while epoch >= first && installed < ROOTS_PER_PASS {
            if membership.stake_table(epoch + 2).is_none()
                && let Some(root) = self.store.item(membership.root_of(epoch)).await?
            {
                let preceding = match epoch > first {
                    true => self.store.item(membership.root_of(epoch - 1)).await?,
                    false => None,
                };
                membership
                    .install_root(
                        root.leaf.block_header().clone(),
                        preceding.map(|item| item.leaf.block_header().clone()),
                    )
                    .await?;
                installed += 1;
            }
            if *epoch == 0 {
                break;
            }
            epoch = epoch - 1;
        }
        Ok(())
    }

    /// Checks the certificates of the newest complete epochs that are not checked yet.
    pub(crate) async fn check_certs(
        &self,
        membership: &Membership,
        peers: &[Url],
    ) -> anyhow::Result<()> {
        let Some(newest) = self.store.newest().await? else {
            return Ok(());
        };
        let first = membership.first_epoch();
        let mut checked = 0;
        // The newest epoch is still being decided.
        let mut epoch = membership.epoch_of(newest);
        while epoch > first && checked < EPOCHS_PER_PASS {
            epoch = epoch - 1;
            if self.certs_checked(epoch).await?
                || membership.stake_table(epoch).is_none()
                || !self.has_all(membership.heights_of(epoch)).await?
            {
                continue;
            }
            self.check_epoch(membership, epoch, peers).await?;
            checked += 1;
        }
        Ok(())
    }

    async fn certs_checked(&self, epoch: EpochNumber) -> anyhow::Result<bool> {
        if self.checked_epochs.lock().unwrap().contains(&epoch) {
            return Ok(true);
        }
        let checked = self.store.certs_checked(epoch).await?;
        if checked {
            self.checked_epochs.lock().unwrap().insert(epoch);
        }
        Ok(checked)
    }

    async fn has_all(&self, heights: Range<u64>) -> anyhow::Result<bool> {
        let missing = self.store.missing_items(heights.start, 1).await?;
        Ok(missing.first().is_none_or(|gap| gap.start >= heights.end))
    }

    /// Fills the certificates `epoch` lacks from the peers, and records the epoch as checked once
    /// a peer has answered for each height.
    async fn check_epoch(
        &self,
        membership: &Membership,
        epoch: EpochNumber,
        peers: &[Url],
    ) -> anyhow::Result<()> {
        let heights = membership.heights_of(epoch);
        let mut wanted: BTreeSet<u64> = self
            .store
            .missing_parts(Part::Cert2, heights.start, count(&heights))
            .await?
            .into_iter()
            .filter(|height| heights.contains(height))
            .collect();
        let root = membership.root_of(epoch);
        if self.store.state_cert(root).await?.is_none() {
            wanted.insert(root);
        }
        for run in runs(&wanted) {
            let mut answered = false;
            for peer in peers {
                match self.take_certs(membership, peer, run.clone()).await {
                    Ok(()) => {
                        answered = true;
                        break;
                    },
                    Err(err) => tracing::debug!(%peer, ?run, "{err:#}"),
                }
            }
            if !answered {
                anyhow::bail!("no peer has the certificates of heights {run:?}");
            }
        }
        self.store.mark_certs_checked(epoch).await?;
        self.checked_epochs.lock().unwrap().insert(epoch);
        tracing::info!(%epoch, "certificates checked");
        Ok(())
    }

    /// Stores the certificates `peer` sends for `heights` that check against the stored leaves.
    /// Fails unless the peer has every height, since a height it lacks says nothing about its
    /// certificates.
    async fn take_certs(
        &self,
        membership: &Membership,
        peer: &Url,
        heights: Range<u64>,
    ) -> anyhow::Result<()> {
        let records = crate::gaps::fetch(peer, heights.clone(), true).await?;
        anyhow::ensure!(
            records.len() == count(&heights),
            "{peer} has {} of {} heights",
            records.len(),
            count(&heights)
        );
        for record in records {
            let Some(item) = self.store.item(record.height).await? else {
                continue;
            };
            let parts = record.parts()?;
            let checked = checked_certs(membership, &item, parts).await?;
            self.store.put_parts(record.height, &checked).await?;
        }
        Ok(())
    }
}

fn count(heights: &Range<u64>) -> usize {
    (heights.end - heights.start) as usize
}

/// The certificates of `parts` that check against `item`, the stored item of their height. A
/// certificate that fails is an error, since it marks the peer that sent it as faulty.
async fn checked_certs(
    membership: &Membership,
    item: &Item,
    parts: Parts,
) -> anyhow::Result<Parts> {
    if let Some(cert2) = &parts.cert2 {
        membership.check_cert2(cert2, &item.leaf).await?;
    }
    if let Some(cert) = &parts.state_cert {
        membership.check_state_cert(cert, item.leaf.block_header())?;
    }
    Ok(Parts {
        cert2: parts.cert2,
        state_cert: parts.state_cert,
        ..Default::default()
    })
}
