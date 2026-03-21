mod drb;
mod vid;
mod vote;

use hotshot_types::{
    drb::DrbInput,
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    simple_vote::QuorumVote2,
    traits::{
        node_implementation::NodeType,
        storage::{LoadDrbProgressFn, StoreDrbProgressFn},
    },
};
use tokio::{
    spawn,
    sync::mpsc::{self},
    task::JoinHandle,
};

use self::{
    drb::DrbRequestTask,
    vid::{VidDisperseTask, VidShareTask},
    vote::VoteCollectionTask,
};
use crate::{
    coordinator::handle::CoordinatorHandle,
    events::{CpuEvent, VidDisperseRequest, VidShareInput},
    message::Vote2,
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
    event_rx: mpsc::Receiver<CpuEvent<TYPES>>,

    vid_disperse_task: Task<VidDisperseRequest<TYPES>>,
    vid_share_task: Task<VidShareInput<TYPES>>,
    vote1_task: Task<QuorumVote2<TYPES>>,
    vote2_task: Task<Vote2<TYPES>>,
    drb_request_task: Task<DrbInput>,
}

impl<TYPES: NodeType> CpuTaskManager<TYPES> {
    pub fn new(
        event_rx: mpsc::Receiver<CpuEvent<TYPES>>,
        coordinator_handle: CoordinatorHandle<TYPES>,
        epoch_membership_coordinator: EpochMembershipCoordinator<TYPES>,
        upgrade_lock: UpgradeLock<TYPES>,
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

        let (vid_share_tx, vid_share_rx) = mpsc::channel(100);
        let vid_share = VidShareTask::new(vid_share_rx, coordinator_handle.clone());
        let vid_share_task = Task::new(vid_share_tx, spawn(vid_share.run()));

        let (vote1_tx, vote1_rx) = mpsc::channel(100);
        let vote1 = VoteCollectionTask::new(
            vote1_rx,
            epoch_membership_coordinator.clone(),
            upgrade_lock.clone(),
        );
        let handle = coordinator_handle.clone();
        let vote1_task = Task::new(
            vote1_tx,
            spawn(vote1.run(move |cert| {
                let handle = handle.clone();
                Box::pin(async move {
                    let _ = handle.respond_certificate1(cert).await;
                })
            })),
        );

        let (vote2_tx, vote2_rx) = mpsc::channel(100);
        let vote2 =
            VoteCollectionTask::new(vote2_rx, epoch_membership_coordinator.clone(), upgrade_lock);
        let handle = coordinator_handle.clone();
        let vote2_task = Task::new(
            vote2_tx,
            spawn(vote2.run(move |cert| {
                let handle = handle.clone();
                Box::pin(async move {
                    let _ = handle.respond_certificate2(cert).await;
                })
            })),
        );

        Self {
            event_rx,
            vid_disperse_task,
            vid_share_task,
            vote1_task,
            vote2_task,
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
                let _ = self.vote1_task.send(vote1.vote).await;
                let _ = self
                    .vid_share_task
                    .send(VidShareInput {
                        share: vote1.vid_share,
                        metadata: None,
                    })
                    .await;
            },
            CpuEvent::Vote2(vote2) => {
                let _ = self.vote2_task.send(vote2).await;
            },
        }
    }
}
