//! The chain as an API service follows it: the newest block it stored, and what the blocks it
//! stored decide for the API, such as stake tables, the protocol version and the chain config.
//!
//! The API service keeps this itself rather than asking consensus, so it serves the same answers
//! whether its blocks come from its own consensus or from other API services.

use std::sync::Arc;

use anyhow::anyhow;
use async_lock::RwLock;
use committable::Committable;
use espresso_types::{
    ChainConfig, Header, Leaf2, SeqTypes,
    v0::traits::{SequencerPersistence, StateCatchup},
};
use hotshot_query_service::availability::LeafQueryData;
use hotshot_types::{
    consensus::ParticipationTracker,
    data::{EpochNumber, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    traits::election::{Membership, MembershipSnapshot},
    utils::{is_epoch_root, is_transition_block},
};
use parking_lot::Mutex;
use tokio::sync::watch;

pub(crate) struct ChainView<P> {
    pub(crate) persistence: Arc<P>,
    pub(crate) coordinator: EpochMembershipCoordinator<SeqTypes>,
    pub(crate) state_catchup: Arc<dyn StateCatchup>,
    pub(crate) decided: watch::Sender<Option<LeafQueryData<SeqTypes>>>,
    pub(crate) chain_config: Arc<RwLock<ChainConfig>>,
    pub(crate) upgrade_lock: UpgradeLock<SeqTypes>,
    pub(crate) epoch_height: u64,
    pub(crate) participation: Arc<Mutex<Participation>>,
}

/// Proposal and vote participation, as far as the stored blocks show it.
///
/// Votes are counted from the certificates of the stored blocks, as consensus counts them. A
/// leader counts as having proposed in a view whose block was decided, and as having missed a
/// view between two consecutive stored blocks; a proposal that was never decided does not count.
pub(crate) struct Participation {
    pub(crate) tracker: ParticipationTracker<SeqTypes>,

    /// The height and view of the last block taken in, to find the views between it and the next.
    last: Option<(u64, ViewNumber)>,
}

impl Participation {
    pub(crate) fn new(tracker: ParticipationTracker<SeqTypes>) -> Self {
        Self {
            tracker,
            last: None,
        }
    }
}

impl<P: SequencerPersistence> ChainView<P> {
    /// Takes in `leaf`, which the API service has just stored, the next after the last one.
    pub(crate) async fn track(&self, leaf: &LeafQueryData<SeqTypes>) -> anyhow::Result<()> {
        self.track_upgrade(leaf.leaf()).await;
        self.track_epoch(leaf.leaf()).await?;
        self.track_chain_config(leaf.header()).await;
        self.track_participation(leaf.leaf());
        self.decided.send_replace(Some(leaf.clone()));
        Ok(())
    }

    /// Continues from `leaf`, the newest block the API service stored before it restarted, which
    /// it took in then.
    pub(crate) async fn resume(&self, leaf: &LeafQueryData<SeqTypes>) {
        self.track_chain_config(leaf.header()).await;
        let last = (leaf.leaf().height(), leaf.leaf().view_number());
        self.participation.lock().last = Some(last);
        self.decided.send_replace(Some(leaf.clone()));
    }

    fn track_participation(&self, leaf: &Leaf2) {
        let (height, view) = (leaf.height(), leaf.view_number());
        let mut participation = self.participation.lock();
        if let Some(epoch) = leaf.epoch(self.epoch_height)
            && let Ok(membership) = self.coordinator.membership_for_epoch(epoch)
        {
            // Views between two consecutive blocks decided nothing, so their leaders missed.
            if let Some((last_height, last_view)) = participation.last
                && height == last_height + 1
            {
                for missed in (*last_view + 1)..*view {
                    if let Ok(leader) = membership.leader(ViewNumber::new(missed)) {
                        participation.tracker.leader_missed(leader, epoch);
                    }
                }
            }
            if let Ok(leader) = membership.leader(view) {
                participation.tracker.leader_proposed(leader, epoch);
            }
        }
        participation
            .tracker
            .on_leaf_decided(leaf, &self.coordinator);
        participation.last = Some((height, view));
    }

    /// As a validator does on decide, so the API validates state certificates against the version
    /// the network runs.
    async fn track_upgrade(&self, leaf: &Leaf2) {
        let Some(cert) = leaf.upgrade_certificate() else {
            return;
        };
        if self.upgrade_lock.decided_upgrade_cert().as_ref() == Some(cert) {
            return;
        }
        tracing::warn!(
            height = leaf.height(),
            version = ?cert.data.new_version,
            "the network decided an upgrade"
        );
        self.upgrade_lock.set_decided_upgrade_cert(cert.clone());
        if let Err(err) = self
            .persistence
            .store_upgrade_certificate(Some(cert.clone()))
            .await
        {
            tracing::warn!(
                height = leaf.height(),
                "cannot persist the upgrade certificate: {err:#}"
            );
        }
    }

    /// Awaited, so the transition block finds the stake table it supplies a DRB for, and a
    /// failed derivation fails the block and is retried with it. Nothing is redone when the
    /// membership already knows the result, as after a restart or a catchup walk.
    async fn track_epoch(&self, leaf: &Leaf2) -> anyhow::Result<()> {
        let height = leaf.height();
        let Some(epoch) = leaf.epoch(self.epoch_height) else {
            return Ok(());
        };
        if is_epoch_root(height, self.epoch_height) && !self.has_stake_table(epoch + 2) {
            self.coordinator
                .add_epoch_root(leaf.block_header().clone())
                .await
                .map_err(|err| anyhow!("deriving the stake table of epoch {}: {err}", epoch + 2))?;
        }
        if is_transition_block(height, self.epoch_height)
            && let Some(drb) = leaf.next_drb_result
            && !self.has_drb(epoch + 1)
        {
            self.coordinator.supply_drb(epoch + 1, drb);
        }
        Ok(())
    }

    pub(crate) fn has_stake_table(&self, epoch: EpochNumber) -> bool {
        self.coordinator.membership().snapshot(epoch).is_some()
    }

    fn has_drb(&self, epoch: EpochNumber) -> bool {
        self.coordinator
            .membership()
            .snapshot(epoch)
            .is_some_and(|snapshot| snapshot.has_drb())
    }

    /// Keep the resolved chain config current, so `submit` enforces the right block size.
    async fn track_chain_config(&self, header: &Header) {
        let resolvable = header.chain_config();
        if resolvable.commit() == self.chain_config.read().await.commit() {
            return;
        }
        let resolved = match resolvable.resolve() {
            Some(chain_config) => Ok(chain_config),
            None => {
                self.state_catchup
                    .fetch_chain_config(resolvable.commit())
                    .await
            },
        };
        match resolved {
            Ok(chain_config) => *self.chain_config.write().await = chain_config,
            Err(err) => tracing::warn!(
                height = header.height(),
                "cannot resolve the chain config: {err:#}"
            ),
        }
    }
}
