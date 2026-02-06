use std::collections::BTreeMap;

use hotshot_types::{
    benchmarking::{
        LeaderViewStats as OtherLeaderViewStats, ReplicaViewStats as OtherReplicaViewStats,
    },
    traits::node_implementation::ConsensusTime,
};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReplicaViewStats {
    pub view: u64,
    pub view_change: Option<i128>,
    pub proposal_timestamp: Option<i128>,
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

impl<V: ConsensusTime> From<OtherReplicaViewStats<V>> for ReplicaViewStats {
    fn from(stats: OtherReplicaViewStats<V>) -> Self {
        Self {
            view: stats.view.u64(),
            ..stats.into()
        }
    }
}

impl<V: ConsensusTime> From<OtherLeaderViewStats<V>> for LeaderViewStats {
    fn from(stats: OtherLeaderViewStats<V>) -> Self {
        Self {
            view: stats.view.u64(),
            ..stats.into()
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LeaderViewStats {
    pub view: u64,
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
#[derive(Clone, Debug, Serialize)]
pub struct NormalizedViewTimeline {
    view: u64,
    previous_proposal_send_time: i128,
    leader_received_proposal_time: i128,
    previous_view_last_quorum_vote_time: i128,
    qc_formed_time: i128,
    block_built_time: i128,
    proposal_send_time: i128,
    da_proposal_send_time: i128,
    vid_sent_time: i128,
    proposal_recv_time: i128,
    proposal_validation_time: i128,
    vid_recv_time: i128,
    da_cert_formed_time: i128,
    da_cert_recv_time: i128,
    quorum_vote_send_time: i128,
}

impl NormalizedViewTimeline {
    fn from_stats(
        leader_record: &LeaderViewStats,
        replica_records: &[ReplicaViewStats],
        last_proposal_send_time: i128,
        last_view_last_quorum_vote_time: i128,
    ) -> Option<Self> {
        Some(Self {
            view: leader_record.view,
            previous_proposal_send_time: last_proposal_send_time,
            leader_received_proposal_time: leader_record.builder_start? / 1_000_000
                - last_proposal_send_time,
            previous_view_last_quorum_vote_time: last_view_last_quorum_vote_time
                - last_proposal_send_time,
            qc_formed_time: leader_record.qc_formed? / 1_000_000 - last_proposal_send_time,
            block_built_time: leader_record.block_built? / 1_000_000 - last_proposal_send_time,
            proposal_send_time: leader_record.proposal_send? / 1_000_000 - last_proposal_send_time,
            da_proposal_send_time: leader_record.da_proposal_send? / 1_000_000
                - last_proposal_send_time,
            vid_sent_time: leader_record.vid_disperse_send? / 1_000_000 - last_proposal_send_time,
            proposal_recv_time: replica_records
                .iter()
                .map(|r| r.proposal_recv.unwrap_or(0) / 1_000_000)
                .sorted()
                .nth((replica_records.len() * 2) / 3 + 1)
                .unwrap()
                - last_proposal_send_time,
            proposal_validation_time: replica_records
                .iter()
                .map(|r| r.proposal_validated.unwrap_or(0) / 1_000_000)
                .sorted()
                .nth((replica_records.len() * 2) / 3 + 1)
                .unwrap()
                - last_proposal_send_time,
            vid_recv_time: replica_records
                .iter()
                .filter(|r| r.vid_share_validated.is_some())
                .map(|r| r.vid_share_validated.unwrap_or(0) / 1_000_000)
                .sorted()
                .nth((replica_records.len() * 2) / 3 + 1)
                .unwrap()
                - last_proposal_send_time,
            da_cert_formed_time: leader_record.da_cert_send.unwrap() / 1_000_000
                - last_proposal_send_time,
            da_cert_recv_time: replica_records
                .iter()
                .map(|r| r.da_certificate_recv.unwrap_or(0) / 1_000_000)
                .sorted()
                .nth((replica_records.len() * 2) / 3 + 1)
                .unwrap()
                - last_proposal_send_time,
            quorum_vote_send_time: replica_records
                .iter()
                .map(|r| r.vote_send.unwrap_or(0) / 1_000_000)
                .sorted()
                .nth((replica_records.len() * 2) / 3 + 1)
                .unwrap()
                - last_proposal_send_time,
        })
    }
}

pub fn remove_views_with_no_preceding(
    normalized_views: &mut BTreeMap<u64, NormalizedViewTimeline>,
) {
    let mut previous_view = 0;
    let mut to_remove = Vec::new();
    for (view, _) in normalized_views.iter() {
        if *view != previous_view + 1 {
            to_remove.push(*view);
        }
        previous_view = *view;
    }
    for view in to_remove {
        normalized_views.remove(&view);
    }
}

fn get_last_quorum_vote_time(records_by_view: &[ReplicaViewStats]) -> i128 {
    records_by_view
        .iter()
        .map(|r| r.vote_send.unwrap() / 1_000_000)
        .nth((records_by_view.len() * 2) / 3 + 1)
        .unwrap()
}

fn normalize_views(
    records_by_view: &BTreeMap<u64, Vec<ReplicaViewStats>>,
    leader_records_by_view: &BTreeMap<u64, LeaderViewStats>,
) -> BTreeMap<u64, NormalizedViewTimeline> {
    let mut normalized_views = BTreeMap::new();
    let first_leader_record = leader_records_by_view.iter().next().unwrap();
    let mut proposal_send_time = first_leader_record.1.proposal_send.unwrap() / 1_000_000;
    let mut last_view_last_quorum_vote_time =
        get_last_quorum_vote_time(records_by_view.get(first_leader_record.0).unwrap());
    for (view, record) in leader_records_by_view.iter().skip(1) {
        let Some(replica_records) = records_by_view.get(view) else {
            println!("Replica records not found for view: {}", view);
            continue;
        };
        let Some(normalized_view) = NormalizedViewTimeline::from_stats(
            record,
            replica_records,
            proposal_send_time,
            last_view_last_quorum_vote_time,
        ) else {
            println!("Normalized view not found for view: {}", view);
            continue;
        };
        proposal_send_time = record.proposal_send.unwrap() / 1_000_000;
        last_view_last_quorum_vote_time = get_last_quorum_vote_time(replica_records);
        normalized_views.insert(*view, normalized_view);
    }
    normalized_views
}

pub fn get_metrics(
    records_by_view: &BTreeMap<u64, Vec<ReplicaViewStats>>,
    leader_records_by_view: BTreeMap<u64, LeaderViewStats>,
) -> BTreeMap<u64, NormalizedViewTimeline> {
    let mut records_by_view = records_by_view.clone();

    records_by_view.retain(|_, records| records.len() > 60);
    let mut leader_records_by_view = leader_records_by_view.clone();
    let first_view = *records_by_view.keys().next().unwrap();
    println!("First view: {}", first_view);
    leader_records_by_view = leader_records_by_view.split_off(&(first_view));

    let mut normalized_views = normalize_views(&records_by_view, &leader_records_by_view);

    // let processed_view_stats = process_view_stats(records_by_view, leader_records_by_view);
    // let durations = calculate_durations(processed_view_stats);
    remove_views_with_no_preceding(&mut normalized_views);
    normalized_views
}
