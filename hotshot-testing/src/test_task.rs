// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{num::NonZeroUsize, sync::Arc, time::Duration};

use async_broadcast::{Receiver, Sender};
use async_lock::RwLock;
use async_trait::async_trait;
use futures::future::select_all;
use hotshot::{
    traits::TestableNodeImplementation,
    types::{Event, Message},
};
use hotshot_task_impls::{events::HotShotEvent, network::NetworkMessageTaskState};
use hotshot_types::{
    message::UpgradeLock,
    traits::{
        network::ConnectedNetwork,
        node_implementation::{NodeType, Versions},
    },
};
use tokio::{
    spawn,
    task::JoinHandle,
    time::{sleep, timeout},
};
use tracing::error;

use crate::test_runner::Node;

/// enum describing how the tasks completed
pub enum TestResult {
    /// the test task passed
    Pass,
    /// the test task failed with an error
    Fail(Box<dyn std::fmt::Debug + Send + Sync>),
}

pub fn spawn_timeout_task(test_sender: Sender<TestEvent>, timeout: Duration) -> JoinHandle<()> {
    tokio::spawn(async move {
        sleep(timeout).await;

        let _ = test_sender.broadcast(TestEvent::Shutdown).await;
    })
}

#[async_trait]
/// Type for mutable task state that can be used as the state for a `Task`
pub trait TestTaskState: Send {
    /// Type of event sent and received by the task
    type Event: Clone + Send + Sync;
    /// Type of error produced by the task
    type Error: std::fmt::Display;

    /// Handles an event from one of multiple receivers.
    async fn handle_event(
        &mut self,
        (event, id): (Self::Event, usize),
    ) -> std::result::Result<(), Self::Error>;

    /// Check the result of the test.
    async fn check(&self) -> TestResult;
}

/// Type alias for type-erased [`TestTaskState`] to be used for
/// dynamic dispatch
pub type AnyTestTaskState<TYPES> = Box<
    dyn TestTaskState<Event = hotshot_types::event::Event<TYPES>, Error = anyhow::Error>
        + Send
        + Sync,
>;

#[async_trait]
impl<TYPES: NodeType> TestTaskState for AnyTestTaskState<TYPES> {
    type Event = Event<TYPES>;
    type Error = anyhow::Error;

    async fn handle_event(
        &mut self,
        event: (Self::Event, usize),
    ) -> std::result::Result<(), anyhow::Error> {
        (**self).handle_event(event).await
    }

    async fn check(&self) -> TestResult {
        (**self).check().await
    }
}

#[async_trait]
pub trait TestTaskStateSeed<TYPES, I, V>: Send
where
    TYPES: NodeType,
    I: TestableNodeImplementation<TYPES>,
    V: Versions,
{
    async fn into_state(
        self: Box<Self>,
        handles: Arc<RwLock<Vec<Node<TYPES, I, V>>>>,
    ) -> AnyTestTaskState<TYPES>;
}

/// A basic task which loops waiting for events to come from `event_receiver`
/// and then handles them using it's state
/// It sends events to other `Task`s through `event_sender`
/// This should be used as the primary building block for long running
/// or medium running tasks (i.e. anything that can't be described as a dependency task)
pub struct TestTask<S: TestTaskState> {
    /// The state of the task.  It is fed events from `event_sender`
    /// and mutates it state ocordingly.  Also it signals the task
    /// if it is complete/should shutdown
    state: S,
    /// Receives events that are broadcast from any task, including itself
    receivers: Vec<Receiver<S::Event>>,
    /// Receiver for test events, used for communication between test tasks.
    test_receiver: Receiver<TestEvent>,
}

#[derive(Clone, Debug)]
pub enum TestEvent {
    Shutdown,
}

impl<S: TestTaskState + Send + 'static> TestTask<S> {
    /// Create a new task
    pub fn new(
        state: S,
        receivers: Vec<Receiver<S::Event>>,
        test_receiver: Receiver<TestEvent>,
    ) -> Self {
        TestTask {
            state,
            receivers,
            test_receiver,
        }
    }

    /// Spawn the task loop, consuming self.  Will continue until
    /// the task reaches some shutdown condition
    pub fn run(mut self) -> JoinHandle<TestResult> {
        spawn(async move {
            loop {
                if let Ok(TestEvent::Shutdown) = self.test_receiver.try_recv() {
                    break self.state.check().await;
                }

                let mut messages = Vec::new();

                for receiver in &mut self.receivers {
                    messages.push(receiver.recv());
                }

                match timeout(Duration::from_millis(2500), select_all(messages)).await {
                    Ok((Ok(input), id, _)) => {
                        let _ = S::handle_event(&mut self.state, (input, id))
                            .await
                            .inspect_err(|e| tracing::error!("{e}"));
                    },
                    Ok((Err(e), _id, _)) => {
                        error!("Error from one channel in test task {:?}", e);
                        sleep(Duration::from_millis(4000)).await;
                    },
                    _ => {},
                };
            }
        })
    }
}

/// Add the network task to handle messages and publish events.
pub async fn add_network_message_test_task<
    TYPES: NodeType,
    V: Versions,
    NET: ConnectedNetwork<TYPES::SignatureKey>,
>(
    internal_event_stream: Sender<Arc<HotShotEvent<TYPES>>>,
    external_event_stream: Sender<Event<TYPES>>,
    upgrade_lock: UpgradeLock<TYPES, V>,
    channel: Arc<NET>,
    public_key: TYPES::SignatureKey,
) -> JoinHandle<()> {
    let net = Arc::clone(&channel);
    let network_state: NetworkMessageTaskState<_, _> = NetworkMessageTaskState {
        internal_event_stream: internal_event_stream.clone(),
        external_event_stream: external_event_stream.clone(),
        public_key,
        transactions_cache: lru::LruCache::new(NonZeroUsize::new(100_000).unwrap()),
        upgrade_lock: upgrade_lock.clone(),
    };

    let network = Arc::clone(&net);
    let mut state = network_state.clone();

    spawn(async move {
        loop {
            // Get the next message from the network
            let message = match network.recv_message().await {
                Ok(message) => message,
                Err(e) => {
                    error!("Failed to receive message: {:?}", e);
                    continue;
                },
            };

            // Deserialize the message
            let deserialized_message: Message<TYPES> =
                match upgrade_lock.deserialize(&message).await {
                    Ok(message) => message,
                    Err(e) => {
                        tracing::error!("Failed to deserialize message: {:?}", e);
                        continue;
                    },
                };

            // Handle the message
            state.handle_message(deserialized_message).await;
        }
    })
}
