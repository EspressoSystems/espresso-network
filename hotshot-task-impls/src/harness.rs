// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{sync::Arc, time::Duration};

use async_broadcast::broadcast;
use hotshot_task::task::{ConsensusTaskRegistry, Task, TaskState};
use hotshot_types::traits::node_implementation::NodeType;
use tokio::time::timeout;

use crate::events::{HotShotEvent, HotShotTaskCompleted};

/// The state for the test harness task. Keeps track of which events and how many we expect to get
pub struct TestHarnessState<TYPES: NodeType> {
    /// The expected events we get from the test.  Maps an event to the number of times we expect to see it
    expected_output: Vec<HotShotEvent<TYPES>>,
    /// If true we won't fail the test if extra events come in
    allow_extra_output: bool,
}

/// Runs a test by building the task using `build_fn` and then passing it the `input` events
/// and testing the make sure all of the `expected_output` events are seen
///
/// # Arguments
/// * `event_stream` - if given, will be used to register the task builder.
/// * `allow_extra_output` - whether to allow an extra output after we've seen all expected
///   outputs. Should be `false` in most cases.
///
/// # Panics
/// Panics if any state the test expects is not set. Panicking causes a test failure
#[allow(clippy::implicit_hasher)]
#[allow(clippy::panic)]
pub async fn run_harness<TYPES, S: TaskState<Event = HotShotEvent<TYPES>> + Send + 'static>(
    input: Vec<HotShotEvent<TYPES>>,
    expected_output: Vec<HotShotEvent<TYPES>>,
    state: S,
    allow_extra_output: bool,
) where
    TYPES: NodeType,
{
    let mut registry = ConsensusTaskRegistry::new();
    // set up two broadcast channels so the test sends to the task and the task back to the test
    let (to_task, from_test) = broadcast(1024);
    let (to_test, mut from_task) = broadcast(1024);
    let mut test_state = TestHarnessState {
        expected_output,
        allow_extra_output,
    };

    let task = Task::new(state, to_test.clone(), from_test.clone());

    let handle = task.run();
    let test_future = async move {
        loop {
            if let Ok(event) = from_task.recv_direct().await {
                if let Some(HotShotTaskCompleted) = check_event(event, &mut test_state) {
                    break;
                }
            }
        }
    };

    registry.register(handle);

    for event in input {
        to_task.broadcast_direct(Arc::new(event)).await.unwrap();
    }

    assert!(
        timeout(Duration::from_secs(2), test_future).await.is_ok(),
        "Test timeout out before all all expected outputs received"
    );
}

/// Handles an event for the Test Harness Task.  If the event is expected, remove it from
/// the `expected_output` in state.  If unexpected fail test.
///
/// # Arguments
/// * `allow_extra_output` - whether to allow an extra output after we've seen all expected
///   outputs. Should be `false` in most cases.
///
///  # Panics
/// Will panic to fail the test when it receives and unexpected event
#[allow(clippy::needless_pass_by_value)]
fn check_event<TYPES: NodeType>(
    event: Arc<HotShotEvent<TYPES>>,
    state: &mut TestHarnessState<TYPES>,
) -> Option<HotShotTaskCompleted> {
    // Check the output in either case:
    // * We allow outputs only in our expected output set.
    // * We haven't received all expected outputs yet.
    if !state.allow_extra_output || !state.expected_output.is_empty() {
        assert!(
            state.expected_output.contains(&event),
            "Got an unexpected event: {event:?}",
        );
    }

    // NOTE: We only care about finding a single instance of the output event, and we just
    // iteratively remove the entries until they're gone.
    let idx = state
        .expected_output
        .iter()
        .position(|x| *x == *event)
        .unwrap();

    state.expected_output.remove(idx);

    if state.expected_output.is_empty() {
        tracing::info!("test harness task completed");
        return Some(HotShotTaskCompleted);
    }

    None
}
