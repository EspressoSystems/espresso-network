use std::collections::BTreeMap;

use either::Either;
use hotshot_types::{traits::node_implementation::NodeType, vote::HasViewNumber};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tokio::{spawn, sync::mpsc};

use crate::events::HotShotEvent;

#[derive(Debug, Clone, Serialize)]
pub enum BenchmarkEventType {
    ViewChange,
    BlockBuilderStart,
    BlockBuilt,
    ProposalRecv,
    VoteSend,
    TimeoutVoteSend,
    DaProposalReceived,
    DaProposalValidated,
    DaCertificateRecv,
    DaCertificateSend,
    DaCertificateValidated,
    DaVoteSend,
    ProposalPrelimValidated,
    ProposalSend,
    DaProposalSend,
    ProposalValidated,
    QcFormed,
    TcFormed,
    SendPayloadCommitmentAndMetadata,
    VidSend,
    VidShareRecv,
    VidShareValidated,
    LeavesDecided,
    // Builder
    AvailableBlocksSent,
    AvailableBlocksReceived,
    BlockClaimsSent,
    BlockClaimsReceived,
    Shutdown,
}

pub enum BuilderEventType {
    AvailableBlocksSent,
    AvailableBlocksReceived,
    BlockClaimsSent,
    BlockClaimsReceived,
}

