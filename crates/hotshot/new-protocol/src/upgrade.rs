//! The upgrade sub-protocol: proposing and voting on protocol version
//! upgrades. Owns the impure inputs of that flow — the wall clock and the
//! window configuration — so the pure consensus state machine never needs
//! them.

use std::{
    collections::BTreeSet,
    time::{SystemTime, UNIX_EPOCH},
};

use committable::Committable;
use hotshot_types::{
    data::{EpochNumber, UpgradeProposal, ViewNumber},
    message::UpgradeLock,
    simple_vote::{UpgradeProposalData, UpgradeVote},
    traits::{node_implementation::NodeType, signature_key::SignatureKey},
    upgrade_config::UpgradeConfig,
};
use tracing::{debug, info, warn};

use crate::message::{UpgradeProposalMessage, UpgradeVoteMessage};

/// The `UpgradeProposalData` every honest node expects for an upgrade
/// proposed at `view`. Voters require full equality with this, so a leader
/// cannot pick rogue offsets (e.g. a `new_version_first_view` so close that
/// the wire format would flip before the decided certificate propagates).
pub(crate) fn expected_upgrade_data<T: NodeType>(
    upgrade: &versions::Upgrade,
    view: ViewNumber,
) -> UpgradeProposalData {
    UpgradeProposalData {
        old_version: upgrade.base,
        new_version: upgrade.target,
        decide_by: view + T::UPGRADE_CONSTANTS.decide_by_offset,
        new_version_hash: upgrade.hash().into(),
        old_version_last_view: view + (T::UPGRADE_CONSTANTS.finish_offset - 1),
        new_version_first_view: view + T::UPGRADE_CONSTANTS.finish_offset,
    }
}

pub struct UpgradeProtocol<T: NodeType> {
    config: UpgradeConfig,
    upgrade_lock: UpgradeLock<T>,
    public_key: T::SignatureKey,
    private_key: <T::SignatureKey as SignatureKey>::PrivateKey,
    proposed_views: BTreeSet<ViewNumber>,
    voted_views: BTreeSet<ViewNumber>,
}

impl<T: NodeType> UpgradeProtocol<T> {
    pub fn new(
        config: UpgradeConfig,
        upgrade_lock: UpgradeLock<T>,
        public_key: T::SignatureKey,
        private_key: <T::SignatureKey as SignatureKey>::PrivateKey,
    ) -> Self {
        Self {
            config,
            upgrade_lock,
            public_key,
            private_key,
            proposed_views: BTreeSet::new(),
            voted_views: BTreeSet::new(),
        }
    }

    /// An upgrade proposal for `view`, when this node leads `view`, the
    /// proposing windows are open, and no upgrade is decided yet.
    pub fn maybe_propose(
        &mut self,
        view: ViewNumber,
        is_leader: bool,
    ) -> Option<UpgradeProposalMessage<T>> {
        if !is_leader || !self.enabled() || self.upgraded() {
            return None;
        }
        if self.proposed_views.contains(&view) {
            return None;
        }
        if !in_window(
            *view,
            self.config.start_proposing_view,
            self.config.stop_proposing_view,
        ) || !in_window(
            unix_time(),
            self.config.start_proposing_time,
            self.config.stop_proposing_time,
        ) {
            return None;
        }
        let data = expected_upgrade_data::<T>(&self.upgrade_lock.upgrade(), view);
        let signature = match T::SignatureKey::sign(&self.private_key, data.commit().as_ref()) {
            Ok(signature) => signature,
            Err(err) => {
                warn!(%view, %err, "failed to sign upgrade proposal");
                return None;
            },
        };
        info!(
            %view,
            new_version = %data.new_version,
            decide_by = %data.decide_by,
            first_view = %data.new_version_first_view,
            "proposing upgrade"
        );
        self.proposed_views.insert(view);
        Some(UpgradeProposalMessage::new(
            UpgradeProposal {
                upgrade_proposal: data,
                view_number: view,
            },
            signature,
        ))
    }

    /// A vote for a received upgrade proposal, when it is exactly the one
    /// this node expects and the voting windows are open.
    pub fn maybe_vote(
        &mut self,
        message: &UpgradeProposalMessage<T>,
        sender: &T::SignatureKey,
        leader: Option<&T::SignatureKey>,
        current_view: ViewNumber,
        current_epoch: EpochNumber,
    ) -> Option<UpgradeVoteMessage<T>> {
        let view = message.data.view_number;
        if !self.enabled() || self.upgraded() {
            return None;
        }
        if self.voted_views.contains(&view) {
            return None;
        }
        if !in_window(
            *view,
            self.config.start_voting_view,
            self.config.stop_voting_view,
        ) || !in_window(
            unix_time(),
            self.config.start_voting_time,
            self.config.stop_voting_time,
        ) {
            debug!(%view, "upgrade proposal outside the voting window");
            return None;
        }
        if view + 1 < current_view {
            debug!(%view, %current_view, "upgrade proposal for stale view");
            return None;
        }
        if leader != Some(sender) {
            warn!(%view, "upgrade proposal not sent by the view leader");
            return None;
        }
        let data = &message.data.upgrade_proposal;
        if !sender.validate(&message.signature, data.commit().as_ref()) {
            warn!(%view, "invalid upgrade proposal signature");
            return None;
        }
        if *data != expected_upgrade_data::<T>(&self.upgrade_lock.upgrade(), view) {
            warn!(%view, ?data, "upgrade proposal data differs from the expected data");
            return None;
        }
        let vote = match UpgradeVote::<T>::create_signed_vote(
            data.clone(),
            view,
            &self.public_key,
            &self.private_key,
            &self.upgrade_lock,
        ) {
            Ok(vote) => vote,
            Err(err) => {
                warn!(%view, %err, "failed to sign upgrade vote");
                return None;
            },
        };
        info!(%view, new_version = %data.new_version, "voting for upgrade");
        self.voted_views.insert(view);
        Some(UpgradeVoteMessage {
            vote,
            epoch: current_epoch,
        })
    }

