use std::collections::{BTreeMap, BTreeSet};

use hotshot_types::{
    data::{EpochNumber, VidCommitment2, VidDisperse2, ViewNumber},
    drb::{DrbInput, DrbResult, compute_drb_result},
    epoch_membership::EpochMembershipCoordinator,
    traits::{
        BlockPayload,
        node_implementation::NodeType,
        storage::{LoadDrbProgressFn, StoreDrbProgressFn},
    },
    vid::avidm_gf2::{AvidmGf2Common, AvidmGf2Scheme, AvidmGf2Share},
};
use tokio::{
    spawn,
    sync::mpsc::{self},
    task::JoinHandle,
};

use crate::{
    coordinator::handle::CoordinatorHandle,
    events::{CpuEvent, VidDisperseRequest, VidShareInput},
    message::{Vote1, Vote2},
};

struct Task<T> {
    tx: mpsc::Sender<T>,
    handle: JoinHandle<()>,
}

impl<T> Task<T> {
    pub fn new(tx: mpsc::Sender<T>, handle: JoinHandle<()>) -> Self {
        Self { tx, handle }
    }
    pub async fn send(&self, item: T) -> Result<(), mpsc::error::SendError<T>> {
        self.tx.send(item).await
    }
}

struct CpuTaskManager<TYPES: NodeType> {
    event_rx: tokio::sync::mpsc::Receiver<CpuEvent<TYPES>>,

    vid_disperse_task: Task<VidDisperseRequest<TYPES>>,
    vid_share_task: Task<VidShareInput<TYPES>>,
    vote1_task: Task<Vote1<TYPES>>,
    vote2_task: Task<Vote2<TYPES>>,
    drb_request_task: Task<DrbInput>,
}

impl<TYPES: NodeType> CpuTaskManager<TYPES> {
    pub fn new(
        event_rx: tokio::sync::mpsc::Receiver<CpuEvent<TYPES>>,
        coordinator_handle: CoordinatorHandle<TYPES>,
        epoch_membership_coordinator: EpochMembershipCoordinator<TYPES>,
        store_drb_progress: StoreDrbProgressFn,
        load_drb_progress: LoadDrbProgressFn,
    ) -> Self {
        let (vid_disperse_tx, vid_disperse_rx) = mpsc::channel(100);
        let vid_disperse = VidDisperseTask::new(
            vid_disperse_rx,
            coordinator_handle.clone(),
            epoch_membership_coordinator.clone(),
        );
        let vid_disperse_task = Task::new(vid_disperse_tx, spawn(vid_disperse.run()));

        let (drb_request_tx, drb_request_rx) = mpsc::channel(100);
        let drb_request = DrbRequestTask::<TYPES>::new(
            drb_request_rx,
            coordinator_handle.clone(),
            store_drb_progress,
            load_drb_progress,
        );
        let drb_request_task = Task::new(drb_request_tx, spawn(drb_request.run()));

        Self {
            event_rx,
            vid_disperse_task,
            vid_share_task: todo!(),
            vote1_task: todo!(),
            vote2_task: todo!(),
            drb_request_task,
        }
    }
    pub async fn run(mut self) {
        while let Some(event) = self.event_rx.recv().await {
            self.handle_event(event).await;
        }
    }
    async fn handle_event(&mut self, event: CpuEvent<TYPES>) {
        match event {
            CpuEvent::DrbRequest(drb_input) => {
                let _ = self.drb_request_task.send(drb_input).await;
            },
            CpuEvent::VidShare(vid_share) => {
                let _ = self.vid_share_task.send(vid_share).await;
            },
            CpuEvent::VidDisperseRequest(vid_disperse_request) => {
                let _ = self.vid_disperse_task.send(vid_disperse_request).await;
            },
            CpuEvent::Vote1(vote1) => {
                let _ = self.vote1_task.send(vote1).await;
            },
            CpuEvent::Vote2(vote2) => {
                let _ = self.vote2_task.send(vote2).await;
            },
        }
    }
}

