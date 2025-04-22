// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::sync::Arc;

use async_broadcast::{Receiver, RecvError, Sender};
use async_trait::async_trait;
use futures::future::try_join_all;
use hotshot_utils::anytrace::*;
use tokio::task::{spawn, JoinHandle};

/// Trait for events that long-running tasks handle
pub trait TaskEvent: PartialEq {
    /// The shutdown signal for this event type
    ///
    /// Note that this is necessarily uniform across all tasks.
    /// Exiting the task loop is handled by the task spawner, rather than the task individually.
    fn shutdown_event() -> Self;
}

#[async_trait]
/// Type for mutable task state that can be used as the state for a `Task`
pub trait TaskState: Send {
    /// Type of event sent and received by the task
    type Event: TaskEvent + Clone + Send + Sync;

    /// Joins all subtasks.
    fn cancel_subtasks(&mut self);

    /// Handles an event, providing direct access to the specific channel we received the event on.
    async fn handle_event(
        &mut self,
        event: Arc<Self::Event>,
        _sender: &Sender<Arc<Self::Event>>,
        _receiver: &Receiver<Arc<Self::Event>>,
    ) -> Result<()>;
}

/// A basic task which loops waiting for events to come from `event_receiver`
/// and then handles them using its state
/// It sends events to other `Task`s through `sender`
/// This should be used as the primary building block for long running
/// or medium running tasks (i.e. anything that can't be described as a dependency task)
pub struct Task<S: TaskState> {
    /// The state of the task.  It is fed events from `receiver`
    /// and mutated via `handle_event`.
    state: S,
    /// Sends events all tasks including itself
    sender: Sender<Arc<S::Event>>,
    /// Receives events that are broadcast from any task, including itself
    receiver: Receiver<Arc<S::Event>>,
}

impl<S: TaskState + Send + 'static> Task<S> {
    /// Create a new task
    pub fn new(state: S, sender: Sender<Arc<S::Event>>, receiver: Receiver<Arc<S::Event>>) -> Self {
        Task {
            state,
            sender,
            receiver,
        }
    }

    /// The state of the task, as a boxed dynamic trait object.
    fn boxed_state(self) -> Box<dyn TaskState<Event = S::Event>> {
        Box::new(self.state) as Box<dyn TaskState<Event = S::Event>>
    }

    /// Spawn the task loop, consuming self.  Will continue until
    /// the task reaches some shutdown condition
    pub fn run(mut self) -> JoinHandle<Box<dyn TaskState<Event = S::Event>>> {
        spawn(async move {
            loop {
                match self.receiver.recv_direct().await {
                    Ok(input) => {
                        if *input == S::Event::shutdown_event() {
                            self.state.cancel_subtasks();

                            break self.boxed_state();
                        }

                        log!(
                            S::handle_event(&mut self.state, input, &self.sender, &self.receiver)
                                .await
                        );
                    },
                    Err(RecvError::Closed) => {
                        break self.boxed_state();
                    },
                    Err(e) => {
                        tracing::error!("Failed to receive from event stream Error: {}", e);
                    },
                }
            }
        })
    }
}

#[derive(Default)]
/// A collection of tasks which can handle shutdown
pub struct ConsensusTaskRegistry<EVENT> {
    /// Tasks this registry controls
    task_handles: Vec<JoinHandle<Box<dyn TaskState<Event = EVENT>>>>,
}

impl<EVENT: Send + Sync + Clone + TaskEvent> ConsensusTaskRegistry<EVENT> {
    #[must_use]
    /// Create a new task registry
    pub fn new() -> Self {
        ConsensusTaskRegistry {
            task_handles: vec![],
        }
    }
    /// Add a task to the registry
    pub fn register(&mut self, handle: JoinHandle<Box<dyn TaskState<Event = EVENT>>>) {
        self.task_handles.push(handle);
    }
    /// Try to cancel/abort the task this registry has
    ///
    /// # Panics
    ///
    /// Should not panic, unless awaiting on the JoinHandle in tokio fails.
    pub async fn shutdown(&mut self) {
        let handles = &mut self.task_handles;

        while let Some(handle) = handles.pop() {
            let _ = handle
                .await
                .map(|mut task_state| task_state.cancel_subtasks());
        }
    }
    /// Take a task, run it, and register it
    pub fn run_task<S>(&mut self, task: Task<S>)
    where
        S: TaskState<Event = EVENT> + Send + 'static,
    {
        self.register(task.run());
    }

    /// Wait for the results of all the tasks registered
    /// # Panics
    /// Panics if one of the tasks panicked
    pub async fn join_all(self) -> Vec<Box<dyn TaskState<Event = EVENT>>> {
        try_join_all(self.task_handles).await.unwrap()
    }
}

#[derive(Default)]
/// A collection of tasks which can handle shutdown
pub struct NetworkTaskRegistry {
    /// Tasks this registry controls
    pub handles: Vec<JoinHandle<()>>,
}

impl NetworkTaskRegistry {
    #[must_use]
    /// Create a new task registry
    pub fn new() -> Self {
        NetworkTaskRegistry { handles: vec![] }
    }

    #[allow(clippy::unused_async)]
    /// Shuts down all tasks managed by this instance.
    ///
    /// This function waits for all tasks to complete before returning.
    ///
    /// # Panics
    ///
    /// When using the tokio executor, this function will panic if any of the
    /// tasks being joined return an error.
    pub async fn shutdown(&mut self) {
        let handles = std::mem::take(&mut self.handles);
        try_join_all(handles)
            .await
            .expect("Failed to join all tasks during shutdown");
    }

    /// Add a task to the registry
    pub fn register(&mut self, handle: JoinHandle<()>) {
        self.handles.push(handle);
    }
}
