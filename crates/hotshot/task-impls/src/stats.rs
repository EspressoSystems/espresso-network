use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use async_broadcast::{Receiver, Sender};
use async_trait::async_trait;
use either::Either;
use hotshot_orchestrator::client::{BenchResults, OrchestratorClient};
use hotshot_task::task::TaskState;
use hotshot_types::{
    benchmarking::{LeaderViewStats, ReplicaViewStats},
    consensus::OuterConsensus,
    epoch_membership::EpochMembershipCoordinator,
    traits::{
        block_contents::BlockHeader,
        node_implementation::{ConsensusTime, NodeType},
        BlockPayload,
    },
    vote::HasViewNumber,
};
use hotshot_utils::{
    anytrace::{Error, Level, Result},
    line_info, warn,
};
use time::OffsetDateTime;
use url::Url;

use crate::events::HotShotEvent;

pub struct StatsTaskState<TYPES: NodeType> {
    node_index: u64,
    view: TYPES::View,
    epoch: Option<TYPES::Epoch>,
    public_key: TYPES::SignatureKey,
    consensus: OuterConsensus<TYPES>,
    membership_coordinator: EpochMembershipCoordinator<TYPES>,
    leader_stats: BTreeMap<TYPES::View, LeaderViewStats<TYPES::View>>,
    replica_stats: BTreeMap<TYPES::View, ReplicaViewStats<TYPES::View>>,
    latencies_by_view: BTreeMap<TYPES::View, i128>,
    sizes_by_view: BTreeMap<TYPES::View, i128>,
    epoch_start_times: BTreeMap<TYPES::Epoch, i128>,
    timeouts: BTreeSet<TYPES::View>,
    orchestrator_client: Option<OrchestratorClient>,
}

impl<TYPES: NodeType> StatsTaskState<TYPES> {
    pub fn new(
        node_index: u64,
        view: TYPES::View,
        epoch: Option<TYPES::Epoch>,
        public_key: TYPES::SignatureKey,
        consensus: OuterConsensus<TYPES>,
        membership_coordinator: EpochMembershipCoordinator<TYPES>,
        orchestrator_url: Option<Url>,
    ) -> Self {
        Self {
            node_index,
            view,
            epoch,
            public_key,
            consensus,
            membership_coordinator,
            leader_stats: BTreeMap::new(),
            replica_stats: BTreeMap::new(),
            latencies_by_view: BTreeMap::new(),
            sizes_by_view: BTreeMap::new(),
            epoch_start_times: BTreeMap::new(),
            timeouts: BTreeSet::new(),
            orchestrator_client: orchestrator_url.map(OrchestratorClient::new),
        }
    }
    fn leader_entry(&mut self, view: TYPES::View) -> &mut LeaderViewStats<TYPES::View> {
        self.leader_stats
            .entry(view)
            .or_insert_with(|| LeaderViewStats::new(view))
    }
    fn replica_entry(&mut self, view: TYPES::View) -> &mut ReplicaViewStats<TYPES::View> {
        self.replica_stats
            .entry(view)
            .or_insert_with(|| ReplicaViewStats::new(view))
    }
    fn garbage_collect(&mut self, view: TYPES::View) {
        self.leader_stats = self.leader_stats.split_off(&view);
        self.replica_stats = self.replica_stats.split_off(&view);
        self.latencies_by_view = self.latencies_by_view.split_off(&view);
        self.sizes_by_view = self.sizes_by_view.split_off(&view);
        self.timeouts = BTreeSet::new();
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

    fn log_basic_stats(&self, now: i128, epoch: &TYPES::Epoch) -> i128 {
        let num_views = self.latencies_by_view.len();
        let total_latency = self.latencies_by_view.values().sum::<i128>();
        let elapsed_time = if let Some(epoch_start_time) = self.epoch_start_times.get(epoch) {
            now - epoch_start_time
        } else {
            0
        };
        let average_latency = total_latency / num_views as i128;
        tracing::warn!("Average latency: {}ms", average_latency);
        tracing::warn!(
            "Number of timeouts in epoch: {}, is {}",
            epoch,
            self.timeouts.len()
        );
        let total_size = self.sizes_by_view.values().sum::<i128>();
        if total_size == 0 {
            // Either no TXNs or we are not in the DA committee and don't know block sizes
            return elapsed_time;
        }
        if elapsed_time > 0 {
            // multiply by 1000 to convert to seconds
            let throughput = (total_size / elapsed_time) * 1000;
            tracing::warn!("Throughput: {} bytes/s", throughput);
            return elapsed_time;
        }
        elapsed_time
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
            HotShotEvent::QuorumVoteRecv(_vote) => {},
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
                self.replica_entry(proposal.data.view_number())
                    .proposal_timestamp =
                    Some(proposal.data.block_header().timestamp_millis() as i128);
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
                let prev_epoch = self.epoch;
                let mut new_epoch = false;
                if self.epoch < *epoch {
                    self.epoch = *epoch;
                    new_epoch = true;
                }
                if *view == TYPES::View::new(0) {
                    return Ok(());
                }

                if new_epoch {
                    let elapsed_time = if let Some(prev_epoch) = prev_epoch {
                        self.log_basic_stats(now, &prev_epoch)
                    } else {
                        0
                    };
                    let _ = self.dump_stats();
                    if let Some(orchestrator_client) = self.orchestrator_client.as_ref() {
                        orchestrator_client
                            .post_bench_results::<TYPES>(BenchResults::<TYPES::View> {
                                node_index: self.node_index,
                                leader_view_stats: self.leader_stats.clone(),
                                replica_view_stats: self.replica_stats.clone(),
                                latencies_by_view: self.latencies_by_view.clone(),
                                sizes_by_view: self.sizes_by_view.clone(),
                                timeouts: self.timeouts.clone(),
                                total_time_millis: elapsed_time,
                            })
                            .await;
                    }
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
            HotShotEvent::Timeout(view, _) => {
                self.replica_entry(*view).timeout_triggered = Some(now);
                self.timeouts.insert(*view);
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
            HotShotEvent::LeavesDecided(leaves) => {
                for leaf in leaves {
                    if leaf.view_number() == TYPES::View::genesis() {
                        continue;
                    }
                    let view = leaf.view_number();
                    let timestamp = leaf.block_header().timestamp_millis() as i128;
                    let now_millis = now / 1_000_000;
                    let latency = now_millis - timestamp;
                    tracing::debug!("View {} Latency: {}ms", view, latency);
                    self.latencies_by_view.insert(view, latency);
                    self.sizes_by_view.insert(
                        view,
                        leaf.block_payload().map(|p| p.txn_bytes()).unwrap_or(0) as i128,
                    );
                }
            },
            _ => {},
        }
        Ok(())
    }

    fn cancel_subtasks(&mut self) {
        // No subtasks to cancel
    }
}