struct VidDisperseTask<TYPES: NodeType> {
    calculations: BTreeMap<ViewNumber, JoinHandle<()>>,
    epoch_membership_coordinator: EpochMembershipCoordinator<TYPES>,
    rx: tokio::sync::mpsc::Receiver<VidDisperseRequest<TYPES>>,
    coordinator_handle: CoordinatorHandle<TYPES>,
    internal_tx: mpsc::Sender<(ViewNumber, VidCommitment2, VidDisperse2<TYPES>)>,
    internal_rx: mpsc::Receiver<(ViewNumber, VidCommitment2, VidDisperse2<TYPES>)>,
}

impl<TYPES: NodeType> VidDisperseTask<TYPES> {
    fn new(
        rx: tokio::sync::mpsc::Receiver<VidDisperseRequest<TYPES>>,
        coordinator_handle: CoordinatorHandle<TYPES>,
        epoch_membership_coordinator: EpochMembershipCoordinator<TYPES>,
    ) -> Self {
        let (internal_tx, internal_rx) = mpsc::channel(100);
        Self {
            calculations: BTreeMap::new(),
            epoch_membership_coordinator,
            rx,
            coordinator_handle,
            internal_tx,
            internal_rx,
        }
    }
    async fn run(mut self) {
        loop {
            tokio::select! {
                Some(request) = self.rx.recv() => {
                if self.calculations.contains_key(&request.view) {
                    continue;
                }
                let view = request.view;
                let handle = self.handle_vid_disperse_request(request);
                self.calculations.insert(view, handle);
            },
            Some(response) = self.internal_rx.recv() => {
                let (view, payload_commitment, disperse) = response;
                self.calculations.remove(&view);
                let _ = self.coordinator_handle.respond_vid_disperse(payload_commitment, disperse).await;
            },
            else => break,
            }
        }
    }
    fn handle_vid_disperse_request(
        &self,
        vid_disperse_request: VidDisperseRequest<TYPES>,
    ) -> JoinHandle<()> {
        let tx = self.internal_tx.clone();
        let epoch_membership_coordinator = self.epoch_membership_coordinator.clone();
        spawn(async move {
            let Ok((disperse, duration)) = VidDisperse2::calculate_vid_disperse(
                &vid_disperse_request.block,
                &epoch_membership_coordinator,
                vid_disperse_request.view,
                Some(vid_disperse_request.epoch),
                Some(vid_disperse_request.epoch),
                &vid_disperse_request.metadata,
            )
            .await
            else {
                // TODO: Handle error
                return;
            };
            let _ = tx
                .send((
                    vid_disperse_request.view,
                    disperse.payload_commitment,
                    disperse,
                ))
                .await;
        })
    }
}

struct DrbRequestTask<TYPES: NodeType> {
    calculations: BTreeMap<EpochNumber, JoinHandle<()>>,
    rx: tokio::sync::mpsc::Receiver<DrbInput>,
    coordinator_handle: CoordinatorHandle<TYPES>,
    store_drb_progress: StoreDrbProgressFn,
    load_drb_progress: LoadDrbProgressFn,
    internal_tx: mpsc::Sender<(EpochNumber, DrbResult)>,
    internal_rx: mpsc::Receiver<(EpochNumber, DrbResult)>,
}

impl<TYPES: NodeType> DrbRequestTask<TYPES> {
    fn new(
        rx: tokio::sync::mpsc::Receiver<DrbInput>,
        coordinator_handle: CoordinatorHandle<TYPES>,
        store_drb_progress: StoreDrbProgressFn,
        load_drb_progress: LoadDrbProgressFn,
    ) -> Self {
        let (internal_tx, internal_rx) = mpsc::channel(100);
        Self {
            calculations: BTreeMap::new(),
            rx,
            coordinator_handle,
            store_drb_progress,
            load_drb_progress,
            internal_tx,
            internal_rx,
        }
    }

    async fn run(mut self) {
        loop {
            tokio::select! {
                Some(drb_input) = self.rx.recv() => {
                    let epoch = EpochNumber::new(drb_input.epoch);
                    if self.calculations.contains_key(&epoch) {
                        continue;
                    }
                    let handle = self.handle_drb_request(drb_input);
                    self.calculations.insert(epoch, handle);
                },
                Some((epoch, result)) = self.internal_rx.recv() => {
                    self.calculations.remove(&epoch);
                    let _ = self.coordinator_handle.respond_drb(result).await;
                },
                else => break,
            }
        }
    }

