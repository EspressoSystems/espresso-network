use std::{mem, sync::Arc};

use async_broadcast::{InactiveReceiver, Sender, broadcast};
use futures::stream::{BoxStream, StreamExt as _};
use hotshot_new_protocol::{
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
use tokio::{select, spawn};
use tokio_util::{sync::CancellationToken, task::AbortOnDropHandle};
use tracing::{error, warn};

/// Owns the new-protocol [`Coordinator`] and the task that drives it.
///
/// The coordinator is built at startup but only runs once [`CoordinatorTask::start`] is called.
/// Queries sent through its `ClientApi` before then wait in the coordinator's request channel.
pub(crate) struct CoordinatorTask<T, S>
where
    T: NodeType,
{
    state: Mutex<State<T, S>>,
    events: InactiveReceiver<CoordinatorEvent<T>>,
}

#[expect(
    clippy::large_enum_variant,
    reason = "held once per node, boxing buys nothing"
)]
enum State<T, S>
where
    T: NodeType,
{
    Parked {
        coordinator: Coordinator<T, S>,
        event_tx: Sender<CoordinatorEvent<T>>,
        queue_len: Option<Arc<dyn Gauge>>,
    },
    Running {
        handle: AbortOnDropHandle<()>,
        shutdown: CancellationToken,
    },
    Stopped,
}

impl<T, S> CoordinatorTask<T, S>
where
    T: NodeType,
    S: NewProtocolStorage<T>,
{
    pub(crate) fn new(
        coordinator: Coordinator<T, S>,
        event_channel_capacity: usize,
        metrics: &dyn Metrics,
    ) -> CoordinatorTask<T, S> {
        let (mut event_tx, mut event_rx) = broadcast(event_channel_capacity);
        event_tx.set_await_active(false);
        event_rx.set_overflow(true);

        let queue_len = metrics.is_recording().then(|| {
            metrics
                .create_gauge("coordinator_event_queue_len".into(), None)
                .into()
        });

        CoordinatorTask {
            state: Mutex::new(State::Parked {
                coordinator,
                event_tx,
                queue_len,
            }),
            events: event_rx.deactivate(),
        }
    }

    /// Every event the coordinator emits from now on.
    pub(crate) fn event_stream(&self) -> BoxStream<'static, CoordinatorEvent<T>> {
        self.events.activate_cloned().boxed()
    }

    /// Spawn the coordinator. Does nothing if it is already running or has been shut down.
    pub(crate) fn start(&self) {
        let mut state = self.state.lock();
        match mem::replace(&mut *state, State::Stopped) {
            State::Parked {
                coordinator,
                event_tx,
                queue_len,
            } => {
                let shutdown = CancellationToken::new();
                *state = State::Running {
                    handle: AbortOnDropHandle::new(spawn(run_coordinator(
                        coordinator,
                        event_tx,
                        queue_len,
                        shutdown.clone(),
                    ))),
                    shutdown,
                };
            },
            running @ State::Running { .. } => *state = running,
            State::Stopped => {},
        }
    }

    /// Stop the coordinator and wait for it to flush its storage.
    ///
    /// # Cancel safety
    ///
    /// This method is not cancel safe. Cancelling it after the coordinator was signalled aborts
    /// the coordinator task before its storage is flushed.
    pub(crate) async fn shut_down(&self) {
        let state = mem::replace(&mut *self.state.lock(), State::Stopped);
        match state {
            State::Running { handle, shutdown } => {
                shutdown.cancel();
                _ = handle.await;
            },
            State::Parked { .. } | State::Stopped => {},
        }
    }
}

async fn run_coordinator<T, S>(
    mut coord: Coordinator<T, S>,
    tx: Sender<CoordinatorEvent<T>>,
    queue_len: Option<Arc<dyn Gauge>>,
    shutdown: CancellationToken,
) where
    T: NodeType,
    S: NewProtocolStorage<T>,
{
    coord.start(None);

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
            payload: payload.clone(),
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