    pub fn gc(&mut self, view: ViewNumber) {
        self.proposed_views = self.proposed_views.split_off(&view);
        self.voted_views = self.voted_views.split_off(&view);
    }

    /// A trivial `Upgrade` (base == target) disables the sub-protocol
    /// entirely, whatever the windows say.
    fn enabled(&self) -> bool {
        let upgrade = self.upgrade_lock.upgrade();
        upgrade.base < upgrade.target
    }

    fn upgraded(&self) -> bool {
        self.upgrade_lock.decided_upgrade_cert().is_some()
    }
}

fn in_window(value: u64, start: u64, stop: u64) -> bool {
    start <= value && value < stop
}

fn unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use hotshot_example_types::node_types::TestTypes;
    use hotshot_types::{
        message::UpgradeLock,
        traits::{node_implementation::NodeType, signature_key::SignatureKey},
    };
    use versions::{Upgrade, version};

    use super::*;

    type Key = <TestTypes as NodeType>::SignatureKey;

    fn upgrade_protocol(config: UpgradeConfig) -> UpgradeProtocol<TestTypes> {
        let (public_key, private_key) = Key::generated_from_seed_indexed([0; 32], 0);
        let lock = UpgradeLock::new(Upgrade::new(version(0, 6), version(0, 7)));
        UpgradeProtocol::new(config, lock, public_key, private_key)
    }

    fn open_view_windows() -> UpgradeConfig {
        UpgradeConfig {
            start_proposing_view: 5,
            stop_proposing_view: 15,
            start_voting_view: 0,
            stop_voting_view: u64::MAX,
            start_proposing_time: 0,
            stop_proposing_time: u64::MAX,
            start_voting_time: 0,
            stop_voting_time: u64::MAX,
        }
    }

    #[test]
    fn expected_data_offsets() {
        let upgrade = Upgrade::new(version(0, 6), version(0, 7));
        let view = ViewNumber::new(10);
        let data = expected_upgrade_data::<TestTypes>(&upgrade, view);
        assert_eq!(data.old_version, version(0, 6));
        assert_eq!(data.new_version, version(0, 7));
        assert_eq!(data.new_version_hash, Vec::<u8>::from(upgrade.hash()));
        assert!(data.decide_by > view);
        assert!(data.decide_by < data.new_version_first_view);
        assert_eq!(data.old_version_last_view + 1, data.new_version_first_view);
    }

    #[test]
    fn proposes_only_inside_window_and_once() {
        let mut protocol = upgrade_protocol(open_view_windows());
        assert!(protocol.maybe_propose(ViewNumber::new(4), true).is_none());
        assert!(protocol.maybe_propose(ViewNumber::new(15), true).is_none());
        assert!(protocol.maybe_propose(ViewNumber::new(7), false).is_none());
        let proposal = protocol.maybe_propose(ViewNumber::new(7), true).unwrap();
        assert_eq!(proposal.data.view_number, ViewNumber::new(7));
        assert!(protocol.maybe_propose(ViewNumber::new(7), true).is_none());
    }

    #[test]
    fn disabled_config_never_proposes() {
        let mut protocol = upgrade_protocol(UpgradeConfig::default());
        assert!(protocol.maybe_propose(ViewNumber::new(7), true).is_none());
    }

    #[test]
    fn trivial_upgrade_never_proposes() {
        let (public_key, private_key) = Key::generated_from_seed_indexed([0; 32], 0);
        let lock = UpgradeLock::<TestTypes>::new(Upgrade::trivial(version(0, 6)));
        let mut protocol = UpgradeProtocol::new(open_view_windows(), lock, public_key, private_key);
        assert!(protocol.maybe_propose(ViewNumber::new(7), true).is_none());
    }

    #[test]
    fn votes_only_for_expected_data_from_leader() {
        let mut leader = upgrade_protocol(open_view_windows());
        let mut voter = upgrade_protocol(open_view_windows());
        let leader_key = leader.public_key;
        let view = ViewNumber::new(7);
        let epoch = EpochNumber::new(1);

        let proposal = leader.maybe_propose(view, true).unwrap();

        let (other_key, _) = Key::generated_from_seed_indexed([0; 32], 1);
        assert!(
            voter
                .maybe_vote(&proposal, &leader_key, Some(&other_key), view, epoch)
                .is_none()
        );

        let mut tampered = proposal.clone();
        tampered.data.upgrade_proposal.new_version_first_view = view + 1;
        assert!(
            voter
                .maybe_vote(&tampered, &leader_key, Some(&leader_key), view, epoch)
                .is_none()
        );

        assert!(
            voter
                .maybe_vote(&proposal, &leader_key, Some(&leader_key), view + 2, epoch)
                .is_none()
        );

        let vote = voter
            .maybe_vote(&proposal, &leader_key, Some(&leader_key), view, epoch)
            .unwrap();
        assert_eq!(vote.epoch, epoch);
        assert_eq!(
            vote.vote.data,
            expected_upgrade_data::<TestTypes>(&voter.upgrade_lock.upgrade(), view)
        );

        assert!(
            voter
                .maybe_vote(&proposal, &leader_key, Some(&leader_key), view, epoch)
                .is_none()
        );
    }
}
