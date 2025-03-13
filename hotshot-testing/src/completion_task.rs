// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::time::Duration;

use async_broadcast::{Receiver, Sender};
use hotshot_task_impls::helpers::broadcast_event;
use tokio::{spawn, task::JoinHandle, time::timeout};

use crate::test_task::TestEvent;

/// Completion task state
pub struct CompletionTask {
    pub tx: Sender<TestEvent>,

    pub rx: Receiver<TestEvent>,
    /// Duration of the task.
    pub duration: Duration,
}

impl CompletionTask {
    pub fn run(mut self) -> JoinHandle<()> {
        spawn(async move {
            if timeout(self.duration, self.wait_for_shutdown())
                .await
                .is_err()
            {
                broadcast_event(TestEvent::Shutdown, &self.tx).await;
            }
        })
    }
    async fn wait_for_shutdown(&mut self) {
        while let Ok(event) = self.rx.recv_direct().await {
            if matches!(event, TestEvent::Shutdown) {
                tracing::error!("Completion Task shutting down");
                return;
            }
        }
    }
}
/// Description for a time-based completion task.
#[derive(Clone, Debug)]
pub struct TimeBasedCompletionTaskDescription {
    /// Duration of the task.
    pub duration: Duration,
}

/// Description for a completion task.
#[derive(Clone, Debug)]
pub enum CompletionTaskDescription {
    /// Time-based completion task.
    TimeBasedCompletionTaskBuilder(TimeBasedCompletionTaskDescription),
}
