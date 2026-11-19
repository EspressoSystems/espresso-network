//! The certificates the chain does not commit to: the `Certificate2` that decided a block directly,
//! and the state certificate of an epoch root.
//!
//! Each is checked against the stake table of its block's epoch, so they are filled an epoch at a
//! time, once membership has that stake table and the store has every block of the epoch. Heights
//! the ingester wrote have their certificates from consensus already; the others are asked of the
//! peers. A height a peer has but sends no `Certificate2` for was decided by a later block, and
//! keeps none. The store then records the epoch as checked, and it is not asked again.
//!
//! Both walks go from the newest epoch back to the fill front, the end of the newest gap in the
//! store: every height from there on is stored, and gaps are filled from the newest back, so the
//! epochs before the front become workable as the front moves back. Stopping there keeps a pass
//! from asking the store about every epoch of a history that is not filled yet.

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
        let Some((front, newest)) = self.filled().await? else {
            return Ok(());
        };
        let first = membership.first_epoch();
        let mut installed = 0;
        let mut epoch = membership.epoch_of(newest);
        while epoch >= first && membership.root_of(epoch) >= front && installed < ROOTS_PER_PASS {
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
        let Some((front, newest)) = self.filled().await? else {
            return Ok(());
        };
        let first = membership.first_epoch();
        let mut checked = 0;
        // The newest epoch is still being decided.
        let mut epoch = membership.epoch_of(newest);
        while epoch > first && checked < EPOCHS_PER_PASS {
            epoch = epoch - 1;
            if membership.heights_of(epoch).start < front {
                break;
            }
            if self.certs_checked(epoch).await? || membership.stake_table(epoch).is_none() {
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

    /// The fill front and the newest height, if the store has any: every height from the front
    /// up to the newest is stored.
    async fn filled(&self) -> anyhow::Result<Option<(u64, u64)>> {
        let Some(newest) = self.store.newest().await? else {
            return Ok(None);
        };
        let front = match self.store.newest_gap().await? {
            Some(gap) => gap.end,
            None => self.store.floor().await?,
        };
        Ok(Some((front, newest)))
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chain_store::{ChainStore, mem::MemBackend};
    use hotshot_types::simple_certificate::QuorumCertificate2;
    use tokio::net::TcpListener;

    use super::*;
    use crate::{ApiService, membership::testing::*};

    const EPOCH: u64 = FIRST_EPOCH + 1;

    /// A store with every height of `EPOCH` and the first of the next, so `EPOCH` is complete and
    /// not the newest.
    async fn store(membership: &Membership) -> (ChainStore, Vec<Item>) {
        let store = ChainStore::new(MemBackend::default());
        let heights = membership.heights_of(EpochNumber::new(EPOCH));
        let mut items = Vec::new();
        for height in heights.start..=heights.end {
            let leaf = leaf(height).await;
            let mut cert1 = QuorumCertificate2::<espresso_types::SeqTypes>::genesis(
                &Default::default(),
                &espresso_types::NodeState::mock(),
                versions::Upgrade::trivial(versions::NEW_PROTOCOL_VERSION),
            )
            .await;
            cert1.data.leaf_commit = committable::Committable::commit(&leaf);
            let item = Item { leaf, cert1 };
            store.put(&item, &Parts::default()).await.unwrap();
            items.push(item);
        }
        (store, items)
    }

    /// A peer that has `items` and, for the height of `decided`, a cert2 signed by `signers`, and
    /// for the epoch root a state certificate.
    async fn peer(membership: &Membership, items: &[Item], decided: u64, signers: &[u64]) -> Url {
        let store = ChainStore::new(MemBackend::default());
        let root = membership.root_of(EpochNumber::new(EPOCH));
        for item in items {
            let mut parts = Parts::default();
            if item.height() == decided {
                parts.cert2 = Some(cert2(&item.leaf, EPOCH, signers));
            }
            if item.height() == root {
                parts.state_cert = Some(state_cert(root, &[0, 1]));
            }
            store.put(item, &parts).await.unwrap();
        }
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let service = ApiService::new(store, "127.0.0.1:1".parse().unwrap());
        tokio::spawn(service.serve(listener));
        url.parse().unwrap()
    }

    fn filler(store: &ChainStore) -> GapFiller {
        GapFiller::new(
            store.clone(),
            "127.0.0.1:1".parse().unwrap(),
            Vec::new(),
            None,
        )
    }

    /// The certificates a peer has for a complete epoch are checked and stored, and the epoch is
    /// recorded as checked.
    #[tokio::test]
    async fn takes_the_certificates_of_a_complete_epoch() {
        let membership = membership();
        let (store, items) = store(&membership).await;
        let decided = items[3].height();
        let peer = peer(&membership, &items, decided, &[0, 1, 2]).await;

        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                if filler(&store)
                    .check_certs(&membership, std::slice::from_ref(&peer))
                    .await
                    .is_ok()
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("a pass succeeds");

        let epoch = EpochNumber::new(EPOCH);
        assert!(store.certs_checked(epoch).await.unwrap());
        assert!(store.cert2(decided).await.unwrap().is_some());
        assert!(store.cert2(decided + 1).await.unwrap().is_none());
        let root = membership.root_of(epoch);
        assert!(store.state_cert(root).await.unwrap().is_some());
    }

    /// A certificate that does not check fails the pass, and the epoch stays unchecked.
    #[tokio::test]
    async fn refuses_a_cert2_without_a_quorum() {
        let membership = membership();
        let (store, items) = store(&membership).await;
        let decided = items[3].height();
        let peer = peer(&membership, &items, decided, &[0, 1]).await;

        let mut failed = false;
        for _ in 0..20 {
            if filler(&store)
                .check_certs(&membership, std::slice::from_ref(&peer))
                .await
                .is_err()
            {
                failed = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert!(failed, "the pass fails");
        assert!(!store.certs_checked(EpochNumber::new(EPOCH)).await.unwrap());
        assert!(store.cert2(decided).await.unwrap().is_none());
    }
}