    fn handle_drb_request(&self, drb_input: DrbInput) -> JoinHandle<()> {
        let tx = self.internal_tx.clone();
        let store_drb_progress = self.store_drb_progress.clone();
        let load_drb_progress = self.load_drb_progress.clone();
        let epoch = EpochNumber::new(drb_input.epoch);
        spawn(async move {
            let result = compute_drb_result(drb_input, store_drb_progress, load_drb_progress).await;
            let _ = tx.send((epoch, result)).await;
        })
    }
}

struct VidShareAccumulator<TYPES: NodeType> {
    shares: Vec<AvidmGf2Share>,
    accumulated_weight: usize,
    common: AvidmGf2Common,
    metadata: <TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
}

impl<TYPES: NodeType> VidShareAccumulator<TYPES> {
    fn has_enough_shares(&self) -> bool {
        self.accumulated_weight >= self.common.param.recovery_threshold
    }
}

struct VidShareTask<TYPES: NodeType> {
    accumulators: BTreeMap<ViewNumber, VidShareAccumulator<TYPES>>,
    reconstructed: BTreeSet<ViewNumber>,
    rx: tokio::sync::mpsc::Receiver<VidShareInput<TYPES>>,
    coordinator_handle: CoordinatorHandle<TYPES>,
    internal_tx: mpsc::Sender<(ViewNumber, VidCommitment2, TYPES::BlockPayload)>,
    internal_rx: mpsc::Receiver<(ViewNumber, VidCommitment2, TYPES::BlockPayload)>,
}

impl<TYPES: NodeType> VidShareTask<TYPES> {
    fn new(
        rx: tokio::sync::mpsc::Receiver<VidShareInput<TYPES>>,
        coordinator_handle: CoordinatorHandle<TYPES>,
    ) -> Self {
        let (internal_tx, internal_rx) = mpsc::channel(100);
        Self {
            accumulators: BTreeMap::new(),
            reconstructed: BTreeSet::new(),
            rx,
            coordinator_handle,
            internal_tx,
            internal_rx,
        }
    }

    async fn run(mut self) {
        loop {
            tokio::select! {
                Some(input) = self.rx.recv() => {
                    let view = input.share.view_number;
                    if self.reconstructed.contains(&view) {
                        continue;
                    }
                    let payload_commitment = input.share.payload_commitment;
                    let weight = input.share.share.weight();
                    let accumulator = self.accumulators.entry(view).or_insert_with(|| {
                        VidShareAccumulator {
                            shares: Vec::new(),
                            accumulated_weight: 0,
                            common: input.share.common.clone(),
                            metadata: input.metadata.clone(),
                        }
                    });
                    accumulator.accumulated_weight += weight;
                    accumulator.shares.push(input.share.share);
                    if accumulator.has_enough_shares() {
                        self.try_reconstruct(view, payload_commitment);
                    }
                },
                Some((view, vid_commitment, payload)) = self.internal_rx.recv() => {
                    self.accumulators.remove(&view);
                    self.reconstructed.insert(view);
                    let _ = self.coordinator_handle.respond_block_reconstructed(view, payload, vid_commitment).await;
                },
                else => break,
            }
        }
    }

    fn try_reconstruct(&self, view: ViewNumber, payload_commitment: VidCommitment2) {
        let Some(accumulator) = self.accumulators.get(&view) else {
            return;
        };
        let shares = accumulator.shares.clone();
        let common = accumulator.common.clone();
        let metadata = accumulator.metadata.clone();
        let tx = self.internal_tx.clone();
        spawn(async move {
            let result =
                tokio::task::spawn_blocking(move || AvidmGf2Scheme::recover(&common, &shares))
                    .await;
            let Ok(Ok(bytes)) = result else {
                return;
            };
            let payload = TYPES::BlockPayload::from_bytes(&bytes, &metadata);
            let _ = tx.send((view, payload_commitment, payload)).await;
        });
    }
}
