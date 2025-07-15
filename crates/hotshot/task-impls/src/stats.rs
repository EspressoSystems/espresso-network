use std::{collections::BTreeMap, sync::Arc};

use async_broadcast::{Receiver, Sender};
use async_trait::async_trait;
use either::Either;
use hotshot_task::task::TaskState;
use hotshot_types::{
    consensus::OuterConsensus,
    epoch_membership::EpochMembershipCoordinator,
    traits::node_implementation::{ConsensusTime, NodeType},
    vote::HasViewNumber,
};
use hotshot_utils::{
    anytrace::{Error, Level, Result},
    line_info, warn,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::events::HotShotEvent;

#[derive(Serialize, Deserialize)]
pub struct LeaderViewStats<TYPES: NodeType> {
    pub view: TYPES::View,
    pub prev_proposal_send: Option<i128>,
    pub proposal_send: Option<i128>,
    pub vote_recv: Option<i128>,
    pub da_proposal_send: Option<i128>,
    pub builder_start: Option<i128>,
    pub block_built: Option<i128>,
    pub vid_disperse_send: Option<i128>,
    pub timeout_certificate_formed: Option<i128>,
    pub qc_formed: Option<i128>,
    pub da_cert_send: Option<i128>,
}

#[derive(Serialize, Deserialize)]
pub struct ReplicaViewStats<TYPES: NodeType> {
    pub view: TYPES::View,
    pub view_change: Option<i128>,
    pub proposal_recv: Option<i128>,
    pub vote_send: Option<i128>,
    pub timeout_vote_send: Option<i128>,
    pub da_proposal_received: Option<i128>,
    pub da_proposal_validated: Option<i128>,
    pub da_certificate_recv: Option<i128>,
    pub proposal_prelim_validated: Option<i128>,
    pub proposal_validated: Option<i128>,
    pub timeout_triggered: Option<i128>,
    pub vid_share_validated: Option<i128>,
    pub vid_share_recv: Option<i128>,
}

impl<TYPES: NodeType> LeaderViewStats<TYPES> {
    fn new(view: TYPES::View) -> Self {
        Self {
            view,
            prev_proposal_send: None,
            proposal_send: None,
            vote_recv: None,
            da_proposal_send: None,
            builder_start: None,
            block_built: None,
            vid_disperse_send: None,
            timeout_certificate_formed: None,
            qc_formed: None,
            da_cert_send: None,
        }
    }
}

impl<TYPES: NodeType> ReplicaViewStats<TYPES> {
    fn new(view: TYPES::View) -> Self {
        Self {
            view,
            view_change: None,
            proposal_recv: None,
            vote_send: None,
            timeout_vote_send: None,
            da_proposal_received: None,
            da_proposal_validated: None,
            da_certificate_recv: None,
            proposal_prelim_validated: None,
            proposal_validated: None,
            timeout_triggered: None,
            vid_share_validated: None,
            vid_share_recv: None,
        }
    }
}

pub struct StatsTaskState<TYPES: NodeType> {
    view: TYPES::View,
    epoch: Option<TYPES::Epoch>,
    public_key: TYPES::SignatureKey,
    consensus: OuterConsensus<TYPES>,
    membership_coordinator: EpochMembershipCoordinator<TYPES>,
    leader_stats: BTreeMap<TYPES::View, LeaderViewStats<TYPES>>,
    replica_stats: BTreeMap<TYPES::View, ReplicaViewStats<TYPES>>,
}

impl<TYPES: NodeType> StatsTaskState<TYPES> {
    pub fn new(
        view: TYPES::View,
        epoch: Option<TYPES::Epoch>,
        public_key: TYPES::SignatureKey,
        consensus: OuterConsensus<TYPES>,
        membership_coordinator: EpochMembershipCoordinator<TYPES>,
    ) -> Self {
        Self {
            view,
            epoch,
            public_key,
            consensus,
            membership_coordinator,
            leader_stats: BTreeMap::new(),
            replica_stats: BTreeMap::new(),
        }
    }
    fn leader_entry(&mut self, view: TYPES::View) -> &mut LeaderViewStats<TYPES> {
        self.leader_stats
            .entry(view)
            .or_insert_with(|| LeaderViewStats::new(view))
    }
    fn replica_entry(&mut self, view: TYPES::View) -> &mut ReplicaViewStats<TYPES> {
        self.replica_stats
            .entry(view)
            .or_insert_with(|| ReplicaViewStats::new(view))
    }
    fn garbage_collect(&mut self, view: TYPES::View) {
        self.leader_stats = self.leader_stats.split_off(&view);
        self.replica_stats = self.replica_stats.split_off(&view);
    }

    fn dump_stats(&self) -> Result<()> {
        let mut writer = csv::Writer::from_writer(vec![]);
        for (_, leader_stats) in self.leader_stats.iter() {
            writer
                .serialize(leader_stats)
                .map_err(|e| warn!("Failed to serialize leader stats: {}", e))?;
        }
        let output = writer
            .into_inner()
            .map_err(|e| warn!("Failed to serialize replica stats: {}", e))?;
        tracing::warn!(
            "Leader stats: {}",
            String::from_utf8(output)
                .map_err(|e| warn!("Failed to convert leader stats to string: {}", e))?
        );
        let mut writer = csv::Writer::from_writer(vec![]);
        for (_, replica_stats) in self.replica_stats.iter() {
            writer
                .serialize(replica_stats)
                .map_err(|e| warn!("Failed to serialize replica stats: {}", e))?;
        }
        let output = writer
            .into_inner()
            .map_err(|e| warn!("Failed to serialize replica stats: {}", e))?;
        tracing::warn!(
            "Replica stats: {}",
            String::from_utf8(output)
                .map_err(|e| warn!("Failed to convert replica stats to string: {}", e))?
        );
        Ok(())
    }
}

#[async_trait]
impl<TYPES: NodeType> TaskState for StatsTaskState<TYPES> {
    type Event = HotShotEvent<TYPES>;

    async fn handle_event(
        &mut self,
        event: Arc<Self::Event>,
        _sender: &Sender<Arc<Self::Event>>,
        _receiver: &Receiver<Arc<Self::Event>>,
    ) -> Result<()> {
        let now = OffsetDateTime::now_utc().unix_timestamp_nanos();

        match event.as_ref() {
            HotShotEvent::BlockRecv(block_recv) => {
                self.leader_entry(block_recv.view_number).block_built = Some(now);
            },
            HotShotEvent::QuorumProposalRecv(proposal, _) => {
                self.replica_entry(proposal.data.view_number())
                    .proposal_recv = Some(now);
            },
            HotShotEvent::QuorumVoteRecv(_vote) => {
                // self.leader_entry(vote.view_number()).vote_recv = Some(now);
            },
            HotShotEvent::TimeoutVoteRecv(_vote) => {},
            HotShotEvent::TimeoutVoteSend(vote) => {
                self.replica_entry(vote.view_number()).timeout_vote_send = Some(now);
            },
            HotShotEvent::DaProposalRecv(proposal, _) => {
                self.replica_entry(proposal.data.view_number())
                    .da_proposal_received = Some(now);
            },
            HotShotEvent::DaProposalValidated(proposal, _) => {
                self.replica_entry(proposal.data.view_number())
                    .da_proposal_validated = Some(now);
            },
            HotShotEvent::DaVoteRecv(_simple_vote) => {},
            HotShotEvent::DaCertificateRecv(simple_certificate) => {
                self.replica_entry(simple_certificate.view_number())
                    .da_certificate_recv = Some(now);
            },
            HotShotEvent::DaCertificateValidated(_simple_certificate) => {},
            HotShotEvent::QuorumProposalSend(proposal, _) => {
                self.leader_entry(proposal.data.view_number()).proposal_send = Some(now);

                // If the last view succeeded, add the metric for time between proposals
                if proposal.data.view_change_evidence().is_none() {
                    if let Some(previous_proposal_time) = self
                        .replica_entry(proposal.data.view_number() - 1)
                        .proposal_recv
                    {
                        self.leader_entry(proposal.data.view_number())
                            .prev_proposal_send = Some(previous_proposal_time);

                        // calculate the elapsed time as milliseconds (from nanoseconds)
                        let elapsed_time = (now - previous_proposal_time) / 1_000_000;

                        if elapsed_time > 0 {
                            self.consensus
                                .read()
                                .await
                                .metrics
                                .previous_proposal_to_proposal_time
                                .add_point(elapsed_time as f64);
                        } else {
                            tracing::warn!("Previous proposal time is in the future");
                        }
                    }
                }
            },
            HotShotEvent::QuorumVoteSend(simple_vote) => {
                self.replica_entry(simple_vote.view_number()).vote_send = Some(now);
            },
            HotShotEvent::ExtendedQuorumVoteSend(simple_vote) => {
                self.replica_entry(simple_vote.view_number()).vote_send = Some(now);
            },
            HotShotEvent::QuorumProposalValidated(proposal, _) => {
                self.replica_entry(proposal.data.view_number())
                    .proposal_validated = Some(now);
            },
            HotShotEvent::DaProposalSend(proposal, _) => {
                self.leader_entry(proposal.data.view_number())
                    .da_proposal_send = Some(now);
            },
            HotShotEvent::DaVoteSend(simple_vote) => {
                self.replica_entry(simple_vote.view_number()).vote_send = Some(now);
            },
            HotShotEvent::QcFormed(either) => {
                match either {
                    Either::Left(qc) => {
                        self.leader_entry(qc.view_number() + 1).qc_formed = Some(now)
                    },
                    Either::Right(tc) => {
                        self.leader_entry(tc.view_number())
                            .timeout_certificate_formed = Some(now)
                    },
                };
            },
            HotShotEvent::Qc2Formed(either) => {
                match either {
                    Either::Left(qc) => {
                        self.leader_entry(qc.view_number() + 1).qc_formed = Some(now)
                    },
                    Either::Right(tc) => {
                        self.leader_entry(tc.view_number())
                            .timeout_certificate_formed = Some(now)
                    },
                };
            },
            HotShotEvent::DacSend(simple_certificate, _) => {
                self.leader_entry(simple_certificate.view_number())
                    .da_cert_send = Some(now);
            },
            HotShotEvent::ViewChange(view, epoch) => {
                // Record the timestamp of the first observed view change
                // This can happen when transitioning to the next view, either due to voting
                // or receiving a proposal, but we only store the first one
                if self.replica_entry(*view + 1).view_change.is_none() {
                    self.replica_entry(*view + 1).view_change = Some(now);
                }

                if *epoch <= self.epoch && *view <= self.view {
                    return Ok(());
                }
                if self.view < *view {
                    self.view = *view;
                }
                let mut new_epoch = false;
                if self.epoch < *epoch {
                    self.epoch = *epoch;
                    new_epoch = true;
                }
                if *view == TYPES::View::new(0) {
                    return Ok(());
                }

                if new_epoch {
                    let _ = self.dump_stats();
                    self.garbage_collect(*view - 1);
                }

                let leader = self
                    .membership_coordinator
                    .membership_for_epoch(*epoch)
                    .await?
                    .leader(*view)
                    .await?;
                if leader == self.public_key {
                    self.leader_entry(*view).builder_start = Some(now);
                }
            },
            HotShotEvent::Timeout(..) => {
                self.replica_entry(self.view).timeout_triggered = Some(now);
            },
            HotShotEvent::TransactionsRecv(_txns) => {
                // TODO: Track transactions by time
                // #3526 https://github.com/EspressoSystems/espresso-network/issues/3526
            },
            HotShotEvent::SendPayloadCommitmentAndMetadata(_, _, _, view, _) => {
                self.leader_entry(*view).vid_disperse_send = Some(now);
            },
            HotShotEvent::VidShareRecv(_, proposal) => {
                self.replica_entry(proposal.data.view_number())
                    .vid_share_recv = Some(now);
            },
            HotShotEvent::VidShareValidated(proposal) => {
                self.replica_entry(proposal.data.view_number())
                    .vid_share_validated = Some(now);
            },
            HotShotEvent::QuorumProposalPreliminarilyValidated(proposal) => {
                self.replica_entry(proposal.data.view_number())
                    .proposal_prelim_validated = Some(now);
            },
            _ => {},
        }
        Ok(())
    }

    fn cancel_subtasks(&mut self) {
        // No subtasks to cancel
    }
}
