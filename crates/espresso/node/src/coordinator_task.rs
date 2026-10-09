use std::sync::Arc;

use async_broadcast::{InactiveReceiver, Sender, broadcast};
use futures::stream::{BoxStream, StreamExt as _};
use hotshot_new_protocol::{
    client::ClientApi,
    consensus::{ConsensusInput, ConsensusOutput},
    coordinator::{
        Coordinator,
        error::{CoordinatorError, Severity},
    },
    storage::NewProtocolStorage,
};
use hotshot_types::{
    data::VidDisperseShare,
    event::LeafInfo,
    new_protocol::CoordinatorEvent,
    traits::{
        ValidatedState,
        metrics::{Gauge, Metrics},
        node_implementation::NodeType,
    },
};
use parking_lot::Mutex;
use tokio::{
    select, spawn,
    sync::{oneshot, watch},
};
use tokio_util::{sync::CancellationToken, task::AbortOnDropHandle};
use tracing::{error, warn};

/// Owns the task that drives the new-protocol [`Coordinator`].
///
/// The task is spawned at startup but only starts the coordinator once [`CoordinatorTask::start`]
/// is called. Queries sent through its [`ClientApi`] before then wait in the coordinator's request
/// channel, so callers that must not wait check [`CoordinatorTask::status`] first.
pub(crate) struct CoordinatorTask<T>
where
    T: NodeType,
{
    client_api: ClientApi<T>,
    start: Mutex<Option<oneshot::Sender<()>>>,
    shutdown: CancellationToken,
    task: Mutex<Option<AbortOnDropHandle<()>>>,
    status: watch::Receiver<Status>,
    events: InactiveReceiver<CoordinatorEvent<T>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Status {
    NotStarted,
    Running,
    Stopped,
}

impl<T> CoordinatorTask<T>
where
    T: NodeType,
{
    pub(crate) fn new<S>(
        coordinator: Coordinator<T, S>,
        event_channel_capacity: usize,
        metrics: &dyn Metrics,
    ) -> CoordinatorTask<T>
    where
        S: NewProtocolStorage<T>,
    {
        let (mut event_tx, mut event_rx) = broadcast(event_channel_capacity);
        event_tx.set_await_active(false);
        event_rx.set_overflow(true);

        let queue_len = metrics.is_recording().then(|| {
            metrics
                .create_gauge("coordinator_event_queue_len".into(), None)
                .into()
        });

        let client_api = coordinator.client_api().clone();
        let (start_tx, start_rx) = oneshot::channel();
        let (status_tx, status_rx) = watch::channel(Status::NotStarted);
        let shutdown = CancellationToken::new();
        let task = spawn(run_coordinator(
            coordinator,
            event_tx,
            queue_len,
            start_rx,
            shutdown.clone(),
            status_tx,
        ));

        CoordinatorTask {
            client_api,
            start: Mutex::new(Some(start_tx)),
            shutdown,
            task: Mutex::new(Some(AbortOnDropHandle::new(task))),
            status: status_rx,
            events: event_rx.deactivate(),
        }
    }

    /// A handle for querying and driving the coordinator.
    pub(crate) fn client_api(&self) -> &ClientApi<T> {
        &self.client_api
    }

    /// Every event the coordinator emits from now on.
    pub(crate) fn event_stream(&self) -> BoxStream<'static, CoordinatorEvent<T>> {
        self.events.activate_cloned().boxed()
    }

    pub(crate) fn status(&self) -> watch::Receiver<Status> {
        self.status.clone()
    }

    /// Start the coordinator. Does nothing if it already started or has been shut down.
    pub(crate) fn start(&self) {
        let start = self.start.lock().take();
        if let Some(start) = start {
            _ = start.send(());
        }
    }

    /// Resolves once the coordinator has stopped, whether it was shut down or failed.
    pub(crate) async fn stopped(&self) {
        _ = self
            .status
            .clone()
            .wait_for(|status| *status == Status::Stopped)
            .await;
    }

    /// Stop the coordinator and wait for it to flush its storage.
    ///
    /// # Cancel safety
    ///
    /// This method is not cancel safe. Cancelling it after the coordinator was signalled aborts
    /// the coordinator task before its storage is flushed.
    pub(crate) async fn shut_down(&self) {
        self.shutdown.cancel();
        let task = self.task.lock().take();
        if let Some(task) = task {
            _ = task.await;
        }
    }
}

async fn run_coordinator<T, S>(
    coord: Coordinator<T, S>,
    tx: Sender<CoordinatorEvent<T>>,
    queue_len: Option<Arc<dyn Gauge>>,
    start: oneshot::Receiver<()>,
    shutdown: CancellationToken,
    status: watch::Sender<Status>,
) where
    T: NodeType,
    S: NewProtocolStorage<T>,
{
    select! {
        started = start => {
            if started.is_ok() {
                status.send_replace(Status::Running);
                drive_coordinator(coord, tx, queue_len, shutdown).await;
            }
        },
        () = shutdown.cancelled() => {},
    }
    status.send_replace(Status::Stopped);
}