impl From<BuilderEventType> for BenchmarkEventType {
    fn from(event: BuilderEventType) -> Self {
        match event {
            BuilderEventType::AvailableBlocksSent => BenchmarkEventType::AvailableBlocksSent,
            BuilderEventType::AvailableBlocksReceived => {
                BenchmarkEventType::AvailableBlocksReceived
            },
            BuilderEventType::BlockClaimsSent => BenchmarkEventType::BlockClaimsSent,
            BuilderEventType::BlockClaimsReceived => BenchmarkEventType::BlockClaimsReceived,
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ViewData {
    view_number: u64,
    view_change: Option<i128>,
    block_building_start: Option<i128>,
    block_built: Option<i128>,
    proposal_recv: Option<i128>,
    vote_send: Option<i128>,
    timeout_vote_send: Option<i128>,
    da_proposal_received: Option<i128>,
    da_proposal_validated: Option<i128>,
    da_certificate_recv: Option<i128>,
    da_vote_send: Option<i128>,
    da_certificate_send: Option<i128>,
    da_certificate_validated: Option<i128>,
    proposal_prelim_validated: Option<i128>,
    proposal_send: Option<i128>,
    da_proposal_send: Option<i128>,
    proposal_validated: Option<i128>,
    qc_formed: Option<i128>,
    tc_formed: Option<i128>,
    send_payload_commitment_and_metadata: Option<i128>,
    vid_send: Option<i128>,
    vid_share_recv: Option<i128>,
    vid_share_validated: Option<i128>,
    leaves_decided: Option<i128>,
    available_blocks_sent: Option<i128>,
    available_blocks_received: Option<i128>,
    block_claims_sent: Option<i128>,
    block_claims_received: Option<i128>,
}

impl ViewData {
    fn new(view_number: u64) -> Self {
        Self {
            view_number,
            ..Default::default()
        }
    }

    fn update(&mut self, event: &BenchmarkEvent) {
        let timestamp = event.timestamp;
        assert_eq!(self.view_number, event.view_number);
        match event.event_type {
            BenchmarkEventType::ViewChange => {
                self.view_change.get_or_insert(timestamp);
            },
            BenchmarkEventType::BlockBuilderStart => {
                self.block_building_start.get_or_insert(timestamp);
            },
            BenchmarkEventType::BlockBuilt => {
                self.block_built.get_or_insert(timestamp);
            },
            BenchmarkEventType::ProposalRecv => {
                self.proposal_recv.get_or_insert(timestamp);
            },
            BenchmarkEventType::VoteSend => {
                self.vote_send.get_or_insert(timestamp);
            },
            BenchmarkEventType::TimeoutVoteSend => {
                self.timeout_vote_send.get_or_insert(timestamp);
            },
            BenchmarkEventType::DaProposalReceived => {
                self.da_proposal_received.get_or_insert(timestamp);
            },
            BenchmarkEventType::DaProposalValidated => {
                self.da_proposal_validated.get_or_insert(timestamp);
            },
            BenchmarkEventType::DaCertificateRecv => {
                self.da_certificate_recv.get_or_insert(timestamp);
            },
            BenchmarkEventType::DaCertificateSend => {
                self.da_certificate_send.get_or_insert(timestamp);
            },
            BenchmarkEventType::DaCertificateValidated => {
                self.da_certificate_validated.get_or_insert(timestamp);
            },
            BenchmarkEventType::DaVoteSend => {
                self.da_vote_send.get_or_insert(timestamp);
            },
            BenchmarkEventType::ProposalPrelimValidated => {
                self.proposal_prelim_validated.get_or_insert(timestamp);
            },
            BenchmarkEventType::ProposalSend => {
                self.proposal_send.get_or_insert(timestamp);
            },
            BenchmarkEventType::DaProposalSend => {
                self.da_proposal_send.get_or_insert(timestamp);
            },
            BenchmarkEventType::ProposalValidated => {
                self.proposal_validated.get_or_insert(timestamp);
            },
            BenchmarkEventType::QcFormed => {
                self.qc_formed.get_or_insert(timestamp);
            },
            BenchmarkEventType::TcFormed => {
                self.tc_formed.get_or_insert(timestamp);
            },
            BenchmarkEventType::SendPayloadCommitmentAndMetadata => {
                self.send_payload_commitment_and_metadata
                    .get_or_insert(timestamp);
            },
            BenchmarkEventType::VidSend => {
                self.vid_send.get_or_insert(timestamp);
            },
            BenchmarkEventType::VidShareRecv => {
                self.vid_share_recv.get_or_insert(timestamp);
            },
            BenchmarkEventType::VidShareValidated => {
                self.vid_share_validated.get_or_insert(timestamp);
            },
            BenchmarkEventType::LeavesDecided => {
                self.leaves_decided.get_or_insert(timestamp);
            },
            BenchmarkEventType::Shutdown => {
                panic!("Shutdown event should not ever be stored");
            },
            BenchmarkEventType::AvailableBlocksSent => {
                self.available_blocks_sent.get_or_insert(timestamp);
            },
            BenchmarkEventType::AvailableBlocksReceived => {
                self.available_blocks_received.get_or_insert(timestamp);
            },
            BenchmarkEventType::BlockClaimsSent => {
                self.block_claims_sent.get_or_insert(timestamp);
            },
            BenchmarkEventType::BlockClaimsReceived => {
                self.block_claims_received.get_or_insert(timestamp);
            },
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkEvent {
    view_number: u64,
    event_type: BenchmarkEventType,
    timestamp: i128,
}

pub fn hothshot_event_to_benchmark_event<TYPES: NodeType>(
    event: &HotShotEvent<TYPES>,
) -> BenchmarkEvent {
    let timestamp = OffsetDateTime::now_utc().unix_timestamp_nanos();
    match event {
        HotShotEvent::BlockRecv(bundle) => BenchmarkEvent {
            view_number: *bundle.view_number,
            event_type: BenchmarkEventType::BlockBuilt,
            timestamp,
        },
        HotShotEvent::QuorumProposalRecv(proposal, _) => BenchmarkEvent {
            view_number: *proposal.data.view_number(),
            event_type: BenchmarkEventType::ProposalRecv,
            timestamp,
        },
        HotShotEvent::DaProposalRecv(proposal, _) => BenchmarkEvent {
            view_number: *proposal.data.view_number(),
            event_type: BenchmarkEventType::DaProposalReceived,
            timestamp,
        },
        HotShotEvent::DaProposalValidated(proposal, _) => BenchmarkEvent {
            view_number: *proposal.data.view_number(),
            event_type: BenchmarkEventType::DaProposalValidated,
            timestamp,
        },
        HotShotEvent::QuorumProposalSend(proposal, _) => BenchmarkEvent {
            view_number: *proposal.data.view_number(),
            event_type: BenchmarkEventType::ProposalSend,
            timestamp,
        },
        HotShotEvent::DaProposalSend(proposal, _) => BenchmarkEvent {
            view_number: *proposal.data.view_number(),
            event_type: BenchmarkEventType::DaProposalSend,
            timestamp,
        },
        HotShotEvent::QuorumVoteSend(vote) => BenchmarkEvent {
            view_number: *vote.view_number(),
            event_type: BenchmarkEventType::VoteSend,
            timestamp,
        },
        HotShotEvent::TimeoutVoteSend(vote) => BenchmarkEvent {
            view_number: *vote.view_number(),
            event_type: BenchmarkEventType::TimeoutVoteSend,
            timestamp,
        },
        HotShotEvent::QuorumProposalValidated(proposal, _) => BenchmarkEvent {
            view_number: *proposal.data.view_number(),
            event_type: BenchmarkEventType::ProposalValidated,
            timestamp,
        },
        HotShotEvent::DaCertificateRecv(certificate) => BenchmarkEvent {
            view_number: *certificate.view_number(),
            event_type: BenchmarkEventType::DaCertificateRecv,
            timestamp,
        },
        HotShotEvent::DacSend(certificate, _) => BenchmarkEvent {
            view_number: *certificate.view_number(),
            event_type: BenchmarkEventType::DaCertificateSend,
            timestamp,
        },
        HotShotEvent::QuorumProposalPreliminarilyValidated(proposal) => BenchmarkEvent {
            view_number: *proposal.data.view_number(),
            event_type: BenchmarkEventType::ProposalPrelimValidated,
            timestamp,
        },
        HotShotEvent::ViewChange(view_number, _) => BenchmarkEvent {
            view_number: **view_number,
            event_type: BenchmarkEventType::ViewChange,
            timestamp,
        },
        HotShotEvent::DaCertificateValidated(simple_certificate) => BenchmarkEvent {
            view_number: *simple_certificate.view_number(),
            event_type: BenchmarkEventType::DaCertificateValidated,
            timestamp,
        },
        // HotShotEvent::QuorumProposalRequestSend(proposal_request_payload, _) => todo!(),
        // HotShotEvent::QuorumProposalRequestRecv(proposal_request_payload, _) => todo!(),
        // HotShotEvent::QuorumProposalResponseSend(_, proposal) => todo!(),
        // HotShotEvent::QuorumProposalResponseRecv(proposal) => todo!(),
        HotShotEvent::DaVoteSend(simple_vote) => BenchmarkEvent {
            view_number: *simple_vote.view_number(),
            event_type: BenchmarkEventType::DaVoteSend,
            timestamp,
        },
        HotShotEvent::Qc2Formed(either) => match either {
            Either::Left(qc) => BenchmarkEvent {
                view_number: *qc.view_number() + 1,
                event_type: BenchmarkEventType::QcFormed,
                timestamp,
            },
            Either::Right(tc) => BenchmarkEvent {
                view_number: *tc.view_number(),
                event_type: BenchmarkEventType::TcFormed,
                timestamp,
            },
        },
        HotShotEvent::SendPayloadCommitmentAndMetadata(_, _, _, view, _) => BenchmarkEvent {
            view_number: **view,
            event_type: BenchmarkEventType::SendPayloadCommitmentAndMetadata,
            timestamp,
        },
        HotShotEvent::VidDisperseSend(proposal, _) => BenchmarkEvent {
            view_number: *proposal.data.view_number(),
            event_type: BenchmarkEventType::VidSend,
            timestamp,
        },
        HotShotEvent::VidShareRecv(_, proposal) => BenchmarkEvent {
            view_number: *proposal.data.view_number(),
            event_type: BenchmarkEventType::VidShareRecv,
            timestamp,
        },
        HotShotEvent::VidShareValidated(proposal) => BenchmarkEvent {
            view_number: *proposal.data.view_number(),
            event_type: BenchmarkEventType::VidShareValidated,
            timestamp,
        },
        // HotShotEvent::VidRequestSend(data_request, ..) => todo!(),
        // HotShotEvent::VidRequestRecv(data_request, _) => todo!(),
        // HotShotEvent::VidResponseSend(_, _, proposal) => todo!(),
        // HotShotEvent::VidResponseRecv(_, proposal) => todo!(),
        // HotShotEvent::HighQcRecv(simple_certificate, simple_certificate1, _) => todo!(),
        // HotShotEvent::HighQcSend(simple_certificate, simple_certificate1, ..) => todo!(),
        // HotShotEvent::ExtendedQcRecv(simple_certificate, simple_certificate1, _) => todo!(),
        // HotShotEvent::ExtendedQcSend(simple_certificate, simple_certificate1, _) => todo!(),
        // HotShotEvent::EpochRootQcSend(epoch_root_quorum_certificate_v2, ..) => todo!(),
        // HotShotEvent::EpochRootQcRecv(epoch_root_quorum_certificate_v2, _) => todo!(),
        HotShotEvent::LeavesDecided(leaf2s) => BenchmarkEvent {
            view_number: *leaf2s.first().unwrap().view_number(),
            event_type: BenchmarkEventType::LeavesDecided,
            timestamp,
        },
        event => {
            panic!("Unhandled event: {:?}", event);
        },
    }
}

pub async fn send_builder_benchmark_event(
    sender: &mpsc::Sender<BenchmarkEvent>,
    event: BuilderEventType,
    view_number: u64,
) {
    let timestamp = OffsetDateTime::now_utc().unix_timestamp_nanos();
    let benchmark_event = BenchmarkEvent {
        view_number,
        event_type: event.into(),
        timestamp,
    };
    let _ = sender.send(benchmark_event).await;
}

pub async fn send_hothshot_benchmark_event<TYPES: NodeType>(
    sender: &mpsc::Sender<BenchmarkEvent>,
    event: &HotShotEvent<TYPES>,
) {
    let _ = sender.send(hothshot_event_to_benchmark_event(event)).await;
}

pub async fn send_benchmark_event(
    sender: &mpsc::Sender<BenchmarkEvent>,
    types: BenchmarkEventType,
    view_number: u64,
) {
    let timestamp = OffsetDateTime::now_utc().unix_timestamp_nanos();
    let _ = sender
        .send(BenchmarkEvent {
            view_number,
            event_type: types,
            timestamp,
        })
        .await;
}

pub struct BenchmarkEventCollector {
    events: BTreeMap<u64, Vec<BenchmarkEvent>>,
    receiver: mpsc::Receiver<BenchmarkEvent>,
}

impl BenchmarkEventCollector {
    pub fn new(receiver: mpsc::Receiver<BenchmarkEvent>) -> Self {
        Self {
            events: BTreeMap::new(),
            receiver,
        }
    }

    pub fn run(mut self) {
        spawn(async move {
            while let Some(event) = self.receiver.recv().await {
                if matches!(event.event_type, BenchmarkEventType::Shutdown) {
                    return;
                }
                self.events
                    .entry(event.view_number)
                    .or_default()
                    .push(event);
            }
        });
    }

    pub fn get_view_data(&self) -> Vec<ViewData> {
        let mut views = Vec::new();
        for (view_number, events) in self.events.iter() {
            let mut view_data = ViewData::new(*view_number);
            for event in events {
                view_data.update(event);
            }
            views.push(view_data);
        }
        views
    }
}
