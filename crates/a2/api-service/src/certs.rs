//! The certificates the chain does not commit to: the `Certificate2` that decided a block directly,
//! and the state certificate of an epoch root.
//!
//! Each is checked against the stake table of its block's epoch, so they are filled an epoch at a
//! time, once membership has that stake table and the store has every block of the epoch. A height
//! whose certificates are known, [`Parts::certs_known`], has them as consensus made them: the
//! ingester's heights, from the decide log, and heights checked here. The others are asked of the
//! peers, and only a peer that knows a height's certificates answers for it: a height it knows
//! without a `Certificate2` was decided by a later block, and keeps none. A peer that filled the
//! height without its certificates may simply not have them.
//!
//! Both walks go from the newest epoch back to the fill front, the end of the newest gap in the
//! store: every height from there on is stored, and gaps are filled from the newest back, so the
//! epochs before the front become workable as the front moves back. Stopping there keeps a pass
//! from asking the store about every epoch of a history that is not filled yet.

use std::{collections::BTreeSet, ops::Range};

use chain_store::{Item, Part, Parts};
use hotshot_types::data::EpochNumber;

use crate::{GapFiller, Membership, gaps::runs, peers::Peer};

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
    pub(crate) async fn check_certs(&mut self, membership: &Membership) -> anyhow::Result<()> {
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
            if self.checked_epochs.contains(&epoch)
                || self.epoch_retries.waiting(*epoch)
                || membership.stake_table(epoch).is_none()
            {
                continue;
            }
            // An epoch no peer can answer for does not hold up the ones before it.
            let peers = self.peers.shuffled();
            match self.check_epoch(membership, epoch, &peers).await {
                Ok(()) => self.epoch_retries.succeeded(*epoch),
                Err(err) => {
                    tracing::warn!(%epoch, "checking certificates failed: {err:#}");
                    self.epoch_retries.failed(*epoch, *epoch + 1);
                },
            }
            checked += 1;
        }
        Ok(())
    }

    /// The fill front and the newest height, as of the last time the gaps were looked for:
    /// every height from the front up to the newest is stored. The gaps the filler has not
    /// filled bound the front; only past the cursor, which a pass that found many gaps leaves
    /// behind the newest height, the store is asked.
    async fn filled(&self) -> anyhow::Result<Option<(u64, u64)>> {
        let Some(newest) = self.newest else {
            return Ok(None);
        };
        let front = match self.items_from > newest {
            true => self.item_retries.newest_end().unwrap_or(0).max(self.floor),
            false => match self.store.newest_gap().await? {
                Some(gap) => gap.end,
                None => self.floor,
            },
        };
        Ok(Some((front, newest)))
    }

    /// Fills the certificates of the heights of `epoch` whose certificates are not known from the
    /// peers, and remembers the epoch as checked once a peer has answered for each height.
    async fn check_epoch(
        &mut self,
        membership: &Membership,
        epoch: EpochNumber,
        peers: &[Peer],
    ) -> anyhow::Result<()> {
        let heights = membership.heights_of(epoch);
        let wanted: BTreeSet<u64> = self
            .store
            .missing_parts(Part::CertsKnown, heights.start, count(&heights))
            .await?
            .into_iter()
            .filter(|height| heights.contains(height))
            .collect();
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
        self.checked_epochs.insert(epoch);
        if !wanted.is_empty() {
            tracing::info!(%epoch, "certificates checked");
        }
        Ok(())
    }

    /// Stores the certificates `peer` sends for `heights` that check against the stored leaves.
    /// Fails unless the peer knows the certificates of every height, since a height it lacks, or
    /// has without knowing its certificates, says nothing about them.
    async fn take_certs(
        &self,
        membership: &Membership,
        peer: &Peer,
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
            let parts = record.parts()?;
            anyhow::ensure!(
                parts.certs_known,
                "{peer} does not know the certificates of height {}",
                record.height
            );
            let Some(item) = self.store.item(record.height).await? else {
                continue;
            };
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
        certs_known: true,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chain_store::{ChainStore, mem::MemBackend};
    use hotshot_types::simple_certificate::QuorumCertificate2;
    use tokio::net::TcpListener;
    use url::Url;

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
    /// for the epoch root a state certificate. It knows the certificates if `knows`.
    async fn peer(
        membership: &Membership,
        items: &[Item],
        decided: u64,
        signers: &[u64],
        knows: bool,
    ) -> Url {
        let store = ChainStore::new(MemBackend::default());
        let root = membership.root_of(EpochNumber::new(EPOCH));
        for item in items {
            let mut parts = Parts {
                certs_known: knows,
                ..Default::default()
            };
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

    /// A filler over `store`, asking `peer`, that has looked for gaps and found none.
    async fn filler(store: &ChainStore, peer: &Url) -> GapFiller {
        let peers = crate::Peers::new("127.0.0.1:1".parse().unwrap(), vec![peer.clone()], None);
        let mut filler = GapFiller::new(store.clone(), peers, None);
        let newest = store.newest().await.unwrap().unwrap();
        (filler.newest, filler.items_from) = (Some(newest), newest + 1);
        filler
    }

    /// Whether every height of `EPOCH` has its certificates known.
    async fn known(store: &ChainStore, membership: &Membership) -> bool {
        let heights = membership.heights_of(EpochNumber::new(EPOCH));
        let missing = store
            .missing_parts(Part::CertsKnown, heights.start, 1)
            .await
            .unwrap();
        !missing
            .first()
            .is_some_and(|height| heights.contains(height))
    }

    /// The certificates a peer has for a complete epoch are checked and stored, and every height
    /// of the epoch then has its certificates known.
    #[tokio::test]
    async fn takes_the_certificates_of_a_complete_epoch() {
        let membership = membership();
        let (store, items) = store(&membership).await;
        let decided = items[3].height();
        let peer = peer(&membership, &items, decided, &[0, 1, 2], true).await;

        tokio::time::timeout(Duration::from_secs(10), async {
            while !known(&store, &membership).await {
                filler(&store, &peer)
                    .await
                    .check_certs(&membership)
                    .await
                    .unwrap();
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("a pass takes the certificates");

        let epoch = EpochNumber::new(EPOCH);
        assert!(known(&store, &membership).await);
        assert!(store.cert2(decided).await.unwrap().is_some());
        assert!(store.cert2(decided + 1).await.unwrap().is_none());
        let root = membership.root_of(epoch);
        assert!(store.state_cert(root).await.unwrap().is_some());
    }

    /// A certificate that does not check leaves the epoch unchecked.
    #[tokio::test]
    async fn refuses_a_cert2_without_a_quorum() {
        let membership = membership();
        let (store, items) = store(&membership).await;
        let decided = items[3].height();
        let peer = peer(&membership, &items, decided, &[0, 1], true).await;
        assert_fails(&store, &membership, &peer).await;
        assert!(store.cert2(decided).await.unwrap().is_none());
    }

    /// A peer that has the heights but does not know their certificates answers for none of them,
    /// since its lack of a certificate says nothing.
    #[tokio::test]
    async fn refuses_a_peer_that_does_not_know_the_certificates() {
        let membership = membership();
        let (store, items) = store(&membership).await;
        let peer = peer(&membership, &items, items[3].height(), &[0, 1, 2], false).await;
        assert_fails(&store, &membership, &peer).await;
        assert!(store.cert2(items[3].height()).await.unwrap().is_none());
    }

    /// A pass over `store` with `peer` leaves `EPOCH` unchecked, to be tried again later.
    async fn assert_fails(store: &ChainStore, membership: &Membership, peer: &Url) {
        let mut filler = filler(store, peer).await;
        filler.check_certs(membership).await.unwrap();
        assert!(filler.epoch_retries.waiting(EPOCH), "the epoch waits");
        assert!(!known(store, membership).await);
    }
}