async fn drive_coordinator<T, S>(
    mut coord: Coordinator<T, S>,
    tx: Sender<CoordinatorEvent<T>>,
    queue_len: Option<Arc<dyn Gauge>>,
    shutdown: CancellationToken,
) where
    T: NodeType,
    S: NewProtocolStorage<T>,
{
    coord.start();

    loop {
        select! {
            () = shutdown.cancelled() => break,
            it = coord.next_consensus_input() => {
                if let Err(err) = apply_input(&mut coord, &tx, it).await {
                    error!(%err, "coordinator: critical error");
                    break;
                }
                if let Some(m) = &queue_len {
                    m.set(tx.len())
                }
            }
        }
    }

    coord.stop().await;
}

async fn apply_input<T, S>(
    coord: &mut Coordinator<T, S>,
    tx: &Sender<CoordinatorEvent<T>>,
    it: Result<ConsensusInput<T>, CoordinatorError>,
) -> Result<(), CoordinatorError>
where
    T: NodeType,
    S: NewProtocolStorage<T>,
{
    match it {
        Ok(it) => coord.apply_consensus(it),
        Err(err) => {
            if err.severity == Severity::Critical {
                return Err(err);
            }
            warn!(%err, "coordinator: non-critical error");
        },
    }

    while let Some(out) = coord.outbox_mut().pop_front() {
        if let Some(e) = consensus_event(coord, &out) {
            broadcast_event(tx, e).await;
        }
        if let Err(err) = coord.process_consensus_output(out) {
            if err.severity == Severity::Critical {
                return Err(err);
            }
            warn!(%err, "coordinator: error processing output");
        }
    }

    while let Some(m) = coord.coordinator_outbox_mut().pop_front() {
        let e = CoordinatorEvent::ExternalMessageReceived {
            sender: m.sender,
            data: m.data,
        };
        broadcast_event(tx, e).await;
    }

    Ok(())
}

// TODO: `ConsensusOutput::LeafDecided` still carries fields (leaves +
// vid_shares) rather than a `Vec<LeafInfo>`. This is because `Consensus` doesn't own `StateManager`
// state and delta only become available one level up, in `Coordinator`.
fn consensus_event<T, S>(
    coordinator: &Coordinator<T, S>,
    output: &ConsensusOutput<T>,
) -> Option<CoordinatorEvent<T>>
where
    T: NodeType,
    S: NewProtocolStorage<T>,
{
    match output {
        ConsensusOutput::LeafDecided {
            leaves,
            cert1,
            cert2,
            vid_shares,
        } => {
            if leaves.is_empty() {
                tracing::error!("coordinator emitted LeafDecided with empty leaves");
                return None;
            }
            let leaf_infos = leaves
                .iter()
                .zip(vid_shares.iter())
                .map(|(leaf, vid_share)| {
                    let (state, delta) = match coordinator.state(leaf.view_number()) {
                        Some(s) => (s.state.clone(), s.delta.clone()),
                        None => {
                            let s = Arc::new(T::ValidatedState::from_header(leaf.block_header()));
                            (s, None)
                        },
                    };
                    let vid_share = vid_share
                        .as_ref()
                        .map(|share| VidDisperseShare::V2(share.data.clone()));
                    LeafInfo::new(leaf.clone(), state, delta, vid_share, None)
                })
                .collect();
            Some(CoordinatorEvent::NewDecide {
                leaf_infos,
                cert1: cert1.clone(),
                cert2: cert2.clone(),
            })
        },
        ConsensusOutput::ProposalValidated { proposal, sender } => {
            Some(CoordinatorEvent::QuorumProposal {
                proposal: proposal.clone(),
                sender: sender.clone(),
            })
        },
        ConsensusOutput::BlockPayloadReconstructed {
            view,
            header,
            payload,
        } => Some(CoordinatorEvent::BlockPayloadReconstructed {
            view: *view,
            header: header.clone(),
            payload: Arc::clone(payload),
        }),
        _ => None,
    }
}

async fn broadcast_event<T>(sender: &Sender<CoordinatorEvent<T>>, event: CoordinatorEvent<T>)
where
    T: NodeType,
{
    match sender.broadcast_direct(event).await {
        Ok(None) => {},
        Ok(Some(overflowed)) => {
            warn!(%overflowed, "coordinator event channel overflow, oldest event dropped");
        },
        Err(err) => {
            warn!(%err, "failed to broadcast consensus event");
        },
    }
}
